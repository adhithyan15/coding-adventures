//! # coding-adventures-ruby-lexer
//!
//! Ruby lexer driven by a TOML-encoded state machine.  See
//! `code/specs/ruby-parser.md` for the architectural overview and
//! `code/specs/ruby-lexer-state-machine.md` for the per-state
//! transition rules.
//!
//! ## Pipeline
//!
//! ```text
//! Ruby source (&str)
//!    │
//!    ▼  RubyLexer::new(version)
//!    │   ┌── EffectfulStateMachine — from `ruby-<ver>.lexer.states.toml`
//!    │
//!    ▼  push(source) — one character at a time
//! action interpreter (this crate) — turns effect strings into Tokens
//!    │
//!    ▼  finish() then drain_tokens()
//! Vec<Token>
//! ```
//!
//! ## Phase 1 scope
//!
//! Per `code/specs/ruby-parser.md`, Phase 1 covers the **paren-
//! required Ruby 1.8 baseline**: identifiers, integers, strings (no
//! interpolation), line comments, common operators and punctuation,
//! and newline-as-token.  Heredocs, regex disambiguation, percent
//! literals, string interpolation, and parser-driven `f /x/`
//! resolution all arrive in later phases.

use std::collections::VecDeque;

use lexer::token::{Token, TokenType};
use state_machine::transducer::{EffectfulInput, EffectfulStateMachine};

mod lex_state;
mod machine;
mod oracle;

pub use lex_state::LexState;
pub use machine::ERA_VERSIONS;
pub use oracle::{NoLocals, ParserOracle, StaticLocals};

// Phase 4d — re-export the numbered-block-param flag bit for
// downstream tooling.  Defined further down in the file.

/// Non-fatal diagnostic produced by the lexer.  Stray bytes /
/// unterminated strings / etc. are recorded here; the lexer keeps
/// going from the next character so callers always get a complete
/// token stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub line: usize,
    pub column: usize,
}

/// Stateful Ruby lexer.
pub struct RubyLexer {
    machine: EffectfulStateMachine,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    /// Accumulator the action interpreter writes into.  Cleared on
    /// each `clear_text`; flushed by every `emit_token`.
    text_buffer: String,
    /// Current source position of the next character to be fed.
    /// 1-based.
    line: usize,
    column: usize,
    /// Source position where the current token started accumulating.
    token_start_line: usize,
    token_start_column: usize,
    /// Phase 2: current lex-state, updated on every emit.  Drives
    /// `/` disambiguation and (in later phases) other ambiguous
    /// constructs.
    lex_state: LexState,
    /// Phase 2: the most recent `Name` token's lexeme.  Used by the
    /// `/` interceptor when `lex_state == ExprArg` — the oracle
    /// classifies this name as a local or a method.
    last_name: String,
    /// Phase 2: the parser-feedback contract.  Defaults to
    /// [`NoLocals`] which treats every name as a method.
    oracle: Box<dyn ParserOracle>,
    /// Phase 3b: nesting depth of `{` / `}` inside a `#{...}`
    /// string interpolation.  Incremented on `{`, decremented on
    /// `}`, and `}` at depth 0 closes the interpolation (interpreter
    /// manually transitions the engine back to `string_d_body`).
    interp_brace_depth: usize,
    /// Phase 3c: FIFO queue of heredoc openers (`<<TAG`) whose body
    /// has not yet been captured.  When a `<<TAG` is detected at an
    /// expression-start position, a [`PendingHeredoc`] is pushed
    /// here; when the line containing the opener ends (a `\n` is
    /// fed), [`RubyLexer::capture_heredoc_bodies`] slurps subsequent
    /// lines into each pending body until each `TAG`-only terminator
    /// line is seen, in FIFO order.  Multiple `<<A; x = <<B` on one
    /// line is supported.
    pending_heredocs: VecDeque<PendingHeredoc>,
    /// Phase 3c: when we just emitted an `<<` Op token at expression
    /// start, we may be at the front of a heredoc.  This field holds
    /// the index of that `<<` token in [`RubyLexer::tokens`]; the very
    /// next emit decides what to do:
    /// - if it's a `Name` token whose lexeme is a valid tag, that
    ///   name is queued as a [`PendingHeredoc`];
    /// - otherwise the candidate is cleared and the `<<` stays as a
    ///   plain left-shift operator.
    heredoc_op_candidate: Option<usize>,
    /// Phase 4b: the Ruby era this lexer is targeting.  Drives the
    /// post-pass token-stream rewriter (see
    /// [`RubyLexer::apply_era_token_fusions`]) which folds adjacent
    /// `Op("-")` + `Op(">")` into `Op("->")` for era ≥ 1.9.1, and
    /// will host further era-gated fusions (`&.` in 2.3, `<<~` in
    /// 2.3, `_1`..`_9` in 2.7, …) as they land.
    era: String,
    /// Phase 4c: parallel array — one entry per token in `tokens`.
    /// `true` if there was at least one whitespace (or comment)
    /// character consumed between the previous emit and this one.
    /// Used by the era-fusion post-pass to distinguish source-level
    /// `&.` from source-level `& .` (with whitespace between) — the
    /// engine's per-token column tracking can't always tell them
    /// apart because peek-state operators report the *follower's*
    /// column rather than their own.
    whitespace_before_token: Vec<bool>,
    /// Phase 4c: scratch flag set on each whitespace consume and
    /// flushed into `whitespace_before_token` on the next emit.
    whitespace_pending: bool,
}

/// Phase 3c — a heredoc opener whose body we still owe.
#[derive(Debug, Clone)]
struct PendingHeredoc {
    /// The tag name (e.g. `EOF`).  The body ends at the first line
    /// whose entire content equals this tag (v0: no leading
    /// whitespace permitted — `<<-`/`<<~` indent modifiers arrive in
    /// Phase 3d).
    tag: String,
    /// Index in [`RubyLexer::tokens`] of the `<<` Op token.  When
    /// the body is captured we replace this token with the assembled
    /// `String` token carrying the verbatim heredoc source.
    op_idx: usize,
    /// Index in [`RubyLexer::tokens`] of the `Name` token that holds
    /// the tag lexeme.  Removed when the heredoc is finalized.
    tag_idx: usize,
    /// Accumulated body text — everything between the opener's
    /// trailing newline and the terminator line, including embedded
    /// newlines.
    body: String,
}

impl RubyLexer {
    /// Build a fresh lexer for the given Ruby version.  Phase 1
    /// only supports `"1.8"`.  Uses the default [`NoLocals`] oracle
    /// — see [`RubyLexer::with_oracle`] to wire in a real parser.
    pub fn new(version: &str) -> Result<Self, String> {
        Self::with_oracle(version, Box::new(NoLocals))
    }

    /// Build a fresh lexer with an explicit parser-feedback oracle.
    /// The oracle is consulted (via [`ParserOracle::is_local`]) when
    /// the lexer encounters a `/` after a name in `ExprArg` state to
    /// decide between regex-literal and division.
    pub fn with_oracle(
        version: &str,
        oracle: Box<dyn ParserOracle>,
    ) -> Result<Self, String> {
        let definition = machine::definition_for_version(version)?;
        let machine = EffectfulStateMachine::from_definition(&definition)
            .map_err(|e| format!("failed to build ruby lexer state machine: {e}"))?;
        let canonical_era = if version.is_empty() { "1.8" } else { version };
        Ok(Self {
            machine,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
            text_buffer: String::new(),
            line: 1,
            column: 1,
            token_start_line: 1,
            token_start_column: 1,
            lex_state: LexState::default(),
            last_name: String::new(),
            oracle,
            interp_brace_depth: 0,
            pending_heredocs: VecDeque::new(),
            heredoc_op_candidate: None,
            era: canonical_era.to_string(),
            whitespace_before_token: Vec::new(),
            whitespace_pending: false,
        })
    }

    /// Current lex-state.  Exposed for tests and tooling; the parser
    /// is the canonical driver of state changes (Phase 2 onward).
    pub fn lex_state(&self) -> LexState {
        self.lex_state
    }

