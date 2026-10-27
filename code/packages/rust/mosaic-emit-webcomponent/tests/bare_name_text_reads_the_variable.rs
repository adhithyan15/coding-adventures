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
//! `{{row}}` and XAML to `{x:Bind Row}`. The Web Component backend used to
//! write the NAME into its template literal as text, `<span>row</span>`, so
//! every row read the word "row". CollectionActions' missing-media list was
//! one place that shipped this way.
//!
//! The fix interpolates `${escapeHtml(<camelCased NAME>)}`, the same
//! escaped interpolation a `slot:` reference gets: the value is host data on
//! its way into `shadowRoot.innerHTML`, so it is never written raw.
//! JavaScript's lexical scoping then gives UI29 §3.4's "For wins": the
//! `.map((row, idx) => …)` arrow parameter shadows the `const row = …` slot
//! line at the top of `_render()`. Outside any loop, a NAME that spells one
//! of the component's own slots reads that slot line, as React's
//! destructured prop does.

/// One component exercising every scope a bare NAME can resolve in:
///
/// | part            | bare NAME     | bound by                            |
/// |-----------------|---------------|-------------------------------------|
/// | `own-slot`      | `display-name`| the component's own slot            |
/// | `row-cell`      | `row-item`    | a `For`'s `as:` (kebab-case)        |
/// | `index-cell`    | `i`           | a `For`'s `index:`                  |
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
    mosaic_emit_webcomponent::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit Ledger Web Component")
        .output
}

/// The `_render()` template literal, the only place a label can appear.
fn template() -> String {
    let out = emit();
    out.lines()
        .find(|line| line.contains("this.shadowRoot.innerHTML = `"))
        .unwrap_or_else(|| panic!("no render template in:\n{out}"))
        .to_string()
}

/// No bare NAME may come out as its own spelling, the old arm's output.
#[test]
fn no_bare_name_is_written_as_text() {
    let template = template();
    for spelling in ["display-name", "row-item", "i", "entry"] {
        assert!(
            !template.contains(&format!(">{spelling}</span>")),
            "`{spelling}` must not be its own label:\n{template}"
        );
    }
}

#[test]
fn a_loop_binding_label_reads_the_loop_variable() {
    let template = template();
    assert!(
        template.contains("rows.map((rowItem, idx) => `<span>${escapeHtml(rowItem)}</span>`)"),
        "{template}"
    );
}

#[test]
fn an_index_binding_label_reads_the_index() {
    let template = template();
    assert!(
        template.contains("rows.map((row, i) => `<span>${escapeHtml(i)}</span>`)"),
        "{template}"
    );
}

#[test]
fn an_own_slot_named_bare_reads_the_slot_line() {
    let out = emit();
    assert!(
        out.contains("const displayName = this.getAttribute(\"display-name\") ?? \"\";"),
        "{out}"
    );
    assert!(
        template().contains("<span>${escapeHtml(displayName)}</span>"),
        "{out}"
    );
}

#[test]
fn a_loop_binding_that_shares_a_slot_name_is_the_loop_variable() {
    // The emitted text cannot say which `entry` JavaScript picks; it names
    // the variable and leaves resolution to JavaScript's scoping, where the
    // arrow parameter is the innermost binding.
    let out = emit();
    assert!(
        out.contains("const entry = this.getAttribute(\"entry\") ?? \"\";"),
        "{out}"
    );
    assert!(
        template().contains("rows.map((entry, idx) => `<span>${escapeHtml(entry)}</span>`)"),
        "{out}"
    );
}

/// Every bare-NAME value is HTML-escaped, never interpolated raw: the
/// labels are host data on their way into `innerHTML`.
#[test]
fn every_bare_name_value_is_escaped() {
    let template = template();
    for name in ["displayName", "rowItem", "i", "entry"] {
        assert!(
            !template.contains(&format!("${{{name}}}")),
            "`{name}` must not be interpolated unescaped:\n{template}"
        );
        assert!(
            template.contains(&format!("${{escapeHtml({name})}}")),
            "`{name}` must be interpolated through escapeHtml:\n{template}"
        );
    }
}
