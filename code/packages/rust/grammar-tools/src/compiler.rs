//! # Compiler — generate Rust source from parsed grammar objects.
//!
//! The grammar-tools library parses `.tokens` and `.grammar` files into
//! in-memory data structures.  This module adds the *compile* step: given
//! a parsed grammar object, generate Rust source code that instantiates
//! the grammar as native Rust structs — no file I/O or parsing at runtime.
//!
//! ## Why compile grammars?
//!
//! The default workflow reads `.tokens` and `.grammar` files at startup.
//! This has three costs that compilation eliminates:
//!
//! 1. **File I/O at startup** — every process must find and open the files.
//!    Packages walk up the directory tree to find `code/grammars/`, which
//!    couples them to the repo layout.
//!
//! 2. **Parse overhead at startup** — the grammar is re-parsed every run.
//!
//! 3. **Deployment coupling** — `.tokens` and `.grammar` files must ship
//!    alongside the compiled binary.
//!
//! The generated Rust file exposes a function (`pub fn token_grammar() ->
//! TokenGrammar`) that callers can invoke directly instead of calling
//! `parse_token_grammar`.  Functions are used (rather than `static`) because
//! `HashMap` cannot be initialized in a `const` or static context without
//! an external crate like `once_cell`.
//!
//! ## Generated output shape (json.tokens → json_tokens.rs)
//!
//! ```text
//! // AUTO-GENERATED FILE — DO NOT EDIT
//! // Source: json.tokens
//!
//! #[allow(unused_imports)]
//! use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
//! #[allow(unused_imports)]
//! use std::collections::HashMap;
//!
//! pub fn token_grammar() -> TokenGrammar {
//!     TokenGrammar {
//!         definitions: vec![
//!             TokenDefinition {
//!                 name: "STRING".to_string(),
//!                 pattern: r#"..."#.to_string(),
//!                 is_regex: true,
//!                 line_number: 1,
//!                 alias: None,
//!             },
//!         ],
//!         keywords: vec![],
//!         // ...
//!     }
//! }
//! ```

use crate::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};
use crate::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};

// ===========================================================================
// Public API
// ===========================================================================

/// Generate Rust source code embedding a `TokenGrammar` as native data.
///
/// # Parameters
///
/// - `grammar`     — the `TokenGrammar` to compile.
/// - `source_file` — the original `.tokens` filename for the header comment.
///   Pass `""` to omit the Source line.
///
/// Returns a `String` of valid Rust source code.  Write it to a `.rs` file.
pub fn compile_token_grammar(grammar: &TokenGrammar, source_file: &str) -> String {
    // Strip newlines so a crafted filename cannot break out of the comment line
    // and inject arbitrary code into the generated file.
    let source_file = source_file.replace('\n', "_").replace('\r', "_");
    let source_file = source_file.as_str();
    let source_line = if source_file.is_empty() {
        String::new()
    } else {
        format!("// Source: {}\n", source_file)
    };

    let defs_src = token_def_vec_src(&grammar.definitions, "        ");
    let skip_src = token_def_vec_src(&grammar.skip_definitions, "        ");
    let err_src = token_def_vec_src(&grammar.error_definitions, "        ");
    let groups_src = groups_src(&grammar.groups, "        ");
    let mode_src = option_str_src(&grammar.mode);
    let escapes_src = option_str_src(&grammar.escapes);

    format!(
        "\
// AUTO-GENERATED FILE \u{2014} DO NOT EDIT
{source_line}// Regenerate with: grammar-tools compile-tokens {source_file}
//
// This file embeds a TokenGrammar as native Rust data structures.
// Call `token_grammar()` instead of reading and parsing the .tokens file.

#[allow(unused_imports)]
use grammar_tools::token_grammar::{{PatternGroup, TokenDefinition, TokenGrammar}};
#[allow(unused_imports)]
use std::collections::HashMap;

pub fn token_grammar() -> TokenGrammar {{
    TokenGrammar {{
        definitions: {defs_src},
        keywords: {kw_src},
        mode: {mode_src},
        skip_definitions: {skip_src},
        reserved_keywords: {rk_src},
        escapes: {escapes_src},
        error_definitions: {err_src},
        groups: {groups_src},
        case_sensitive: {case_sensitive},
        version: {version},
        case_insensitive: {case_insensitive},
        context_keywords: {context_kw_src},
        soft_keywords: {soft_kw_src},
        layout_keywords: {layout_kw_src},
    }}
}}
",
        source_line = source_line,
        source_file = source_file,
        defs_src = defs_src,
        kw_src = string_vec_src(&grammar.keywords),
        mode_src = mode_src,
        skip_src = skip_src,
        rk_src = string_vec_src(&grammar.reserved_keywords),
        escapes_src = escapes_src,
        err_src = err_src,
        groups_src = groups_src,
        case_sensitive = grammar.case_sensitive,
        version = grammar.version,
        case_insensitive = grammar.case_insensitive,
        context_kw_src = string_vec_src(&grammar.context_keywords),
        soft_kw_src = string_vec_src(&grammar.soft_keywords),
        layout_kw_src = string_vec_src(&grammar.layout_keywords),
    )
}

