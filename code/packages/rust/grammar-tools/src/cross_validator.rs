//! # Cross-Validator — checking consistency between `.tokens` and `.grammar` files.
//!
//! The whole point of having two separate grammar files is that they reference
//! each other: the `.grammar` file uses UPPERCASE names to refer to tokens
//! defined in the `.tokens` file. This module checks that the two files are
//! consistent.
//!
//! # Why cross-validate?
//!
//! Each file can be valid on its own but broken when used together:
//!
//! - A grammar might reference `SEMICOLON`, but the `.tokens` file only
//!   defines `SEMI`. Each file is fine individually, but the pair is broken.
//! - A `.tokens` file might define `TILDE = "~"` that no grammar rule ever
//!   uses. This is not an error — it might be intentional — but it is worth
//!   warning about because unused tokens add complexity without value.
//!
//! This is analogous to how a C compiler checks that every function you call
//! is actually declared (and vice versa, warns about unused functions).
//!
//! # What we check
//!
//! 1. **Missing token references**: Every UPPERCASE name in the grammar must
//!    correspond to a token definition. If not, the generated parser will try
//!    to match a token type that the lexer never produces.
//!
//! 2. **Unused tokens**: Every token defined in the `.tokens` file should ideally
//!    be referenced somewhere in the grammar. Unused tokens suggest either a
//!    typo or leftover cruft. We report these as warnings, not errors.

use crate::parser_grammar::{grammar_token_references, ParserGrammar};
use crate::token_grammar::{token_names, TokenGrammar};

