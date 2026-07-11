//! # Grammar-driven parser — parsing any language from a `.grammar` file.
//!
//! The hand-written parser in [`crate::parser`] can only parse one language:
//! our Python subset. If we wanted to parse JavaScript, Ruby, or any other
//! language, we would need to write another parser from scratch.
//!
//! This module takes a different approach: **grammar-driven parsing**. Instead
//! of hard-coding grammar rules as Rust functions, we read the rules from a
//! `.grammar` file (parsed by the `grammar_tools` crate) and use them to
//! drive the parse at runtime.
//!
//! # Extensions for Starlark-like languages
//!
//! This parser supports several extensions beyond basic EBNF interpretation:
//!
//! - **Packrat memoization**: Caches parse results for each (rule, position) pair,
//!   avoiding exponential backtracking. Essential for grammars with ~40 rules.
//!
//! - **Significant newlines**: If the grammar references NEWLINE tokens, they are
//!   treated as significant (not auto-skipped). Otherwise, NEWLINEs are transparent.
//!
//! - **Furthest failure tracking**: When parsing fails, the error message reports
//!   what was expected at the furthest position reached, not just the first failure.
//!
//! - **String-based token matching**: Tokens with a `type_name` field are matched
//!   by their string name, allowing grammars with custom token types beyond the
//!   fixed `TokenType` enum.
//!
//! # How it works
//!
//! 1. A `.grammar` file defines the language's syntax in EBNF notation.
//! 2. The `grammar_tools` crate parses this file into a `ParserGrammar`.
//! 3. This module's `GrammarParser` walks the grammar rule tree while
//!    consuming tokens. Each EBNF element type has a natural interpretation:
//!
//!    | Element       | Strategy                                    |
//!    |---------------|---------------------------------------------|
//!    | Sequence      | Match all children in order (AND)           |
//!    | Alternation   | Try each choice until one matches (OR)      |
//!    | Repetition    | Match zero or more times (loop)             |
//!    | Optional      | Match zero or one time                      |
//!    | Group         | Delegate to inner element                   |
//!    | RuleReference | Recursively parse the named rule            |
//!    | TokenReference| Match if current token has the right type   |
//!    | Literal       | Match if current token has the right value  |

use lexer::token::{Token, TokenType, string_to_token_type};
use grammar_tools::parser_grammar::{GrammarElement, ParserGrammar, GrammarRule};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// AST types for grammar-driven parsing
// ===========================================================================

/// A child of a grammar AST node — either a nested node or a raw token.
#[derive(Debug, Clone, PartialEq)]
pub enum ASTNodeOrToken {
    /// A nested AST node produced by matching a grammar rule.
    Node(GrammarASTNode),
    /// A raw token that was matched directly (token reference or literal).
    Token(Token),
}

/// A node in the grammar-driven AST.
///
/// Each node corresponds to a successfully matched grammar rule. The
/// `rule_name` says which rule matched, and `children` contains the
/// sub-matches (either deeper rule matches or individual tokens).
#[derive(Debug, Clone, PartialEq)]
pub struct GrammarASTNode {
    pub rule_name: String,
    pub children: Vec<ASTNodeOrToken>,
    /// The 1-based line number where this node's first token appears.
    pub start_line: Option<usize>,
    /// The 1-based column number where this node's first token appears.
    pub start_column: Option<usize>,
    /// The 1-based line number where this node's last token appears.
    pub end_line: Option<usize>,
    /// The 1-based column number where this node's last token appears.
    pub end_column: Option<usize>,
}

impl GrammarASTNode {
    /// Check if this node is a "leaf" — a node with exactly one child that
    /// is a raw token.
    pub fn is_leaf(&self) -> bool {
        if self.children.len() == 1 {
            matches!(&self.children[0], ASTNodeOrToken::Token(_))
        } else {
            false
        }
    }

    /// If this is a leaf node, return a reference to its token.
    pub fn token(&self) -> Option<&Token> {
        if self.is_leaf() {
            match &self.children[0] {
                ASTNodeOrToken::Token(tok) => Some(tok),
                _ => None,
            }
        } else {
            None
        }
    }
}

// ===========================================================================
// Error type
// ===========================================================================

/// An error encountered during grammar-driven parsing.
#[derive(Debug, Clone)]
pub struct GrammarParseError {
    pub message: String,
    pub token: Token,
}

impl fmt::Display for GrammarParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.token.line, self.token.column, self.message
        )
    }
}

impl std::error::Error for GrammarParseError {}

// ===========================================================================
// Memo entry — packrat memoization cache entry
// ===========================================================================

/// A cached result from parsing a rule at a specific position.
///
/// Packrat memoization stores the result of every (rule, position) attempt
/// so that re-parsing the same rule at the same position is O(1). This is
/// essential for grammars with ~40 rules that would otherwise cause
/// exponential backtracking.
struct MemoEntry {
    /// The matched children, or None if the rule failed.
    children: Option<Vec<ASTNodeOrToken>>,
    /// The position after the match (or where we gave up).
    end_pos: usize,
    /// Whether the match succeeded.
    ok: bool,
}

// ===========================================================================
// Grammar parser
// ===========================================================================

/// A parser that uses a `ParserGrammar` (from a `.grammar` file) to parse
/// a token stream into a generic AST.
///
/// Includes packrat memoization, significant newline detection, and
/// furthest failure tracking for better error messages.
pub struct GrammarParser {
    tokens: Vec<Token>,
    grammar: ParserGrammar,
    pos: usize,
    rules: HashMap<String, GrammarRule>,

    /// Index of each rule name for memo key generation.
    rule_index: HashMap<String, usize>,

    /// Whether newlines are significant in this grammar.
    newlines_significant: bool,

    /// Packrat memoization cache: "rule_idx,pos" -> MemoEntry.
    memo: HashMap<String, MemoEntry>,

    /// Furthest position reached during parsing.
    furthest_pos: usize,

    /// What was expected at the furthest position.
    furthest_expected: Vec<String>,
    /// Set of (rule_index, pos) pairs currently being parsed.
    /// Used to detect and break left recursion: if we try to parse a rule
    /// at a position where we're already inside that same rule (but haven't
    /// cached the result yet), we know it's left recursion and should fail.
    in_progress: std::collections::HashSet<String>,

    /// Pre-parse hooks: transform token list before parsing.
    /// Each hook is a function `Vec<Token> -> Vec<Token>`. Multiple hooks compose left-to-right.
    // Boxed-closure hook list; the type documents the hook signature inline.
    #[allow(clippy::type_complexity)]
    pre_parse_hooks: Vec<Box<dyn Fn(Vec<Token>) -> Vec<Token>>>,

    /// Post-parse hooks: transform AST after parsing.
    /// Each hook is a function `GrammarASTNode -> GrammarASTNode`. Multiple hooks compose left-to-right.
    post_parse_hooks: Vec<Box<dyn Fn(GrammarASTNode) -> GrammarASTNode>>,

