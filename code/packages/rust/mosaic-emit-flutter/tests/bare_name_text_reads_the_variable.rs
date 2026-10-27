//! A bare NAME in `Text ( content: … )` reads the variable, not nothing.
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
//! `{{row}}` and XAML to `{x:Bind Row}`. Flutter used to lower it to
//! `rows.map((_) => const Text(""))`: the Text was blank, and because a
//! loop picks its closure parameter by scanning the emitted body for the
//! binding, the blank body also made the loop discard the binding.
//! CollectionActions' missing-media list was one place that shipped this way.
//!
//! The fix lowers the bare NAME to `<camelCased NAME>.toString()`, which
//! names the binding, so the loop binds `(row)` (or `final i = entry.key;`)
//! again. Dart's lexical scoping then gives UI29 §3.4's "For wins": the
//! closure parameter shadows a widget field of the same name. Outside any
//! loop, a NAME that spells one of the component's own slots reads that
//! slot's field, as React's destructured prop does. `.toString()` is there
//! because `Text` takes a `String`, and an `index:` binding is an `int`, a
//! `list<number>` element a `double`.

/// One component exercising every scope a bare NAME can resolve in:
///
/// | part            | bare NAME     | bound by                            |
/// |-----------------|---------------|-------------------------------------|
/// | `own-slot`      | `display-name`| the component's own slot            |
/// | `row-cell`      | `row-item`    | a `For`'s `as:` (kebab-case)        |
/// | `index-cell`    | `i`           | a `For`'s `index:` (an `int`)       |
/// | `shadowed-cell` | `entry`       | a `For`'s `as:` AND a slot: For wins|
/// | `number-cell`   | `n`           | a `For` over a `list<number>`       |
const MIL: &str = "component Ledger {
  slot rows : list<text> ;
  slot amounts : list<number> ;
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
    For ( each: slot: amounts, as: n ) {
      Text [ number-cell ] ( content: n )
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
    mosaic_emit_flutter::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit Ledger Flutter")
        .output
}

/// No loop may throw its binding away, and no label may be the blank
/// constant the old arm emitted.
#[test]
fn no_loop_discards_its_binding_and_no_label_is_blank() {
    let out = emit();
    assert!(!out.contains("map((_) =>"), "{out}");
    assert!(!out.contains("const Text(\"\")"), "{out}");
}

#[test]
fn a_loop_binding_label_reads_the_loop_variable() {
    let out = emit();
    assert!(
        out.contains("...rows.map((rowItem) => Text(rowItem.toString()))"),
        "{out}"
    );
}

#[test]
fn an_index_binding_label_reads_the_index() {
    let out = emit();
    assert!(out.contains("final i = entry.key;"), "{out}");
    assert!(out.contains("return Text(i.toString());"), "{out}");
}

#[test]
fn an_own_slot_named_bare_reads_the_slot_field() {
    let out = emit();
    assert!(out.contains("final String displayName;"), "{out}");
    assert!(out.contains("Text(displayName.toString())"), "{out}");
}

#[test]
fn a_loop_binding_that_shares_a_slot_name_is_the_loop_variable() {
    // The emitted text cannot say which `entry` Dart picks; it names the
    // variable and leaves resolution to Dart's scoping, where the closure
    // parameter is the innermost binding.
    let out = emit();
    assert!(out.contains("final String entry;"), "{out}");
    assert!(
        out.contains("...rows.map((entry) => Text(entry.toString()))"),
        "{out}"
    );
}

#[test]
fn a_number_element_label_is_converted_to_text() {
    let out = emit();
    assert!(
        out.contains("...amounts.map((n) => Text(n.toString()))"),
        "{out}"
    );
}
