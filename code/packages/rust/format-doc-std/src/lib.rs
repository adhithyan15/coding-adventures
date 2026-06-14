//! # `format-doc-std` — reusable templates over [`format_doc`].
//!
//! The "80% layer" in the formatter stack.  Rust port of
//! [P2D04](../../specs/P2D04-format-doc-std.md).
//!
//! ## Architecture
//!
//! ```text
//! language-specific AST printer
//!   → format-doc-std templates       ← this crate
//!   → Doc                            (format-doc)
//!   → DocLayoutTree                  (format-doc)
//!   → PaintScene                     (format-doc-to-paint, future)
//!   → paint-vm-ascii                 (downstream)
//! ```
//!
//! `format-doc` owns the primitive document algebra; `format-doc-std`
//! owns the common syntax shapes most languages reuse.  Language
//! formatters compose these templates and override the remaining
//! unusual constructs.
//!
//! ## What's in v1
//!
//! Four templates that cover the bulk of real formatter output:
//!
//! | Template            | Covers                                                                 |
//! |---------------------|------------------------------------------------------------------------|
//! | [`delimited_list`]  | Arrays, tuples, parameter lists, argument lists, object fields         |
//! | [`call_like`]       | Function and constructor calls (callee + delimited args)               |
//! | [`block_like`]      | Braces / `begin … end` / indented block bodies                         |
//! | [`infix_chain`]     | Arithmetic, boolean, pipeline, type-operator chains                    |
//!
//! ## Design principles (mirrors P2D04)
//!
//! - **Build Docs, not strings.**  Every template returns a [`Doc`].
//! - **Flat by default, broken when needed.**  Templates rely on
//!   `group()` / `line()` / `softline()` / `indent()` so the
//!   width-fitting algorithm picks the layout.
//! - **Small policy surface.**  Configs cover the choices that
//!   usually vary by language (delimiters, separators, trailing-
//!   separator behaviour, break-before-vs-after operators, empty
//!   spacing).  Edge cases stay in the language packages.
//! - **Escape hatches.**  When a language has unusual rules, build
//!   the `Doc` directly from `format-doc` primitives or wrap the
//!   templates with a thin local helper.
//!
//! ## Example
//!
//! ```rust
//! use format_doc::{layout_doc, render_text, text, LayoutOptions};
//! use format_doc_std::{call_like, CallLikeConfig};
//!
//! // print(a, b, c)
//! let doc = call_like(
//!     text("print"),
//!     vec![text("a"), text("b"), text("c")],
//!     &CallLikeConfig::default(),
//! );
//! let layout = layout_doc(doc, &LayoutOptions::default());
//! assert_eq!(render_text(&layout), "print(a, b, c)");
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use format_doc::{concat, group, if_break, indent, join, line, nil, softline, text, Doc};

/// Package version, mirrored in tests as a smoke check.
pub const VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// TrailingSeparator
// ---------------------------------------------------------------------------

/// Whether a delimited list should emit a trailing separator.
///
/// - [`Never`](TrailingSeparator::Never) — `[a, b, c]` (default).
/// - [`Always`](TrailingSeparator::Always) — `[a, b, c,]` even when flat.
/// - [`IfBreak`](TrailingSeparator::IfBreak) — trailing separator
///   only when the list is rendered broken (the prettier convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TrailingSeparator {
    /// No trailing separator (default).
    #[default]
    Never,
    /// Always emit the separator at the end.
    Always,
    /// Emit the separator only when the list breaks.
    IfBreak,
}

// ---------------------------------------------------------------------------
// delimited_list
// ---------------------------------------------------------------------------

/// Configuration for [`delimited_list`].  Defaults: comma separator,
/// no trailing separator, no inner spacing for empty lists.
#[derive(Debug, Clone)]
pub struct DelimitedListConfig {
    /// Separator between items.  Default: `text(",")`.
    pub separator: Doc,
    /// Trailing-separator policy.
    pub trailing_separator: TrailingSeparator,
    /// Whether `[]` becomes `[ ]` (one inner space).
    pub empty_spacing: bool,
}