    /// When true, emit a `[TRACE]` line to stderr for every rule attempt.
    ///
    /// Trace mode is invaluable when debugging why a grammar does not match
    /// a particular input. Instead of reading the grammar rules and mentally
    /// simulating the parse, you can see exactly which rule was attempted at
    /// which token position and whether it succeeded or failed.
    ///
    /// Example output:
    /// ```text
    /// [TRACE] rule 'expression' at token 0 (Name "x") → match
    /// [TRACE] rule 'term' at token 0 (Name "x") → match
    /// [TRACE] rule 'factor' at token 2 (Plus "+") → fail
    /// ```
    trace: bool,

    /// Current recursion depth of the recursive-descent parse.
    ///
    /// Incremented on entry to [`Self::parse_rule`] and decremented on exit.
    /// Every nested rule reference deepens this counter by one. It exists
    /// purely to bound the *native* call stack: the left-recursion guard
    /// (`in_progress`) bounds left recursion, but does nothing for deep
    /// *right* recursion / nesting like `((((…))))` or `[[[…]]]`, where each
    /// extra layer of brackets is a fresh `(rule, pos)` pair the memo never
    /// short-circuits. Such input recurses once per layer and overflows the
    /// native stack — an *uncatchable* process abort.
    depth: usize,

    /// Maximum permitted recursion depth before [`Self::parse_rule`] bails
    /// out with a recoverable failure instead of recursing deeper.
    ///
    /// See [`DEFAULT_MAX_RULE_DEPTH`] for the rationale behind the value.
    max_depth: usize,

    /// Sticky flag set the first time the depth cap is hit. `parse()` reads
    /// it to surface a clear "input nests deeper than the supported limit"
    /// [`GrammarParseError`] instead of the generic furthest-failure message,
    /// so callers can tell a genuine syntax error apart from a depth refusal.
    depth_exceeded: bool,
}

/// Default recursion-depth cap for the grammar-driven parser.
///
/// # Why a cap at all
///
/// `parse_rule` recurses through `match_element` back into `parse_rule` for
/// every nested rule reference. Deeply-nested input — `((((…))))`, `[[[…]]]`,
/// or any deep right-recursive rule — therefore recurses once per nesting
/// level. Past a few hundred levels this overflows the *native* thread stack,
/// which on most platforms is an **uncatchable** `SIGSEGV` / stack-overflow
/// abort: it kills the whole process, so a `Result`-returning entry point
/// like [`GrammarParser::parse`] cannot report it. Every SIR frontend
/// (twig / ruby / python / javascript) reaches this parser through its public
/// entry, so a ~300-deep nested literal would crash the host *before* the
/// frontend's own source-level depth checks could fire.
///
/// # Why 128
///
/// This cap has to satisfy two opposing constraints, and 128 threads the
/// needle:
///
/// * **Below the native-overflow point.** Each level of `group`-style nesting
///   pushes several `parse_rule` / `match_element` frames, and those frames
///   are large (token clones, `format!` memo keys, position computation). On a
///   default ~2 MiB thread stack (the stack a default-spawned worker — and the
///   Rust test runner — gets), this implementation overflows somewhere between
///   depth ~192 and ~224 in a debug build; release frames are smaller, so the
///   overflow point only rises. A cap of 128 trips the clean error with a
///   comfortable margin *below* the worst-case (debug) overflow, on the
///   default stack, with no special stack sizing required by callers. This was
///   pinned empirically by binary-searching the cap that still returns `Err`
///   instead of overflowing on a default-stack worker thread.
///
/// * **Above any real program's nesting — for JS-shaped grammars.** In a
///   grammar whose expression rule-chain is shallow (like ECMAScript), each
///   source-nesting level costs only a few `parse_rule` frames, so 128 frames
///   is dozens of source levels — far beyond hand-written JS, which virtually
///   never nests grouping even a few dozen deep. This is why closurec opts into
///   this value: real JS parses identically and the guard fires *only* on
///   pathological, DoS-shaped input.
///
/// # Why this is NOT a safe *global default*
///
/// Rule-chain depth ≠ source-nesting depth. A rich grammar (e.g. Wolfram)
/// spends *dozens* of rule-frames per source-nesting level, so 128 frames is
/// only a handful of real brackets — far too few for legitimate *moderate*
/// nesting. And frontends that already guard themselves on an enlarged stack
/// (python-to-semantic-ir / javascript-to-semantic-ir) rely on their own
/// *lowerer's* depth check firing, which a parser cap would preempt. That is
/// why [`GrammarParser::new`] defaults to *unlimited* and the guard is opt-in
/// per caller — a single global cap cannot be both DoS-safe on the heaviest
/// grammar's default stack *and* generous enough for every grammar's real
/// input.
pub const DEFAULT_MAX_RULE_DEPTH: usize = 128;

impl GrammarParser {
    /// Create a new grammar-driven parser (trace disabled).
    pub fn new(tokens: Vec<Token>, grammar: ParserGrammar) -> Self {
        Self::new_with_trace(tokens, grammar, false)
    }

    /// Create a new grammar-driven parser with optional trace mode.
    ///
    /// When `trace` is `true`, every rule attempt emits a `[TRACE]` line to
    /// stderr showing the rule name, the token position, the current token
    /// type and value, and whether the rule matched or failed.
    ///
    /// This is intended for debugging grammar issues. Keep it off in
    /// production because the output can be voluminous.
    ///
    /// # Format
    ///
    /// ```text
    /// [TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match
    /// [TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → fail
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// use parser::grammar_parser::GrammarParser;
    /// use grammar_tools::parser_grammar::parse_parser_grammar;
    /// use lexer::token::{Token, TokenType};
    ///
    /// let grammar = parse_parser_grammar("value = NUMBER ;").unwrap();
    /// let tokens = vec![
    ///     Token { cv: None, type_: TokenType::Number, value: "42".into(), line: 1, column: 1, type_name: None, flags: None },
    ///     Token { cv: None, type_: TokenType::Eof,    value: "".into(),   line: 1, column: 3, type_name: None, flags: None },
    /// ];
    /// let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
    /// let result = parser.parse();
    /// assert!(result.is_ok());
    /// ```
    pub fn new_with_trace(tokens: Vec<Token>, grammar: ParserGrammar, trace: bool) -> Self {
        let mut rules = HashMap::new();
        let mut rule_index = HashMap::new();

        for (i, rule) in grammar.rules.iter().enumerate() {
            rules.insert(rule.name.clone(), rule.clone());
            rule_index.insert(rule.name.clone(), i);
        }

        let newlines_significant = grammar_references_newline(&grammar);

        GrammarParser {
            tokens,
            grammar,
            pos: 0,
            rules,
            rule_index,
            newlines_significant,
            memo: HashMap::new(),
            furthest_pos: 0,
            furthest_expected: Vec::new(),
            in_progress: std::collections::HashSet::new(),
            pre_parse_hooks: Vec::new(),
            post_parse_hooks: Vec::new(),
            trace,
            depth: 0,
            // The recursion-depth guard is OPT-IN: `new()` defaults to
            // *unlimited*. A single global default cap is unsound because
            // rule-chain depth ≠ source-nesting depth — a rich grammar (e.g.
            // Wolfram) spends dozens of rule-frames per bracket, so any cap low
            // enough to sit below the native-stack overflow point on the
            // default stack (~200 frames) would reject legitimate *moderate*
            // nesting (40 parens ≈ 1280 frames), and would also preempt
            // frontends that already guard themselves on an enlarged stack
            // (python-to-semantic-ir / javascript-to-semantic-ir run the parse
            // on a big-stack worker and rely on their *lowerer's* own depth
            // check firing). Callers that parse untrusted input on the default
            // stack and want a DoS backstop opt in explicitly with
            // `.with_max_depth(DEFAULT_MAX_RULE_DEPTH)`.
            max_depth: usize::MAX,
            depth_exceeded: false,
        }
    }

