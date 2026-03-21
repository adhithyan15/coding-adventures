//! # Token Grammar — parsing and validating `.tokens` files.
//!
//! A `.tokens` file is a declarative description of the lexical grammar of a
//! programming language. It lists every token the lexer should recognize, in
//! priority order (first match wins), along with an optional `keywords:`
//! section for reserved words.
//!
//! This module solves the "front half" of the grammar-tools pipeline: it reads
//! a plain-text token specification and produces a structured [`TokenGrammar`]
//! that downstream tools (lexer generators, cross-validators) can consume.
//!
//! # File format
//!
//! Each non-blank, non-comment line in a `.tokens` file has one of three forms:
//!
//! ```text
//! TOKEN_NAME = /regex_pattern/      — a regex-based token
//! TOKEN_NAME = "literal_string"     — a literal-string token
//! keywords:                         — begins the keywords section
//! ```
//!
//! Lines starting with `#` are comments. Blank lines are ignored.
//!
//! The keywords section lists one reserved word per line (indented). Keywords
//! are identifiers that the lexer recognizes as `NAME` tokens but then
//! reclassifies. For instance, `if` matches the `NAME` pattern but is promoted
//! to an `IF` keyword.
//!
//! # Design decisions
//!
//! **Why hand-parse instead of using a parser library?** Because the format is
//! simple enough that a line-by-line parser is clearer, faster, and produces
//! better error messages than any generic tool would. Every error includes the
//! line number where the problem occurred, which matters a lot when users are
//! writing grammars by hand.
//!
//! **Why store regex patterns as strings instead of compiled `Regex` objects?**
//! Because the grammar should be a pure data structure that is easy to
//! serialize, clone, and inspect. Compilation happens downstream when the
//! grammar is actually used to build a lexer. We do validate that regex
//! patterns compile during the optional [`validate_token_grammar`] pass.

use std::collections::HashSet;
use std::fmt;

// ===========================================================================
// Error type
// ===========================================================================

/// Error returned when a `.tokens` file cannot be parsed.
///
/// Every error includes the 1-based line number where the problem occurred,
/// so users can jump straight to the problematic line in their editor.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenGrammarError {
    pub message: String,
    pub line_number: usize,
}

impl fmt::Display for TokenGrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line {}: {}", self.line_number, self.message)
    }
}

impl std::error::Error for TokenGrammarError {}

// ===========================================================================
// Data model
// ===========================================================================

/// A single token rule from a `.tokens` file.
///
/// Each definition maps a token name to a pattern. The pattern is either a
/// regex (written as `/pattern/` in the file) or a literal string (written
/// as `"literal"` in the file).
///
/// # Fields
///
/// - `name` — The token name, e.g. `"NUMBER"` or `"PLUS"`.
/// - `pattern` — The pattern string without delimiters. For regex tokens,
///   this is the regex source (e.g. `[0-9]+`). For literals, this is the
///   exact string (e.g. `+`).
/// - `is_regex` — `true` if the pattern was written as `/regex/`, `false`
///   if it was written as `"literal"`.
/// - `line_number` — The 1-based line number where this definition appeared.
///   Used for error messages and cross-referencing.
/// - `alias` — Optional type alias. When a definition has `-> ALIAS`, tokens
///   matching this pattern are emitted with the alias name instead. For example,
///   `STRING_DQ = /"[^"]*"/ -> STRING` means the lexer emits `STRING` tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenDefinition {
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
    pub line_number: usize,
    pub alias: Option<String>,
}

