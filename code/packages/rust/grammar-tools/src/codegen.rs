//! # Codegen — emit Rust source code that reconstructs a parsed grammar.
//!
//! Language frontends (twig-lexer, twig-parser, brainfuck, dartmouth-basic,
//! …) historically read their `.tokens` and `.grammar` files at runtime via
//! `std::fs::read_to_string`.  That worked for educational use but had
//! three drawbacks:
//!
//!   1. The runtime depended on the grammar file existing at a known path.
//!   2. Each lexer/parser construction paid a parse cost.
//!   3. Sandboxed tooling (Miri's isolation mode, embedded targets,
//!      WASM with no FS) couldn't run the lexer at all.
//!
//! This module fixes all three.  It takes a parsed [`TokenGrammar`] or
//! [`ParserGrammar`] and emits a Rust source string that, when compiled
//! into a consumer crate, reconstructs the grammar as a `&'static`
//! reference via `OnceLock`-cached initialisation.  Consumer crates put a
//! `build.rs` in their root that calls these functions and writes the
//! result to `$OUT_DIR`; their `lib.rs` then `include!`s the generated
//! file.
//!
//! ## Output shape
//!
//! [`token_grammar_to_rust_source`] produces a function declaration:
//!
//! ```text
//! pub fn <fn_name>() -> &'static ::grammar_tools::token_grammar::TokenGrammar {
//!     use std::sync::OnceLock;
//!     static GRAMMAR: OnceLock<::grammar_tools::token_grammar::TokenGrammar> =
//!         OnceLock::new();
//!     GRAMMAR.get_or_init(|| ::grammar_tools::token_grammar::TokenGrammar {
//!         definitions: vec![ /* ... */ ],
//!         /* ... */
//!     })
//! }
//! ```
//!
//! [`parser_grammar_to_rust_source`] produces an analogous function for
//! [`ParserGrammar`].
//!
//! ## Why fully-qualified paths
//!
//! The generated code is `include!`d into a consumer crate that may or
//! may not have `grammar_tools` imported.  Fully-qualified
//! `::grammar_tools::…` paths let the generated code work regardless of
//! the consumer's `use` statements.  The leading `::` ensures absolute
//! path resolution rather than searching the current module.
//!
//! ## Why a function rather than a `static LazyLock`
//!
//! `LazyLock` is stable since Rust 1.80 but slightly newer than `OnceLock`
//! (stable 1.70).  Using `OnceLock` keeps the MSRV slightly lower for
//! consumer crates.  The function form also gives consumers a clean call
//! site (`twig_token_grammar()`) rather than `&*TWIG_TOKEN_GRAMMAR`.
//!
//! ## Future direction
//!
//! A subsequent PR can flatten the generated initialiser to skip even the
//! `OnceLock` and parse, by emitting struct literals into a `static` —
//! eliminating the one-time parse cost entirely.  PR 3 stops at OnceLock
//! because it already eliminates file I/O and Miri compatibility issues,
//! which is the user-visible benefit.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};
use crate::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};

// ===========================================================================
// String escaping
// ===========================================================================

/// Escape a string for embedding inside a Rust raw string literal `"…"`.
///
/// Handles the four characters Rust string literals require to be escaped:
/// `\`, `"`, newlines, and carriage returns.  Tab and other control
/// characters survive — Rust's compiler handles them fine inside `"…"`.
fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            // Tab and other control chars round-trip via the escaped
            // sequence; printable chars pass through verbatim.
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// `"…".to_string()` — the canonical way to materialise a `String` literal
/// in generated code.  The grammar structs are owned, so we always emit
/// owned `String`s.
fn rust_string_literal(s: &str) -> String {
    format!("\"{}\".to_string()", escape_str(s))
}

/// Render `Option<String>` as `Some("...".to_string())` or `None`.
fn rust_optional_string(opt: &Option<String>) -> String {
    match opt {
        Some(s) => format!("Some({})", rust_string_literal(s)),
        None => "None".to_string(),
    }
}