/// Cross-validate a token grammar and a parser grammar.
///
/// Checks that every UPPERCASE name referenced in the parser grammar
/// exists in the token grammar, and warns about tokens that are defined
/// but never used.
///
/// # Arguments
///
/// - `token_grammar` — A parsed `.tokens` file.
/// - `parser_grammar` — A parsed `.grammar` file.
///
/// # Returns
///
/// A list of error/warning strings. Errors describe broken references;
/// warnings describe unused definitions. An empty list means the two
/// grammars are fully consistent.
///
/// # Example
///
/// ```
/// use grammar_tools::token_grammar::parse_token_grammar;
/// use grammar_tools::parser_grammar::parse_parser_grammar;
/// use grammar_tools::cross_validator::cross_validate;
///
/// let tokens = parse_token_grammar("NUMBER = /[0-9]+/\nPLUS = \"+\"").unwrap();
/// let grammar = parse_parser_grammar("expression = NUMBER { PLUS NUMBER } ;").unwrap();
/// let issues = cross_validate(&tokens, &grammar);
/// assert!(issues.is_empty()); // All tokens used, all references resolved.
/// ```
pub fn cross_validate(
    token_grammar: &TokenGrammar,
    parser_grammar: &ParserGrammar,
) -> Vec<String> {
    let mut issues = Vec::new();

    let mut defined_tokens = token_names(token_grammar);
    let referenced_tokens = grammar_token_references(parser_grammar);

    // --- Implicit tokens ---
    // EOF is always implicitly available (every token stream ends with it).
    defined_tokens.insert("EOF".to_string());

    // In indentation mode, INDENT/DEDENT/NEWLINE are synthesized by the
    // lexer and don't need to be defined in the .tokens file.
    if token_grammar.mode.as_deref() == Some("indentation") {
        defined_tokens.insert("INDENT".to_string());
        defined_tokens.insert("DEDENT".to_string());
        defined_tokens.insert("NEWLINE".to_string());
    }

    // --- Missing token references (errors) ---
    let mut sorted_refs: Vec<&String> = referenced_tokens.iter().collect();
    sorted_refs.sort();
    for ref_name in sorted_refs {
        if !defined_tokens.contains(ref_name.as_str()) {
            issues.push(format!(
                "Error: Grammar references token '{}' which is not defined in the tokens file",
                ref_name
            ));
        }
    }

    // --- Unused tokens (warnings) ---
    // A token counts as "used" if either its name or its alias is referenced.
    for defn in &token_grammar.definitions {
        let name_used = referenced_tokens.contains(&defn.name);
        let alias_used = defn.alias.as_ref().map_or(false, |a| referenced_tokens.contains(a));
        if !name_used && !alias_used {
            issues.push(format!(
                "Warning: Token '{}' (line {}) is defined but never used in the grammar",
                defn.name, defn.line_number
            ));
        }
    }

    issues
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
    // Happy paths — grammars are consistent
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_errors_when_all_references_resolve() {
        // When every token used in the grammar is defined, no errors.
        let tokens = parse_token_grammar(
            "NUMBER = /[0-9]+/\nPLUS = \"+\"\nNAME = /[a-zA-Z]+/\nLPAREN = \"(\"\nRPAREN = \")\"",
        ).unwrap();
        let grammar = parse_parser_grammar(
            "expression = term { PLUS term } ; term = NUMBER | NAME | LPAREN expression RPAREN ;",
        ).unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let errors: Vec<&String> = issues.iter().filter(|i| i.starts_with("Error")).collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_no_unused_warnings_when_all_tokens_used() {
        // When every token is used, no unused warnings.
        let tokens = parse_token_grammar("NUMBER = /[0-9]+/\nPLUS = \"+\"").unwrap();
        let grammar = parse_parser_grammar(
            "expression = NUMBER { PLUS NUMBER } ;",
        ).unwrap();
        let issues = cross_validate(&tokens, &grammar);
        assert!(issues.is_empty());
    }

    // -----------------------------------------------------------------------
    // Error cases — grammars are inconsistent
    // -----------------------------------------------------------------------

    #[test]
    fn test_missing_token_reference() {
        // A token referenced in the grammar but not in .tokens is an error.
        let tokens = parse_token_grammar("NUMBER = /[0-9]+/").unwrap();
        let grammar = parse_parser_grammar("expression = NUMBER PLUS NUMBER ;").unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let errors: Vec<&String> = issues.iter().filter(|i| i.starts_with("Error")).collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("PLUS"));
    }

    #[test]
    fn test_unused_token_warning() {
        // A token defined in .tokens but not used in the grammar is a warning.
        let tokens = parse_token_grammar(
            "NUMBER = /[0-9]+/\nPLUS = \"+\"\nMINUS = \"-\"",
        ).unwrap();
        let grammar = parse_parser_grammar(
            "expression = NUMBER { PLUS NUMBER } ;",
        ).unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let warnings: Vec<&String> = issues.iter().filter(|i| i.starts_with("Warning")).collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("MINUS"));
    }

    #[test]
    fn test_multiple_issues_at_once() {
        // Multiple errors and warnings can be reported at once.
        let tokens = parse_token_grammar(
            "NUMBER = /[0-9]+/\nUNUSED_A = \"a\"\nUNUSED_B = \"b\"",
        ).unwrap();
        let grammar = parse_parser_grammar(
            "expression = NUMBER PLUS MINUS ;",
        ).unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let errors: Vec<&String> = issues.iter().filter(|i| i.starts_with("Error")).collect();
        let warnings: Vec<&String> = issues.iter().filter(|i| i.starts_with("Warning")).collect();
        // Missing: PLUS, MINUS
        assert_eq!(errors.len(), 2);
        // Unused: UNUSED_A, UNUSED_B
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_empty_grammars() {
        // Empty grammars produce no issues.
        let tokens = parse_token_grammar("").unwrap();
        let grammar = parse_parser_grammar("").unwrap();
        let issues = cross_validate(&tokens, &grammar);
        assert!(issues.is_empty());
    }

    // -----------------------------------------------------------------------
    // Indentation mode implicit tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_indent_dedent_newline_implicit_in_indent_mode() {
        // In indentation mode, INDENT/DEDENT/NEWLINE are implicitly available.
        let tokens = parse_token_grammar(
            "mode: indentation\nNAME = /[a-z]+/\nCOLON = \":\"",
        ).unwrap();
        let grammar = parse_parser_grammar(
            "file = { NAME COLON NEWLINE INDENT NAME NEWLINE DEDENT } ;",
        ).unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let errors: Vec<&String> = issues.iter().filter(|i| i.starts_with("Error")).collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_indent_missing_without_indent_mode() {
        // Without indentation mode, INDENT is not implicitly available.
        let tokens = parse_token_grammar("NAME = /[a-z]+/").unwrap();
        let grammar = parse_parser_grammar("file = NAME INDENT NAME ;").unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let errors: Vec<&String> = issues.iter().filter(|i| i.starts_with("Error")).collect();
        assert!(errors.iter().any(|e| e.contains("INDENT")));
    }

    // -----------------------------------------------------------------------
    // Alias cross-validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_aliased_tokens_not_reported_unused() {
        // A token with an alias should not be reported as unused if the
        // alias is referenced in the grammar.
        let tokens = parse_token_grammar(
            r#"STRING_DQ = /"[^"]*"/ -> STRING"#,
        ).unwrap();
        let grammar = parse_parser_grammar("expr = STRING ;").unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let warnings: Vec<&String> = issues.iter().filter(|i| i.starts_with("Warning")).collect();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_eof_always_implicit() {
        // EOF is always implicitly available, even without defining it.
        let tokens = parse_token_grammar("NAME = /[a-z]+/").unwrap();
        let grammar = parse_parser_grammar("file = NAME EOF ;").unwrap();
        let issues = cross_validate(&tokens, &grammar);
        let errors: Vec<&String> = issues.iter().filter(|i| i.starts_with("Error")).collect();
        assert!(!errors.iter().any(|e| e.contains("EOF")));
    }
}
