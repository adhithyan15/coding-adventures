//! package_compiles — smoke test for the mosaic-pkg-card package.
//!
//! Mirrors the test in `mosaic-pkg-grid/tests/package_compiles.rs`, but
//! pared down to the minimum that this package needs.  Card has exactly
//! one component, no loops, no conditionals, no host-table references,
//! and only kernel primitives that are stable in every backend — so the
//! whole package SHOULD compile clean end-to-end.  If any of these
//! assertions ever fails, it's a regression in the kernel-frontend
//! pipeline (mosmodel / moslayout / mosstyle), not in this package.
//!
//! What this asserts
//! -----------------
//!   1. mosaic-package.toml parses as TOML and matches the §4.2 shape:
//!      - [components].exports == ["Card"]
//!      - [kernel].version    == "1"
//!   2. src/Card.mil compiles via mosmodel_compiler::compile, and the
//!      resulting interface has exactly three slots (title / body /
//!      footer, all `text`) and zero emits.
//!   3. src/Card.mll compiles via moslayout_compiler::compile against
//!      the .mil interface descriptor, and the layout tree is:
//!        Column [card-root]
//!          Box [card-title]
//!          Box [card-body]
//!          Box [card-footer]
//!   4. src/Card.dark.msl compiles via mosstyle_compiler::compile
//!      against the .mll part map, and produces exactly four parts:
//!      card-root / card-title / card-body / card-footer.
//!   5. The expected source tree is on disk (belt-and-suspenders).

use std::fs;
use std::path::PathBuf;

/// Path helpers — anchor everything to the package root.
fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_path(name: &str) -> PathBuf {
    package_root().join("src").join(name)
}

fn read_source(name: &str) -> String {
    let path = src_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// 1. Manifest
// ---------------------------------------------------------------------------

#[test]
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");

    let value: toml::Value = toml::from_str(&manifest_src)
        .expect("mosaic-package.toml must parse as valid TOML");

    // [package].name
    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-card", "[package].name mismatch");

    // [package].version
    let version = value
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[package].version must be set");
    assert_eq!(version, "0.1.0", "[package].version must be 0.1.0 for the initial release");

    // [components].exports
    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        export_names,
        vec!["Card"],
        "[components].exports must list exactly Card"
    );

    // [kernel].version
    let kernel_version = value
        .get("kernel")
        .and_then(|k| k.get("version"))
        .and_then(|v| v.as_str())
        .expect("[kernel].version must be set");
    assert_eq!(kernel_version, "1", "[kernel].version must target UI29 kernel v1");

    // [dependencies] table present (even if empty) — §4.2 shape requirement.
    assert!(
        value.get("dependencies").is_some(),
        "[dependencies] table must be present (may be empty)"
    );
}

// ---------------------------------------------------------------------------
// 2. mosmodel — Card.mil compilation
// ---------------------------------------------------------------------------

/// The .mil must compile, name the component `Card`, expose exactly the
/// three text slots, and declare zero emits.
#[test]
fn card_mil_compiles_with_expected_interface() {
    let src = read_source("Card.mil");
    let out = mosmodel_compiler::compile(&src)
        .unwrap_or_else(|e| panic!("Card.mil failed to compile via mosmodel_compiler:\n{:#?}", e));

    assert_eq!(
        out.component.component, "Card",
        "Card.mil must declare component named 'Card'"
    );

    // Slots — exactly three, in declared order, all of type text.
    let slot_names: Vec<&str> = out.component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec!["title", "body", "footer"],
        "Card must expose exactly three slots in order: title, body, footer"
    );
    for slot in &out.component.slots {
        assert!(
            matches!(slot.r#type, mosmodel_compiler::SlotType::Text),
            "slot `{}` must be of type text (got {:?})",
            slot.name, slot.r#type
        );
    }

    // Emits — Card is display-only.
    assert!(
        out.component.emits.is_empty(),
        "Card must declare zero emits (got {} emits: {:?})",
        out.component.emits.len(),
        out.component
            .emits
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// 3. moslayout — Card.mll compilation
// ---------------------------------------------------------------------------

/// The .mll must compile against the .mil interface and yield the tree
/// described in the file's doc-comment: Column[card-root] with three
/// Box children named card-title / card-body / card-footer.
#[test]
fn card_mll_compiles_against_its_interface() {
    let mil_src = read_source("Card.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src)
        .unwrap_or_else(|e| panic!("Card.mil precompile failed: {:#?}", e));

    let mll_src = read_source("Card.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .unwrap_or_else(|e| panic!("Card.mll failed to compile via moslayout_compiler:\n{:#?}", e));

    let root = &mll_out.def.root;
    assert_eq!(root.tag, "Column", "Card root must be a Column");
    assert_eq!(
        root.part_name.as_deref(),
        Some("card-root"),
        "Card root Column must be named part `card-root`"
    );
    assert_eq!(
        root.children.len(),
        3,
        "Card root must contain exactly three Box children (got {})",
        root.children.len()
    );

    let expected_parts = ["card-title", "card-body", "card-footer"];
    for (child, expected_part) in root.children.iter().zip(expected_parts.iter()) {
        assert_eq!(
            child.tag, "Box",
            "each direct child of the Column must be a Box (got `{}`)",
            child.tag
        );
        assert_eq!(
            child.part_name.as_deref(),
            Some(*expected_part),
            "Box must be named part `{}`",
            expected_part
        );
        // Each Box wraps a single Text node.
        assert_eq!(
            child.children.len(),
            1,
            "Box[{}] must contain exactly one child Text node",
            expected_part
        );
        assert_eq!(
            child.children[0].tag, "Text",
            "Box[{}] must wrap a Text node",
            expected_part
        );
    }
}

// ---------------------------------------------------------------------------
// 4. mosstyle — Card.dark.msl compilation
// ---------------------------------------------------------------------------

/// The .msl must compile against the part map produced by Card.mll and
/// produce exactly four part entries — one for each part declared in
/// the layout (card-root + card-title + card-body + card-footer).
#[test]
fn card_dark_msl_compiles_against_its_part_map() {
    let mll_src = read_source("Card.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, None)
        .unwrap_or_else(|e| panic!("Card.mll precompile failed: {:#?}", e));

    let msl_src = read_source("Card.dark.msl");
    let msl_out = mosstyle_compiler::compile(&msl_src, Some(&mll_out.part_map_json))
        .unwrap_or_else(|e| panic!("Card.dark.msl failed to compile via mosstyle_compiler:\n{:#?}", e));

    let part_names: Vec<&str> = msl_out.def.parts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        part_names,
        vec!["card-root", "card-title", "card-body", "card-footer"],
        "Card.dark.msl must declare exactly four parts in the order: \
         card-root, card-title, card-body, card-footer"
    );
}

// ---------------------------------------------------------------------------
// 5. Source-shape sanity
// ---------------------------------------------------------------------------

/// Belt-and-suspenders: every file the package promises is on disk.
#[test]
fn source_tree_has_expected_shape() {
    let expected = ["Card.mil", "Card.mll", "Card.dark.msl"];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected package source file missing: {}",
            path.display()
        );
    }
}
