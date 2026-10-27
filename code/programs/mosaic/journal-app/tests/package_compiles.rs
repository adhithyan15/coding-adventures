//! package_compiles — the Journal app package contract.
//!
//! The sources compile in both themes, the manifest parses, the components it
//! composes resolve, the timeline is drawn through `mosaic-pkg-card`'s Card,
//! the Rust app's props are exactly this package's slots, and it builds on all
//! eight backends.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use journal_mosaic_app::JournalMosaicApp;
use mosaic_app_runtime::{MosaicApp, Platform, StartContext};
use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use mosaic_package_resolver::{Resolution, Resolver};
use moslayout_compiler::{LayoutNode, LayoutPropValue};

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_source(name: &str) -> String {
    let path = package_root().join("src").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn resolver() -> Resolver {
    let packages = package_root().join("../../../packages");
    mosaic_package_resolver::build(&package_root(), &[packages.clone(), packages.join("mosaic")])
        .expect("Journal's dependencies resolve")
}

#[test]
fn manifest_declares_the_app_and_its_dependencies() {
    let manifest = mosaic_package_manifest::parse(
        &fs::read_to_string(package_root().join("mosaic-package.toml")).unwrap(),
    )
    .expect("manifest parses and validates");
    assert_eq!(manifest.package.name, "journal-app");
    assert_eq!(manifest.components.exports, ["JournalApp"]);
    assert!(manifest.dependencies.contains_key("mosaic-pkg-toolkit"));
    assert!(manifest.dependencies.contains_key("mosaic-pkg-draft-editor"));
}

/// The Card dependency names the version Card's own manifest declares, exactly
/// (version requirements are enforced by the resolver, so a stale pin fails
/// the build rather than drifting).
#[test]
fn the_card_dependency_is_cards_current_version() {
    let read = |path: PathBuf| {
        mosaic_package_manifest::parse(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{} parses: {e:?}", path.display()))
    };
    let journal = read(package_root().join("mosaic-package.toml"));
    let card = read(package_root().join("../../../packages/mosaic/mosaic-pkg-card/mosaic-package.toml"));
    assert_eq!(
        journal.dependencies.get("mosaic-pkg-card"),
        Some(&card.package.version),
        "journal-app must depend on mosaic-pkg-card at its current version"
    );
}

#[test]
fn sources_compile_in_both_themes() {
    let mil = mosmodel_compiler::compile(&read_source("JournalApp.mil")).expect("mil compiles");
    let mll = moslayout_compiler::compile(&read_source("JournalApp.mll"), Some(&mil.descriptor_json))
        .expect("mll compiles against the mil");
    for theme in ["light", "dark"] {
        mosstyle_compiler::compile(
            &read_source(&format!("JournalApp.{theme}.msl")),
            Some(&mll.part_map_json),
        )
        .unwrap_or_else(|e| panic!("{theme} msl compiles: {e:?}"));
    }
}

#[test]
fn the_composed_components_resolve() {
    let r = resolver();
    for (component, package) in [
        ("RecordList", "mosaic-pkg-toolkit"),
        ("EmptyState", "mosaic-pkg-toolkit"),
        ("Card", "mosaic-pkg-card"),
        ("DraftEditor", "mosaic-pkg-draft-editor"),
    ] {
        match r.resolve(component) {
            Some(Resolution::Component { package: p, .. }) => assert_eq!(p, package, "{component}"),
            other => panic!("expected {component} to resolve, got {other:?}"),
        }
    }
}

/// The pane scrolls (JO-5, journal-mosaic-app.md "The pane scrolls"). A native
/// window scrolls nothing unless the layout says so, and before this every
/// entry past the first two was below the fold of the 1100x760 window with no
/// way to reach it. The split's first child (its pane) must be a vertical
/// `HostScroll [ timeline-scroll ]` wrapping exactly the `timeline` column, so
/// the column's padding scrolls with it.
#[test]
fn the_pane_scrolls() {
    let mil = mosmodel_compiler::compile(&read_source("JournalApp.mil")).unwrap();
    let mll = moslayout_compiler::compile(&read_source("JournalApp.mll"), Some(&mil.descriptor_json))
        .unwrap();
    let split = &mll.def.root;
    assert_eq!(split.tag, "HostNavigationSplit");
    let pane = &split.children[0];
    assert_eq!(pane.tag, "HostScroll", "the pane is a scroll container");
    assert_eq!(pane.part_name.as_deref(), Some("timeline-scroll"));
    assert!(
        pane.props.iter().all(|p| p.name != "axis"),
        "no axis: the default, vertical, scroll ({:?})",
        pane.props
    );
    assert_eq!(pane.children.len(), 1, "the scroll wraps one column");
    assert_eq!(pane.children[0].tag, "Column");
    assert_eq!(pane.children[0].part_name.as_deref(), Some("timeline"));
    // The detail (the editor) is bounded, and stays unwrapped.
    assert_eq!(split.children[1].part_name.as_deref(), Some("editor"));
}

// ── the timeline is drawn through Card ──────────────────────────────────────

/// Every node in `node`'s subtree, depth first, `node` included.
fn walk<'a>(node: &'a LayoutNode, out: &mut Vec<&'a LayoutNode>) {
    out.push(node);
    for child in &node.children {
        walk(child, out);
    }
}

fn prop<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a LayoutPropValue> {
    node.props.iter().find(|p| p.name == name).map(|p| &p.value)
}

/// An expression prop's text with the spacing taken out: `( row[2] )` → `(row[2])`.
fn expr(node: &LayoutNode, name: &str) -> String {
    match prop(node, name) {
        Some(LayoutPropValue::Expr(text)) => text.split_whitespace().collect(),
        other => panic!("{} `{name}` should be an expression, got {other:?}", node.tag),
    }
}

/// The `For` over `timeline-rows`: the timeline itself.
fn timeline_for(root: &LayoutNode) -> LayoutNode {
    let mut all = Vec::new();
    walk(root, &mut all);
    let loops: Vec<&LayoutNode> = all
        .into_iter()
        .filter(|n| {
            n.tag == "For"
                && matches!(prop(n, "each"), Some(LayoutPropValue::SlotRef(s)) if s == "timeline-rows")
        })
        .collect();
    assert_eq!(loops.len(), 1, "one For draws timeline-rows");
    loops[0].clone()
}

/// Backlog item 4: each timeline row is a `mosaic-pkg-card` Card, bound to the
/// row's title, excerpt and meta, with an Open button beside it (never around
/// it: a HostButton's children are dropped on six of eight backends, EM-8).
/// Both branches, the open entry's and the others', are checked.
#[test]
fn the_timeline_draws_each_row_as_a_card() {
    let mil = mosmodel_compiler::compile(&read_source("JournalApp.mil")).unwrap();
    let mll = moslayout_compiler::compile(&read_source("JournalApp.mll"), Some(&mil.descriptor_json))
        .unwrap();
    let rows = timeline_for(&mll.def.root);
    let mut nodes = Vec::new();
    walk(&rows, &mut nodes);

    let cards: Vec<&&LayoutNode> =
        nodes.iter().filter(|n| n.package_ref() == Some(("mosaic-pkg-card", "Card"))).collect();
    assert_eq!(cards.len(), 2, "one Card in each branch (open entry, the rest)");
    for card in &cards {
        assert_eq!(expr(card, "title"), "(row[2])");
        assert_eq!(expr(card, "body-text"), "(row[3])");
        assert_eq!(expr(card, "footer"), "(row[4])");
    }

    let buttons: Vec<&&LayoutNode> = nodes.iter().filter(|n| n.tag == "HostButton").collect();
    assert_eq!(buttons.len(), 2, "one Open button in each branch");
    for button in &buttons {
        assert_eq!(prop(button, "label"), Some(&LayoutPropValue::String("Open".into())));
        assert_eq!(expr(button, "a11y-label"), "(row[6])", "named \"Open {{title}}\"");
        assert_eq!(
            prop(button, "onClick"),
            Some(&LayoutPropValue::EmitRef("onSelectEntry".into()))
        );
        assert!(button.children.is_empty(), "the Card must sit beside the button, not in it");
    }

    // The day heading keeps its heading role.
    let heading = nodes
        .iter()
        .find(|n| n.part_name.as_deref() == Some("timeline-day"))
        .expect("a day heading");
    assert_eq!(prop(heading, "a11y-role"), Some(&LayoutPropValue::Keyword("heading".into())));

    // No RecordList draws the timeline any more; "On this day" still uses one.
    let mut all = Vec::new();
    walk(&mll.def.root, &mut all);
    let lists: Vec<&LayoutNode> =
        all.into_iter().filter(|n| n.component() == "RecordList").collect();
    assert_eq!(lists.len(), 1, "only the On this day RecordList remains");
    assert_eq!(
        prop(lists[0], "rows"),
        Some(&LayoutPropValue::SlotRef("on-this-day-rows".into()))
    );
}

/// The Rust app and this interface are one contract: every slot is a prop the
/// app provides, and the app provides nothing the interface does not declare.
#[test]
fn the_apps_props_are_exactly_the_slots() {
    let mil = mosmodel_compiler::compile(&read_source("JournalApp.mil")).unwrap();
    let slots: BTreeSet<String> = mil.component.slots.iter().map(|s| s.name.clone()).collect();

    let mut app = JournalMosaicApp::default();
    let update = app.start(StartContext::new("en-US", Platform::Linux)).unwrap();
    let props: BTreeSet<String> = update.props.as_object().unwrap().keys().cloned().collect();
    assert_eq!(props, slots);
}

#[test]
fn builds_on_every_backend() {
    for backend in Backend::ALL {
        let out = tempfile::TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: package_root(),
            output_root: out.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} build failed: {e}"));
    }
}