/// Generate Rust source code embedding a `ParserGrammar` as native data.
///
/// # Parameters
///
/// - `grammar`     — the `ParserGrammar` to compile.
/// - `source_file` — the original `.grammar` filename for the header comment.
///
/// Returns a `String` of valid Rust source code.
pub fn compile_parser_grammar(grammar: &ParserGrammar, source_file: &str) -> String {
    // Strip newlines so a crafted filename cannot break out of the comment line.
    let source_file = source_file.replace('\n', "_").replace('\r', "_");
    let source_file = source_file.as_str();
    let source_line = if source_file.is_empty() {
        String::new()
    } else {
        format!("// Source: {}\n", source_file)
    };

    let rules_src = if grammar.rules.is_empty() {
        "vec![]".to_string()
    } else {
        let rule_lines: Vec<String> = grammar
            .rules
            .iter()
            .map(|r| grammar_rule_src(r, "        "))
            .collect();
        format!("vec![\n{},\n    ]", rule_lines.join(",\n"))
    };

    format!(
        "\
// AUTO-GENERATED FILE \u{2014} DO NOT EDIT
{source_line}// Regenerate with: grammar-tools compile-grammar {source_file}
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{{GrammarElement, GrammarRule, ParserGrammar}};

pub fn parser_grammar() -> ParserGrammar {{
    ParserGrammar {{
        rules: {rules_src},
        version: {version},
    }}
}}
",
        source_line = source_line,
        source_file = source_file,
        rules_src = rules_src,
        version = grammar.version,
    )
}

// ===========================================================================
// Token grammar helpers
// ===========================================================================

/// Render a `String` as a Rust string literal.
///
/// Prefers a raw string literal (`` r#"..."# ``) to avoid backslash clutter
/// in patterns, but falls back to a regular string literal if the raw string
/// would be ambiguous (i.e. if the value contains `"#`).
fn rust_string_lit(s: &str) -> String {
    // Raw string r#"..."# is invalid when the content contains `"#`.
    if !s.contains("\"#") {
        format!("r#\"{}\"#", s)
    } else {
        // Escape the string the standard way.
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("\"{}\"", escaped)
    }
}

/// Render an `Option<String>` as a Rust expression (`None` or `Some(...)`).
fn option_str_src(opt: &Option<String>) -> String {
    match opt {
        None => "None".to_string(),
        Some(s) => format!("Some({}.to_string())", rust_string_lit(s)),
    }
}

/// Render a `Vec<String>` as a Rust `vec![...]` expression.
fn string_vec_src(v: &[String]) -> String {
    if v.is_empty() {
        return "vec![]".to_string();
    }
    let items: Vec<String> = v
        .iter()
        .map(|s| format!("{}.to_string()", rust_string_lit(s)))
        .collect();
    format!("vec![{}]", items.join(", "))
}

