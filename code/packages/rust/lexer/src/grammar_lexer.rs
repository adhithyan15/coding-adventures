//! # Grammar-driven lexer — tokenize any language from a grammar specification.
//!
//! The hand-written lexer in [`crate::tokenizer`] knows how to tokenize
//! Python-like code because its rules are baked into Rust source code. But
//! what if you want to tokenize a *different* language — say, SQL, or JSON,
//! or a custom DSL? You would need to write a whole new lexer.
//!
//! The grammar-driven lexer solves this problem. Instead of hard-coding
//! rules, it reads them from a [`TokenGrammar`] — a structured description
//! of all the tokens in a language, parsed from a `.tokens` file by the
//! [`grammar_tools`] crate.
//!
//! # How it works
//!
//! The grammar-driven lexer operates in two phases:
//!
//! ## Phase 1: Compilation (at construction time)
//!
//! When you create a `GrammarLexer`, it compiles each token definition
//! from the grammar into a regex pattern anchored to the start of the
//! string (`^`). Literal patterns are escaped so that special regex
//! characters like `+` and `*` are treated as literal characters.
//!
//! ```text
//! Grammar definition          Compiled regex
//! ------------------          --------------
//! NUMBER = /[0-9]+/     ->    ^[0-9]+
//! PLUS   = "+"          ->    ^\+
//! ```
//!
//! ## Phase 2: Tokenization (the main loop)
//!
//! The lexer walks through the source code. At each position, it tries
//! every compiled pattern in order. The **first pattern that matches wins**
//! (first-match-wins semantics). This is why the order of definitions in
//! the `.tokens` file matters — `==` must come before `=`, or `=` would
//! always match first and `==` would never be recognized.
//!
//! # Extensions for Starlark-like languages
//!
//! This lexer supports several extensions beyond basic grammar-driven tokenization:
//!
//! - **Skip patterns**: Patterns from the `skip:` section consume input without
//!   emitting tokens (e.g. whitespace, comments).
//!
//! - **Type aliases**: When a definition has `-> ALIAS`, the emitted token uses
//!   the alias name instead (e.g. `STRING_DQ -> STRING`).
//!
//! - **Reserved keywords**: Keywords from the `reserved:` section cause a lexer
//!   error if encountered in source code.
//!
//! - **Indentation mode**: When `mode: indentation` is set, the lexer tracks
//!   indentation levels and emits synthetic INDENT/DEDENT/NEWLINE tokens,
//!   following the Python/Starlark whitespace rules.

use regex::{Regex, RegexBuilder};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition};

use crate::token::{LexerError, Token, TokenType, string_to_token_type, TOKEN_CONTEXT_KEYWORD};

// ===========================================================================
// ContextAction — deferred mutations from the on-token callback
// ===========================================================================

/// An action recorded by a [`LexerContext`] callback.
///
/// The Rust borrow checker prevents the callback from directly mutating
/// the lexer (the callback borrows the token, which borrows the lexer).
/// Instead, the callback records actions in the context. After the
/// callback returns, the lexer's main loop applies them in order.
///
/// This is the classic "collect-then-apply" pattern — it keeps the
/// borrow checker happy while giving callbacks full control over lexer
/// state transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextAction {
    /// Push a named group onto the group stack.
    /// The group becomes active for the next token match.
    Push(String),

    /// Pop the current group from the stack.
    /// No-op if only the default group remains.
    Pop,

    /// Inject a synthetic token after the current one.
    /// Emitted tokens do NOT trigger the callback (prevents infinite loops).
    Emit(Token),

    /// Suppress the current token — omit it from output.
    Suppress,

    /// Toggle skip pattern processing on or off.
    SetSkipEnabled(bool),
}

// ===========================================================================
// LexerContext — the callback's view of the lexer
// ===========================================================================

/// Interface that on-token callbacks use to control the lexer.
///
/// When a callback is registered via [`GrammarLexer::set_on_token()`], it
/// receives a `&mut LexerContext` on every token match. The context
/// provides controlled access to the group stack, token emission, and
/// skip control.
///
/// Methods that modify state (push/pop/emit/suppress) record actions
/// that take effect after the callback returns — they do not interrupt
/// the current match.
///
/// # Example — XML lexer callback
///
/// ```text
/// |token, ctx| {
///     if token.type_name.as_deref() == Some("OPEN_TAG") {
///         ctx.push_group("tag").unwrap();
///     } else if token.type_name.as_deref() == Some("TAG_CLOSE") {
///         ctx.pop_group();
///     }
/// }
/// ```
pub struct LexerContext<'a> {
    /// The names of all defined groups (for validation in push_group).
    group_names: &'a HashSet<String>,

    /// Read-only view of the current group stack.
    group_stack: &'a Vec<String>,

    /// The source code being tokenized (for peek operations).
    source: &'a str,

    /// The byte position immediately after the current token.
    pos_after_token: usize,

    /// Accumulated actions to apply after the callback returns.
    actions: Vec<ContextAction>,

    /// Whether the current token has been suppressed.
    suppressed: bool,

    // -----------------------------------------------------------------------
    // Extension fields: lookbehind, bracket depth, newline detection
    // -----------------------------------------------------------------------

    /// The most recently emitted token (for lookbehind).
    ///
    /// "Emitted" means the token actually made it into the output list --
    /// suppressed tokens are not counted. This provides lookbehind
    /// capability for context-sensitive decisions.
    ///
    /// For example, in JavaScript `/` is a regex literal after `=`, `(`
    /// or `,` but a division operator after `)`, `]`, identifiers, or
    /// numbers. The callback can check `ctx.previous_token()` to decide.
    previous_token: Option<Token>,

    /// Per-type bracket nesting depths: (paren, bracket, brace).
    ///
    /// Exposed to callbacks so they can make context-sensitive decisions
    /// based on bracket nesting (e.g., template literal interpolation in
    /// JavaScript, where `}` at brace-depth 0 closes the interpolation).
    bracket_depths: BracketDepths,

    /// The current token's line number (for newline detection).
    current_token_line: usize,
}

impl<'a> LexerContext<'a> {
    /// Push a pattern group onto the stack.
    ///
    /// The pushed group becomes active for the next token match.
    /// Returns `Err` if the group name is not defined in the grammar.
    pub fn push_group(&mut self, group_name: &str) -> Result<(), String> {
        if !self.group_names.contains(group_name) {
            return Err(format!(
                "Unknown pattern group: {:?}. Available groups: {:?}",
                group_name,
                {
                    let mut names: Vec<&String> = self.group_names.iter().collect();
                    names.sort();
                    names
                }
            ));
        }
        self.actions.push(ContextAction::Push(group_name.to_string()));
        Ok(())
    }

    /// Pop the current group from the stack.
    ///
    /// If only the default group remains, this is a no-op. The default
    /// group is the floor and cannot be popped.
    pub fn pop_group(&mut self) {
        self.actions.push(ContextAction::Pop);
    }

    /// Return the name of the currently active group.
    pub fn active_group(&self) -> &str {
        self.group_stack.last().map(|s| s.as_str()).unwrap_or("default")
    }

    /// Return the depth of the group stack (always >= 1).
    pub fn group_stack_depth(&self) -> usize {
        self.group_stack.len()
    }

    /// Inject a synthetic token after the current one.
    ///
    /// Emitted tokens do NOT trigger the callback (prevents infinite
    /// loops). Multiple `emit()` calls produce tokens in call order.
    pub fn emit(&mut self, token: Token) {
        self.actions.push(ContextAction::Emit(token));
    }

    /// Suppress the current token — do not include it in output.
    pub fn suppress(&mut self) {
        self.suppressed = true;
    }

    /// Peek at a source character past the current token.
    ///
    /// `offset` is 1-based: `peek(1)` returns the character immediately
    /// after the current token. Returns `""` if past EOF.
    pub fn peek(&self, offset: usize) -> &str {
        let idx = self.pos_after_token + offset - 1;
        if idx < self.source.len() {
            // Safe because we check bounds. We return a 1-char slice.
            let ch = &self.source[idx..];
            let end = ch.char_indices().nth(1).map_or(ch.len(), |(i, _)| i);
            &self.source[idx..idx + end]
        } else {
            ""
        }
    }

    /// Peek at the next `length` characters past the current token.
    pub fn peek_str(&self, length: usize) -> &str {
        let start = self.pos_after_token;
        if start >= self.source.len() {
            return "";
        }
        // Find the byte position `length` chars ahead.
        let remaining = &self.source[start..];
        let end_byte = remaining
            .char_indices()
            .nth(length)
            .map_or(remaining.len(), |(i, _)| i);
        &self.source[start..start + end_byte]
    }

    /// Toggle skip pattern processing.
    ///
    /// When disabled, skip patterns (whitespace, comments) are not tried.
    /// Useful for groups where whitespace is significant (e.g., CDATA).
    pub fn set_skip_enabled(&mut self, enabled: bool) {
        self.actions.push(ContextAction::SetSkipEnabled(enabled));
    }

    // -----------------------------------------------------------------------
    // Extension: Token lookbehind
    // -----------------------------------------------------------------------

    /// Return the most recently emitted token, or `None` at the start of input.
    ///
    /// "Emitted" means the token actually made it into the output list --
    /// suppressed tokens are not counted. This provides lookbehind
    /// capability for context-sensitive decisions.
    pub fn previous_token(&self) -> Option<&Token> {
        self.previous_token.as_ref()
    }

    // -----------------------------------------------------------------------
    // Extension: Bracket depth tracking
    // -----------------------------------------------------------------------

    /// Return the current nesting depth for a specific bracket type.
    ///
    /// Depth starts at 0 and increments on each opener (`(`, `[`, `{`),
    /// decrements on each closer (`)`, `]`, `}`). The count never goes
    /// below 0 -- unmatched closers are clamped.
    pub fn bracket_depth(&self, kind: BracketKind) -> usize {
        match kind {
            BracketKind::Paren => self.bracket_depths.paren,
            BracketKind::Bracket => self.bracket_depths.bracket,
            BracketKind::Brace => self.bracket_depths.brace,
        }
    }

    /// Return the total bracket nesting depth across all types.
    pub fn total_bracket_depth(&self) -> usize {
        self.bracket_depths.paren + self.bracket_depths.bracket + self.bracket_depths.brace
    }

    // -----------------------------------------------------------------------
    // Extension: Newline detection
    // -----------------------------------------------------------------------

    /// Return true if a newline appeared between the previous token
    /// and the current token (i.e., they are on different lines).
    ///
    /// Used by languages with automatic semicolon insertion (JavaScript, Go)
    /// to detect line breaks that trigger implicit statement termination.
    ///
    /// Returns false if there is no previous token (start of input).
    pub fn preceded_by_newline(&self) -> bool {
        match &self.previous_token {
            None => false,
            Some(prev) => prev.line < self.current_token_line,
        }
    }
}