impl Default for DelimitedListConfig {
    fn default() -> Self {
        DelimitedListConfig {
            separator: text(","),
            trailing_separator: TrailingSeparator::Never,
            empty_spacing: false,
        }
    }
}

/// Format a list surrounded by delimiters.
///
/// ## Behaviour
///
/// - Empty list: `[]` by default; `[ ]` when `empty_spacing = true`.
/// - Flat: `[a, b, c]`.
/// - Broken: each item on its own line, indented one level inside
///   `open` and `close`.
///
/// The `separator` and `open`/`close` are full `Doc`s so callers
/// can customise (e.g. `text("; ")` for SQL `IN (…)`,
/// `text("{")` for object literals).
pub fn delimited_list(open: Doc, items: Vec<Doc>, close: Doc) -> Doc {
    delimited_list_with(open, items, close, &DelimitedListConfig::default())
}

/// Like [`delimited_list`] but with a caller-supplied config.
pub fn delimited_list_with(
    open: Doc,
    items: Vec<Doc>,
    close: Doc,
    config: &DelimitedListConfig,
) -> Doc {
    if items.is_empty() {
        return concat([open, if config.empty_spacing { text(" ") } else { nil() }, close]);
    }
    let body = join(concat([config.separator.clone(), line()]), items);
    let trailing = trailing_doc(&config.separator, config.trailing_separator);
    group(concat([
        open,
        indent(concat([softline(), body, trailing]), 1),
        softline(),
        close,
    ]))
}

// ---------------------------------------------------------------------------
// call_like
// ---------------------------------------------------------------------------

/// Configuration for [`call_like`].  Defaults: parens, comma, no
/// trailing separator.
#[derive(Debug, Clone)]
pub struct CallLikeConfig {
    /// Opening delimiter.  Default: `text("(")`.
    pub open: Doc,
    /// Closing delimiter.  Default: `text(")")`.
    pub close: Doc,
    /// Separator between arguments.  Default: `text(",")`.
    pub separator: Doc,
    /// Trailing-separator policy.
    pub trailing_separator: TrailingSeparator,
}

impl Default for CallLikeConfig {
    fn default() -> Self {
        CallLikeConfig {
            open: text("("),
            close: text(")"),
            separator: text(","),
            trailing_separator: TrailingSeparator::Never,
        }
    }
}

/// Format a call: a callee followed by a delimited argument list.
///
/// Equivalent to:
///
/// ```text
/// concat([callee, delimited_list_with(open, args, close, ...)])
/// ```
pub fn call_like(callee: Doc, args: Vec<Doc>, config: &CallLikeConfig) -> Doc {
    let list_config = DelimitedListConfig {
        separator: config.separator.clone(),
        trailing_separator: config.trailing_separator,
        empty_spacing: false,
    };
    concat([
        callee,
        delimited_list_with(config.open.clone(), args, config.close.clone(), &list_config),
    ])
}

// ---------------------------------------------------------------------------
// block_like
// ---------------------------------------------------------------------------

/// Configuration for [`block_like`].  Defaults: `empty_spacing = true`
/// (an empty `{ }` includes one inner space).
#[derive(Debug, Clone)]
pub struct BlockLikeConfig {
    /// Whether an empty body becomes `{ }` (one inner space).
    pub empty_spacing: bool,
}

impl Default for BlockLikeConfig {
    fn default() -> Self {
        BlockLikeConfig { empty_spacing: true }
    }
}

/// Format a block: `open <body> close`.
///
/// Short blocks stay inline if they fit.  Longer blocks break to
/// one body per line with indentation.  An empty body collapses to
/// `open close` (or `open<space>close` if `empty_spacing = true`).
pub fn block_like(open: Doc, body: Doc, close: Doc) -> Doc {
    block_like_with(open, body, close, &BlockLikeConfig::default())
}

