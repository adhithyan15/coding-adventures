//! # `twig-completion` — LSP code-completion items for Twig.
//!
//! Walks a parsed [`twig_parser::Program`] plus an optional
//! partial-typed prefix and returns the editor's autocomplete
//! menu — defined symbols + built-in keywords + constant
//! literals.  Drives `textDocument/completion`.
//!
//! The **sixth piece of the Twig authoring-experience layer**
//! (alongside [`twig-formatter`](../twig-formatter/),
//! [`twig-semantic-tokens`](../twig-semantic-tokens/),
//! [`twig-document-symbols`](../twig-document-symbols/),
//! [`twig-folding-ranges`](../twig-folding-ranges/),
//! [`twig-hover`](../twig-hover/)).
//!
//! ## What's surfaced
//!
//! | Source                                           | `CompletionKind` | `detail`               |
//! |--------------------------------------------------|------------------|------------------------|
//! | Top-level `(define name (lambda params body))`   | `Function`       | `"(params)"`           |
//! | Top-level `(define name expr)` (any other)       | `Variable`       | `None`                 |
//! | Built-in keyword (`if`/`let`/`lambda`/`begin`/`define`/`quote`) | `Keyword` | `None`        |
//! | Constant literal (`#t`/`#f`/`nil`)               | `Constant`       | `None`                 |
//!
//! ## Public API
//!
//! - [`completions(source, prefix)`] — `&str` + `Option<&str>` →
//!   `Result<Vec<CompletionItem>, TwigParseError>`.  The common case.
//! - [`completions_for_program(program, prefix)`] — already-parsed
//!   `&Program` + `Option<&str>` → `Vec<CompletionItem>`.
//!
//! When `prefix` is `Some`, items whose `label` doesn't start with
//! it are filtered out.  When `None`, every item is returned.
//! Editors typically pass the substring already typed before the
//! cursor, e.g. for `(squa|` the prefix is `"squa"`.
//!
//! Items come back sorted: keywords first (in declaration order),
//! constants next, then user-defined symbols in name order.  This
//! matches what most editor completion menus expect when no
//! score-based ranking is supplied.
//!
//! ## What this crate does NOT do
//!
//! - **No fuzzy match.**  Prefix filtering is exact.  Editors that
//!   want fuzzy matching should pass `prefix = None` and run their
//!   own client-side fuzzy matcher.
//! - **No snippets.**  `(if ${1:cond} ${2:then} ${3:else})` would
//!   be a separate `insert_text` field; v1 only emits `label`.
//! - **No scope-aware suggestions.**  Parameters and let-bindings
//!   aren't surfaced (the parser doesn't thread per-binding
//!   positions).  Fixes alongside `twig-hover`'s scope-resolution
//!   follow-up.
//! - **No LSP wire encoding.**  Returns a typed `Vec<CompletionItem>`;
//!   the JSON `CompletionItem[]` shape is one level up.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use twig_document_symbols::{symbols_for_program, DocumentSymbol, SymbolKind};
use twig_parser::{parse, Program, TwigParseError};

// ---------------------------------------------------------------------------
// CompletionKind
// ---------------------------------------------------------------------------

/// Classification of a completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompletionKind {
    /// User-defined function (a `(define name (lambda …))`).
    Function,
    /// User-defined value (a `(define name expr)` for non-lambda
    /// `expr`).
    Variable,
    /// Built-in keyword (`if`, `let`, `lambda`, `begin`, `define`,
    /// `quote`).
    Keyword,
    /// Constant literal (`#t`, `#f`, `nil`).
    Constant,
}

impl CompletionKind {
    /// Stable lowercase mnemonic.
    pub fn mnemonic(self) -> &'static str {
        match self {
            CompletionKind::Function => "function",
            CompletionKind::Variable => "variable",
            CompletionKind::Keyword => "keyword",
            CompletionKind::Constant => "constant",
        }
    }
}

impl fmt::Display for CompletionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// ---------------------------------------------------------------------------
// CompletionItem
// ---------------------------------------------------------------------------

/// One menu entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// What the user sees and what gets inserted.
    pub label: String,
    /// Classification.
    pub kind: CompletionKind,
    /// Optional one-line summary surfaced next to the label
    /// (e.g. lambda parameter signature).
    pub detail: Option<String>,
}

// ---------------------------------------------------------------------------
// Built-in items
// ---------------------------------------------------------------------------

/// The six Twig keywords surfaced for completion, in declaration
/// order — matches the order users learn them.
const BUILTIN_KEYWORDS: &[&str] = &["define", "if", "let", "lambda", "begin", "quote"];

/// The three constant literals.
const BUILTIN_CONSTANTS: &[&str] = &["#t", "#f", "nil"];

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse `source` and return completion items, optionally filtered
/// to those whose `label` starts with `prefix`.
pub fn completions(
    source: &str,
    prefix: Option<&str>,
) -> Result<Vec<CompletionItem>, TwigParseError> {
    let program = parse(source)?;
    Ok(completions_for_program(&program, prefix))
}

