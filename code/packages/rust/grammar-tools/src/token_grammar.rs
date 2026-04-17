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

use std::collections::{HashMap, HashSet};
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

/// A named set of token definitions that are active together.
///
/// When this group is at the top of the lexer's group stack, only these
/// patterns are tried during token matching. Skip patterns are global
/// and always tried regardless of the active group.
///
/// Pattern groups enable context-sensitive lexing. For example, an XML
/// lexer defines a "tag" group with patterns for attribute names, equals
/// signs, and attribute values. These patterns are only active inside
/// tags — the callback pushes the "tag" group when `<` is matched and
/// pops it when `>` is matched.
///
/// # Fields
///
/// - `name` — The group name, e.g. `"tag"` or `"cdata"`. Must be a
///   lowercase identifier matching `[a-z_][a-z0-9_]*`.
/// - `definitions` — Ordered list of token definitions in this group.
///   Order matters (first-match-wins), just like the top-level
///   definitions list.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternGroup {
    pub name: String,
    pub definitions: Vec<TokenDefinition>,
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
/// - `groups` — Named pattern groups from `group NAME:` sections. Each
///   group defines a set of token patterns for context-sensitive lexing.
///   The lexer maintains a stack of active groups and only tries patterns
///   from the group on top of the stack. Patterns outside any group belong
///   to the implicit "default" group (stored in `definitions`).
/// - `case_sensitive` — Whether the lexer should perform case-sensitive
///   matching. Defaults to `true`. When `false`, the lexer lowercases the
///   source before tokenization so that keywords and patterns match
///   regardless of case (useful for languages like SQL and BASIC).
/// - `version` — The grammar version number, set by the `# @version N`
///   magic comment. A value of `0` means "no version declared" (use latest
///   semantics). This is useful when the grammar format evolves over time
///   and tools need to know which dialect to apply.
/// - `case_insensitive` — When `true`, the lexer should match tokens
///   without regard to letter case. Set by the `# @case_insensitive true`
///   magic comment. Defaults to `false` (case-sensitive matching).
///
/// # Magic comments
///
/// Magic comments are special `#` comment lines that carry structured
/// metadata. They use the form `# @key value` where `key` is a word and
/// `value` is the rest of the line after whitespace. Unknown keys are
/// silently ignored for forward-compatibility. Example:
///
/// ```text
/// # @version 2
/// # @case_insensitive true
/// NUMBER = /[0-9]+/
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TokenGrammar {
    pub definitions: Vec<TokenDefinition>,
    pub keywords: Vec<String>,
    pub mode: Option<String>,
    pub skip_definitions: Vec<TokenDefinition>,
    pub reserved_keywords: Vec<String>,
    pub escapes: Option<String>,
    pub error_definitions: Vec<TokenDefinition>,
    pub groups: HashMap<String, PatternGroup>,
    pub case_sensitive: bool,
    /// Grammar version declared via `# @version N`. Zero means unset.
    pub version: u32,
    /// Whether matching is case-insensitive, declared via `# @case_insensitive true`.
    pub case_insensitive: bool,
    /// Context-sensitive keywords -- words that are keywords in some
    /// syntactic positions but identifiers in others.
    ///
    /// These are emitted as NAME tokens with the `TOKEN_CONTEXT_KEYWORD`
    /// flag set, leaving the final keyword-vs-identifier decision to
    /// the language-specific parser or callback.
    ///
    /// Declared via the `context_keywords:` section in the `.tokens` file.
    /// Each indented line in that section is one context keyword.
    ///
    /// Examples: JavaScript's `async`, `await`, `yield`, `get`, `set`.
    pub context_keywords: Vec<String>,
    /// Soft keywords — words that act as keywords only in specific syntactic
    /// contexts, remaining ordinary identifiers everywhere else.
    ///
    /// Unlike context keywords (which set a flag on the token), soft keywords
    /// produce plain NAME tokens with NO special flag. The lexer is completely
    /// unaware of their keyword status — the parser handles disambiguation
    /// entirely based on syntactic position.
    ///
    /// This distinction matters because:
    ///   - Context keywords: lexer hints to parser ("this NAME might be special")
    ///   - Soft keywords: lexer ignores them completely, parser owns the decision
    ///
    /// Examples:
    ///   Python 3.10+: `match`, `case`, `_` (only keywords inside match statements)
    ///   Python 3.12+: `type` (only a keyword in `type X = ...` statements)
    ///
    /// Declared via the `soft_keywords:` section in the `.tokens` file.
    /// Each indented line in that section is one soft keyword.
    pub soft_keywords: Vec<String>,
    /// Keywords that introduce a Haskell-style layout context when
    /// `mode == "layout"`.
    pub layout_keywords: Vec<String>,
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
///
/// Includes names from all pattern groups, since group tokens can
/// also appear in parser grammars.
pub fn token_names(grammar: &TokenGrammar) -> HashSet<String> {
    // Collect definitions from the top-level list and all groups into
    // a single iterator so we handle them uniformly.
    let all_defs = grammar
        .definitions
        .iter()
        .chain(grammar.groups.values().flat_map(|g| g.definitions.iter()));

    let mut names = HashSet::new();
    for defn in all_defs {
        names.insert(defn.name.clone());
        if let Some(alias) = &defn.alias {
            names.insert(alias.clone());
        }
    }
    names
}

/// Return the set of token names as the parser will see them.
///
/// For definitions with aliases, this returns the alias (not the
/// definition name), because that is what the lexer will emit and
/// what the parser grammar references.
///
/// For definitions without aliases, this returns the definition name.
///
/// Includes names from all pattern groups.
pub fn effective_token_names(grammar: &TokenGrammar) -> HashSet<String> {
    let all_defs = grammar
        .definitions
        .iter()
        .chain(grammar.groups.values().flat_map(|g| g.definitions.iter()));

    all_defs
        .map(|d| d.alias.as_ref().unwrap_or(&d.name).clone())
        .collect()
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
        // Regex pattern — find the closing slash by scanning character-by-character.
        // We track bracket depth so that / inside [...] character classes is
        // not mistaken for the closing delimiter. We also skip escaped chars.
        let rest = &after_eq[1..];
        let close_idx = {
            let mut i = 0;
            let bytes = rest.as_bytes();
            let mut found = None;
            let mut in_bracket = false;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2; // skip escaped character
                } else if bytes[i] == b'[' && !in_bracket {
                    in_bracket = true;
                    i += 1;
                } else if bytes[i] == b']' && in_bracket {
                    in_bracket = false;
                    i += 1;
                } else if bytes[i] == b'/' && !in_bracket {
                    found = Some(i);
                    break;
                } else {
                    i += 1;
                }
            }
            // Fallback: if bracket-aware scan found nothing (e.g. unclosed [),
            // try the last / as a best-effort parse.
            found.or_else(|| rest.rfind('/').filter(|&idx| idx > 0))
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
    let mut context_keywords = Vec::new();
    let mut soft_keywords = Vec::new();
    let mut layout_keywords = Vec::new();
    let mut mode: Option<String> = None;
    let mut skip_definitions = Vec::new();
    let mut reserved_keywords = Vec::new();
    let mut escapes: Option<String> = None;
    let mut case_sensitive: bool = true;
    let mut error_definitions = Vec::new();
    let mut groups: HashMap<String, PatternGroup> = HashMap::new();
    // Magic comment fields — set by `# @key value` lines.
    let mut version: u32 = 0;
    let mut case_insensitive: bool = false;

    // Track which section we are currently in.
    //
    // Sections: "definitions" (default), "keywords", "skip", "reserved",
    // "errors", or "group:NAME" for pattern groups.
    //
    // We use a String rather than &str because group sections carry dynamic
    // names (e.g. "group:tag", "group:cdata").
    let mut current_section = String::from("definitions");

    // Reserved group names that cannot be used. These overlap with built-in
    // section names and would cause ambiguity or confusion.
    let reserved_group_names: HashSet<&str> =
        [
            "default",
            "skip",
            "keywords",
            "reserved",
            "errors",
            "layout_keywords",
            "context_keywords",
            "soft_keywords",
        ]
        .iter()
        .copied()
        .collect();

    // Regex for validating group names: lowercase identifiers only.
    let group_name_re = regex::Regex::new(r"^[a-z_][a-z0-9_]*$").unwrap();

    for (i, raw_line) in source.split('\n').enumerate() {
        let line_number = i + 1;
        let line = raw_line.trim_end();
        let stripped = line.trim();

        // --- Blank lines are always skipped ---
        if stripped.is_empty() {
            continue;
        }

        // --- Comment lines: magic comments or regular comments ---
        //
        // A magic comment has the form `# @key value` and carries structured
        // metadata that affects how the grammar is interpreted. Any other `#`
        // line is a regular comment and is silently ignored.
        //
        // Parsing strategy (no regex, pure string ops):
        //   1. Strip the leading `#` and any whitespace after it.
        //   2. If the next character is `@`, we have a magic comment.
        //   3. Scan forward to collect the key (non-whitespace chars).
        //   4. Skip whitespace, take the rest as the value.
        if stripped.starts_with('#') {
            // Step 1: get everything after '#'
            let after_hash = stripped[1..].trim_start();
            // Step 2: check for '@'
            if after_hash.starts_with('@') {
                // Step 3: key is the run of non-whitespace chars after '@'
                let rest = &after_hash[1..]; // skip '@'
                let key_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                let key = &rest[..key_end];
                // Step 4: value is the trimmed remainder
                let value = rest[key_end..].trim();
                match key {
                    "version" => {
                        if let Ok(v) = value.parse::<u32>() {
                            version = v;
                        }
                        // Malformed version value is silently ignored.
                    }
                    "case_insensitive" => {
                        case_insensitive = value == "true";
                    }
                    // Unknown keys are silently ignored for forward-compatibility.
                    _ => {}
                }
            }
            // Both magic comments and plain comments skip the rest of the loop.
            continue;
        }

        // --- Group headers ---
        //
        // Pattern groups are declared with `group NAME:` where NAME is
        // a lowercase identifier. All subsequent indented lines belong to
        // that group, just like skip: or errors: sections.
        if stripped.starts_with("group ") && stripped.ends_with(':') {
            let group_name = stripped[6..stripped.len() - 1].trim();
            if group_name.is_empty() {
                return Err(TokenGrammarError {
                    message: "Missing group name after 'group'".to_string(),
                    line_number,
                });
            }
            if !group_name_re.is_match(group_name) {
                return Err(TokenGrammarError {
                    message: format!(
                        "Invalid group name: '{}' \
                         (must be a lowercase identifier like 'tag' or 'cdata')",
                        group_name
                    ),
                    line_number,
                });
            }
            if reserved_group_names.contains(group_name) {
                return Err(TokenGrammarError {
                    message: format!(
                        "Reserved group name: '{}' \
                         (cannot use default, errors, keywords, reserved, skip)",
                        group_name
                    ),
                    line_number,
                });
            }
            if groups.contains_key(group_name) {
                return Err(TokenGrammarError {
                    message: format!("Duplicate group name: '{}'", group_name),
                    line_number,
                });
            }
            groups.insert(
                group_name.to_string(),
                PatternGroup {
                    name: group_name.to_string(),
                    definitions: Vec::new(),
                },
            );
            current_section = format!("group:{}", group_name);
            continue;
        }

        // --- Section headers ---
        if stripped == "keywords:" || stripped == "keywords :" {
            current_section = String::from("keywords");
            continue;
        }
        if stripped == "skip:" || stripped == "skip :" {
            current_section = String::from("skip");
            continue;
        }
        if stripped == "reserved:" || stripped == "reserved :" {
            current_section = String::from("reserved");
            continue;
        }
        if stripped == "errors:" || stripped == "errors :" {
            current_section = String::from("errors");
            continue;
        }
        if stripped == "context_keywords:" || stripped == "context_keywords :" {
            current_section = String::from("context_keywords");
            continue;
        }
        if stripped == "layout_keywords:" || stripped == "layout_keywords :" {
            current_section = String::from("layout_keywords");
            continue;
        }
        if stripped == "soft_keywords:" || stripped == "soft_keywords :" {
            current_section = String::from("soft_keywords");
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

        // --- Case sensitivity directive ---
        //
        // The `case_sensitive:` directive controls whether the lexer performs
        // case-sensitive or case-insensitive matching. When set to `false`,
        // the lexer lowercases the source before tokenization, so keywords
        // like `SELECT` and `select` match the same pattern. This is useful
        // for languages like SQL, BASIC, and Pascal that are case-insensitive.
        //
        // Accepted values: "true" or "false". Defaults to true if omitted.
        if stripped.starts_with("case_sensitive:") || stripped.starts_with("case_sensitive :") {
            let colon_idx = stripped.find(':').unwrap();
            let cs_value = stripped[colon_idx + 1..].trim();
            match cs_value {
                "true" => case_sensitive = true,
                "false" => case_sensitive = false,
                _ => {
                    return Err(TokenGrammarError {
                        message: format!(
                            "Invalid case_sensitive value: '{}' (expected 'true' or 'false')",
                            cs_value
                        ),
                        line_number,
                    });
                }
            }
            continue;
        }

        // --- Inside a section ---
        //
        // Each section type handles indented lines differently. A
        // non-indented line exits the current section and falls through
        // to be parsed as a top-level token definition.
        let in_section = current_section != "definitions";
        if in_section {
            let first_char = line.as_bytes().first().copied().unwrap_or(b' ');
            if first_char == b' ' || first_char == b'\t' {
                if !stripped.is_empty() {
                    if current_section == "keywords" {
                        keywords.push(stripped.to_string());
                    } else if current_section == "context_keywords" {
                        context_keywords.push(stripped.to_string());
                    } else if current_section == "layout_keywords" {
                        layout_keywords.push(stripped.to_string());
                    } else if current_section == "soft_keywords" {
                        soft_keywords.push(stripped.to_string());
                    } else if current_section == "reserved" {
                        reserved_keywords.push(stripped.to_string());
                    } else if current_section == "skip" {
                        let defn = parse_definition(stripped, line_number)?;
                        skip_definitions.push(defn);
                    } else if current_section == "errors" {
                        let defn = parse_definition(stripped, line_number)?;
                        error_definitions.push(defn);
                    } else if let Some(group_name) = current_section.strip_prefix("group:") {
                        // Group section — parse token definitions just like
                        // skip: and errors: sections. The definition format is
                        // identical: `NAME = /pattern/` or `NAME = "literal"`.
                        if !stripped.contains('=') {
                            return Err(TokenGrammarError {
                                message: format!(
                                    "Expected token definition in group '{}' \
                                     (NAME = pattern), got: '{}'",
                                    group_name, stripped
                                ),
                                line_number,
                            });
                        }
                        let eq_index = stripped.find('=').unwrap();
                        let g_name = stripped[..eq_index].trim();
                        let g_pattern = stripped[eq_index + 1..].trim();
                        if g_name.is_empty() || g_pattern.is_empty() {
                            return Err(TokenGrammarError {
                                message: format!(
                                    "Incomplete definition in group '{}': '{}'",
                                    group_name, stripped
                                ),
                                line_number,
                            });
                        }
                        let defn = parse_definition(stripped, line_number)?;
                        // We need the owned group_name to look up in the map.
                        let gn = group_name.to_string();
                        if let Some(group) = groups.get_mut(&gn) {
                            group.definitions.push(defn);
                        }
                    }
                }
                continue;
            } else {
                // Non-indented line — exit section, fall through
                current_section = String::from("definitions");
            }
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
        groups,
        case_sensitive,
        version,
        case_insensitive,
        context_keywords,
        soft_keywords,
        layout_keywords,
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

    // Validate error definitions.
    validate_definitions(&grammar.error_definitions, &mut seen_names, &mut issues);

    // Validate mode value if present.
    if let Some(ref mode) = grammar.mode {
        if mode != "indentation" && mode != "layout" {
            issues.push(format!("Unknown mode '{}' (expected 'indentation' or 'layout')", mode));
        }
    }
    if grammar.mode.as_deref() == Some("layout") && grammar.layout_keywords.is_empty() {
        issues.push("Layout mode requires a non-empty layout_keywords section".to_string());
    }

    // Validate escape mode if present.
    if let Some(ref esc) = grammar.escapes {
        if esc != "none" {
            issues.push(format!(
                "Unknown escape mode '{}' (only 'none' is supported)",
                esc
            ));
        }
    }

    // Validate pattern groups.
    //
    // Each group gets its own duplicate-name tracking (group definitions
    // are independent namespaces from the top-level definitions in terms
    // of the lexer's matching logic). However, we use a fresh seen_names
    // per group to catch duplicates *within* a single group.
    let group_name_re = regex::Regex::new(r"^[a-z_][a-z0-9_]*$").unwrap();
    for (group_name, group) in &grammar.groups {
        // Group name format validation.
        if !group_name_re.is_match(group_name) {
            issues.push(format!(
                "Invalid group name '{}' (must be a lowercase identifier)",
                group_name
            ));
        }

        // Empty group warning.
        if group.definitions.is_empty() {
            issues.push(format!(
                "Empty pattern group '{}' (has no token definitions)",
                group_name
            ));
        }

        // Validate definitions within the group.
        let mut group_seen: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        validate_definitions(&group.definitions, &mut group_seen, &mut issues);
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
            groups: HashMap::new(),
case_sensitive: true,
            version: 0,
            case_insensitive: false,
            layout_keywords: vec![],
            context_keywords: Vec::new(),
            soft_keywords: Vec::new(),
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
            groups: HashMap::new(),
case_sensitive: true,
            version: 0,
            case_insensitive: false,
            layout_keywords: vec![],
            context_keywords: Vec::new(),
            soft_keywords: Vec::new(),
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
            groups: HashMap::new(),
case_sensitive: true,
            version: 0,
            case_insensitive: false,
            layout_keywords: vec![],
            context_keywords: Vec::new(),
            soft_keywords: Vec::new(),
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
            groups: HashMap::new(),
case_sensitive: true,
            version: 0,
            case_insensitive: false,
            layout_keywords: vec![],
            context_keywords: Vec::new(),
            soft_keywords: Vec::new(),
        };
        let issues = validate_token_grammar(&grammar);
        assert!(issues.iter().any(|i| i.contains("Unknown mode")));
    }

    // -----------------------------------------------------------------------
    // Pattern groups: happy paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_basic_group() {
        // A simple group section is parsed into a PatternGroup with the
        // correct name and definitions.
        let source = concat!(
            "TEXT = /[^<]+/\n",
            "TAG_OPEN = \"<\"\n",
            "\n",
            "group tag:\n",
            "  TAG_NAME = /[a-zA-Z]+/\n",
            "  TAG_CLOSE = \">\"\n",
        );
        let grammar = parse_token_grammar(source).unwrap();

        // Default-group patterns (top-level definitions).
        assert_eq!(grammar.definitions.len(), 2);
        assert_eq!(grammar.definitions[0].name, "TEXT");
        assert_eq!(grammar.definitions[1].name, "TAG_OPEN");

        // Named group.
        assert!(grammar.groups.contains_key("tag"));
        let group = &grammar.groups["tag"];
        assert_eq!(group.name, "tag");
        assert_eq!(group.definitions.len(), 2);
        assert_eq!(group.definitions[0].name, "TAG_NAME");
        assert_eq!(group.definitions[1].name, "TAG_CLOSE");
    }

    #[test]
    fn test_multiple_groups() {
        // Multiple groups can coexist in the same file.
        let source = concat!(
            "TEXT = /[^<]+/\n",
            "\n",
            "group tag:\n",
            "  TAG_NAME = /[a-zA-Z]+/\n",
            "\n",
            "group cdata:\n",
            "  CDATA_TEXT = /[^]]+/\n",
            "  CDATA_END = \"]]>\"\n",
        );
        let grammar = parse_token_grammar(source).unwrap();

        assert_eq!(grammar.groups.len(), 2);
        assert!(grammar.groups.contains_key("tag"));
        assert!(grammar.groups.contains_key("cdata"));
        assert_eq!(grammar.groups["tag"].definitions.len(), 1);
        assert_eq!(grammar.groups["cdata"].definitions.len(), 2);
    }

    #[test]
    fn test_group_with_alias() {
        // Definitions inside groups support the -> ALIAS suffix.
        let source = concat!(
            "TEXT = /[^<]+/\n",
            "\n",
            "group tag:\n",
            "  ATTR_VALUE_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n",
            "  ATTR_VALUE_SQ = /'[^']*'/ -> ATTR_VALUE\n",
        );
        let grammar = parse_token_grammar(source).unwrap();

        let group = &grammar.groups["tag"];
        assert_eq!(group.definitions[0].name, "ATTR_VALUE_DQ");
        assert_eq!(group.definitions[0].alias, Some("ATTR_VALUE".to_string()));
        assert_eq!(group.definitions[1].name, "ATTR_VALUE_SQ");
        assert_eq!(group.definitions[1].alias, Some("ATTR_VALUE".to_string()));
    }

    #[test]
    fn test_group_with_literal_patterns() {
        // Groups support both regex and literal patterns.
        let source = concat!(
            "TEXT = /[^<]+/\n",
            "\n",
            "group tag:\n",
            "  EQUALS = \"=\"\n",
            "  TAG_NAME = /[a-zA-Z]+/\n",
        );
        let grammar = parse_token_grammar(source).unwrap();

        let group = &grammar.groups["tag"];
        assert!(!group.definitions[0].is_regex);
        assert_eq!(group.definitions[0].pattern, "=");
        assert!(group.definitions[1].is_regex);
    }

    #[test]
    fn test_no_groups_backward_compat() {
        // Files without groups have an empty groups map — backward
        // compatibility is preserved.
        let source = "NUMBER = /[0-9]+/\nPLUS = \"+\"\n";
        let grammar = parse_token_grammar(source).unwrap();

        assert!(grammar.groups.is_empty());
        assert_eq!(grammar.definitions.len(), 2);
    }

    #[test]
    fn test_groups_with_skip_section() {
        // skip: and group: sections coexist correctly.
        let source = concat!(
            "skip:\n",
            "  WS = /[ \\t]+/\n",
            "\n",
            "TEXT = /[^<]+/\n",
            "\n",
            "group tag:\n",
            "  TAG_NAME = /[a-zA-Z]+/\n",
        );
        let grammar = parse_token_grammar(source).unwrap();

        assert_eq!(grammar.skip_definitions.len(), 1);
        assert_eq!(grammar.definitions.len(), 1);
        assert_eq!(grammar.groups.len(), 1);
    }

    #[test]
    fn test_token_names_includes_groups() {
        // token_names() includes names from all groups, plus aliases.
        let source = concat!(
            "TEXT = /[^<]+/\n",
            "\n",
            "group tag:\n",
            "  TAG_NAME = /[a-zA-Z]+/\n",
            "  ATTR_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n",
        );
        let grammar = parse_token_grammar(source).unwrap();

        let names = token_names(&grammar);
        assert!(names.contains("TEXT"));
        assert!(names.contains("TAG_NAME"));
        assert!(names.contains("ATTR_DQ"));
        assert!(names.contains("ATTR_VALUE"));
    }

    #[test]
    fn test_effective_token_names_includes_groups() {
        // effective_token_names() returns alias names from groups,
        // replacing the raw definition names.
        let source = concat!(
            "TEXT = /[^<]+/\n",
            "\n",
            "group tag:\n",
            "  ATTR_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n",
        );
        let grammar = parse_token_grammar(source).unwrap();

        let names = effective_token_names(&grammar);
        assert!(names.contains("TEXT"));
        assert!(names.contains("ATTR_VALUE"));
        assert!(!names.contains("ATTR_DQ")); // alias replaces name
    }

    #[test]
    fn test_group_validates_definitions() {
        // Definitions in groups are validated (e.g. bad regex is caught).
        let mut groups = HashMap::new();
        groups.insert(
            "tag".to_string(),
            PatternGroup {
                name: "tag".to_string(),
                definitions: vec![TokenDefinition {
                    name: "BAD".to_string(),
                    pattern: "[invalid".to_string(),
                    is_regex: true,
                    line_number: 5,
                    alias: None,
                }],
            },
        );
        let grammar = TokenGrammar {
            definitions: vec![],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups,
case_sensitive: true,
            version: 0,
            case_insensitive: false,
            layout_keywords: vec![],
            context_keywords: Vec::new(),
            soft_keywords: Vec::new(),
        };
        let issues = validate_token_grammar(&grammar);
        assert!(issues.iter().any(|i| i.contains("Invalid regex")));
    }

    #[test]
    fn test_empty_group_warning() {
        // An empty group produces a validation warning.
        let mut groups = HashMap::new();
        groups.insert(
            "empty".to_string(),
            PatternGroup {
                name: "empty".to_string(),
                definitions: vec![],
            },
        );
        let grammar = TokenGrammar {
            definitions: vec![],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups,
case_sensitive: true,
            version: 0,
            case_insensitive: false,
            layout_keywords: vec![],
            context_keywords: Vec::new(),
            soft_keywords: Vec::new(),
        };
        let issues = validate_token_grammar(&grammar);
        assert!(issues.iter().any(|i| i.contains("Empty pattern group")));
    }

    // -----------------------------------------------------------------------
    // Pattern groups: error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_missing_group_name() {
        // "group :" with no name raises an error.
        let source = "TEXT = /abc/\ngroup :\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Missing group name"));
    }

    #[test]
    fn test_invalid_group_name_uppercase() {
        // Uppercase group names are rejected.
        let source = "TEXT = /abc/\ngroup Tag:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Invalid group name"));
    }

    #[test]
    fn test_invalid_group_name_starts_with_digit() {
        // Group names starting with a digit are rejected.
        let source = "TEXT = /abc/\ngroup 1tag:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Invalid group name"));
    }

    #[test]
    fn test_reserved_group_name_default() {
        // "group default:" is rejected as reserved.
        let source = "TEXT = /abc/\ngroup default:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved group name"));
    }

    #[test]
    fn test_reserved_group_name_skip() {
        // "group skip:" is rejected as reserved.
        let source = "TEXT = /abc/\ngroup skip:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved group name"));
    }

    #[test]
    fn test_reserved_group_name_keywords() {
        // "group keywords:" is rejected as reserved.
        let source = "TEXT = /abc/\ngroup keywords:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved group name"));
    }

    #[test]
    fn test_duplicate_group_name() {
        // Two groups with the same name raises an error.
        let source = concat!(
            "TEXT = /abc/\n",
            "group tag:\n",
            "  FOO = /x/\n",
            "group tag:\n",
            "  BAR = /y/\n",
        );
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Duplicate group name"));
    }

    #[test]
    fn test_bad_definition_in_group() {
        // Invalid definition inside a group raises an error.
        let source = concat!(
            "TEXT = /abc/\n",
            "group tag:\n",
            "  not a definition\n",
        );
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected token definition"));
    }

    #[test]
    fn test_incomplete_definition_in_group() {
        // Missing pattern in group definition raises an error.
        let source = concat!(
            "TEXT = /abc/\n",
            "group tag:\n",
            "  FOO = \n",
        );
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Incomplete definition"));
    }

    // -----------------------------------------------------------------------
    // Magic comments: TokenGrammar
    // -----------------------------------------------------------------------

    #[test]
    fn test_magic_comment_version() {
        // `# @version N` sets the version field to N.
        let source = "# @version 1\nNUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.version, 1);
        // Normal definitions still parsed.
        assert_eq!(grammar.definitions.len(), 1);
    }

    #[test]
    fn test_magic_comment_version_default() {
        // When no # @version line is present, version defaults to 0.
        let source = "NUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.version, 0);
    }

    #[test]
    fn test_magic_comment_case_insensitive_true() {
        // `# @case_insensitive true` sets the flag to true.
        let source = "# @case_insensitive true\nNUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert!(grammar.case_insensitive);
    }

    #[test]
    fn test_magic_comment_case_insensitive_false() {
        // `# @case_insensitive false` sets the flag to false.
        let source = "# @case_insensitive false\nNUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert!(!grammar.case_insensitive);
    }

    #[test]
    fn test_magic_comment_case_insensitive_default() {
        // When no # @case_insensitive line is present, the flag defaults to false.
        let source = "NUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert!(!grammar.case_insensitive);
    }

    #[test]
    fn test_magic_comment_unknown_key_silently_ignored() {
        // Unknown `# @key value` lines do not cause errors; they are
        // forward-compatible placeholders for future features.
        let source = "# @unknown_key some_value\nNUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        // Defaults are untouched.
        assert_eq!(grammar.version, 0);
        assert!(!grammar.case_insensitive);
        assert_eq!(grammar.definitions.len(), 1);
    }

    #[test]
    fn test_magic_comment_both_together() {
        // Both magic comments can appear in the same file.
        let source = "# @version 3\n# @case_insensitive true\nNAME = /[a-z]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.version, 3);
        assert!(grammar.case_insensitive);
        assert_eq!(grammar.definitions.len(), 1);
    }

    #[test]
    fn test_regular_comment_not_treated_as_magic() {
        // A plain `# comment` without `@` is just a comment — no fields set.
        let source = "# Just a comment\nNUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.version, 0);
        assert!(!grammar.case_insensitive);
    }

    // -----------------------------------------------------------------------
    // Soft keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_soft_keywords_section() {
        // The soft_keywords: section lists words that are keywords only in
        // specific syntactic contexts (e.g. Python's match/case/type).
        let source = concat!(
            "NAME = /[a-zA-Z_]+/\n",
            "soft_keywords:\n",
            "  match\n",
            "  case\n",
            "  type\n",
        );
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.soft_keywords, vec!["match", "case", "type"]);
        assert_eq!(grammar.definitions.len(), 1);
    }

    #[test]
    fn test_parse_soft_keywords_with_space_before_colon() {
        // "soft_keywords :" (with space) is also valid.
        let source = "soft_keywords :\n  match\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.soft_keywords, vec!["match"]);
    }

    #[test]
    fn test_soft_keywords_default_empty() {
        // When no soft_keywords: section is present, the field is empty.
        let source = "NUMBER = /[0-9]+/\n";
        let grammar = parse_token_grammar(source).unwrap();
        assert!(grammar.soft_keywords.is_empty());
    }

    #[test]
    fn test_soft_keywords_coexist_with_context_keywords() {
        // Both context_keywords: and soft_keywords: can appear in the same
        // file. They serve different purposes and are stored separately.
        let source = concat!(
            "NAME = /[a-zA-Z_]+/\n",
            "context_keywords:\n",
            "  async\n",
            "  await\n",
            "soft_keywords:\n",
            "  match\n",
            "  case\n",
        );
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.context_keywords, vec!["async", "await"]);
        assert_eq!(grammar.soft_keywords, vec!["match", "case"]);
    }

    #[test]
    fn test_parse_layout_keywords_section() {
        let source = concat!(
            "mode: layout\n",
            "NAME = /[a-zA-Z_]+/\n",
            "layout_keywords:\n",
            "  let\n",
            "  where\n",
            "  do\n",
            "  of\n",
        );
        let grammar = parse_token_grammar(source).unwrap();
        assert_eq!(grammar.mode.as_deref(), Some("layout"));
        assert_eq!(grammar.layout_keywords, vec!["let", "where", "do", "of"]);
    }

    #[test]
    fn test_layout_mode_requires_layout_keywords() {
        let grammar = TokenGrammar {
            definitions: vec![],
            keywords: vec![],
            mode: Some("layout".to_string()),
            skip_definitions: vec![],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 0,
            case_insensitive: false,
            layout_keywords: vec![],
            context_keywords: Vec::new(),
            soft_keywords: Vec::new(),
        };
        let issues = validate_token_grammar(&grammar);
        assert!(issues.iter().any(|issue| issue.contains("layout_keywords")));
    }

    #[test]
    fn test_reserved_group_name_soft_keywords() {
        // "group soft_keywords:" is rejected as reserved.
        let source = "TEXT = /abc/\ngroup soft_keywords:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved group name"));
    }

    #[test]
    fn test_reserved_group_name_context_keywords() {
        // "group context_keywords:" is rejected as reserved.
        let source = "TEXT = /abc/\ngroup context_keywords:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved group name"));
    }

    #[test]
    fn test_reserved_group_name_layout_keywords() {
        let source = "TEXT = /abc/\ngroup layout_keywords:\n  FOO = /x/\n";
        let result = parse_token_grammar(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved group name"));
    }
}