    /// Feed the whole source into the lexer.
    ///
    /// Phase 3c: heredoc-aware.  Characters normally flow through the
    /// engine one at a time via `step_char`, but when a line-ending
    /// `\n` is fed and one or more `<<TAG` openers are pending, this
    /// method diverts the *next* lines straight into each pending
    /// heredoc body (bypassing the engine) until every terminator
    /// has been seen — only then does normal char-by-char lexing
    /// resume.  This is the cleanest way to implement Ruby's
    /// "line-based" heredoc semantics on top of a fundamentally
    /// character-based state machine.
    pub fn push(&mut self, source: &str) -> Result<(), String> {
        let chars: Vec<char> = source.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            self.step_char(ch)?;
            i += 1;
            if ch == '\n' && !self.pending_heredocs.is_empty() {
                i = self.capture_heredoc_bodies(&chars, i)?;
            }
        }
        Ok(())
    }

    /// Phase 3c body slurp.  Called after the `\n` that ends the
    /// opener line.  Reads whole lines from `chars[i..]`, appending
    /// each to the front pending heredoc's body until a line equal
    /// to its tag arrives (the terminator).  Terminators pop the
    /// heredoc; non-terminators extend the body.  Returns the new
    /// cursor position so the outer scan can resume normal lexing.
    ///
    /// Multi-heredoc FIFO example: `x = <<A; y = <<B\n` queues `A`
    /// then `B`.  After the `\n`, this method reads lines into A's
    /// body until a line "A" terminates it, then into B's body
    /// until "B" terminates it.
    fn capture_heredoc_bodies(
        &mut self,
        chars: &[char],
        mut i: usize,
    ) -> Result<usize, String> {
        let mut finalized: Vec<PendingHeredoc> = Vec::new();
        while !self.pending_heredocs.is_empty() {
            if i >= chars.len() {
                // Unterminated heredoc — record a diagnostic, then
                // finalize each pending entry with whatever body we
                // managed to collect so the token stream still
                // reflects user intent.
                self.diagnostics.push(Diagnostic {
                    code: "unterminated-heredoc".to_string(),
                    line: self.line,
                    column: self.column,
                });
                while let Some(h) = self.pending_heredocs.pop_front() {
                    finalized.push(h);
                }
                break;
            }
            // Read one line (up to and including `\n`, or to EOF).
            let line_start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            let line: String = chars[line_start..i].iter().collect();
            let had_newline = i < chars.len() && chars[i] == '\n';
            if had_newline {
                i += 1;
                self.line += 1;
                self.column = 1;
            }

            // Terminator check uses exact-equality (v0): no leading
            // whitespace tolerance.  `<<-`/`<<~` arrive in Phase 3d.
            let front_tag = self.pending_heredocs.front().unwrap().tag.clone();
            if line == front_tag {
                let h = self.pending_heredocs.pop_front().unwrap();
                finalized.push(h);
            } else {
                let front_mut = self.pending_heredocs.front_mut().unwrap();
                front_mut.body.push_str(&line);
                if had_newline {
                    front_mut.body.push('\n');
                }
            }
        }
        // Apply token replacements in descending order so earlier
        // indices remain valid as later tokens are removed.
        finalized.sort_by_key(|h| std::cmp::Reverse(h.op_idx));
        for h in finalized {
            self.finalize_heredoc(h);
        }
        Ok(i)
    }

    /// Splice a captured heredoc into the token stream.  The `<<` Op
    /// token at `h.op_idx` becomes a `String` token carrying the
    /// verbatim heredoc source (`<<TAG\n<body>TAG`); the following
    /// `Name` token at `h.tag_idx` (the tag lexeme) is removed.
    fn finalize_heredoc(&mut self, h: PendingHeredoc) {
        let value = format!("<<{}\n{}{}", h.tag, h.body, h.tag);
        // Remove the tag Name first (higher index) so op_idx stays
        // valid.  Both indices were captured at emit time and the
        // tokens between them have not moved (body capture inserts
        // no tokens).
        if h.tag_idx < self.tokens.len() {
            self.tokens.remove(h.tag_idx);
        }
        if h.op_idx < self.tokens.len() {
            let line = self.tokens[h.op_idx].line;
            let column = self.tokens[h.op_idx].column;
            self.tokens[h.op_idx] = Token {
                type_: TokenType::String,
                value,
                line,
                column,
                type_name: None,
                flags: None,
            };
        }
    }

    /// Signal end-of-input.  Drains any pending state (some peek
    /// states need one or two EOF events to fully flush their
    /// accumulators), emits an EOF token, then applies era-specific
    /// token-stream fusions (Phase 4b+).
    pub fn finish(&mut self) -> Result<(), String> {
        const MAX_DRAIN: usize = 32;
        for _ in 0..MAX_DRAIN {
            if self.machine.is_final() {
                self.apply_era_token_fusions();
                return Ok(());
            }
            let step = self
                .machine
                .process(EffectfulInput::end())
                .map_err(|e| format!("ruby lexer drain error: {e}"))?;
            self.apply_effects(&step.effects, None)?;
        }
        Err(format!(
            "ruby lexer did not reach final state within {MAX_DRAIN} drain iterations"
        ))
    }

    /// Phase 4b — apply era-gated token-stream rewrites.
    ///
    /// The 1.8 baseline state machine is deliberately conservative —
    /// it emits the minimal token shapes that every Ruby era agrees
    /// on, and leaves the era-specific *combinations* (lambda `->`,
    /// safe-nav `&.`, …) for this post-pass to fold in.  This keeps
    /// the TOML state machine a single source of truth and lets us
    /// add new era deltas without forking the (large) TOML file.
    ///
    /// ## v0 fusions
    ///
    /// - `1.9.1+`: adjacent `Op("-")` + `Op(">")` (same line, no gap)
    ///   fuses into `Op("->")` — the lambda literal opener.  The 1.8
    ///   era keeps them as two tokens so `f -> g` parses as
    ///   subtraction followed by `>` (Ruby 1.8 doesn't know about
    ///   lambda literals).
    /// - `2.3+`: adjacent `Op("&")` + `Dot(".")` fuses into a single
    ///   `Op("&.")` — the safe-navigation operator.  `a&.b` calls
    ///   `b` on `a` if `a` is non-nil, otherwise short-circuits to
    ///   `nil`.  Pre-2.3 eras keep them as two tokens so `a & .b`
    ///   stays parseable as bitwise-AND + (a stray) dot.
    ///
    /// Future eras will plug in here without touching the
    /// state-machine TOML.
    fn apply_era_token_fusions(&mut self) {
        if era_at_least(&self.era, "1.9.1") {
            self.fuse_lambda_arrow();
        }
        if era_at_least(&self.era, "2.3") {
            self.fuse_safe_nav();
        }
        if era_at_least(&self.era, "2.7") {
            self.mark_numbered_block_params();
        }
    }

    /// 2.7: tag Name tokens whose lexeme is `_1` through `_9` with
    /// flag bit [`NUMBERED_BLOCK_PARAM_FLAG`] so the parser/SIR
    /// frontend can identify them as implicit block parameters.
    ///
    /// In Ruby 2.7+, an identifier of the form `_<digit>` inside a
    /// block body refers to that ordinal positional argument: `_1`
    /// is the first, `_2` the second, and so on through `_9`.
    /// Pre-2.7 these are just regular local variables.  The lexer
    /// can't tell whether a given `_1` is *actually* inside a block
    /// (that's parser-level context), but it can flag every `_N`
    /// lexeme as a *candidate* numbered-param so downstream
    /// consumers can apply the era-aware semantics without
    /// re-scanning the token stream.
    fn mark_numbered_block_params(&mut self) {
        for tok in &mut self.tokens {
            if tok.type_ == TokenType::Name && is_numbered_block_param(&tok.value) {
                let prev = tok.flags.unwrap_or(0);
                tok.flags = Some(prev | NUMBERED_BLOCK_PARAM_FLAG);
            }
        }
    }

    /// 2.3: fold adjacent `Op("&")` + `Dot(".")` tokens into a
    /// single `Op("&.")` (safe-nav opener).  Same adjacency
    /// heuristic as `fuse_lambda_arrow` — single-char operators
    /// like `&` are emitted on the follower character, so the `&`
    /// token's recorded column can be up to 2 ahead of the source
    /// position.
    fn fuse_safe_nav(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_amp = a.value == "&" && matches!(a.type_, TokenType::Name);
                let b_is_dot = b.type_ == TokenType::Dot && b.value == ".";
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_amp && b_is_dot && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "&.".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None,
                };
            }
            i += 1;
        }
    }

    /// 1.9.1: fold adjacent `Op("-")` + `Op(">")` tokens into a
    /// single `Op("->")` (lambda opener) when they were emitted
    /// without any whitespace between them.
    ///
    /// Adjacency note: single-char operators like `>` are emitted by
    /// the engine on the *follower* character (the engine peeks one
    /// char ahead to disambiguate `>` vs `>=`), so the `>` token's
    /// recorded `column` is the column of that follower, not the
    /// source position of `>` itself.  In practice this leaves a 1-
    /// or 2-column gap between the `-` and the `>` tokens even
    /// though there's no source whitespace.  We allow up to 2
    /// columns of "virtual gap" so the adjacency check is robust
    /// against this quirk without accidentally matching real
    /// whitespace-separated `-` `>` sequences (where the gap is
    /// strictly ≥ 3 — `-` at col N, space at N+1, `>` token emitted
    /// from col N+3).
    fn fuse_lambda_arrow(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_minus =
                    a.type_ == TokenType::Minus && a.value == "-";
                let b_is_gt = b.value == ">"
                    && matches!(b.type_, TokenType::Name);
                let same_line = a.line == b.line;
                // Phase 4c: require that no whitespace was consumed
                // between the two tokens — the parallel array tracks
                // this explicitly so we don't have to interpret
                // peek-state column quirks.
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_minus && b_is_gt && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "->".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None,
                };
            }
            i += 1;
        }
    }

    /// Take ownership of all tokens emitted so far.
    pub fn drain_tokens(&mut self) -> Vec<Token> {
        std::mem::take(&mut self.tokens)
    }

    /// Non-fatal diagnostics recorded during lexing.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Step the machine on one character.  Honours `consume = false`
    /// transitions by re-feeding the same character to the new
    /// state (capped — a state machine that ping-pongs without
    /// consuming input is a bug).
    fn step_char(&mut self, ch: char) -> Result<(), String> {
        // Phase 2: `/` disambiguation.  Before the state machine
        // sees the slash, we may need to manually divert it into
        // the `regex_body` sub-machine (which the action interpreter
        // entered "by hand", outside the engine's transition table).
        // This keeps the TOML state machine simple — it always
        // emits `/` as a division operator from `data`, and the
        // interpreter rewrites that to a regex when context says so.
        if ch == '/' && self.machine.current_state() == "data" && self.should_open_regex() {
            self.machine
                .set_current_state("regex_body")
                .map_err(|e| format!("ruby lexer: failed to enter regex_body: {e}"))?;
            // Clear the text buffer so the regex body accumulates
            // cleanly; record the source position so the emitted
            // Regex token's line/column point at the opening `/`.
            self.text_buffer.clear();
            self.token_start_line = self.line;
            self.token_start_column = self.column;
            // The `/` is consumed but emits no token — it's part of
            // the regex literal.  Advance position past it.
            self.column += 1;
            return Ok(());
        }

        // Phase 3b: `}` at brace-depth 0 inside `string_d_interp`
        // closes the `#{...}` interpolation.  Append the `}` to the
        // string body and force the engine back to `string_d_body`.
        // Inner `{` / `}` increment / decrement the depth and pass
        // through to the engine's normal `append_text(current)` arm.
        if self.machine.current_state() == "string_d_interp" {
            if ch == '{' {
                self.interp_brace_depth += 1;
            } else if ch == '}' {
                if self.interp_brace_depth > 0 {
                    self.interp_brace_depth -= 1;
                } else {
                    // Closing brace of the interpolation.  Append
                    // `}` to the string body and pop back to
                    // string_d_body.  Note: we DO NOT feed `}` to
                    // the engine — the engine's `anything` arm
                    // would re-enter string_d_interp and keep
                    // accumulating.
                    self.text_buffer.push('}');
                    self.machine
                        .set_current_state("string_d_body")
                        .map_err(|e| {
                            format!("ruby lexer: failed to leave string_d_interp: {e}")
                        })?;
                    self.column += 1;
                    return Ok(());
                }
            }
        }

        let mut buf = [0u8; 4];
        let event = ch.encode_utf8(&mut buf);
        const MAX_REENTRY: usize = 8;
        for _ in 0..MAX_REENTRY {
            let step = self
                .machine
                .process(EffectfulInput::event(event))
                .map_err(|e| format!("ruby lexer error at {}:{}: {e}", self.line, self.column))?;
            self.apply_effects(&step.effects, Some(ch))?;
            if step.consume {
                // Phase 4c: flag whitespace consumes so the next
                // emitted token records that whitespace preceded it.
                // The era-fusion post-pass uses this signal to keep
                // source-level `&.` distinct from source-level `& .`
                // — the per-token column tracking alone can't always
                // tell them apart because peek-state operators
                // report the follower's column.
                if ch == ' ' || ch == '\t' {
                    self.whitespace_pending = true;
                }
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                return Ok(());
            }
        }
        Err(format!(
            "ruby lexer ping-pong at {}:{} — no transition consumed input",
            self.line, self.column
        ))
    }

    /// Decide whether a `/` at the current position opens a regex
    /// literal.  Folds the lex-state and oracle answer together:
    ///
    /// - `ExprBeg` / `ExprMid` / (default state at file start) → regex.
    /// - `ExprEnd` → division (the slash follows a value).
    /// - `ExprArg` → ask the oracle whether the preceding name is a
    ///   local variable.  Local → division (because we're dividing
    ///   the local's value).  Not-local → regex (the name was a
    ///   method call and the regex is its argument).
    fn should_open_regex(&self) -> bool {
        if self.lex_state.slash_is_regex() {
            return true;
        }
        if self.lex_state.needs_oracle_for_slash() {
            return !self.oracle.is_local(&self.last_name);
        }
        false
    }

    fn apply_effects(&mut self, effects: &[String], current: Option<char>) -> Result<(), String> {
        for raw in effects {
            self.apply_one_effect(raw, current)?;
        }
        Ok(())
    }

    fn apply_one_effect(&mut self, raw: &str, current: Option<char>) -> Result<(), String> {
        let (verb, arg) = split_verb_arg(raw);
        match verb {
            "clear_text" => {
                self.text_buffer.clear();
                self.token_start_line = self.line;
                self.token_start_column = self.column;
            }
            "set_text" => {
                self.text_buffer.clear();
                if let Some(a) = arg {
                    self.text_buffer.push_str(a);
                }
                self.token_start_line = self.line;
                self.token_start_column = self.column;
            }
            "append_text" => {
                let a = arg.ok_or_else(|| "append_text requires an argument".to_string())?;
                if a == "current" {
                    if let Some(c) = current {
                        if self.text_buffer.is_empty() {
                            self.token_start_line = self.line;
                            self.token_start_column = self.column;
                        }
                        self.text_buffer.push(c);
                    }
                } else {
                    self.text_buffer.push_str(&decode_action_literal(a));
                }
            }
            "emit" => {
                let kind = arg.ok_or_else(|| {
                    "emit requires a token-kind argument".to_string()
                })?;
                self.emit_token_by_name(kind);
            }
            "parse_error" => {
                self.diagnostics.push(Diagnostic {
                    code: arg.unwrap_or("unknown").to_string(),
                    line: self.line,
                    column: self.column,
                });
            }
            other => {
                return Err(format!("ruby lexer: unknown action verb `{other}`"));
            }
        }
        Ok(())
    }

    fn emit_token_by_name(&mut self, kind_name: &str) {
        // Phase 3c: snapshot the lex_state and the prior heredoc
        // candidate before `push_token` mutates self.  We consult
        // `prev_state` to decide whether a `<<` opener is at an
        // expression-start position; `prior_candidate` is what an
        // immediately-preceding `<<` set up for *this* emit to
        // potentially consume as a tag.
        let prev_state = self.lex_state;
        let prior_candidate = self.heredoc_op_candidate.take();
        match kind_name {
            "Eof" => self.push_token(TokenType::Eof, String::new()),
            "Newline" => self.push_token(TokenType::Newline, "\n".to_string()),
            "LParen" => self.push_token(TokenType::LParen, "(".to_string()),
            "RParen" => self.push_token(TokenType::RParen, ")".to_string()),
            "LBracket" => self.push_token(TokenType::LBracket, "[".to_string()),
            "RBracket" => self.push_token(TokenType::RBracket, "]".to_string()),
            "LBrace" => self.push_token(TokenType::LBrace, "{".to_string()),
            "RBrace" => self.push_token(TokenType::RBrace, "}".to_string()),
            "Comma" => self.push_token(TokenType::Comma, ",".to_string()),
            "Semi" => self.push_token(TokenType::Semicolon, ";".to_string()),
            "Colon" => self.push_token(TokenType::Colon, ":".to_string()),
            "ColonColon" => {
                // No dedicated TokenType for `::` yet — encode as
                // Colon with value `::` so the parser can dispatch
                // by value.
                self.push_token(TokenType::Colon, "::".to_string());
            }
            "Dot" => self.push_token(TokenType::Dot, ".".to_string()),
            "Int" => {
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::Number, text);
            }
            "String" => {
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::String, text);
            }
            "Name" => {
                let text = std::mem::take(&mut self.text_buffer);
                let kind = classify_name_token(&text);
                // Phase 2: record this name so the `/` interceptor
                // can ask the oracle "is this a local?".  We record
                // it regardless of whether it's a keyword — the
                // lex_state transition table handles the keyword case
                // by routing past `ExprArg`.
                self.last_name = text.clone();
                self.push_token(kind, text.clone());
                // Phase 3c: if the immediately-preceding emit was a
                // `<<` Op at expression-start, this Name's lexeme is
                // the heredoc tag.  Queue the opener (FIFO) — the
                // body will be slurped after the line's `\n`.  Keep
                // both the Op and Name tokens in the stream for now;
                // `finalize_heredoc` rewrites the Op into a String
                // and removes this Name once the body is captured.
                if let Some(op_idx) = prior_candidate {
                    // The engine emitted a "Name" event, which the
                    // interpreter may have classified as `Keyword`
                    // (e.g. `<<END` — `END` is the reserved at-exit
                    // hook).  Either kind is a valid heredoc tag —
                    // we only care that the lexeme shape is an
                    // identifier.
                    let lexeme_kind_ok =
                        matches!(kind, TokenType::Name | TokenType::Keyword);
                    if lexeme_kind_ok && is_heredoc_tag(&text) {
                        let tag_idx = self.tokens.len() - 1;
                        self.pending_heredocs.push_back(PendingHeredoc {
                            tag: text,
                            op_idx,
                            tag_idx,
                            body: String::new(),
                        });
                    }
                }
            }
            "Regex" => {
                // Phase 3a shape: the text buffer holds `body/`
                // (when no flags follow) or `body/flags` (when one
                // or more flag letters were slurped in
                // `regex_flags`).  The closing `/` is appended to
                // the buffer by the `regex_body → regex_flags`
                // transition, so we only need to prepend the
                // leading `/` here to get the verbatim source-shape
                // of the literal.
                let text = std::mem::take(&mut self.text_buffer);
                let value = format!("/{}", text);
                self.push_token(TokenType::String, value);
            }
            "PercentW" => {
                // %w[a b c] — string-array literal.  The text buffer
                // contains the verbatim source `%w[a b c]` (including
                // the `%w[` opener and `]` closer).  Encode as
                // TokenType::String with the verbatim value; the
                // parser inspects the lexeme prefix to know it's a
                // string array.
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::String, text);
            }
            "PercentQ" => {
                // %q{single-quoted-style body} — non-interpolating
                // string.  Same encoding strategy as PercentW.
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::String, text);
            }
            "Op" => {
                let text = std::mem::take(&mut self.text_buffer);
                let kind = classify_op_token(&text);
                let is_heredoc_open = text == "<<"
                    && matches!(prev_state, LexState::ExprBeg | LexState::ExprMid);
                self.push_token(kind, text);
                // Phase 3c: arm the heredoc detector.  The very next
                // emitted token is examined as a possible tag (see
                // the `"Name"` arm above).  If it isn't a Name, the
                // `prior_candidate` snapshot at the top of this
                // function will clear it naturally — no extra
                // bookkeeping needed because we `take()` the field.
                if is_heredoc_open {
                    self.heredoc_op_candidate = Some(self.tokens.len() - 1);
                }
            }
            other => {
                self.diagnostics.push(Diagnostic {
                    code: format!("unknown-emit-kind:{}", other),
                    line: self.line,
                    column: self.column,
                });
            }
        }
    }

    fn push_token(&mut self, type_: TokenType, value: String) {
        // Phase 2: advance lex-state based on the token we are
        // about to emit.  Order matters — we transition *before*
        // pushing so the next `/` interceptor sees the up-to-date
        // state.
        self.lex_state = next_lex_state(self.lex_state, type_, &value);
        // Phase 4c: snapshot whether whitespace was consumed since
        // the last emit, then clear the flag.  The era-fusion
        // post-pass reads this parallel array to keep source-level
        // `&.` distinct from `& .`.
        let had_ws = self.whitespace_pending;
        self.whitespace_pending = false;
        self.tokens.push(Token {
            type_,
            value,
            line: self.token_start_line,
            column: self.token_start_column,
            type_name: None,
            flags: None,
        });
        self.whitespace_before_token.push(had_ws);
        // Reset start position so the next immediate-emit token
        // (e.g. an `LParen` right after a `Name`) gets the current
        // source position, not the prior token's start.
        self.token_start_line = self.line;
        self.token_start_column = self.column;
    }
}

