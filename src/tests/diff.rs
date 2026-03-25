use crate::{api::*, interpretations::diff::NodeSpec};

#[test]
fn diff_tree_supports_normal_traversal() -> Res<()> {
    let diff = crate::interpretations::diff::Cell::from_node(
        NodeSpec::new("object")
            .diff_changed()
            .with_sub(NodeSpec::new("string").label("name").value("new value")),
    );

    assert_eq!(diff.interpretation(), "diff");
    assert_eq!(diff.read().ty()?, "object");
    assert_eq!(diff.attr().get("diff_changed").read().value()?, Value::None);

    let child = diff.sub().get("name");
    assert_eq!(child.read().ty()?, "string");
    assert_eq!(child.read().value()?, Value::Str("new value"));
    assert_eq!(child.head()?.1, Relation::Sub);

    Ok(())
}

#[test]
fn diff_tree_supports_replacement_pairs() -> Res<()> {
    let diff = crate::interpretations::diff::Cell::from_node(
        NodeSpec::new("object")
            .with_sub(NodeSpec::new("string").label("name").value("old value").diff_old())
            .with_sub(NodeSpec::new("number").label("title").value(7).diff_new()),
    );

    let children = diff.sub();
    assert_eq!(children.len()?, 2);

    let old_node = children.at(0);
    assert_eq!(old_node.read().label()?, Value::Str("name"));
    assert_eq!(old_node.read().ty()?, "string");
    assert_eq!(old_node.attr().get("diff_old").read().value()?, Value::None);

    let new_node = children.at(1);
    assert_eq!(new_node.read().label()?, Value::Str("title"));
    assert_eq!(new_node.read().ty()?, "number");
    assert_eq!(new_node.attr().get("diff_new").read().value()?, Value::None);

    Ok(())
}

#[test]
fn diff_tree_supports_old_value_annotation() -> Res<()> {
    let diff = crate::interpretations::diff::Cell::from_node(
        NodeSpec::new("object").with_sub(
            NodeSpec::new("string")
                .label("name")
                .value("new value")
                .diff_old_value("old value"),
        ),
    );

    let child = diff.sub().get("name");
    assert_eq!(child.read().value()?, Value::Str("new value"));
    assert_eq!(
        child.attr().get("diff_old_value").read().value()?,
        Value::Str("old value")
    );

    Ok(())
}

#[test]
fn diff_scalar_nodes_keep_unchanged_scalars() -> Res<()> {
    let left = Xell::from("same");
    let right = Xell::from("same");

    let diff = crate::interpretations::diff::Cell::from_nodes(
        crate::interpretations::diff::diff_scalar_nodes(&left, &right)?,
    );

    let child = diff.sub().at(0);
    assert_eq!(child.read().ty()?, "value");
    assert_eq!(child.read().value()?, Value::Str("same"));
    assert!(child.attr().is_empty());

    Ok(())
}

#[test]
fn diff_scalar_nodes_annotate_value_changes() -> Res<()> {
    let left = Xell::from("old");
    let right = Xell::from("new");

    let diff = crate::interpretations::diff::Cell::from_nodes(
        crate::interpretations::diff::diff_scalar_nodes(&left, &right)?,
    );

    let child = diff.sub().at(0);
    assert_eq!(child.read().value()?, Value::Str("new"));
    assert_eq!(
        child.attr().get("diff_old_value").read().value()?,
        Value::Str("old")
    );

    Ok(())
}

#[test]
fn diff_scalar_nodes_emit_replacement_pairs_for_type_changes() -> Res<()> {
    let left = Xell::from("7").be("json");
    let right = Xell::from("7");

    let diff = crate::interpretations::diff::Cell::from_nodes(
        crate::interpretations::diff::diff_scalar_nodes(&left, &right)?,
    );

    let children = diff.sub();
    assert_eq!(children.len()?, 2);
    assert_eq!(children.at(0).attr().get("diff_old").read().value()?, Value::None);
    assert_eq!(children.at(1).attr().get("diff_new").read().value()?, Value::None);
    assert_eq!(children.at(0).read().value()?, Value::Int(7usize.into()));
    assert_eq!(children.at(1).read().value()?, Value::Str("7"));

    Ok(())
}

#[test]
fn diff_one_level_marks_parent_when_labeled_child_value_changes() -> Res<()> {
    let left = Xell::from(r#"{"name":"old","same":1}"#).be("json");
    let right = Xell::from(r#"{"name":"new","same":1}"#).be("json");

    let diff = crate::interpretations::diff::Cell::from_nodes(
        crate::interpretations::diff::diff_one_level(&left, &right)?,
    );

    let root = diff.sub().at(0);
    assert_eq!(root.read().ty()?, "object");
    assert_eq!(root.attr().get("diff_changed").read().value()?, Value::None);
    assert_eq!(
        root.sub().get("name").attr().get("diff_old_value").read().value()?,
        Value::Str("old")
    );
    assert!(root.sub().get("same").attr().is_empty());

    Ok(())
}

#[test]
fn diff_one_level_marks_parent_when_indexed_child_changes() -> Res<()> {
    let left = Xell::from(r#"[1,2]"#).be("json");
    let right = Xell::from(r#"[1,3,4]"#).be("json");

    let diff = crate::interpretations::diff::Cell::from_nodes(
        crate::interpretations::diff::diff_one_level(&left, &right)?,
    );

    let root = diff.sub().at(0);
    let items = root.sub();
    assert_eq!(root.attr().get("diff_changed").read().value()?, Value::None);
    assert_eq!(items.len()?, 3);
    assert!(items.at(0).attr().is_empty());
    assert_eq!(
        items.at(1).attr().get("diff_old_value").read().value()?,
        Value::Int(2usize.into())
    );
    assert_eq!(items.at(2).attr().get("diff_new").read().value()?, Value::None);

    Ok(())
}

#[test]
fn xell_diff_returns_queryable_diff_tree() -> Res<()> {
    let left = Xell::from(r#"{"name":"old","same":1}"#).be("json");
    let right = Xell::from(r#"{"name":"new","same":1}"#).be("json");

    let diff = left.diff(&right)?;
    let root = diff.sub().at(0);

    assert_eq!(diff.interpretation(), "diff");
    assert_eq!(root.read().ty()?, "object");
    assert_eq!(root.attr().get("diff_changed").read().value()?, Value::None);
    assert_eq!(
        diff.to("/[0]/name@diff_old_value").read().value()?,
        Value::Str("old")
    );

    Ok(())
}

#[test]
fn xell_diff_returns_replacement_pairs_for_type_changes() -> Res<()> {
    let left = Xell::from("7").be("json");
    let right = Xell::from("7");

    let diff = left.diff(&right)?;
    let children = diff.sub();

    assert_eq!(children.len()?, 2);
    assert_eq!(children.at(0).attr().get("diff_old").read().value()?, Value::None);
    assert_eq!(children.at(1).attr().get("diff_new").read().value()?, Value::None);

    Ok(())
}