/// Render `Vec<String>` as `vec!["a".to_string(), "b".to_string()]`.
fn rust_vec_string(v: &[String]) -> String {
    if v.is_empty() {
        return "Vec::new()".to_string();
    }
    let mut out = String::from("vec![");
    for (i, s) in v.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&rust_string_literal(s));
    }
    out.push(']');
    out
}

// ===========================================================================
// TokenGrammar codegen
// ===========================================================================

/// Render a [`TokenDefinition`] as a Rust struct literal expression.
fn render_token_def(d: &TokenDefinition) -> String {
    format!(
        "::grammar_tools::token_grammar::TokenDefinition {{ \
         name: {name}, pattern: {pattern}, is_regex: {is_regex}, \
         line_number: {line_number}usize, alias: {alias} }}",
        name = rust_string_literal(&d.name),
        pattern = rust_string_literal(&d.pattern),
        is_regex = d.is_regex,
        line_number = d.line_number,
        alias = rust_optional_string(&d.alias),
    )
}

/// Render `Vec<TokenDefinition>` as a `vec![...]`.
fn render_token_def_vec(defs: &[TokenDefinition]) -> String {
    if defs.is_empty() {
        return "Vec::new()".to_string();
    }
    let mut out = String::from("vec![");
    for (i, d) in defs.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n            ");
        }
        out.push_str(&render_token_def(d));
    }
    out.push(']');
    out
}

/// Render a [`PatternGroup`] as a Rust struct literal.
fn render_pattern_group(g: &PatternGroup) -> String {
    format!(
        "::grammar_tools::token_grammar::PatternGroup {{ \
         name: {name}, definitions: {defs} }}",
        name = rust_string_literal(&g.name),
        defs = render_token_def_vec(&g.definitions),
    )
}

/// Render `HashMap<String, PatternGroup>` as a `let mut m = HashMap::new();
/// m.insert(...); m.insert(...); m` block.
fn render_groups(groups: &HashMap<String, PatternGroup>) -> String {
    if groups.is_empty() {
        return "::std::collections::HashMap::new()".to_string();
    }
    // Stable iteration order so generated output is reproducible — the
    // build cache hashes the generated file, so non-determinism would
    // bust the cache on every build.
    let mut keys: Vec<&String> = groups.keys().collect();
    keys.sort();
    let mut out = String::from("{ let mut __m = ::std::collections::HashMap::new();");
    for k in keys {
        let _ = write!(
            out,
            " __m.insert({}, {});",
            rust_string_literal(k),
            render_pattern_group(&groups[k]),
        );
    }
    out.push_str(" __m }");
    out
}