// ===========================================================================
// Bracket depth tracking
// ===========================================================================

/// The three kinds of brackets tracked by the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BracketKind {
    /// Parentheses `(` and `)`.
    Paren,
    /// Square brackets `[` and `]`.
    Bracket,
    /// Curly braces `{` and `}`.
    Brace,
}

/// Per-type bracket nesting depth counters.
///
/// Tracks `()`, `[]`, and `{}` independently. Updated after each
/// token match. Exposed to callbacks via [`LexerContext::bracket_depth()`].
#[derive(Debug, Clone, Default)]
pub struct BracketDepths {
    pub paren: usize,
    pub bracket: usize,
    pub brace: usize,
}

impl BracketDepths {
    /// Update depths based on a matched token value.
    ///
    /// Call this after each token emission. Increments on openers,
    /// decrements (clamped to 0) on closers.
    fn update(&mut self, value: &str) {
        match value {
            "(" => self.paren += 1,
            ")" => self.paren = self.paren.saturating_sub(1),
            "[" => self.bracket += 1,
            "]" => self.bracket = self.bracket.saturating_sub(1),
            "{" => self.brace += 1,
            "}" => self.brace = self.brace.saturating_sub(1),
            _ => {}
        }
    }
}

// ===========================================================================
// Compiled pattern — a pre-compiled regex ready for matching
// ===========================================================================

/// A single token pattern, compiled and ready to match against source text.
struct CompiledPattern {
    /// The token name from the grammar (e.g., "NAME", "NUMBER", "PLUS").
    name: String,

    /// The compiled regex, anchored to the start of the string.
    pattern: Regex,

    /// Optional alias — when set, tokens matching this pattern are emitted
    /// with the alias as their type name instead of `name`.
    alias: Option<String>,
}

/// Compile a list of token definitions into anchored regex patterns.
fn compile_patterns(definitions: &[TokenDefinition], case_sensitive: bool) -> Vec<CompiledPattern> {
    definitions
        .iter()
        .map(|defn| {
            let regex_str = if defn.is_regex {
                format!("^(?:{})", defn.pattern)
            } else {
                format!("^{}", regex::escape(&defn.pattern))
            };

            let compiled = RegexBuilder::new(&regex_str)
                .case_insensitive(!case_sensitive)
                .build()
                .unwrap_or_else(|e| {
                panic!(
                    "Failed to compile pattern for token {}: {}",
                    defn.name, e
                )
            });

            CompiledPattern {
                name: defn.name.clone(),
                pattern: compiled,
                alias: defn.alias.clone(),
            }
        })
        .collect()
}

// ===========================================================================
// Callback type alias
// ===========================================================================

/// Type alias for the on-token callback.
///
/// The callback receives a reference to the matched token and a mutable
/// reference to a [`LexerContext`]. It can use the context to push/pop
/// groups, emit synthetic tokens, suppress the current token, or toggle
/// skip processing.
pub type OnTokenCallback = Box<dyn FnMut(&Token, &mut LexerContext)>;

// ===========================================================================
// GrammarLexer
// ===========================================================================

/// A lexer that tokenizes source code according to a [`TokenGrammar`].
///
/// Supports standard mode (simple pattern matching with whitespace skipping)
/// and indentation mode (Python/Starlark-style significant whitespace with
/// synthetic INDENT/DEDENT/NEWLINE tokens).
///
/// # Pattern groups and callbacks
///
/// The lexer supports **pattern groups** — named sets of token patterns
/// that can be activated dynamically via a callback. This enables
/// context-sensitive lexing without modifying the grammar.
///
/// For example, an XML lexer defines a "tag" group with patterns for
/// attribute names, equals signs, and attribute values. A callback
/// pushes the "tag" group when `<` is matched and pops it when `>`
/// is matched. Outside tags, only the default group's patterns are tried.
///
/// The callback is registered via [`set_on_token()`](GrammarLexer::set_on_token)
/// and receives each token plus a [`LexerContext`] that provides controlled
/// access to group stack manipulation, token emission, and skip control.
pub struct GrammarLexer<'a> {
    /// The source code as character vector for indexed access.
    chars: Vec<char>,

    /// The original source string (for regex matching on slices).
    ///
    /// We preserve the original casing so identifiers and string literals
    /// retain their source text even when the grammar matches case-insensitively.
    source: Cow<'a, str>,

    /// Current byte position in the source string.
    byte_pos: usize,

    /// Current character position.
    char_pos: usize,

    /// Current line number (1-based).
    line: usize,

    /// Current column number (1-based).
    column: usize,

    /// Keywords for keyword promotion (NAME -> KEYWORD).
    keyword_set: HashSet<String>,

    /// Reserved keywords that cause errors if encountered.
    reserved_set: HashSet<String>,

    /// Pre-compiled token patterns in priority order (the "default" group).
    patterns: Vec<CompiledPattern>,

    /// Pre-compiled skip patterns (whitespace, comments).
    skip_patterns: Vec<CompiledPattern>,

    /// Whether indentation mode is active.
    indent_mode: bool,
    /// Whether Haskell-style layout mode is active.
    layout_mode: bool,

    /// Escape processing mode. When set to `"none"`, STRING tokens have their
    /// quotes stripped but escape sequences are left as raw text. This is used
    /// by grammars like CSS and TOML where escape processing is deferred to a
    /// semantic layer (e.g., TOML has four string types with different escape
    /// rules). When `None`, the default escape processing (`\n`, `\t`, `\\`,
    /// `\"`) is applied.
    escape_mode: Option<String>,

    // --- Pattern group support ---

    /// Compiled patterns for each named group. The "default" group contains
    /// the top-level definitions. Named groups come from `group NAME:` sections.
    group_patterns: HashMap<String, Vec<CompiledPattern>>,

    /// The set of all valid group names (for validation in LexerContext).
    group_names: HashSet<String>,

    /// The group stack. Bottom is always "default". Top is the active group
    /// whose patterns are tried during token matching.
    group_stack: Vec<String>,

    /// On-token callback — `None` means no callback (zero overhead path).
    ///
    /// The callback fires after each token match, before emission. It
    /// receives the matched token and a `LexerContext` for recording
    /// actions. The callback is NOT invoked for:
    /// - Skip pattern matches (they produce no tokens)
    /// - Tokens emitted via `context.emit()` (prevents infinite loops)
    /// - The EOF token
    on_token: Option<OnTokenCallback>,

    /// Whether skip patterns should be tried. Callbacks can toggle this
    /// via `LexerContext::set_skip_enabled()` for groups where whitespace
    /// is significant (e.g., CDATA, raw strings).
    skip_enabled: bool,

    /// Pre-tokenize hooks: transform source text before lexing.
    /// Each hook is a function `String -> String`. Multiple hooks compose left-to-right.
    pre_tokenize_hooks: Vec<Box<dyn Fn(String) -> String>>,

    /// Post-tokenize hooks: transform token list after lexing.
    /// Each hook is a function `Vec<Token> -> Vec<Token>`. Multiple hooks compose left-to-right.
    post_tokenize_hooks: Vec<Box<dyn Fn(Vec<Token>) -> Vec<Token>>>,

    /// Whether keyword matching is case-insensitive.
    ///
    /// When `true` (set by `# @case_insensitive true` in the grammar file),
    /// keywords are stored as uppercase strings in `keyword_set` and
    /// `reserved_set`. During NAME token matching, the token value is
    /// uppercased before the lookup, and if a keyword is found, the emitted
    /// token value is normalized to uppercase.
    ///
    /// This means `select`, `SELECT`, and `Select` all produce a KEYWORD
    /// token with value `"SELECT"` when `SELECT` is in the keyword list.
    case_insensitive: bool,

    // -----------------------------------------------------------------------
    // Extension fields: lookbehind, bracket depth, context keywords
    // -----------------------------------------------------------------------

    /// The most recently emitted token, for lookbehind in callbacks.
    ///
    /// Updated after each token push (including callback-emitted tokens).
    /// Reset to `None` on each `tokenize()` call. Exposed to callbacks
    /// via [`LexerContext::previous_token()`].
    last_emitted_token: Option<Token>,

    /// Per-type bracket nesting depth counters.
    ///
    /// Tracks `()`, `[]`, and `{}` independently. Updated after each
    /// token match in both standard and indentation modes. Exposed to
    /// callbacks via [`LexerContext::bracket_depth()`].
    bracket_depths: BracketDepths,

    /// Context-sensitive keywords -- words that are keywords in some
    /// syntactic positions but identifiers in others.
    ///
    /// These are emitted as NAME tokens with the [`TOKEN_CONTEXT_KEYWORD`]
    /// flag set, leaving the final keyword-vs-identifier decision to the
    /// language-specific parser or callback.
    ///
    /// Examples: JavaScript's `async`, `await`, `yield`, `get`, `set`.
    context_keyword_set: HashSet<String>,
    /// Keywords that introduce Haskell-style layout contexts.
    layout_keyword_set: HashSet<String>,
}