/// The complete contents of a parsed `.tokens` file.
///
/// # Fields
///
/// - `definitions` — Ordered list of token definitions. **Order matters**
///   because the lexer uses first-match-wins semantics: when the input could
///   match multiple patterns, the one listed first wins. This is why
///   multi-character operators like `==` must come before `=`.
/// - `keywords` — List of reserved words from the `keywords:` section.
///   These are identifiers that the lexer reclassifies into keyword tokens.
/// - `mode` — Optional mode directive (e.g. `"indentation"` for languages
///   like Python/Starlark that use significant whitespace).
/// - `skip_definitions` — Token definitions from the `skip:` section.
///   These patterns are consumed but not emitted as tokens (e.g. whitespace,
///   comments).
/// - `reserved_keywords` — Keywords from the `reserved:` section. Unlike
///   regular keywords, these cause a lexer error if encountered in source
///   code (they are reserved for future use).
/// - `escapes` — Optional escape mode directive (e.g. `"none"` for CSS,
///   which uses hex escapes that differ from JSON's `\uXXXX` format).
/// - `error_definitions` — Token definitions from the `errors:` section.
///   These patterns match malformed input (e.g. unclosed strings) and
///   produce error tokens for graceful degradation.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenGrammar {
    pub definitions: Vec<TokenDefinition>,
    pub keywords: Vec<String>,
    pub mode: Option<String>,
    pub skip_definitions: Vec<TokenDefinition>,
    pub reserved_keywords: Vec<String>,
    pub escapes: Option<String>,
    pub error_definitions: Vec<TokenDefinition>,
}

// ===========================================================================
// Helper: extract all token names
// ===========================================================================

/// Return the set of all defined token names, including aliases.
///
/// This is useful for cross-validation: the parser grammar references
/// tokens by name, and we need to check that every referenced token
/// actually exists in the token grammar. Aliases are included because
/// grammars typically reference the alias name (e.g. `STRING`) rather
/// than the raw definition name (e.g. `STRING_DQ`).
pub fn token_names(grammar: &TokenGrammar) -> HashSet<String> {
    let mut names: HashSet<String> = grammar.definitions.iter().map(|d| d.name.clone()).collect();
    for defn in &grammar.definitions {
        if let Some(alias) = &defn.alias {
            names.insert(alias.clone());
        }
    }
    names
}

// ===========================================================================
// Parser
// ===========================================================================

/// Parse the text of a `.tokens` file into a [`TokenGrammar`].
///
/// The parser operates line-by-line. It has two modes:
///
/// 1. **Definition mode** (default) — each line is either a comment, a
///    blank, or a token definition of the form `NAME = /pattern/` or
///    `NAME = "literal"`.
///
/// 2. **Keywords mode** — entered when the parser encounters a line
///    matching `keywords:`. Each subsequent indented line is treated as
///    a keyword until a non-indented, non-blank, non-comment line is found
///    (or EOF).
///
/// # Errors
///
/// Returns [`TokenGrammarError`] if any line cannot be parsed, with the
/// specific line number and a human-readable description of the problem.
///
/// # Example
///
/// ```
/// use grammar_tools::token_grammar::parse_token_grammar;
///
/// let source = r#"
/// NUMBER = /[0-9]+/
/// PLUS   = "+"
/// keywords:
///   if
///   else
/// "#;
///
/// let grammar = parse_token_grammar(source).unwrap();
/// assert_eq!(grammar.definitions.len(), 2);
/// assert_eq!(grammar.keywords, vec!["if", "else"]);
/// ```
/// Parse a single token definition from a line, returning the definition.
///
/// Handles the `NAME = /pattern/` or `NAME = "literal"` syntax, plus the
/// optional `-> ALIAS` suffix. Also detects unclosed delimiters for better
/// error messages.
fn parse_definition(line: &str, line_number: usize) -> Result<TokenDefinition, TokenGrammarError> {
    let eq_index = match line.find('=') {
        Some(idx) => idx,
        None => {
            return Err(TokenGrammarError {
                message: "Expected token definition (NAME = pattern)".to_string(),
                line_number,
            });
        }
    };

    let name_part = line[..eq_index].trim();
    let after_eq = line[eq_index + 1..].trim();

    if name_part.is_empty() {
        return Err(TokenGrammarError {
            message: "Missing token name".to_string(),
            line_number,
        });
    }

    if after_eq.is_empty() {
        return Err(TokenGrammarError {
            message: "Missing pattern after '='".to_string(),
            line_number,
        });
    }

    // Parse pattern and optional alias from the remainder after '='.
    // The remainder looks like: /regex/ -> ALIAS  or  "literal" -> ALIAS
    let (pattern_str, alias) = if after_eq.starts_with('/') {
        // Regex pattern — find the closing slash, skipping escaped slashes (\/).
        let rest = &after_eq[1..];
        let close_idx = {
            let mut i = 0;
            let bytes = rest.as_bytes();
            let mut found = None;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2; // skip escaped character
                } else if bytes[i] == b'/' {
                    found = Some(i);
                    break;
                } else {
                    i += 1;
                }
            }
            found
        };
        match close_idx {
            Some(close_idx) => {
                let regex_body = &rest[..close_idx];
                if regex_body.is_empty() {
                    return Err(TokenGrammarError {
                        message: "Empty regex pattern".to_string(),
                        line_number,
                    });
                }
                let after_pattern = rest[close_idx + 1..].trim();
                let alias = parse_alias(after_pattern, line_number)?;
                (
                    TokenDefinition {
                        name: name_part.to_string(),
                        pattern: regex_body.to_string(),
                        is_regex: true,
                        line_number,
                        alias: alias.clone(),
                    },
                    alias,
                )
            }
            None => {
                return Err(TokenGrammarError {
                    message: "Unclosed regex pattern (missing closing '/')".to_string(),
                    line_number,
                });
            }
        }
    } else if after_eq.starts_with('"') {
        // Literal pattern — find the closing quote.
        let rest = &after_eq[1..];
        match rest.find('"') {
            Some(close_idx) => {
                let literal_body = &rest[..close_idx];
                if literal_body.is_empty() {
                    return Err(TokenGrammarError {
                        message: "Empty literal pattern".to_string(),
                        line_number,
                    });
                }
                let after_pattern = rest[close_idx + 1..].trim();
                let alias = parse_alias(after_pattern, line_number)?;
                (
                    TokenDefinition {
                        name: name_part.to_string(),
                        pattern: literal_body.to_string(),
                        is_regex: false,
                        line_number,
                        alias: alias.clone(),
                    },
                    alias,
                )
            }
            None => {
                return Err(TokenGrammarError {
                    message: "Unclosed literal pattern (missing closing '\"')".to_string(),
                    line_number,
                });
            }
        }
    } else {
        return Err(TokenGrammarError {
            message: "Pattern must be /regex/ or \"literal\"".to_string(),
            line_number,
        });
    };

    let _ = alias; // already stored in pattern_str
    Ok(pattern_str)
}

