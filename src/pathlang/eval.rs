use crate::pathlang::path::{PathItem, Relation};
use crate::{
    base::common::*,
    base::rust_api::*,
    guard_ok, guard_some,
    pathlang::{path::Expression, Path},
    verbose, verbose_error, InterpretationCell, InterpretationGroup,
};

#[derive(Clone, Debug)]
pub struct EvalIter<'a> {
    path: Path<'a>,
    stack: Vec<CellNode>,
}

#[derive(Clone, Debug)]
pub struct CellNode {
    cell: Res<Cell>,
    path_indices: Vec<usize>,
}

impl<'a> EvalIter<'a> {
    pub(crate) fn new(start: Cell, path: Path<'a>) -> EvalIter<'a> {
        let start_node = CellNode {
            cell: Ok(start),
            path_indices: vec![0],
        };
        let stack = vec![start_node];
        let eval_iter = EvalIter { path, stack };
        eval_iter
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
        let CellNode { cell, path_indices } = guard_some!(self.stack.pop(), {
            return None;
        });
        let cell = guard_ok!(cell, err => {
            verbose_error(err);
            return None;
        });

        if path_indices.iter().any(|i| *i >= self.path.0.len()) {
            return Some(Ok(cell));
        }

        let has_relation =
            |r, path: &Vec<PathItem<'a>>| path_indices.iter().any(|i| path[*i].relation == r);

        if has_relation(Relation::Interpretation, &self.path.0) {
            match Self::subgroup(Relation::Interpretation, &cell) {
                Err(err) => verbose_error(err),
                Ok(group) => {
                    Self::push_interpretations(&group, &path_indices, &self.path.0, &mut self.stack)
                }
            }
        }

        for relation in [Relation::Attr, Relation::Sub] {
            if has_relation(relation, &self.path.0) {
                match Self::subgroup(relation, &cell) {
                    Err(err) => verbose_error(err),
                    Ok(group) => {
                        let (star, doublestar) =
                            Self::has_stars(relation, &path_indices, &self.path.0);
                        if star || doublestar {
                            Self::push_by_relation_with_stars(
                                relation,
                                &group,
                                &path_indices,
                                &self.path.0,
                                &mut self.stack,
                                doublestar,
                            );
                        } else {
                            Self::push_by_relation_without_stars(
                                relation,
                                &group,
                                &path_indices,
                                &self.path.0,
                                &mut self.stack,
                            )
                        }
                    }
                }
            }
        }

        if has_relation(Relation::Field, &self.path.0) {
            match Self::subgroup(Relation::Field, &cell) {
                Err(err) => verbose_error(err),
                Ok(group) => Self::push_field(&group, &path_indices, &self.path.0, &mut self.stack),
            }
        }

        None
    }

    fn push_interpretations(
        group: &Group,
        path_indices: &Vec<usize>,
        path: &Vec<PathItem<'a>>,
        stack: &mut Vec<CellNode>,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            if path_item.relation != Relation::Interpretation {
                continue;
            }
            if let Some(interpretation) = path_item.selector {
                let subcell = guard_ok!(group.get(interpretation), err => {
                    verbose_error(err);
                    continue;
                });

                if !Self::eval_filters_match(&subcell, path_item) {
                    continue;
                }

                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: vec![path_index + 1],
                });
            }
        }
    }

    fn push_by_relation_with_stars(
        relation: Relation,
        group: &Group,
        path_indices: &Vec<usize>,
        path: &Vec<PathItem<'a>>,
        stack: &mut Vec<CellNode>,
        double_stars: bool,
    ) {
        // println!(
        //     "-- push_by_relation_with_stars: relation {:?}, double_stars {}\npath: {} ; path_indices: {:?}",
        //     relation, double_stars, DisplayPath(path), path_indices
        // );
        for i in (0..group.len()).rev() {
            let mut accepted_path_indices = vec![];
            let subcell = guard_ok!(group.at(i), err => {
                verbose_error(err);
                continue;
            });
            for path_index in path_indices {
                let path_item = &path[*path_index];

                if relation != path_item.relation {
                    continue;
                }
                if Self::accept_subcell(subcell.clone(), path_item) {
                    if double_stars {
                        accepted_path_indices.push(*path_index);
                    }
                    accepted_path_indices.push(*path_index + 1);
                }
            }
            if !accepted_path_indices.is_empty() {
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
        path_indices: &Vec<usize>,
        path: &Vec<PathItem<'a>>,
        stack: &mut Vec<CellNode>,
    ) {
        // println!(
        //     "-- push_by_relation_without_stars: relation {:?}\npath_indices: {:?}",
        //     relation, path_indices
        // );
        for path_index in path_indices {
            let path_item = &path[*path_index];
            let mut accepted_path_indices = vec![];
            if relation != path_item.relation {
                continue;
            }
            let subcell_res = if let Some(sel) = path_item.selector {
                group.get(sel)
            } else if let Some(idx) = path_item.index {
                group.at(idx)
            } else {
                verbose!(
                    "error: empty selector and index in path: {:?}",
                    path_item.selector
                );
                continue;
            };
            let subcell = guard_ok!(subcell_res, err => {
                verbose_error(err);
                continue;
            });
            if Self::accept_subcell(subcell.clone(), path_item) {
                accepted_path_indices.push(path_index + 1);
            }
            if !accepted_path_indices.is_empty() {
                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: accepted_path_indices,
                });
            }
        }
    }

    fn push_field(
        group: &Group,
        path_indices: &Vec<usize>,
        path: &Vec<PathItem<'a>>,
        stack: &mut Vec<CellNode>,
    ) {
        for path_index in path_indices {
            let path_item = &path[*path_index];
            if path_item.relation != Relation::Field {
                continue;
            }
            if let Some(field) = path_item.selector {
                let subcell = guard_ok!(group.get(field), err => {
                    verbose_error(err);
                    continue;
                });

                if !Self::eval_filters_match(&subcell, path_item) {
                    continue;
                }

                stack.push(CellNode {
                    cell: Ok(subcell),
                    path_indices: vec![path_index + 1],
                });
            }
        }
    }

    fn subgroup(relation: Relation, cell: &Cell) -> Res<Group> {
        match relation {
            Relation::Sub => cell.sub(),
            Relation::Attr => cell.attr(),
            Relation::Interpretation => cell.clone().elevate(),
            Relation::Field => cell.field(),
        }
    }

    fn has_stars(
        relation: Relation,
        path_indices: &Vec<usize>,
        path: &Vec<PathItem<'a>>,
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
            match subcell.index() {
                Ok(cellindex) => {
                    if index != cellindex {
                        return false;
                    }
                }
                Err(e) => {
                    verbose_error(e);
                    return false;
                }
            }
        }

        let res = EvalIter::eval_filters_match(&subcell, path_item);
        res
    }

    fn eval_filters_match(subcell: &Cell, path_item: &PathItem) -> bool {
        for filter in &path_item.filters {
            match EvalIter::eval_bool_expression(subcell.clone(), &filter.expr) {
                Err(e) => {
                    verbose_error(e);
                    return false;
                }
                Ok(false) => return false,
                Ok(true) => {}
            }
        }
        true
    }

    fn cell_matches_selector(cell: &Cell, sel: &Selector) -> bool {
        // println!("cell_matches_selector: selector {:?}; cell {:?}", sel, cell);
        if *sel == Selector::Star || *sel == Selector::DoubleStar {
            return true;
        } else {
            match cell.label() {
                Ok(ref k) => {
                    if sel == k {
                        return true;
                    }
                }
                Err(e) => verbose_error(e),
            }
        }
        false
    }

    fn eval_bool_expression(cell: Cell, expr: &Expression<'a>) -> Res<bool> {
        let eval_iter_left = Self::new(cell, expr.left.clone());
        for cell in eval_iter_left {
            let cell = guard_ok!(cell, err => {
                verbose_error(err);
                continue;
            });
            if let Some(op) = expr.op {
                if let Some(right) = expr.right {
                    let lvalue = guard_ok!(cell.value(), err => {
                        verbose_error(err);
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

impl<'a> Iterator for EvalIter<'a> {
    type Item = Res<Cell>;
    fn next(&mut self) -> Option<Res<Cell>> {
        self.eval_next()
    }
}