impl<'a> GrammarLexer<'a> {
    /// Create a new grammar-driven lexer for the given source code.
    ///
    /// Compiles all token definitions (including group definitions) into
    /// anchored regex patterns. The "default" group contains the top-level
    /// definitions; named groups come from `group NAME:` sections.
    pub fn new(source: &'a str, grammar: &TokenGrammar) -> Self {
        let case_sensitive = grammar.case_sensitive;
        let case_insensitive = grammar.case_insensitive;

        // When case-insensitive mode is on, store all keywords as uppercase
        // so that lookups can be performed with value.to_uppercase(). This
        // way "select", "SELECT", and "Select" all match the same entry.
        let keyword_set: HashSet<String> = if case_insensitive {
            grammar.keywords.iter().map(|kw| kw.to_uppercase()).collect()
        } else {
            grammar.keywords.iter().cloned().collect()
        };
        let reserved_set: HashSet<String> = if case_insensitive {
            grammar.reserved_keywords.iter().map(|kw| kw.to_uppercase()).collect()
        } else {
            grammar.reserved_keywords.iter().cloned().collect()
        };
        let patterns = compile_patterns(&grammar.definitions, case_sensitive);
        let skip_patterns = compile_patterns(&grammar.skip_definitions, case_sensitive);
        let indent_mode = grammar.mode.as_deref() == Some("indentation");
        let layout_mode = grammar.mode.as_deref() == Some("layout");
        let escape_mode = grammar.escapes.clone();

        // --- Build per-group compiled patterns ---
        // The "default" group uses the top-level definitions. Named groups
        // use their own definitions from the grammar's `groups` map.
        let mut group_patterns: HashMap<String, Vec<CompiledPattern>> = HashMap::new();

        // Clone the default patterns for the "default" group entry.
        let default_compiled: Vec<CompiledPattern> = grammar
            .definitions
            .iter()
            .map(|defn| {
            let regex_str = if defn.is_regex {
                format!("^(?:{})", defn.pattern)
            } else {
                format!("^{}", regex::escape(&defn.pattern))
            };
                CompiledPattern {
                    name: defn.name.clone(),
                    pattern: RegexBuilder::new(&regex_str)
                        .case_insensitive(!case_sensitive)
                        .build()
                        .unwrap(),
                    alias: defn.alias.clone(),
                }
            })
            .collect();
        group_patterns.insert("default".to_string(), default_compiled);

        for (group_name, group) in &grammar.groups {
            let compiled = compile_patterns(&group.definitions, case_sensitive);
            group_patterns.insert(group_name.clone(), compiled);
        }

        // Build the set of valid group names for validation.
        let group_names: HashSet<String> = group_patterns.keys().cloned().collect();

        // Build the context keyword set from the grammar.
        // Context keywords are emitted as NAME tokens with TOKEN_CONTEXT_KEYWORD flag.
        let context_keyword_set: HashSet<String> = grammar
            .context_keywords
            .iter()
            .cloned()
            .collect();
        let layout_keyword_set: HashSet<String> = grammar
            .layout_keywords
            .iter()
            .cloned()
            .collect();
        GrammarLexer {
            chars: source.chars().collect(),
            source: Cow::Borrowed(source),
            byte_pos: 0,
            char_pos: 0,
            line: 1,
            column: 1,
            keyword_set,
            reserved_set,
            patterns,
            skip_patterns,
            indent_mode,
            layout_mode,
            escape_mode,
            group_patterns,
            group_names,
            group_stack: vec!["default".to_string()],
            on_token: None,
            skip_enabled: true,
            pre_tokenize_hooks: Vec::new(),
            post_tokenize_hooks: Vec::new(),
            case_insensitive,
            last_emitted_token: None,
            bracket_depths: BracketDepths::default(),
            context_keyword_set,
            layout_keyword_set,
        }
    }

    /// Register a callback that fires on every token match.
    ///
    /// The callback receives the matched token and a [`LexerContext`].
    /// It can use the context to push/pop groups, emit extra tokens,
    /// or suppress the current token.
    ///
    /// Only one callback can be registered. Pass `None` to clear.
    ///
    /// The callback is NOT invoked for:
    /// - Skip pattern matches (they produce no tokens)
    /// - Tokens emitted via `context.emit()` (prevents infinite loops)
    /// - The EOF token
    pub fn set_on_token(&mut self, callback: Option<OnTokenCallback>) {
        self.on_token = callback;
    }

    /// Register a text transform to run before tokenization.
    ///
    /// The hook receives the source string and returns a (possibly modified)
    /// source string. Multiple hooks compose left-to-right.
    pub fn add_pre_tokenize(&mut self, hook: Box<dyn Fn(String) -> String>) {
        self.pre_tokenize_hooks.push(hook);
    }

    /// Register a token transform to run after tokenization.
    ///
    /// The hook receives the full token list and returns a (possibly modified)
    /// token list. Multiple hooks compose left-to-right.
    pub fn add_post_tokenize(&mut self, hook: Box<dyn Fn(Vec<Token>) -> Vec<Token>>) {
        self.post_tokenize_hooks.push(hook);
    }

    // -----------------------------------------------------------------------
    // Cursor operations
    // -----------------------------------------------------------------------