/// Lex-state transition table.  Given the prior state, the kind of
/// token just emitted, and its lexeme, return the new state.
///
/// The rules are simplified for v0:
///   - Keywords get fine-grained transitions (e.g. `def` → `ExprFname`).
///   - Names go to `ExprArg` (might be a call without parens).
///   - Value-yielding atoms (`Int`, `String`, closing brackets) → `ExprEnd`.
///   - Binary operators / `=` / opening brackets / `,` / `;` /
///     newline / most keywords → `ExprBeg` (or `ExprMid`).
///   - `.` → `ExprDot`.
///   - `Eof` doesn't change state.
fn next_lex_state(prev: LexState, kind: TokenType, value: &str) -> LexState {
    use TokenType::*;
    match (kind, value) {
        (Keyword, "def") | (Keyword, "class") | (Keyword, "module") | (Keyword, "alias") | (Keyword, "undef") => {
            LexState::ExprFname
        }
        (Keyword, "self") | (Keyword, "nil") | (Keyword, "true") | (Keyword, "false") => LexState::ExprEnd,
        (Keyword, _) => LexState::ExprBeg,
        (Name, _) => LexState::ExprArg,
        (Number, _) | (String, _) => LexState::ExprEnd,
        (RParen, _) | (RBracket, _) | (RBrace, _) => LexState::ExprEnd,
        (LParen, _) | (LBracket, _) | (LBrace, _) | (Comma, _) | (Semicolon, _) | (Newline, _) => {
            LexState::ExprBeg
        }
        (Plus, _) | (Minus, _) | (Star, _) | (Slash, _) | (Equals, _) | (EqualsEquals, _) | (Bang, _) => {
            LexState::ExprMid
        }
        (Dot, _) => LexState::ExprDot,
        // Colon may be a hash separator or `::` — both treat the
        // following position as expression-start.  Symbols like
        // `:foo` are not yet handled at the lexer level (Phase 3).
        (Colon, _) => LexState::ExprBeg,
        (Eof, _) => prev,
        _ => prev,
    }
}