    /// Override the recursion-depth cap (default [`DEFAULT_MAX_RULE_DEPTH`]).
    ///
    /// Lowering it makes the guard easier to exercise in tests without
    /// building pathologically deep input; raising it is rarely needed since
    /// the default already sits far above any real program's nesting and
    /// below the native-stack overflow point. Returns `self` for chaining.
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Whether newlines are treated as significant tokens in this grammar.
    pub fn is_newlines_significant(&self) -> bool {
        self.newlines_significant
    }

    /// Register a token transform to run before parsing.
    ///
    /// The hook receives the token list and returns a (possibly modified)
    /// token list. Multiple hooks compose left-to-right.
    pub fn add_pre_parse(&mut self, hook: Box<dyn Fn(Vec<Token>) -> Vec<Token>>) {
        self.pre_parse_hooks.push(hook);
    }

    /// Register an AST transform to run after parsing.
    ///
    /// The hook receives the parsed AST root and returns a (possibly modified)
    /// AST. Multiple hooks compose left-to-right.
    pub fn add_post_parse(&mut self, hook: Box<dyn Fn(GrammarASTNode) -> GrammarASTNode>) {
        self.post_parse_hooks.push(hook);
    }

    /// Get the current token without consuming it.
    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    /// Record a failed expectation at the current position for error reporting.
    fn record_failure(&mut self, expected: &str) {
        if self.pos > self.furthest_pos {
            self.furthest_pos = self.pos;
            self.furthest_expected = vec![expected.to_string()];
        } else if self.pos == self.furthest_pos
            && !self.furthest_expected.contains(&expected.to_string()) {
                self.furthest_expected.push(expected.to_string());
            }
    }

    /// Parse the token stream according to the grammar.
    ///
    /// Uses the first rule in the grammar as the entry point (start symbol).
    pub fn parse(&mut self) -> Result<GrammarASTNode, GrammarParseError> {
        // Pre-parse hooks: transform the token list before parsing begins.
        if !self.pre_parse_hooks.is_empty() {
            let mut tokens = std::mem::take(&mut self.tokens);
            for hook in &self.pre_parse_hooks {
                tokens = hook(tokens);
            }
            self.tokens = tokens;
        }

        if self.grammar.rules.is_empty() {
            return Err(GrammarParseError {
                message: "Grammar has no rules".to_string(),
                token: self.current().clone(),
            });
        }

        let entry_rule_name = self.grammar.rules[0].name.clone();
        let result = self.parse_rule(&entry_rule_name);

        match result {
            None => {
                let tok = self.current().clone();
                // A depth-cap refusal takes priority over the generic
                // furthest-failure message: the parse did not fail because the
                // input was syntactically wrong, but because it nested deeper
                // than we are willing to recurse. Surface that explicitly so
                // callers (and the SIR frontends) can distinguish a DoS-shaped
                // input from an ordinary syntax error.
                if self.depth_exceeded {
                    return Err(GrammarParseError {
                        message: format!(
                            "input nests deeper than the supported limit ({})",
                            self.max_depth
                        ),
                        token: tok,
                    });
                }
                if !self.furthest_expected.is_empty() {
                    let expected = self.furthest_expected.join(" or ");
                    let furthest_tok = if self.furthest_pos < self.tokens.len() {
                        self.tokens[self.furthest_pos].clone()
                    } else {
                        tok.clone()
                    };
                    Err(GrammarParseError {
                        message: format!(
                            "Expected {}, got {:?}",
                            expected, furthest_tok.value
                        ),
                        token: furthest_tok,
                    })
                } else {
                    Err(GrammarParseError {
                        message: "Failed to parse using grammar".to_string(),
                        token: tok,
                    })
                }
            }
            Some(node) => {
                // Skip trailing newlines.
                while self.pos < self.tokens.len()
                    && self.current().type_ == TokenType::Newline
                {
                    self.pos += 1;
                }

                // Check that we consumed all tokens.
                if self.pos < self.tokens.len()
                    && self.current().type_ != TokenType::Eof
                {
                    let tok = self.current().clone();
                    if !self.furthest_expected.is_empty() && self.furthest_pos > self.pos {
                        let expected = self.furthest_expected.join(" or ");
                        let furthest_tok = if self.furthest_pos < self.tokens.len() {
                            self.tokens[self.furthest_pos].clone()
                        } else {
                            tok.clone()
                        };
                        return Err(GrammarParseError {
                            message: format!(
                                "Expected {}, got {:?}",
                                expected, furthest_tok.value
                            ),
                            token: furthest_tok,
                        });
                    }
                    return Err(GrammarParseError {
                        message: format!(
                            "Unexpected token: {:?}",
                            tok.value
                        ),
                        token: tok,
                    });
                }

                // Post-parse hooks: transform the AST after parsing completes.
                let mut result = node;
                for hook in &self.post_parse_hooks {
                    result = hook(result);
                }

                Ok(result)
            }
        }
    }

    // =========================================================================
    // Rule parsing (with packrat memoization)
    // =========================================================================

    /// Try to match a named grammar rule.
    ///
    /// This is a thin depth-guarding wrapper around [`Self::parse_rule_inner`],
    /// which holds the actual memoization + left-recursion logic. Every entry
    /// into a (sub-)rule passes through here, so incrementing `depth` on the
    /// way in and decrementing on the way out gives an exact running count of
    /// the *native* recursion depth — regardless of which of the inner
    /// function's several early-return paths is taken.
    ///
    /// If we are already at the cap, we refuse to recurse one level deeper:
    /// we set the sticky `depth_exceeded` flag, record a failure for error
    /// reporting, and return `None`. Returning `None` (a normal parse failure)
    /// rather than panicking keeps the whole thing recoverable — it unwinds
    /// back through the recursive-descent stack exactly like an ordinary
    /// no-match, and `parse()` turns it into a clean `GrammarParseError`.
    fn parse_rule(&mut self, rule_name: &str) -> Option<GrammarASTNode> {
        if self.depth >= self.max_depth {
            // Refuse to descend further. Mark the refusal so `parse()` can
            // emit a precise message, and record a failure at the current
            // position so any surrounding furthest-failure logic stays sane.
            self.depth_exceeded = true;
            self.record_failure("input within the supported nesting limit");
            return None;
        }

        self.depth += 1;
        let result = self.parse_rule_inner(rule_name);
        self.depth -= 1;
        result
    }