    fn advance(&mut self) {
        if self.char_pos < self.chars.len() {
            let ch = self.chars[self.char_pos];
            self.byte_pos += ch.len_utf8();
            self.char_pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    #[allow(dead_code)]
    fn current_char(&self) -> Option<char> {
        self.chars.get(self.char_pos).copied()
    }

    // -----------------------------------------------------------------------
    // Token type resolution
    // -----------------------------------------------------------------------

    /// Resolve a grammar token name to a TokenType and optional string type name.
    ///
    /// Handles keyword promotion, reserved keyword rejection, alias resolution,
    /// and fallback to string-based type names for custom token types.
    fn resolve_token_type(
        &self,
        token_name: &str,
        alias: Option<&str>,
        value: &str,
    ) -> Result<(TokenType, Option<String>), LexerError> {
        // When case-insensitive mode is active, compare against the keyword
        // and reserved sets using the uppercased form of the value. The sets
        // were built with uppercase entries in GrammarLexer::new.
        let lookup_key: String;
        let lookup_value: &str = if self.case_insensitive {
            lookup_key = value.to_uppercase();
            &lookup_key
        } else {
            value
        };

        // Check reserved keywords first — if a NAME matches a reserved word,
        // that's an error.
        if token_name == "NAME" && self.reserved_set.contains(lookup_value) {
            return Err(LexerError {
                message: format!("Reserved keyword '{}' cannot be used as an identifier", value),
                line: self.line,
                column: self.column,
            });
        }

        // Keyword promotion: NAME tokens whose value is in the keyword set.
        // When case-insensitive, emit the normalized (uppercase) form so that
        // `select`, `SELECT`, and `Select` all produce the same token value.
        if token_name == "NAME" && self.keyword_set.contains(lookup_value) {
            return Ok((TokenType::Keyword, Some("KEYWORD".to_string())));
        }

        // Determine the effective type name (alias takes precedence).
        let effective_name = alias.unwrap_or(token_name);

        // Try to map to a known TokenType enum variant.
        let token_type = string_to_token_type(effective_name);

        // If string_to_token_type returned Name but the effective name is not
        // "NAME", it means we have a custom type — store it as type_name.
        if token_type == TokenType::Name && effective_name != "NAME" {
            Ok((token_type, Some(effective_name.to_string())))
        } else {
            Ok((token_type, None))
        }
    }

    // -----------------------------------------------------------------------
    // Quote detection helpers
    // -----------------------------------------------------------------------

    /// Check if a matched string starts and ends with matching quote characters.
    ///
    /// Handles both single-char quotes (`"..."`, `'...'`) and triple-char
    /// quotes (`"""..."""`, `'''...'''`) used by languages like TOML and Python.
    fn is_quoted(s: &str) -> bool {
        if s.len() >= 6 && (s.starts_with("\"\"\"") && s.ends_with("\"\"\"")
            || s.starts_with("'''") && s.ends_with("'''"))
        {
            return true;
        }
        if s.len() >= 2 {
            let first = s.as_bytes()[0];
            let last = s.as_bytes()[s.len() - 1];
            return (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'');
        }
        false
    }

    /// Return the number of quote characters on each side (1 or 3).
    fn quote_len(s: &str) -> usize {
        if s.len() >= 6 && (s.starts_with("\"\"\"") || s.starts_with("'''")) {
            3
        } else {
            1
        }
    }

    // -----------------------------------------------------------------------
    // Escape processing
    // -----------------------------------------------------------------------

    fn process_escapes(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                let next = chars[i + 1];
                match next {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    other => result.push(other),
                }
                i += 2;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // Skip pattern matching
    // -----------------------------------------------------------------------

    /// Try to match and consume a skip pattern at the current position.
    /// Returns true if something was skipped.
    fn try_skip(&mut self) -> bool {
        let remaining = &self.source[self.byte_pos..];
        for p in &self.skip_patterns {
            if let Some(m) = p.pattern.find(remaining) {
                let char_count = m.as_str().chars().count();
                self.advance_n(char_count);
                return true;
            }
        }
        false
    }

    /// Try to match a token pattern at the current position using the
    /// default group's patterns. Used by indentation mode (which does
    /// not support group switching).
    ///
    /// Returns `(name, alias, matched_text)` or `None`.
    fn try_match_token(&self) -> Option<(String, Option<String>, String)> {
        let remaining = &self.source[self.byte_pos..];
        for p in &self.patterns {
            if let Some(m) = p.pattern.find(remaining) {
                return Some((
                    p.name.clone(),
                    p.alias.clone(),
                    m.as_str().to_string(),
                ));
            }
        }
        None
    }

    /// Try to match a token pattern from a specific named group.
    ///
    /// Tries each compiled pattern in the named group in priority order
    /// (first match wins). Falls back to the default patterns if the
    /// group name is not found (defensive, should not happen in practice).
    ///
    /// Returns `(name, alias, matched_text)` or `None`.
    fn try_match_token_in_group(&self, group_name: &str) -> Option<(String, Option<String>, String)> {
        let remaining = &self.source[self.byte_pos..];
        let patterns = match self.group_patterns.get(group_name) {
            Some(p) => p,
            None => &self.patterns, // fallback to default
        };
        for p in patterns {
            if let Some(m) = p.pattern.find(remaining) {
                return Some((
                    p.name.clone(),
                    p.alias.clone(),
                    m.as_str().to_string(),
                ));
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Main tokenization — standard mode
    // -----------------------------------------------------------------------

    fn tokenize_standard(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        let has_skip_patterns = !self.skip_patterns.is_empty();

        while self.char_pos < self.chars.len() {
            let ch = self.chars[self.char_pos];

            // --- Skip patterns (grammar-defined) ---
            // When the grammar has skip patterns AND skip is enabled, they
            // take over whitespace handling. The callback can disable skip
            // processing for groups where whitespace is significant (CDATA).
            if has_skip_patterns {
                if self.skip_enabled && self.try_skip() {
                    continue;
                }
            } else {
                // --- Default whitespace skip ---
                // Without skip patterns, use hardcoded behavior: skip
                // spaces, tabs, carriage returns silently.
                if ch == ' ' || ch == '\t' || ch == '\r' {
                    self.advance();
                    continue;
                }
            }

            // --- Newlines ---
            if ch == '\n' {
                tokens.push(Token {
                    type_: TokenType::Newline,
                    value: "\\n".to_string(),
                    line: self.line,
                    column: self.column,
                    type_name: None, flags: None,
                });
                self.advance();
                continue;
            }

            // --- Try active group's token patterns (first match wins) ---
            // The active group is the top of the group stack. When no
            // groups are defined, this is always "default" (the top-level
            // definitions), preserving backward compatibility.
            let active_group = self.group_stack.last().cloned().unwrap_or_else(|| "default".to_string());
            if let Some((name, alias, matched)) = self.try_match_token_in_group(&active_group) {
                let start_line = self.line;
                let start_col = self.column;

                let (token_type, type_name) = self.resolve_token_type(
                    &name,
                    alias.as_deref(),
                    &matched,
                )?;

                // For STRING tokens, strip quotes and process escapes.
                //
                // A token is considered a "string" if its effective name ends
                // with "STRING" (catches STRING, BASIC_STRING, LITERAL_STRING,
                // ML_BASIC_STRING, ML_LITERAL_STRING, etc.) AND its matched
                // text starts and ends with matching quote characters.
                //
                // When escape_mode is "none", we strip quotes but leave escape
                // sequences as raw text. This is used by grammars like CSS and
                // TOML where the semantic layer handles type-specific escape
                // processing (e.g., TOML has four string types with different
                // escape rules).
                let effective_name = alias.as_deref().unwrap_or(&name);
                let final_value = if effective_name.ends_with("STRING") && matched.len() >= 2
                    && Self::is_quoted(&matched)
                {
                    let quote_len = Self::quote_len(&matched);
                    let inner = &matched[quote_len..matched.len() - quote_len];
                    if self.escape_mode.as_deref() == Some("none") {
                        inner.to_string()
                    } else {
                        Self::process_escapes(inner)
                    }
                } else if self.case_insensitive && token_type == TokenType::Keyword {
                    // In case-insensitive mode, normalize keyword values to
                    // uppercase so that "select", "SELECT", and "Select" all
                    // produce the same canonical token value ("SELECT").
                    matched.to_uppercase()
                } else {
                    matched.clone()
                };

                let char_count = matched.chars().count();
                self.advance_n(char_count);

                // Set the TOKEN_CONTEXT_KEYWORD flag for context-sensitive
                // keywords. These are NAME tokens whose value appears in
                // the `context_keywords:` section of the grammar. The flag
                // tells the parser that this identifier might be a keyword
                // depending on syntactic context.
                let flags = if (token_type == TokenType::Name || type_name.as_deref() == Some("NAME"))
                    && self.context_keyword_set.contains(&final_value)
                {
                    Some(TOKEN_CONTEXT_KEYWORD)
                } else {
                    None
                };

                let token = Token {
                    type_: token_type,
                    value: final_value,
                    line: start_line,
                    column: start_col,
                    type_name,
                    flags,
                };

                // --- Invoke on-token callback ---
                // The callback can push/pop groups, emit extra tokens,
                // suppress the current token, or toggle skip processing.
                // Emitted tokens do NOT re-trigger the callback.
                if self.on_token.is_some() {
                    // Build a LexerContext for the callback. We need to
                    // temporarily take the callback out of self to satisfy
                    // the borrow checker (callback borrows token, which
                    // would conflict with &mut self).
                    let mut ctx = LexerContext {
                        group_names: &self.group_names,
                        group_stack: &self.group_stack,
                        source: &self.source,
                        pos_after_token: self.byte_pos,
                        actions: Vec::new(),
                        suppressed: false,
                        previous_token: self.last_emitted_token.clone(),
                        bracket_depths: self.bracket_depths.clone(),
                        current_token_line: token.line,
                    };

                    // Take the callback out temporarily.
                    let mut callback = self.on_token.take().unwrap();
                    callback(&token, &mut ctx);
                    // Put it back.
                    self.on_token = Some(callback);

                    // Apply suppression: if the callback suppressed this
                    // token, don't add it to the output.
                    if !ctx.suppressed {
                        self.bracket_depths.update(&token.value);
                        self.last_emitted_token = Some(token.clone());
                        tokens.push(token);
                    }

                    // Process actions in order.
                    for action in ctx.actions {
                        match action {
                            ContextAction::Push(group_name) => {
                                self.group_stack.push(group_name);
                            }
                            ContextAction::Pop => {
                                if self.group_stack.len() > 1 {
                                    self.group_stack.pop();
                                }
                            }
                            ContextAction::Emit(emitted_token) => {
                                self.bracket_depths.update(&emitted_token.value);
                                self.last_emitted_token = Some(emitted_token.clone());
                                tokens.push(emitted_token);
                            }
                            ContextAction::Suppress => {
                                // Already handled above via ctx.suppressed
                            }
                            ContextAction::SetSkipEnabled(enabled) => {
                                self.skip_enabled = enabled;
                            }
                        }
                    }
                } else {
                    self.bracket_depths.update(&token.value);
                    self.last_emitted_token = Some(token.clone());
                    tokens.push(token);
                }
                continue;
            }

            return Err(LexerError {
                message: format!("Unexpected sequence {:?}", ch),
                line: self.line,
                column: self.column,
            });
        }

        tokens.push(Token {
            type_: TokenType::Eof,
            value: String::new(),
            line: self.line,
            column: self.column,
            type_name: None, flags: None,
        });

        // Reset group stack and skip_enabled for reuse.
        self.group_stack = vec!["default".to_string()];
        self.skip_enabled = true;

        Ok(tokens)
    }

    // -----------------------------------------------------------------------
    // Main tokenization — indentation mode
    // -----------------------------------------------------------------------

    /// Tokenize with indentation tracking (Python/Starlark style).
    ///
    /// In indentation mode, the lexer:
    /// - Tracks an indent stack (starts at [0])
    /// - At each logical line start, counts leading spaces and emits
    ///   INDENT/DEDENT tokens as needed
    /// - Suppresses NEWLINE/INDENT/DEDENT inside brackets
    /// - Skips blank lines and comment-only lines
    /// - Rejects tabs in leading indentation
    fn tokenize_indentation(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        let mut indent_stack: Vec<usize> = vec![0];
        let mut bracket_depth: usize = 0;
        let mut at_line_start = true;

        while self.char_pos < self.chars.len() {
            // --- Line start: handle indentation ---
            if at_line_start && bracket_depth == 0 {
                at_line_start = false;

                // Count leading spaces (reject tabs).
                let mut spaces = 0;
                while self.char_pos < self.chars.len() {
                    let ch = self.chars[self.char_pos];
                    if ch == ' ' {
                        spaces += 1;
                        self.advance();
                    } else if ch == '\t' {
                        return Err(LexerError {
                            message: "Tabs are not allowed in indentation (use spaces)".to_string(),
                            line: self.line,
                            column: self.column,
                        });
                    } else {
                        break;
                    }
                }

                // Check for blank line or comment-only line.
                let is_blank_or_comment = if self.char_pos >= self.chars.len() {
                    true
                } else {
                    let ch = self.chars[self.char_pos];
                    ch == '\n' || ch == '\r' || ch == '#'
                };

                // Handle blank/comment lines — skip them without emitting
                // NEWLINE, but consume through the end of line.
                if is_blank_or_comment {
                    // Try skip patterns (for comments).
                    self.try_skip();
                    // Consume the newline if present.
                    if self.char_pos < self.chars.len() {
                        let ch = self.chars[self.char_pos];
                        if ch == '\n' {
                            self.advance();
                        } else if ch == '\r' {
                            self.advance();
                            if self.char_pos < self.chars.len() && self.chars[self.char_pos] == '\n' {
                                self.advance();
                            }
                        }
                    }
                    at_line_start = true;
                    continue;
                }

                // Compare indentation with the current stack top.
                let current_indent = *indent_stack.last().unwrap();
                let indent_line = self.line;
                let indent_col = self.column;

                if spaces > current_indent {
                    indent_stack.push(spaces);
                    tokens.push(Token {
                        type_: TokenType::Indent,
                        value: String::new(),
                        line: indent_line,
                        column: indent_col,
                        type_name: None, flags: None,
                    });
                } else if spaces < current_indent {
                    // Emit DEDENT for each level we're leaving.
                    while indent_stack.len() > 1 && *indent_stack.last().unwrap() > spaces {
                        indent_stack.pop();
                        tokens.push(Token {
                            type_: TokenType::Dedent,
                            value: String::new(),
                            line: indent_line,
                            column: indent_col,
                            type_name: None, flags: None,
                        });
                    }
                    // Check that we landed on a valid indentation level.
                    if *indent_stack.last().unwrap() != spaces {
                        return Err(LexerError {
                            message: "Indentation does not match any outer level".to_string(),
                            line: indent_line,
                            column: indent_col,
                        });
                    }
                }
                // If spaces == current_indent, no INDENT/DEDENT needed.

                continue;
            }

            let ch = self.chars[self.char_pos];

            // --- Skip whitespace (not newlines) ---
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // --- Try skip patterns ---
            if self.try_skip() {
                continue;
            }

            // --- Newlines ---
            if ch == '\n' {
                if bracket_depth == 0 {
                    tokens.push(Token {
                        type_: TokenType::Newline,
                        value: "\\n".to_string(),
                        line: self.line,
                        column: self.column,
                        type_name: None, flags: None,
                    });
                    at_line_start = true;
                }
                self.advance();
                continue;
            }

            // --- Try each pattern ---
            if let Some((name, alias, matched)) = self.try_match_token() {
                let start_line = self.line;
                let start_col = self.column;

                let (token_type, type_name) = self.resolve_token_type(
                    &name,
                    alias.as_deref(),
                    &matched,
                )?;

                // Track bracket depth.
                match matched.as_str() {
                    "(" | "[" | "{" => bracket_depth += 1,
                    ")" | "]" | "}" => {
                        if bracket_depth > 0 {
                            bracket_depth -= 1;
                        }
                    }
                    _ => {}
                }

                let effective_name = alias.as_deref().unwrap_or(&name);
                let final_value = if effective_name.ends_with("STRING") && matched.len() >= 2
                    && Self::is_quoted(&matched)
                {
                    let quote_len = Self::quote_len(&matched);
                    let inner = &matched[quote_len..matched.len() - quote_len];
                    if self.escape_mode.as_deref() == Some("none") {
                        inner.to_string()
                    } else {
                        Self::process_escapes(inner)
                    }
                } else if self.case_insensitive && token_type == TokenType::Keyword {
                    // In case-insensitive mode, normalize keyword values to
                    // uppercase so that "select", "SELECT", and "Select" all
                    // produce the same canonical token value ("SELECT").
                    matched.to_uppercase()
                } else {
                    matched.clone()
                };

                tokens.push(Token {
                    type_: token_type,
                    value: final_value,
                    line: start_line,
                    column: start_col,
                    type_name,
                    flags: None,
                });

                let char_count = matched.chars().count();
                self.advance_n(char_count);
                continue;
            }

            return Err(LexerError {
                message: format!("Unexpected sequence {:?}", ch),
                line: self.line,
                column: self.column,
            });
        }

        // At EOF: emit remaining DEDENTs.
        if bracket_depth == 0 {
            // Emit a final NEWLINE if the last token isn't one.
            let need_newline = tokens.last().map_or(false, |t| t.type_ != TokenType::Newline);
            if need_newline {
                tokens.push(Token {
                    type_: TokenType::Newline,
                    value: "\\n".to_string(),
                    line: self.line,
                    column: self.column,
                    type_name: None, flags: None,
                });
            }

            while indent_stack.len() > 1 {
                indent_stack.pop();
                tokens.push(Token {
                    type_: TokenType::Dedent,
                    value: String::new(),
                    line: self.line,
                    column: self.column,
                    type_name: None, flags: None,
                });
            }
        }

        tokens.push(Token {
            type_: TokenType::Eof,
            value: String::new(),
            line: self.line,
            column: self.column,
            type_name: None, flags: None,
        });

        Ok(tokens)
    }

    fn tokenize_layout(&mut self) -> Result<Vec<Token>, LexerError> {
        let tokens = self.tokenize_standard()?;
        Ok(self.apply_layout(tokens))
    }

    fn apply_layout(&self, tokens: Vec<Token>) -> Vec<Token> {
        let mut result = Vec::with_capacity(tokens.len());
        let mut layout_stack: Vec<usize> = Vec::new();
        let mut pending_layouts = 0usize;
        let mut suppress_depth = 0usize;

        for index in 0..tokens.len() {
            let token = tokens[index].clone();
            let type_name = token.type_name.as_deref().unwrap_or(match token.type_ {
                TokenType::Newline => "NEWLINE",
                TokenType::Eof => "EOF",
                _ => "",
            });

            if type_name == "NEWLINE" {
                result.push(token.clone());
                if suppress_depth == 0 {
                    if let Some(next_token) = self.next_layout_token(&tokens, index + 1) {
                        while !layout_stack.is_empty() && next_token.column < *layout_stack.last().unwrap() {
                            result.push(self.virtual_layout_token("VIRTUAL_RBRACE", "}", next_token));
                            layout_stack.pop();
                        }

                        let next_type = next_token.type_name.as_deref().unwrap_or(match next_token.type_ {
                            TokenType::Newline => "NEWLINE",
                            TokenType::Eof => "EOF",
                            _ => "",
                        });
                        if !layout_stack.is_empty()
                            && next_type != "EOF"
                            && next_token.value != "}"
                            && next_token.column == *layout_stack.last().unwrap()
                        {
                            result.push(self.virtual_layout_token("VIRTUAL_SEMICOLON", ";", next_token));
                        }
                    }
                }
                continue;
            }

            if type_name == "EOF" {
                while !layout_stack.is_empty() {
                    result.push(self.virtual_layout_token("VIRTUAL_RBRACE", "}", &token));
                    layout_stack.pop();
                }
                result.push(token);
                continue;
            }

            if pending_layouts > 0 {
                if token.value == "{" {
                    pending_layouts -= 1;
                } else {
                    for _ in 0..pending_layouts {
                        layout_stack.push(token.column);
                        result.push(self.virtual_layout_token("VIRTUAL_LBRACE", "{", &token));
                    }
                    pending_layouts = 0;
                }
            }

            result.push(token.clone());

            if !token
                .type_name
                .as_deref()
                .unwrap_or("")
                .starts_with("VIRTUAL_")
            {
                match token.value.as_str() {
                    "(" | "[" | "{" => suppress_depth += 1,
                    ")" | "]" | "}" => {
                        if suppress_depth > 0 {
                            suppress_depth -= 1;
                        }
                    }
                    _ => {}
                }
            }

            if self.is_layout_keyword(&token) {
                pending_layouts += 1;
            }
        }

        result
    }

    fn next_layout_token<'b>(&self, tokens: &'b [Token], start_index: usize) -> Option<&'b Token> {
        tokens[start_index..]
            .iter()
            .find(|token| {
                let type_name = token.type_name.as_deref().unwrap_or(match token.type_ {
                    TokenType::Newline => "NEWLINE",
                    TokenType::Eof => "EOF",
                    _ => "",
                });
                type_name != "NEWLINE"
            })
    }

    fn virtual_layout_token(&self, type_name: &str, value: &str, anchor: &Token) -> Token {
        Token {
            type_: TokenType::Name,
            value: value.to_string(),
            line: anchor.line,
            column: anchor.column,
            type_name: Some(type_name.to_string()),
            flags: None,
        }
    }

    fn is_layout_keyword(&self, token: &Token) -> bool {
        if self.layout_keyword_set.is_empty() {
            return false;
        }
        self.layout_keyword_set.contains(&token.value)
            || self.layout_keyword_set.contains(&token.value.to_lowercase())
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Tokenize the source code according to the grammar's patterns.
    ///
    /// Dispatches to either standard or indentation mode based on the
    /// grammar's mode directive.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        // Stage 1: Pre-tokenize hooks transform the source text.
        if !self.pre_tokenize_hooks.is_empty() {
            let mut source = self.source.clone().into_owned();
            for hook in &self.pre_tokenize_hooks {
                source = hook(source);
            }
            // Rebuild chars and source from the transformed text.
            self.chars = source.chars().collect();
            self.source = Cow::Owned(source);
        }

        // Stage 2: Core tokenization.
        let mut tokens = if self.indent_mode {
            self.tokenize_indentation()?
        } else if self.layout_mode {
            self.tokenize_layout()?
        } else {
            self.tokenize_standard()?
        };

        // Stage 3: Post-tokenize hooks transform the token list.
        for hook in &self.post_tokenize_hooks {
            tokens = hook(tokens);
        }

        Ok(tokens)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use grammar_tools::token_grammar::parse_token_grammar;

    // -----------------------------------------------------------------------
    // Helper: build a Python-like grammar for testing
    // -----------------------------------------------------------------------

    fn python_grammar() -> TokenGrammar {
        parse_token_grammar(
            r#"
# Token definitions for a subset of Python
NAME          = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER        = /[0-9]+/
STRING        = /"([^"\\]|\\.)*"/
EQUALS_EQUALS = "=="
EQUALS        = "="
PLUS          = "+"
MINUS         = "-"
STAR          = "*"
SLASH         = "/"
LPAREN        = "("
RPAREN        = ")"
COMMA         = ","
COLON         = ":"
SEMICOLON     = ";"
LBRACE        = "{"
RBRACE        = "}"
LBRACKET      = "["
RBRACKET      = "]"
DOT           = "."
BANG          = "!"
keywords:
  if
  else
  while
  def
  return
"#,
        )
        .unwrap()
    }

    fn tokenize(source: &str) -> Vec<Token> {
        let grammar = python_grammar();
        GrammarLexer::new(source, &grammar).tokenize().unwrap()
    }

    // -----------------------------------------------------------------------
    // Basic arithmetic
    // -----------------------------------------------------------------------

    #[test]
    fn test_math_expression() {
        let tokens = tokenize("x = 1 + 2 * 3");

        let expected = vec![
            (TokenType::Name, "x"),
            (TokenType::Equals, "="),
            (TokenType::Number, "1"),
            (TokenType::Plus, "+"),
            (TokenType::Number, "2"),
            (TokenType::Star, "*"),
            (TokenType::Number, "3"),
            (TokenType::Eof, ""),
        ];

        assert_eq!(tokens.len(), expected.len());
        for (i, (exp_type, exp_val)) in expected.iter().enumerate() {
            assert_eq!(tokens[i].type_, *exp_type, "token {} type mismatch", i);
            assert_eq!(tokens[i].value, *exp_val, "token {} value mismatch", i);
        }
    }

    // -----------------------------------------------------------------------
    // Keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_keyword_promotion() {
        let tokens = tokenize("if x == 5");
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "if");
        assert_eq!(tokens[2].type_, TokenType::EqualsEquals);
    }

    #[test]
    fn test_non_keyword_is_name() {
        let tokens = tokenize("foo");
        assert_eq!(tokens[0].type_, TokenType::Name);
    }

    // -----------------------------------------------------------------------
    // String literals
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_literal() {
        let tokens = tokenize(r#""hello""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "hello");
    }

    #[test]
    fn test_string_escape_newline() {
        let tokens = tokenize(r#"print("Hello\n")"#);
        assert_eq!(tokens[2].type_, TokenType::String);
        assert_eq!(tokens[2].value, "Hello\n");
    }

    #[test]
    fn test_string_escape_tab() {
        let tokens = tokenize(r#""\t""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "\t");
    }

    #[test]
    fn test_string_escape_backslash() {
        let tokens = tokenize(r#""\\""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "\\");
    }

    // -----------------------------------------------------------------------
    // All single-character tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_operators_and_delimiters() {
        let tokens = tokenize("+ - * / ( ) , : ; { } [ ] . !");

        let expected_types = vec![
            TokenType::Plus,
            TokenType::Minus,
            TokenType::Star,
            TokenType::Slash,
            TokenType::LParen,
            TokenType::RParen,
            TokenType::Comma,
            TokenType::Colon,
            TokenType::Semicolon,
            TokenType::LBrace,
            TokenType::RBrace,
            TokenType::LBracket,
            TokenType::RBracket,
            TokenType::Dot,
            TokenType::Bang,
            TokenType::Eof,
        ];

        assert_eq!(tokens.len(), expected_types.len());
        for (i, exp_type) in expected_types.iter().enumerate() {
            assert_eq!(tokens[i].type_, *exp_type, "token {} type mismatch", i);
        }
    }

    // -----------------------------------------------------------------------
    // Equals vs EqualsEquals
    // -----------------------------------------------------------------------

    #[test]
    fn test_equals_single() {
        let tokens = tokenize("x = 5");
        assert_eq!(tokens[1].type_, TokenType::Equals);
    }

    #[test]
    fn test_equals_double() {
        let tokens = tokenize("x == 5");
        assert_eq!(tokens[1].type_, TokenType::EqualsEquals);
    }

    // -----------------------------------------------------------------------
    // Newlines
    // -----------------------------------------------------------------------

    #[test]
    fn test_newline_tokens() {
        let tokens = tokenize("x\ny");
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[1].type_, TokenType::Newline);
        assert_eq!(tokens[2].type_, TokenType::Name);
    }

    // -----------------------------------------------------------------------
    // Position tracking
    // -----------------------------------------------------------------------

    #[test]
    fn test_position_tracking() {
        let tokens = tokenize("x = 1");
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);
        assert_eq!(tokens[1].line, 1);
        assert_eq!(tokens[1].column, 3);
        assert_eq!(tokens[2].line, 1);
        assert_eq!(tokens[2].column, 5);
    }

    #[test]
    fn test_multiline_position() {
        let tokens = tokenize("x\ny");
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[2].line, 2);
    }

    // -----------------------------------------------------------------------
    // Empty input / error
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }

