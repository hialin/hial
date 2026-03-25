mod myers;

use std::rc::Rc;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    tree: Rc<Tree>,
    node_id: usize,
}

#[derive(Debug)]
pub(crate) struct CellReader {
    tree: Rc<Tree>,
    node_id: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    tree: Rc<Tree>,
    parent_id: usize,
    relation: Relation,
}

#[derive(Clone, Debug)]
struct Tree {
    nodes: Vec<Node>,
}

#[derive(Clone, Debug)]
struct Node {
    ty: String,
    label: Option<OwnValue>,
    value: Option<OwnValue>,
    parent: Option<(usize, Relation)>,
    index: usize,
    sub: Vec<usize>,
    attr: Vec<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct NodeSpec {
    ty: String,
    label: Option<OwnValue>,
    value: Option<OwnValue>,
    sub: Vec<NodeSpec>,
    attr: Vec<NodeSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChildSignature {
    ty: String,
    label: Option<OwnValue>,
}

implement_try_from_xell!(Cell, Diff);

impl NodeSpec {
    pub(crate) fn new(ty: impl Into<String>) -> Self {
        Self {
            ty: ty.into(),
            label: None,
            value: None,
            sub: vec![],
            attr: vec![],
        }
    }

    pub(crate) fn label(mut self, label: impl Into<OwnValue>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub(crate) fn value(mut self, value: impl Into<OwnValue>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub(crate) fn with_sub(mut self, child: NodeSpec) -> Self {
        self.sub.push(child);
        self
    }

    pub(crate) fn with_attr(mut self, child: NodeSpec) -> Self {
        self.attr.push(child);
        self
    }

    pub(crate) fn diff_changed(self) -> Self {
        self.with_diff_marker("diff_changed")
    }

    pub(crate) fn diff_old(self) -> Self {
        self.with_diff_marker("diff_old")
    }

    pub(crate) fn diff_new(self) -> Self {
        self.with_diff_marker("diff_new")
    }

    pub(crate) fn diff_old_value(mut self, old_value: impl Into<OwnValue>) -> Self {
        self.attr.push(
            NodeSpec::new("value")
                .label("diff_old_value")
                .value(old_value.into()),
        );
        self
    }

    fn with_diff_marker(mut self, label: &'static str) -> Self {
        self.attr
            .push(NodeSpec::new("value").label(label).value(OwnValue::None));
        self
    }
}

impl Cell {
    pub(crate) fn from_node(node: NodeSpec) -> Xell {
        let mut nodes = Vec::new();
        let root_id = append_node(&mut nodes, node, None, 0);
        debug_assert_eq!(root_id, 0);
        Xell::new_from(
            DynCell::from(Cell {
                tree: Rc::new(Tree { nodes }),
                node_id: root_id,
            }),
            None,
        )
    }

    pub(crate) fn from_nodes(nodes: Vec<NodeSpec>) -> Xell {
        let root = nodes
            .into_iter()
            .fold(NodeSpec::new("diff_root"), |root, node| root.with_sub(node));
        Self::from_node(root)
    }
}

pub(crate) fn diff_scalar_nodes(left: &Xell, right: &Xell) -> Res<Vec<NodeSpec>> {
    let left_reader = left.read().err()?;
    let right_reader = right.read().err()?;

    let left_ty = left_reader.ty()?;
    let right_ty = right_reader.ty()?;
    let left_label = reader_label(&left_reader)?;
    let right_label = reader_label(&right_reader)?;
    let left_value = reader_value(&left_reader)?;
    let right_value = reader_value(&right_reader)?;

    let left_node = snapshot_node(left_ty, left_label.clone(), left_value.clone());
    let right_node = snapshot_node(right_ty, right_label.clone(), right_value.clone());

    if left_ty != right_ty || left_label != right_label {
        return Ok(vec![left_node.diff_old(), right_node.diff_new()]);
    }

    if left_value != right_value {
        return Ok(vec![right_node.diff_old_value(
            left_value.unwrap_or(OwnValue::None),
        )]);
    }

    Ok(vec![right_node])
}

pub(crate) fn diff_one_level(left: &Xell, right: &Xell) -> Res<Vec<NodeSpec>> {
    diff_nodes(left, right)
}

fn snapshot_node(ty: &str, label: Option<OwnValue>, value: Option<OwnValue>) -> NodeSpec {
    let mut node = NodeSpec::new(ty.to_string());
    if let Some(label) = label {
        node = node.label(label);
    }
    if let Some(value) = value {
        node = node.value(value);
    }
    node
}

fn snapshot_xell_shallow(cell: &Xell) -> Res<NodeSpec> {
    let reader = cell.read().err()?;
    Ok(snapshot_node(
        reader.ty()?,
        reader_label(&reader)?,
        reader_value(&reader)?,
    ))
}

fn snapshot_xell_recursive(cell: &Xell) -> Res<NodeSpec> {
    let mut node = snapshot_xell_shallow(cell)?;
    node.sub = snapshot_group_recursive(&cell.sub())?;
    node.attr = snapshot_group_recursive(&cell.attr())?;
    Ok(node)
}

fn snapshot_group_recursive(group: &crate::api::Group) -> Res<Vec<NodeSpec>> {
    let mut items = Vec::with_capacity(group.len()?);
    for i in 0..group.len()? {
        items.push(snapshot_xell_recursive(&group.at(i))?);
    }
    Ok(items)
}

fn diff_nodes(left: &Xell, right: &Xell) -> Res<Vec<NodeSpec>> {
    let left_reader = left.read().err()?;
    let right_reader = right.read().err()?;

    let left_ty = left_reader.ty()?;
    let right_ty = right_reader.ty()?;
    let left_label = reader_label(&left_reader)?;
    let right_label = reader_label(&right_reader)?;

    if left_ty != right_ty || left_label != right_label {
        return Ok(vec![
            snapshot_xell_recursive(left)?.diff_old(),
            snapshot_xell_recursive(right)?.diff_new(),
        ]);
    }

    let left_value = reader_value(&left_reader)?;
    let right_value = reader_value(&right_reader)?;

    let mut node = snapshot_node(right_ty, right_label, right_value.clone());
    let mut marker_attrs = Vec::new();
    if left_value != right_value {
        marker_attrs.push(
            NodeSpec::new("value")
                .label("diff_old_value")
                .value(left_value.unwrap_or(OwnValue::None)),
        );
    }

    let mut changed = false;
    node.sub = diff_group_children(left, right, Relation::Sub, &mut changed)?;
    node.attr = marker_attrs;
    node.attr
        .extend(diff_group_children(left, right, Relation::Attr, &mut changed)?);
    if changed {
        node = node.diff_changed();
    }

    Ok(vec![node])
}

fn diff_group_children(
    left: &Xell,
    right: &Xell,
    relation: Relation,
    changed: &mut bool,
) -> Res<Vec<NodeSpec>> {
    let left_group = group_for_relation(left, relation);
    let right_group = group_for_relation(right, relation);
    let left_children = collect_children(&left_group)?;
    let right_children = collect_children(&right_group)?;
    let left_signatures = left_children
        .iter()
        .map(shallow_signature)
        .collect::<Res<Vec<_>>>()?;
    let right_signatures = right_children
        .iter()
        .map(shallow_signature)
        .collect::<Res<Vec<_>>>()?;

    let mut out = Vec::new();
    for op in myers::diff(&left_signatures, &right_signatures) {
        match op {
            myers::EditOp::Match(li, ri) => {
                let child_diff = diff_nodes(&left_children[li], &right_children[ri])?;
                if child_has_markers(&child_diff[0]) || child_diff.len() != 1 {
                    *changed = true;
                }
                out.extend(child_diff);
            }
            myers::EditOp::Delete(li) => {
                *changed = true;
                out.push(snapshot_xell_recursive(&left_children[li])?.diff_old());
            }
            myers::EditOp::Insert(ri) => {
                *changed = true;
                out.push(snapshot_xell_recursive(&right_children[ri])?.diff_new());
            }
        }
    }
    Ok(out)
}

fn group_for_relation(cell: &Xell, relation: Relation) -> crate::api::Group {
    match relation {
        Relation::Sub => cell.sub(),
        Relation::Attr => cell.attr(),
        Relation::Field | Relation::Interpretation => unreachable!("unsupported diff relation"),
    }
}

fn collect_children(group: &crate::api::Group) -> Res<Vec<Xell>> {
    let len = group.len()?;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        out.push(group.at(i));
    }
    Ok(out)
}

fn shallow_signature(cell: &Xell) -> Res<ChildSignature> {
    let reader = cell.read().err()?;
    Ok(ChildSignature {
        ty: reader.ty()?.to_string(),
        label: reader_label(&reader)?,
    })
}

fn child_has_markers(node: &NodeSpec) -> bool {
    has_diff_marker(node, "diff_old")
        || has_diff_marker(node, "diff_new")
        || has_diff_marker(node, "diff_old_value")
        || has_diff_marker(node, "diff_changed")
}

fn has_diff_marker(node: &NodeSpec, label: &str) -> bool {
    node.attr.iter().any(|attr| {
        attr.label
            .as_ref()
            .is_some_and(|node_label| node_label.as_value() == Value::Str(label))
    })
}

fn reader_label(reader: &crate::api::CellReader) -> Res<Option<OwnValue>> {
    match reader.label() {
        Ok(value) => Ok(Some(value.to_owned_value())),
        Err(err) if err.kind == HErrKind::None => Ok(None),
        Err(err) => Err(err),
    }
}

fn reader_value(reader: &crate::api::CellReader) -> Res<Option<OwnValue>> {
    match reader.value() {
        Ok(value) => Ok(Some(value.to_owned_value())),
        Err(err) if err.kind == HErrKind::None => Ok(None),
        Err(err) => Err(err),
    }
}

fn append_node(
    arena: &mut Vec<Node>,
    spec: NodeSpec,
    parent: Option<(usize, Relation)>,
    index: usize,
) -> usize {
    let node_id = arena.len();
    arena.push(Node {
        ty: spec.ty,
        label: spec.label,
        value: spec.value,
        parent,
        index,
        sub: vec![],
        attr: vec![],
    });

    let sub_ids = spec
        .sub
        .into_iter()
        .enumerate()
        .map(|(i, child)| append_node(arena, child, Some((node_id, Relation::Sub)), i))
        .collect();
    let attr_ids = spec
        .attr
        .into_iter()
        .enumerate()
        .map(|(i, child)| append_node(arena, child, Some((node_id, Relation::Attr)), i))
        .collect();

    arena[node_id].sub = sub_ids;
    arena[node_id].attr = attr_ids;
    node_id
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        Ok(&self.tree.nodes[self.node_id].ty)
    }

    fn index(&self) -> Res<usize> {
        Ok(self.tree.nodes[self.node_id].index)
    }

    fn label(&self) -> Res<Value<'_>> {
        self.tree.nodes[self.node_id]
            .label
            .as_ref()
            .map(OwnValue::as_value)
            .ok_or_else(noerr)
    }

    fn value(&self) -> Res<Value<'_>> {
        self.tree.nodes[self.node_id]
            .value
            .as_ref()
            .map(OwnValue::as_value)
            .ok_or_else(noerr)
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = Cell;

    fn interpretation(&self) -> &str {
        "diff"
    }

    fn read(&self) -> Res<CellReader> {
        Ok(CellReader {
            tree: Rc::clone(&self.tree),
            node_id: self.node_id,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        inputres("cannot write a diff cell")
    }

    fn sub(&self) -> Res<Self::Group> {
        Ok(Group {
            tree: Rc::clone(&self.tree),
            parent_id: self.node_id,
            relation: Relation::Sub,
        })
    }

    fn attr(&self) -> Res<Self::Group> {
        Ok(Group {
            tree: Rc::clone(&self.tree),
            parent_id: self.node_id,
            relation: Relation::Attr,
        })
    }

    fn head(&self) -> Res<(Self, Relation)> {
        let Some((parent_id, relation)) = self.tree.nodes[self.node_id].parent else {
            return nores();
        };
        Ok((
            Cell {
                tree: Rc::clone(&self.tree),
                node_id: parent_id,
            },
            relation,
        ))
    }
}

impl CellWriterTrait for Cell {
    fn set_value(&mut self, _: OwnValue) -> Res<()> {
        inputres("cannot write a diff cell")
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::vec::IntoIter<Res<Self::Cell>>;

    fn label_type(&self) -> LabelType {
        match self.relation {
            Relation::Attr => LabelType {
                is_indexed: true,
                unique_labels: true,
            },
            Relation::Sub => LabelType {
                is_indexed: true,
                unique_labels: false,
            },
            Relation::Field | Relation::Interpretation => LabelType::default(),
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(self.child_ids().len())
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        let Some(node_id) = self.child_ids().get(index).copied() else {
            return nores();
        };
        Ok(Cell {
            tree: Rc::clone(&self.tree),
            node_id,
        })
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        let items = self
            .child_ids()
            .iter()
            .copied()
            .filter(|node_id| {
                self.tree.nodes[*node_id]
                    .label
                    .as_ref()
                    .is_some_and(|node_label| node_label.as_value() == label)
            })
            .map(|node_id| {
                Ok(Cell {
                    tree: Rc::clone(&self.tree),
                    node_id,
                })
            })
            .collect::<Vec<_>>();
        Ok(items.into_iter())
    }
}

impl Group {
    fn child_ids(&self) -> &[usize] {
        let node = &self.tree.nodes[self.parent_id];
        match self.relation {
            Relation::Sub => &node.sub,
            Relation::Attr => &node.attr,
            Relation::Field | Relation::Interpretation => &[],
        }
    }
}