    /// Try to match a named grammar rule with memoization. The depth guard
    /// lives in the [`Self::parse_rule`] wrapper; do not call this directly.
    fn parse_rule_inner(&mut self, rule_name: &str) -> Option<GrammarASTNode> {
        let rule = {
            let r = self.rules.get(rule_name)?;
            r.clone()
        };

        // Check memo cache.
        if let Some(&idx) = self.rule_index.get(rule_name) {
            let key = format!("{},{}", idx, self.pos);
            if let Some(entry) = self.memo.get(&key) {
                let end_pos = entry.end_pos;
                let ok = entry.ok;
                let children = entry.children.clone();
                self.pos = end_pos;
                if !ok {
                    return None;
                }
                let c = children.unwrap();
                let (sl, sc, el, ec) = compute_node_position(&c);
                return Some(GrammarASTNode {
                    rule_name: rule_name.to_string(),
                    children: c,
                    start_line: sl,
                    start_column: sc,
                    end_line: el,
                    end_column: ec,
                });
            }

            // Left-recursion guard: if we're already trying to parse this
            // rule at this position (but haven't finished and cached the
            // result yet), then we've hit left recursion. Return None to
            // break the cycle. This handles grammars with rules like:
            //   primary = ... | primary LBRACKET expression RBRACKET
            // where `primary` appears as the first element of an alternative.
            if !self.in_progress.insert(key.clone()) {
                // key was already present — left recursion detected
                return None;
            }
        }

        let start_pos = self.pos;

        // Capture trace info BEFORE mutating self.pos via match_element.
        // We snapshot the token at start_pos so the trace line shows the
        // token the rule is attempting to match at the moment of the attempt.
        let trace_token_info = if self.trace {
            let tok = if start_pos < self.tokens.len() {
                &self.tokens[start_pos]
            } else {
                &self.tokens[self.tokens.len() - 1]
            };
            // Prefer the string type_name (grammar-driven tokens like "IDENT",
            // "NUMBER") over the enum variant name for readability.
            let type_label = if let Some(ref tn) = tok.type_name {
                tn.clone()
            } else {
                format!("{}", tok.type_)
            };
            Some((start_pos, type_label, tok.value.clone()))
        } else {
            None
        };

        let children = self.match_element(&rule.body);

        // Emit [TRACE] line to stderr now that we know success/failure.
        // The arrow character → (U+2192) mirrors the task spec exactly.
        if let Some((idx, type_label, value)) = trace_token_info {
            let outcome = if children.is_some() { "match" } else { "fail" };
            eprintln!(
                "[TRACE] rule '{}' at token {} ({} \"{}\") \u{2192} {}",
                rule_name, idx, type_label, value, outcome
            );
        }

        // Cache result and remove from in_progress set.
        if let Some(&idx) = self.rule_index.get(rule_name) {
            let key = format!("{},{}", idx, start_pos);
            self.in_progress.remove(&key);
            if let Some(ref result) = children {
                self.memo.insert(key, MemoEntry {
                    children: Some(result.clone()),
                    end_pos: self.pos,
                    ok: true,
                });
            } else {
                self.memo.insert(key, MemoEntry {
                    children: None,
                    end_pos: self.pos,
                    ok: false,
                });
            }
        }

        match children {
            Some(c) => {
                let (sl, sc, el, ec) = compute_node_position(&c);
                Some(GrammarASTNode {
                    rule_name: rule_name.to_string(),
                    children: c,
                    start_line: sl,
                    start_column: sc,
                    end_line: el,
                    end_column: ec,
                })
            }
            None => {
                self.pos = start_pos;
                self.record_failure(rule_name);
                None
            }
        }
    }

    // =========================================================================
    // Element matching
    // =========================================================================

    fn match_element(&mut self, element: &GrammarElement) -> Option<Vec<ASTNodeOrToken>> {
        let save_pos = self.pos;

        match element {
            GrammarElement::Sequence { elements } => {
                let mut children = Vec::new();
                for sub in elements {
                    match self.match_element(sub) {
                        Some(mut result) => children.append(&mut result),
                        None => {
                            self.pos = save_pos;
                            return None;
                        }
                    }
                }
                Some(children)
            }

            GrammarElement::Alternation { choices } => {
                for choice in choices {
                    self.pos = save_pos;
                    if let Some(result) = self.match_element(choice) {
                        return Some(result);
                    }
                }
                self.pos = save_pos;
                None
            }

            GrammarElement::Repetition { element: inner } => {
                let mut children = Vec::new();
                loop {
                    let save_rep = self.pos;
                    match self.match_element(inner) {
                        Some(mut result) => children.append(&mut result),
                        None => {
                            self.pos = save_rep;
                            break;
                        }
                    }
                }
                Some(children)
            }

            GrammarElement::Optional { element: inner } => {
                match self.match_element(inner) {
                    Some(result) => Some(result),
                    None => Some(Vec::new()),
                }
            }

            GrammarElement::Group { element: inner } => {
                self.match_element(inner)
            }

            GrammarElement::RuleReference { name } => {
                // Is this an uppercase token reference?
                let is_token = name.chars().all(|c| c.is_uppercase() || c == '_');

                if is_token {
                    self.match_token_reference(name)
                } else {
                    match self.parse_rule(name) {
                        Some(node) => Some(vec![ASTNodeOrToken::Node(node)]),
                        None => {
                            self.pos = save_pos;
                            None
                        }
                    }
                }
            }

            GrammarElement::TokenReference { name } => {
                self.match_token_reference(name)
            }

            GrammarElement::Literal { value } => {
                // Skip insignificant newlines before literal matching.
                if !self.newlines_significant {
                    while self.current().type_ == TokenType::Newline {
                        self.pos += 1;
                    }
                }

                if self.current().value == *value {
                    let tok = self.current().clone();
                    self.pos += 1;
                    Some(vec![ASTNodeOrToken::Token(tok)])
                } else {
                    self.record_failure(&format!("\"{}\"", value));
                    None
                }
            }

            // ---------------------------------------------------------------
            // Extension: Syntactic predicates (lookahead without consuming)
            // ---------------------------------------------------------------

            GrammarElement::PositiveLookahead { element: inner } => {
                // Succeed if inner element matches, but consume no input.
                let result = self.match_element(inner);
                self.pos = save_pos;
                if result.is_some() { Some(Vec::new()) } else { None }
            }

            GrammarElement::NegativeLookahead { element: inner } => {
                // Succeed if inner element does NOT match, consume no input.
                let result = self.match_element(inner);
                self.pos = save_pos;
                if result.is_none() { Some(Vec::new()) } else { None }
            }

            // ---------------------------------------------------------------
            // Extension: One-or-more repetition
            // ---------------------------------------------------------------

            GrammarElement::OneOrMore { element: inner } => {
                // Match one required, then zero or more additional.
                let first = self.match_element(inner);
                match first {
                    None => {
                        self.pos = save_pos;
                        None
                    }
                    Some(mut children) => {
                        loop {
                            let save_rep = self.pos;
                            match self.match_element(inner) {
                                Some(mut result) => children.append(&mut result),
                                None => {
                                    self.pos = save_rep;
                                    break;
                                }
                            }
                        }
                        Some(children)
                    }
                }
            }

            // ---------------------------------------------------------------
            // Extension: Separated repetition
            // ---------------------------------------------------------------

            GrammarElement::SeparatedRepetition { element: inner, separator, at_least_one } => {
                // Match: element { separator element }
                // Or with at_least_one=false: [ element { separator element } ]
                let first = self.match_element(inner);
                match first {
                    None => {
                        self.pos = save_pos;
                        if *at_least_one { None } else { Some(Vec::new()) }
                    }
                    Some(mut children) => {
                        loop {
                            let save_sep = self.pos;
                            let sep = self.match_element(separator);
                            match sep {
                                None => {
                                    self.pos = save_sep;
                                    break;
                                }
                                Some(mut sep_children) => {
                                    let next = self.match_element(inner);
                                    match next {
                                        None => {
                                            self.pos = save_sep;
                                            break;
                                        }
                                        Some(mut next_children) => {
                                            children.append(&mut sep_children);
                                            children.append(&mut next_children);
                                        }
                                    }
                                }
                            }
                        }
                        Some(children)
                    }
                }
            }
        }
    }

