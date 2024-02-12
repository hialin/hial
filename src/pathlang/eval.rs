use std::collections::HashSet;

use crate::{
    base::*,
    debug, debug_err, guard_ok, guard_some,
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
pub struct EvalIter<'s> {
    path: Vec<PathItem<'s>>,
    // dfs exploration of the cell tree in search of the path
    stack: Vec<CellNode>,
    next_max_path_index: usize,
}

// a cell that matches the search together with the indices of the path that it matches
// one cell can match multiple indices of the same path (because of doublestars)
#[derive(Clone, Debug)]
pub struct CellNode {
    cell: Res<Cell>,
    path_indices: HashSet<usize>,
}

impl<'s> EvalIter<'s> {
    pub(crate) fn new(start: Cell, path: Path<'s>) -> EvalIter<'s> {
        ifdebug!(println!(
            "\nnew EvalIter, path: {:?}:",
            path.0
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
        ));
        let mut path_indices = HashSet::from([0]);
        if Self::is_doublestar_match(&start, path.0.first()) {
            path_indices.insert(1);
        }
        let start_node = CellNode {
            cell: Ok(start),
            path_indices,
        };
        let stack = vec![start_node];
        EvalIter {
            path: path.0,
            stack,
            next_max_path_index: 0,
        }
    }

    fn eval_next(&mut self) -> Option<Res<Cell>> {
        while !self.stack.is_empty() {
            if let Some(cell) = self.pump() {
                Self::update_next_max_path_index(&self.stack, &mut self.next_max_path_index);
                return Some(cell);
            }
        }
        None
    }

    fn pump(&mut self) -> Option<Res<Cell>> {
        ifdebug!(println!(
            "----\nstack:{}",
            self.stack
                .iter()
                .map(|cn| format!(
                    "    `{}` : {:?}",
                    cn.cell.as_ref().unwrap().debug_string(),
                    cn.path_indices
                ))
                .collect::<Vec<String>>()
                .join("\n")
        ));

        // pop the last cell match from the stack
        let CellNode {
            cell,
            mut path_indices,
        } = guard_some!(self.stack.pop(), { return None });
        let cell = guard_ok!(cell, err => {
            debug_err!(err);
            return None;
        });

        ifdebug!(println!(
            "now testing:     `{}` : {:?}",
            cell.debug_string(),
            path_indices
        ));

        // if there are associated path indices larger than the path length,
        // it means that the cell is a match and we should return it
        if path_indices.iter().any(|i| *i >= self.path.len()) {
            ifdebug!(println!(
                "found result: `{}`;    push back with indices = {:?}",
                cell.debug_string(),
                path_indices
            ));

            // keep smaller path indices for further exploration
            path_indices.retain(|i| *i < self.path.len());
            if !path_indices.is_empty() {
                self.stack.push(CellNode {
                    cell: Ok(cell.clone()),
                    path_indices,
                });
                Self::update_next_max_path_index(&self.stack, &mut self.next_max_path_index);
            }

            return Some(Ok(cell));
        }

        let has_relation =
            |r, path: &[PathItem<'s>]| path_indices.iter().any(|i| path[*i].relation == r);

        if has_relation(Relation::Interpretation, &self.path) {
            match cell.elevate().err() {
                Err(err) => debug_err!(err),
                Ok(group) => Self::push_interpretations(
                    &group,
                    &path_indices,
                    &self.path,
                    &mut self.stack,
                    &mut self.next_max_path_index,
                ),
            }
        }

        // output order is reverse: field first, attr second, subs last
        // because we put them in a stack
        for relation in [Relation::Sub, Relation::Attr, Relation::Field] {
            if !has_relation(relation, &self.path) {
                continue;
            }

            let (star, doublestar) = Self::has_stars(relation, &path_indices, &self.path);
            let group = guard_ok!(Self::subgroup(relation, &cell), err => {
                debug_err!(err);
                continue;
            });

            if star || doublestar {
                Self::push_by_relation_with_stars(
                    relation,
                    &group,
                    &path_indices,
                    &self.path,
                    &mut self.stack,
                    &mut self.next_max_path_index,
                    doublestar,
                );
            } else {
                Self::push_by_relation_without_stars(
                    relation,
                    &group,
                    &path_indices,
                    &self.path,
                    &mut self.stack,
                    &mut self.next_max_path_index,
                )
            }
        }

        if has_relation(Relation::Field, &self.path) {
            match Self::subgroup(Relation::Field, &cell) {
                Err(err) => debug_err!(err),
                Ok(group) => Self::push_field(
                    &group,
                    &path_indices,
                    &self.path,
                    &mut self.stack,
                    &mut self.next_max_path_index,
                ),
            }
        }

        None
    }

    fn push_interpretations(
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
        next_max_path_index: &mut usize,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            if path_item.relation != Relation::Interpretation {
                continue;
            }
            if let Some(interpretation) = path_item.selector {
                let interpretation = match interpretation {
                    Selector::Star | Selector::DoubleStar => {
                        warning!("star or doublestar in interpretation");
                        Value::None
                    }
                    Selector::Top => Value::None,
                    Selector::Str(s) => Value::Str(s),
                };
                let subcell = guard_ok!(group.get(interpretation).err(), err => {
                    debug_err!(err);
                    continue;
                });

                if !Self::eval_filters_match(&subcell, path_item) {
                    continue;
                }

                ifdebug!(println!(
                    "push interpretation: `{}` : pathindex={}",
                    subcell.debug_string(),
                    path_index + 1
                ));
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: HashSet::from([path_index + 1]),
                });
                Self::update_next_max_path_index(stack, next_max_path_index);
            }
        }
    }

    fn push_by_relation_with_stars(
        relation: Relation,
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
        next_max_path_index: &mut usize,
        double_stars: bool,
    ) {
        let len = group.len().unwrap_or(0);
        for i in (0..len).rev() {
            let mut accepted_path_indices = HashSet::new();
            let subcell = guard_ok!(group.at(i).err(), err => {
                debug_err!(err);
                continue;
            });
            for path_index in path_indices {
                let path_item = &path[*path_index];

                if relation != path_item.relation {
                    continue;
                }
                if Self::accept_subcell(subcell.clone(), path_item) {
                    ifdebug!(println!(
                        "match: `{}` for {}",
                        subcell.debug_string(),
                        path_item
                    ));
                    if double_stars {
                        accepted_path_indices.insert(*path_index);
                    }
                    accepted_path_indices.insert(*path_index + 1);
                    if Self::is_doublestar_match(&subcell, path.get(*path_index + 1)) {
                        accepted_path_indices.insert(*path_index + 2);
                    }
                } else {
                    ifdebug!(println!(
                        "no match `{}` for {}",
                        subcell.debug_string(),
                        path_item
                    ));
                }
            }
            if !accepted_path_indices.is_empty() {
                ifdebug!(println!(
                    "push by star relation: `{}` : {:?}",
                    subcell.debug_string(),
                    accepted_path_indices
                ));
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: accepted_path_indices,
                });
                Self::update_next_max_path_index(stack, next_max_path_index);
            }
        }
    }

    fn is_doublestar_match(cell: &Cell, path_item: Option<&PathItem<'s>>) -> bool {
        if let Some(path_item) = path_item {
            if let Some(Selector::DoubleStar) = path_item.selector {
                if EvalIter::eval_filters_match(cell, path_item) {
                    return true;
                }
            }
        }
        false
    }

    fn push_by_relation_without_stars(
        relation: Relation,
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
        next_max_path_index: &mut usize,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            let mut accepted_path_indices = HashSet::new();
            if relation != path_item.relation {
                continue;
            }
            let subcell_res = if let Some(sel) = path_item.selector {
                let key = match sel {
                    Selector::Star | Selector::DoubleStar => {
                        warning!("star or doublestar in relation");
                        Value::None
                    }
                    Selector::Top => Value::None,
                    Selector::Str(s) => Value::Str(s),
                };

                group.get(key).err()
            } else if let Some(idx) = path_item.index {
                group.at(idx).err()
            } else {
                debug!(
                    "error: empty selector and index in path: {:?}",
                    path_item.selector
                );
                continue;
            };
            let subcell = guard_ok!(subcell_res, err => {
                debug_err!(err);
                continue;
            });
            if Self::accept_subcell(subcell.clone(), path_item) {
                accepted_path_indices.insert(path_index + 1);
                if Self::is_doublestar_match(&subcell, path.get(*path_index + 1)) {
                    accepted_path_indices.insert(*path_index + 2);
                }
            }
            if !accepted_path_indices.is_empty() {
                ifdebug!(println!(
                    "push by non-star relation: `{}` : {:?}",
                    subcell.debug_string(),
                    accepted_path_indices
                ));
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: accepted_path_indices,
                });
                Self::update_next_max_path_index(stack, next_max_path_index);
            }
        }
    }

    fn push_field(
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
        next_max_path_index: &mut usize,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            if path_item.relation != Relation::Field {
                continue;
            }
            if let Some(field_selector) = path_item.selector {
                let key = match field_selector {
                    Selector::Str(s) => Value::Str(s),
                    _ => {
                        warning!("star or doublestar or top in field relation");
                        Value::None
                    }
                };
                let subcell = guard_ok!(group.get(key).err(), err => {
                    debug_err!(err);
                    continue;
                });

                ifdebug!(println!(
                    "push by field: `{}` : {:?}",
                    subcell.debug_string(),
                    path_index + 1
                ));
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: HashSet::from([path_index + 1]),
                });
                Self::update_next_max_path_index(stack, next_max_path_index);
            }
        }
    }

    fn subgroup(relation: Relation, cell: &Cell) -> Res<Group> {
        match relation {
            Relation::Sub => cell.sub().err(),
            Relation::Attr => cell.attr().err(),
            Relation::Interpretation => cell.elevate().err(),
            Relation::Field => cell.field().err(),
        }
    }

    fn has_stars(
        relation: Relation,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
    ) -> (bool, bool) {
        path_indices
            .iter()
            .any(|i| path[*i].relation == relation && path[*i].selector == Some(Selector::Star));
        let (mut star, mut doublestar) = (false, false);
        for i in path_indices {
            if relation == path[*i].relation {
                if path[*i].selector == Some(Selector::Star) {
                    star = true;
                }
                if path[*i].selector == Some(Selector::DoubleStar) {
                    doublestar = true;
                }
            }
        }
        (star, doublestar)
    }

    fn accept_subcell(subcell: Cell, path_item: &PathItem) -> bool {
        if let Some(selector) = path_item.selector {
            if !EvalIter::cell_matches_selector(&subcell, &selector) {
                return false;
            }
        } else if let Some(index) = path_item.index {
            match subcell.read().err() {
                Ok(reader) => match reader.index() {
                    Ok(cellindex) => {
                        if index != cellindex {
                            return false;
                        }
                    }
                    Err(e) => {
                        debug_err!(e);
                        return false;
                    }
                },
                Err(e) => {
                    debug_err!(e);
                    return false;
                }
            }
        }

        EvalIter::eval_filters_match(&subcell, path_item)
    }

    fn eval_filters_match(subcell: &Cell, path_item: &PathItem) -> bool {
        for filter in &path_item.filters {
            match EvalIter::eval_bool_expression(subcell.clone(), &filter.expr) {
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

    fn cell_matches_selector(cell: &Cell, sel: &Selector) -> bool {
        if *sel == Selector::Star || *sel == Selector::DoubleStar {
            ifdebug!(println!(
                "cell `{}` matches star selector",
                cell.debug_string()
            ));
            return true;
        } else {
            match cell.read().err() {
                Ok(reader) => match reader.label() {
                    Ok(ref k) => {
                        if sel == k {
                            ifdebug!(println!(
                                "cell `{}` matches selector {} ",
                                cell.debug_string(),
                                sel,
                            ));
                            return true;
                        }
                    }
                    Err(e) => debug_err!(e),
                },
                Err(e) => debug_err!(e),
            }
        }
        false
    }

    fn eval_bool_expression(cell: Cell, expr: &Expression<'s>) -> Res<bool> {
        ifdebug!(println!(
            "{{{{\neval_bool_expression cell `{}` for expr `{}`",
            cell.debug_string(),
            expr
        ));
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
                    if Self::eval_expr(op, lvalue, right)? {
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

    fn update_next_max_path_index(stack: &[CellNode], next_max_path_index: &mut usize) {
        let max_i = stack
            .iter()
            .map(|cn| cn.path_indices.iter().copied().max().unwrap_or(0))
            .max()
            .unwrap_or(0);
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

impl<'s> Iterator for EvalIter<'s> {
    type Item = Res<Cell>;
    fn next(&mut self) -> Option<Res<Cell>> {
        self.eval_next()
    }
}