/// Emit Rust source code that reconstructs `grammar` and exposes it via
/// a `pub fn <fn_name>() -> &'static TokenGrammar`.
///
/// The output is a complete, self-contained Rust source file.  Consumer
/// crates' `build.rs` writes this string to `$OUT_DIR` and the consumer's
/// `lib.rs` `include!`s it.
///
/// # Arguments
///
/// - `grammar` — the parsed grammar to emit.
/// - `fn_name` — the name of the generated function (e.g. `"twig_token_grammar"`).
///   Must be a valid Rust identifier; the function is generated unchecked,
///   so passing junk here produces a compile error in the consumer.
pub fn token_grammar_to_rust_source(grammar: &TokenGrammar, fn_name: &str) -> String {
    let mut out = String::with_capacity(4096);
    let _ = writeln!(
        out,
        "// AUTO-GENERATED by grammar_tools::codegen::token_grammar_to_rust_source.\n\
         // Do not edit by hand — re-run the consuming crate's build.rs to regenerate.\n\
         pub fn {fn_name}() -> &'static ::grammar_tools::token_grammar::TokenGrammar {{",
    );
    let _ = writeln!(
        out,
        "    use ::std::sync::OnceLock;\n    \
         static GRAMMAR: OnceLock<::grammar_tools::token_grammar::TokenGrammar> = OnceLock::new();\n    \
         GRAMMAR.get_or_init(|| ::grammar_tools::token_grammar::TokenGrammar {{",
    );
    let _ = writeln!(out, "        definitions: {},", render_token_def_vec(&grammar.definitions));
    let _ = writeln!(out, "        keywords: {},", rust_vec_string(&grammar.keywords));
    let _ = writeln!(out, "        mode: {},", rust_optional_string(&grammar.mode));
    let _ = writeln!(out, "        skip_definitions: {},", render_token_def_vec(&grammar.skip_definitions));
    let _ = writeln!(out, "        reserved_keywords: {},", rust_vec_string(&grammar.reserved_keywords));
    let _ = writeln!(out, "        escapes: {},", rust_optional_string(&grammar.escapes));
    let _ = writeln!(out, "        error_definitions: {},", render_token_def_vec(&grammar.error_definitions));
    let _ = writeln!(out, "        groups: {},", render_groups(&grammar.groups));
    let _ = writeln!(out, "        case_sensitive: {},", grammar.case_sensitive);
    let _ = writeln!(out, "        version: {}u32,", grammar.version);
    let _ = writeln!(out, "        case_insensitive: {},", grammar.case_insensitive);
    let _ = writeln!(out, "        context_keywords: {},", rust_vec_string(&grammar.context_keywords));
    let _ = writeln!(out, "        soft_keywords: {},", rust_vec_string(&grammar.soft_keywords));
    let _ = writeln!(out, "        layout_keywords: {},", rust_vec_string(&grammar.layout_keywords));
    out.push_str("    })\n}\n");
    out
}

// ===========================================================================
// ParserGrammar codegen
// ===========================================================================

/// Render a [`GrammarElement`] as a Rust expression.  Recursive: every
/// variant carrying a `Box<GrammarElement>` or `Vec<GrammarElement>` walks
/// into its children before emitting.
fn render_grammar_element(e: &GrammarElement) -> String {
    use GrammarElement::*;
    let path = "::grammar_tools::parser_grammar::GrammarElement";
    match e {
        RuleReference { name } => {
            format!("{path}::RuleReference {{ name: {} }}", rust_string_literal(name))
        }
        TokenReference { name } => {
            format!("{path}::TokenReference {{ name: {} }}", rust_string_literal(name))
        }
        Literal { value } => {
            format!("{path}::Literal {{ value: {} }}", rust_string_literal(value))
        }
        Sequence { elements } => {
            format!(
                "{path}::Sequence {{ elements: {} }}",
                render_element_vec(elements),
            )
        }
        Alternation { choices } => {
            format!(
                "{path}::Alternation {{ choices: {} }}",
                render_element_vec(choices),
            )
        }
        Repetition { element } => {
            format!(
                "{path}::Repetition {{ element: ::std::boxed::Box::new({}) }}",
                render_grammar_element(element),
            )
        }
        Optional { element } => {
            format!(
                "{path}::Optional {{ element: ::std::boxed::Box::new({}) }}",
                render_grammar_element(element),
            )
        }
        Group { element } => {
            format!(
                "{path}::Group {{ element: ::std::boxed::Box::new({}) }}",
                render_grammar_element(element),
            )
        }
        PositiveLookahead { element } => {
            format!(
                "{path}::PositiveLookahead {{ element: ::std::boxed::Box::new({}) }}",
                render_grammar_element(element),
            )
        }
        NegativeLookahead { element } => {
            format!(
                "{path}::NegativeLookahead {{ element: ::std::boxed::Box::new({}) }}",
                render_grammar_element(element),
            )
        }
        OneOrMore { element } => {
            format!(
                "{path}::OneOrMore {{ element: ::std::boxed::Box::new({}) }}",
                render_grammar_element(element),
            )
        }
        SeparatedRepetition { element, separator, at_least_one } => {
            format!(
                "{path}::SeparatedRepetition {{ \
                 element: ::std::boxed::Box::new({}), \
                 separator: ::std::boxed::Box::new({}), \
                 at_least_one: {} }}",
                render_grammar_element(element),
                render_grammar_element(separator),
                at_least_one,
            )
        }
    }
}

