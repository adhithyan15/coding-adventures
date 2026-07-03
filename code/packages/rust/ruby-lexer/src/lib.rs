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
    /// Phase 6p companion — set by `push` for one `step_char` call
    /// when the upcoming character is `/` and its immediate follower
    /// is `=`.  Forces `should_open_regex` to return `false` so the
    /// state machine emits `/` as a plain Op token.  The compound-
    /// assign fusion pass then folds `Op(/) Equals` → `Name("/=")`.
    suppress_regex_open: bool,
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
    /// Phase 4o — opener variant: plain `<<`, `<<-` (indent-tolerant
    /// terminator), or `<<~` (indent-stripping body + indent-tolerant
    /// terminator).  Captured at open time so finalize knows which
    /// post-processing to apply.
    variant: HeredocVariant,
}

/// Phase 4o — heredoc opener form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeredocVariant {
    /// `<<EOF` — terminator must start at column 0 with no leading
    /// whitespace, body captured verbatim.
    Plain,
    /// `<<-EOF` (since Ruby 1.9) — terminator may have leading
    /// whitespace; body captured verbatim.
    DashIndent,
    /// `<<~EOF` (since Ruby 2.3) — terminator may have leading
    /// whitespace; body has its common leading whitespace stripped
    /// from every line (the smallest leading-ws prefix across all
    /// non-empty body lines).
    TildeIndent,
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
            suppress_regex_open: false,
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
            // Phase FC — `__END__` alone on a line (column 0) terminates
            // the program: everything after it is the `DATA` section, not
            // Ruby code.  When we're at a line start and the upcoming line
            // is exactly `__END__`, stop feeding the engine so no tokens
            // are produced for the trailing data.  (`finish()` still
            // flushes the EOF token.)  The line-start guard means we only
            // trigger during normal lexing — heredoc bodies are consumed
            // inside `capture_heredoc_bodies`, which bypasses this loop.
            if (i == 0 || chars[i - 1] == '\n') && Self::is_end_marker(&chars, i) {
                break;
            }
            let ch = chars[i];
            // Phase 6p companion — `/=` compound-assignment guard.
            // If we're about to feed a `/` whose immediate follower is
            // `=`, suppress the regex-vs-divide decision and let the
            // state machine emit `/` as a plain Op.  The compound-
            // assign fusion pass then folds `Op(/) Equals` into a
            // single `Name("/=")` token.  Without this guard, the
            // oracle treats `x /= 1` as `x / <regex starting `=`...`
            // which would never terminate.
            self.suppress_regex_open = ch == '/' && chars.get(i + 1) == Some(&'=');
            self.step_char(ch)?;
            self.suppress_regex_open = false;
            i += 1;
            if ch == '\n' && !self.pending_heredocs.is_empty() {
                i = self.capture_heredoc_bodies(&chars, i)?;
            }
        }
        Ok(())
    }

    /// Phase FC — true iff `chars[i..]` is exactly the program-terminator
    /// line `__END__` (assumed to start at column 0 by the caller),
    /// followed by end-of-input, `\n`, or `\r`.  Ruby treats such a line
    /// as the end of the source; the remainder is the `DATA` section.
    ///
    /// Scope: this only halts tokenization.  `DATA` itself stays an
    /// ordinary constant read (no synthesized file handle) — a deliberate
    /// follow-up if a real data stream is ever needed.  A line like
    /// `__END__foo` (trailing non-newline) is *not* a marker, matching
    /// Ruby, which requires `__END__` alone on the line.
    fn is_end_marker(chars: &[char], i: usize) -> bool {
        const MARK: [char; 7] = ['_', '_', 'E', 'N', 'D', '_', '_'];
        if i + MARK.len() > chars.len() || chars[i..i + MARK.len()] != MARK {
            return false;
        }
        matches!(chars.get(i + MARK.len()), None | Some('\n') | Some('\r'))
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

            // Phase 4o — terminator match depends on the opener
            // variant:
            //   Plain (`<<EOF`):    exact line == tag.
            //   DashIndent (`<<-`): line stripped of leading ws == tag.
            //   TildeIndent (`<<~`): line stripped of leading ws == tag.
            let front = self.pending_heredocs.front().unwrap();
            let front_tag = front.tag.clone();
            let front_variant = front.variant;
            let trimmed_line = line.trim_start();
            let is_terminator = match front_variant {
                HeredocVariant::Plain => line == front_tag,
                HeredocVariant::DashIndent | HeredocVariant::TildeIndent => {
                    trimmed_line == front_tag
                }
            };
            if is_terminator {
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
    /// reconstructed heredoc source.
    ///
    /// Phase 4o variants:
    /// - `Plain` (`<<TAG\n<body>TAG`): body captured verbatim.
    /// - `DashIndent` (`<<-TAG\n<body>TAG`): body captured verbatim;
    ///   the only change is the `<<-` prefix in the reconstructed
    ///   source so downstream tooling can detect the modifier.
    /// - `TildeIndent` (`<<~TAG\n<body>TAG`): strip the common
    ///   leading-whitespace prefix from every non-empty body line.
    ///   That prefix is the minimum across all non-empty lines.
    fn finalize_heredoc(&mut self, h: PendingHeredoc) {
        let body = match h.variant {
            HeredocVariant::Plain | HeredocVariant::DashIndent => h.body.clone(),
            HeredocVariant::TildeIndent => strip_common_leading_whitespace(&h.body),
        };
        let prefix = match h.variant {
            HeredocVariant::Plain => "<<",
            HeredocVariant::DashIndent => "<<-",
            HeredocVariant::TildeIndent => "<<~",
        };
        let value = format!("{prefix}{}\n{body}{}", h.tag, h.tag);
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
                flags: None, cv: None,
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
        // The range fusions (`..` / `...`) are unconditional — Ruby
        // has had range literals since 1.0.  They run first so
        // later era-gated passes operate on the already-fused
        // stream.
        self.fuse_range_ops();
        // Phase 4l — radix-prefixed integers (`0x1F`, `0b1010`,
        // `0o17`, `0d10`) are pre-1.0 Ruby; no era gating.  Runs
        // BEFORE float fusion — they share the `Int "0"` left flank,
        // but radix prefixes start with a *letter* (x/b/o/d) and
        // float dot doesn't, so the two passes are non-overlapping.
        // Ordering is documentation-only.
        self.fuse_radix_integers();
        // Phase 4k — float literals (`1.5`, `1e10`, `1.5e-3`, etc.)
        // are pre-1.0 Ruby; no era gating.  Runs after range fusion
        // so `1..5` (which the range pass converts to `Int ".." Int`)
        // doesn't get mistaken for a float.
        self.fuse_float_literals();
        // Phase 8a-2 (FC) companion — pre-fuse `>>` and `>>=`.
        //
        // The 1.8-baseline state machine emits `>>` as two separate
        // `Name(">")` tokens (no dedicated right-shift token), and the
        // greedy `>=` classifier folds `>` immediately followed by `=`
        // into `Name(">=")`.  That means `x >>= 5` arrives here as
        // `Name("x")`, `Name(">")`, `Name(">=")`, `Number("5")` — the
        // second `>` got eaten by the `>=` rule.
        //
        // This pre-fusion pass restores the right shapes:
        //   `Name(">")` + `Name(">")`  (no ws) → `Name(">>")`
        //   `Name(">")` + `Name(">=")` (no ws) → `Name(">>=")`
        //
        // Must run BEFORE `fuse_compound_assigns` so the standalone `=`
        // case (e.g. `x = 5`) is unaffected, and so that any future
        // `Name(">>")` left-shift token gets a chance to participate in
        // compound-assign fusion (currently moot — `>>=` is folded
        // directly here, but the pattern stays consistent).
        self.fuse_right_shifts();
        // Phase 6p companion — fuse compound-assignment operators
        // (`+=`, `-=`, `*=`, `/=`, `||=`, `&&=`).  Pre-1.0 Ruby, so
        // no era gating.  Runs AFTER float/radix fusions (which fold
        // numeric forms) so `x +=` doesn't fight with `1e+10` etc.
        self.fuse_compound_assigns();
        // Phase 6q companion — re-tag the trailing-modifier keywords
        // (`if`, `unless`, `while`, `until`) to `if_modifier`,
        // `unless_modifier`, `while_modifier`, `until_modifier` when
        // they appear after an expression-ending token on the same
        // line.  The grammar's `modifier_statement` rule keys off the
        // re-tagged values; the leading-keyword statement rules
        // (`if_statement`, etc.) still key off the bare `if`/`while`
        // values.  This lexer-side disambiguation sidesteps the
        // grammar's newline-insensitive default mode (modifier syntax
        // is a same-line construct in Ruby).  Pre-1.0 Ruby — no era
        // gating.  Runs LAST among the fusions so it sees the final
        // fused-token shape (notably `end` of a same-line block
        // already in place if relevant).
        self.tag_modifier_keywords();
        if era_at_least(&self.era, "1.9.1") {
            self.fuse_lambda_arrow();
        }
        if era_at_least(&self.era, "2.1") {
            self.fuse_numeric_suffixes();
        }
        if era_at_least(&self.era, "2.3") {
            self.fuse_safe_nav();
        }
        if era_at_least(&self.era, "2.6") {
            self.mark_endless_ranges();
        }
        if era_at_least(&self.era, "2.7") {
            self.mark_numbered_block_params();
        }
    }

    /// Phase 4l — fuse radix-prefixed integer literals emitted by the
    /// 1.8-baseline state machine into a single `Number` token.
    ///
    /// Ruby's explicit-radix integer prefixes:
    /// | Prefix      | Base | Example      | Example digits     |
    /// |-------------|------|--------------|--------------------|
    /// | `0x` / `0X` | 16   | `0xDEAD_BEEF`| `0-9a-fA-F_`       |
    /// | `0b` / `0B` |  2   | `0b1010_1100`| `0-1_`             |
    /// | `0o` / `0O` |  8   | `0o755`      | `0-7_`             |
    /// | `0d` / `0D` | 10   | `0d42`       | `0-9_`             |
    ///
    /// The state machine emits these as two tokens:
    ///   - `Int("0")` from the int_body sub-machine.
    ///   - `Name("xDEAD_BEEF")` from the ident_body sub-machine
    ///     (which starts on the `x` letter and slurps alphanumerics
    ///     and underscores).
    ///
    /// This post-pass detects the `Int("0") Name(prefix+digits)`
    /// pattern (with no whitespace and on the same line) and fuses
    /// into a single `Number("0xDEAD_BEEF")` token.  Same mechanism
    /// as `fuse_numeric_suffixes` (2.1 `2r`/`3i`) and
    /// `fuse_float_literals` (1.0 `1.5`/`1e10`).
    ///
    /// **What this is NOT doing:**
    /// - Old-style C-flavoured octal `017`: that's already a single
    ///   `Int("017")` token from int_body — interpretation is up to
    ///   downstream (the parser / SIR lowerer can decide whether a
    ///   leading-zero integer is octal or decimal-with-padding).
    /// - Validation of digit alphabets vs base: `0x9z` would NOT
    ///   fuse because `9z` isn't a valid hex body (z isn't in the
    ///   allowed alphabet).  Diagnostics for invalid digits are out
    ///   of scope for v0 — the post-pass simply doesn't fire and
    ///   the parser ends up with `Int(0)` + `Name("x9z")` which it
    ///   will reject as a syntax error.
    fn fuse_radix_integers(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_zero = a.type_ == TokenType::Number && a.value == "0";
                let b_is_radix_body = b.type_ == TokenType::Name
                    && is_radix_integer_body(&b.value);
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_zero && b_is_radix_body && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "0{}",
                    self.tokens[i + 1].value
                );
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    /// Phase 4k — fuse float-literal token sequences emitted by the
    /// 1.8-baseline state machine into a single `Number` token.
    ///
    /// The state-machine TOML emits the simple shapes:
    /// - integer body (`1`, `1_000`) → `Int("1")` / `Int("1_000")`
    /// - bare `.` between numbers → `Dot`
    /// - bare identifier starting with `e` or `E` → `Name("e10")` etc.
    /// - `+` / `-` between tokens → `Plus` / `Minus`
    ///
    /// Float literals show up in the stream as one of these patterns
    /// (no whitespace between the constituent tokens — same trick
    /// `fuse_numeric_suffixes` uses):
    ///
    /// | Source       | Pre-fusion tokens                   |
    /// |--------------|-------------------------------------|
    /// | `1.5`        | Int "1", Dot, Int "5"               |
    /// | `1e10`       | Int "1", Name "e10"                 |
    /// | `1e+10`      | Int "1", Name "e", Plus, Int "10"   |
    /// | `1.5e10`     | Int "1", Dot, Int "5", Name "e10"   |
    /// | `1.5e-3`     | Int "1", Dot, Int "5", Name "e",    |
    /// |              |   Minus, Int "3"                    |
    ///
    /// Float literals have been in Ruby since 1.0, so this pass is
    /// **unconditional** (no era gate).
    ///
    /// Why not handle this in the TOML directly?  The state machine
    /// would need lookahead to decide between `1.5` (one float
    /// token) and `1.method` (Int + Dot + Name).  Lookahead in our
    /// state machine is awkward — we'd have to peek/unpeek `.` and
    /// re-emit if it turns out to be a method call.  The post-pass
    /// approach is uniform with how `..` / `...` / `->` / `&.` / `2r`
    /// / `_1` already work and avoids that complexity.
    fn fuse_float_literals(&mut self) {
        // Step 1: `Int Dot Int` (no whitespace between any of the
        // three) fuses to a single `Number("X.Y")` token.
        let mut i = 0;
        while i + 2 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let c = &self.tokens[i + 2];
                let a_is_int = a.type_ == TokenType::Number
                    && self.is_integer_lexeme(&a.value);
                let b_is_dot = b.type_ == TokenType::Dot;
                let c_is_int = c.type_ == TokenType::Number
                    && self.is_integer_lexeme(&c.value);
                let same_line = a.line == b.line && b.line == c.line;
                let no_ws_b = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                let no_ws_c = !self
                    .whitespace_before_token
                    .get(i + 2)
                    .copied()
                    .unwrap_or(false);
                a_is_int && b_is_dot && c_is_int && same_line && no_ws_b && no_ws_c
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "{}.{}",
                    self.tokens[i].value,
                    self.tokens[i + 2].value
                );
                self.tokens.remove(i + 2);
                self.whitespace_before_token.remove(i + 2);
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
                // Don't advance i — the scientific-notation suffix
                // (if any) may follow.
            } else {
                i += 1;
            }
        }
        // Step 2: `Number Name(e<digits>)` (or `Int Name(e<digits>)`)
        // fuses the trailing scientific notation when the name's
        // lexeme is exactly `[eE]<digit_or_underscore>+`.  No sign
        // case (handled in step 3).
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_num = a.type_ == TokenType::Number;
                let b_is_unsigned_exp = b.type_ == TokenType::Name
                    && is_unsigned_exponent_lexeme(&b.value);
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_num && b_is_unsigned_exp && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "{}{}",
                    self.tokens[i].value,
                    self.tokens[i + 1].value
                );
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
                // Don't advance — the signed-exponent step below
                // might still match (unlikely but harmless).
            } else {
                i += 1;
            }
        }
        // Step 3: `Number Name("e"|"E") (Plus|Minus) Int` fuses the
        // trailing signed exponent.  Four tokens collapse to one.
        let mut i = 0;
        while i + 3 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let c = &self.tokens[i + 2];
                let d = &self.tokens[i + 3];
                let a_is_num = a.type_ == TokenType::Number;
                let b_is_e = b.type_ == TokenType::Name
                    && (b.value == "e" || b.value == "E");
                let c_is_sign = (c.type_ == TokenType::Plus && c.value == "+")
                    || (c.type_ == TokenType::Minus && c.value == "-");
                let d_is_int = d.type_ == TokenType::Number
                    && self.is_integer_lexeme(&d.value);
                let same_line =
                    a.line == b.line && b.line == c.line && c.line == d.line;
                let no_ws_b = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                let no_ws_c = !self
                    .whitespace_before_token
                    .get(i + 2)
                    .copied()
                    .unwrap_or(false);
                let no_ws_d = !self
                    .whitespace_before_token
                    .get(i + 3)
                    .copied()
                    .unwrap_or(false);
                a_is_num
                    && b_is_e
                    && c_is_sign
                    && d_is_int
                    && same_line
                    && no_ws_b
                    && no_ws_c
                    && no_ws_d
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "{}{}{}{}",
                    self.tokens[i].value,
                    self.tokens[i + 1].value,
                    self.tokens[i + 2].value,
                    self.tokens[i + 3].value
                );
                for _ in 0..3 {
                    self.tokens.remove(i + 1);
                    self.whitespace_before_token.remove(i + 1);
                }
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            } else {
                i += 1;
            }
        }
    }

    /// Phase 8a-2 (FC) companion — fold `>>` and `>>=`.
    ///
    /// The state machine doesn't carry a dedicated right-shift token
    /// (unlike `<<`, which the heredoc opener already emits as a
    /// single `Name("<<")`).  Two-character `>>` therefore arrives as
    /// two separate `Name(">")` tokens.  Worse, when followed by `=`
    /// the greedy `>=` classifier eats the second `>` together with
    /// the trailing `=`, so `x >>= 5` arrives as `>`, `>=` — *neither*
    /// `>>` nor `=` survives intact.
    ///
    /// This pass scans for the two shapes:
    ///
    /// | Incoming pair (adjacent, no whitespace gap) | Folded into     |
    /// |---------------------------------------------|-----------------|
    /// | `Name(">")` + `Name(">")`                   | `Name(">>")`    |
    /// | `Name(">")` + `Name(">=")`                  | `Name(">>=")`   |
    ///
    /// Adjacency gate: same line, no whitespace before the second
    /// token.  Same gating as `fuse_compound_assigns` — a space breaks
    /// the fusion (`x > > 5` stays two tokens, which is a syntax error
    /// in Ruby but not a right shift / compound-shift).
    ///
    /// Why a separate pass rather than extending
    /// `fuse_compound_assigns`?  Because the input shape is `>`, `>=`
    /// — there is no standalone `=` for the existing fuse pass to fold
    /// against.  The `>=` token is already a unit by the time the
    /// fusion pipeline runs.
    ///
    /// Era: pre-1.0 Ruby — every era ≥ 1.8 emits the same problematic
    /// pre-fusion shape, so no gating.
    fn fuse_right_shifts(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            // Left token must be `Name(">")`.
            let is_gt = {
                let a = &self.tokens[i];
                a.type_ == TokenType::Name && a.value == ">"
            };
            if !is_gt {
                i += 1;
                continue;
            }
            // Right token must be `Name(">")` or `Name(">=")`, on the
            // same line, with no whitespace gap.
            let (new_value_opt, same_line, no_ws) = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                let nv: Option<&'static str> = if b.type_ == TokenType::Name {
                    match b.value.as_str() {
                        ">" => Some(">>"),
                        ">=" => Some(">>="),
                        _ => None,
                    }
                } else {
                    None
                };
                (nv, same_line, no_ws)
            };
            if let Some(nv) = new_value_opt {
                if same_line && no_ws {
                    let span_line = self.tokens[i].line;
                    let span_col = self.tokens[i].column;
                    self.tokens.remove(i + 1);
                    self.whitespace_before_token.remove(i + 1);
                    self.tokens[i] = Token {
                        type_: TokenType::Name,
                        value: nv.to_string(),
                        line: span_line,
                        column: span_col,
                        type_name: None,
                        flags: None, cv: None,
                    };
                    // Don't advance — three-`>` sequences (e.g. `a>>>b`,
                    // which is illegal in Ruby anyway) get correctly
                    // partial-folded on the next iteration.
                    continue;
                }
            }
            i += 1;
        }
    }

    /// Phase 6p companion — fuse compound-assignment operators
    /// `+=`, `-=`, `*=`, `/=`, `||=`, `&&=` into a single Name-typed
    /// token whose value is the fused operator (`+=` etc.).
    ///
    /// Phase 8a (FC) extension — also fuse the remaining compound
    /// assigns Ruby supports on the left-hand side: `%=`, `**=`,
    /// `<<=`, `&=`, `|=`, `^=`.  These all come through as `Name`
    /// tokens with the operator value (e.g. `Name("%")`, `Name("**")`,
    /// `Name("<<")`, `Name("&")`, `Name("|")`, `Name("^")`), so the
    /// same fold pattern applies — we just widen the `match` arm to
    /// recognise them by value.  Note that `>>=` is NOT handled here
    /// because the 1.8-era state machine emits `>>` as two separate
    /// `Name(">")` tokens; folding that requires a dedicated `>>`
    /// pre-fusion pass and is deferred.
    ///
    /// The 1.8-baseline state machine emits compound assigns as two
    /// tokens:
    ///   - The op (Plus / Minus / Star / Slash for `+`/`-`/`*`/`/`,
    ///     or Name for `||` / `&&` / `%` / `**` / `<<` / `&` / `|`
    ///     / `^` — classify_op_token's catch-all).
    ///   - Equals for the trailing `=`.
    ///
    /// This pass folds the pair into a single token so the grammar
    /// can match by value (`"+="`, `"-="`, etc.) the same way it
    /// matches `"=>"`, `"<="`, `"&&"`.
    ///
    /// Adjacency gate: the `=` must NOT have whitespace before it.
    /// `x + = 1` (with a space) stays two tokens — that's a syntax
    /// error in real Ruby but it's not a compound assignment.
    ///
    /// Era: pre-1.0 Ruby — every era ≥ 1.8 emits the same fused
    /// shape, so no gating.
    fn fuse_compound_assigns(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let (left_lexeme, is_arith): (&str, bool) = {
                let a = &self.tokens[i];
                match a.type_ {
                    TokenType::Plus => ("+", true),
                    TokenType::Minus => ("-", true),
                    TokenType::Star => ("*", true),
                    TokenType::Slash => ("/", true),
                    // `||`, `&&`, `%`, `**`, `<<`, `&`, `|`, `^`
                    // all come through as Name (catch-all in
                    // classify_op_token).  Filter by value so we
                    // don't accidentally fuse `foo =` into something.
                    TokenType::Name
                        if matches!(
                            a.value.as_str(),
                            "||" | "&&" | "%" | "**" | "<<" | "&" | "|" | "^"
                        ) =>
                    {
                        let lex: &'static str = match a.value.as_str() {
                            "||" => "||",
                            "&&" => "&&",
                            "%" => "%",
                            "**" => "**",
                            "<<" => "<<",
                            "&" => "&",
                            "|" => "|",
                            "^" => "^",
                            _ => unreachable!(),
                        };
                        (lex, false)
                    }
                    _ => {
                        i += 1;
                        continue;
                    }
                }
            };
            let merge = {
                let b = &self.tokens[i + 1];
                let b_is_eq = b.type_ == TokenType::Equals;
                let same_line = self.tokens[i].line == b.line;
                // No whitespace gap between op and `=`.
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                b_is_eq && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!("{left_lexeme}=");
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
                // Don't advance — chained compounds are illegal but
                // the next iteration's guard will handle it cleanly.
            } else {
                // Reference `is_arith` once so the compiler doesn't
                // warn — the variable is documentary for now.
                let _ = is_arith;
                i += 1;
            }
        }
    }

    /// Phase 6q companion — re-tag trailing-modifier keywords.
    ///
    /// Ruby's modifier conditionals and loops (`x if y`, `x unless y`,
    /// `x while y`, `x until y`) are a same-line surface syntax that's
    /// semantically distinct from the leading-keyword statement forms
    /// (`if y\n  x\nend`).  The lexer needs to disambiguate the two
    /// surface forms because the grammar runs in newline-insensitive
    /// mode — a naive `modifier_statement = expression (...) expression`
    /// rule would greedily eat newlines and mis-parse two-line
    /// programs like:
    ///
    /// ```text
    /// x = 1
    /// if x ...
    /// ```
    ///
    /// as `(x = 1) if x` rather than two statements.
    ///
    /// Trigger:
    /// - Token is `Keyword("if"|"unless"|"while"|"until")`.
    /// - The preceding non-`Newline` token exists, AND
    /// - That preceding token is on the *same* `line`, AND
    /// - That preceding token is an expression-ending token (numbers,
    ///   strings, names, closers `)`/`]`/`}`, and the
    ///   expression-ending keywords `nil`/`true`/`false`/`self`/`end`).
    ///
    /// Effect: the token's `value` is rewritten to
    /// `if_modifier` / `unless_modifier` / `while_modifier` /
    /// `until_modifier`.  Its `type_` stays `Keyword`.
    ///
    /// The grammar literal matches by value, so:
    ///   - `if_statement = "if" expression { ... } "end" ;` continues
    ///     to match the BARE `if` token at statement-start position.
    ///   - `modifier_statement = ... ( "if_modifier" | ... ) expression ;`
    ///     matches the RE-TAGGED `if_modifier` token after an
    ///     expression on the same line.
    ///
    /// Era: pre-1.0 Ruby (modifier `if` predates 1.0).  No gating.
    fn tag_modifier_keywords(&mut self) {
        let n = self.tokens.len();
        for i in 0..n {
            // Cheap value match first.
            let is_target = matches!(
                self.tokens[i].value.as_str(),
                "if" | "unless" | "while" | "until"
            ) && self.tokens[i].type_ == TokenType::Keyword;
            if !is_target {
                continue;
            }
            // Find the preceding non-Newline token (if any).
            let prev_idx = (0..i).rev().find(|&k| {
                self.tokens[k].type_ != TokenType::Newline
            });
            let Some(j) = prev_idx else { continue; };
            // Same line?  Different line ⇒ statement-start position,
            // not a modifier.
            if self.tokens[j].line != self.tokens[i].line {
                continue;
            }
            // Is the preceding token expression-ending?  This is what
            // gates `x ; if y ... end` (semicolon is NOT
            // expression-ending) versus `puts 1 if y` (the `1` IS
            // expression-ending).
            let prev_ends_expr = match self.tokens[j].type_ {
                TokenType::Number
                | TokenType::String
                | TokenType::Name
                | TokenType::RParen
                | TokenType::RBracket
                | TokenType::RBrace => true,
                TokenType::Keyword => matches!(
                    self.tokens[j].value.as_str(),
                    "nil" | "true" | "false" | "self" | "end"
                ),
                _ => false,
            };
            if !prev_ends_expr {
                continue;
            }
            // Re-tag.  Mutating `value` only — `type_` stays Keyword.
            let new_value = format!("{}_modifier", self.tokens[i].value);
            self.tokens[i].value = new_value;
        }
    }

    /// Phase 4k helper — true iff `s` is a pure integer-shaped lexeme
    /// (digits and underscore separators only, no dot or exponent).
    /// Used to discriminate already-fused float Numbers from the
    /// integer Numbers that float-fusion is allowed to consume.
    fn is_integer_lexeme(&self, s: &str) -> bool {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_digit() || c == '_')
    }

    /// 2.1: fold a `Number` token followed (with no whitespace) by
    /// a single-letter `Name("r")` or `Name("i")` into one fused
    /// numeric token — Ruby 2.1's rational (`2r`) and complex
    /// (`3i`) literal suffixes.  Pre-2.1 these were two separate
    /// tokens (a number plus a stray identifier).  The era gate
    /// keeps that behaviour faithful for older programs.
    fn fuse_numeric_suffixes(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_num = a.type_ == TokenType::Number;
                let b_is_suffix = b.type_ == TokenType::Name
                    && (b.value == "r" || b.value == "i");
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_num && b_is_suffix && same_line && no_ws
            };
            if merge {
                let suffix = self.tokens[i + 1].value.clone();
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!("{}{}", self.tokens[i].value, suffix);
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    /// Unconditional: fold adjacent `Dot` tokens into a single
    /// `Op("..")` (inclusive range) or `Op("...")` (exclusive
    /// range).  Ruby has had these range operators since 1.0; the
    /// 1.8 state machine emits each `.` as a separate Dot and
    /// leaves the multi-dot composition to this post-pass — same
    /// pattern Phase 4b/4c used for `->` and `&.`.
    fn fuse_range_ops(&mut self) {
        // First pass: combine pairs of adjacent Dots into `..`.
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let both_dots =
                    a.type_ == TokenType::Dot && b.type_ == TokenType::Dot;
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                both_dots && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "..".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
        // Second pass: `..` (now a Name token from the first pass)
        // followed by a Dot fuses into `...`.
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_dotdot = a.type_ == TokenType::Name && a.value == "..";
                let b_is_dot = b.type_ == TokenType::Dot;
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_dotdot && b_is_dot && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "...".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    /// 2.6: tag `..` / `...` range tokens followed by a *closer*
    /// (right paren, right bracket, comma, semicolon, newline, or
    /// EOF) with [`ENDLESS_RANGE_FLAG`].  Pre-2.6 these positions
    /// were parse errors; 2.6 made them legal endless ranges
    /// (`(1..)`, `arr[2..]`, etc.).
    fn mark_endless_ranges(&mut self) {
        let closers = [
            TokenType::RParen,
            TokenType::RBracket,
            TokenType::RBrace,
            TokenType::Comma,
            TokenType::Semicolon,
            TokenType::Newline,
            TokenType::Eof,
        ];
        for i in 0..self.tokens.len() {
            let is_range = self.tokens[i].type_ == TokenType::Name
                && (self.tokens[i].value == ".." || self.tokens[i].value == "...");
            if !is_range {
                continue;
            }
            let next_kind = self
                .tokens
                .get(i + 1)
                .map(|t| t.type_)
                .unwrap_or(TokenType::Eof);
            if closers.contains(&next_kind) {
                let prev = self.tokens[i].flags.unwrap_or(0);
                self.tokens[i].flags = Some(prev | ENDLESS_RANGE_FLAG);
            }
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
                    flags: None, cv: None,
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
                    flags: None, cv: None,
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
        // Phase 6p companion — `push` sets `suppress_regex_open` for
        // exactly one `step_char` call when the upcoming `/` is
        // immediately followed by `=`.  Force a plain-Op emit so
        // `fuse_compound_assigns` can fold `Op(/) Equals` into
        // `Name("/=")`.
        if self.suppress_regex_open {
            return false;
        }
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
                        // Phase 4o — inspect the opener's text to
                        // pick the variant.  The state machine emits
                        // exactly one of `<<` / `<<-` / `<<~` for the
                        // Op token; defaulting to Plain on unexpected
                        // shapes keeps the lowering total.
                        let variant = match self.tokens[op_idx].value.as_str() {
                            "<<-" => HeredocVariant::DashIndent,
                            "<<~" => HeredocVariant::TildeIndent,
                            _ => HeredocVariant::Plain,
                        };
                        self.pending_heredocs.push_back(PendingHeredoc {
                            tag: text,
                            op_idx,
                            tag_idx,
                            body: String::new(),
                            variant,
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
            "PercentI" | "PercentBigI" => {
                // Phase 4g — Ruby 2.0+ symbol array literals.
                // Like the other percent literals, emit as a
                // `TokenType::String` carrying the verbatim source
                // (`%i[a b c]` or `%I[a b c]`).  The post-pass
                // downgrades them back to `%` + identifier under
                // era < 2.0 so pre-2.0 lexings stay faithful.
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::String, text);
            }
            "PercentR" | "PercentS" | "PercentX" => {
                // Phase 4n — `%r{regex}`, `%s{symbol}`, `%x{cmd}`.
                // Like the other percent literals (`%w[…]`, `%q{…}`),
                // emit as `TokenType::String` carrying the verbatim
                // source (`%r{pat}`, `%s{name}`, `%x{cmd}`).  The
                // parser distinguishes them from plain strings by the
                // leading `%` + type letter — same sentinel-by-prefix
                // trick the rest of the percent family uses.
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::String, text);
            }
            "Backtick" => {
                // Phase 4m — `` `cmd args` `` command-execution literal.
                // The text buffer holds the *body* (without the
                // surrounding backticks).  Encode as `TokenType::String`
                // with the backticks re-wrapped so the parser can
                // distinguish backtick literals from plain strings by
                // inspecting the lexeme's first character.  Same
                // sentinel-by-prefix trick the percent literals
                // (`%w[…]`) and heredocs (`<<TAG\n…TAG`) use.
                let body = std::mem::take(&mut self.text_buffer);
                let value = format!("`{body}`");
                self.push_token(TokenType::String, value);
            }
            "Op" => {
                let text = std::mem::take(&mut self.text_buffer);
                let kind = classify_op_token(&text);
                // Phase 4o — accept `<<`, `<<-`, `<<~` as heredoc
                // openers when the lex-state is expression-start.
                let is_heredoc_open = matches!(text.as_str(), "<<" | "<<-" | "<<~")
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
            flags: None, cv: None,
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

/// Phase 4e — flag bit set on `Token.flags` for range tokens
/// (`..` / `...`) under era ≥ 2.6 when they're followed by a
/// *closer* token (right paren/bracket/brace, comma, semicolon,
/// newline, or EOF) — i.e. the syntactic position of an "endless
/// range" like `(1..)` or `arr[2..]`.  Pre-2.6 these positions
/// were parse errors, so the flag stays off and any downstream
/// parser can reject them.
pub const ENDLESS_RANGE_FLAG: u32 = 1 << 1;

/// Phase 4d — true if `s` is `_1`, `_2`, …, `_9` exactly.  These
/// are the only nine numbered block parameter lexemes Ruby 2.7+
/// recognises.  `_0`, `_10`, `_1abc` etc. are NOT numbered params
/// and lex as regular Name tokens.
fn is_numbered_block_param(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 2 && bytes[0] == b'_' && (b'1'..=b'9').contains(&bytes[1])
}

/// Phase 4l — true iff `s` is the *body* of a radix-prefixed integer
/// (the part that ends up as a Name token after the lexer splits
/// `0xDEAD` into `Int("0")` + `Name("xDEAD")`).
///
/// Recognised shapes (the first char is the radix prefix letter,
/// the rest must be digits in that radix or underscore separators
/// — with at least one digit overall):
/// - `x`/`X` + hex digits (`[0-9a-fA-F_]+`, at least one digit)
/// - `b`/`B` + binary digits (`[01_]+`, at least one digit)
/// - `o`/`O` + octal digits (`[0-7_]+`, at least one digit)
/// - `d`/`D` + decimal digits (`[0-9_]+`, at least one digit)
fn is_radix_integer_body(s: &str) -> bool {
    let mut chars = s.chars();
    let prefix = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    let mut saw_digit = false;
    match prefix {
        'x' | 'X' => {
            for c in chars {
                if c.is_ascii_hexdigit() {
                    saw_digit = true;
                } else if c == '_' {
                    // separators OK
                } else {
                    return false;
                }
            }
        }
        'b' | 'B' => {
            for c in chars {
                if c == '0' || c == '1' {
                    saw_digit = true;
                } else if c == '_' {
                    // separators OK
                } else {
                    return false;
                }
            }
        }
        'o' | 'O' => {
            for c in chars {
                if ('0'..='7').contains(&c) {
                    saw_digit = true;
                } else if c == '_' {
                    // separators OK
                } else {
                    return false;
                }
            }
        }
        'd' | 'D' => {
            for c in chars {
                if c.is_ascii_digit() {
                    saw_digit = true;
                } else if c == '_' {
                    // separators OK
                } else {
                    return false;
                }
            }
        }
        _ => return false,
    }
    saw_digit
}

/// Phase 4k — true iff `s` looks like an exponent suffix that the
/// lexer's ident-body absorbed: starts with `e` or `E`, followed by
/// one or more digits (or underscore separators).  No sign character
/// — signed exponents (`e+10`) lex as three separate tokens and are
/// folded by the signed-exponent step of `fuse_float_literals`.
fn is_unsigned_exponent_lexeme(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    if bytes[0] != b'e' && bytes[0] != b'E' {
        return false;
    }
    let mut saw_digit = false;
    for &c in &bytes[1..] {
        if c.is_ascii_digit() {
            saw_digit = true;
        } else if c == b'_' {
            // separators OK, but must have at least one digit too
        } else {
            return false;
        }
    }
    saw_digit
}

fn is_heredoc_tag(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Phase 4o helper — strip the common leading-whitespace prefix from
/// every non-empty line of `body`.  The prefix is the minimum across
/// all non-empty lines (mirrors Ruby's `<<~` semantics).
///
/// Empty lines do NOT contribute to the prefix length (so a single
/// blank line in the middle doesn't force-zero the indent), but they
/// also don't get whitespace prepended.  Final trailing newline (if
/// any) is preserved.
fn strip_common_leading_whitespace(body: &str) -> String {
    // Determine the common prefix length.  We look at the leading
    // run of space/tab characters on every non-empty line and take
    // the minimum.
    let prefix_len = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .count()
        })
        .min()
        .unwrap_or(0);
    if prefix_len == 0 {
        return body.to_string();
    }
    // Strip exactly `prefix_len` leading whitespace chars from each
    // non-empty line.  Preserve the line endings (`body.lines()` drops
    // them, so we reconstruct from the original `body`).
    let mut out = String::with_capacity(body.len());
    let mut start = 0;
    let bytes = body.as_bytes();
    while start < bytes.len() {
        // Find the next `\n` (or EOF).
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'\n' {
            end += 1;
        }
        let line = &body[start..end];
        let line_is_empty = line.trim().is_empty();
        if line_is_empty {
            out.push_str(line);
        } else {
            // Strip up to prefix_len leading ws chars.  Use char
            // iteration to be safe with multi-byte UTF-8 (though
            // tabs/spaces are ASCII so the count == byte count in
            // practice).
            let mut stripped = line;
            for _ in 0..prefix_len {
                match stripped.chars().next() {
                    Some(c) if c == ' ' || c == '\t' => {
                        stripped = &stripped[c.len_utf8()..];
                    }
                    _ => break,
                }
            }
            out.push_str(stripped);
        }
        // Re-attach the newline (if present).
        if end < bytes.len() {
            out.push('\n');
            start = end + 1;
        } else {
            start = end;
        }
    }
    out
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
    fn end_marker_halts_token_stream() {
        // Phase FC — `__END__` on its own line ends the program; the
        // trailing data section must NOT be tokenized as code.
        let toks = tokenize_ruby("foo\n__END__\nthis is data, not code!\n");
        let vals: Vec<&str> = toks.iter().map(|t| t.value.as_str()).collect();
        assert!(vals.contains(&"foo"), "expected the code before __END__");
        // None of the data-section words leak into the token stream, and
        // the `__END__` marker itself is consumed (not emitted).
        for leaked in ["__END__", "this", "data", "code", "not"] {
            assert!(
                !vals.contains(&leaked),
                "data-section token `{leaked}` leaked into the stream: {vals:?}"
            );
        }
    }

    #[test]
    fn end_marker_at_eof_without_trailing_newline() {
        // `__END__` as the final line with no trailing newline still
        // terminates cleanly (only the preceding code is tokenized).
        let toks = tokenize_ruby("x = 1\n__END__");
        let vals: Vec<&str> = toks.iter().map(|t| t.value.as_str()).collect();
        assert!(vals.contains(&"x") && vals.contains(&"1"));
        assert!(!vals.contains(&"__END__"), "marker should be consumed");
    }

    #[test]
    fn end_marker_requires_column_zero() {
        // An indented `  __END__` is NOT the terminator (Ruby requires
        // column 0); it lexes as an ordinary Name and the following line
        // is still tokenized as code.
        let toks = tokenize_ruby("  __END__\nfoo\n");
        let vals: Vec<&str> = toks.iter().map(|t| t.value.as_str()).collect();
        assert!(
            vals.contains(&"__END__"),
            "indented __END__ should lex as a normal Name: {vals:?}"
        );
        assert!(
            vals.contains(&"foo"),
            "code after an indented (non-marker) __END__ must survive: {vals:?}"
        );
    }

    #[test]
    fn end_marker_not_triggered_mid_line() {
        // `__END__` not at line start (here as an assignment RHS) is an
        // ordinary Name, not the terminator.
        let toks = tokenize_ruby("y = __END__\n");
        let vals: Vec<&str> = toks.iter().map(|t| t.value.as_str()).collect();
        assert!(
            vals.contains(&"y") && vals.contains(&"__END__"),
            "mid-line __END__ must be a normal Name: {vals:?}"
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
    // Phase 4g — 2.0 `%i[]` / `%I[]` symbol-array percent literals.
    // -----------------------------------------------------------------

    #[test]
    fn era_2_0_lexes_percent_i_array() {
        let toks = tokenize_ruby_for_version("%i[a b c]", "2.0").unwrap();
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["%i[a b c]"]);
    }

    #[test]
    fn era_2_0_lexes_percent_big_i_array() {
        let toks = tokenize_ruby_for_version("%I[a b c]", "2.0").unwrap();
        let strings: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::String)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(strings, vec!["%I[a b c]"]);
    }

    #[test]
    fn percent_i_unterminated_records_diagnostic() {
        let (toks, diags) = tokenize_ruby_diag("%i[a b c");
        assert!(diags.iter().any(|d| d.code == "unterminated_percent_i"));
        // We still emit some token shape so the parser has something to
        // chew on.
        let _ = toks;
    }

    #[test]
    fn percent_modulo_still_works() {
        // `%` followed by something other than w/q/i/I should fall
        // back to the modulo operator (no percent literal).
        let toks = tokenize_ruby_for_version("5 % 2", "2.0").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(values.contains(&"%"));
    }

    // -----------------------------------------------------------------
    // Phase 4f — 2.1 numeric suffixes `r` / `i`.
    // -----------------------------------------------------------------

    #[test]
    fn era_2_1_fuses_rational_suffix() {
        let toks = tokenize_ruby_for_version("2r + 3r", "2.1").unwrap();
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["2r", "3r"]);
    }

    #[test]
    fn era_2_1_fuses_complex_suffix() {
        let toks = tokenize_ruby_for_version("4i", "2.1").unwrap();
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["4i"]);
    }

    #[test]
    fn era_2_0_does_not_fuse_numeric_suffixes() {
        // 2.0 < 2.1 — the suffixes stay as separate Name tokens
        // (which is what Ruby 2.0 actually did).
        let toks = tokenize_ruby_for_version("2r", "2.0").unwrap();
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["2"]);
        let has_r = toks
            .iter()
            .any(|t| t.type_ == TokenType::Name && t.value == "r");
        assert!(has_r, "2.0 should leave `r` as a separate Name");
    }

    #[test]
    fn era_2_1_does_not_fuse_with_whitespace_before_suffix() {
        // `2 r` has whitespace between — not a numeric suffix.
        let toks = tokenize_ruby_for_version("2 r", "2.1").unwrap();
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["2"]);
    }

    #[test]
    fn era_2_1_does_not_fuse_with_unrelated_suffix() {
        // `2x` — `x` isn't a recognised suffix.  Stays split.
        let toks = tokenize_ruby_for_version("2x", "2.1").unwrap();
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["2"]);
    }

    // -----------------------------------------------------------------
    // Phase 4e — range fusions `..` / `...` and 2.6 endless ranges.
    // -----------------------------------------------------------------

    fn has_endless_range_flag(t: &Token) -> bool {
        (t.flags.unwrap_or(0) & ENDLESS_RANGE_FLAG) != 0
    }

    #[test]
    fn range_dotdot_fuses_unconditionally() {
        // `..` is range syntax since Ruby 1.0 — every era fuses it.
        for era in ["1.0", "1.8", "2.0", "3.3"] {
            let toks = tokenize_ruby_for_version("1..5", era).unwrap();
            let values: Vec<&str> = toks
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .map(|t| t.value.as_str())
                .collect();
            assert!(
                values.contains(&".."),
                "era {era} should fuse `..` (got {:?})",
                values
            );
            // No bare `.` should remain in the fused range.
            assert_eq!(values.iter().filter(|v| **v == ".").count(), 0);
        }
    }

    #[test]
    fn range_dotdotdot_fuses_unconditionally() {
        let toks = tokenize_ruby_for_version("1...5", "1.8").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert!(values.contains(&"..."));
        assert!(!values.contains(&".."));
    }

    #[test]
    fn era_2_6_flags_endless_range_before_rparen() {
        let toks = tokenize_ruby_for_version("(1..)", "2.6").unwrap();
        let any_flagged = toks
            .iter()
            .any(|t| t.value == ".." && has_endless_range_flag(t));
        assert!(any_flagged, "expected endless-range flag on `..`");
    }

    #[test]
    fn era_2_3_does_not_flag_endless_range() {
        // 2.3 < 2.6 — the flag must stay off so callers reject
        // endless ranges as parse errors (which they were pre-2.6).
        let toks = tokenize_ruby_for_version("(1..)", "2.3").unwrap();
        let any_flagged = toks.iter().any(has_endless_range_flag);
        assert!(!any_flagged, "2.3 must not flag endless ranges");
    }

    #[test]
    fn era_2_6_does_not_flag_normal_range() {
        // `1..5` has a non-closer follower (`5`), so even under 2.6
        // it isn't an endless range — flag stays clear.
        let toks = tokenize_ruby_for_version("1..5", "2.6").unwrap();
        let any_flagged = toks.iter().any(has_endless_range_flag);
        assert!(!any_flagged, "normal range must not get endless flag");
    }

    #[test]
    fn era_2_6_flags_endless_range_before_newline_and_comma() {
        for src in ["1..\n", "x = [1.., 2]\n"] {
            let toks = tokenize_ruby_for_version(src, "2.6").unwrap();
            let any_flagged = toks
                .iter()
                .any(|t| t.value == ".." && has_endless_range_flag(t));
            assert!(
                any_flagged,
                "expected endless-range flag in source `{}`",
                src
            );
        }
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

    // -----------------------------------------------------------------
    // Phase 4h — 1.9.1 hash shorthand `{a: 1}` (Name followed by `:`
    // at hash-key position).
    //
    // Design note (load-bearing for backends): the hash-shorthand
    // syntax `{a: 1}` was introduced in Ruby 1.9.1.  Pre-1.9.1, the
    // only valid hash literal was the hash-rocket form `{:a => 1}`.
    //
    // At the *lexer* level, no token-level change is needed —
    //   `{a: 1}` lexes uniformly across all 15 eras as
    //   `LBrace Name(a) Colon Number(1) RBrace`
    // because the colon is just a standalone `Colon` token in every
    // era.  Real Ruby differentiates the two forms at the *parser*
    // level (era-gated `hash_entry` production), which we already
    // ship as Phase 6d: `hash_entry = NAME COLON expression | …`.
    //
    // So this phase is intentionally a *no-op at token granularity*.
    // The era-gating belongs at the parser layer (already shipped)
    // and at a later "reject-pre-1.9.1-hash-shorthand" pass that runs
    // after the AST is in hand.  The tests below pin the invariant
    // that the token stream for `{a: 1}` is era-independent so
    // backends can trust it.
    // -----------------------------------------------------------------

    #[test]
    fn hash_shorthand_lexes_uniformly_across_all_eras() {
        // The same source `{a: 1}` must produce the identical token
        // kind stream under every supported era.  If a future phase
        // ever needs to differentiate at the lexer level (e.g. a new
        // token kind for hash-shorthand colon), this assertion is the
        // canary that will fail loudly.
        let src = "{a: 1}";
        let mut baseline: Option<Vec<TokenType>> = None;
        for &v in ERA_VERSIONS {
            let toks = tokenize_ruby_for_version(src, v)
                .unwrap_or_else(|e| panic!("version {v} failed: {e}"));
            let kinds: Vec<TokenType> = toks
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .map(|t| t.type_)
                .collect();
            match baseline {
                None => baseline = Some(kinds),
                Some(ref b) => {
                    assert_eq!(
                        &kinds, b,
                        "era {v} produced a divergent token stream for `{src}`",
                    );
                }
            }
        }
        // Sanity-check the baseline shape: LBrace Name Colon Number RBrace
        let kinds = baseline.expect("at least one era ran");
        assert_eq!(
            kinds,
            vec![
                TokenType::LBrace,
                TokenType::Name,
                TokenType::Colon,
                TokenType::Number,
                TokenType::RBrace,
            ],
            "baseline token shape unexpected"
        );
    }

    #[test]
    fn hash_shorthand_with_two_entries_lexes_uniformly() {
        // Multi-entry hash literal — the comma is era-independent too.
        let src = "{a: 1, b: 2}";
        for &v in ERA_VERSIONS {
            let toks = tokenize_ruby_for_version(src, v).unwrap();
            let kinds: Vec<TokenType> = toks
                .iter()
                .filter(|t| t.type_ != TokenType::Eof)
                .map(|t| t.type_)
                .collect();
            assert_eq!(
                kinds,
                vec![
                    TokenType::LBrace,
                    TokenType::Name,
                    TokenType::Colon,
                    TokenType::Number,
                    TokenType::Comma,
                    TokenType::Name,
                    TokenType::Colon,
                    TokenType::Number,
                    TokenType::RBrace,
                ],
                "era {v} produced divergent token stream"
            );
        }
    }

    #[test]
    fn hash_rocket_form_lexes_uniformly_across_all_eras() {
        // The classic hash-rocket form `{:a => 1}` predates 1.9.1
        // shorthand and remains valid in every Ruby version.  Lexer
        // tokenisation must also be era-independent.  We just spot-
        // check that `=>` appears as part of the stream.
        let src = "{:a => 1}";
        for &v in ERA_VERSIONS {
            let toks = tokenize_ruby_for_version(src, v).unwrap();
            let values: Vec<&str> =
                toks.iter().filter(|t| t.type_ != TokenType::Eof).map(|t| t.value.as_str()).collect();
            assert!(
                values.contains(&"=>"),
                "era {v} did not produce a `=>` token in {values:?}",
            );
        }
    }

    #[test]
    fn hash_shorthand_and_rocket_differ_only_in_value_tokens() {
        // The two forms produce different token streams (hash-rocket
        // has `=>`, shorthand has `Colon`) but each form is invariant
        // across eras.  This is the load-bearing guarantee parsers
        // depend on when era-gating hash shorthand acceptance.
        let shorthand = tokenize_ruby_for_version("{a: 1}", "1.9.1").unwrap();
        let rocket = tokenize_ruby_for_version("{:a => 1}", "1.9.1").unwrap();
        let shorthand_has_arrow = shorthand
            .iter()
            .any(|t| t.value == "=>");
        let rocket_has_arrow = rocket.iter().any(|t| t.value == "=>");
        assert!(!shorthand_has_arrow);
        assert!(rocket_has_arrow);
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

    // -----------------------------------------------------------------
    // Phase 4i / 4j — instance vars `@x`, class vars `@@x`, globals `$x`.
    // -----------------------------------------------------------------
    //
    // All three sigil-prefixed lexemes have existed since Ruby 1.0;
    // they emit as `TokenType::Name` carrying the *full lexeme*
    // (sigils included).  Downstream code (parser / SIR lowerer)
    // dispatches by the leading character.  These tests are NOT era-
    // gated — they should produce the same shape across every era.

    #[test]
    fn lexes_instance_variable() {
        let toks = tokenize_ruby("@count");
        let names: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["@count"]);
    }

    #[test]
    fn lexes_class_variable() {
        let toks = tokenize_ruby("@@all");
        let names: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["@@all"]);
    }

    #[test]
    fn lexes_global_variable() {
        let toks = tokenize_ruby("$LOAD_PATH");
        let names: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["$LOAD_PATH"]);
    }

    #[test]
    fn ivar_with_digits_and_underscore() {
        // `@foo_bar2` — ident-body rules permit digits/underscore
        // after the first ident-starter.
        let toks = tokenize_ruby("@foo_bar2");
        let names: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["@foo_bar2"]);
    }

    #[test]
    fn sigil_vars_in_assignment_context() {
        // `@x = 1` → three tokens: Name(@x), Equals, Number(1).
        let toks = tokenize_ruby("@x = 1");
        let kinds: Vec<(TokenType, &str)> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| (t.type_, t.value.as_str()))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (TokenType::Name, "@x"),
                (TokenType::Equals, "="),
                (TokenType::Number, "1"),
            ]
        );
    }

    #[test]
    fn invalid_ivar_falls_back_to_op_with_diagnostic() {
        // `@1` (digit after `@`) is not a valid ivar in Ruby.  We
        // record a diagnostic and emit `@` as a bare Op token so the
        // parser still gets a clean stream.
        let (toks, diags) = tokenize_ruby_diag("@1");
        assert!(diags.iter().any(|d| d.code == "invalid_ivar"));
        // First non-Eof token should be the bare `@` Op.
        let first = toks
            .iter()
            .find(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .expect("expected at least one non-Eof token");
        assert_eq!(first.value, "@");
    }

    #[test]
    fn invalid_gvar_falls_back_to_op_with_diagnostic() {
        // `$ ` (dollar followed by space) — v0 doesn't recognise the
        // punctuation globals, so this records `invalid_gvar` and
        // emits `$` as a bare Op.
        let (toks, diags) = tokenize_ruby_diag("$ x");
        assert!(diags.iter().any(|d| d.code == "invalid_gvar"));
        let first = toks
            .iter()
            .find(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .expect("expected at least one non-Eof token");
        assert_eq!(first.value, "$");
    }

    #[test]
    fn sigil_vars_unchanged_across_all_eras() {
        // Sigil vars have been in Ruby since 1.0, so every era from
        // 1.8 forward should produce the identical token shape.
        let src = "@a + @@b + $c";
        let baseline_names: Vec<String> = tokenize_ruby_for_version(src, "1.8")
            .unwrap()
            .into_iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value)
            .collect();
        assert_eq!(baseline_names, vec!["@a", "@@b", "$c"]);
        for v in machine::ERA_VERSIONS {
            let names: Vec<String> = tokenize_ruby_for_version(src, v)
                .unwrap()
                .into_iter()
                .filter(|t| t.type_ == TokenType::Name)
                .map(|t| t.value)
                .collect();
            assert_eq!(
                names, baseline_names,
                "sigil-var lexing diverged in era {v}"
            );
        }
    }

    // -----------------------------------------------------------------
    // Phase 4k — float literals (`1.5`, `1e10`, `1.5e-3`, …)
    // -----------------------------------------------------------------
    //
    // Floats have been in Ruby since 1.0 (no era gate).  The state
    // machine emits the simple shape (Int, Dot, Int, …), and the
    // `fuse_float_literals` post-pass collapses sequences into one
    // `Number` token.  These tests pin the canonical fusions.

    fn numbers(src: &str) -> Vec<String> {
        tokenize_ruby(src)
            .into_iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value)
            .collect()
    }

    #[test]
    fn lexes_simple_float() {
        // `1.5` → one Number token, value "1.5".
        assert_eq!(numbers("1.5"), vec!["1.5"]);
    }

    #[test]
    fn lexes_float_in_assignment() {
        // `x = 1.5` — token stream is Name, Equals, Number("1.5"), …
        let toks = tokenize_ruby("x = 1.5");
        let interesting: Vec<(TokenType, &str)> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| (t.type_, t.value.as_str()))
            .collect();
        assert_eq!(
            interesting,
            vec![
                (TokenType::Name, "x"),
                (TokenType::Equals, "="),
                (TokenType::Number, "1.5"),
            ]
        );
    }

    #[test]
    fn lexes_float_with_unsigned_exponent() {
        // `1e10` — one Number, "1e10".
        assert_eq!(numbers("1e10"), vec!["1e10"]);
        // `2E5` — capital E also OK.
        assert_eq!(numbers("2E5"), vec!["2E5"]);
        // `1.5e10` — combined dot + exponent.
        assert_eq!(numbers("1.5e10"), vec!["1.5e10"]);
    }

    #[test]
    fn lexes_float_with_signed_exponent() {
        // `1.5e-3` — four-token fusion: Number, Name("e"), Minus, Int.
        assert_eq!(numbers("1.5e-3"), vec!["1.5e-3"]);
        // `1e+10` — same path with leading bare int (no dot).
        assert_eq!(numbers("1e+10"), vec!["1e+10"]);
    }

    #[test]
    fn float_does_not_swallow_range_operator() {
        // `1..5` is a range, not `1.` then `.5`.  The range pass runs
        // first and fuses the two dots into `Name("..")`, so float
        // fusion never sees an `Int Dot Int` shape here.
        let toks = tokenize_ruby("1..5");
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["1", "5"]);
        let has_dotdot = toks
            .iter()
            .any(|t| t.type_ == TokenType::Name && t.value == "..");
        assert!(has_dotdot, "expected `..` Name token, got {toks:?}");
    }

    #[test]
    fn float_does_not_swallow_method_call() {
        // `1.method` — `1.` is an Int followed by a Dot; `method` is
        // a NAME.  Since `method` starts with `m` (not a digit), the
        // Int Dot Int fusion shouldn't fire.
        let toks = tokenize_ruby("1.method");
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["1"]);
        let names: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["method"]);
    }

    #[test]
    fn float_requires_no_whitespace() {
        // `1 . 5` with spaces — three separate tokens, NOT a float.
        let toks = tokenize_ruby("1 . 5");
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["1", "5"]);
    }

    #[test]
    fn float_lexes_uniformly_across_all_eras() {
        // Floats have been in Ruby since 1.0; lexing must be era-
        // invariant.  Pin the same Number stream across every era.
        let src = "x = 1.5 + 2e10";
        let baseline: Vec<String> = tokenize_ruby_for_version(src, "1.8")
            .unwrap()
            .into_iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value)
            .collect();
        assert_eq!(baseline, vec!["1.5", "2e10"]);
        for v in machine::ERA_VERSIONS {
            let nums: Vec<String> = tokenize_ruby_for_version(src, v)
                .unwrap()
                .into_iter()
                .filter(|t| t.type_ == TokenType::Number)
                .map(|t| t.value)
                .collect();
            assert_eq!(nums, baseline, "float lexing diverged in era {v}");
        }
    }

    #[test]
    fn is_unsigned_exponent_lexeme_smoke() {
        assert!(is_unsigned_exponent_lexeme("e10"));
        assert!(is_unsigned_exponent_lexeme("E5"));
        assert!(is_unsigned_exponent_lexeme("e1_000"));
        assert!(!is_unsigned_exponent_lexeme("e"));        // no digits
        assert!(!is_unsigned_exponent_lexeme("e+10"));     // signed → 3 tokens
        assert!(!is_unsigned_exponent_lexeme("foo"));      // doesn't start with e/E
        assert!(!is_unsigned_exponent_lexeme(""));         // empty
        assert!(!is_unsigned_exponent_lexeme("ex"));       // non-digit body
    }

    // -----------------------------------------------------------------
    // Phase 4l — radix-prefixed integers (`0x1F`, `0b1010`, `0o17`, `0d10`)
    // -----------------------------------------------------------------
    //
    // Pre-1.0 Ruby (no era gate).  Fuse `Int("0") Name("xDEAD")` into
    // one `Number("0xDEAD")` token.

    #[test]
    fn lexes_hex_integer() {
        assert_eq!(numbers("0x1F"), vec!["0x1F"]);
        assert_eq!(numbers("0xDEAD_BEEF"), vec!["0xDEAD_BEEF"]);
        assert_eq!(numbers("0Xff"), vec!["0Xff"]);  // capital X
    }

    #[test]
    fn lexes_binary_integer() {
        assert_eq!(numbers("0b1010"), vec!["0b1010"]);
        assert_eq!(numbers("0B1010_1100"), vec!["0B1010_1100"]);
    }

    #[test]
    fn lexes_octal_integer() {
        assert_eq!(numbers("0o755"), vec!["0o755"]);
        assert_eq!(numbers("0O17"), vec!["0O17"]);
    }

    #[test]
    fn lexes_decimal_explicit_radix() {
        // `0d42` — explicit decimal prefix.
        assert_eq!(numbers("0d42"), vec!["0d42"]);
        assert_eq!(numbers("0D100_000"), vec!["0D100_000"]);
    }

    #[test]
    fn invalid_hex_does_not_fuse() {
        // `0xZZ` — Z isn't a hex digit, so the fusion shouldn't fire.
        // Token stream stays as Int(0), Name("xZZ").
        let toks = tokenize_ruby("0xZZ");
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["0"]);
        let names: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert!(names.contains(&"xZZ"));
    }

    #[test]
    fn radix_integer_requires_no_whitespace() {
        // `0 x1F` (with space) — separate tokens, NO fusion.
        let toks = tokenize_ruby("0 x1F");
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["0"]);
    }

    #[test]
    fn radix_does_not_swallow_method_call() {
        // `0.method` is Int, Dot, Name — not a radix integer.
        let toks = tokenize_ruby("0.method");
        let nums: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(nums, vec!["0"]);
        let has_dot = toks.iter().any(|t| t.type_ == TokenType::Dot);
        assert!(has_dot);
    }

    #[test]
    fn radix_integers_lex_uniformly_across_all_eras() {
        // Radix prefixes are pre-1.0 — era-invariant.
        let src = "x = 0x1F + 0b1010 + 0o17 + 0d42";
        let baseline: Vec<String> = tokenize_ruby_for_version(src, "1.8")
            .unwrap()
            .into_iter()
            .filter(|t| t.type_ == TokenType::Number)
            .map(|t| t.value)
            .collect();
        assert_eq!(baseline, vec!["0x1F", "0b1010", "0o17", "0d42"]);
        for v in machine::ERA_VERSIONS {
            let nums: Vec<String> = tokenize_ruby_for_version(src, v)
                .unwrap()
                .into_iter()
                .filter(|t| t.type_ == TokenType::Number)
                .map(|t| t.value)
                .collect();
            assert_eq!(nums, baseline, "radix-int lexing diverged in era {v}");
        }
    }

    #[test]
    fn is_radix_integer_body_smoke() {
        assert!(is_radix_integer_body("x1F"));
        assert!(is_radix_integer_body("XDEAD"));
        assert!(is_radix_integer_body("xDEAD_BEEF"));
        assert!(is_radix_integer_body("b1010"));
        assert!(is_radix_integer_body("B10_10"));
        assert!(is_radix_integer_body("o755"));
        assert!(is_radix_integer_body("O17"));
        assert!(is_radix_integer_body("d42"));
        assert!(is_radix_integer_body("D100"));
        // Negative cases
        assert!(!is_radix_integer_body(""));      // empty
        assert!(!is_radix_integer_body("x"));     // no digits
        assert!(!is_radix_integer_body("x___"));  // only separators, no digit
        assert!(!is_radix_integer_body("b2"));    // 2 isn't binary
        assert!(!is_radix_integer_body("o9"));    // 9 isn't octal
        assert!(!is_radix_integer_body("xZZ"));   // Z isn't hex
        assert!(!is_radix_integer_body("foo"));   // wrong prefix letter
        assert!(!is_radix_integer_body("e10"));   // e is exponent, not radix
    }

    // -----------------------------------------------------------------
    // Phase 4m — backtick command literals `` `cmd args` ``
    // -----------------------------------------------------------------
    //
    // The lexer emits a single `TokenType::String` token whose value is
    // the verbatim source including the surrounding backticks
    // (`` `ls -la` `` → value `` `ls -la` ``).  Parser code can
    // distinguish backtick literals from plain strings by inspecting
    // the leading character — same trick used by percent literals and
    // heredocs.

    /// Helper: tokens whose `value` starts with a backtick.
    fn backtick_values(toks: &[Token]) -> Vec<String> {
        toks.iter()
            .filter(|t| t.type_ == TokenType::String && t.value.starts_with('`'))
            .map(|t| t.value.clone())
            .collect()
    }

    #[test]
    fn backtick_simple_command_lexes_as_string_with_backticks() {
        let toks = tokenize_ruby_for_version("`ls -la`", "1.8").unwrap();
        assert_eq!(backtick_values(&toks), vec!["`ls -la`"]);
    }

    #[test]
    fn backtick_empty_body_lexes_to_two_backticks() {
        // Empty body — opening and closing backtick are adjacent.
        let toks = tokenize_ruby_for_version("``", "1.8").unwrap();
        assert_eq!(backtick_values(&toks), vec!["``"]);
    }

    #[test]
    fn backtick_escape_sequences_resolved_in_body() {
        // `\n`, `\t`, `\r`, `\\` get resolved to the escape characters
        // (same as `string_d_escape`).  Backtick-escape `` \` `` lets
        // you embed a literal backtick.
        let toks = tokenize_ruby_for_version(r#"`echo \`hi\`\n`"#, "1.8").unwrap();
        let v = &backtick_values(&toks)[0];
        // The escaped backticks become literal backticks; \n becomes a
        // newline character.  Outer wrapping backticks remain.
        assert_eq!(v, "`echo `hi`\n`");
    }

    #[test]
    fn backtick_multiline_command_keeps_newlines() {
        // Real ruby allows multi-line `` `…` `` — the body captures
        // newlines verbatim (same shape as double-quoted strings).
        let src = "`ls\\\nfoo`"; // backslash-newline continuation in command
        // Without continuation handling, we just verify a literal
        // newline inside the body survives.
        let toks = tokenize_ruby_for_version("`line1\nline2`", "1.8").unwrap();
        let v = &backtick_values(&toks)[0];
        assert!(v.contains("line1\nline2"), "expected newline inside body, got {v:?}");
        // Sanity check the source-based test above did not crash.
        let _ = tokenize_ruby_for_version(src, "1.8").unwrap();
    }

    #[test]
    fn backtick_lexing_is_era_invariant() {
        // Backticks have been in Ruby since 1.0 — every era produces
        // the same token shape.
        let src = "`pwd`";
        let baseline = backtick_values(&tokenize_ruby_for_version(src, "1.8").unwrap());
        assert_eq!(baseline, vec!["`pwd`"]);
        for v in machine::ERA_VERSIONS {
            let actual = backtick_values(&tokenize_ruby_for_version(src, v).unwrap());
            assert_eq!(actual, baseline, "backtick lexing diverged in era {v}");
        }
    }

    #[test]
    fn backtick_does_not_swallow_following_tokens() {
        // After the closing backtick, the dispatcher resumes normal
        // tokenizing — `puts \`pwd\``  followed by  `+`  `1`  should
        // yield Name(puts), String(`pwd`), Op(+), Int(1).
        let toks = tokenize_ruby_for_version("`pwd` + 1", "1.8").unwrap();
        let kinds: Vec<TokenType> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.type_)
            .collect();
        // Expect: String, Plus, Number  (whitespace skipped).
        assert_eq!(
            kinds,
            vec![TokenType::String, TokenType::Plus, TokenType::Number],
            "got tokens {toks:?}"
        );
    }

    // -----------------------------------------------------------------
    // Phase 4o — heredoc opener variants `<<-TAG` and `<<~TAG`
    // -----------------------------------------------------------------
    //
    // Builds on the existing Phase 3c `<<TAG` plain heredoc support.
    //
    //   <<-TAG  (1.9+)   indent-tolerant terminator (body verbatim)
    //   <<~TAG  (2.3+)   indent-tolerant terminator + indent-stripped body
    //
    // The token shape is unchanged: a single `TokenType::String` token
    // whose value is the reconstructed source (`<<TAG\nBODY\nTAG`,
    // `<<-TAG\n…\nTAG`, or `<<~TAG\n…\nTAG`).  The parser distinguishes
    // by inspecting the leading `<<` / `<<-` / `<<~` prefix.

    /// Helper: pull all heredoc String tokens (values starting with `<<`).
    fn heredoc_values(toks: &[Token]) -> Vec<String> {
        toks.iter()
            .filter(|t| t.type_ == TokenType::String && t.value.starts_with("<<"))
            .map(|t| t.value.clone())
            .collect()
    }

    #[test]
    fn heredoc_dash_indent_terminator_allows_leading_whitespace() {
        // `<<-EOF` permits leading whitespace on the terminator line.
        let src = "x = <<-EOF\n  body line\n  EOF\n";
        let toks = tokenize_ruby_for_version(src, "1.8").unwrap();
        let here = heredoc_values(&toks);
        assert_eq!(here.len(), 1, "expected one heredoc, got {here:?}");
        assert!(here[0].starts_with("<<-EOF"), "got {here:?}");
        // Body retains the leading whitespace (DashIndent does NOT
        // strip — only TildeIndent does).
        assert!(here[0].contains("  body line"), "got {here:?}");
    }

    #[test]
    fn heredoc_tilde_indent_strips_common_leading_whitespace() {
        // `<<~EOF` strips the common leading-ws prefix from every
        // non-empty line.  Input has 4 spaces of common prefix.
        let src = "x = <<~EOF\n    body line one\n    body line two\n    EOF\n";
        let toks = tokenize_ruby_for_version(src, "1.8").unwrap();
        let here = heredoc_values(&toks);
        assert_eq!(here.len(), 1);
        assert!(here[0].starts_with("<<~EOF"));
        // Body lines should have the 4-space prefix stripped.
        assert!(here[0].contains("body line one\n"), "got {:?}", here[0]);
        assert!(!here[0].contains("    body line one"), "got {:?}", here[0]);
    }

    #[test]
    fn heredoc_tilde_indent_uses_minimum_prefix_across_lines() {
        // Mixed indents — body takes the smallest leading-ws prefix.
        // 2 spaces on first line, 4 on second; common = 2 spaces.
        let src = "x = <<~EOF\n  two\n    four\n  EOF\n";
        let toks = tokenize_ruby_for_version(src, "1.8").unwrap();
        let here = heredoc_values(&toks);
        assert_eq!(here.len(), 1);
        // After stripping 2 spaces: "two\n  four\n".
        assert!(here[0].contains("two\n  four\n"), "got {:?}", here[0]);
    }

    #[test]
    fn heredoc_plain_form_still_requires_exact_terminator() {
        // `<<EOF` still demands the terminator at column 0.
        // Leading whitespace on `EOF` line means the heredoc never
        // terminates — diagnostic + body absorbs everything.
        let src = "x = <<EOF\nbody\n  EOF\n";
        let mut lx = RubyLexer::new("1.8").expect("lexer build");
        lx.push(src).unwrap();
        lx.finish().unwrap();
        let diags = lx.diagnostics();
        assert!(
            diags.iter().any(|d| d.code.contains("unterminated-heredoc")),
            "expected unterminated-heredoc diagnostic for indented terminator after <<EOF, got {:?}",
            diags
        );
    }

    #[test]
    fn heredoc_dash_and_tilde_variants_lex_uniformly_across_eras() {
        // The new openers should produce a heredoc String in every
        // era ≥ 1.8 (even though `<<-` was 1.9 and `<<~` was 2.3 in
        // real Ruby — the lexer is permissive; era gating belongs to
        // downstream tooling if needed).
        let src = "x = <<-EOF\nhello\n  EOF\n";
        let baseline = heredoc_values(&tokenize_ruby_for_version(src, "1.8").unwrap());
        assert_eq!(baseline.len(), 1);
        for v in machine::ERA_VERSIONS {
            let actual = heredoc_values(&tokenize_ruby_for_version(src, v).unwrap());
            assert_eq!(actual, baseline, "heredoc lexing diverged in era {v}");
        }
    }

    // -----------------------------------------------------------------
    // Phase 4n — `%r{regex}`, `%s{symbol}`, `%x{cmd}` percent literals
    // -----------------------------------------------------------------
    //
    // The lexer emits each as a `TokenType::String` token whose value
    // is the verbatim source (`%r{pat}`, `%s{name}`, `%x{cmd}`).
    // Parser code distinguishes them from plain strings — and from
    // each other — by the leading `%` + type letter.  Same encoding
    // strategy as the existing `%w[…]` / `%q{…}` / `%i[…]` family.

    /// Helper: pull all `TokenType::String` lexemes that start with a
    /// given percent prefix (e.g. `"%r"` for regex literals).
    fn percent_values(toks: &[Token], prefix: &str) -> Vec<String> {
        toks.iter()
            .filter(|t| t.type_ == TokenType::String && t.value.starts_with(prefix))
            .map(|t| t.value.clone())
            .collect()
    }

    // -----------------------------------------------------------------
    // Phase 6p companion — fuse compound-assign operators
    // -----------------------------------------------------------------
    //
    // `fuse_compound_assigns` folds adjacent `Op` + `Equals` token
    // pairs into a single Name-typed token carrying the fused
    // operator value (`+=`, `-=`, `*=`, `/=`, `||=`, `&&=`).  For
    // `/=`, `push` sets `suppress_regex_open` so the slash isn't
    // mis-interpreted as a regex opener — without that gate `x /= 1`
    // would lex as `x / <regex starting `=`...>` and never terminate.

    #[test]
    fn compound_assign_arithmetic_ops_fuse_into_single_token() {
        for (src, fused) in [
            ("x += 1", "+="),
            ("x -= 1", "-="),
            ("x *= 1", "*="),
            ("x /= 1", "/="),
        ] {
            let toks = tokenize_ruby_for_version(src, "1.8").unwrap();
            let has_fused = toks
                .iter()
                .any(|t| t.type_ == TokenType::Name && t.value == fused);
            assert!(has_fused, "expected {fused} token in {src:?}, got {toks:?}");
        }
    }

    #[test]
    fn compound_assign_logical_ops_fuse_into_single_token() {
        for (src, fused) in [("x ||= 1", "||="), ("x &&= 1", "&&=")] {
            let toks = tokenize_ruby_for_version(src, "1.8").unwrap();
            let has_fused = toks
                .iter()
                .any(|t| t.type_ == TokenType::Name && t.value == fused);
            assert!(has_fused, "expected {fused} token in {src:?}, got {toks:?}");
        }
    }

    #[test]
    fn compound_assign_does_not_fuse_with_whitespace_gap() {
        // `x + = 1` with a space between `+` and `=` is two tokens —
        // the fusion gate requires no whitespace between op and `=`.
        let toks = tokenize_ruby_for_version("x + = 1", "1.8").unwrap();
        // No fused `+=` token expected; instead we see Plus + Equals.
        let has_fused = toks
            .iter()
            .any(|t| t.type_ == TokenType::Name && t.value == "+=");
        assert!(!has_fused, "spaced `x + = 1` should NOT fuse, got {toks:?}");
        let has_plus = toks.iter().any(|t| t.type_ == TokenType::Plus);
        let has_eq = toks.iter().any(|t| t.type_ == TokenType::Equals);
        assert!(has_plus && has_eq, "expected separate Plus + Equals, got {toks:?}");
    }

    // Phase 8a-2 (FC) — `>>` and `>>=` pre-fusion.

    #[test]
    fn right_shift_compound_assign_fuses_into_single_token() {
        // `x >>= 5` should produce a single `Name(">>=")` token —
        // not the pre-fusion shape `>`, `>=`.
        let toks = tokenize_ruby_for_version("x >>= 5", "1.8").unwrap();
        let has_fused = toks
            .iter()
            .any(|t| t.type_ == TokenType::Name && t.value == ">>=");
        assert!(
            has_fused,
            "expected `>>=` Name token after fusion, got {toks:?}"
        );
        // And no orphan `>=` left over.
        let has_orphan_ge = toks
            .iter()
            .any(|t| t.type_ == TokenType::Name && t.value == ">=");
        assert!(
            !has_orphan_ge,
            "expected no orphan `>=` after `>>=` fusion, got {toks:?}"
        );
    }

    #[test]
    fn right_shift_binary_operator_fuses_into_single_token() {
        // `5 >> 3` should produce a single `Name(">>")` — not two
        // separate `Name(">")` tokens.
        let toks = tokenize_ruby_for_version("5 >> 3", "1.8").unwrap();
        let shift_count = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name && t.value == ">>")
            .count();
        let bare_gt_count = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name && t.value == ">")
            .count();
        assert_eq!(shift_count, 1, "expected exactly one `>>` token in {toks:?}");
        assert_eq!(bare_gt_count, 0, "expected no bare `>` tokens in {toks:?}");
    }

    #[test]
    fn right_shift_fusion_respects_whitespace_gap() {
        // `5 > > 3` (with a space between the two `>`s) must NOT fuse.
        let toks = tokenize_ruby_for_version("5 > > 3", "1.8").unwrap();
        let shift_count = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name && t.value == ">>")
            .count();
        let gt_count = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name && t.value == ">")
            .count();
        assert_eq!(shift_count, 0, "should NOT fuse across whitespace: {toks:?}");
        assert_eq!(gt_count, 2, "expected two bare `>` tokens: {toks:?}");
    }

    #[test]
    fn right_shift_fusion_leaves_unrelated_ge_alone() {
        // `a >= b` must stay as a single `Name(">=")` — only the
        // post-`>` form is folded.
        let toks = tokenize_ruby_for_version("a >= b", "1.8").unwrap();
        let ge_count = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name && t.value == ">=")
            .count();
        assert_eq!(ge_count, 1, "expected one `>=` token unchanged: {toks:?}");
        // No spurious `>>=` or `>>` tokens.
        for forbidden in [">>=", ">>"] {
            let n = toks
                .iter()
                .filter(|t| t.type_ == TokenType::Name && t.value == forbidden)
                .count();
            assert_eq!(n, 0, "did not expect `{forbidden}` in {toks:?}");
        }
    }

    #[test]
    fn percent_r_regex_literal_lexes_as_string_with_prefix() {
        let toks = tokenize_ruby_for_version("%r{hello}", "1.8").unwrap();
        assert_eq!(percent_values(&toks, "%r"), vec!["%r{hello}"]);
    }

    #[test]
    fn percent_s_symbol_literal_lexes_as_string_with_prefix() {
        let toks = tokenize_ruby_for_version("%s{my_sym}", "1.8").unwrap();
        assert_eq!(percent_values(&toks, "%s"), vec!["%s{my_sym}"]);
    }

    #[test]
    fn percent_x_command_literal_lexes_as_string_with_prefix() {
        let toks = tokenize_ruby_for_version("%x{ls -la}", "1.8").unwrap();
        assert_eq!(percent_values(&toks, "%x"), vec!["%x{ls -la}"]);
    }

    #[test]
    fn percent_r_empty_body_lexes() {
        // `%r{}` — empty regex body.
        let toks = tokenize_ruby_for_version("%r{}", "1.8").unwrap();
        assert_eq!(percent_values(&toks, "%r"), vec!["%r{}"]);
    }

    #[test]
    fn percent_r_s_x_lex_uniformly_across_all_eras() {
        // The percent r/s/x family pre-dates the era split — every era
        // (1.8, 1.9, 2.0, …) emits the same token shape.
        let src = "%r{a} %s{b} %x{c}";
        let baseline_r = percent_values(&tokenize_ruby_for_version(src, "1.8").unwrap(), "%r");
        let baseline_s = percent_values(&tokenize_ruby_for_version(src, "1.8").unwrap(), "%s");
        let baseline_x = percent_values(&tokenize_ruby_for_version(src, "1.8").unwrap(), "%x");
        assert_eq!(baseline_r, vec!["%r{a}"]);
        assert_eq!(baseline_s, vec!["%s{b}"]);
        assert_eq!(baseline_x, vec!["%x{c}"]);
        for v in machine::ERA_VERSIONS {
            let toks = tokenize_ruby_for_version(src, v).unwrap();
            assert_eq!(percent_values(&toks, "%r"), baseline_r, "%r diverged in era {v}");
            assert_eq!(percent_values(&toks, "%s"), baseline_s, "%s diverged in era {v}");
            assert_eq!(percent_values(&toks, "%x"), baseline_x, "%x diverged in era {v}");
        }
    }

    #[test]
    fn percent_x_does_not_swallow_following_tokens() {
        // After the closing `}`, the dispatcher resumes normal lexing.
        // `%x{pwd} + 1` should produce: String, Plus, Number.
        let toks = tokenize_ruby_for_version("%x{pwd} + 1", "1.8").unwrap();
        let kinds: Vec<TokenType> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.type_)
            .collect();
        assert_eq!(
            kinds,
            vec![TokenType::String, TokenType::Plus, TokenType::Number],
            "got tokens {toks:?}"
        );
    }

    #[test]
    fn percent_r_unterminated_reports_diagnostic() {
        let mut lx = RubyLexer::new("1.8").expect("lexer build");
        lx.push("%r{unterminated\n").unwrap();
        lx.finish().unwrap();
        let diags = lx.diagnostics();
        assert!(
            diags.iter().any(|d| d.code.contains("unterminated_percent_r")),
            "expected unterminated_percent_r diagnostic, got {:?}",
            diags
        );
    }

    // -----------------------------------------------------------------
    // Phase 6q companion — re-tag trailing-modifier keywords
    // -----------------------------------------------------------------
    //
    // `tag_modifier_keywords` rewrites `if`/`unless`/`while`/`until`
    // Keyword tokens to `if_modifier`/`unless_modifier`/`while_modifier`/
    // `until_modifier` when they follow an expression-ending token on
    // the same line.  The leading-keyword forms (`if y ... end`) are
    // never re-tagged — at statement-start position the preceding
    // non-Newline token is either absent or not expression-ending.
    //
    // Pre-1.0 Ruby — modifier conditionals predate the era split, so
    // all eras emit the same shape (covered by the cross-era test).

    fn has_token(toks: &[Token], type_: TokenType, value: &str) -> bool {
        toks.iter().any(|t| t.type_ == type_ && t.value == value)
    }

    #[test]
    fn modifier_if_after_method_call_no_paren_is_retagged() {
        // `puts "hi" if cond` — the `if` follows a String token on the
        // same line, so it's re-tagged.
        let toks = tokenize_ruby_for_version("puts \"hi\" if cond", "1.8").unwrap();
        assert!(
            has_token(&toks, TokenType::Keyword, "if_modifier"),
            "expected `if_modifier` token, got {toks:?}"
        );
        assert!(
            !has_token(&toks, TokenType::Keyword, "if"),
            "bare `if` token should be gone, got {toks:?}"
        );
    }

    #[test]
    fn modifier_unless_while_until_all_retagged() {
        // Smoke-test all four modifier forms on the same shape.
        for (src, expected) in [
            ("x = 1 if y",     "if_modifier"),
            ("x = 1 unless y", "unless_modifier"),
            ("x = 1 while y",  "while_modifier"),
            ("x = 1 until y",  "until_modifier"),
        ] {
            let toks = tokenize_ruby_for_version(src, "1.8").unwrap();
            assert!(
                has_token(&toks, TokenType::Keyword, expected),
                "expected `{expected}` in {src:?}, got {toks:?}"
            );
        }
    }

    #[test]
    fn leading_if_at_statement_start_is_not_retagged() {
        // `if y\n  x = 1\nend` — the `if` is at the start of the file,
        // no preceding token, so NOT re-tagged.  The trailing `end`
        // is also untouched (no modifier after it).
        let toks = tokenize_ruby_for_version("if y\n  x = 1\nend", "1.8").unwrap();
        assert!(
            has_token(&toks, TokenType::Keyword, "if"),
            "bare `if` should survive at line start, got {toks:?}"
        );
        assert!(
            !has_token(&toks, TokenType::Keyword, "if_modifier"),
            "leading `if` must NOT be re-tagged, got {toks:?}"
        );
    }

    #[test]
    fn newline_between_expr_and_if_prevents_retag() {
        // `x = 1\nif y` — two statements.  Even though `1` is an
        // expression-ending token, the newline shifts `if` to a
        // different line, so the same-line guard prevents re-tagging.
        let toks = tokenize_ruby_for_version("x = 1\nif y\nend", "1.8").unwrap();
        assert!(
            has_token(&toks, TokenType::Keyword, "if"),
            "expected bare `if` across newline, got {toks:?}"
        );
        assert!(
            !has_token(&toks, TokenType::Keyword, "if_modifier"),
            "newline must block re-tag, got {toks:?}"
        );
    }

    #[test]
    fn modifier_retag_uniform_across_all_eras() {
        // Trailing modifiers predate the era split — every era ≥ 1.8
        // must emit the same `*_modifier` token shape.
        let src = "x = 1 if y";
        for era in ["1.8", "1.9.1", "2.0", "2.3", "2.7", "3.0"] {
            let toks = tokenize_ruby_for_version(src, era).unwrap();
            assert!(
                has_token(&toks, TokenType::Keyword, "if_modifier"),
                "expected `if_modifier` in era {era}, got {toks:?}"
            );
        }
    }

    #[test]
    fn backtick_unterminated_reports_diagnostic() {
        // A backtick with no closing `` ` `` should leave a diagnostic
        // behind.  The lexer still finishes (terminates on EOF) — the
        // `parse_error(unterminated_backtick)` action records the
        // error rather than aborting.  We use the public RubyLexer API
        // directly so we can read its diagnostics() after `finish()`.
        let mut lx = RubyLexer::new("1.8").expect("lexer build");
        lx.push("`unterminated\n").unwrap();
        lx.finish().unwrap();
        let diags = lx.diagnostics();
        assert!(
            diags.iter().any(|d| d.code.contains("unterminated_backtick")),
            "expected unterminated_backtick diagnostic, got {:?}",
            diags
        );
    }
}
