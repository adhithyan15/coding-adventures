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
// Backtracking checkpoints
// ===========================================================================

/// A point in the parse that [`GrammarParser::restore_to`] can roll back
/// to: both the token cursor and how much of the angle-bracket split undo
/// log had been written, captured together so one call undoes both in
/// lockstep.
///
/// Before nested-generic-closer splitting existed, `self.pos` was the only
/// mutable state a failed/abandoned attempt could have touched, so every
/// backtracking site in [`GrammarParser::match_element`] and
/// [`GrammarParser::parse_rule_inner`] restored it alone. Splitting a
/// merged `>>`/`>>>` token mutates `self.tokens` in place, which breaks
/// that assumption — an abandoned attempt (a failed `Alternation` arm, a
/// `Sequence` element that failed after an earlier sibling split a token,
/// a lookahead predicate that must "consume no input") needs to undo the
/// split too, not just rewind the cursor. `undo_len` is what makes that
/// possible: [`GrammarParser::restore_to`] pops
/// [`GrammarParser::split_undo_log`] back down to it, reverting each
/// mutation in reverse order.
#[derive(Clone, Copy)]
struct Checkpoint {
    pos: usize,
    undo_len: usize,
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

    /// Packrat memoization cache: `(rule_idx, pos)` -> `MemoEntry`.
    ///
    /// Keyed by a plain `(usize, usize)` tuple, not a `format!`-allocated
    /// string — every memo lookup happens on the hot path (once per rule
    /// attempted at every token position, for every grammar in this repo
    /// built on `GrammarParser`), so allocating and hashing a fresh string
    /// just to look up an already-cached `(rule, pos)` pair was pure
    /// overhead a tuple key avoids entirely (`usize` hashes and compares in
    /// O(1) with no allocation).
    memo: HashMap<(usize, usize), MemoEntry>,

    /// Furthest position reached during parsing.
    furthest_pos: usize,

    /// What was expected at the furthest position.
    furthest_expected: Vec<String>,
    /// Set of `(rule_index, pos)` pairs currently being parsed.
    /// Used to detect and break left recursion: if we try to parse a rule
    /// at a position where we're already inside that same rule (but haven't
    /// cached the result yet), we know it's left recursion and should fail.
    /// Same tuple-key rationale as [`Self::memo`] — no `format!` allocation
    /// needed to test/insert/remove a `(usize, usize)` pair.
    in_progress: std::collections::HashSet<(usize, usize)>,