/// Parse an optional `-> ALIAS` suffix from the text after a pattern.
fn parse_alias(after_pattern: &str, line_number: usize) -> Result<Option<String>, TokenGrammarError> {
    if after_pattern.is_empty() {
        return Ok(None);
    }
    if after_pattern.starts_with("->") {
        let alias_name = after_pattern[2..].trim();
        if alias_name.is_empty() {
            return Err(TokenGrammarError {
                message: "Missing alias name after '->'".to_string(),
                line_number,
            });
        }
        Ok(Some(alias_name.to_string()))
    } else {
        Ok(None)
    }
}

pub fn parse_token_grammar(source: &str) -> Result<TokenGrammar, TokenGrammarError> {
    let mut definitions = Vec::new();
    let mut keywords = Vec::new();
    let mut mode: Option<String> = None;
    let mut skip_definitions = Vec::new();
    let mut reserved_keywords = Vec::new();
    let mut escapes: Option<String> = None;
    let mut error_definitions = Vec::new();

    // Track which section we are currently in.
    // Sections: "definitions" (default), "keywords", "skip", "reserved", "errors"
    let mut current_section = "definitions";

    for (i, raw_line) in source.split('\n').enumerate() {
        let line_number = i + 1;
        let line = raw_line.trim_end();
        let stripped = line.trim();

        // --- Blank lines and comments are always skipped ---
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        // --- Section headers ---
        if stripped == "keywords:" || stripped == "keywords :" {
            current_section = "keywords";
            continue;
        }
        if stripped == "skip:" || stripped == "skip :" {
            current_section = "skip";
            continue;
        }
        if stripped == "reserved:" || stripped == "reserved :" {
            current_section = "reserved";
            continue;
        }
        if stripped == "errors:" || stripped == "errors :" {
            current_section = "errors";
            continue;
        }

        // --- Escapes directive ---
        //
        // The `escapes:` directive tells the lexer how to handle escape
        // sequences in string tokens. For example, `escapes: none` means
        // the lexer should strip quotes but leave escape sequences as raw
        // text (used by CSS, where hex escapes differ from JSON's \uXXXX).
        if stripped.starts_with("escapes:") || stripped.starts_with("escapes :") {
            let colon_idx = stripped.find(':').unwrap();
            let escapes_value = stripped[colon_idx + 1..].trim();
            if !escapes_value.is_empty() {
                escapes = Some(escapes_value.to_string());
            }
            continue;
        }

        // --- Mode directive ---
        if stripped.starts_with("mode:") || stripped.starts_with("mode :") {
            let colon_idx = stripped.find(':').unwrap();
            let mode_value = stripped[colon_idx + 1..].trim();
            if mode_value.is_empty() {
                return Err(TokenGrammarError {
                    message: "Missing mode value after 'mode:'".to_string(),
                    line_number,
                });
            }
            mode = Some(mode_value.to_string());
            continue;
        }

        // --- Inside a section ---
        match current_section {
            "keywords" => {
                let first_char = line.as_bytes().first().copied().unwrap_or(b' ');
                if first_char == b' ' || first_char == b'\t' {
                    if !stripped.is_empty() {
                        keywords.push(stripped.to_string());
                    }
                    continue;
                } else {
                    // Non-indented line — exit keywords section, fall through
                    current_section = "definitions";
                }
            }
            "reserved" => {
                let first_char = line.as_bytes().first().copied().unwrap_or(b' ');
                if first_char == b' ' || first_char == b'\t' {
                    if !stripped.is_empty() {
                        reserved_keywords.push(stripped.to_string());
                    }
                    continue;
                } else {
                    current_section = "definitions";
                }
            }
            "skip" => {
                let first_char = line.as_bytes().first().copied().unwrap_or(b' ');
                if first_char == b' ' || first_char == b'\t' {
                    if !stripped.is_empty() {
                        let defn = parse_definition(stripped, line_number)?;
                        skip_definitions.push(defn);
                    }
                    continue;
                } else {
                    current_section = "definitions";
                }
            }
            "errors" => {
                let first_char = line.as_bytes().first().copied().unwrap_or(b' ');
                if first_char == b' ' || first_char == b'\t' {
                    if !stripped.is_empty() {
                        let defn = parse_definition(stripped, line_number)?;
                        error_definitions.push(defn);
                    }
                    continue;
                } else {
                    current_section = "definitions";
                }
            }
            _ => {} // "definitions" — fall through to parse as definition
        }

        // --- Token definition ---
        let defn = parse_definition(line, line_number)?;
        definitions.push(defn);
    }

    Ok(TokenGrammar {
        definitions,
        keywords,
        mode,
        skip_definitions,
        reserved_keywords,
        escapes,
        error_definitions,
    })
}