    // =========================================================================
    // Token reference matching
    // =========================================================================

    /// Match a token reference, handling string-based type names and
    /// newline skipping.
    fn match_token_reference(&mut self, expected_type: &str) -> Option<Vec<ASTNodeOrToken>> {
        // Skip newlines when matching non-NEWLINE tokens (if insignificant).
        if !self.newlines_significant && expected_type != "NEWLINE" {
            while self.current().type_ == TokenType::Newline {
                self.pos += 1;
            }
        }

        let token = self.current();

        // First, check string-based type_name for custom token types.
        if let Some(ref type_name) = token.type_name {
            if type_name == expected_type {
                let tok = token.clone();
                self.pos += 1;
                return Some(vec![ASTNodeOrToken::Token(tok)]);
            }
        }

        // Fall back to enum-based matching.
        let expected = string_to_token_type(expected_type);

        // If the expected type maps to `Name`, the token grammar's
        // string-based `type_name` is the source of truth — the legacy
        // `type_: Name` enum value is only a fallback for builtin
        // identifiers that don't carry a custom `type_name`.  Two cases:
        //
        //   - `expected_type == "NAME"`: the grammar wants a *bare*
        //     name token (no custom type_name).  Reject tokens whose
        //     `type_name` is set — e.g. a QUOTE token (whose type_ is
        //     Name only because string_to_token_type("QUOTE") falls
        //     back to Name) must not satisfy a NAME reference.
        //
        //   - `expected_type` is some custom Name-based type
        //     (`AT_KEYWORD`, `VARIABLE`, `IDENT`, …): the type_name
        //     check above has already covered the success case, so if
        //     we got here, this token's type_name doesn't match — even
        //     a `type_name: None` bare-Name token shouldn't be
        //     coerced into a custom type, because that would let
        //     unrelated grammars cross-pollute (e.g. an IDENT match
        //     accepting a VARIABLE token).
        //
        // The original behaviour allowed bare-Name tokens to match any
        // custom Name-based type, which was a footgun: it caused
        // languages with both NAME and a sibling Name-typed token (like
        // Twig's `QUOTE = "'"`) to misclassify atoms.  See twig-parser
        // tests for the regression that motivated this tightening.
        if expected == TokenType::Name {
            if expected_type == "NAME" {
                if token.type_name.is_some() {
                    self.record_failure(expected_type);
                    return None;
                }
            } else {
                // Custom Name-based reference; the type_name check at
                // the top of this function is the only path to success.
                self.record_failure(expected_type);
                return None;
            }
        }

        if token.type_ == expected {
            let tok = token.clone();
            self.pos += 1;
            Some(vec![ASTNodeOrToken::Token(tok)])
        } else {
            self.record_failure(expected_type);
            None
        }
    }
}

// ===========================================================================
// AST position computation
// ===========================================================================

/// Compute position info for a GrammarASTNode from its children.
///
/// Walks the children to find the first and last leaf tokens, then uses
/// their line/column as the node's span. Returns `(None, None, None, None)`
/// if there are no tokens (e.g., empty repetition).
fn compute_node_position(
    children: &[ASTNodeOrToken],
) -> (Option<usize>, Option<usize>, Option<usize>, Option<usize>) {
    let first = find_first_token(children);
    let last = find_last_token(children);
    match (first, last) {
        (Some(f), Some(l)) => (Some(f.line), Some(f.column), Some(l.line), Some(l.column)),
        _ => (None, None, None, None),
    }
}

fn find_first_token(children: &[ASTNodeOrToken]) -> Option<&Token> {
    for child in children {
        match child {
            ASTNodeOrToken::Token(tok) => return Some(tok),
            ASTNodeOrToken::Node(node) => {
                if let Some(tok) = find_first_token(&node.children) {
                    return Some(tok);
                }
            }
        }
    }
    None
}

fn find_last_token(children: &[ASTNodeOrToken]) -> Option<&Token> {
    for child in children.iter().rev() {
        match child {
            ASTNodeOrToken::Token(tok) => return Some(tok),
            ASTNodeOrToken::Node(node) => {
                if let Some(tok) = find_last_token(&node.children) {
                    return Some(tok);
                }
            }
        }
    }
    None
}

// ===========================================================================
// AST walking utilities
// ===========================================================================

/// Visitor interface for [`walk_ast`].
///
/// Both callbacks are optional. Each receives the current node and its parent
/// (or `None` for the root). Returning `Some(node)` replaces the visited node;
/// returning `None` keeps the original.
pub trait ASTVisitor {
    /// Called before visiting children. Return `Some` to replace the node.
    fn enter(&mut self, _node: &GrammarASTNode, _parent: Option<&GrammarASTNode>) -> Option<GrammarASTNode> {
        None
    }
    /// Called after visiting children. Return `Some` to replace the node.
    fn leave(&mut self, _node: &GrammarASTNode, _parent: Option<&GrammarASTNode>) -> Option<GrammarASTNode> {
        None
    }
}