    #[test]
    fn test_unexpected_character() {
        let grammar = python_grammar();
        let result = GrammarLexer::new("x @ y", &grammar).tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unexpected"));
    }

    // -----------------------------------------------------------------------
    // Complex expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_definition() {
        let tokens = tokenize("def add(x, y):\n    return x + y");
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "def");
        let return_tok = tokens.iter().find(|t| t.value == "return").unwrap();
        assert_eq!(return_tok.type_, TokenType::Keyword);
    }

    // -----------------------------------------------------------------------
    // Process escapes (unit test)
    // -----------------------------------------------------------------------

    #[test]
    fn test_process_escapes_newline() {
        assert_eq!(GrammarLexer::process_escapes(r"hello\nworld"), "hello\nworld");
    }

    #[test]
    fn test_process_escapes_tab() {
        assert_eq!(GrammarLexer::process_escapes(r"a\tb"), "a\tb");
    }

    #[test]
    fn test_process_escapes_backslash() {
        assert_eq!(GrammarLexer::process_escapes(r"a\\b"), "a\\b");
    }

    #[test]
    fn test_process_escapes_quote() {
        assert_eq!(GrammarLexer::process_escapes(r#"a\"b"#), "a\"b");
    }

    #[test]
    fn test_process_escapes_unknown() {
        assert_eq!(GrammarLexer::process_escapes(r"a\xb"), "axb");
    }

    #[test]
    fn test_process_escapes_no_escapes() {
        assert_eq!(GrammarLexer::process_escapes("plain text"), "plain text");
    }

    // -----------------------------------------------------------------------
    // Grammar without keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_without_keywords() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nNUMBER = /[0-9]+/\nPLUS = \"+\"\n",
        )
        .unwrap();
        let tokens = GrammarLexer::new("if 42", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "if");
    }

    // -----------------------------------------------------------------------
    // Consistency: grammar lexer matches hand-written lexer
    // -----------------------------------------------------------------------

    #[test]
    fn test_consistency_with_hand_written_lexer() {
        use crate::tokenizer::Lexer;
        let source = "x = 1 + 2 * 3";
        let hand_tokens = Lexer::new(source, None).tokenize().unwrap();
        let grammar = python_grammar();
        let grammar_tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();
        assert_eq!(hand_tokens.len(), grammar_tokens.len());
        for (i, (h, g)) in hand_tokens.iter().zip(grammar_tokens.iter()).enumerate() {
            assert_eq!(h.type_, g.type_, "token {} type mismatch", i);
            assert_eq!(h.value, g.value, "token {} value mismatch", i);
        }
    }

    #[test]
    fn test_consistency_with_keywords() {
        use crate::tokenizer::{Lexer, LexerConfig};
        let source = "if x == 5";
        let config = LexerConfig {
            keywords: vec!["if".to_string()],
        };
        let hand_tokens = Lexer::new(source, Some(config)).tokenize().unwrap();
        let grammar = python_grammar();
        let grammar_tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();
        assert_eq!(hand_tokens.len(), grammar_tokens.len());
        for (i, (h, g)) in hand_tokens.iter().zip(grammar_tokens.iter()).enumerate() {
            assert_eq!(h.type_, g.type_, "token {} type mismatch", i);
            assert_eq!(h.value, g.value, "token {} value mismatch", i);
        }
    }

    #[test]
    fn test_consistency_with_strings() {
        use crate::tokenizer::Lexer;
        let source = r#"print("Hello\n")"#;
        let hand_tokens = Lexer::new(source, None).tokenize().unwrap();
        let grammar = python_grammar();
        let grammar_tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();
        assert_eq!(hand_tokens.len(), grammar_tokens.len());
        for (i, (h, g)) in hand_tokens.iter().zip(grammar_tokens.iter()).enumerate() {
            assert_eq!(h.type_, g.type_, "token {} type mismatch", i);
            assert_eq!(h.value, g.value, "token {} value mismatch", i);
        }
    }

    // -----------------------------------------------------------------------
    // Skip patterns
    // -----------------------------------------------------------------------

    #[test]
    fn test_skip_patterns() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nNUMBER = /[0-9]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/",
        ).unwrap();
        let tokens = GrammarLexer::new("x 42 # comment", &grammar).tokenize().unwrap();
        // Should see: NAME("x"), NUMBER("42"), EOF — comment and whitespace skipped
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].value, "x");
        assert_eq!(tokens[1].value, "42");
        assert_eq!(tokens[2].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Aliases
    // -----------------------------------------------------------------------

    #[test]
    fn test_alias_resolution() {
        let grammar = parse_token_grammar(
            r#"INT = /[0-9]+/ -> NUMBER
PLUS = "+""#,
        ).unwrap();
        let tokens = GrammarLexer::new("42 + 5", &grammar).tokenize().unwrap();
        // The token should be resolved to NUMBER type via the alias.
        assert_eq!(tokens[0].type_, TokenType::Number);
        assert_eq!(tokens[0].value, "42");
    }

    // -----------------------------------------------------------------------
    // Reserved keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_reserved_keyword_error() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nreserved:\n  class\n  import",
        ).unwrap();
        let result = GrammarLexer::new("class", &grammar).tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved keyword"));
    }

    #[test]
    fn test_reserved_keyword_allows_non_reserved() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nreserved:\n  class",
        ).unwrap();
        let tokens = GrammarLexer::new("foo", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "foo");
    }

    // -----------------------------------------------------------------------
    // String-based type names for custom tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_type_for_custom_tokens() {
        let grammar = parse_token_grammar(
            "IDENTIFIER = /[a-zA-Z_]+/\nINT = /[0-9]+/",
        ).unwrap();
        let tokens = GrammarLexer::new("foo 42", &grammar).tokenize().unwrap();
        // Custom types should have type_name set.
        assert_eq!(tokens[0].type_name, Some("IDENTIFIER".to_string()));
        assert_eq!(tokens[1].type_name, Some("INT".to_string()));
    }

    // -----------------------------------------------------------------------
    // Indentation mode
    // -----------------------------------------------------------------------

    #[test]
    fn test_indent_basic() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nCOLON = \":\"",
        ).unwrap();
        let source = "foo:\n    bar\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Expected: NAME("foo"), COLON, NEWLINE, INDENT, NAME("bar"), NEWLINE, DEDENT, EOF
        let types: Vec<TokenType> = tokens.iter().map(|t| t.type_).collect();
        assert!(types.contains(&TokenType::Indent));
        assert!(types.contains(&TokenType::Dedent));
    }

    #[test]
    fn test_indent_nested() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nCOLON = \":\"",
        ).unwrap();
        let source = "a:\n    b:\n        c\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Count INDENTs and DEDENTs.
        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.type_ == TokenType::Dedent).count();
        assert_eq!(indent_count, 2);
        assert_eq!(dedent_count, 2);
    }

    #[test]
    fn test_indent_blank_lines_skipped() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/",
        ).unwrap();
        let source = "a\n\n    \nb\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Blank lines should not produce NEWLINE tokens — just a, NEWLINE, b, NEWLINE, EOF
        let names: Vec<&str> = tokens.iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn test_indent_tab_rejected() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/",
        ).unwrap();
        let source = "a\n\tb\n";
        let result = GrammarLexer::new(source, &grammar).tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Tabs"));
    }

    #[test]
    fn test_indent_bracket_suppression() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nLPAREN = \"(\"\nRPAREN = \")\"",
        ).unwrap();
        let source = "a(\n    b\n)\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Inside brackets, no INDENT/DEDENT should be emitted.
        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        assert_eq!(indent_count, 0);
    }

    // -----------------------------------------------------------------------
    // Indentation mode with skip patterns
    // -----------------------------------------------------------------------

    #[test]
    fn test_indent_with_skip_patterns() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nCOLON = \":\"\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/",
        ).unwrap();
        let source = "foo:\n    # comment\n    bar\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        let names: Vec<&str> = tokens.iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["foo", "bar"]);
    }

    // ===================================================================
    // Helper: build a group grammar for testing (XML-like)
    // ===================================================================

    /// Create a grammar with pattern groups for testing.
    ///
    /// This simulates a simplified XML-like grammar:
    /// - Default group: TEXT and OPEN_TAG
    /// - tag group: TAG_NAME, EQUALS, VALUE, TAG_CLOSE
    ///
    /// The grammar uses skip patterns for whitespace and escape mode "none".
    fn make_group_grammar() -> TokenGrammar {
        parse_token_grammar(
            "escapes: none\n\n\
             skip:\n  WS = /[ \\t\\r\\n]+/\n\n\
             TEXT      = /[^<]+/\n\
             OPEN_TAG  = \"<\"\n\n\
             group tag:\n\
             \x20 TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/\n\
             \x20 EQUALS    = \"=\"\n\
             \x20 VALUE     = /\"[^\"]*\"/\n\
             \x20 TAG_CLOSE = \">\"\n",
        )
        .unwrap()
    }

    /// Normalize a token's type to a string name.
    ///
    /// Tokens can have a `type_name` (for grammar-driven custom types) or
    /// fall back to the `TokenType` enum variant name. This helper gives
    /// a consistent string for assertions.
    fn token_type_name(t: &Token) -> String {
        if let Some(ref name) = t.type_name {
            name.clone()
        } else {
            format!("{}", t.type_)
        }
    }

    // ===================================================================
    // LexerContext unit tests
    // ===================================================================

    #[test]
    fn test_ctx_push_group_records_action() {
        // push_group() records a Push action.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let mut ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        ctx.push_group("tag").unwrap();
        assert_eq!(ctx.actions, vec![ContextAction::Push("tag".to_string())]);
    }

    #[test]
    fn test_ctx_push_unknown_group_errors() {
        // push_group() with unknown name returns Err.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let mut ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        let result = ctx.push_group("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown pattern group"));
    }

    #[test]
    fn test_ctx_pop_group_records_action() {
        // pop_group() records a Pop action.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let mut ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        ctx.pop_group();
        assert_eq!(ctx.actions, vec![ContextAction::Pop]);
    }

    #[test]
    fn test_ctx_active_group_reads_stack() {
        // active_group() returns the top of the lexer's group stack.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        assert_eq!(ctx.active_group(), "default");
    }

    #[test]
    fn test_ctx_group_stack_depth() {
        // group_stack_depth() returns the length of the group stack.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        assert_eq!(ctx.group_stack_depth(), 1);
    }

    #[test]
    fn test_ctx_emit_appends_token() {
        // emit() records an Emit action with the given token.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let mut ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        let synthetic = Token {
            type_: TokenType::Name,
            value: "!".to_string(),
            line: 1,
            column: 1,
            type_name: Some("SYNTHETIC".to_string()),
            flags: None,
        };
        ctx.emit(synthetic.clone());
        assert_eq!(ctx.actions, vec![ContextAction::Emit(synthetic)]);
    }

    #[test]
    fn test_ctx_suppress_sets_flag() {
        // suppress() sets the suppressed flag.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let mut ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        assert!(!ctx.suppressed);
        ctx.suppress();
        assert!(ctx.suppressed);
    }

    #[test]
    fn test_ctx_peek_reads_source() {
        // peek() reads characters from the source after the token.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("hello", &grammar);
        // Suppose token ended at byte position 3 (consumed "hel")
        let ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "hello",
            pos_after_token: 3,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        assert_eq!(ctx.peek(1), "l");
        assert_eq!(ctx.peek(2), "o");
        assert_eq!(ctx.peek(3), "");  // past EOF
    }

    #[test]
    fn test_ctx_peek_str_reads_source() {
        // peek_str() reads a substring from the source after the token.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("hello world", &grammar);
        let ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "hello world",
            pos_after_token: 5,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        assert_eq!(ctx.peek_str(6), " world");
    }

    #[test]
    fn test_ctx_set_skip_enabled() {
        // set_skip_enabled() records a SetSkipEnabled action.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let mut ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        ctx.set_skip_enabled(false);
        assert_eq!(ctx.actions, vec![ContextAction::SetSkipEnabled(false)]);
    }

    #[test]
    fn test_ctx_multiple_pushes() {
        // Multiple push_group() calls are recorded in order.
        let grammar = make_group_grammar();
        let lexer = GrammarLexer::new("x", &grammar);
        let mut ctx = LexerContext {
            group_names: &lexer.group_names,
            group_stack: &lexer.group_stack,
            source: "x",
            pos_after_token: 1,
            actions: Vec::new(),
            suppressed: false,
            previous_token: None,
            bracket_depths: BracketDepths::default(),
            current_token_line: 1,
        };
        ctx.push_group("tag").unwrap();
        ctx.push_group("tag").unwrap();
        assert_eq!(ctx.actions, vec![
            ContextAction::Push("tag".to_string()),
            ContextAction::Push("tag".to_string()),
        ]);
    }

    // ===================================================================
    // Pattern group tokenization tests
    // ===================================================================

    #[test]
    fn test_no_callback_uses_default_group() {
        // Without a callback, only default group patterns are used.
        let grammar = make_group_grammar();
        let tokens = GrammarLexer::new("hello", &grammar).tokenize().unwrap();
        assert_eq!(token_type_name(&tokens[0]), "TEXT");
        assert_eq!(tokens[0].value, "hello");
    }

    #[test]
    fn test_callback_push_pop_group() {
        // Callback can push/pop groups to switch pattern sets.
        // Simulates: <div> where < triggers push("tag"), > triggers pop().
        let grammar = make_group_grammar();
        let mut lexer = GrammarLexer::new("<div>hello", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                ctx.push_group("tag").unwrap();
            } else if token.type_name.as_deref() == Some("TAG_CLOSE") {
                ctx.pop_group();
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        let types: Vec<(String, String)> = tokens.iter()
            .filter(|t| token_type_name(t) != "EOF")
            .map(|t| (token_type_name(t), t.value.clone()))
            .collect();
        assert_eq!(types, vec![
            ("OPEN_TAG".to_string(), "<".to_string()),
            ("TAG_NAME".to_string(), "div".to_string()),
            ("TAG_CLOSE".to_string(), ">".to_string()),
            ("TEXT".to_string(), "hello".to_string()),
        ]);
    }

    #[test]
    fn test_callback_with_attributes() {
        // Callback handles tag with attributes.
        // Simulates: <div class="main"> with the tag group.
        let grammar = make_group_grammar();
        let mut lexer = GrammarLexer::new("<div class=\"main\">", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                ctx.push_group("tag").unwrap();
            } else if token.type_name.as_deref() == Some("TAG_CLOSE") {
                ctx.pop_group();
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        let types: Vec<(String, String)> = tokens.iter()
            .filter(|t| token_type_name(t) != "EOF")
            .map(|t| (token_type_name(t), t.value.clone()))
            .collect();
        assert_eq!(types, vec![
            ("OPEN_TAG".to_string(), "<".to_string()),
            ("TAG_NAME".to_string(), "div".to_string()),
            ("TAG_NAME".to_string(), "class".to_string()),
            ("Equals".to_string(), "=".to_string()),
            ("VALUE".to_string(), "\"main\"".to_string()),
            ("TAG_CLOSE".to_string(), ">".to_string()),
        ]);
    }

    #[test]
    fn test_nested_tags() {
        // Group stack handles nested structures.
        // Simulates: <a>text<b>inner</b></a> with push/pop on < and >.
        let grammar = parse_token_grammar(
            "escapes: none\n\n\
             skip:\n  WS = /[ \\t\\r\\n]+/\n\n\
             TEXT             = /[^<]+/\n\
             CLOSE_TAG_START  = \"</\"\n\
             OPEN_TAG         = \"<\"\n\n\
             group tag:\n\
             \x20 TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/\n\
             \x20 TAG_CLOSE = \">\"\n\
             \x20 SLASH     = \"/\"\n",
        ).unwrap();

        let mut lexer = GrammarLexer::new("<a>text<b>inner</b></a>", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            let name = token.type_name.as_deref().unwrap_or("");
            if name == "OPEN_TAG" || name == "CLOSE_TAG_START" {
                ctx.push_group("tag").unwrap();
            } else if name == "TAG_CLOSE" {
                ctx.pop_group();
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        let types: Vec<(String, String)> = tokens.iter()
            .filter(|t| token_type_name(t) != "EOF")
            .map(|t| (token_type_name(t), t.value.clone()))
            .collect();
        assert_eq!(types, vec![
            ("OPEN_TAG".to_string(), "<".to_string()),
            ("TAG_NAME".to_string(), "a".to_string()),
            ("TAG_CLOSE".to_string(), ">".to_string()),
            ("TEXT".to_string(), "text".to_string()),
            ("OPEN_TAG".to_string(), "<".to_string()),
            ("TAG_NAME".to_string(), "b".to_string()),
            ("TAG_CLOSE".to_string(), ">".to_string()),
            ("TEXT".to_string(), "inner".to_string()),
            ("CLOSE_TAG_START".to_string(), "</".to_string()),
            ("TAG_NAME".to_string(), "b".to_string()),
            ("TAG_CLOSE".to_string(), ">".to_string()),
            ("CLOSE_TAG_START".to_string(), "</".to_string()),
            ("TAG_NAME".to_string(), "a".to_string()),
            ("TAG_CLOSE".to_string(), ">".to_string()),
        ]);
    }

    #[test]
    fn test_suppress_token() {
        // Callback can suppress tokens (remove from output).
        let grammar = make_group_grammar();
        let mut lexer = GrammarLexer::new("<hello", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                ctx.suppress();
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        let types: Vec<String> = tokens.iter()
            .filter(|t| token_type_name(t) != "EOF")
            .map(|t| token_type_name(t))
            .collect();
        // OPEN_TAG was suppressed, only TEXT remains
        assert_eq!(types, vec!["TEXT"]);
    }

    #[test]
    fn test_emit_synthetic_token() {
        // Callback can emit synthetic tokens after the current one.
        let grammar = make_group_grammar();
        let mut lexer = GrammarLexer::new("<hello", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                ctx.emit(Token {
                    type_: TokenType::Name,
                    value: "[start]".to_string(),
                    line: token.line,
                    column: token.column,
                    type_name: Some("MARKER".to_string()),
                    flags: None,
                });
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        let types: Vec<(String, String)> = tokens.iter()
            .filter(|t| token_type_name(t) != "EOF")
            .map(|t| (token_type_name(t), t.value.clone()))
            .collect();
        assert_eq!(types, vec![
            ("OPEN_TAG".to_string(), "<".to_string()),
            ("MARKER".to_string(), "[start]".to_string()),
            ("TEXT".to_string(), "hello".to_string()),
        ]);
    }

    #[test]
    fn test_suppress_and_emit() {
        // Suppress + emit = token replacement.
        let grammar = make_group_grammar();
        let mut lexer = GrammarLexer::new("<hello", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                ctx.suppress();
                ctx.emit(Token {
                    type_: TokenType::Name,
                    value: "<".to_string(),
                    line: token.line,
                    column: token.column,
                    type_name: Some("REPLACED".to_string()),
                    flags: None,
                });
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        let types: Vec<(String, String)> = tokens.iter()
            .filter(|t| token_type_name(t) != "EOF")
            .map(|t| (token_type_name(t), t.value.clone()))
            .collect();
        assert_eq!(types, vec![
            ("REPLACED".to_string(), "<".to_string()),
            ("TEXT".to_string(), "hello".to_string()),
        ]);
    }

    #[test]
    fn test_pop_at_bottom_is_noop() {
        // Popping when only default remains is a no-op (no crash).
        let grammar = make_group_grammar();
        let mut lexer = GrammarLexer::new("hello", &grammar);
        lexer.set_on_token(Some(Box::new(|_token: &Token, ctx: &mut LexerContext| {
            ctx.pop_group();
        })));
        let tokens = lexer.tokenize().unwrap();

        // Should still produce TEXT token without crashing.
        assert_eq!(token_type_name(&tokens[0]), "TEXT");
    }

    #[test]
    fn test_set_skip_enabled_false() {
        // Callback can disable skip patterns for significant whitespace.
        let grammar = parse_token_grammar(
            "escapes: none\n\n\
             skip:\n  WS = /[ \\t]+/\n\n\
             TEXT      = /[^<]+/\n\
             START     = \"<!\"\n\n\
             group raw:\n\
             \x20 RAW_TEXT = /[^>]+/\n\
             \x20 END      = \">\"\n",
        ).unwrap();

        let mut lexer = GrammarLexer::new("<! hello world >after", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            let name = token.type_name.as_deref().unwrap_or("");
            if name == "START" {
                ctx.push_group("raw").unwrap();
                ctx.set_skip_enabled(false);
            } else if name == "END" {
                ctx.pop_group();
                ctx.set_skip_enabled(true);
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        let types: Vec<(String, String)> = tokens.iter()
            .filter(|t| token_type_name(t) != "EOF")
            .map(|t| (token_type_name(t), t.value.clone()))
            .collect();
        assert_eq!(types, vec![
            ("START".to_string(), "<!".to_string()),
            ("RAW_TEXT".to_string(), " hello world ".to_string()),
            ("END".to_string(), ">".to_string()),
            ("TEXT".to_string(), "after".to_string()),
        ]);
    }

    #[test]
    fn test_no_groups_backward_compat() {
        // A grammar with no groups behaves identically to before.
        let grammar = parse_token_grammar(
            "NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/\n\
             NUMBER = /[0-9]+/\n\
             PLUS   = \"+\"\n",
        ).unwrap();
        let tokens = GrammarLexer::new("x + 1", &grammar).tokenize().unwrap();

        let types: Vec<(String, String)> = tokens.iter()
            .filter(|t| {
                let name = token_type_name(t);
                name != "Newline" && name != "EOF"
            })
            .map(|t| (token_type_name(t), t.value.clone()))
            .collect();
        assert_eq!(types, vec![
            ("Name".to_string(), "x".to_string()),
            ("Plus".to_string(), "+".to_string()),
            ("Number".to_string(), "1".to_string()),
        ]);
    }

    #[test]
    fn test_clear_callback() {
        // Passing None to set_on_token clears the callback.
        let grammar = make_group_grammar();
        use std::sync::{Arc, Mutex};
        let called = Arc::new(Mutex::new(Vec::<String>::new()));
        let called_clone = called.clone();

        let mut lexer = GrammarLexer::new("hello", &grammar);
        lexer.set_on_token(Some(Box::new(move |token: &Token, _ctx: &mut LexerContext| {
            called_clone.lock().unwrap().push(token_type_name(token));
        })));
        lexer.set_on_token(None);
        lexer.tokenize().unwrap();

        assert!(called.lock().unwrap().is_empty());
    }

    #[test]
    fn test_group_stack_resets_between_calls() {
        // The group stack resets when tokenize() is called again.
        // In Rust, we create a fresh lexer each time (GrammarLexer doesn't
        // support re-tokenizing the same source), but the reset logic in
        // tokenize_standard ensures clean state.
        let grammar = make_group_grammar();

        // First tokenization: push "tag" group
        let mut lexer1 = GrammarLexer::new("<div", &grammar);
        lexer1.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                ctx.push_group("tag").unwrap();
            }
        })));
        let tokens1 = lexer1.tokenize().unwrap();
        assert!(tokens1.iter().any(|t| token_type_name(t) == "TAG_NAME"));

        // Second tokenization: should start fresh from "default"
        let mut lexer2 = GrammarLexer::new("<div", &grammar);
        lexer2.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                ctx.push_group("tag").unwrap();
            }
        })));
        let tokens2 = lexer2.tokenize().unwrap();
        assert!(tokens2.iter().any(|t| token_type_name(t) == "TAG_NAME"));
    }

    #[test]
    fn test_multiple_push_pop_sequence() {
        // Multiple push/pop in one callback are applied in order.
        let grammar = make_group_grammar();

        let mut lexer = GrammarLexer::new("<div", &grammar);
        lexer.set_on_token(Some(Box::new(|token: &Token, ctx: &mut LexerContext| {
            if token.type_name.as_deref() == Some("OPEN_TAG") {
                // Push tag twice (stacking)
                ctx.push_group("tag").unwrap();
                ctx.push_group("tag").unwrap();
            }
        })));
        let tokens = lexer.tokenize().unwrap();

        // Should not crash and should still produce TAG_NAME.
        assert!(tokens.iter().any(|t| token_type_name(t) == "TAG_NAME"));
    }

    // -----------------------------------------------------------------------
    // Case-insensitive keyword support
    // -----------------------------------------------------------------------

    /// Build a small SQL-like grammar with case_insensitive mode enabled.
    ///
    /// The grammar declares `SELECT` and `FROM` as keywords. With
    /// `# @case_insensitive true`, the lexer stores them as uppercase
    /// internally and accepts any casing in source code.
    fn case_insensitive_grammar() -> TokenGrammar {
        // Note: indentation in skip: and keywords: sections MUST be preserved
        // in the source string. Rust's \n\ continuation strips leading whitespace,
        // so the indented content is written inline with explicit \n and spaces.
        parse_token_grammar(
            "# @case_insensitive true\nNAME = /[a-zA-Z_][a-zA-Z0-9_]*/\nskip:\n  WS = /[ \\t\\n]+/\nkeywords:\n  SELECT\n  FROM\n"
        )
        .unwrap()
    }

    fn layout_grammar() -> TokenGrammar {
        parse_token_grammar(
            "mode: layout\n\
             NAME = /[a-zA-Z_][a-zA-Z0-9_]*/\n\
             EQUALS = \"=\"\n\
             LBRACE = \"{\"\n\
             RBRACE = \"}\"\n\
             skip:\n\
             \x20 WS = /[ \\t]+/\n\
             layout_keywords:\n\
             \x20 let\n\
             \x20 where\n\
             \x20 do\n\
             \x20 of\n",
        )
        .unwrap()
    }

    #[test]
    fn test_layout_mode_injects_virtual_tokens() {
        let grammar = layout_grammar();
        let tokens = GrammarLexer::new("let\n  x = y\n  z = q\n", &grammar)
            .tokenize()
            .unwrap();

        let types: Vec<String> = tokens.iter().map(token_type_name).collect();
        assert_eq!(
            types,
            vec![
                "Name",
                "Newline",
                "VIRTUAL_LBRACE",
                "Name",
                "Equals",
                "Name",
                "Newline",
                "VIRTUAL_SEMICOLON",
                "Name",
                "Equals",
                "Name",
                "Newline",
                "VIRTUAL_RBRACE",
                "EOF",
            ]
        );
    }

    #[test]
    fn test_layout_mode_respects_explicit_braces() {
        let grammar = layout_grammar();
        let tokens = GrammarLexer::new("let {\n  x = y\n}\n", &grammar)
            .tokenize()
            .unwrap();

        let types: Vec<String> = tokens.iter().map(token_type_name).collect();
        assert!(!types.iter().any(|t| t == "VIRTUAL_LBRACE"));
        assert!(!types.iter().any(|t| t == "VIRTUAL_SEMICOLON"));
    }

    #[test]
    fn test_case_insensitive_lowercase_keyword() {
        // Lowercase "select" must be promoted to KEYWORD with value "SELECT".
        //
        // Truth table row: input="select", case_insensitive=true
        //   lookup key = "SELECT" (in keyword_set) → KEYWORD
        //   emitted value = "SELECT"               (normalized)
        let grammar = case_insensitive_grammar();
        let tokens = GrammarLexer::new("select", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword,
            "lowercase 'select' must be KEYWORD when case_insensitive=true");
        assert_eq!(tokens[0].value, "SELECT",
            "emitted value must be normalized to uppercase");
    }

    #[test]
    fn test_case_insensitive_uppercase_keyword() {
        // Uppercase "SELECT" must also be promoted and emitted as "SELECT".
        //
        // Truth table row: input="SELECT", case_insensitive=true
        //   lookup key = "SELECT" (in keyword_set) → KEYWORD
        //   emitted value = "SELECT"
        let grammar = case_insensitive_grammar();
        let tokens = GrammarLexer::new("SELECT", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword,
            "uppercase 'SELECT' must be KEYWORD when case_insensitive=true");
        assert_eq!(tokens[0].value, "SELECT",
            "emitted value must be normalized to uppercase");
    }

    #[test]
    fn test_case_insensitive_mixed_case_keyword() {
        // Mixed-case "Select" must also be promoted and emitted as "SELECT".
        //
        // Truth table row: input="Select", case_insensitive=true
        //   lookup key = "SELECT" (in keyword_set) → KEYWORD
        //   emitted value = "SELECT"
        let grammar = case_insensitive_grammar();
        let tokens = GrammarLexer::new("Select", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword,
            "mixed-case 'Select' must be KEYWORD when case_insensitive=true");
        assert_eq!(tokens[0].value, "SELECT",
            "emitted value must be normalized to uppercase");
    }

    #[test]
    fn test_case_sensitive_default_keeps_original_value() {
        // When case_insensitive=false (the default), keyword lookup is
        // case-sensitive. "select" is a keyword; its emitted value stays
        // "select" — no uppercasing is applied.
        //
        // Truth table row: input="select", case_insensitive=false
        //   lookup key = "select" (in keyword_set) → KEYWORD
        //   emitted value = "select"                (no normalization)
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_][a-zA-Z0-9_]*/\nskip:\n  WS = /[ \\t\\n]+/\nkeywords:\n  select\n",
        ).unwrap();
        let tokens = GrammarLexer::new("select", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Keyword,
            "'select' must be KEYWORD when it is in the keyword list");
        assert_eq!(tokens[0].value, "select",
            "case-sensitive mode must not alter the emitted value");
    }

    #[test]
    fn test_case_sensitive_uppercase_not_promoted() {
        // When case_insensitive=false, "SELECT" is NOT a keyword if only
        // "select" (lowercase) is in the keyword list.
        //
        // Truth table row: input="SELECT", case_insensitive=false, keyword="select"
        //   lookup key = "SELECT" (not in keyword_set) → NAME
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_][a-zA-Z0-9_]*/\nskip:\n  WS = /[ \\t\\n]+/\nkeywords:\n  select\n",
        ).unwrap();
        let tokens = GrammarLexer::new("SELECT", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name,
            "uppercase 'SELECT' must be NAME when grammar is case-sensitive and keyword is lowercase");
        assert_eq!(tokens[0].value, "SELECT",
            "NAME token value must preserve original casing");
    }

    #[test]
    fn test_case_insensitive_non_keyword_identifier_preserves_case() {
        // A non-keyword identifier must be emitted as NAME with its original
        // case, even when case_insensitive=true. Only keywords are normalized.
        //
        // Truth table row: input="myTable", case_insensitive=true, keyword="SELECT"
        //   lookup key = "MYTABLE" (not in keyword_set) → NAME
        //   emitted value = "myTable"                    (no normalization)
        let grammar = case_insensitive_grammar();
        let tokens = GrammarLexer::new("myTable", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name,
            "non-keyword identifier must be NAME even in case_insensitive mode");
        assert_eq!(tokens[0].value, "myTable",
            "NAME tokens must retain their original casing");
    }

    #[test]
    fn test_case_insensitive_multiple_keywords_in_sequence() {
        // A realistic SQL-like snippet: "select id from users" should produce
        // KEYWORD("SELECT"), NAME("id"), KEYWORD("FROM"), NAME("users").
        let grammar = case_insensitive_grammar();
        let tokens = GrammarLexer::new("select id from users", &grammar).tokenize().unwrap();

        let pairs: Vec<(TokenType, &str)> = tokens.iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect();

        assert_eq!(pairs, vec![
            (TokenType::Keyword, "SELECT"),
            (TokenType::Name,    "id"),
            (TokenType::Keyword, "FROM"),
            (TokenType::Name,    "users"),
        ]);
    }
}
