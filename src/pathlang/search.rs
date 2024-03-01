/// We traverse the cell tree in dfs order, cells to be explored being stored
/// in a stack together with the points in the search path that need to be
/// matched against their children. Initially each cell is matched against
/// only one point in the path. Some operators in the search path (e.g. double
/// kleene) can lead to a split of the search path locations that match the
/// cell.
/// Loosely inspired from https://swtch.com/~rsc/regexp/regexp2.html
use crate::{
    base::*,
    debug_err, guard_ok, guard_some,
    pathlang::{
        path::{Expression, PathItem},
        Path,
    },
    warning,
};

macro_rules! ifdebug {
    ( $body:expr ) => {
        $body
    };
}

#[derive(Clone, Debug)]
pub struct Searcher<'s> {
    path: Vec<PathItem<'s>>,
    // dfs exploration of the cell tree in search of the path
    stack: Vec<MatchTest>,
    // to find out where the search failed
    next_max_path_index: usize,
}

/// a cell to be matched against path_index
#[derive(Clone, Debug)]
pub struct MatchTest {
    // parent of cells to be matched against the path_index
    parent: Cell,
    // path index to be tested
    path_index: usize,
}

//  Let us have this tree:
//  root:
//      a:
//          aa: 1
//      b:
//          bb: 2
// and this path: /**/a/**/aa
// search goes like this:
// ----
// - push (root,0)
// - stack is: [(root, 0:/**)]
//     test root/* against /** (index 0)  -> accepted -> push (b,1), (a,1)
//     test root itself against doublestar (index 0)  -> accepted -> push (root,1)
// - stack is: [(b,1), (a,1), (root, 1)]
//      test root/* against /a (index 1)  -> accepted -> push (a,2)
// - stack is: [(b,1), (a,1), (a,2)]
//      test a/* against `**`  (index 2) -> accepted -> push (aa,3)
//      test a itself against doublestar  (index 2) -> accepted -> push (a,3)
// - stack is: [(b,1), (a,1), (aa,3), (a,3)]
//      test a/* against `aa` (index 3) -> accepted -> push (aa,4)
// - stack is: [(b,1), (a,1), (aa,3), (aa,4)]
//      found match aa,4
// - stack is: [(b,1), (a,1), (aa,3)]
//      test aa/* against `aa`  (index 3) -> no match
// ... (no more matches)