/// Walk an already-parsed `Program` and return completion items,
/// optionally filtered.
pub fn completions_for_program(program: &Program, prefix: Option<&str>) -> Vec<CompletionItem> {
    let mut out: Vec<CompletionItem> = Vec::new();

    // Keywords first — matches the editor convention of grouping
    // language constructs above user code.
    for kw in BUILTIN_KEYWORDS {
        out.push(CompletionItem {
            label: (*kw).to_owned(),
            kind: CompletionKind::Keyword,
            detail: None,
        });
    }
    // Constants next.
    for c in BUILTIN_CONSTANTS {
        out.push(CompletionItem {
            label: (*c).to_owned(),
            kind: CompletionKind::Constant,
            detail: None,
        });
    }

    // User-defined symbols, sorted by name for deterministic output.
    let mut symbols = symbols_for_program(program);
    symbols.sort_by(|a, b| a.name.cmp(&b.name));
    for sym in &symbols {
        out.push(item_for_symbol(sym));
    }

    if let Some(p) = prefix {
        out.retain(|c| c.label.starts_with(p));
    }
    out
}

fn item_for_symbol(sym: &DocumentSymbol) -> CompletionItem {
    let kind = match sym.kind {
        SymbolKind::Function => CompletionKind::Function,
        SymbolKind::Variable => CompletionKind::Variable,
        // SymbolKind is `#[non_exhaustive]`; future variants
        // surface as Variable until this crate is updated.
        _ => CompletionKind::Variable,
    };
    CompletionItem {
        label: sym.name.clone(),
        kind,
        detail: sym.detail.clone(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cs(src: &str) -> Vec<CompletionItem> {
        completions(src, None).expect("parse")
    }

    fn cs_prefix(src: &str, prefix: &str) -> Vec<CompletionItem> {
        completions(src, Some(prefix)).expect("parse")
    }

    fn labels(items: &[CompletionItem]) -> Vec<&str> {
        items.iter().map(|c| c.label.as_str()).collect()
    }

    // ---------- CompletionKind ----------

    #[test]
    fn completion_kind_mnemonics_distinct() {
        let all = [
            CompletionKind::Function,
            CompletionKind::Variable,
            CompletionKind::Keyword,
            CompletionKind::Constant,
        ];
        let mut mns: Vec<&'static str> = all.iter().map(|k| k.mnemonic()).collect();
        mns.sort();
        mns.dedup();
        assert_eq!(mns.len(), all.len());
    }

    #[test]
    fn completion_kind_displays_as_mnemonic() {
        assert_eq!(format!("{}", CompletionKind::Function), "function");
    }

    // ---------- Built-in items always present ----------

    #[test]
    fn empty_program_still_yields_keywords_and_constants() {
        let items = cs("");
        let lbls = labels(&items);
        // All keywords present.
        for kw in BUILTIN_KEYWORDS {
            assert!(lbls.contains(kw), "missing keyword {kw}");
        }
        // All constants present.
        for c in BUILTIN_CONSTANTS {
            assert!(lbls.contains(c), "missing constant {c}");
        }
    }

    #[test]
    fn keyword_items_are_keyword_kind() {
        let items = cs("");
        let kw_items: Vec<&CompletionItem> = items
            .iter()
            .filter(|c| c.kind == CompletionKind::Keyword)
            .collect();
        assert_eq!(kw_items.len(), BUILTIN_KEYWORDS.len());
    }

    #[test]
    fn constant_items_are_constant_kind() {
        let items = cs("");
        let cnt_items: Vec<&CompletionItem> = items
            .iter()
            .filter(|c| c.kind == CompletionKind::Constant)
            .collect();
        assert_eq!(cnt_items.len(), BUILTIN_CONSTANTS.len());
    }

    // ---------- Order: keywords → constants → symbols ----------

    #[test]
    fn order_keywords_first_then_constants_then_symbols() {
        let items = cs("(define b 1) (define a 2)");
        // Find the first keyword index, first constant index,
        // first symbol (Variable) index.
        let mut first_kw = None;
        let mut first_const = None;
        let mut first_sym = None;
        for (i, c) in items.iter().enumerate() {
            match c.kind {
                CompletionKind::Keyword => {
                    if first_kw.is_none() {
                        first_kw = Some(i);
                    }
                }
                CompletionKind::Constant => {
                    if first_const.is_none() {
                        first_const = Some(i);
                    }
                }
                CompletionKind::Variable | CompletionKind::Function => {
                    if first_sym.is_none() {
                        first_sym = Some(i);
                    }
                }
            }
        }
        assert!(first_kw.unwrap() < first_const.unwrap());
        assert!(first_const.unwrap() < first_sym.unwrap());
    }

    #[test]
    fn user_symbols_sorted_alphabetically() {
        let items = cs("(define banana 1) (define apple 2) (define cherry 3)");
        let symbol_labels: Vec<&str> = items
            .iter()
            .filter(|c| matches!(c.kind, CompletionKind::Variable | CompletionKind::Function))
            .map(|c| c.label.as_str())
            .collect();
        assert_eq!(symbol_labels, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn keyword_order_is_declaration_order() {
        let items = cs("");
        let keyword_labels: Vec<&str> = items
            .iter()
            .filter(|c| c.kind == CompletionKind::Keyword)
            .map(|c| c.label.as_str())
            .collect();
        // Must match BUILTIN_KEYWORDS verbatim.
        assert_eq!(keyword_labels, BUILTIN_KEYWORDS.to_vec());
    }

    // ---------- User symbols ----------

    #[test]
    fn user_function_has_signature_detail() {
        let items = cs("(define (square x) (* x x))");
        let square = items.iter().find(|c| c.label == "square").unwrap();
        assert_eq!(square.kind, CompletionKind::Function);
        assert_eq!(square.detail.as_deref(), Some("(x)"));
    }

    #[test]
    fn user_variable_has_no_detail() {
        let items = cs("(define greeting 'hello)");
        let g = items.iter().find(|c| c.label == "greeting").unwrap();
        assert_eq!(g.kind, CompletionKind::Variable);
        assert!(g.detail.is_none());
    }

    // ---------- Prefix filtering ----------

    #[test]
    fn empty_prefix_keeps_everything() {
        let with_none = cs("(define x 1)");
        let with_empty = cs_prefix("(define x 1)", "");
        assert_eq!(with_none.len(), with_empty.len());
    }

    #[test]
    fn prefix_filters_to_matching_items_only() {
        let items = cs_prefix("(define apple 1) (define banana 2)", "ap");
        let lbls = labels(&items);
        assert_eq!(lbls, vec!["apple"]);
    }

    #[test]
    fn prefix_matches_keywords_too() {
        let items = cs_prefix("", "le");
        let lbls = labels(&items);
        assert_eq!(lbls, vec!["let"]);
    }

    #[test]
    fn prefix_matches_constants_too() {
        let items = cs_prefix("", "#");
        let lbls = labels(&items);
        // #t and #f.  nil starts with `n`, not `#`.
        assert_eq!(lbls, vec!["#t", "#f"]);
    }

    #[test]
    fn prefix_no_match_returns_empty() {
        let items = cs_prefix("(define x 1)", "zzz");
        assert!(items.is_empty());
    }

    #[test]
    fn prefix_is_case_sensitive() {
        let items = cs_prefix("(define MyVar 1)", "my");
        // `MyVar` doesn't start with lowercase `my`.
        let lbls = labels(&items);
        assert!(!lbls.contains(&"MyVar"));
    }

    // ---------- Errors ----------

    #[test]
    fn unparseable_input_returns_parse_error() {
        let err = completions("(unbalanced", None).unwrap_err();
        let _ = err;
    }

    // ---------- completions_for_program direct path ----------

    #[test]
    fn completions_for_program_skips_parse() {
        let p = twig_parser::parse("(define x 1)").expect("parse");
        let items = completions_for_program(&p, None);
        assert!(items.iter().any(|c| c.label == "x"));
    }

    #[test]
    fn completions_for_program_with_prefix() {
        let p = twig_parser::parse("(define apple 1) (define banana 2)").expect("parse");
        let items = completions_for_program(&p, Some("ban"));
        assert_eq!(labels(&items), vec!["banana"]);
    }

    // ---------- Realistic ----------

    #[test]
    fn realistic_program_yields_expected_menu() {
        let src = "(define greeting 'hello)\n\
                   (define (square x) (* x x))\n\
                   (define (factorial n) (if (= n 0) 1 (* n (factorial (- n 1)))))\n\
                   (define pi 3)";
        let items = cs(src);
        // 6 keywords + 3 constants + 4 user symbols
        assert_eq!(items.len(), 6 + 3 + 4);

        // User symbols sorted: factorial, greeting, pi, square.
        let user: Vec<&CompletionItem> = items
            .iter()
            .filter(|c| {
                matches!(c.kind, CompletionKind::Function | CompletionKind::Variable)
            })
            .collect();
        let user_labels: Vec<&str> = user.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(user_labels, vec!["factorial", "greeting", "pi", "square"]);

        // factorial is a function with detail "(n)".
        let fact = user.iter().find(|c| c.label == "factorial").unwrap();
        assert_eq!(fact.kind, CompletionKind::Function);
        assert_eq!(fact.detail.as_deref(), Some("(n)"));
    }

    // ---------- Determinism ----------

    #[test]
    fn output_is_deterministic_across_calls() {
        let src = "(define z 1) (define a 2) (define m 3)";
        let a = cs(src);
        let b = cs(src);
        assert_eq!(a, b);
    }
}