    /// Undo log for in-place token mutations performed by
    /// `split_angle_bracket_run` (nested-generic `>>`/`>>>` closer
    /// splitting): `(pos, token_that_was_at_pos_before_this_mutation)`,
    /// pushed in chronological order.
    ///
    /// Every other kind of "try, maybe fail, roll back" in this parser
    /// backtracks by restoring `self.pos` alone, because until this log
    /// existed, `self.pos` was the *only* mutable state a failed attempt
    /// could have touched. Angle-bracket splitting breaks that invariant —
    /// it mutates `self.tokens[pos]` in place — so every one of this
    /// engine's existing backtracking sites (`Sequence`, `Alternation`,
    /// `Repetition`, `OneOrMore`, `SeparatedRepetition`, both lookaheads,
    /// and rule-level failure in `parse_rule_inner`) now pairs its
    /// `self.pos` snapshot/restore with a snapshot/restore of this log via
    /// [`Self::checkpoint`]/[`Self::restore_to`], so an abandoned attempt
    /// that triggered a split leaves *no* trace — not in `self.pos`, and
    /// not in `self.tokens` either. Entries are LIFO (a `Vec` used as a
    /// stack), so restoring to an earlier checkpoint correctly unwinds
    /// multiple layered splits at the same position (e.g. `>>>` split
    /// once into `>` + `>>`, then that `>>` split again into `>` + `>`) in
    /// reverse order.
    split_undo_log: Vec<(usize, Token)>,

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
            split_undo_log: Vec::new(),
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
    ///
    /// The membership check below compares `expected` against the existing
    /// `&str`s directly (`s.as_str() == expected`) rather than allocating an
    /// `expected.to_string()` up front just to ask "is this already here?" —
    /// this function runs on every failed rule/token attempt across the
    /// whole parse, so the previous `!v.contains(&expected.to_string())`
    /// paid a `String` allocation on every single call, including the
    /// overwhelmingly common case where the expectation was already recorded
    /// and nothing new needs to be pushed at all.
    fn record_failure(&mut self, expected: &str) {
        if self.pos > self.furthest_pos {
            self.furthest_pos = self.pos;
            self.furthest_expected = vec![expected.to_string()];
        } else if self.pos == self.furthest_pos
            && !self.furthest_expected.iter().any(|s| s.as_str() == expected) {
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

    // =========================================================================
    // Backtracking checkpoint/restore
    // =========================================================================

    /// Snapshot the current token cursor and angle-bracket-split undo-log
    /// length. Pair with [`Self::restore_to`] at every point this engine
    /// backtracks (a failed `Sequence` element, an abandoned `Alternation`
    /// arm, a lookahead predicate, a failed rule) so an abandoned attempt
    /// undoes everything it touched, not just `self.pos`.
    fn checkpoint(&self) -> Checkpoint {
        Checkpoint { pos: self.pos, undo_len: self.split_undo_log.len() }
    }

    /// Roll back to a [`Checkpoint`]: restores `self.pos` and reverts any
    /// angle-bracket-run token splits performed since the checkpoint was
    /// taken, in reverse (LIFO) order via [`Self::set_token_and_invalidate_memo`]
    /// — so a reverted split gets exactly the same memo-invalidation
    /// treatment a forward split does (see that method's doc comment for
    /// why: an entry memoized *while* the token was in its split state is
    /// just as stale, in the opposite direction, once the split is undone).
    fn restore_to(&mut self, checkpoint: Checkpoint) {
        // Revert every mutated token directly (O(1) each — plain index
        // assignment, no memo scan), tracking the smallest position
        // touched. `HashMap::retain`'s effect is monotonic in its
        // threshold (a smaller `pos` invalidates a superset of what a
        // larger one would), so invalidating once against the *minimum*
        // touched position produces exactly the same result as running
        // `set_token_and_invalidate_memo`'s scan once per popped entry —
        // just in one scan instead of up to `max_depth` of them.
        // `/security-review` (round 3) flagged the naive per-entry-scan
        // version as a real cost multiplier: a single abandoned,
        // deeply-nested-generic attempt could otherwise pay for a full
        // memo-table scan once per layer unwound, not once per backtrack.
        let mut min_touched_pos: Option<usize> = None;
        while self.split_undo_log.len() > checkpoint.undo_len {
            let (pos, original) = self.split_undo_log.pop().expect(
                "loop condition guarantees split_undo_log.len() > checkpoint.undo_len >= 0",
            );
            self.tokens[pos] = original;
            min_touched_pos = Some(min_touched_pos.map_or(pos, |m| m.min(pos)));
        }
        if let Some(pos) = min_touched_pos {
            self.memo.retain(|_, entry| entry.end_pos < pos);
        }
        self.pos = checkpoint.pos;
    }

    /// Overwrite the token at `pos`, and invalidate every memoized rule
    /// result whose completed parse could have read `pos` — used when
    /// performing an angle-bracket split (`match_token_reference`).
    /// `restore_to` needs the same invalidation logic when *undoing* a
    /// split on backtrack, but batches it across however many splits a
    /// single checkpoint unwinds into one memo scan rather than calling
    /// this once per reverted token, so it inlines the equivalent
    /// `retain` call itself instead of reusing this method directly — see
    /// `restore_to`'s own doc comment for why that batching is necessary.
    ///
    /// A rule's own `end_pos` (recorded at the moment its result was
    /// memoized) is normally "the first position this rule's completed
    /// parse did *not* read" — for an ordinary (non-split) match, `end_pos
    /// == pos` means the rule stopped strictly *before* `pos` and is safe
    /// to keep. But an angle-bracket split deliberately does *not* advance
    /// `self.pos` (the whole point is to leave the remainder at the same
    /// index for the next match attempt) — so a rule whose own *last* step
    /// was itself a split has `end_pos == pos`, yet it very much did read
    /// (partially consume) the token at `pos`. `MemoEntry` doesn't track
    /// "did the last step split," so there's no way to tell these two
    /// `end_pos == pos` cases apart from the entry alone — the invalidation
    /// here must treat `end_pos == pos` as "might have," not "didn't."
    ///
    /// **Caught by `/security-review` (round 3)**: an earlier version used
    /// `entry.end_pos <= pos` (keeping `end_pos == pos`), which silently
    /// kept a genuinely stale entry for a split-ending rule — replaying a
    /// two-level nested-generic close (`Map<String, List<Integer>>`) after
    /// a sibling `Alternation` arm (`method_declaration` vs
    /// `field_declaration`) backtracked reused the *inner* `List<Integer>`
    /// closer's cached result without re-splitting the (reverted) merged
    /// token, leaving a dangling `>` the outer `Map<...>` closer's own
    /// fresh split then only partially resolved, corrupting the parse.
    /// `entry.end_pos < pos` (drop on equality too) closes this: it may
    /// occasionally invalidate a few ordinary, unrelated entries that
    /// happen to end exactly at `pos` for other reasons (a pure
    /// performance cost — they simply get correctly recomputed), but never
    /// leaves a split-produced stale entry behind. Cost stays bounded by
    /// how many cached entries actually span `pos` (grammar depth/local
    /// ambiguity in practice), not by total file length — a full
    /// `self.memo.clear()` here was flagged in an earlier `/security-review`
    /// pass as an algorithmic-complexity DoS vector, since the memo table
    /// grows with how much of the file has already been parsed and
    /// ordinary, non-adversarial source can contain many nested-generic
    /// occurrences.
    fn set_token_and_invalidate_memo(&mut self, pos: usize, new_token: Token) {
        self.tokens[pos] = new_token;
        self.memo.retain(|_, entry| entry.end_pos < pos);
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
            let key = (idx, self.pos);
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
            if !self.in_progress.insert(key) {
                // key was already present — left recursion detected
                return None;
            }
        }

        let start_checkpoint = self.checkpoint();
        let start_pos = start_checkpoint.pos;

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
            let key = (idx, start_pos);
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
                self.restore_to(start_checkpoint);
                self.record_failure(rule_name);
                None
            }
        }
    }

    // =========================================================================
    // Element matching
    // =========================================================================

    fn match_element(&mut self, element: &GrammarElement) -> Option<Vec<ASTNodeOrToken>> {
        let checkpoint = self.checkpoint();

        match element {
            GrammarElement::Sequence { elements } => {
                let mut children = Vec::new();
                for sub in elements {
                    match self.match_element(sub) {
                        Some(mut result) => children.append(&mut result),
                        None => {
                            self.restore_to(checkpoint);
                            return None;
                        }
                    }
                }
                Some(children)
            }

            GrammarElement::Alternation { choices } => {
                for choice in choices {
                    self.restore_to(checkpoint);
                    if let Some(result) = self.match_element(choice) {
                        return Some(result);
                    }
                }
                self.restore_to(checkpoint);
                None
            }

            GrammarElement::Repetition { element: inner } => {
                let mut children = Vec::new();
                loop {
                    let rep_checkpoint = self.checkpoint();
                    match self.match_element(inner) {
                        Some(mut result) => children.append(&mut result),
                        None => {
                            self.restore_to(rep_checkpoint);
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
                            self.restore_to(checkpoint);
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

                // A `Literal` element matches an OPERATOR/KEYWORD lexeme by
                // its literal spelling -- the trick every grammar comment
                // in this codebase describes as "the parser dispatches by
                // value" for tokens the lexer leaves on a catch-all type
                // (`<`, `<<`, `&&`, `and`, …). But `TokenType::String`
                // carries arbitrary user-supplied STRING-LITERAL CONTENT,
                // not a lexeme — its `value` is whatever text sat between
                // the quotes. Without this guard, a Ruby program containing
                // a string literal whose CONTENT happens to equal an
                // operator spelling (e.g. `foo(1, "*")`, `x = "&&"`) gets
                // that string token silently swallowed by an unrelated
                // `Literal { value: "*" }` element (e.g. the splat marker
                // in `call_arg`'s `[ "*" | "**" | "&" ] expression`),
                // leaving the REST of that rule with nothing to match and
                // producing a confusing parse failure (or, in a
                // panic-on-parse-error caller, a hard crash) for a
                // perfectly ordinary Ruby program. Excluding `String` here
                // is a pure narrowing of what `Literal` can match: no
                // grammar's `Literal` element is ever intended to match
                // arbitrary string-literal content, so this can only fix
                // previously-incorrect matches, never reject a
                // previously-correct one.
                if self.current().type_ != TokenType::String && self.current().value == *value {
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
                // Succeed if inner element matches, but consume no input --
                // and, symmetrically, leave no trace of any angle-bracket
                // split the inner attempt performed: `restore_to` (not a
                // bare `self.pos = checkpoint.pos`) is what makes "consume
                // no input" actually true, not just true for the cursor.
                let result = self.match_element(inner);
                self.restore_to(checkpoint);
                if result.is_some() { Some(Vec::new()) } else { None }
            }

            GrammarElement::NegativeLookahead { element: inner } => {
                // Succeed if inner element does NOT match, consume no input.
                let result = self.match_element(inner);
                self.restore_to(checkpoint);
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
                        self.restore_to(checkpoint);
                        None
                    }
                    Some(mut children) => {
                        loop {
                            let rep_checkpoint = self.checkpoint();
                            match self.match_element(inner) {
                                Some(mut result) => children.append(&mut result),
                                None => {
                                    self.restore_to(rep_checkpoint);
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
                        self.restore_to(checkpoint);
                        if *at_least_one { None } else { Some(Vec::new()) }
                    }
                    Some(mut children) => {
                        loop {
                            let sep_checkpoint = self.checkpoint();
                            let sep = self.match_element(separator);
                            match sep {
                                None => {
                                    self.restore_to(sep_checkpoint);
                                    break;
                                }
                                Some(mut sep_children) => {
                                    let next = self.match_element(inner);
                                    match next {
                                        None => {
                                            self.restore_to(sep_checkpoint);
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
            // Contextual token-splitting for nested generic-argument-list
            // closers (`Map<String, List<Integer>>`, `Box<Box<Box<T>>>`)
            // — see `split_angle_bracket_run`'s own doc comment for the
            // full rationale. Only reached when the exact-match check
            // above already failed, and only fires for the specific
            // (expected "GREATER_THAN", actual "RIGHT_SHIFT"/
            // "UNSIGNED_RIGHT_SHIFT") pairing, so grammars that don't use
            // this exact C-family token-naming convention (the vast
            // majority of grammars this shared engine also serves) never
            // take this branch at all.
            // `self.pos < self.tokens.len()` guards a real, if currently
            // latent, panic (`/security-review` round 3): once `self.pos`
            // runs past the end of the token stream, `current()` (just
            // above) falls back to *reading* `tokens[len - 1]` without
            // moving `self.pos` there — but a split needs to *write* the
            // remainder back to wherever `token` actually lives, and
            // `self.tokens[self.pos]` indexing with the raw, out-of-range
            // `self.pos` would panic. This repo's own Java/C# pipelines
            // always append a trailing EOF token (never itself
            // `RIGHT_SHIFT`/`UNSIGNED_RIGHT_SHIFT`-shaped) before
            // `self.pos` could run this far, so the guard is never
            // observed to fire today — but `GrammarParser` is a public,
            // reusable engine with no enforced "must end in EOF"
            // precondition (and exposes `add_pre_parse` hooks that could
            // truncate the token list), so this is real hardening against
            // misuse of that public surface, not dead code. Splitting a
            // token you're not genuinely positioned at doesn't make sense
            // anyway — there's nothing wrong with simply not matching here.
            if self.pos < self.tokens.len() {
                if let Some((consumed, remainder)) = split_angle_bracket_run(token, expected_type) {
                    let split_pos = self.pos;
                    let original = token.clone();
                    // Record the pre-split token so a backtrack that
                    // abandons whatever is currently being attempted can
                    // undo this mutation via `restore_to` — see
                    // `split_undo_log`'s own doc comment. Without this, a
                    // failed `Alternation` arm or a lookahead predicate
                    // that triggers a split would leave the mutated
                    // (shorter) token behind for a sibling attempt that
                    // expects the original merged `>>`/`>>>` shape,
                    // silently corrupting an otherwise-unrelated parse.
                    self.split_undo_log.push((split_pos, original));
                    self.set_token_and_invalidate_memo(split_pos, remainder);
                    return Some(vec![ASTNodeOrToken::Token(consumed)]);
                }
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
// Nested-generic-closer token splitting
// ===========================================================================

/// Split a merged multi-`>` token (`>>`/`>>>`) into one consumed `>` plus a
/// shorter remainder token, when the grammar is specifically trying to
/// match a bare `GREATER_THAN` — the classic C-family "nested generic
/// argument list" ambiguity (`Map<String, List<Integer>>`,
/// `Box<Box<Box<T>>>`): a context-free lexer cannot know it should *not*
/// merge consecutive `>` characters into a single right-shift-shaped token
/// without knowing it's inside a type-argument list, so it always merges
/// them (the same way a real `x >> 2` right-shift expression tokenizes) —
/// and the *parser*, which does have that context (it only ever asks for a
/// lone `GREATER_THAN` when closing exactly one generic-argument-list
/// level), is the only place this can be resolved.
///
/// Real parsers for C-family languages solve this exact problem with
/// contextual token-splitting rather than a lexer-level special case
/// (which would incorrectly break genuine `>>`/`>>>` shift operators
/// elsewhere in the same grammar). This is the same technique, applied at
/// the one call site (`match_token_reference`) where the grammar's
/// expectation (`expected_type`) and the token's actual shape can both be
/// seen at once.
///
/// Deliberately narrow: only fires for the exact
/// (`expected_type == "GREATER_THAN"`, `actual_type_name` is
/// `"RIGHT_SHIFT"` or `"UNSIGNED_RIGHT_SHIFT"`) pairing — the C-family
/// token-naming convention shared by this repo's Java and C# grammars
/// (confirmed identical in both `.tokens` files). A grammar using
/// different token names for these operators (or not defining them at
/// all) never reaches this function with a matching `actual_type_name`,
/// so this is a no-op for every other language this shared engine serves
/// — it cannot, by construction, affect Ruby/Python/Twig/etc. parsing.
///
/// Deliberately does NOT handle `GREATER_EQUALS`/`RIGHT_SHIFT_EQUALS`/
/// `UNSIGNED_RIGHT_SHIFT_EQUALS` (any `>`-run with a trailing `=`) —
/// those aren't pure `>` runs, so splitting a leading `>` off `>=` would
/// produce a nonsensical remainder (`=`, not a valid start of any further
/// token); `>=` immediately following a type-argument list is not valid
/// Java/C# syntax regardless (an assignment-shaped token can't follow a
/// generic close in an expression position), so no real program needs
/// this case handled.
///
/// Returns `(consumed, remainder)`: `consumed` is the single `>` token to
/// report as this match's result; `remainder` is written back into the
/// parser's own token stream at the *same* position (see the call site),
/// so the very next match attempt sees it fresh — this is what lets
/// `Map<String, List<Integer>>` close both nesting levels from one merged
/// `>>` token, one `GREATER_THAN` match at a time.
fn split_angle_bracket_run(token: &Token, expected_type: &str) -> Option<(Token, Token)> {
    if expected_type != "GREATER_THAN" {
        return None;
    }
    let actual_type_name = token.type_name.as_deref()?;
    let remainder_type_name = match actual_type_name {
        "RIGHT_SHIFT" => "GREATER_THAN",
        "UNSIGNED_RIGHT_SHIFT" => "RIGHT_SHIFT",
        _ => return None,
    };
    // Defensive: only split a token whose value is genuinely a bare run of
    // `>` characters matching the token name's own expected length — a
    // malformed/unexpected token shape (should never happen given how the
    // lexer emits these, but this function must not panic on it) falls
    // through to the normal "no match" path instead of guessing.
    if !token.value.chars().all(|c| c == '>') || token.value.len() < 2 {
        return None;
    }
    // Both halves originate from the same source bytes as `token`, so they
    // carry its `cv` id forward unchanged. `consumed` keeps `token`'s own
    // flags (it starts at the same position `token` did); `remainder`
    // starts mid-token with no newline in between, so it gets a clean
    // `None` — these are operator tokens, so neither
    // `TOKEN_PRECEDED_BY_NEWLINE` nor `TOKEN_CONTEXT_KEYWORD` is meaningful
    // for it regardless.
    let consumed = Token {
        type_: TokenType::Name,
        value: ">".to_string(),
        line: token.line,
        column: token.column,
        type_name: Some("GREATER_THAN".to_string()),
        flags: token.flags,
        cv: token.cv.clone(),
    };
    let remainder = Token {
        type_: TokenType::Name,
        value: token.value[1..].to_string(),
        line: token.line,
        column: token.column + 1,
        type_name: Some(remainder_type_name.to_string()),
        flags: None,
        cv: token.cv.clone(),
    };
    Some((consumed, remainder))
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

/// Find all nodes matching a rule name (depth-first, pre-order).
///
/// Iterative (an explicit `Vec`-backed stack), not recursive, deliberately:
/// this is a public entry point that accepts any caller-constructed
/// `GrammarASTNode`, not only trees produced by [`GrammarParser::parse`]'s
/// own depth-capped recursion. `/security-review` flagged the original
/// recursive version as a CWE-674 uncontrolled-recursion risk — a
/// pathologically deep hand-built tree handed straight to this function
/// could overflow the native call stack, an *uncatchable* crash no
/// `Result` could report. An iterative walk has no such ceiling: its
/// "stack" is heap-allocated and bounded by available memory, not the
/// thread's fixed native stack size, and it can't be bypassed by a caller
/// skipping `GrammarParser` entirely (as at least one `-to-semantic-ir`
/// frontend's own `compile()` does, working directly with a raw
/// `GrammarASTNode`). Preserves the original recursive version's
/// visitation order: a node is checked before its children, and each
/// child's own subtree is fully explored before moving to the next
/// sibling.
pub fn find_nodes(node: &GrammarASTNode, rule_name: &str) -> Vec<GrammarASTNode> {
    let mut results = Vec::new();
    let mut stack: Vec<&GrammarASTNode> = vec![node];
    while let Some(current) = stack.pop() {
        if current.rule_name == rule_name {
            results.push(current.clone());
        }
        // Push in reverse so the leftmost child ends up on top of the
        // stack and is therefore popped (and thus visited) first --
        // reproducing the same left-to-right pre-order traversal the
        // original recursive version walked.
        for child in current.children.iter().rev() {
            if let ASTNodeOrToken::Node(child_node) = child {
                stack.push(child_node);
            }
        }
    }
    results
}

/// Collect all tokens in depth-first order, optionally filtered by type.
///
/// If `type_filter` is `None`, all tokens are collected. If `Some(type_name)`,
/// only tokens whose effective type name matches are collected.
///
/// Iterative for the same reason [`find_nodes`] is — see its doc comment.
/// Uses an explicit stack of child-slice iterators (rather than a stack of
/// nodes, as `find_nodes` uses) because token results must interleave
/// correctly with descending into sibling nodes: a `children` list mixes
/// `Token` and `Node` entries in source order, so each node's children
/// need to resume exactly where they left off after a nested node's own
/// subtree has been fully drained, not be visited as one atomic unit.
pub fn collect_tokens(node: &GrammarASTNode, type_filter: Option<&str>) -> Vec<Token> {
    let mut results = Vec::new();
    let mut stack: Vec<std::slice::Iter<'_, ASTNodeOrToken>> = vec![node.children.iter()];
    while let Some(top) = stack.last_mut() {
        match top.next() {
            None => {
                stack.pop();
            }
            Some(ASTNodeOrToken::Node(child_node)) => {
                stack.push(child_node.children.iter());
            }
            Some(ASTNodeOrToken::Token(tok)) => {
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
    results
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

    /// Bug fix: a `Literal` element matches an operator/keyword LEXEME by
    /// its spelling -- the "dispatches by value" trick every grammar
    /// comment describes for tokens the lexer leaves on a catch-all type
    /// (`<`, `<<`, `&&`, `and`, …). But `TokenType::String` carries
    /// arbitrary user-supplied string-LITERAL CONTENT, not a lexeme.
    /// Before this fix, `Literal { value: "hello" }` matched a `String`
    /// token whose CONTENT happened to be `"hello"` just as readily as a
    /// `Name`-typed operator token spelled `hello` -- e.g. Ruby's
    /// `foo(1, "*")` had its `"*"` STRING ARGUMENT silently swallowed by
    /// `call_arg`'s `[ "*" | "**" | "&" ] expression` splat-marker
    /// alternative, leaving the actual `expression` with nothing to
    /// consume and producing a confusing parse failure for an ordinary
    /// Ruby program.
    #[test]
    fn test_literal_does_not_match_a_string_token_with_the_same_content() {
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
        // A STRING token whose CONTENT is "hello" (e.g. Ruby source
        // `"hello"`) must NOT satisfy a `Literal { value: "hello" }`
        // element -- only a non-String token spelled `hello` (the
        // catch-all-Name-typed operator/keyword case this element exists
        // for) should.
        let tokens = vec![tok(TokenType::String, "hello"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        assert!(
            parser.parse().is_err(),
            "a String-typed token must not satisfy a Literal match on its content"
        );
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
    // find_nodes / collect_tokens (`/security-review` CWE-674 hardening)
    // -----------------------------------------------------------------------

    fn leaf_node(rule_name: &str, tok: Token) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule_name.to_string(),
            children: vec![ASTNodeOrToken::Token(tok)],
            start_line: None, start_column: None, end_line: None, end_column: None,
        }
    }

    fn wrap_node(rule_name: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule_name.to_string(),
            children,
            start_line: None, start_column: None, end_line: None, end_column: None,
        }
    }

    /// `find_nodes` must visit the root before its children, and each
    /// child's own subtree before moving to the next sibling -- the same
    /// order the original recursive version walked.
    #[test]
    fn find_nodes_visits_in_pre_order_left_to_right() {
        let a = wrap_node("target", vec![ASTNodeOrToken::Token(tok(TokenType::Number, "1"))]);
        let b_inner = wrap_node("target", vec![ASTNodeOrToken::Token(tok(TokenType::Number, "2"))]);
        let b = wrap_node("wrapper", vec![ASTNodeOrToken::Node(b_inner)]);
        let root = wrap_node(
            "target",
            vec![ASTNodeOrToken::Node(a), ASTNodeOrToken::Node(b)],
        );

        let matches = find_nodes(&root, "target");
        assert_eq!(matches.len(), 3);
        // Root first (its own child is a Node, not a Token), then "a"
        // (left child, value "1"), then "b"'s nested "target" (value "2").
        let values: Vec<&str> = matches
            .iter()
            .map(|n| match &n.children[0] {
                ASTNodeOrToken::Token(t) => t.value.as_str(),
                ASTNodeOrToken::Node(_) => "<root>",
            })
            .collect();
        assert_eq!(values[1], "1");
        assert_eq!(values[2], "2");
    }

    /// `collect_tokens` must interleave correctly with node descent: a
    /// bare token sibling and a nested node's own tokens must come out in
    /// source order, not "all direct tokens then all nested tokens."
    #[test]
    fn collect_tokens_preserves_source_order_across_nested_nodes() {
        let inner = wrap_node(
            "inner",
            vec![ASTNodeOrToken::Token(tok(TokenType::Number, "2"))],
        );
        let root = wrap_node(
            "root",
            vec![
                ASTNodeOrToken::Token(tok(TokenType::Number, "1")),
                ASTNodeOrToken::Node(inner),
                ASTNodeOrToken::Token(tok(TokenType::Number, "3")),
            ],
        );
        let values: Vec<String> = collect_tokens(&root, None).into_iter().map(|t| t.value).collect();
        assert_eq!(values, vec!["1".to_string(), "2".to_string(), "3".to_string()]);
    }

    #[test]
    fn collect_tokens_respects_type_filter() {
        let root = wrap_node(
            "root",
            vec![
                ASTNodeOrToken::Token(tok(TokenType::Number, "1")),
                ASTNodeOrToken::Token(tok(TokenType::Plus, "+")),
                ASTNodeOrToken::Token(tok(TokenType::Number, "2")),
            ],
        );
        let numbers = collect_tokens(&root, Some("NUMBER"));
        assert_eq!(numbers.len(), 2);
        assert_eq!(numbers[0].value, "1");
        assert_eq!(numbers[1].value, "2");
    }

    /// Regression guard for the `/security-review` CWE-674 finding: both
    /// `find_nodes` and `collect_tokens` are public entry points reachable
    /// on a raw, caller-constructed `GrammarASTNode` -- not only trees
    /// produced by `GrammarParser::parse`'s own depth-capped recursion.
    /// Before the iterative rewrite, a pathologically deep hand-built tree
    /// handed straight to either function would overflow the *native*
    /// call stack -- an uncatchable crash, not a recoverable error. This
    /// builds a tree far deeper than any recursive implementation could
    /// survive on a default-stack thread and proves both functions still
    /// complete cleanly and correctly.
    #[test]
    fn find_nodes_and_collect_tokens_survive_pathologically_deep_trees_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let mut inner = leaf_node("target", tok(TokenType::Number, "42"));
            for _ in 0..50_000 {
                inner = wrap_node("wrapper", vec![ASTNodeOrToken::Node(inner)]);
            }

            let matches = find_nodes(&inner, "target");
            assert_eq!(matches.len(), 1, "the one deeply-buried target node must still be found");

            let tokens = collect_tokens(&inner, None);
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].value, "42");

            // Deliberately leak `inner` rather than let it drop normally.
            // This is testing `find_nodes`/`collect_tokens` specifically,
            // not `GrammarASTNode`'s ordinary compiler-generated `Drop`
            // glue -- which, being itself a plain recursive walk of the
            // same nested `Vec<ASTNodeOrToken>` structure, independently
            // overflows the native stack on a tree this deep. That's a
            // real, separate CWE-674-shaped gap (discovered while writing
            // this test), logged as its own follow-up rather than fixed
            // here or silently worked around by shrinking this test's
            // depth to hide it.
            std::mem::forget(inner);
        });
        handle
            .join()
            .expect("find_nodes/collect_tokens must not overflow the native stack on deep input");
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
    // Angle-bracket split undo on backtrack (`/security-review` round 2)
    // -----------------------------------------------------------------------

    /// A failed `Alternation` arm that triggers an angle-bracket split
    /// must not leave the mutated (shorter) token behind for a sibling
    /// arm that expects the original merged token. `/security-review`
    /// (round 2) demonstrated this concretely with exactly this grammar
    /// shape before `restore_to`/`split_undo_log` existed: `choiceA`
    /// (`GREATER_THAN IMPOSSIBLE`) matches `GREATER_THAN` by splitting the
    /// `RIGHT_SHIFT`-typed `">>"` token, then fails on the nonexistent
    /// `IMPOSSIBLE` token and backtracks; `choiceB` (`RIGHT_SHIFT`) must
    /// then see the token exactly as it was before `choiceA` ever ran.
    #[test]
    fn failed_alternation_arm_undoes_its_own_angle_bracket_split() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "start".to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::TokenReference { name: "GREATER_THAN".to_string() },
                                GrammarElement::TokenReference { name: "IMPOSSIBLE".to_string() },
                            ],
                        },
                        GrammarElement::TokenReference { name: "RIGHT_SHIFT".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        let tokens = vec![
            tok_named(TokenType::Name, ">>", "RIGHT_SHIFT"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        assert_eq!(result.rule_name, "start");
        // The surviving match is choiceB: a single, whole ">>" token, not
        // the split-off ">" choiceA would have produced.
        let matched_values: Vec<String> = collect_tokens(&result, None)
            .iter()
            .map(|t| t.value.clone())
            .collect();
        assert_eq!(matched_values, vec![">>".to_string()]);
    }

    /// Same shape as above, but through a `PositiveLookahead` — documented
    /// as "consume no input," which must include not leaving a mutated
    /// token behind either. `/security-review` named this construct
    /// explicitly: syntactic disambiguation between "this closes a
    /// generic-argument list" and "this is a real shift/comparison
    /// operator" is exactly what lookahead predicates are for.
    #[test]
    fn positive_lookahead_undoes_its_own_angle_bracket_split() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "start".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::PositiveLookahead {
                            element: Box::new(GrammarElement::TokenReference {
                                name: "GREATER_THAN".to_string(),
                            }),
                        },
                        GrammarElement::TokenReference { name: "RIGHT_SHIFT".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        let tokens = vec![
            tok_named(TokenType::Name, ">>", "RIGHT_SHIFT"),
            tok(TokenType::Eof, ""),
        ];

        let mut parser = GrammarParser::new(tokens, grammar);
        let result = parser.parse().unwrap();
        let matched_values: Vec<String> = collect_tokens(&result, None)
            .iter()
            .map(|t| t.value.clone())
            .collect();
        assert_eq!(matched_values, vec![">>".to_string()]);
    }

    /// Regression guard for a panic `/security-review` (round 3) found in
    /// the split path itself: once `self.pos` runs past the end of the
    /// token stream, `current()` falls back to *reading* the last token
    /// without moving `self.pos` there, but the (pre-fix) split branch
    /// still *wrote* to `self.tokens[self.pos]` using the raw,
    /// out-of-range `self.pos` — an index-out-of-bounds panic. This
    /// repo's own Java/C# pipelines always append a trailing EOF token
    /// first, so the guard never fires in practice today, but
    /// `GrammarParser` is a public, reusable engine with no enforced
    /// "must end in EOF" precondition. This grammar deliberately omits an
    /// EOF token to reach that state directly.
    #[test]
    fn split_past_end_of_token_stream_does_not_panic() {
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "start".to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference { name: "RIGHT_SHIFT".to_string() },
                        GrammarElement::TokenReference { name: "GREATER_THAN".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        // No trailing EOF token -- after matching RIGHT_SHIFT, self.pos
        // runs past the end of this single-token stream.
        let tokens = vec![tok_named(TokenType::Name, ">>", "RIGHT_SHIFT")];

        let mut parser = GrammarParser::new(tokens, grammar);
        // Must fail cleanly (no second token to close with), not panic.
        assert!(parser.parse().is_err());
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

    #[test]
    fn test_furthest_failure_expectations_are_deduplicated() {
        // Two alternatives that expect the exact same token type at the same
        // furthest position (a contrived but valid grammar: NUMBER | NUMBER)
        // both call `record_failure("NUMBER")` when a NAME token shows up
        // instead. `record_failure`'s dedup check (`!v.iter().any(|s| s ==
        // expected)`, changed from an allocate-then-`Vec::contains` check
        // during a performance pass) must still record "NUMBER" only once,
        // not once per failing alternative.
        let grammar = ParserGrammar {
            rules: vec![GrammarRule {
                name: "value".to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                        GrammarElement::TokenReference { name: "NUMBER".to_string() },
                    ],
                },
                line_number: 1,
            }],
            version: 0,
        };

        let tokens = vec![tok(TokenType::Name, "x"), tok(TokenType::Eof, "")];
        let mut parser = GrammarParser::new(tokens, grammar);
        let err = parser.parse().unwrap_err();
        // "NUMBER" must appear exactly once in the message, not duplicated
        // ("NUMBER or NUMBER") the way an un-deduplicated list would render.
        assert_eq!(
            err.message.matches("NUMBER").count(),
            1,
            "expected \"NUMBER\" to appear exactly once (deduplicated), got: {}",
            err.message
        );
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