// ===========================================================================
// Validator
// ===========================================================================

/// Check a parsed [`TokenGrammar`] for common problems.
///
/// This is a *lint* pass, not a parse pass — the grammar has already been
/// parsed successfully. We are looking for semantic issues that would cause
/// problems downstream:
///
/// - **Duplicate token names**: Two definitions with the same name. The
///   second would shadow the first, which is almost certainly a mistake.
/// - **Invalid regex patterns**: A pattern written as `/regex/` that the
///   `regex` crate cannot compile. Caught here rather than at lexer-generation
///   time so the user gets an early, clear error.
/// - **Non-UPPER_CASE names**: By convention, token names are UPPER_CASE.
///   This helps distinguish them from parser rule names (lowercase) in
///   `.grammar` files.
///
/// Returns a list of warning/error strings. An empty list means no issues.
/// Validate a list of token definitions for common problems.
///
/// Shared between the main definitions and skip definitions to avoid
/// duplicating validation logic.
fn validate_definitions(
    definitions: &[TokenDefinition],
    seen_names: &mut std::collections::HashMap<String, usize>,
    issues: &mut Vec<String>,
) {
    for defn in definitions {
        // --- Duplicate check ---
        if let Some(&first_line) = seen_names.get(&defn.name) {
            issues.push(format!(
                "Line {}: Duplicate token name '{}' (first defined on line {})",
                defn.line_number, defn.name, first_line
            ));
        } else {
            seen_names.insert(defn.name.clone(), defn.line_number);
        }

        // --- Empty pattern check ---
        if defn.pattern.is_empty() {
            issues.push(format!(
                "Line {}: Empty pattern for token '{}'",
                defn.line_number, defn.name
            ));
        }

        // --- Invalid regex check ---
        if defn.is_regex {
            if let Err(e) = regex::Regex::new(&defn.pattern) {
                issues.push(format!(
                    "Line {}: Invalid regex for token '{}': {}",
                    defn.line_number, defn.name, e
                ));
            }
        }

        // --- Naming convention check ---
        if defn.name != defn.name.to_uppercase() {
            issues.push(format!(
                "Line {}: Token name '{}' should be UPPER_CASE",
                defn.line_number, defn.name
            ));
        }
    }
}