fn render_element_vec(es: &[GrammarElement]) -> String {
    if es.is_empty() {
        return "Vec::new()".to_string();
    }
    let mut out = String::from("vec![");
    for (i, e) in es.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&render_grammar_element(e));
    }
    out.push(']');
    out
}

fn render_grammar_rule(r: &GrammarRule) -> String {
    format!(
        "::grammar_tools::parser_grammar::GrammarRule {{ \
         name: {name}, body: {body}, line_number: {ln}usize }}",
        name = rust_string_literal(&r.name),
        body = render_grammar_element(&r.body),
        ln = r.line_number,
    )
}

fn render_rule_vec(rules: &[GrammarRule]) -> String {
    if rules.is_empty() {
        return "Vec::new()".to_string();
    }
    let mut out = String::from("vec![");
    for (i, r) in rules.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n            ");
        }
        out.push_str(&render_grammar_rule(r));
    }
    out.push(']');
    out
}

/// Emit Rust source code that reconstructs `grammar` and exposes it via
/// a `pub fn <fn_name>() -> &'static ParserGrammar`.
///
/// See [`token_grammar_to_rust_source`] for the output shape — this
/// function is the same idea applied to [`ParserGrammar`].
pub fn parser_grammar_to_rust_source(grammar: &ParserGrammar, fn_name: &str) -> String {
    let mut out = String::with_capacity(4096);
    let _ = writeln!(
        out,
        "// AUTO-GENERATED by grammar_tools::codegen::parser_grammar_to_rust_source.\n\
         // Do not edit by hand — re-run the consuming crate's build.rs to regenerate.\n\
         pub fn {fn_name}() -> &'static ::grammar_tools::parser_grammar::ParserGrammar {{",
    );
    let _ = writeln!(
        out,
        "    use ::std::sync::OnceLock;\n    \
         static GRAMMAR: OnceLock<::grammar_tools::parser_grammar::ParserGrammar> = OnceLock::new();\n    \
         GRAMMAR.get_or_init(|| ::grammar_tools::parser_grammar::ParserGrammar {{",
    );
    let _ = writeln!(out, "        rules: {},", render_rule_vec(&grammar.rules));
    let _ = writeln!(out, "        version: {}u32,", grammar.version);
    out.push_str("    })\n}\n");
    out
}