/// Depth-first walk of an AST tree with enter/leave visitor callbacks.
///
/// Visitor callbacks can return a replacement node or `None` (keep original).
/// Token children are not visited -- only `GrammarASTNode` children are walked.
///
/// This is the generic traversal primitive. Language packages use it for
/// cover grammar rewriting, desugaring, and semantic analysis.
pub fn walk_ast(node: &GrammarASTNode, visitor: &mut dyn ASTVisitor) -> GrammarASTNode {
    walk_node(node, None, visitor)
}

fn walk_node(
    node: &GrammarASTNode,
    parent: Option<&GrammarASTNode>,
    visitor: &mut dyn ASTVisitor,
) -> GrammarASTNode {
    // Enter phase -- visitor may replace the node.
    let mut current = match visitor.enter(node, parent) {
        Some(replacement) => replacement,
        None => node.clone(),
    };

    // Walk children recursively.
    let mut children_changed = false;
    let mut new_children = Vec::with_capacity(current.children.len());
    for child in &current.children {
        match child {
            ASTNodeOrToken::Node(child_node) => {
                let walked = walk_node(child_node, Some(&current), visitor);
                if walked != *child_node {
                    children_changed = true;
                }
                new_children.push(ASTNodeOrToken::Node(walked));
            }
            ASTNodeOrToken::Token(tok) => {
                new_children.push(ASTNodeOrToken::Token(tok.clone()));
            }
        }
    }

    if children_changed {
        current.children = new_children;
    }

    // Leave phase -- visitor may replace the node.
    match visitor.leave(&current, parent) {
        Some(replacement) => replacement,
        None => current,
    }
}

/// Find all nodes matching a rule name (depth-first order).
pub fn find_nodes(node: &GrammarASTNode, rule_name: &str) -> Vec<GrammarASTNode> {
    let mut results = Vec::new();
    collect_matching_nodes(node, rule_name, &mut results);
    results
}

fn collect_matching_nodes(
    node: &GrammarASTNode,
    rule_name: &str,
    results: &mut Vec<GrammarASTNode>,
) {
    if node.rule_name == rule_name {
        results.push(node.clone());
    }
    for child in &node.children {
        if let ASTNodeOrToken::Node(child_node) = child {
            collect_matching_nodes(child_node, rule_name, results);
        }
    }
}

/// Collect all tokens in depth-first order, optionally filtered by type.
///
/// If `type_filter` is `None`, all tokens are collected. If `Some(type_name)`,
/// only tokens whose effective type name matches are collected.
pub fn collect_tokens(node: &GrammarASTNode, type_filter: Option<&str>) -> Vec<Token> {
    let mut results = Vec::new();
    collect_tokens_recursive(node, type_filter, &mut results);
    results
}