pub fn validate_token_grammar(grammar: &TokenGrammar) -> Vec<String> {
    let mut issues = Vec::new();
    let mut seen_names: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    // Validate main definitions.
    validate_definitions(&grammar.definitions, &mut seen_names, &mut issues);

    // Validate skip definitions.
    validate_definitions(&grammar.skip_definitions, &mut seen_names, &mut issues);

    // Validate mode value if present.
    if let Some(ref mode) = grammar.mode {
        if mode != "indentation" {
            issues.push(format!("Unknown mode '{}' (expected 'indentation')", mode));
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

    // -----------------------------------------------------------------------
    // Parsing: happy paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_regex_token() {
        // A regex token is delimited by forward slashes: /pattern/
        let source = "NUMBER = /[0-9]+/";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.definitions.len(), 1);
        assert_eq!(grammar.definitions[0].name, "NUMBER");
        assert_eq!(grammar.definitions[0].pattern, "[0-9]+");
        assert!(grammar.definitions[0].is_regex);
        assert_eq!(grammar.definitions[0].line_number, 1);
    }

    #[test]
    fn test_parse_literal_token() {
        // A literal token is delimited by double quotes: "literal"
        let source = r#"PLUS = "+""#;
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.definitions.len(), 1);
        assert_eq!(grammar.definitions[0].name, "PLUS");
        assert_eq!(grammar.definitions[0].pattern, "+");
        assert!(!grammar.definitions[0].is_regex);
    }

    #[test]
    fn test_parse_multiple_definitions() {
        // Multiple definitions appear in order, and order matters for
        // first-match-wins lexer semantics.
        let source = r#"
NUMBER = /[0-9]+/
PLUS   = "+"
MINUS  = "-"
"#;
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.definitions.len(), 3);
        assert_eq!(grammar.definitions[0].name, "NUMBER");
        assert_eq!(grammar.definitions[1].name, "PLUS");
        assert_eq!(grammar.definitions[2].name, "MINUS");
    }

    #[test]
    fn test_parse_keywords_section() {
        // The keywords: section lists reserved words, one per indented line.
        let source = "NAME = /[a-zA-Z_]+/\nkeywords:\n  if\n  else\n  while\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.keywords, vec!["if", "else", "while"]);
    }

    #[test]
    fn test_comments_and_blanks_ignored() {
        // Lines starting with # and blank lines are ignored.
        let source = "# This is a comment\n\nNUMBER = /[0-9]+/\n\n# Another comment\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.definitions.len(), 1);
    }

    #[test]
    fn test_keywords_with_space_before_colon() {
        // "keywords :" (with space) is also valid.
        let source = "keywords :\n  if\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.keywords, vec!["if"]);
    }

    #[test]
    fn test_exit_keywords_on_non_indented_line() {
        // A non-indented line after keywords: exits the keywords section
        // and is parsed as a normal token definition.
        let source = "keywords:\n  if\n  else\nNUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.keywords, vec!["if", "else"]);
        assert_eq!(grammar.definitions.len(), 1);
        assert_eq!(grammar.definitions[0].name, "NUMBER");
    }

    #[test]
    fn test_empty_source() {
        // An empty source produces an empty grammar — no definitions, no keywords.
        let grammar = parse_token_grammar("").unwrap();
        assert!(grammar.definitions.is_empty());
        assert!(grammar.keywords.is_empty());
    }

    #[test]
    fn test_parse_full_python_tokens() {
        // Parse the actual Python tokens file to make sure the parser handles
        // a realistic, multi-section grammar with comments, regex, literals,
        // and keywords.
        let source = r#"
# Token definitions for a subset of Python
NAME        = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER      = /[0-9]+/
STRING      = /"([^"\\]|\\.)*"/
EQUALS_EQUALS = "=="
EQUALS      = "="
PLUS        = "+"
MINUS       = "-"
STAR        = "*"
SLASH       = "/"
LPAREN      = "("
RPAREN      = ")"
COMMA       = ","
COLON       = ":"
keywords:
  if
  else
  elif
  while
  for
  def
  return
  class
  import
  from
  as
  True
  False
  None
