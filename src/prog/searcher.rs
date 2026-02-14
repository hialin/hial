/// We traverse the cell tree in dfs order, cells to be explored being stored
/// in a stack together with the points in the search path that need to be
/// matched against their children. Initially each cell is matched against
/// only one point in the path. Some operators in the search path (e.g. double
/// kleene) can lead to a split of the search path locations that match the
/// cell.
/// Loosely inspired from https://swtch.com/~rsc/regexp/regexp2.html
use crate::{
    api::*,
    debug_err, guard_ok, guard_some,
    prog::{
        Path,
        path::{Expression, PathItem},
    },
    warning,
};

use super::path::{ElevationPathItem, NormalPathItem};

macro_rules! ifdebug {
    ( $body:expr ) => {
        // $body
    };
}

#[derive(Clone, Debug)]
pub struct Searcher<'s> {
    path: Vec<PathItem<'s>>,
    // dfs exploration of the cell tree in search of the path
    stack: Vec<MatchTest>,
    // to find out where the search failed
    next_max_path_index: usize,
    filter_eval: bool,
}

/// a cell to be matched against path_index
#[derive(Clone, Debug)]
pub struct MatchTest {
    // parent of cells to be matched against the path_index
    parent: Xell,
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
    pub(crate) fn new(start: Xell, path: Path<'s>) -> Searcher<'s> {
        Self::new_with_filter_eval(start, path, false)
    }

    fn new_with_filter_eval(start: Xell, path: Path<'s>, filter_eval: bool) -> Searcher<'s> {
        ifdebug!(println!(
            "\nnew Searcher, path: {:?}:",
            path.0
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
        ));
        // start cell is the parent of cells to be matched against path index 0
        let start_match = MatchTest {
            parent: start,
            path_index: 0,
        };
        Searcher {
            path: path.0,
            stack: vec![start_match],
            next_max_path_index: 0,
            filter_eval,
        }
    }

    fn eval_next(&mut self) -> Option<Res<Xell>> {
        while !self.stack.is_empty() {
            if let Some(cell) = self.pump_stack() {
                Self::update_next_max_path_index(&self.stack, &mut self.next_max_path_index);
                ifdebug!(println!(
                    "returning cell {:?}",
                    cell.as_ref().map(|x| x.debug_string())
                ));
                match cell.and_then(|cell| cell.err()) {
                    Ok(cell) => return Some(Ok(cell)),
                    Err(e) => {
                        if e.kind != HErrKind::None {
                            warning!("search error: {}", e)
                        }
                    }
                }
            }
        }
        None
    }

    fn pump_stack(&mut self) -> Option<Res<Xell>> {
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

        match pi {
            PathItem::Elevation(npi) => {
                let opt_res = Self::process_elevation(
                    &mut self.stack,
                    &self.path,
                    self.filter_eval,
                    npi,
                    parent,
                    path_index,
                    &mut self.next_max_path_index,
                );
                if let Some(Err(e)) = opt_res {
                    return Some(Err(e));
                }
            }
            PathItem::Normal(npi) => {
                let group = match npi.relation {
                    Relation::Sub => parent.sub(),
                    Relation::Attr => parent.attr(),
                    Relation::Field => parent.field(),
                    Relation::Interpretation => {
                        return Some(fault("interpretation relation not expected here"));
                    }
                };
                match group.err() {
                    Err(e) => {
                        if e.kind != HErrKind::None {
                            return Some(Err(e));
                        }
                    }
                    Ok(group) => {
                        Self::process_group(
                            &mut self.stack,
                            &self.path,
                            &parent,
                            group,
                            path_index,
                            &mut self.next_max_path_index,
                        );
                    }
                }

                // test for doublestar matching nothing, i.e. the parent against the next path item
                if npi.selector == Some(Selector::DoubleStar) {
                    Self::process_cell(
                        &mut self.stack,
                        &self.path,
                        parent,
                        path_index,
                        true,
                        &mut self.next_max_path_index,
                    )
                }
            }
        }
        None
    }

    #[must_use]
    fn process_elevation(
        stack: &mut Vec<MatchTest>,
        path: &[PathItem],
        filter_eval: bool,
        epi: &ElevationPathItem,
        parent: Xell,
        path_index: usize,
        next_max_path_index: &mut usize,
    ) -> Option<Res<()>> {
        ifdebug!(println!(
            "process_elevation, parent: {}, epi: {:?}",
            parent.debug_string(),
            epi
        ));
        let group = parent.elevate();
        let itp_cell = match epi.interpretation {
            Selector::Str(itp) => group.get(itp),
            _ => return Some(userres("bad interpretation selector")),
        };
        let itp_cell = guard_ok!(itp_cell.err(), err => {
            ifdebug!(println!("no such interpretation: {:?}", err));
            return Some(Err(err));
        });
        if !epi.params.is_empty() {
            let attrs = guard_ok!(itp_cell.attr().err(), err => {
                return Some(Err(err));
            });
            for (i, param) in epi.params.iter().enumerate() {
                if let Some(n) = &param.name {
                    if attrs.get(n).err().is_err() {
                        let label = OwnValue::from(n.clone());
                        let newcell = guard_ok!(attrs.create(Some(label), Some(param.value.clone())), err => {
                            return Some(Err(err));
                        });
                        guard_ok!(attrs.add(None, newcell), err => {
                            return Some(Err(err));
                        });
                    } else {
                        guard_ok!(attrs.get(n).write().value(param.value.clone()), err => {
                            return Some(Err(err));
                        });
                    }
                } else {
                    let newcell = guard_ok!(attrs.create(Some(OwnValue::from(i)), Some(param.value.clone())), err => {
                        return Some(Err(err));
                    });
                    guard_ok!(attrs.add(None, newcell), err => {
                        return Some(Err(err));
                    });
                }
            }
        }
        let cell = guard_ok!(itp_cell.sub().at(0).err(), err => {
            // After `*` or `**`, some candidates may not expose an elevation child; skip them.
            if filter_eval|| (path_index > 0 && matches!(
                    &path[path_index - 1],
                    PathItem::Normal(NormalPathItem {
                        selector: Some(Selector::Star | Selector::DoubleStar),
                        ..
                    })
                ))
            {
                return None;
            }

            return Some(Err(err));
        });

        ifdebug!(println!(
            "match, push (elevation): `{}` : {:?}",
            cell.debug_string(),
            path_index + 1
        ));
        stack.push(MatchTest {
            parent: cell.clone(),
            path_index: path_index + 1,
        });
        Self::update_next_max_path_index(stack, next_max_path_index);
        None
    }

    fn process_group(
        stack: &mut Vec<MatchTest>,
        path: &[PathItem],
        parent: &Xell,
        group: Group,
        path_index: usize,
        next_max_path_index: &mut usize,
    ) {
        let pi = match &path[path_index] {
            PathItem::Elevation(_) => panic!("elevation path item unexpected here"),
            PathItem::Normal(npi) => npi,
        };
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
                let at_index = if index < 0 {
                    let len = group.len().unwrap_or_else(|e| {
                        warning!("Error while searching: cannot get group length: {:?}", e);
                        0
                    });
                    (len as isize + index) as usize
                } else {
                    index as usize
                };
                let cell = guard_ok!(group.at(at_index).err(), err => {
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
                    let opt_cell = if index < 0 {
                        iter.rev().nth((-index - 1) as usize)
                    } else {
                        iter.nth(index as usize)
                    };
                    if let Some(cell) = opt_cell {
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
        }
    }

    fn process_cell(
        stack: &mut Vec<MatchTest>,
        path: &[PathItem],
        cell: Xell,
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

        let pi = match &path[path_index] {
            PathItem::Elevation(_) => panic!("elevation path item not expected here"),
            PathItem::Normal(npi) => npi,
        };

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

    fn eval_filters_match(subcell: &Xell, path_item: &NormalPathItem) -> bool {
        for filter in &path_item.filters {
            match Searcher::eval_expression(subcell.clone(), &filter.expr) {
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

    fn eval_expression(cell: Xell, expr: &Expression<'s>) -> Res<bool> {
        match expr {
            Expression::Ternary { left, op_right } => {
                Self::eval_ternary_expression(cell, left.clone(), op_right)
            }
            Expression::Type { ty } => cell.read().ty().map(|t| t == *ty),
            Expression::Or { expressions } => {
                for expr in expressions {
                    if Self::eval_expression(cell.clone(), expr)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    fn eval_ternary_expression(
        cell: Xell,
        left: Path<'s>,
        op_right: &Option<(&'s str, OwnValue)>,
    ) -> Res<bool> {
        ifdebug!(println!(
            "{{{{\neval_ternary_expression cell `{}` for expr `{}`",
            cell.debug_string(),
            expr
        ));

        fn eval_expr(op: &str, left: Value, right: &OwnValue) -> Res<bool> {
            if !["==", "!="].contains(&op) {
                return userres(format!("bad operand: {}", op));
            }
            match op {
                "==" if left == right.as_value() => Ok(true),
                "!=" if left != right.as_value() => Ok(true),
                _ => Ok(false),
            }
        }

        let eval_iter_left = Self::new_with_filter_eval(cell, left, true);
        for cell in eval_iter_left {
            let cell = guard_ok!(cell, err => {
                debug_err!(err);
                continue;
            });
            if let Some((op, right)) = op_right {
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
    type Item = Res<Xell>;
    fn next(&mut self) -> Option<Res<Xell>> {
        self.eval_next()
    }
}
