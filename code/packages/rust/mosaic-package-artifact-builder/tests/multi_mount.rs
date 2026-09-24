//! A package component mounted more than once (UI34 §5 step 4).
//!
//! Mounting the toolkit's `EmptyState` twice used to fail with
//! `DuplicatePart 'empty-state'`, because inlined parts keep their authored
//! names. Now the first mount keeps them and the second is suffixed `-m2`, and
//! the composed style gives each renamed part a copy of the original's style.

use std::path::PathBuf;

use mosaic_package_artifact_builder::compose_component;

fn search_paths() -> Vec<PathBuf> {
    let packages = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("code/packages");
    vec![packages.clone(), packages.join("mosaic")]
}

const MIL: &str = "component Twice { }";
const MLL: &str = r#"layout Twice {
  Column [ root ] {
    pkg::mosaic-pkg-toolkit::EmptyState ( title: "First" , message: "One" , action-label: "" )
    pkg::mosaic-pkg-toolkit::EmptyState ( title: "Second" , message: "Two" , action-label: "" )
  }
}"#;

#[test]
fn a_component_mounted_twice_composes_with_both_mounts_styled() {
    let composed = compose_component("Twice", MIL, MLL, "style Twice { }", &search_paths(), None)
        .expect("two EmptyState mounts compose");

    let parts: Vec<&str> = composed
        .layout
        .parts
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    for name in [
        "empty-state",
        "empty-state-title",
        "empty-state-m2",
        "empty-state-title-m2",
    ] {
        assert!(parts.contains(&name), "missing part {name}: {parts:?}");
    }

    let style = |name: &str| {
        composed
            .style
            .parts
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("no style for {name}"))
    };
    for (first, second) in [
        ("empty-state", "empty-state-m2"),
        ("empty-state-title", "empty-state-title-m2"),
    ] {
        assert_eq!(
            style(first).base,
            style(second).base,
            "{second} is styled like {first}"
        );
        assert_eq!(
            style(first).states,
            style(second).states,
            "{second} keeps {first}'s states"
        );
    }
}

#[test]
fn a_consumer_style_for_the_renamed_part_wins_over_the_copy() {
    let msl = r##"style Twice { part empty-state-m2 { background : "#ff0000" ; } }"##;
    let composed = compose_component("Twice", MIL, MLL, msl, &search_paths(), None)
        .expect("composes with a consumer style");
    let renamed: Vec<_> = composed
        .style
        .parts
        .iter()
        .filter(|p| p.name == "empty-state-m2")
        .collect();
    assert_eq!(
        renamed.len(),
        1,
        "no copy is added beside the consumer's own style"
    );
}

#[test]
fn a_single_mount_is_unchanged() {
    let once = r#"layout Twice { pkg::mosaic-pkg-toolkit::EmptyState ( title: "Only" , message: "One" , action-label: "" ) }"#;
    let composed = compose_component("Twice", MIL, once, "style Twice { }", &search_paths(), None)
        .expect("one mount composes");
    assert!(composed
        .style
        .parts
        .iter()
        .all(|p| !p.name.ends_with("-m2")));
}
