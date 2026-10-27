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
//! `{{row}}` and XAML to `{x:Bind Row}`. Compose used to lower it to
//! `Text(text = "")`, so on Android every row of the list was blank.
//! CollectionActions' missing-media list was one place that shipped this way.
//!
//! The fix lowers the bare NAME to the string template
//! `"$<camelCased NAME>"`. Kotlin's lexical scoping then gives UI29 §3.4's
//! "For wins": the `forEach` lambda parameter shadows a composable
//! parameter of the same name. Outside any loop, a NAME that spells one of
//! the component's own slots reads that slot's parameter, as React's
//! destructured prop does. The template is there because `Text(text = …)`
//! takes only a `String`, and an `index:` binding is a `Double` (the
//! emitter's `_kotlinIdx<i>.toDouble()` shadow), as is a `list<number>`
//! element; unlike `.toString()`, it draws no "Redundant call of conversion
//! method" warning from `kotlinc` when the value already is a `String`.

/// One component exercising every scope a bare NAME can resolve in:
///
/// | part            | bare NAME     | bound by                            |
/// |-----------------|---------------|-------------------------------------|
/// | `own-slot`      | `display-name`| the component's own slot            |
/// | `row-cell`      | `row-item`    | a `For`'s `as:` (kebab-case)        |
/// | `index-cell`    | `i`           | a `For`'s `index:` (a `Double`)     |
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
    mosaic_emit_compose::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit Ledger Compose")
        .output
}

/// No bare NAME may come out as the blank label the old arm emitted.
#[test]
fn no_bare_name_label_is_blank() {
    let out = emit();
    assert!(!out.contains("Text(text = \"\")"), "{out}");
}

#[test]
fn a_loop_binding_label_reads_the_loop_variable() {
    let out = emit();
    assert!(
        out.contains("rows.forEach { rowItem ->"),
        "the loop binds the camelCased name:\n{out}"
    );
    assert!(out.contains("Text(text = \"$rowItem\")"), "{out}");
}

#[test]
fn an_index_binding_label_reads_the_index() {
    let out = emit();
    assert!(
        out.contains("val i: Double = _kotlinIdxi.toDouble()"),
        "{out}"
    );
    assert!(out.contains("Text(text = \"$i\")"), "{out}");
}

#[test]
fn an_own_slot_named_bare_reads_the_slot_parameter() {
    let out = emit();
    assert!(out.contains("displayName: String"), "{out}");
    assert!(out.contains("Text(text = \"$displayName\")"), "{out}");
}

#[test]
fn a_loop_binding_that_shares_a_slot_name_is_the_loop_variable() {
    // The emitted text cannot say which `entry` Kotlin picks; it names the
    // variable and leaves resolution to Kotlin's scoping, where the lambda
    // parameter is the innermost binding.
    let out = emit();
    assert!(out.contains("entry: String"), "{out}");
    assert!(out.contains("rows.forEach { entry ->"), "{out}");
    assert!(out.contains("Text(text = \"$entry\")"), "{out}");
}

#[test]
fn a_number_element_label_is_converted_to_text() {
    let out = emit();
    assert!(out.contains("amounts: List<Double>"), "{out}");
    assert!(out.contains("amounts.forEach { n ->"), "{out}");
    assert!(out.contains("Text(text = \"$n\")"), "{out}");
}