/// Parse `verb(arg)` or `verb` into `(verb, Some(arg))` / `(verb, None)`.
fn split_verb_arg(s: &str) -> (&str, Option<&str>) {
    if let Some(open) = s.find('(') {
        if let Some(close) = s.rfind(')') {
            if close > open {
                return (&s[..open], Some(&s[open + 1..close]));
            }
        }
    }
    (s, None)
}

/// Decode `\n`, `\t`, `\r`, `\\`, `\"`, `\'` inside an action-string
/// argument.  Anything else passes through with the backslash.
fn decode_action_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Identifiers that are reserved keywords vs ordinary names.
fn classify_name_token(text: &str) -> TokenType {
    if is_ruby_keyword(text) {
        TokenType::Keyword
    } else {
        TokenType::Name
    }
}

/// Map operator lexemes to their dedicated `TokenType` where one
/// exists.  Operators without a dedicated kind (`!=`, `<=`, `>=`,
/// `&&`, `||`, `=>`, `**`, etc.) land on `TokenType::Name` with the
/// lexeme preserved in `value` — the parser dispatches by value.
fn classify_op_token(text: &str) -> TokenType {
    match text {
        "+" => TokenType::Plus,
        "-" => TokenType::Minus,
        "*" => TokenType::Star,
        "/" => TokenType::Slash,
        "=" => TokenType::Equals,
        "==" => TokenType::EqualsEquals,
        "!" => TokenType::Bang,
        _ => TokenType::Name,
    }
}

/// Phase 3c: is `s` a valid heredoc tag?
///
/// v0 rule: non-empty identifier (ASCII letters / digits / underscore,
/// not starting with a digit).  Ruby itself permits any identifier
/// shape here — the heredoc-vs-shift decision is driven purely by
/// the expression-start context that the call site already checked.
/// Phase 4b — true iff `era` is the same as or newer than `min`.
///
/// Compares the two era strings against
/// [`machine::ERA_VERSIONS`] (chronological order).  Unknown era
/// strings sort as the *baseline* `1.8` to match the lexer's
/// default — that way a misconfigured caller still gets the
/// conservative pre-1.9.1 behaviour rather than silently enabling
/// modern syntax fusions.
fn era_at_least(era: &str, min: &str) -> bool {
    let normalise = |v: &str| -> usize {
        let needle = if v.is_empty() { "1.8" } else { v };
        machine::ERA_VERSIONS
            .iter()
            .position(|&e| e == needle)
            .unwrap_or_else(|| {
                // Unknown era — fall back to the 1.8 baseline index
                // so era-gated fusions stay disabled.
                machine::ERA_VERSIONS
                    .iter()
                    .position(|&e| e == "1.8")
                    .unwrap_or(0)
            })
    };
    normalise(era) >= normalise(min)
}

