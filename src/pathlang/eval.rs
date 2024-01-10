use std::collections::HashSet;

use crate::{
    base::*,
    debug, debug_err, guard_ok, guard_some,
    pathlang::{
        path::{Expression, PathItem},
        Path,
    },
};

macro_rules! eval_debug {
    (
        $body:block
    ) => {
        // $body
    };
}

#[derive(Clone, Debug)]
pub struct EvalIter<'s> {
    path: Vec<PathItem<'s>>,
    stack: Vec<CellNode>,
}

#[derive(Clone, Debug)]
pub struct CellNode {
    cell: Res<Cell>,
    path_indices: HashSet<usize>,
}

impl<'s> EvalIter<'s> {
    pub(crate) fn new(start: Cell, path: Path<'s>) -> EvalIter<'s> {
        eval_debug!({
            println!("\n********************************");
            println!("==> path is: {}\n", path)
        });
        let mut path_indices = HashSet::from([0]);
        if Self::is_doublestar_match(&start, 0, &path.0) {
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
        }
    }

    fn is_doublestar_match(cell: &Cell, path_index: usize, path: &[PathItem<'s>]) -> bool {
        eval_debug!({
            // println!(
            //     "is_doublestar_match: {} index {}",
            //     cell.debug_string(),
            //     path_index
            // )
        });
        if let Some(path_item) = path.get(path_index) {
            if let Some(Selector::DoubleStar) = path_item.selector {
                if EvalIter::eval_filters_match(cell, path_item) {
                    return true;
                }
            }
        }
        false
    }

    fn eval_next(&mut self) -> Option<Res<Cell>> {
        while !self.stack.is_empty() {
            if let Some(cell) = self.pump() {
                return Some(cell);
            }
        }
        None
    }

    fn pump(&mut self) -> Option<Res<Cell>> {
        eval_debug!({
            println!("----");
            print!("stack:");
            for cn in &self.stack {
                print!(
                    "    {} : {:?}",
                    cn.cell.as_ref().unwrap().debug_string(),
                    cn.path_indices
                );
            }
            println!();
        });

        let CellNode {
            cell,
            mut path_indices,
        } = guard_some!(self.stack.pop(), {
            return None;
        });
        let cell = guard_ok!(cell, err => {
            debug_err!(err);
            return None;
        });

        eval_debug!({
            println!("pump:     {} : {:?}", cell.debug_string(), path_indices);
        });

        if path_indices.iter().any(|i| *i >= self.path.len()) {
            path_indices.retain(|i| *i < self.path.len());
            eval_debug!({
                println!(
                    "found result: {};    push back with indices = {:?}",
                    cell.debug_string(),
                    path_indices
                );
            });
            self.stack.push(CellNode {
                cell: Ok(cell.clone()),
                path_indices,
            });
            return Some(Ok(cell));
        }

        let has_relation =
            |r, path: &[PathItem<'s>]| path_indices.iter().any(|i| path[*i].relation == r);
        if has_relation(Relation::Interpretation, &self.path) {
            match Self::subgroup(Relation::Interpretation, &cell) {
                Err(err) => debug_err!(err),
                Ok(group) => {
                    Self::push_interpretations(&group, &path_indices, &self.path, &mut self.stack)
                }
            }
        }

        // output order is reverse: field first, attr second, subs last
        for relation in [Relation::Sub, Relation::Attr, Relation::Field] {
            if has_relation(relation, &self.path) {
                let (star, doublestar) = Self::has_stars(relation, &path_indices, &self.path);
                match Self::subgroup(relation, &cell) {
                    Err(err) => debug_err!(err),
                    Ok(group) => {
                        if star || doublestar {
                            Self::push_by_relation_with_stars(
                                relation,
                                &group,
                                &path_indices,
                                &self.path,
                                &mut self.stack,
                                doublestar,
                            );
                        } else {
                            Self::push_by_relation_without_stars(
                                relation,
                                &group,
                                &path_indices,
                                &self.path,
                                &mut self.stack,
                            )
                        }
                    }
                }
            }
        }

        if has_relation(Relation::Field, &self.path) {
            match Self::subgroup(Relation::Field, &cell) {
                Err(err) => debug_err!(err),
                Ok(group) => Self::push_field(&group, &path_indices, &self.path, &mut self.stack),
            }
        }

        None
    }

    fn push_interpretations(
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            if path_item.relation != Relation::Interpretation {
                continue;
            }
            if let Some(interpretation) = path_item.selector {
                let subcell = guard_ok!(group.get(interpretation).err(), err => {
                    debug_err!(err);
                    continue;
                });

                if !Self::eval_filters_match(&subcell, path_item) {
                    continue;
                }

                eval_debug!({
                    println!(
                        "push interpretation: {} : pathindex={}",
                        subcell.debug_string(),
                        path_index + 1
                    );
                });
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: HashSet::from([path_index + 1]),
                });
            }
        }
    }