/// Like [`block_like`] but with a caller-supplied config.
pub fn block_like_with(open: Doc, body: Doc, close: Doc, config: &BlockLikeConfig) -> Doc {
    if matches!(body, Doc::Nil) {
        return concat([open, if config.empty_spacing { text(" ") } else { nil() }, close]);
    }
    group(concat([
        open,
        indent(concat([line(), body]), 1),
        line(),
        close,
    ]))
}

// ---------------------------------------------------------------------------
// infix_chain
// ---------------------------------------------------------------------------

/// Configuration for [`infix_chain`].  Default:
/// `break_before_operators = false` (operators stay at end of line
/// when broken — the C / Java / JavaScript convention).  Set
/// `true` for break-before-operator (the Haskell / Elixir / SQL
/// convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InfixChainConfig {
    /// If `true`, broken form puts each operator at the start of
    /// the new line (`<line><op> <operand>`).  If `false`, operators
    /// trail the previous line (`<sp><op><line><operand>`).
    pub break_before_operators: bool,
}

/// Format a chain of operands separated by infix operators.
///
/// `operators.len()` must equal `operands.len() - 1`; panics
/// otherwise (programmer error, not attacker input).
///
/// - Empty operands → [`Doc::Nil`].
/// - Single operand → that operand alone.
/// - Multi: `op0 <break> op operand1 <break> op operand2 …` where
///   the `<break>` is either before or after the operator
///   depending on [`InfixChainConfig::break_before_operators`].
pub fn infix_chain(operands: Vec<Doc>, operators: Vec<Doc>, config: &InfixChainConfig) -> Doc {
    if operands.is_empty() {
        return nil();
    }
    assert_eq!(
        operators.len(),
        operands.len() - 1,
        "infix_chain: operators.len() must equal operands.len() - 1"
    );
    let mut iter = operands.into_iter();
    let first = iter.next().unwrap();
    if iter.len() == 0 {
        return first;
    }
    let break_before = config.break_before_operators;
    let mut rest: Vec<Doc> = Vec::with_capacity(operators.len() * 4);
    for (operator, operand) in operators.into_iter().zip(iter) {
        if break_before {
            // <line><op><space><operand>
            rest.push(line());
            rest.push(operator);
            rest.push(text(" "));
            rest.push(operand);
        } else {
            // <space><op><line><operand>
            rest.push(text(" "));
            rest.push(operator);
            rest.push(line());
            rest.push(operand);
        }
    }
    group(concat([first, indent(concat(rest), 1)]))
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn trailing_doc(separator: &Doc, trailing: TrailingSeparator) -> Doc {
    match trailing {
        TrailingSeparator::Never => nil(),
        TrailingSeparator::Always => separator.clone(),
        TrailingSeparator::IfBreak => if_break(separator.clone(), nil()),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use format_doc::{layout_doc, render_text, LayoutOptions};

    fn render(doc: Doc, width: usize) -> String {
        render_text(&layout_doc(doc, &LayoutOptions { print_width: width, ..Default::default() }))
    }

    // ---------- VERSION smoke ----------

    #[test]
    fn version_constant_is_present() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ---------- delimited_list ----------

    #[test]
    fn delimited_list_empty_default() {
        let d = delimited_list(text("["), vec![], text("]"));
        assert_eq!(render(d, 80), "[]");
    }

    #[test]
    fn delimited_list_empty_with_spacing() {
        let cfg = DelimitedListConfig {
            empty_spacing: true,
            ..DelimitedListConfig::default()
        };
        let d = delimited_list_with(text("["), vec![], text("]"), &cfg);
        assert_eq!(render(d, 80), "[ ]");
    }

    #[test]
    fn delimited_list_flat_when_fits() {
        let d = delimited_list(text("["), vec![text("a"), text("b"), text("c")], text("]"));
        assert_eq!(render(d, 80), "[a, b, c]");
    }

    #[test]
    fn delimited_list_broken_when_does_not_fit() {
        let d = delimited_list(
            text("["),
            vec![
                text("aaaaaaaaaa"),
                text("bbbbbbbbbb"),
                text("cccccccccc"),
            ],
            text("]"),
        );
        let s = render(d, 12);
        assert!(s.contains('\n'), "expected broken, got:\n{s}");
        // Items on separate lines, indented.
        assert!(s.starts_with("[\n"));
        assert!(s.ends_with("]"));
    }

    #[test]
    fn delimited_list_custom_separator() {
        let cfg = DelimitedListConfig {
            separator: text(";"),
            ..DelimitedListConfig::default()
        };
        let d = delimited_list_with(
            text("("),
            vec![text("a"), text("b"), text("c")],
            text(")"),
            &cfg,
        );
        assert_eq!(render(d, 80), "(a; b; c)");
    }

    #[test]
    fn delimited_list_trailing_always_flat() {
        let cfg = DelimitedListConfig {
            trailing_separator: TrailingSeparator::Always,
            ..DelimitedListConfig::default()
        };
        let d = delimited_list_with(
            text("["),
            vec![text("a"), text("b")],
            text("]"),
            &cfg,
        );
        assert_eq!(render(d, 80), "[a, b,]");
    }

    #[test]
    fn delimited_list_trailing_if_break_flat_omits() {
        let cfg = DelimitedListConfig {
            trailing_separator: TrailingSeparator::IfBreak,
            ..DelimitedListConfig::default()
        };
        let d = delimited_list_with(
            text("["),
            vec![text("a"), text("b")],
            text("]"),
            &cfg,
        );
        // Flat: no trailing comma.
        assert_eq!(render(d, 80), "[a, b]");
    }

    #[test]
    fn delimited_list_trailing_if_break_broken_emits() {
        let cfg = DelimitedListConfig {
            trailing_separator: TrailingSeparator::IfBreak,
            ..DelimitedListConfig::default()
        };
        let d = delimited_list_with(
            text("["),
            vec![text("aaaaaaaa"), text("bbbbbbbb"), text("cccccccc")],
            text("]"),
            &cfg,
        );
        // Force break.
        let s = render(d, 10);
        assert!(s.contains('\n'));
        assert!(s.contains(",\n]"), "expected trailing comma in broken form, got:\n{s}");
    }

    // ---------- call_like ----------

    #[test]
    fn call_like_default_parens_and_commas() {
        let d = call_like(text("print"), vec![text("a"), text("b"), text("c")], &CallLikeConfig::default());
        assert_eq!(render(d, 80), "print(a, b, c)");
    }

    #[test]
    fn call_like_empty_args() {
        let d = call_like(text("now"), vec![], &CallLikeConfig::default());
        assert_eq!(render(d, 80), "now()");
    }

    #[test]
    fn call_like_breaks_when_args_too_long() {
        let d = call_like(
            text("very_long_function_name"),
            vec![text("first_argument"), text("second_argument"), text("third_argument")],
            &CallLikeConfig::default(),
        );
        let s = render(d, 30);
        assert!(s.contains('\n'), "expected broken, got:\n{s}");
        assert!(s.starts_with("very_long_function_name("));
    }

    #[test]
    fn call_like_custom_brackets() {
        let cfg = CallLikeConfig {
            open: text("["),
            close: text("]"),
            ..CallLikeConfig::default()
        };
        let d = call_like(text("idx"), vec![text("0")], &cfg);
        assert_eq!(render(d, 80), "idx[0]");
    }

    // ---------- block_like ----------

    #[test]
    fn block_like_default_empty_spacing() {
        let d = block_like(text("{"), nil(), text("}"));
        assert_eq!(render(d, 80), "{ }");
    }

    #[test]
    fn block_like_empty_no_spacing() {
        let cfg = BlockLikeConfig { empty_spacing: false };
        let d = block_like_with(text("{"), nil(), text("}"), &cfg);
        assert_eq!(render(d, 80), "{}");
    }

    #[test]
    fn block_like_inline_when_fits() {
        let d = block_like(text("{"), text("body"), text("}"));
        assert_eq!(render(d, 80), "{ body }");
    }

    #[test]
    fn block_like_breaks_when_body_too_long() {
        let d = block_like(
            text("{"),
            text("body_that_exceeds_print_width_to_force_break"),
            text("}"),
        );
        let s = render(d, 20);
        assert!(s.contains('\n'), "expected broken, got:\n{s}");
        assert!(s.starts_with("{\n"));
        assert!(s.ends_with("\n}"));
    }

    // ---------- infix_chain ----------

    #[test]
    fn infix_chain_empty_is_nil() {
        let d = infix_chain(vec![], vec![], &InfixChainConfig::default());
        assert!(matches!(d, Doc::Nil));
    }

    #[test]
    fn infix_chain_single_operand_unchanged() {
        let d = infix_chain(vec![text("x")], vec![], &InfixChainConfig::default());
        assert_eq!(render(d, 80), "x");
    }

    #[test]
    fn infix_chain_break_after_operators_default() {
        let d = infix_chain(
            vec![text("a"), text("b"), text("c")],
            vec![text("+"), text("-")],
            &InfixChainConfig::default(),
        );
        // Flat: a + b - c
        assert_eq!(render(d, 80), "a + b - c");
    }

    #[test]
    fn infix_chain_break_after_operators_broken() {
        let d = infix_chain(
            vec![text("aaaaaaaa"), text("bbbbbbbb"), text("cccccccc")],
            vec![text("+"), text("-")],
            &InfixChainConfig::default(),
        );
        let s = render(d, 12);
        assert!(s.contains('\n'), "expected broken, got:\n{s}");
        // C-style: operators trail previous line, operands lead next.
        let lines: Vec<&str> = s.split('\n').collect();
        assert!(lines[0].ends_with('+'), "first line should end with +, got: {:?}", lines[0]);
    }

    #[test]
    fn infix_chain_break_before_operators() {
        let d = infix_chain(
            vec![text("aaaaaaaa"), text("bbbbbbbb"), text("cccccccc")],
            vec![text("+"), text("-")],
            &InfixChainConfig { break_before_operators: true },
        );
        let s = render(d, 12);
        assert!(s.contains('\n'), "expected broken, got:\n{s}");
        // Haskell-style: operators lead the new line.
        let lines: Vec<&str> = s.split('\n').collect();
        assert!(lines[1].trim_start().starts_with('+'), "second line should start with +, got: {:?}", lines[1]);
    }

    #[test]
    #[should_panic(expected = "operators.len() must equal operands.len() - 1")]
    fn infix_chain_arity_mismatch_panics() {
        let _ = infix_chain(
            vec![text("a"), text("b"), text("c")],
            vec![text("+")], // need 2 operators, given 1
            &InfixChainConfig::default(),
        );
    }

    // ---------- composability ----------

    #[test]
    fn templates_compose_into_realistic_expression() {
        // print(x + y, z)
        let sum = infix_chain(
            vec![text("x"), text("y")],
            vec![text("+")],
            &InfixChainConfig::default(),
        );
        let call = call_like(text("print"), vec![sum, text("z")], &CallLikeConfig::default());
        assert_eq!(render(call, 80), "print(x + y, z)");
    }

    #[test]
    fn nested_delimited_lists_break_outer_first() {
        // [[a, b], [c, d], [e, f]] — wide enough to fit flat.
        let inner = |xs: Vec<Doc>| delimited_list(text("["), xs, text("]"));
        let outer = delimited_list(
            text("["),
            vec![
                inner(vec![text("a"), text("b")]),
                inner(vec![text("c"), text("d")]),
                inner(vec![text("e"), text("f")]),
            ],
            text("]"),
        );
        assert_eq!(render(outer, 80), "[[a, b], [c, d], [e, f]]");
    }
}