/// Phase 4d — flag bit set on the `Token.flags` field for Name
/// tokens that match the `_<digit>` numbered-block-param pattern
/// (Ruby 2.7+).  Downstream tooling can check
/// `(tok.flags.unwrap_or(0) & NUMBERED_BLOCK_PARAM_FLAG) != 0` to
/// dispatch on numbered-param semantics.  Higher bits are reserved
/// for future era flags.
pub const NUMBERED_BLOCK_PARAM_FLAG: u32 = 1 << 0;

/// Phase 4d — true if `s` is `_1`, `_2`, …, `_9` exactly.  These
/// are the only nine numbered block parameter lexemes Ruby 2.7+
/// recognises.  `_0`, `_10`, `_1abc` etc. are NOT numbered params
/// and lex as regular Name tokens.
fn is_numbered_block_param(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 2 && bytes[0] == b'_' && (b'1'..=b'9').contains(&bytes[1])
}

fn is_heredoc_tag(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_ruby_keyword(s: &str) -> bool {
    matches!(
        s,
        "BEGIN"
            | "END"
            | "alias"
            | "and"
            | "begin"
            | "break"
            | "case"
            | "class"
            | "def"
            | "defined?"
            | "do"
            | "else"
            | "elsif"
            | "end"
            | "ensure"
            | "false"
            | "for"
            | "if"
            | "in"
            | "module"
            | "next"
            | "nil"
            | "not"
            | "or"
            | "redo"
            | "rescue"
            | "retry"
            | "return"
            | "self"
            | "super"
            | "then"
            | "true"
            | "undef"
            | "unless"
            | "until"
            | "when"
            | "while"
            | "yield"
    )
}

// ---------------------------------------------------------------------------
// Convenience entry points — preserve the prior public surface so
// `ruby-parser` keeps working without changes.
// ---------------------------------------------------------------------------

/// Tokenize Ruby source (Phase-1 = Ruby 1.8 lexer).  Returns the
/// EOF-terminated token list.  Diagnostics are dropped silently;
/// call [`tokenize_ruby_diag`] to inspect them.
pub fn tokenize_ruby(source: &str) -> Vec<Token> {
    tokenize_ruby_diag(source).0
}

/// Phase 4 — tokenize against a specific Ruby era.  `version` must
/// be one of the strings in [`ERA_VERSIONS`] (or `""` for the
/// default 1.8 baseline).  Returns an error if the version is not
/// recognized; on success the behaviour matches [`tokenize_ruby`]
/// for that era.
///
/// v0 caveat: all 15 eras currently share the 1.8 token grammar —
/// only the underlying state-machine `name` differs.  Era-specific
/// deltas (e.g. lambda `->` in 1.9.1, safe-nav `&.` in 2.3) arrive
/// in subsequent phases.  Callers that need version gating today
/// can already plumb the era string through to downstream tooling.
pub fn tokenize_ruby_for_version(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let mut lexer = RubyLexer::new(version)?;
    lexer.push(source)?;
    lexer.finish()?;
    Ok(lexer.drain_tokens())
}

/// Tokenize with a custom [`ParserOracle`].  Use this when the
/// caller already has a local-variable scope to consult — most
/// notably the parser, which collects locals as it walks the
/// source.
pub fn tokenize_ruby_with_oracle(
    source: &str,
    oracle: Box<dyn ParserOracle>,
) -> Vec<Token> {
    let mut lexer = RubyLexer::with_oracle("1.8", oracle).expect("ruby 1.8 lexer definition");
    if lexer.push(source).is_err() {
        return lexer.drain_tokens();
    }
    if lexer.finish().is_err() {
        return lexer.drain_tokens();
    }
    lexer.drain_tokens()
}

/// Same as [`tokenize_ruby`] but also returns recorded diagnostics.
pub fn tokenize_ruby_diag(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lexer = RubyLexer::new("1.8").expect("ruby 1.8 lexer definition");
    if let Err(e) = lexer.push(source) {
        let mut diags = lexer.diagnostics.clone();
        diags.push(Diagnostic {
            code: format!("lex-error:{e}"),
            line: lexer.line,
            column: lexer.column,
        });
        return (lexer.drain_tokens(), diags);
    }
    if let Err(e) = lexer.finish() {
        let mut diags = lexer.diagnostics.clone();
        diags.push(Diagnostic {
            code: format!("finish-error:{e}"),
            line: lexer.line,
            column: lexer.column,
        });
        return (lexer.drain_tokens(), diags);
    }
    let tokens = lexer.drain_tokens();
    let diags = lexer.diagnostics.clone();
    (tokens, diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pull `(type_, value)` pairs out of a token list, omitting EOF.
    fn pairs(toks: &[Token]) -> Vec<(TokenType, &str)> {
        toks.iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    #[test]
    fn empty_source_just_eof() {
        let toks = tokenize_ruby("");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].type_, TokenType::Eof);
    }

    #[test]
    fn single_identifier() {
        let toks = tokenize_ruby("foo");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Name, "foo")]);
    }

    #[test]
    fn keyword_vs_name() {
        let toks = tokenize_ruby("def foo");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Keyword, "def"),
                (TokenType::Name, "foo"),
            ]
        );
    }

    #[test]
    fn integer_literal() {
        let toks = tokenize_ruby("42");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Number, "42")]);
    }

    #[test]
    fn integer_with_underscores() {
        let toks = tokenize_ruby("1_000_000");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Number, "1_000_000")]);
    }

    #[test]
    fn double_quoted_string() {
        let toks = tokenize_ruby(r#""hello""#);
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "hello")]);
    }

    #[test]
    fn single_quoted_string() {
        let toks = tokenize_ruby("'hello'");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "hello")]);
    }

    #[test]
    fn string_escapes() {
        let toks = tokenize_ruby(r#""line1\nline2""#);
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "line1\nline2")]);
    }

    #[test]
    fn line_comment_skipped() {
        let toks = tokenize_ruby("# this is a comment\nfoo\n");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Newline, "\n"),
                (TokenType::Name, "foo"),
                (TokenType::Newline, "\n"),
            ]
        );
    }

    #[test]
    fn binary_operators_dispatch_to_dedicated_kinds() {
        // Phase 2: with the NoLocals default oracle, `d / e` would
        // lex as a regex (because `d` is treated as a method call).
        // To exercise binary `/` we declare the operands as locals
        // via a `StaticLocals` oracle.
        let oracle = Box::new(StaticLocals::with_locals(["a", "b", "c", "d", "e"]));
        let toks = tokenize_ruby_with_oracle("a + b - c * d / e", oracle);
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "a"),
                (TokenType::Plus, "+"),
                (TokenType::Name, "b"),
                (TokenType::Minus, "-"),
                (TokenType::Name, "c"),
                (TokenType::Star, "*"),
                (TokenType::Name, "d"),
                (TokenType::Slash, "/"),
                (TokenType::Name, "e"),
            ]
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 2 — lex_state + ParserOracle wiring
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn lex_state_starts_at_expr_beg() {
        let lexer = RubyLexer::new("1.8").expect("lexer");
        assert_eq!(lexer.lex_state(), LexState::ExprBeg);
    }

    #[test]
    fn lex_state_after_name_is_expr_arg() {
        let mut lexer = RubyLexer::new("1.8").expect("lexer");
        lexer.push("foo").unwrap();
        lexer.finish().unwrap();
        assert_eq!(lexer.lex_state(), LexState::ExprArg);
    }

    #[test]
    fn lex_state_after_int_is_expr_end() {
        let mut lexer = RubyLexer::new("1.8").expect("lexer");
        lexer.push("42").unwrap();
        lexer.finish().unwrap();
        assert_eq!(lexer.lex_state(), LexState::ExprEnd);
    }

    #[test]
    fn lex_state_after_binary_op_is_expr_mid() {
        let mut lexer = RubyLexer::new("1.8").expect("lexer");
        lexer.push("1 + ").unwrap();
        lexer.finish().unwrap();
        assert_eq!(lexer.lex_state(), LexState::ExprMid);
    }

    #[test]
    fn lex_state_after_def_keyword_is_expr_fname() {
        let mut lexer = RubyLexer::new("1.8").expect("lexer");
        lexer.push("def").unwrap();
        lexer.finish().unwrap();
        assert_eq!(lexer.lex_state(), LexState::ExprFname);
    }

    #[test]
    fn lex_state_after_dot_is_expr_dot() {
        let mut lexer = RubyLexer::new("1.8").expect("lexer");
        lexer.push("obj.").unwrap();
        lexer.finish().unwrap();
        assert_eq!(lexer.lex_state(), LexState::ExprDot);
    }

    #[test]
    fn slash_at_start_of_expression_is_regex() {
        // No preceding token → lex_state == ExprBeg → `/x/` is regex.
        let toks = tokenize_ruby("/foo/");
        let p = pairs(&toks);
        // Regex literal encoded as a String token with `/.../` shape.
        assert_eq!(p, vec![(TokenType::String, "/foo/")]);
    }

    #[test]
    fn slash_after_binary_op_is_regex() {
        // `1 + /foo/` — `/` follows `+`, which leaves lex_state in
        // ExprMid → regex.
        let toks = tokenize_ruby("1 + /foo/");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Number, "1"),
                (TokenType::Plus, "+"),
                (TokenType::String, "/foo/"),
            ]
        );
    }

    #[test]
    fn slash_after_int_is_division() {
        // `1 / 2` — lex_state after `1` is ExprEnd → division.
        let toks = tokenize_ruby("1 / 2");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Number, "1"),
                (TokenType::Slash, "/"),
                (TokenType::Number, "2"),
            ]
        );
    }

    #[test]
    fn slash_after_method_name_is_regex_via_default_oracle() {
        // With the NoLocals default (`f` is not a known local), the
        // spec says treat every name as a method — so `f /x/` is a
        // method call with a regex argument.
        let toks = tokenize_ruby("f /x/");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![(TokenType::Name, "f"), (TokenType::String, "/x/")]
        );
    }

    #[test]
    fn slash_after_local_name_is_division() {
        // Oracle declares `f` as a local — `f /x/` is `f / x /`.
        let oracle = Box::new(StaticLocals::with_locals(["f", "x"]));
        let toks = tokenize_ruby_with_oracle("f /x/", oracle);
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "f"),
                (TokenType::Slash, "/"),
                (TokenType::Name, "x"),
                (TokenType::Slash, "/"),
            ]
        );
    }

    #[test]
    fn regex_with_backslash_escape() {
        // `/\d+/` — body contains `\d+`; the lexer preserves both
        // the backslash and the `d`.
        let toks = tokenize_ruby(r"/\d+/");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, r"/\d+/")]);
    }

    #[test]
    fn regex_with_escaped_slash() {
        // `/a\/b/` — the body has an escaped `/`, so the literal
        // doesn't terminate at the inner slash.
        let toks = tokenize_ruby(r"/a\/b/");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, r"/a\/b/")]);
    }

    #[test]
    fn static_locals_oracle_supports_dynamic_inserts() {
        let mut locals = StaticLocals::new();
        locals.insert("counter");
        // Wrap in Box for the convenience entry.
        let toks = tokenize_ruby_with_oracle("counter / 2", Box::new(locals));
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "counter"),
                (TokenType::Slash, "/"),
                (TokenType::Number, "2"),
            ]
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 3a — regex flags + `%w[]` / `%q{}` percent literals
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn regex_with_single_flag() {
        let toks = tokenize_ruby("/foo/i");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "/foo/i")]);
    }

    #[test]
    fn regex_with_multiple_flags() {
        let toks = tokenize_ruby("/foo/imx");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "/foo/imx")]);
    }

    #[test]
    fn regex_with_uppercase_flag() {
        // Uppercase `I`, `M`, etc. are also recognised flag letters.
        let toks = tokenize_ruby("/foo/IM");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "/foo/IM")]);
    }

    #[test]
    fn regex_flag_slurp_is_greedy_then_splits() {
        // `/foo/i puts` → regex with `i` flag, then a separate `puts`
        // Name token.  Greedy flag matching exits on the space.
        let toks = tokenize_ruby("/foo/i puts");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::String, "/foo/i"),
                (TokenType::Name, "puts"),
            ]
        );
    }

    #[test]
    fn regex_without_flags_preserves_phase_2_behaviour() {
        // /\d+/ — no flag letter follows; emit `/\d+/`.
        let toks = tokenize_ruby(r"/\d+/");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, r"/\d+/")]);
    }

    #[test]
    fn percent_w_array_basic() {
        let toks = tokenize_ruby("%w[a b c]");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "%w[a b c]")]);
    }

    #[test]
    fn percent_w_array_with_newlines_inside_body() {
        let toks = tokenize_ruby("%w[a\n  b\n  c]");
        // The token preserves the verbatim body including the
        // newlines — the parser splits on whitespace later.
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["%w[a\n  b\n  c]"]);
    }

    #[test]
    fn percent_q_string_basic() {
        let toks = tokenize_ruby("%q{hello world}");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "%q{hello world}")]);
    }

    #[test]
    fn percent_q_string_with_quotes_inside() {
        // `%q{...}` preserves embedded `"` and `'` literally.
        let toks = tokenize_ruby(r#"%q{he said "hi"}"#);
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, r#"%q{he said "hi"}"#)]);
    }

    #[test]
    fn modulo_operator_still_works() {
        // `%` not followed by `w` / `q` is still the modulo operator.
        // `1 % 2` — there's no special letter following `%`, so it
        // falls back to Op.
        let toks = tokenize_ruby("1 % 2");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Number, "1"),
                (TokenType::Name, "%"),
                (TokenType::Number, "2"),
            ]
        );
        // (`%` lands on TokenType::Name with value "%" because the
        // existing op-classifier doesn't have a dedicated kind for
        // modulo.  Match Phase 1 behaviour.)
    }

    #[test]
    fn percent_w_then_method_call() {
        let toks = tokenize_ruby("%w[a b c].length");
        let values: Vec<&str> = toks.iter().map(|t| t.value.as_str()).collect();
        assert!(values.contains(&"%w[a b c]"));
        assert!(values.contains(&"."));
        assert!(values.contains(&"length"));
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 3b — string interpolation `"#{...}"`
    //
    // v0 captures the interpolation expression verbatim inside the
    // String token's value; the parser decides how to evaluate.
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn string_with_simple_interpolation() {
        let toks = tokenize_ruby("\"hello #{name}\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "hello #{name}")]);
    }

    #[test]
    fn string_with_expression_interpolation() {
        let toks = tokenize_ruby("\"sum is #{1+2}\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "sum is #{1+2}")]);
    }

    #[test]
    fn string_with_interpolation_at_start() {
        let toks = tokenize_ruby("\"#{x}!\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "#{x}!")]);
    }

    #[test]
    fn string_with_interpolation_at_end() {
        let toks = tokenize_ruby("\"hi #{name}\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "hi #{name}")]);
    }

    #[test]
    fn string_with_multiple_interpolations() {
        let toks = tokenize_ruby("\"a #{x} b #{y} c\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "a #{x} b #{y} c")]);
    }

    #[test]
    fn string_with_hash_but_no_brace_is_literal() {
        // `"a # b"` — the `#` is just a literal character because
        // it isn't followed by `{`.
        let toks = tokenize_ruby("\"a # b\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "a # b")]);
    }

    #[test]
    fn string_with_hash_at_end_is_literal() {
        let toks = tokenize_ruby("\"trailing #\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "trailing #")]);
    }

    #[test]
    fn string_interpolation_with_nested_braces() {
        // `"#{ {a: 1} }"` — the inner `{a: 1}` is part of the
        // interpolation expression, not a closing brace.  The brace
        // depth tracker handles the nesting.
        let toks = tokenize_ruby("\"#{ {a: 1} }\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "#{ {a: 1} }")]);
    }

    #[test]
    fn string_interpolation_with_method_call() {
        // `"#{arr.length}"` — interpolation containing a method call.
        let toks = tokenize_ruby("\"len = #{arr.length}\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "len = #{arr.length}")]);
    }

    #[test]
    fn single_quoted_string_does_not_interpolate() {
        // `'#{name}'` — single-quoted strings DO NOT interpolate.
        // The `#{...}` stays as literal text.
        let toks = tokenize_ruby("'#{name}'");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "#{name}")]);
    }

    #[test]
    fn string_interpolation_with_string_inside() {
        // `"#{ "inner" }"` — the brace tracker is string-agnostic,
        // so after `#{` we accumulate everything (including
        // embedded `"`s) until matching `}` at depth 0.  Result:
        // a single String with body `x #{"y"}`.
        let toks = tokenize_ruby("\"x #{\"y\"}\"");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "x #{\"y\"}")]);
    }

    #[test]
    fn comparison_operators_preserve_lexeme() {
        let toks = tokenize_ruby("a == b\nc != d\ne <= f\ng >= h");
        let p = pairs(&toks);
        assert!(p.contains(&(TokenType::EqualsEquals, "==")));
        // `!=`, `<=`, `>=` currently land on Name with value preserved.
        assert!(p.iter().any(|(_, v)| *v == "!="));
        assert!(p.iter().any(|(_, v)| *v == "<="));
        assert!(p.iter().any(|(_, v)| *v == ">="));
    }

    #[test]
    fn assignment_and_hash_rocket() {
        let toks = tokenize_ruby("x = 1\nh = { :a => 1 }");
        let p = pairs(&toks);
        assert!(p.iter().any(|(t, _)| *t == TokenType::Equals));
        assert!(p.iter().any(|(_, v)| *v == "=>"));
    }

    #[test]
    fn parens_and_punctuation() {
        let toks = tokenize_ruby("foo(1, 2)");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "foo"),
                (TokenType::LParen, "("),
                (TokenType::Number, "1"),
                (TokenType::Comma, ","),
                (TokenType::Number, "2"),
                (TokenType::RParen, ")"),
            ]
        );
    }

    #[test]
    fn factorial_program_tokenizes() {
        let src = "def factorial(n)\n  if n == 0\n    1\n  else\n    n * factorial(n - 1)\n  end\nend\n";
        let toks = tokenize_ruby(src);
        let kw: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Keyword)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(kw, vec!["def", "if", "else", "end", "end"]);
        // Two `factorial` identifiers (def site + recursive call).
        let factorial_count = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name && t.value == "factorial")
            .count();
        assert_eq!(factorial_count, 2);
    }

    #[test]
    fn method_name_with_query_suffix() {
        let toks = tokenize_ruby("empty?");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Name, "empty?")]);
    }

    #[test]
    fn method_name_with_bang_suffix() {
        let toks = tokenize_ruby("save!");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Name, "save!")]);
    }

    #[test]
    fn newline_is_significant() {
        let toks = tokenize_ruby("a\nb");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "a"),
                (TokenType::Newline, "\n"),
                (TokenType::Name, "b"),
            ]
        );
    }

    #[test]
    fn double_colon_emits_colon_colon_value() {
        let toks = tokenize_ruby("Foo::Bar");
        let cc: Vec<&str> = toks.iter().map(|t| t.value.as_str()).collect();
        assert!(cc.contains(&"::"));
    }

    #[test]
    fn determinism() {
        let src = "def f(x)\n  x + 1\nend\n";
        let a = tokenize_ruby(src);
        let b = tokenize_ruby(src);
        let av: Vec<(TokenType, String)> =
            a.iter().map(|t| (t.type_, t.value.clone())).collect();
        let bv: Vec<(TokenType, String)> =
            b.iter().map(|t| (t.type_, t.value.clone())).collect();
        assert_eq!(av, bv);
    }

    #[test]
    fn class_def_tokenizes() {
        let toks = tokenize_ruby("class Foo\n  def bar\n    1\n  end\nend\n");
        let kw: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Keyword)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(kw, vec!["class", "def", "end", "end"]);
    }

    // -----------------------------------------------------------------
    // Phase 3c — heredocs (`<<TAG\nbody\nTAG`).
    // -----------------------------------------------------------------

    #[test]
    fn heredoc_simple() {
        // `<<EOF` at expression-start, body slurped to terminator.
        let toks = tokenize_ruby("x = <<EOF\nhello\nworld\nEOF\n");
        let s: Vec<(TokenType, &str)> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect();
        assert_eq!(
            s,
            vec![
                (TokenType::Name, "x"),
                (TokenType::Equals, "="),
                (TokenType::String, "<<EOF\nhello\nworld\nEOF"),
                (TokenType::Newline, "\n"),
            ]
        );
    }

    #[test]
    fn heredoc_empty_body() {
        // Opener-line then terminator-line with no body content.
        let toks = tokenize_ruby("x = <<EOF\nEOF\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<EOF\nEOF"]);
    }

    #[test]
    fn heredoc_single_line_body() {
        let toks = tokenize_ruby("x = <<EOF\nhello\nEOF\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<EOF\nhello\nEOF"]);
    }

    #[test]
    fn heredoc_preserves_special_chars_in_body() {
        // The body is captured verbatim — interpolation syntax is
        // preserved as text (no `#{...}` expansion at the lexer level,
        // matching the Phase 3b precedent).  Use `<<MARKER` rather
        // than `<<END` because `END` is a Ruby keyword (paired with
        // `BEGIN` for top-level blocks), and a keyword-shaped tag
        // would land in the keyword branch of `classify_name_token` —
        // the action interpreter accepts either kind, but starting
        // with the unambiguous case here keeps the test focused on
        // the verbatim-body invariant.
        let toks = tokenize_ruby("x = <<MARKER\nhi #{name} bye\nMARKER\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<MARKER\nhi #{name} bye\nMARKER"]);
    }

    #[test]
    fn heredoc_with_keyword_shaped_tag() {
        // Ruby permits any identifier-shaped tag, including those
        // that double as keywords (`<<END` is the classic example —
        // `END { ... }` is the at-exit hook).  The action interpreter
        // queues the heredoc regardless of whether `classify_name_token`
        // categorized the tag as `Name` or `Keyword`.
        let toks = tokenize_ruby("x = <<END\nbody\nEND\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<END\nbody\nEND"]);
    }

    #[test]
    fn heredoc_lowercase_tag() {
        // Tag identifiers can be any case — Ruby accepts `<<eof`.
        let toks = tokenize_ruby("x = <<eof\nbody\neof\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<eof\nbody\neof"]);
    }

    #[test]
    fn heredoc_multiple_per_line_fifo() {
        // Two heredocs on one line — bodies arrive FIFO.
        let toks = tokenize_ruby("x = <<A; y = <<B\nbody-a\nA\nbody-b\nB\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<A\nbody-a\nA", "<<B\nbody-b\nB"]);
    }

    #[test]
    fn heredoc_chained_method_call_keeps_postfix_tokens() {
        // `x = <<EOF.upcase` — the `.upcase` after the tag must stay
        // in the token stream after heredoc rewriting.
        let toks = tokenize_ruby("x = <<EOF.upcase\nbody\nEOF\n");
        let s: Vec<(TokenType, String)> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.clone()))
            .collect();
        assert_eq!(
            s,
            vec![
                (TokenType::Name, "x".to_string()),
                (TokenType::Equals, "=".to_string()),
                (TokenType::String, "<<EOF\nbody\nEOF".to_string()),
                (TokenType::Dot, ".".to_string()),
                (TokenType::Name, "upcase".to_string()),
                (TokenType::Newline, "\n".to_string()),
            ]
        );
    }

    #[test]
    fn double_left_shift_is_not_heredoc_after_value() {
        // After an integer (`ExprEnd`), `<<` is the left-shift
        // operator, not a heredoc opener.  No heredoc gets queued
        // and the tokens following `<<` are lexed normally.
        let toks = tokenize_ruby("3 << 1\n");
        let s: Vec<(TokenType, String)> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.clone()))
            .collect();
        assert_eq!(
            s,
            vec![
                (TokenType::Number, "3".to_string()),
                (TokenType::Name, "<<".to_string()),
                (TokenType::Number, "1".to_string()),
                (TokenType::Newline, "\n".to_string()),
            ]
        );
    }

    #[test]
    fn unterminated_heredoc_records_diagnostic() {
        // EOF before the terminator line is reached — the lexer
        // still finalizes the heredoc (with whatever body it managed
        // to collect) and records a diagnostic.
        let (toks, diags) = tokenize_ruby_diag("x = <<EOF\nhello\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<EOF\nhello\nEOF"]);
        assert!(diags.iter().any(|d| d.code == "unterminated-heredoc"));
    }

    #[test]
    fn heredoc_after_lparen_at_expression_start() {
        // `(<<EOF)` — `<<` follows `(`, which is expression-start.
        let toks = tokenize_ruby("(<<EOF)\nbody\nEOF\n");
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["<<EOF\nbody\nEOF"]);
    }

    // -----------------------------------------------------------------
    // Phase 4 — multi-era version dispatch.
    // -----------------------------------------------------------------

    #[test]
    fn tokenize_ruby_for_version_accepts_all_eras() {
        // The same baseline source lexes identically under every
        // era version in v0 — the only difference is the
        // state-machine identifier carried by the definition (see
        // `machine::tests::all_15_era_versions_are_accepted`).  When
        // Phase 4b+ forks individual era transitions, the token
        // stream will start diverging here.
        let src = "x = 1 + 2\n";
        for &v in ERA_VERSIONS {
            let toks = tokenize_ruby_for_version(src, v)
                .unwrap_or_else(|e| panic!("version {v} failed: {e}"));
            let kinds: Vec<TokenType> = toks
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .map(|t| t.type_)
                .collect();
            assert_eq!(
                kinds,
                vec![
                    TokenType::Name,
                    TokenType::Equals,
                    TokenType::Number,
                    TokenType::Plus,
                    TokenType::Number,
                    TokenType::Newline,
                ],
                "version {v} produced unexpected token stream",
            );
        }
    }

    #[test]
    fn tokenize_ruby_for_version_rejects_unknown() {
        let err = tokenize_ruby_for_version("x\n", "5.0").unwrap_err();
        assert!(err.contains("not a recognized Ruby era"));
    }

    // -----------------------------------------------------------------
    // Phase 4b — 1.9.1 lambda `->` token fusion.
    // -----------------------------------------------------------------

    #[test]
    fn era_1_9_1_fuses_lambda_arrow() {
        // `f = ->(a) { a + 1 }` — under 1.9.1 the `->` is a single
        // operator token (lambda opener).  We just check the token
        // stream for the fused `->` lexeme; the parser will use it
        // to dispatch to the lambda production once Phase 6 ships
        // the grammar extension.
        let toks = tokenize_ruby_for_version("->(a)", "1.9.1").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(
            values.contains(&"->"),
            "expected fused `->` token, got {:?}",
            values
        );
        // No bare `-` or `>` should remain in this stream.
        assert!(!values.contains(&"-"));
        assert!(!values.contains(&">"));
    }

    #[test]
    fn era_1_8_does_not_fuse_lambda_arrow() {
        // Under 1.8 (and earlier), `->` is two separate tokens —
        // Ruby 1.8 doesn't know about lambda literals.  The Phase
        // 4b fusion is era-gated to 1.9.1+ specifically so 1.8
        // programs keep their pre-lambda lexing.
        let toks = tokenize_ruby_for_version("->(a)", "1.8").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(!values.contains(&"->"), "1.8 must not fuse `->`");
        assert!(values.contains(&"-"));
        assert!(values.contains(&">"));
    }

    #[test]
    fn era_1_9_1_does_not_fuse_minus_greater_with_whitespace_between() {
        // `1 - > 2` (with a space between `-` and `>`) is *not* a
        // lambda — it's a syntax error in real Ruby, but the lexer
        // still emits two separate tokens.  The fusion must require
        // strict column-adjacency.
        let toks = tokenize_ruby_for_version("1 - > 2", "1.9.1").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(!values.contains(&"->"));
    }

    #[test]
    fn era_2_3_inherits_lambda_arrow_from_1_9_1() {
        // The fusion is era-gated to ≥ 1.9.1 — every later era
        // (2.0 through 3.3) inherits it.  Spot-check a couple.
        for era in ["2.0", "2.3", "2.7", "3.0", "3.3"] {
            let toks = tokenize_ruby_for_version("->(a)", era).unwrap();
            let values: Vec<&str> = toks
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .map(|t| t.value.as_str())
                .collect();
            assert!(
                values.contains(&"->"),
                "era {era} should inherit lambda-arrow fusion"
            );
        }
    }

    // -----------------------------------------------------------------
    // Phase 4c — 2.3 safe-navigation `&.` token fusion.
    // -----------------------------------------------------------------

    #[test]
    fn era_2_3_fuses_safe_nav() {
        let toks = tokenize_ruby_for_version("a&.b", "2.3").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(
            values.contains(&"&."),
            "expected fused `&.` token, got {:?}",
            values
        );
        assert!(!values.contains(&"&"));
    }

    #[test]
    fn era_1_8_does_not_fuse_safe_nav() {
        let toks = tokenize_ruby_for_version("a&.b", "1.8").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(!values.contains(&"&."), "1.8 must not fuse `&.`");
        assert!(values.contains(&"&"));
        assert!(values.contains(&"."));
    }

    #[test]
    fn era_2_3_does_not_fuse_amp_dot_with_whitespace_between() {
        let toks = tokenize_ruby_for_version("a & .b", "2.3").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(!values.contains(&"&."));
    }

    #[test]
    fn era_3_3_inherits_safe_nav_from_2_3() {
        for era in ["2.5", "2.7", "3.0", "3.3"] {
            let toks = tokenize_ruby_for_version("a&.b", era).unwrap();
            let values: Vec<&str> = toks
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .map(|t| t.value.as_str())
                .collect();
            assert!(
                values.contains(&"&."),
                "era {era} should inherit safe-nav fusion"
            );
        }
    }

    #[test]
    fn era_2_1_does_not_fuse_safe_nav_yet() {
        // 2.1 < 2.3 — the era gate must keep them separate so
        // pre-2.3 programs lex the same way they did when written.
        let toks = tokenize_ruby_for_version("a&.b", "2.1").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(!values.contains(&"&."), "2.1 should NOT fuse `&.`");
    }

    // -----------------------------------------------------------------
    // Phase 4d — 2.7 numbered block params `_1` .. `_9`.
    // -----------------------------------------------------------------

    fn has_numbered_param_flag(t: &Token) -> bool {
        (t.flags.unwrap_or(0) & NUMBERED_BLOCK_PARAM_FLAG) != 0
    }

    #[test]
    fn era_2_7_flags_underscore_digit_names() {
        let toks = tokenize_ruby_for_version("_1 + _2", "2.7").unwrap();
        let flagged: Vec<&str> = toks
            .iter()
            .filter(|t| has_numbered_param_flag(t))
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(flagged, vec!["_1", "_2"]);
    }

    #[test]
    fn era_2_6_does_not_flag_underscore_digit_names() {
        // 2.6 < 2.7 — the flag must stay off so callers treat `_1`
        // as a regular local variable.
        let toks = tokenize_ruby_for_version("_1 + _2", "2.6").unwrap();
        let any_flagged = toks.iter().any(has_numbered_param_flag);
        assert!(!any_flagged, "2.6 must not flag _N tokens");
    }

    #[test]
    fn era_2_7_does_not_flag_other_underscore_names() {
        // `_foo`, `_`, `_0`, `_10` are NOT numbered block params.
        let toks = tokenize_ruby_for_version("_foo + _ + _0 + _10", "2.7").unwrap();
        let any_flagged = toks.iter().any(has_numbered_param_flag);
        assert!(
            !any_flagged,
            "lexemes other than `_1`..`_9` must not get the flag"
        );
    }

    #[test]
    fn era_3_3_inherits_numbered_block_param_flag() {
        for era in ["2.7", "3.0", "3.3"] {
            let toks = tokenize_ruby_for_version("_1", era).unwrap();
            let any_flagged = toks.iter().any(has_numbered_param_flag);
            assert!(any_flagged, "era {era} should flag _1");
        }
    }

    #[test]
    fn is_numbered_block_param_classifies_correctly() {
        assert!(is_numbered_block_param("_1"));
        assert!(is_numbered_block_param("_5"));
        assert!(is_numbered_block_param("_9"));
        // Edge cases that look numbered but aren't.
        assert!(!is_numbered_block_param("_0"));
        assert!(!is_numbered_block_param("_10"));
        assert!(!is_numbered_block_param("_"));
        assert!(!is_numbered_block_param("_a"));
        assert!(!is_numbered_block_param("_1a"));
        assert!(!is_numbered_block_param(""));
        assert!(!is_numbered_block_param("1"));
    }

    #[test]
    fn era_at_least_comparator_is_total_and_chronological() {
        assert!(era_at_least("1.9.1", "1.9.1"));
        assert!(era_at_least("2.0", "1.9.1"));
        assert!(era_at_least("3.3", "1.0"));
        assert!(!era_at_least("1.8", "1.9.1"));
        assert!(!era_at_least("1.0", "1.6"));
        // Unknown era folds to 1.8.
        assert!(!era_at_least("99.0", "1.9.1"));
        // Empty string folds to 1.8 (the lexer default).
        assert!(!era_at_least("", "1.9.1"));
    }

    #[test]
    fn era_versions_includes_1_0_through_3_3() {
        // Spot-check that the public re-export from `machine` is
        // wired through and contains the expected end-points.
        assert!(ERA_VERSIONS.contains(&"1.0"));
        assert!(ERA_VERSIONS.contains(&"1.8"));
        assert!(ERA_VERSIONS.contains(&"3.3"));
        assert_eq!(ERA_VERSIONS.len(), 15);
    }

    #[test]
    fn is_heredoc_tag_accepts_valid_idents() {
        assert!(is_heredoc_tag("EOF"));
        assert!(is_heredoc_tag("eof"));
        assert!(is_heredoc_tag("_PRIVATE"));
        assert!(is_heredoc_tag("My_Tag_2"));
        assert!(!is_heredoc_tag(""));
        assert!(!is_heredoc_tag("2tag"));
        assert!(!is_heredoc_tag("with space"));
        assert!(!is_heredoc_tag("dash-tag"));
    }
}