/// Render one `TokenDefinition` as a Rust struct expression.
fn token_def_src(defn: &TokenDefinition, indent: &str) -> String {
    let alias_src = match &defn.alias {
        None => "None".to_string(),
        Some(a) => format!("Some({}.to_string())", rust_string_lit(a)),
    };
    format!(
        "{indent}TokenDefinition {{\n\
         {indent}    name: {name}.to_string(),\n\
         {indent}    pattern: {pattern}.to_string(),\n\
         {indent}    is_regex: {is_regex},\n\
         {indent}    line_number: {line_number},\n\
         {indent}    alias: {alias},\n\
         {indent}}}",
        indent = indent,
        name = rust_string_lit(&defn.name),
        pattern = rust_string_lit(&defn.pattern),
        is_regex = defn.is_regex,
        line_number = defn.line_number,
        alias = alias_src,
    )
}

/// Render a `Vec<TokenDefinition>` as a Rust `vec![...]` expression.
fn token_def_vec_src(defs: &[TokenDefinition], indent: &str) -> String {
    if defs.is_empty() {
        return "vec![]".to_string();
    }
    let inner = format!("{}    ", indent);
    let items: Vec<String> = defs.iter().map(|d| token_def_src(d, &inner)).collect();
    format!("vec![\n{},\n{}]", items.join(",\n"), indent)
}

/// Render a `HashMap<String, PatternGroup>` as a Rust expression.
fn groups_src(groups: &std::collections::HashMap<String, PatternGroup>, indent: &str) -> String {
    if groups.is_empty() {
        return "HashMap::new()".to_string();
    }
    let inner = format!("{}    ", indent);
    let inner2 = format!("{}        ", indent);
    let mut entries: Vec<String> = groups
        .iter()
        .map(|(name, group)| {
            let defs_lit = token_def_vec_src(&group.definitions, &inner2);
            // Sanitize the group name to a valid Rust identifier fragment:
            // keep only ASCII alphanumeric chars and underscores, replace all
            // others with '_'.  This prevents a crafted group name from
            // breaking out of the `let mut __g_<safe>` identifier context.
            let safe: String = name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
                .collect();
            format!(
                "{inner}let mut __g_{safe} = PatternGroup {{ name: {name}.to_string(), definitions: {defs} }};\n\
                 {inner}__map.insert({name}.to_string(), __g_{safe});",
                inner = inner,
                safe = safe,
                name = rust_string_lit(name),
                defs = defs_lit,
            )
        })
        .collect();
    // Sort for determinism
    entries.sort();
    format!(
        "{{\n\
         {inner}    let mut __map: HashMap<String, PatternGroup> = HashMap::new();\n\
         {entries}\n\
         {inner}    __map\n\
         {indent}}}",
        inner = indent,
        entries = entries.join("\n"),
        indent = indent,
    )
}

// ===========================================================================
// Parser grammar helpers
// ===========================================================================

/// Render one `GrammarRule` as a Rust struct expression.
fn grammar_rule_src(rule: &GrammarRule, indent: &str) -> String {
    let body_src = element_src(&rule.body, &format!("{}    ", indent));
    format!(
        "{indent}GrammarRule {{\n\
         {indent}    name: {name}.to_string(),\n\
         {indent}    body: {body},\n\
         {indent}    line_number: {line_number},\n\
         {indent}}}",
        indent = indent,
        name = rust_string_lit(&rule.name),
        body = body_src,
        line_number = rule.line_number,
    )
}