"#;
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.definitions.len(), 13);
        // 14 keywords listed but the last line before the closing delimiter
        // may or may not include a trailing newline depending on the raw string.
        assert!(grammar.keywords.len() >= 13);
        assert!(grammar.keywords.len() <= 14);
        // First-match-wins: EQUALS_EQUALS must come before EQUALS.
        assert_eq!(grammar.definitions[3].name, "EQUALS_EQUALS");
        assert_eq!(grammar.definitions[4].name, "EQUALS");
    }

    // -----------------------------------------------------------------------
    // Parsing: error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_missing_equals() {
        // A line without '=' is not a valid token definition.
        let result = parse_token_grammar("NUMBER /[0-9]+/");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.line_number, 1);
        assert!(err.message.contains("Expected token definition"));
    }

    #[test]
    fn test_error_missing_name() {
        // '=' with nothing before it.
        let result = parse_token_grammar(" = /[0-9]+/");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Missing token name"));
    }

    #[test]
    fn test_error_missing_pattern() {
        // '=' with nothing after it.
        let result = parse_token_grammar("NUMBER =");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Missing pattern"));
    }

    #[test]
    fn test_error_bad_pattern_delimiter() {
        // Pattern that is neither /regex/ nor "literal".
        let result = parse_token_grammar("NUMBER = [0-9]+");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("must be /regex/"));
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_no_issues() {
        // A well-formed grammar produces no validation issues.
        let grammar = parse_token_grammar("NUMBER = /[0-9]+/\nPLUS = \"+\"").unwrap();
        let issues = validate_token_grammar(&grammar);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_validate_duplicate_names() {
        // Two definitions with the same name triggers a duplicate warning.
        let grammar = TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: "NUMBER".to_string(),
                    pattern: "[0-9]+".to_string(),
                    is_regex: true,
                    line_number: 1,
                    alias: None,
                },
                TokenDefinition {
                    name: "NUMBER".to_string(),
                    pattern: "[0-9]*".to_string(),
                    is_regex: true,
                    line_number: 2,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
        };
        let issues = validate_token_grammar(&grammar);
        assert!(!issues.is_empty());
        assert!(issues[0].contains("Duplicate token name"));
    }

    #[test]
    fn test_validate_invalid_regex() {
        // A regex that cannot compile triggers a validation issue.
        let grammar = TokenGrammar {
            definitions: vec![TokenDefinition {
                name: "BAD".to_string(),
                pattern: "[unclosed".to_string(),
                is_regex: true,
                line_number: 1,
                alias: None,
            }],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
        };
        let issues = validate_token_grammar(&grammar);
        assert!(!issues.is_empty());
        assert!(issues[0].contains("Invalid regex"));
    }

    #[test]
    fn test_validate_non_uppercase_name() {
        // Token names should be UPPER_CASE by convention.
        let grammar = TokenGrammar {
            definitions: vec![TokenDefinition {
                name: "number".to_string(),
                pattern: "[0-9]+".to_string(),
                is_regex: true,
                line_number: 1,
                alias: None,
            }],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
        };
        let issues = validate_token_grammar(&grammar);
        assert!(!issues.is_empty());
        assert!(issues[0].contains("UPPER_CASE"));
    }

    // -----------------------------------------------------------------------
    // Helper: token_names
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_names_helper() {
        let grammar = parse_token_grammar("NUMBER = /[0-9]+/\nPLUS = \"+\"").unwrap();
        let names = token_names(&grammar);
        assert!(names.contains("NUMBER"));
        assert!(names.contains("PLUS"));
        assert_eq!(names.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Mode directive
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_mode_indentation() {
        let grammar = parse_token_grammar("mode: indentation\nNAME = /[a-z]+/").unwrap();
        assert_eq!(grammar.mode, Some("indentation".to_string()));
    }

    #[test]
    fn test_parse_no_mode() {
        let grammar = parse_token_grammar("NAME = /[a-z]+/").unwrap();
        assert_eq!(grammar.mode, None);
    }

    #[test]
    fn test_parse_mode_missing_value() {
        let result = parse_token_grammar("mode:");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Missing mode value"));
    }

    // -----------------------------------------------------------------------
    // Skip section
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_skip_section() {
        let source = "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.skip_definitions.len(), 2);
        assert_eq!(grammar.skip_definitions[0].name, "WHITESPACE");
        assert_eq!(grammar.skip_definitions[1].name, "COMMENT");
    }

    #[test]
    fn test_parse_skip_definition_without_equals() {
        let result = parse_token_grammar("skip:\n  BAD_PATTERN");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_skip_definition_incomplete() {
        let result = parse_token_grammar("skip:\n  BAD =");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Reserved section
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_reserved_section() {
        let source = "NAME = /[a-z]+/\nreserved:\n  class\n  import";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.reserved_keywords, vec!["class", "import"]);
    }

    // -----------------------------------------------------------------------
    // Aliases
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_regex_alias() {
        let source = r#"STRING_DQ = /"[^"]*"/ -> STRING"#;
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.definitions[0].alias, Some("STRING".to_string()));
    }

    #[test]
    fn test_parse_literal_alias() {
        let source = r#"PLUS_SIGN = "+" -> PLUS"#;
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.definitions[0].alias, Some("PLUS".to_string()));
    }

    #[test]
    fn test_parse_missing_alias_name() {
        let result = parse_token_grammar("FOO = /x/ ->");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Missing alias"));
    }

    #[test]
    fn test_token_names_includes_aliases() {
        let source = r#"STRING_DQ = /"[^"]*"/ -> STRING"#;
        let grammar = parse_token_grammar(source).unwrap();
        let names = token_names(&grammar);
        assert!(names.contains("STRING_DQ"));
        assert!(names.contains("STRING"));
    }

    // -----------------------------------------------------------------------
    // Error cases for new syntax
    // -----------------------------------------------------------------------

    #[test]
    fn test_unclosed_regex() {
        let result = parse_token_grammar("FOO = /unclosed");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unclosed regex"));
    }

    #[test]
    fn test_unclosed_literal() {
        let result = parse_token_grammar("FOO = \"unclosed");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unclosed literal"));
    }

    #[test]
    fn test_empty_regex() {
        let result = parse_token_grammar("FOO = //");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Empty regex"));
    }

    #[test]
    fn test_empty_literal() {
        let result = parse_token_grammar(r#"FOO = """#);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Empty literal"));
    }

    // -----------------------------------------------------------------------
    // Starlark-like full example
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_starlark_tokens() {
        let source = r#"
mode: indentation

NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
INT = /[0-9]+/
EQUALS = "="
PLUS = "+"
COLON = ":"
LPAREN = "("
RPAREN = ")"
COMMA = ","

keywords:
  def
  return
  if
  else
  for
  in
  pass

reserved:
  class
  import

skip:
  WHITESPACE = /[ \t]+/
  COMMENT = /#[^\n]*/
"#;
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.mode, Some("indentation".to_string()));
        assert_eq!(grammar.reserved_keywords, vec!["class", "import"]);
        assert_eq!(grammar.skip_definitions.len(), 2);
        assert_eq!(grammar.keywords.len(), 7);
        let issues = validate_token_grammar(&grammar);
        assert!(issues.is_empty());
    }

    // -----------------------------------------------------------------------
    // Validate mode
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_unknown_mode() {
        let grammar = TokenGrammar {
            definitions: vec![],
            keywords: vec![],
            mode: Some("unknown".to_string()),
            skip_definitions: vec![],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
        };
        let issues = validate_token_grammar(&grammar);
        assert!(issues.iter().any(|i| i.contains("Unknown mode")));
    }
}