impl<'s> Searcher<'s> {
    pub(crate) fn new(start: Cell, path: Path<'s>) -> Searcher<'s> {
        ifdebug!(println!(
            "\nnew Searcher, path: {:?}:",
            path.0
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
        ));
        let p = PathItem {
            relation: Relation::Interpretation,
            selector: None,
            index: None,
            filters: vec![],
        };

        // start cell is the parent of cells to be matched against path index 0
        let start_match = MatchTest {
            parent: start,
            path_index: 0,
        };
        Searcher {
            path: path.0,
            stack: vec![start_match],
            next_max_path_index: 0,
        }
    }

    fn eval_next(&mut self) -> Option<Res<Cell>> {
        while !self.stack.is_empty() {
            if let Some(cell) = self.pump_stack() {
                Self::update_next_max_path_index(&self.stack, &mut self.next_max_path_index);
                ifdebug!(println!(
                    "returning cell {:?}",
                    cell.as_ref().map(|x| x.debug_string())
                ));
                return Some(cell);
            }
        }
        None
    }

    fn pump_stack(&mut self) -> Option<Res<Cell>> {
        ifdebug!(println!(
            "----\nstack:{}",
            self.stack
                .iter()
                .map(|cn| format!("    `{}` : {:?}", cn.parent.debug_string(), cn.path_index))
                .collect::<Vec<String>>()
                .join("  ")
        ));

        // pop the last cell match from the stack
        let MatchTest { parent, path_index } = guard_some!(self.stack.pop(), { return None });

        if path_index >= self.path.len() {
            return Some(Ok(parent));
        }
        let pi = &self.path[path_index];

        ifdebug!(println!(
            "test children of `{}` against `{}` (path index {})",
            parent.debug_string(),
            pi,
            path_index
        ));

        let group = match pi.relation {
            Relation::Sub => parent.sub(),
            Relation::Attr => parent.attr(),
            Relation::Interpretation => parent.elevate(),
            Relation::Field => parent.field(),
        };
        let group = match group.err() {
            Ok(group) => Some(group),
            Err(err) => {
                if err.kind != HErrKind::None {
                    return Some(Err(err));
                }
                None
            }
        };

        if let Some(group) = group {
            Self::process_group(
                &mut self.stack,
                &self.path,
                &parent,
                group,
                path_index,
                &mut self.next_max_path_index,
            );
        }

        // test for doublestar matching nothing, i.e. the parent against the next path item
        if pi.selector == Some(Selector::DoubleStar) {
            Self::process_cell(
                &mut self.stack,
                &self.path,
                parent,
                path_index,
                true,
                &mut self.next_max_path_index,
            )
        }

        None
    }

    fn process_group(
        stack: &mut Vec<MatchTest>,
        path: &[PathItem],
        parent: &Cell,
        group: Group,
        path_index: usize,
        next_max_path_index: &mut usize,
    ) {
        let pi = &path[path_index];
        match (pi.selector, pi.index) {
            (Some(Selector::Star) | Some(Selector::DoubleStar), None) => {
                ifdebug!(println!("iterating over all children"));
                for i in (0..group.len().unwrap_or(0)).rev() {
                    let cell = guard_ok!(group.at(i).err(), err => {
                        warning!("Error while searching: cannot get cell: {:?}", err);
                        continue;
                    });
                    Self::process_cell(
                        stack,
                        path,
                        cell,
                        path_index,
                        pi.selector != Some(Selector::DoubleStar),
                        next_max_path_index,
                    )
                }
            }
            (None | Some(Selector::Star) | Some(Selector::DoubleStar), Some(index)) => {
                ifdebug!(println!("get child by index"));
                let cell = guard_ok!(group.at(index).err(), err => {
                    if err.kind != HErrKind::None {
                        warning!("Error while searching: cannot get cell: {:?}", err);
                    }
                    return ;
                });
                Self::process_cell(
                    stack,
                    path,
                    cell,
                    path_index,
                    pi.selector != Some(Selector::DoubleStar),
                    next_max_path_index,
                );
            }
            (Some(Selector::Str(label)), opt_index) => {
                ifdebug!(println!("iterating over children by label"));
                let mut iter = guard_ok!(group.get_all(label).err(), err => {
                    if err.kind != HErrKind::None {
                        warning!("Error while searching: cannot get cell iterator: {:?}", err);
                    }
                    return ;
                });
                if let Some(index) = opt_index {
                    if let Some(cell) = iter.nth(index) {
                        Self::process_cell(stack, path, cell, path_index, true, next_max_path_index)
                    }
                } else {
                    for cell in iter {
                        Self::process_cell(stack, path, cell, path_index, true, next_max_path_index)
                    }
                }
            }
            (None, None) => {
                warning!("missing both selector and index in search");
            }
            (Some(Selector::Top), _) => {
                warning!("Selector::Top not supported in search");
            }
        }
    }

    fn process_cell(
        stack: &mut Vec<MatchTest>,
        path: &[PathItem],
        cell: Cell,
        path_index: usize,
        advance_index: bool,
        next_max_path_index: &mut usize,
    ) {
        let cell = guard_ok!(cell.err(), err => {
            if err.kind != HErrKind::None {
                warning!("Error while searching: cannot get cell: {:?}", err);
            }
            return;
        });

        let pi = &path[path_index];

        ifdebug!(println!("test: `{}` for {}", cell.debug_string(), pi));

        if !Self::eval_filters_match(&cell, pi) {
            ifdebug!(println!("no match `{}` for {}", cell.debug_string(), pi));
            return;
        }

        let next_path_index = if advance_index {
            path_index + 1
        } else {
            path_index
        };

        ifdebug!(println!(
            "match, push: `{}` : {:?}",
            cell.debug_string(),
            next_path_index
        ));
        stack.push(MatchTest {
            parent: cell.clone(),
            path_index: next_path_index,
        });
        Self::update_next_max_path_index(stack, next_max_path_index);
    }

    fn eval_filters_match(subcell: &Cell, path_item: &PathItem) -> bool {
        for filter in &path_item.filters {
            match Searcher::eval_bool_expression(subcell.clone(), &filter.expr) {
                Err(e) => {
                    ifdebug!(println!("eval_bool_expression failed"));
                    debug_err!(e);
                    return false;
                }
                Ok(false) => {
                    ifdebug!(println!(
                        "no match of cell `{}` for filter `{}`",
                        subcell.debug_string(),
                        filter
                    ));
                    return false;
                }
                Ok(true) => {}
            }
        }
        true
    }

    fn eval_bool_expression(cell: Cell, expr: &Expression<'s>) -> Res<bool> {
        ifdebug!(println!(
            "{{{{\neval_bool_expression cell `{}` for expr `{}`",
            cell.debug_string(),
            expr
        ));

        fn eval_expr(op: &str, left: Value, right: Value) -> Res<bool> {
            if !["==", "!="].contains(&op) {
                return userres(format!("bad operand: {}", op));
            }
            match op {
                "==" if left == right => Ok(true),
                "!=" if left != right => Ok(true),
                _ => Ok(false),
            }
        }

        let eval_iter_left = Self::new(cell, expr.left.clone());
        for cell in eval_iter_left {
            let cell = guard_ok!(cell, err => {
                debug_err!(err);
                continue;
            });
            if let Some(op) = expr.op {
                if let Some(right) = expr.right {
                    let reader = guard_ok!(cell.read().err(), err => {
                        debug_err!(err);
                        continue;
                    });

                    let lvalue = guard_ok!(reader.value(), err => {
                        debug_err!(err);
                        continue;
                    });
                    if eval_expr(op, lvalue, right)? {
                        ifdebug!(println!("eval_bool_expression true\n}}}}"));
                        return Ok(true);
                    }
                }
            } else {
                ifdebug!(println!("eval_bool_expression true\n}}}}"));
                return Ok(true);
            }
        }
        ifdebug!(println!("eval_bool_expression false\n}}}}"));
        Ok(false)
    }

    fn update_next_max_path_index(stack: &[MatchTest], next_max_path_index: &mut usize) {
        let max_i = stack.iter().map(|cn| cn.path_index).max().unwrap_or(0);
        if max_i > *next_max_path_index {
            *next_max_path_index = max_i;
        }
    }

    /// returns the minimal path that failed to match
    pub fn unmatched_path(&self) -> String {
        let mut path = String::new();
        for i in 0..self.next_max_path_index {
            path.push_str(self.path[i].to_string().as_str());
        }
        if self.next_max_path_index < self.path.len() {
            path.push_str(self.path[self.next_max_path_index].to_string().as_str());
        }
        path
    }
}

impl<'s> Iterator for Searcher<'s> {
    type Item = Res<Cell>;
    fn next(&mut self) -> Option<Res<Cell>> {
        self.eval_next()
    }
}