// ===========================================================================
// Tests — round-trip a parsed grammar through codegen and back
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_grammar::parse_parser_grammar;
    use crate::token_grammar::parse_token_grammar;

    /// Round-trip rule: parse a grammar, codegen Rust source, **and** verify
    /// the source contains the expected struct path.  We can't compile the
    /// generated source inside a unit test (that needs a full crate
    /// compilation), but we can sanity-check the shape.
    #[test]
    fn token_grammar_codegen_emits_expected_shape() {
        let src = "\
NUMBER = /[0-9]+/
PLUS = \"+\"
keywords:
  if
  while
";
        let grammar = parse_token_grammar(src).unwrap();
        let rust = token_grammar_to_rust_source(&grammar, "test_grammar");

        // Function header
        assert!(rust.contains("pub fn test_grammar() -> &'static"));
        assert!(rust.contains("::grammar_tools::token_grammar::TokenGrammar"));
        // OnceLock infrastructure
        assert!(rust.contains("OnceLock"));
        assert!(rust.contains("get_or_init"));
        // Definitions field with the parsed tokens
        assert!(rust.contains(r#""NUMBER".to_string()"#));
        assert!(rust.contains(r#""PLUS".to_string()"#));
        // Keywords
        assert!(rust.contains(r#""if".to_string()"#));
        assert!(rust.contains(r#""while".to_string()"#));
    }

    #[test]
    fn token_grammar_codegen_handles_empty_fields() {
        let grammar = parse_token_grammar("FOO = \"foo\"").unwrap();
        let rust = token_grammar_to_rust_source(&grammar, "empty_grammar");
        // Empty Vec emits Vec::new(), not vec![]
        assert!(rust.contains("Vec::new()"));
        // Empty HashMap emits ::std::collections::HashMap::new()
        assert!(rust.contains("::std::collections::HashMap::new()"));
    }

    #[test]
    fn parser_grammar_codegen_emits_expected_shape() {
        let src = "expr = NUMBER ;";
        let grammar = parse_parser_grammar(src).unwrap();
        let rust = parser_grammar_to_rust_source(&grammar, "test_parser_grammar");

        assert!(rust.contains("pub fn test_parser_grammar() -> &'static"));
        assert!(rust.contains("::grammar_tools::parser_grammar::ParserGrammar"));
        assert!(rust.contains("::grammar_tools::parser_grammar::GrammarRule"));
        // The rule name appears
        assert!(rust.contains(r#""expr".to_string()"#));
        // The token reference is present
        assert!(rust.contains("TokenReference"));
        assert!(rust.contains(r#""NUMBER".to_string()"#));
    }

    #[test]
    fn parser_grammar_codegen_handles_recursive_elements() {
        // Sequence + Alternation + Repetition — exercises Box<GrammarElement>
        let src = "expr = a | { b c } ;";
        let grammar = parse_parser_grammar(src).unwrap();
        let rust = parser_grammar_to_rust_source(&grammar, "g");
        // Both the Alternation and the Repetition should show up
        assert!(rust.contains("Alternation"));
        assert!(rust.contains("Repetition"));
        assert!(rust.contains("Box::new"));
    }

    #[test]
    fn escape_handles_special_characters() {
        let s = "a\\b\"c\nd\re\tf\u{01}";
        let escaped = escape_str(s);
        assert!(escaped.contains("\\\\"), "backslash escaped: {escaped}");
        assert!(escaped.contains("\\\""), "quote escaped: {escaped}");
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\r"));
        assert!(escaped.contains("\\t"));
        assert!(escaped.contains("\\u{1}"), "control char escaped: {escaped}");
    }

    /// The tightest possible test: round-trip a real grammar through
    /// the codegen pipeline.  We can't compile + execute the generated
    /// code in a unit test, but a future integration test in a
    /// consumer crate (twig-lexer) will.  Here we just verify the
    /// generated source is non-empty and ends with a `}` (closing the
    /// function) — catches obvious truncation bugs.
    #[test]
    fn output_ends_with_closing_brace() {
        let g = parse_token_grammar("FOO = \"x\"").unwrap();
        let rust = token_grammar_to_rust_source(&g, "f");
        assert!(rust.trim_end().ends_with('}'));

        let pg = parse_parser_grammar("foo = FOO ;").unwrap();
        let rust2 = parser_grammar_to_rust_source(&pg, "f");
        assert!(rust2.trim_end().ends_with('}'));
    }

    #[test]
    fn groups_emitted_in_sorted_order() {
        // Reproducible builds need deterministic codegen output.
        // HashMap iteration order is random, so we sort keys before
        // emitting.  This test catches any future regression where
        // the sort gets dropped.
        let src = "
group inner:
  X = \"x\"
group alpha:
  Y = \"y\"
NAME = /[a-z]+/
";
        let grammar = parse_token_grammar(src).unwrap();
        let rust = token_grammar_to_rust_source(&grammar, "g");
        let alpha_pos = rust.find(r#""alpha".to_string()"#).expect("alpha key present");
        let inner_pos = rust.find(r#""inner".to_string()"#).expect("inner key present");
        assert!(alpha_pos < inner_pos, "groups must be emitted in sorted order");
    }
}