/// Recursively render a `GrammarElement` as a Rust enum constructor expression.
///
/// Rust's enum variants carry their own data, so there is no need for a
/// `type:` discriminant field — we just write the variant name directly.
fn element_src(element: &GrammarElement, indent: &str) -> String {
    let i = format!("{}    ", indent);
    match element {
        GrammarElement::RuleReference { name } => {
            format!(
                "GrammarElement::RuleReference {{ name: {}.to_string() }}",
                rust_string_lit(name)
            )
        }
        GrammarElement::TokenReference { name } => {
            format!(
                "GrammarElement::TokenReference {{ name: {}.to_string() }}",
                rust_string_lit(name)
            )
        }
        GrammarElement::Literal { value } => {
            format!(
                "GrammarElement::Literal {{ value: {}.to_string() }}",
                rust_string_lit(value)
            )
        }
        GrammarElement::Sequence { elements } => {
            let items: Vec<String> = elements
                .iter()
                .map(|e| format!("{}{}", i, element_src(e, &i)))
                .collect();
            format!(
                "GrammarElement::Sequence {{ elements: vec![\n{},\n{}] }}",
                items.join(",\n"),
                indent
            )
        }
        GrammarElement::Alternation { choices } => {
            let items: Vec<String> = choices
                .iter()
                .map(|c| format!("{}{}", i, element_src(c, &i)))
                .collect();
            format!(
                "GrammarElement::Alternation {{ choices: vec![\n{},\n{}] }}",
                items.join(",\n"),
                indent
            )
        }
        GrammarElement::Repetition { element } => {
            let child = element_src(element, &i);
            format!(
                "GrammarElement::Repetition {{ element: Box::new({}) }}",
                child
            )
        }
        GrammarElement::Optional { element } => {
            let child = element_src(element, &i);
            format!(
                "GrammarElement::Optional {{ element: Box::new({}) }}",
                child
            )
        }
        GrammarElement::Group { element } => {
            let child = element_src(element, &i);
            format!("GrammarElement::Group {{ element: Box::new({}) }}", child)
        }
        GrammarElement::PositiveLookahead { element } => {
            let child = element_src(element, &i);
            format!(
                "GrammarElement::PositiveLookahead {{ element: Box::new({}) }}",
                child
            )
        }
        GrammarElement::NegativeLookahead { element } => {
            let child = element_src(element, &i);
            format!(
                "GrammarElement::NegativeLookahead {{ element: Box::new({}) }}",
                child
            )
        }
        GrammarElement::OneOrMore { element } => {
            let child = element_src(element, &i);
            format!(
                "GrammarElement::OneOrMore {{ element: Box::new({}) }}",
                child
            )
        }
        GrammarElement::SeparatedRepetition {
            element,
            separator,
            at_least_one,
        } => {
            let elem_child = element_src(element, &i);
            let sep_child = element_src(separator, &i);
            format!(
                "GrammarElement::SeparatedRepetition {{ element: Box::new({}), separator: Box::new({}), at_least_one: {} }}",
                elem_child, sep_child, at_least_one
            )
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_grammar::parse_parser_grammar;
    use crate::token_grammar::parse_token_grammar;

    // -----------------------------------------------------------------------
    // compile_token_grammar — output structure checks
    // -----------------------------------------------------------------------

    #[test]
    fn token_grammar_do_not_edit_header() {
        let g = parse_token_grammar("").unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("DO NOT EDIT"));
    }

    #[test]
    fn token_grammar_source_line_present() {
        let g = parse_token_grammar("").unwrap();
        let code = compile_token_grammar(&g, "json.tokens");
        assert!(code.contains("json.tokens"));
    }

    #[test]
    fn token_grammar_source_line_omitted_when_empty() {
        let g = parse_token_grammar("").unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(!code.contains("// Source:"));
    }

    #[test]
    fn token_grammar_includes_use_statement() {
        let g = parse_token_grammar("").unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("use grammar_tools::token_grammar"));
    }

    #[test]
    fn token_grammar_function_name() {
        let g = parse_token_grammar("").unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("pub fn token_grammar()"));
    }

    // -----------------------------------------------------------------------
    // compile_token_grammar — field content checks
    // -----------------------------------------------------------------------

    #[test]
    fn token_grammar_regex_token_name_and_pattern() {
        let g = parse_token_grammar("NUMBER = /[0-9]+/").unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("NUMBER"));
        assert!(code.contains("[0-9]+"));
        assert!(code.contains("is_regex: true"));
    }

    #[test]
    fn token_grammar_literal_token() {
        let g = parse_token_grammar(r#"PLUS = "+""#).unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("PLUS"));
        assert!(code.contains('+'));
        assert!(code.contains("is_regex: false"));
    }

    #[test]
    fn token_grammar_alias() {
        let g = parse_token_grammar(r#"STRING_DQ = /"[^"]*"/ -> STRING"#).unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("STRING"));
    }

    #[test]
    fn token_grammar_keywords() {
        let source = "NAME = /[a-z]+/\nkeywords:\n  if\n  else\n";
        let g = parse_token_grammar(source).unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("if"));
        assert!(code.contains("else"));
    }

    #[test]
    fn token_grammar_skip_definitions() {
        let source = "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n";
        let g = parse_token_grammar(source).unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("WHITESPACE"));
    }

    #[test]
    fn token_grammar_version() {
        let source = "# @version 5\nNAME = /[a-z]+/";
        let g = parse_token_grammar(source).unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("version: 5"));
    }

    #[test]
    fn token_grammar_case_insensitive() {
        let source = "# @case_insensitive true\nNAME = /[a-z]+/";
        let g = parse_token_grammar(source).unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("case_insensitive: true"));
    }

    #[test]
    fn token_grammar_groups() {
        let source = "TEXT = /[^<]+/\ngroup tag:\n  ATTR = /[a-z]+/\n";
        let g = parse_token_grammar(source).unwrap();
        let code = compile_token_grammar(&g, "");
        assert!(code.contains("tag"));
        assert!(code.contains("ATTR"));
    }

    // -----------------------------------------------------------------------
    // compile_parser_grammar — output structure checks
    // -----------------------------------------------------------------------

    #[test]
    fn parser_grammar_do_not_edit_header() {
        let g = parse_parser_grammar("").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("DO NOT EDIT"));
    }

    #[test]
    fn parser_grammar_function_name() {
        let g = parse_parser_grammar("").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("pub fn parser_grammar()"));
    }

    #[test]
    fn parser_grammar_includes_use_statement() {
        let g = parse_parser_grammar("").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("use grammar_tools::parser_grammar"));
    }

    // -----------------------------------------------------------------------
    // compile_parser_grammar — element type checks
    // -----------------------------------------------------------------------

    #[test]
    fn parser_grammar_token_reference() {
        let g = parse_parser_grammar("value = NUMBER ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("TokenReference"));
        assert!(code.contains("NUMBER"));
    }

    #[test]
    fn parser_grammar_rule_reference() {
        let g = parse_parser_grammar("program = expr ;\nexpr = NUMBER ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("RuleReference"));
        assert!(code.contains("expr"));
    }

    #[test]
    fn parser_grammar_alternation() {
        let g = parse_parser_grammar("value = A | B | C ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("Alternation"));
    }

    #[test]
    fn parser_grammar_sequence() {
        let g = parse_parser_grammar("pair = KEY COLON VALUE ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("Sequence"));
    }

    #[test]
    fn parser_grammar_repetition() {
        let g = parse_parser_grammar("stmts = { stmt } ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("Repetition"));
    }

    #[test]
    fn parser_grammar_optional() {
        let g = parse_parser_grammar("expr = NUMBER [ PLUS NUMBER ] ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("Optional"));
    }

    #[test]
    fn parser_grammar_literal() {
        let g = parse_parser_grammar(r#"start = "hello" ;"#).unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("Literal"));
        assert!(code.contains("hello"));
    }

    #[test]
    fn parser_grammar_group() {
        let g = parse_parser_grammar("expr = ( A | B ) ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("Group"));
    }

    #[test]
    fn parser_grammar_version() {
        let g = parse_parser_grammar("# @version 4\nvalue = NUMBER ;").unwrap();
        let code = compile_parser_grammar(&g, "");
        assert!(code.contains("version: 4"));
    }
}
