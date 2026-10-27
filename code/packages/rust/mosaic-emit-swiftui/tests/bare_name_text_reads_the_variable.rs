//! A bare NAME in `Text ( content: … )` reads the variable, not its spelling.
//!
//! Mosaic has two ways to name a value inside a layout:
//!
//! ```text
//! Text ( content: slot: title )    a slot reference, always the slot
//! Text ( content: row )            a bare NAME (`LayoutPropValue::Keyword`)
//! ```
//!
//! UI29 §3.3 gives a bare NAME one meaning, `name_ref ::= NAME`, the name a
//! `For` bound with `as:` or `index:`. So in
//!
//! ```text
//! For ( each: slot: rows, as: row ) { Text ( content: row ) }
//! ```
//!
//! every label shows that row's value. React lowers it to `{row}`, HTML to
//! `{{row}}` and XAML to `{x:Bind Row}`. SwiftUI used to lower it to
//! `Text("row")`, so on Apple platforms every row read the word "row".
//! CollectionActions' missing-media list was one place that shipped this way.
//!
//! The fix lowers the bare NAME to `_mosaicText(<camelCased NAME>)`, as every
//! other SwiftUI prop that takes a bare NAME already did (button labels,
//! placeholders, accessibility labels). Swift's lexical scoping then gives
//! UI29 §3.4's "For wins": the `ForEach` closure parameter shadows a
//! component property of the same name. Outside any loop, a NAME that
//! spells one of the component's own slots reads that slot's property, as
//! React's destructured prop does.
//!
//! The macOS test hands the generated file to `swiftc -typecheck`, which is
//! what proves each `_mosaicText(…)` names something in scope, including
//! the shadowing case where a slot and a loop binding share a name.

#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::process::Command;

/// One component exercising every scope a bare NAME can resolve in:
///
/// | part            | bare NAME     | bound by                            |
/// |-----------------|---------------|-------------------------------------|
/// | `row-cell`      | `row-item`    | a `For`'s `as:` (kebab-case)        |
/// | `index-cell`    | `i`           | a `For`'s `index:`                  |
/// | `own-slot`      | `display-name`| the component's own slot            |
/// | `shadowed-cell` | `entry`       | a `For`'s `as:` AND a slot: For wins|
const MIL: &str = "component Ledger {
  slot rows : list<text> ;
  slot display-name : text ;
  slot entry : text ;
}
";

const MLL: &str = "layout Ledger {
  Column [ root ] {
    Text [ own-slot ] ( content: display-name )
    For ( each: slot: rows, as: row-item ) {
      Text [ row-cell ] ( content: row-item )
    }
    For ( each: slot: rows, as: row, index: i ) {
      Text [ index-cell ] ( content: i )
    }
    For ( each: slot: rows, as: entry ) {
      Text [ shadowed-cell ] ( content: entry )
    }
  }
}
";

/// Compile the fixture through the real three-language pipeline, so the
/// test sees the same `LayoutPropValue::Keyword` the parser produces.
fn emit() -> String {
    let interface = mosmodel_compiler::compile(MIL).expect("compile Ledger interface");
    let layout = moslayout_compiler::compile(MLL, Some(&interface.descriptor_json))
        .expect("compile Ledger layout");
    let style = mosstyle_compiler::compile("style Ledger { }\n", Some(&layout.part_map_json))
        .expect("compile Ledger style");
    mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit Ledger SwiftUI")
        .output
}

#[test]
fn a_loop_binding_label_reads_the_loop_variable() {
    let out = emit();
    assert!(
        out.contains("ForEach(rows, id: \\.self) { rowItem in"),
        "the loop binds the camelCased name:\n{out}"
    );
    assert!(out.contains("_mosaicText(rowItem)"), "{out}");
    assert!(
        !out.contains("Text(\"row-item\")"),
        "the label must not be the binding's spelling:\n{out}"
    );
}

#[test]
fn an_index_binding_label_reads_the_index() {
    let out = emit();
    assert!(out.contains("let i: Double = Double(_swiftIdxi)"), "{out}");
    assert!(out.contains("_mosaicText(i)"), "{out}");
    assert!(!out.contains("Text(\"i\")"), "{out}");
}

#[test]
fn an_own_slot_named_bare_reads_the_slot_property() {
    let out = emit();
    assert!(out.contains("let displayName: String"), "{out}");
    assert!(out.contains("_mosaicText(displayName)"), "{out}");
    assert!(!out.contains("Text(\"display-name\")"), "{out}");
}

#[test]
fn a_loop_binding_that_shares_a_slot_name_is_the_loop_variable() {
    // The emitted text cannot say which `entry` Swift picks; it names the
    // variable and leaves resolution to Swift's scoping, where the closure
    // parameter is the innermost binding. `swiftc -typecheck` below checks
    // the shadowing compiles.
    let out = emit();
    assert!(out.contains("let entry: String"), "{out}");
    assert!(
        out.contains("ForEach(rows, id: \\.self) { entry in"),
        "{out}"
    );
    assert!(out.contains("_mosaicText(entry)"), "{out}");
    assert!(!out.contains("Text(\"entry\")"), "{out}");
}

#[cfg(target_os = "macos")]
#[test]
fn every_bare_name_label_typechecks() {
    let source_path =
        std::env::temp_dir().join(format!("mosaic-bare-name-{}.swift", std::process::id()));
    fs::write(&source_path, emit()).expect("write generated Swift fixture");
    let result = Command::new("swiftc")
        .arg("-typecheck")
        .arg(&source_path)
        .output()
        .expect("run swiftc");
    let _ = fs::remove_file(&source_path);
    assert!(
        result.status.success(),
        "every bare-NAME label must name a Swift variable in scope:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