fn collect_tokens_recursive(
    node: &GrammarASTNode,
    type_filter: Option<&str>,
    results: &mut Vec<Token>,
) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(child_node) => {
                collect_tokens_recursive(child_node, type_filter, results);
            }
            ASTNodeOrToken::Token(tok) => {
                match type_filter {
                    None => results.push(tok.clone()),
                    Some(type_name) => {
                        if tok.effective_type_name() == type_name {
                            results.push(tok.clone());
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Newline detection — scan grammar for NEWLINE references
// ===========================================================================

/// Check if any rule in the grammar references the NEWLINE token.
fn grammar_references_newline(grammar: &ParserGrammar) -> bool {
    grammar.rules.iter().any(|rule| element_references_newline(&rule.body))
}

/// Recursively check if a grammar element references NEWLINE.
fn element_references_newline(element: &GrammarElement) -> bool {
    match element {
        GrammarElement::TokenReference { name } => name == "NEWLINE",
        GrammarElement::RuleReference { name } => name == "NEWLINE",
        GrammarElement::Sequence { elements } => {
            elements.iter().any(element_references_newline)
        }
        GrammarElement::Alternation { choices } => {
            choices.iter().any(element_references_newline)
        }
        GrammarElement::Repetition { element: inner }
        | GrammarElement::Optional { element: inner }
        | GrammarElement::Group { element: inner }
        | GrammarElement::PositiveLookahead { element: inner }
        | GrammarElement::NegativeLookahead { element: inner }
        | GrammarElement::OneOrMore { element: inner } => {
            element_references_newline(inner)
        }
        GrammarElement::SeparatedRepetition { element, separator, .. } => {
            element_references_newline(element) || element_references_newline(separator)
        }
        GrammarElement::Literal { .. } => false,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    /// Helper: create a token with default position.
    fn tok(type_: TokenType, value: &str) -> Token {
        Token { cv: None,
            type_,
            value: value.to_string(),
            line: 1,
            column: 1,
            type_name: None, flags: None,
        }
    }

    /// Helper: create a token with a string type name.
    fn tok_named(type_: TokenType, value: &str, type_name: &str) -> Token {
        Token { cv: None,
            type_,
            value: value.to_string(),
            line: 1,
            column: 1,
            type_name: Some(type_name.to_string()),
            flags: None,
        }
    }

    /// Build a simple test grammar:
    ///
    /// ```text
    /// expression = term { PLUS term } ;
    /// term       = NUMBER ;
    /// ```
    fn simple_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
                GrammarRule {
                    name: "expression".to_string(),
                    body: GrammarElement::Sequence {
                        elements: vec![
                            GrammarElement::RuleReference { name: "term".to_string() },
                            GrammarElement::Repetition {
                                element: Box::new(GrammarElement::Sequence {
                                    elements: vec![
                                        GrammarElement::TokenReference { name: "PLUS".to_string() },
                                        GrammarElement::RuleReference { name: "term".to_string() },
                                    ],
                                }),
                            },
                        ],
                    },
                    line_number: 1,
                },
                GrammarRule {
                    name: "term".to_string(),
                    body: GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    line_number: 2,
                },
            ],
            version: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Basic parsing
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_parse_single_number() {
        let tokens = vec![
            tok(TokenType::Number, "42"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        assert_eq!(result.children.len(), 1);
    }

    #[test]
    fn test_grammar_parse_addition() {
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        assert_eq!(result.children.len(), 3);
    }

    #[test]
    fn test_grammar_parse_chained_addition() {
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "3"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 5);
    }

    #[test]
    fn test_grammar_parse_empty_grammar() {
        let tokens = vec![tok(TokenType::Eof, "")];
        let grammar = ParserGrammar { rules: vec![], version: 0 };
        let mut parser = GrammarParser::new(tokens, grammar);
        assert!(parser.parse().is_err());
    }

    // -----------------------------------------------------------------------
    // Alternation, Optional, Literal, Group
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_parse_alternation() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "value".to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                        GrammarElement::TokenReference { name: "NAME".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        let tokens = vec![tok(TokenType::Number, "42"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar.clone());
        assert!(parser.parse().is_ok());

        let tokens = vec![tok(TokenType::Name, "x"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        assert!(parser.parse().is_ok());
    }

    #[test]
    fn test_grammar_parse_optional() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "maybe_number".to_string(),
                body: GrammarElement::Optional {
                    element: Box::new(GrammarElement::TokenReference {
                        name: "NUMBER".to_string(),
                    }),
                },
                line_number: 1,
            }],
            version: 0,
        };

        let tokens = vec![tok(TokenType::Number, "42"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar.clone());
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 1);

        let tokens = vec![tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.children.len(), 0);
    }

    #[test]
    fn test_grammar_parse_literal() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "greeting".to_string(),
                body: GrammarElement::Literal {
                    value: "hello".to_string(),
                },
                line_number: 1,
            }],
            version: 0,
        };
        let tokens = vec![tok(TokenType::Name, "hello"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "greeting");
    }

    #[test]
    fn test_grammar_parse_group() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "expr".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                        GrammarElement::Group {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::TokenReference { name: "PLUS".to_string() },
                                    GrammarElement::TokenReference { name: "MINUS".to_string() },
                                ],
                            }),
                        },
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];
        let mut parser = GrammarParser::new(tokens, grammar);
        assert_eq!(parser.parse().unwrap().children.len(), 3);
    }

    // -----------------------------------------------------------------------
    // AST node helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_ast_node_helpers() {
        let leaf = GrammarASTNode {
            rule_name: "number".to_string(),
            children: vec![ASTNodeOrToken::Token(tok(TokenType::Number, "42"))],
            start_line: None, start_column: None, end_line: None, end_column: None,
        };
        assert!(leaf.is_leaf());
        assert_eq!(leaf.token().unwrap().value, "42");

        let non_leaf = GrammarASTNode {
            rule_name: "expr".to_string(),
            children: vec![
                ASTNodeOrToken::Node(leaf.clone()),
                ASTNodeOrToken::Token(tok(TokenType::Plus, "+")),
            ],
            start_line: None, start_column: None, end_line: None, end_column: None,
        };
        assert!(!non_leaf.is_leaf());
        assert!(non_leaf.token().is_none());
    }

    // -----------------------------------------------------------------------
    // Integration: parser with lexer output
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_parser_with_lexer() {
        let source = "1 + 2";
        let mut lexer = lexer::tokenizer::Lexer::new(source, None);
        let tokens = lexer.tokenize().unwrap();
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        assert_eq!(result.children.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Packrat memoization
    // -----------------------------------------------------------------------

    #[test]
    fn test_packrat_memoization() {
        // Parse the same input twice to exercise memo cache.
        // The grammar has alternation that can cause re-parsing of the same
        // rule at the same position.
        let grammar = ParserGrammar {
            rules: vec![
                GrammarRule {
                    name: "start".to_string(),
                    body: GrammarElement::Alternation {
                        choices: vec![
                            // First alternative: NUMBER PLUS NUMBER
                            GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::RuleReference { name: "atom".to_string() },
                                    GrammarElement::TokenReference { name: "PLUS".to_string() },
                                    GrammarElement::RuleReference { name: "atom".to_string() },
                                ],
                            },
                            // Second alternative: just an atom
                            GrammarElement::RuleReference { name: "atom".to_string() },
                        ],
                    },
                    line_number: 1,
                },
                GrammarRule {
                    name: "atom".to_string(),
                    body: GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    line_number: 2,
                },
            ],
            version: 0,
        };

        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "start");
        // Memo should have been populated.
        assert!(!parser.memo.is_empty());
    }

    // -----------------------------------------------------------------------
    // String-based token types
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_token_types() {
        // Use custom token type names that don't map to TokenType variants.
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "expr".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference { name: "INT".to_string() },
                        GrammarElement::TokenReference { name: "PLUS".to_string() },
                        GrammarElement::TokenReference { name: "INT".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        let tokens = vec![
            tok_named(TokenType::Name, "1", "INT"),
            tok(TokenType::Plus, "+"),
            tok_named(TokenType::Name, "2", "INT"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expr");
        assert_eq!(result.children.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Significant newlines
    // -----------------------------------------------------------------------

    #[test]
    fn test_significant_newlines_detected() {
        // A grammar that references NEWLINE should be detected as
        // newlines-significant.
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "file".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference { name: "NAME".to_string() },
                        GrammarElement::TokenReference { name: "NEWLINE".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        let parser = GrammarParser::new(vec![], grammar);
        assert!(parser.is_newlines_significant());
    }

    #[test]
    fn test_insignificant_newlines_detected() {
        // A grammar without NEWLINE references should not be significant.
        let parser = GrammarParser::new(vec![], simple_grammar());
        assert!(!parser.is_newlines_significant());
    }

    // -----------------------------------------------------------------------
    // Furthest failure tracking
    // -----------------------------------------------------------------------

    #[test]
    fn test_furthest_failure_error_message() {
        // When parsing fails, the error should report what was expected
        // at the furthest position reached.
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Name, "x"),  // Invalid: expected PLUS or EOF
            tok(TokenType::Eof, ""),
        ];

        let grammar = simple_grammar();
        let mut parser = GrammarParser::new(tokens, grammar);
        let err = parser.parse().unwrap_err();
        // The error should mention what was expected.
        assert!(err.message.contains("Expected") || err.message.contains("Unexpected"));
    }

    // -----------------------------------------------------------------------
    // Starlark-like pipeline test
    // -----------------------------------------------------------------------

    #[test]
    fn test_starlark_pipeline() {
        // End-to-end: lex with grammar lexer, then parse with grammar parser.
        use grammar_tools::token_grammar::parse_token_grammar;
        use grammar_tools::parser_grammar::parse_parser_grammar;
        use lexer::grammar_lexer::GrammarLexer;

        let token_source = r#"
NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER = /[0-9]+/
EQUALS = "="
PLUS = "+"
"#;
        let grammar_source = r#"
program    = { statement } ;
statement  = assignment ;
assignment = NAME EQUALS expression ;
expression = term { PLUS term } ;
term       = NUMBER | NAME ;
"#;

        let token_grammar = parse_token_grammar(token_source).unwrap();
        let parser_grammar = parse_parser_grammar(grammar_source).unwrap();

        let tokens = GrammarLexer::new("x = 1 + 2", &token_grammar)
            .tokenize()
            .unwrap();

        let mut parser = GrammarParser::new(tokens, parser_grammar);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Trace mode
    // -----------------------------------------------------------------------

    #[test]
    fn test_trace_mode_parse_succeeds() {
        // new_with_trace(trace=true) must parse correctly — the same result
        // as new() with trace=false. Trace output goes to stderr so it does
        // not affect the return value.
        let tokens = vec![
            tok(TokenType::Number, "7"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
        let result = parser.parse();
        assert!(result.is_ok(), "trace mode must not affect parse correctness");
        assert_eq!(result.unwrap().rule_name, "expression");
    }

    #[test]
    fn test_trace_mode_no_panic_on_failure() {
        // When the input does not match the grammar, trace mode must not panic.
        // The error is the same as without trace mode.
        let tokens = vec![
            tok(TokenType::Plus, "+"), // Does not match `NUMBER`
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
        let result = parser.parse();
        assert!(result.is_err(), "invalid input should still produce an error in trace mode");
    }

    #[test]
    fn test_trace_mode_addition() {
        // Trace mode works correctly for a multi-token sequence.
        let tokens = vec![
            tok(TokenType::Number, "1"),
            tok(TokenType::Plus, "+"),
            tok(TokenType::Number, "2"),
            tok(TokenType::Eof, ""),
        ];
        let grammar = simple_grammar();
        let mut parser = GrammarParser::new_with_trace(tokens, grammar, true);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "expression");
        // expression expands to: term + term = NUMBER "+" NUMBER
        // children: the NUMBER node, the Plus token, the NUMBER node.
        assert_eq!(result.children.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Recursion-depth guard (DoS protection)
    // -----------------------------------------------------------------------

    /// A right-recursive grouping grammar:
    ///
    /// ```text
    /// group = "(" group ")" | NUMBER ;
    /// ```
    ///
    /// Each `(` forces one more level of `group` recursion, so an input of
    /// `N` open parens recurses `N` deep — exactly the nesting shape that
    /// overflows the native stack without a depth guard.
    fn nested_group_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![GrammarRule {
                name: "group".to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::Literal { value: "(".to_string() },
                                GrammarElement::RuleReference { name: "group".to_string() },
                                GrammarElement::Literal { value: ")".to_string() },
                            ],
                        },
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        }
    }

    /// Build the token stream for `n` nested parens around a `0`:
    /// `( ( … ( 0 ) … ) )`.
    fn nested_paren_tokens(n: usize) -> Vec<Token> {
        let mut tokens = Vec::with_capacity(2 * n + 2);
        for _ in 0..n {
            tokens.push(tok(TokenType::LParen, "("));
        }
        tokens.push(tok(TokenType::Number, "0"));
        for _ in 0..n {
            tokens.push(tok(TokenType::RParen, ")"));
        }
        tokens.push(tok(TokenType::Eof, ""));
        tokens
    }

    /// Deeply-nested input must produce a recoverable `GrammarParseError`
    /// ("input nests deeper than the supported limit") instead of overflowing
    /// the native stack (an uncatchable abort).
    ///
    /// We build a few thousand nested parens — far past `DEFAULT_MAX_RULE_DEPTH`
    /// — and parse on a worker thread with a generous 32 MiB stack. The large
    /// stack is deliberate: it guarantees the *guard* is what stops the
    /// recursion, not the stack running out, so the test is deterministic on
    /// any default-stack test runner. (Without the guard this same input would
    /// segfault even on this larger stack once it nested deep enough.)
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let grammar = nested_group_grammar();
                let tokens = nested_paren_tokens(5000);
                // The guard is opt-in (`new()` is unlimited), so a caller that
                // wants the DoS backstop dials it in with `with_max_depth`.
                let mut parser =
                    GrammarParser::new(tokens, grammar).with_max_depth(DEFAULT_MAX_RULE_DEPTH);
                let result = parser.parse();
                let err = result.expect_err(
                    "deeply-nested input must fail with an error, not parse or crash",
                );
                assert!(
                    err.message.contains("nests deeper than the supported limit"),
                    "expected a depth-limit error, got: {err}"
                );
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// Input that nests *exactly up to* the cap still parses cleanly — the
    /// guard only trips when we would recurse *past* the limit, so legal
    /// not-quite-as-deep input is unaffected. Uses a lowered cap so the test
    /// stays cheap. This is the no-regression half of the guard's contract.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        // group recursion depth for n parens is n+1 (n group→group steps plus
        // the final NUMBER alternative is reached within the n-th group). With
        // a cap of 64, 60 parens stays safely under the limit.
        let grammar = nested_group_grammar();
        let tokens = nested_paren_tokens(60);
        let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(64);
        let result = parser.parse();
        assert!(
            result.is_ok(),
            "input within the depth cap must parse, got: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().rule_name, "group");
    }

    /// Lowering the cap makes the guard trip on correspondingly shallower
    /// input, and the error is the precise depth-limit message. This pins the
    /// guard's behaviour without needing thousands of tokens or a big stack.
    #[test]
    fn test_low_cap_trips_depth_guard() {
        let grammar = nested_group_grammar();
        // 200 parens is well past a cap of 32.
        let tokens = nested_paren_tokens(200);
        let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(32);
        let err = parser.parse().expect_err("nesting past the cap must error");
        assert!(
            err.message.contains("nests deeper than the supported limit"),
            "expected depth-limit error, got: {err}"
        );
    }

    /// A caller that opts into [`DEFAULT_MAX_RULE_DEPTH`] must have the guard
    /// trip *before* the native stack overflows on a default-stack thread —
    /// otherwise a production caller (e.g. closurec) on an ordinary thread
    /// would still crash. We parse far-too-deep input on a worker thread with
    /// **no** `stack_size` override (the same ~2 MiB a default thread / the
    /// test runner gets). If the guard did not fire in time, the thread would
    /// overflow and `join()` would return `Err`; a clean parse-`Err` here
    /// proves the recommended opt-in cap sits safely below the overflow point
    /// on the default stack. (Empirically this implementation overflows around
    /// depth ~200 in a debug build on the default stack, so the cap of 128
    /// leaves comfortable headroom.)
    #[test]
    fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let grammar = nested_group_grammar();
            let tokens = nested_paren_tokens(5000);
            let mut parser =
                GrammarParser::new(tokens, grammar).with_max_depth(DEFAULT_MAX_RULE_DEPTH);
            let err = parser
                .parse()
                .expect_err("deeply-nested input must error, not crash");
            assert!(
                err.message.contains("nests deeper than the supported limit"),
                "expected depth-limit error, got: {err}"
            );
        });
        handle
            .join()
            .expect("opt-in cap must trip BEFORE native overflow on default stack");
    }

    #[test]
    fn test_trace_false_same_as_new() {
        // new_with_trace(trace=false) is identical in behaviour to new().
        let tokens = vec![
            tok(TokenType::Number, "99"),
            tok(TokenType::Eof, ""),
        ];
        let g1 = simple_grammar();
        let g2 = simple_grammar();
        let mut p1 = GrammarParser::new(tokens.clone(), g1);
        let mut p2 = GrammarParser::new_with_trace(tokens, g2, false);
        let r1 = p1.parse().unwrap();
        let r2 = p2.parse().unwrap();
        assert_eq!(r1.rule_name, r2.rule_name);
        assert_eq!(r1.children.len(), r2.children.len());
    }
}