    fn push_by_relation_with_stars(
        relation: Relation,
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
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
                    eval_debug!({
                        println!("match: {} for {}", subcell.debug_string(), path_item);
                    });
                    if double_stars {
                        accepted_path_indices.insert(*path_index);
                    }
                    accepted_path_indices.insert(*path_index + 1);
                    if Self::is_doublestar_match(&subcell, *path_index + 1, path) {
                        accepted_path_indices.insert(*path_index + 2);
                    }
                } else {
                    eval_debug!({
                        println!("no match {} for {}", subcell.debug_string(), path_item);
                    });
                }
            }
            if !accepted_path_indices.is_empty() {
                eval_debug!({
                    println!(
                        "push by star relation: {} : {:?}",
                        subcell.debug_string(),
                        accepted_path_indices
                    );
                });
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: accepted_path_indices,
                });
            }
        }
    }

    fn push_by_relation_without_stars(
        relation: Relation,
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            let mut accepted_path_indices = HashSet::new();
            if relation != path_item.relation {
                continue;
            }
            let subcell_res = if let Some(sel) = path_item.selector {
                group.get(sel).err()
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
                if Self::is_doublestar_match(&subcell, *path_index + 1, path) {
                    accepted_path_indices.insert(*path_index + 2);
                }
            }
            if !accepted_path_indices.is_empty() {
                eval_debug!({
                    println!(
                        "push by non-star relation: {} : {:?}",
                        subcell.debug_string(),
                        accepted_path_indices
                    );
                });
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: accepted_path_indices,
                });
            }
        }
    }

    fn push_field(
        group: &Group,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
        stack: &mut Vec<CellNode>,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            if path_item.relation != Relation::Field {
                continue;
            }
            if let Some(field) = path_item.selector {
                let subcell = guard_ok!(group.get(field).err(), err => {
                    debug_err!(err);
                    continue;
                });

                eval_debug!({
                    println!(
                        "push by field: {} : {:?}",
                        subcell.debug_string(),
                        path_index + 1
                    );
                });
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: HashSet::from([path_index + 1]),
                });
            }
        }
    }

    fn subgroup(relation: Relation, cell: &Cell) -> Res<Group> {
        match relation {
            Relation::Sub => cell.sub().err(),
            Relation::Attr => cell.attr().err(),
            Relation::Interpretation => cell.elevate().err(),
            Relation::Field => cell.field(),
        }
    }

    fn has_stars(
        relation: Relation,
        path_indices: &HashSet<usize>,
        path: &[PathItem<'s>],
    ) -> (bool, bool) {
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
                    eval_debug!({
                        println!("verbose eval filter match ERROR");
                    });
                    debug_err!(e);
                    return false;
                }
                Ok(false) => {
                    eval_debug!({
                        println!("verbose eval filter match FALSE");
                    });
                    return false;
                }
                Ok(true) => {}
            }
        }
        eval_debug!({
            println!("eval filter match test is TRUE: {}", subcell.debug_string());
        });
        true
    }

    fn cell_matches_selector(cell: &Cell, sel: &Selector) -> bool {
        eval_debug!({
            println!("cell_matches_selector: selector {:?}; cell {:?}", sel, cell);
        });
        if *sel == Selector::Star || *sel == Selector::DoubleStar {
            return true;
        } else {
            match cell.read().err() {
                Ok(reader) => match reader.label() {
                    Ok(ref k) => {
                        if sel == k {
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
                        return Ok(true);
                    }
                }
            } else {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn eval_expr(op: &str, left: Value, right: Value) -> Res<bool> {
        if !["==", "!="].contains(&op) {
            return Err(HErr::Other(format!("bad operand: {}", op)));
        }
        match op {
            "==" if left == right => Ok(true),
            "!=" if left != right => Ok(true),
            _ => Ok(false),
        }
    }
}

impl<'s> Iterator for EvalIter<'s> {
    type Item = Res<Cell>;
    fn next(&mut self) -> Option<Res<Cell>> {
        self.eval_next()
    }
}
