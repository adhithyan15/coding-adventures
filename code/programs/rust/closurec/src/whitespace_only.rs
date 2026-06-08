//! `whitespace_only` — the body of `--compilation_level WHITESPACE_ONLY`.
//!
//! # What WHITESPACE_ONLY means
//!
//! Per the upstream Java Closure Compiler (`CompilationLevel.java`,
//! `WHITESPACE_ONLY` enum value), this level:
//!
//! - **removes comments** (line and block);
//! - **collapses runs of whitespace** to nothing where token
//!   boundaries don't need a separator, or to a single space
//!   where they do (e.g. between two adjacent identifier-like
//!   tokens such as `return` `x`);
//! - **does NOT rename identifiers, fold constants, dead-code
//!   eliminate, or reorder**.
//!
//! It's the cheapest, most conservative optimisation Closure
//! offers — its output is byte-equivalent semantically and
//! visually recognisable to a human who knows the source.
//!
//! # Why we operate at the token level (and not the AST level)
//!
//! Today the parser produces a `GrammarASTNode` (from
//! `grammar-tools`), but the emitter from CLOC07 only knows
//! `javascript_ast::Program`. There is no bridge between the two
//! yet. Building that bridge is a substantial separate piece of
//! work and isn't on the CLOC11 critical path.
//!
//! Token-level whitespace removal sidesteps the AST entirely:
//! tokenize the source, drop the trivia tokens, re-join the
//! survivors with the minimum-necessary inter-token whitespace.
//! This matches what `WHITESPACE_ONLY` actually does inside
//! Closure (its `RenameVars` / `InlineVariables` passes are all
//! disabled at this level).
//!
//! Later compilation levels (SIMPLE, ADVANCED) will need real
//! AST processing and will route through the bridge when it
//! exists.
//!
//! # Inter-token spacing rule
//!
//! Two consecutive non-trivia tokens need a space between them
//! iff *omitting* the space would alter the lexer's tokenization
//! of the joined string. The safe conservative rule:
//!
//! > Insert a single space between two adjacent tokens if BOTH
//! > are "word-like" (identifier, number, keyword, regex,
//! > template). Otherwise emit them back-to-back.
//!
//! Examples:
//!
//! | Token A   | Token B   | Joined        |
//! |-----------|-----------|---------------|
//! | `return`  | `x`       | `return x`    | (both word-like)
//! | `x`       | `+`       | `x+`          | (B is punctuation)
//! | `+`       | `x`       | `+x`          | (A is punctuation)
//! | `(`       | `1`       | `(1`          |
//! | `1`       | `+`       | `1+`          |
//! | `++`      | `x`       | `++x`         | (operator + word: safe)
//! | `x`       | `++`      | `x++`         |
//!
//! This is a *conservative* under-removal — we never produce
//! incorrect output, but a more aggressive minifier could remove
//! the space in some edge cases. CC's WHITESPACE_ONLY is
//! similarly conservative.

use coding_adventures_javascript_lexer::tokenize_javascript_typed;
use coding_adventures_javascript_tokens::EsVersion;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Reasons whitespace-only minification can fail. Currently the
/// only failure path is the underlying tokenizer rejecting the
/// source — every other operation is infallible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinifyError {
    /// The tokenizer rejected the source. Inner string is the
    /// tokenizer's own error message.
    LexError(String),
}

impl std::fmt::Display for MinifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MinifyError::LexError(s) => write!(f, "whitespace-only: tokenizer failed: {s}"),
        }
    }
}

impl std::error::Error for MinifyError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// What kind of block does an open `{` start? Pushed onto
/// `brace_stack` and consulted on the matching `}` to decide
/// whether to emit a synthetic `;` after the closing brace.
///
/// | Kind        | Trigger to push                          | `}` action |
/// |-------------|------------------------------------------|-------------|
/// | `Function`  | `function` keyword at stmt boundary      | always emit `;` (gap-030 rule B) |
/// | `TryChain`  | `try`/`catch`/`finally` keyword          | emit `;` IFF next non-trivia is NOT `catch`/`finally` (gap-033) |
/// | `Other`     | any other block (if/while/for/plain block)| no synthetic `;` |
///
/// The `TryChain` variant captures both the try-body block and
/// each catch/finally clause body in the chain. The chain ends
/// when a `}` closing a `TryChain` block is NOT followed by
/// another `catch` or `finally` keyword — at that point the
/// trailing `;` is emitted, mirroring upstream Closure
/// v20240317's normalisation of try-statement output shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BlockKind {
    Function,
    TryChain,
    /// gap-034: class declaration body. Pushed when `{`
    /// follows `class [Name]` at a statement boundary.
    /// Popped on matching `}`; ALWAYS emits a synthetic `;`
    /// (same shape as `Function`), normalising upstream
    /// Closure's `class C{}` → `class C{};` output.
    Class,
    /// gap-036: switch statement body. Pushed when `{`
    /// follows the matching `)` of a `switch(...)` head.
    /// Popped on matching `}`; ALWAYS emits a synthetic `;`
    /// (same shape as `Function`), mirroring upstream
    /// Closure's `switch(x){...}` → `switch(x){...};` output.
    Switch,
    Other,
}

/// Apply WHITESPACE_ONLY to a single source string.
///
/// Returns the comment-stripped, whitespace-collapsed equivalent.
/// `version` selects the JS spec the tokenizer should use.
pub fn whitespace_only_minify(
    source: &str,
    version: EsVersion,
) -> Result<String, MinifyError> {
    let tokens = tokenize_javascript_typed(source, version)
        .map_err(MinifyError::LexError)?;

    // Filter out trivia tokens (comments, whitespace, newlines)
    // and the EOF sentinel. The lexer marks trivia via the
    // `type_name` string (set by the grammar's named rules);
    // we accept multiple common spellings since different
    // grammars label them differently.
    let kept: Vec<_> = tokens
        .iter()
        .filter(|t| !is_trivia(t))
        .filter(|t| !is_eof(t))
        .collect();

    // Re-stitch: insert a single space between two adjacent
    // word-like tokens; otherwise concatenate directly. String
    // literals get re-quoted because the lexer's `value` field
    // is the *unescaped* content (per the `lexer::token::Token`
    // docstring); emitting it raw would corrupt the program.
    //
    // gap-030 rules layered on top of the simple re-stitch:
    //
    // **Rule A — drop `;` immediately before `}`:** Per
    // ECMAScript §11.9 (Automatic Semicolon Insertion), `}`
    // terminates any in-progress statement, so a `;` right
    // before `}` is redundant noise. EXCEPT when that `;` is
    // structurally a body slot (the EmptyStatement body of
    // `if(x);` / `while(x);` / `for(;;);`). The
    // `body_position_next` flag tracks whether we're emitting
    // into a statement-body position (set by the closing `)`
    // of an if/while/for head) and suppresses the drop.
    //
    // **Rule B — emit `;` after a function-DECLARATION's
    // closing `}`:** Upstream Closure normalises this so the
    // function-decl output shape stays predictable for
    // concatenation. We distinguish declarations from
    // expressions by tracking whether the `function` keyword
    // was seen at a statement boundary (start-of-input, or
    // after a previously-emitted `;`/`}`/`{`); declarations
    // are at-boundary, expressions live mid-expression.
    //
    // **Rule C — dedup `;` after synthetic `;`:** When Rule B
    // emits a `;` and the very next source token is also `;`,
    // skip the source `;` to avoid `};;` in output. The
    // semantic content is identical (the source's `;` would
    // have been an `EmptyStatement` that contributes nothing).
    let mut out = String::with_capacity(source.len());

    // State machine:
    let mut brace_stack: Vec<BlockKind> = Vec::new();
    // ^ for each open `{`, the kind of block it opens.
    //   `BlockKind::Function` triggers gap-030 rule-B (emit
    //   `;` after closing `}`); `BlockKind::TryChain` triggers
    //   gap-033 rule (emit `;` after closing `}` iff next
    //   non-trivia token is NOT `catch` or `finally` — i.e.
    //   the chain has ended).
    let mut paren_stack: Vec<bool> = Vec::new();
    // ^ for each open `(`, true iff this `(` was the head of
    //   an `if` / `while` / `for` (control-flow construct
    //   whose body slot follows the close-paren). Popped on
    //   matching `)`.
    let mut paren_is_switch_stack: Vec<bool> = Vec::new();
    // ^ parallel to `paren_stack`. For each open `(`, true
    //   iff this `(` was the head of a `switch(...)`. On the
    //   matching `)` pop, arms `next_block_is_switch_body`
    //   so the upcoming `{` gets pushed as `BlockKind::Switch`
    //   for gap-036's trailing-`;` rule.
    let mut saw_function_kw_at_boundary = false;
    let mut next_block_is_try_chain = false;
    // ^ gap-033: armed by `try` / `catch` / `finally`
    //   keywords; consumed by the next `{` and stored on the
    //   brace stack as `BlockKind::TryChain`. `try` arms it
    //   at a statement boundary. `catch` and `finally` arm
    //   it whenever they appear (they only legally appear
    //   after a preceding `}` that closed a try-chain block,
    //   so the boundary context is already correct).
    let mut saw_class_kw_at_boundary = false;
    // ^ gap-034: armed by the `class` keyword at a statement
    //   boundary. The next `{` (after the optional class name
    //   and optional `extends` clause) consumes the flag and
    //   pushes `BlockKind::Class`.
    let mut saw_async_kw_at_boundary = false;
    // ^ gap-037: armed by `async` keyword at a statement
    //   boundary when the very next token is `function`. The
    //   next `function` token consumes the flag and treats
    //   the upcoming function-decl shape as if `function`
    //   itself were at a statement boundary, so the matching
    //   `}` emits the gap-030 trailing `;`.
    let mut next_paren_is_control_flow_head = false;
    let mut next_paren_is_switch_head = false;
    // ^ gap-036: armed by the `switch` keyword. When the `(`
    //   pushes this onto its own paren-stack frame, the `)`
    //   pop will arm `next_block_is_switch_body`. The next
    //   `{` consumes that and pushes `BlockKind::Switch`.
    let mut next_block_is_switch_body = false;
    let mut body_position_next = false;
    let mut at_stmt_boundary = true;
    let mut last_emit_was_synthetic_semi = false;
    let mut prev_emitted_tok: Option<&lexer::token::Token> = None;

    let mut idx = 0usize;
    while idx < kept.len() {
        let tok = kept[idx];
        let val = tok.value.as_str();

        // Rule C: dedup source `;` directly after our
        // synthetic `;` from rule B.
        if val == ";" && last_emit_was_synthetic_semi {
            last_emit_was_synthetic_semi = false;
            idx += 1;
            continue;
        }
        // The synthetic-semi window only spans one token; any
        // other token after the synthetic `;` clears the flag.
        last_emit_was_synthetic_semi = false;

        // gap-032: single-statement block flattening. When a
        // `{` appears in body position and the block contains
        // exactly one simple statement ending with `;`, we
        // strip the braces and pre-emit the contents directly.
        //
        // Upstream Closure normalises `if(x){a();}else{b();}`
        // to `if(x)a();else b();` — both `{}` wrappers gone,
        // statements inlined. We match that shape.
        //
        // **Eligibility for flatten** (all must hold):
        // 1. `body_position_next` is true (we're a body slot).
        // 2. We can find the matching `}` for this `{`.
        // 3. Inside the block: exactly one `;` at depth 1
        //    (counted relative to our outer `{`).
        // 4. Inside the block: no nested `{`.
        // 5. The token just before the closing `}` is `;`
        //    (the trailing terminator that becomes our
        //    inline terminator after flatten).
        // 6. Inside the block: no `function`/`try`/`if`/
        //    `while`/`for`/`do`/`switch` keyword at depth 1.
        //    These would introduce structure (their own
        //    `{...}` or body slots) that the conservative
        //    pre-emit pathway can't track.
        //
        // When eligible, we skip the `{`, copy content tokens
        // directly into `out` (bypassing rule A so the
        // trailing `;` survives as our terminator), then skip
        // the closing `}`. Brace_stack is untouched — we
        // never pushed for this `{`, so we never pop for the
        // skipped `}`.
        if val == "{" && body_position_next {
            // Walk forward to find matching `}` and gather
            // eligibility info.
            let mut depth: i32 = 0;
            let mut semi_count: u32 = 0;
            let mut has_nested_brace = false;
            let mut has_blocking_keyword = false;
            let mut matching_close: Option<usize> = None;
            let mut scan = idx + 1;
            while scan < kept.len() {
                let s = kept[scan].value.as_str();
                match s {
                    "{" => {
                        has_nested_brace = true;
                        depth += 1;
                    }
                    "}" => {
                        if depth == 0 {
                            matching_close = Some(scan);
                            break;
                        }
                        depth -= 1;
                    }
                    "(" | "[" => {
                        depth += 1;
                    }
                    ")" | "]" => {
                        depth -= 1;
                    }
                    ";" if depth == 0 => {
                        semi_count += 1;
                    }
                    "function" | "try" | "if" | "while" | "for"
                    | "do" | "switch" | "class"
                        if depth == 0 =>
                    {
                        has_blocking_keyword = true;
                    }
                    _ => {}
                }
                scan += 1;
            }

            if let Some(close_idx) = matching_close {
                let last_before_close = if close_idx > idx + 1 {
                    kept[close_idx - 1].value.as_str()
                } else {
                    ""
                };
                let eligible = !has_nested_brace
                    && !has_blocking_keyword
                    && semi_count == 1
                    && last_before_close == ";";
                if eligible {
                    // Pre-emit content tokens (idx+1 ..
                    // close_idx). Each token gets the same
                    // separator + quoting treatment as the
                    // main loop, but the state machine isn't
                    // run on them — they're carried through
                    // verbatim. This is safe because we
                    // verified the contents are a single
                    // simple statement with no nested
                    // structure.
                    for content_idx in (idx + 1)..close_idx {
                        let t = kept[content_idx];
                        if let Some(prev) = prev_emitted_tok {
                            if needs_separator(prev, t) {
                                out.push(' ');
                            }
                        }
                        if is_string_literal(t) {
                            out.push('"');
                            push_quoted_string_content(
                                &mut out, &t.value,
                            );
                            out.push('"');
                        } else if is_number_literal(t) {
                            out.push_str(&normalize_number_value(&t.value));
                        } else {
                            out.push_str(&t.value);
                        }
                        prev_emitted_tok = Some(t);
                    }
                    // The body slot is now filled.
                    body_position_next = false;
                    at_stmt_boundary = true;
                    // Advance past the closing `}`.
                    idx = close_idx + 1;
                    continue;
                }
            }
            // Not eligible — fall through to normal `{`
            // handling (push brace_stack, etc.).
        }

        // Rule A: drop `;` directly before `}`, UNLESS the
        // `;` is a body slot (body_position_next).
        if val == ";" && !body_position_next {
            if let Some(next) = kept.get(idx + 1) {
                if next.value == "}" {
                    // Skip this `;`. We are still at a stmt
                    // boundary after dropping it (the
                    // upcoming `}` terminates the enclosing
                    // statement just as a `;` would).
                    at_stmt_boundary = true;
                    idx += 1;
                    continue;
                }
            }
        }

        // gap-031: empty `{}` in body position collapses to
        // `;`. Upstream Closure substitutes an EmptyStatement
        // (`;`) for an empty Block when the block is in the
        // body slot of a control-flow construct
        // (`for(...){...}` / `while(x){}` / `if(x){}` etc.).
        // The trigger is `{}` (a `{` immediately followed by
        // `}`) WHERE `body_position_next` is true. We emit a
        // single `;` and skip both braces. Function-decl
        // bodies are unaffected because `body_position_next`
        // is false in that context (no paren-stack push of
        // control-flow head). Plain block-as-statement at
        // top level is also unaffected (also no control-flow
        // head). The substitution is semantically identical
        // — ECMAScript §13.2 lets either Block or
        // EmptyStatement satisfy the Statement nonterminal
        // in the body slot.
        if val == "{" && body_position_next {
            if let Some(next) = kept.get(idx + 1) {
                if next.value == "}" {
                    // Emit `;` in place of `{}`.
                    out.push(';');
                    // The body slot is now filled by the
                    // EmptyStatement we just emitted.
                    body_position_next = false;
                    at_stmt_boundary = true;
                    // Arm rule-C dedup so an immediately-
                    // following source `;` (e.g.
                    // `for(...){};var x=1;`) gets folded into
                    // the synthetic one — output stays
                    // `for(...);var x=1;` instead of
                    // becoming `for(...);;var x=1;`. Same
                    // mechanism rule B uses for function-decl
                    // trailing `;`. Pre-push security review
                    // flagged this as an optimality regression.
                    last_emit_was_synthetic_semi = true;
                    // prev_emitted_tok is left untouched —
                    // the next real token will compute its
                    // separator against whatever came before
                    // our substituted `;` (typically `)`),
                    // which is not word-like so no separator
                    // is needed.
                    idx += 2; // Skip `{` and `}`.
                    continue;
                }
            }
        }

        // Snapshot prev BEFORE the emit overwrites it, so
        // the state-update branches below can inspect the
        // token that came *before* the current one. Needed
        // for gap-033's `catch`/`finally` guard.
        let prev_before_this_emit = prev_emitted_tok;

        // Emit the token (with separator if needed).
        if let Some(prev) = prev_emitted_tok {
            if needs_separator(prev, tok) {
                out.push(' ');
            }
        }
        if is_string_literal(tok) {
            out.push('"');
            push_quoted_string_content(&mut out, &tok.value);
            out.push('"');
        } else if is_number_literal(tok) {
            // gap-038: rewrite hex/oct/bin literals to
            // decimal when decimal is no longer than source.
            out.push_str(&normalize_number_value(&tok.value));
        } else {
            out.push_str(&tok.value);
        }
        prev_emitted_tok = Some(tok);

        // Rule B / gap-033: a `}` that closes a
        // function-declaration body, or the LAST clause of a
        // try-chain, gets a synthetic `;` appended.
        if val == "}" {
            let kind = brace_stack.pop().unwrap_or(BlockKind::Other);
            let needs_synthetic_semi = match kind {
                BlockKind::Function => true,
                BlockKind::Class => true,    // gap-034
                BlockKind::Switch => true,   // gap-036
                BlockKind::TryChain => {
                    // gap-033: the chain continues iff the
                    // very next non-trivia token is `catch`
                    // or `finally`. If so, suppress the `;`
                    // because the next clause will own the
                    // terminator. Otherwise the chain has
                    // ended — emit `;`.
                    let next_val = kept.get(idx + 1).map(|t| t.value.as_str());
                    !matches!(next_val, Some("catch") | Some("finally"))
                }
                BlockKind::Other => false,
            };
            if needs_synthetic_semi {
                out.push(';');
                last_emit_was_synthetic_semi = true;
            }
            at_stmt_boundary = true;
            body_position_next = false;
        } else if val == "{" {
            // Priority: TryChain > Switch > Class > Function
            // > Other. A `{` immediately after a
            // `try`/`catch`/`finally` is a try-chain body; a
            // `{` after `switch(...)`'s closing `)` is a
            // switch body; a `{` after `class [Name]` is a
            // class body; a `{` after `function [Name](...)`
            // at a statement boundary is a function-decl
            // body; everything else is Other.
            let kind = if next_block_is_try_chain {
                BlockKind::TryChain
            } else if next_block_is_switch_body {
                BlockKind::Switch
            } else if saw_class_kw_at_boundary {
                BlockKind::Class
            } else if saw_function_kw_at_boundary {
                BlockKind::Function
            } else {
                BlockKind::Other
            };
            brace_stack.push(kind);
            next_block_is_try_chain = false;
            next_block_is_switch_body = false;
            saw_class_kw_at_boundary = false;
            saw_function_kw_at_boundary = false;
            // A `{` either opens a Block-as-body (consumes
            // body_position_next) or opens a function-decl /
            // try-chain / class / switch body (independent of
            // body_position). Either way the body slot is
            // filled by this brace.
            body_position_next = false;
            at_stmt_boundary = true;
        } else if val == "(" {
            paren_stack.push(next_paren_is_control_flow_head);
            paren_is_switch_stack.push(next_paren_is_switch_head);
            next_paren_is_control_flow_head = false;
            next_paren_is_switch_head = false;
            at_stmt_boundary = false;
        } else if val == ")" {
            let was_cf = paren_stack.pop().unwrap_or(false);
            let was_switch = paren_is_switch_stack.pop().unwrap_or(false);
            // The body slot of an if/while/for opens after
            // the closing `)`. The next emitted statement
            // (or `;` / `{`) fills that slot.
            body_position_next = was_cf;
            // gap-036: if this `)` closed a `switch(...)`
            // head, the next `{` opens a switch body.
            if was_switch {
                next_block_is_switch_body = true;
            }
            at_stmt_boundary = false;
        } else if val == ";" {
            // We emitted a real `;` (either a terminator or
            // a body slot per body_position_next). Either
            // way, body_position_next is consumed.
            body_position_next = false;
            at_stmt_boundary = true;
        } else if is_keyword_function(tok) {
            // Only treat as a function-DECLARATION when at a
            // statement boundary OR when we just saw `async`
            // at a statement boundary (gap-037). Expressions
            // live mid-expression and don't qualify.
            if at_stmt_boundary || saw_async_kw_at_boundary {
                saw_function_kw_at_boundary = true;
            }
            saw_async_kw_at_boundary = false;
            at_stmt_boundary = false;
        } else if val == "async" {
            // gap-037: `async` keyword before `function` makes
            // `async function f(){}` a function-DECLARATION
            // shape. Track that we just saw `async` at a
            // statement boundary, and propagate that boundary
            // forward through the next `function` keyword
            // arming. Filter out method-shorthand `async
            // m(){}` (next token would be IDENT followed by
            // `(`) and async-arrow `async()=>{}` (next is
            // `(`) by requiring the next non-trivia to be
            // `function`. Without this guard the flag would
            // leak forward and contaminate an unrelated `{`.
            let next_is_function =
                kept.get(idx + 1).map(|t| t.value.as_str()) == Some("function");
            if at_stmt_boundary && next_is_function {
                saw_async_kw_at_boundary = true;
            }
            at_stmt_boundary = false;
        } else if val == "try" {
            // gap-033: `try` opens a try-chain ONLY when it
            // is acting as a statement-starting keyword. Two
            // guards needed:
            //
            //   1. `at_stmt_boundary` — filters out e.g.
            //      `return try` (which is illegal anyway,
            //      defense in depth).
            //   2. Next non-trivia token must be `{` — this
            //      filters out `try` appearing as a PROPERTY
            //      NAME in an object literal (`{try: 1}`)
            //      where `at_stmt_boundary` would be true
            //      after the object-opening `{` but `try` is
            //      semantically a key, not a statement.
            //      Without this guard, the flag would leak
            //      forward and contaminate the next
            //      unrelated `{`, turning e.g.
            //      `var t={try:1};do{y}while(x);` into
            //      `var t={try:1};do{y};while(x);` — a
            //      do/while SyntaxError. Caught by pre-push
            //      security review.
            let next_is_brace =
                kept.get(idx + 1).map(|t| t.value.as_str()) == Some("{");
            if at_stmt_boundary && next_is_brace {
                next_block_is_try_chain = true;
            }
            at_stmt_boundary = false;
        } else if matches!(val, "catch" | "finally") {
            // `catch` / `finally` open a try-chain clause
            // ONLY when they follow a try-chain block's
            // closing `}` — which is the only legal position
            // per ECMAScript §14.15. The previous emitted
            // token tells us this. Without the guard, member
            // access like `obj.catch(...)` (where the lexer
            // still tags `catch` as KEYWORD) would arm the
            // flag and contaminate the next unrelated `{`.
            // Same family of defect as the `try` filter above.
            let prev_is_brace =
                prev_before_this_emit.map(|t| t.value.as_str()) == Some("}");
            if prev_is_brace {
                next_block_is_try_chain = true;
            }
            at_stmt_boundary = false;
        } else if matches!(val, "if" | "while" | "for") {
            // The next `(` is a control-flow head.
            next_paren_is_control_flow_head = true;
            at_stmt_boundary = false;
        } else if val == "switch" {
            // gap-036: arm the switch-head flag ONLY when
            // the very next token is `(`. Mirrors gap-033's
            // `try` guard family: `switch` can legally
            // appear as a reserved word in property-name
            // position (`var o={switch:1};`) and the lexer
            // still tags it as KEYWORD. Without this guard,
            // the flag would leak forward and contaminate
            // an unrelated `(` somewhere downstream, then
            // `)`, then a `{`, causing a spurious
            // `BlockKind::Switch` push and a stray trailing
            // `;` on an innocent block. The `(` requirement
            // is grammatical: `SwitchStatement → switch (
            // Expression ) CaseBlock` per §13.12.
            let next_is_paren =
                kept.get(idx + 1).map(|t| t.value.as_str()) == Some("(");
            if next_is_paren {
                next_paren_is_switch_head = true;
            }
            at_stmt_boundary = false;
        } else if val == "class" {
            // gap-034: `class` at a statement boundary opens
            // a class-declaration body. Same guard family as
            // gap-033's `try` filter: defend against `class`
            // appearing as a property name (`var o={class:1};`)
            // or other mid-expression uses where the KEYWORD
            // tag is misleading.
            //
            // The legal token positions for a class
            // DECLARATION right after the `class` keyword
            // are: `{` (anonymous class declaration is
            // illegal at statement position but we accept
            // for symmetry with class expressions),
            // `extends` (the extends clause), or an IDENT
            // (the class name). Property-name position
            // would put `:`/`,`/`}` next; method shorthand
            // would put `(` next. Filter all those out.
            let next_val =
                kept.get(idx + 1).map(|t| t.value.as_str());
            let next_looks_class_decl = match next_val {
                Some("{") | Some("extends") => true,
                Some(v) => !matches!(
                    v,
                    ":" | "," | ";" | "}" | ")" | "]" | "." | "=" | "("
                ),
                None => false,
            };
            if at_stmt_boundary && next_looks_class_decl {
                saw_class_kw_at_boundary = true;
            }
            at_stmt_boundary = false;
        } else if val == "else" {
            // gap-032: `else` opens the else-clause's body
            // slot. Mirror the post-`)` arming so gap-031 /
            // gap-032 can fire on `else{}` / `else{a;}`.
            // Without this, the else-body would never see
            // body_position_next=true and the rules wouldn't
            // apply.
            body_position_next = true;
            at_stmt_boundary = false;
        } else {
            // Any other token consumes the body slot (the
            // body of `if(x)y();` is `y()`, so once we emit
            // `y` the slot is filled).
            body_position_next = false;
            at_stmt_boundary = false;
        }

        idx += 1;
    }
    Ok(out)
}

/// True iff this token is the `function` keyword. Used by
/// gap-030 part 2 to decide whether a `{` opens a
/// function-DECLARATION body. We check by literal value plus
/// the grammar's KEYWORD type tag, so we don't accidentally
/// pick up a method/property named `function` (those would
/// arrive as `NAME` or `STRING`).
fn is_keyword_function(tok: &lexer::token::Token) -> bool {
    if tok.value != "function" {
        return false;
    }
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        // Accept any of the common keyword-rule names a JS
        // grammar might use. If the grammar doesn't tag it,
        // fall back to value-only matching — bare value
        // `function` is reserved per §11.6.2.1 so the
        // identifier-collision case can't legally arise.
        return upper == "KEYWORD"
            || upper == "FUNCTION"
            || upper.starts_with("KEYWORD");
    }
    true
}

/// True iff this token is a numeric literal (NUMBER token).
/// Mirrors `is_string_literal`'s grammar-name detection but
/// for the numeric-rule family. Used by gap-038's shortest-
/// form normalisation.
fn is_number_literal(tok: &lexer::token::Token) -> bool {
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        if upper == "NUMBER" || upper == "NUMERIC_LITERAL" || upper == "NUMBER_LITERAL" {
            return true;
        }
    }
    matches!(tok.type_, lexer::token::TokenType::Number)
}

/// gap-038: rewrite hex / octal / binary integer literals to
/// their decimal form when decimal is no longer than source.
///
/// Upstream Closure under WHITESPACE_ONLY normalises numeric
/// literals to a shortest-form representation. For non-BigInt
/// hex/oct/bin literals, the decimal form is chosen iff its
/// string length is **less than or equal to** the source-form
/// length — i.e. tie-breaks go to decimal.
///
/// **Worked examples** (verified against
/// `closure-compiler-v20240317.jar`):
///   `0xff`         (4 chars) → `255`             (3 chars) → DECIMAL wins
///   `0xfff`        (5 chars) → `4095`            (4 chars) → DECIMAL wins
///   `0xffffffff`  (10 chars) → `4294967295`     (10 chars) → DECIMAL wins (tie)
///   `0xfffffffff` (11 chars) → `68719476735`    (11 chars) → DECIMAL wins (tie)
///   `0xffffffffffff` (14 chars) → `281474976710655` (15 chars) → HEX wins
///   `0o777`        (5 chars) → `511`             (3 chars) → DECIMAL wins
///   `0b1010`       (6 chars) → `10`              (2 chars) → DECIMAL wins
///
/// **Not handled (left for follow-up gaps):**
///   - BigInt literals (anything ending in `n`) need
///     arbitrary-precision arithmetic. `0xfn` stays as
///     `0xfn` rather than becoming `15n`.
///   - Decimal floating-point normalisation: `0.5` → `.5`,
///     `1e3` → `1E3`, `10.0` → `10`. These are different
///     rules in different parts of the upstream code path.
///   - Numbers that exceed `u128::MAX` parse as `None` and
///     stay verbatim.
fn normalize_number_value(value: &str) -> String {
    // BigInt — defer to a future gap.
    if value.ends_with('n') {
        return value.to_string();
    }
    // Try each radix prefix in turn. The prefixes are
    // case-insensitive per §12.8.3.
    let parsed: Option<u128> = if let Some(rest) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u128::from_str_radix(rest, 16).ok()
    } else if let Some(rest) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        u128::from_str_radix(rest, 8).ok()
    } else if let Some(rest) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        u128::from_str_radix(rest, 2).ok()
    } else {
        // Decimal or no radix prefix — leave alone. Decimal
        // floating-point shortest-form is a separate concern
        // (see function doc).
        return value.to_string();
    };

    let Some(n) = parsed else {
        // Doesn't fit in u128 — leave verbatim.
        return value.to_string();
    };
    let decimal = n.to_string();
    // `<= value.len()` because upstream tie-breaks to decimal
    // (verified via JAR probe: `0xffffffff` (10) → `4294967295` (10)
    // → DECIMAL wins).
    if decimal.len() <= value.len() {
        decimal
    } else {
        value.to_string()
    }
}

/// True iff this token came from a string-literal rule. The
/// canonical names in JS grammars are `STRING` and `STRING_LITERAL`;
/// we accept both.
fn is_string_literal(tok: &lexer::token::Token) -> bool {
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        if upper == "STRING" || upper == "STRING_LITERAL" {
            return true;
        }
    }
    matches!(tok.type_, lexer::token::TokenType::String)
}

/// Push the JS-escaped form of `content` into `out`. Closure's
/// WHITESPACE_ONLY canonicalizes to double-quoted strings; we
/// follow. We escape:
///
/// - `"`  → `\"`
/// - `\`  → `\\`
/// - LF   → `\n`
/// - CR   → `\r`
/// - TAB  → `\t`
///
/// Other control characters and non-ASCII pass through
/// unchanged. (CC has a more elaborate escape table; we'll
/// expand to match in a follow-up if needed.)
fn push_quoted_string_content(out: &mut String, content: &str) {
    for c in content.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
}

// ---------------------------------------------------------------------------
// Token classification helpers
// ---------------------------------------------------------------------------

/// True iff this token is trivia (comment / whitespace / newline)
/// that WHITESPACE_ONLY discards.
///
/// We check both the grammar-supplied `type_name` (an `Option<String>`)
/// and the structural `TokenType`'s `is_trivia()`. The two should
/// agree but we accept either: defensive against grammar evolution.
pub(crate) fn is_trivia(tok: &lexer::token::Token) -> bool {
    // Match grammar-supplied type names *exactly* — substring
    // matching against "COMMENT" would misclassify hypothetical
    // tokens like "NON_COMMENT_LITERAL". The set below is the
    // closed list of trivia rule names the JS grammar (and its
    // peers) use today.
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        return matches!(
            upper.as_str(),
            "COMMENT"
                | "LINE_COMMENT"
                | "BLOCK_COMMENT"
                | "WHITESPACE"
                | "WS"
                | "NEWLINE"
                | "LINE_TERMINATOR"
                | "SKIP"
        );
    }
    matches!(
        tok.type_,
        lexer::token::TokenType::Newline
            | lexer::token::TokenType::Indent
            | lexer::token::TokenType::Dedent
    )
}

/// True iff this is the EOF sentinel emitted by the lexer.
pub(crate) fn is_eof(tok: &lexer::token::Token) -> bool {
    matches!(tok.type_, lexer::token::TokenType::Eof)
}

/// True iff two adjacent tokens need a space between them to
/// preserve their separate identity when re-tokenized.
fn needs_separator(a: &lexer::token::Token, b: &lexer::token::Token) -> bool {
    // Conservative rule: both word-like → space; otherwise none.
    if is_word_like(a) && is_word_like(b) {
        return true;
    }
    // gap-035: `var{` / `var[` / `let{` / `let[` / `const{` /
    // `const[` get a separator inserted before the bracket,
    // matching upstream Closure's output for destructuring
    // declarations. Without this, `var{a}=x;` round-trips as
    // `var{a}=x;`; upstream emits `var {a}=x;`.
    if matches!(a.value.as_str(), "var" | "let" | "const")
        && matches!(b.value.as_str(), "{" | "[")
    {
        return true;
    }
    false
}

/// True iff a token is "word-like" — its value would merge with
/// an adjacent word-like value if no separator were inserted.
///
/// Word-like tokens: identifiers (NAME), numbers (NUMBER),
/// keywords (KEYWORD), regexes (REGEX), template literals
/// (TEMPLATE*), BigInts (BIGINT), private names (PRIVATE_NAME).
///
/// String literals are NOT word-like in this sense: `"a""b"`
/// re-tokenizes correctly as two strings.
fn is_word_like(tok: &lexer::token::Token) -> bool {
    // Prefer the grammar's type_name when present; fall back to
    // the structural TokenType for the basic categories.
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        return matches!(
            upper.as_str(),
            "NAME"
                | "NUMBER"
                | "KEYWORD"
                | "REGEX"
                | "BIGINT"
                | "PRIVATE_NAME"
                | "TEMPLATE"
                | "TEMPLATE_NO_SUB"
                | "TEMPLATE_HEAD"
                | "TEMPLATE_MIDDLE"
                | "TEMPLATE_TAIL"
                | "IDENT"
                | "IDENTIFIER"
        );
    }
    matches!(
        tok.type_,
        lexer::token::TokenType::Name | lexer::token::TokenType::Number | lexer::token::TokenType::Keyword
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn minify(src: &str) -> String {
        whitespace_only_minify(src, EsVersion::Es2025).expect("ok")
    }

    #[test]
    fn empty_source_yields_empty_output() {
        assert_eq!(minify(""), "");
    }

    #[test]
    fn strips_line_comment() {
        let src = "// removed\nvar x = 1;";
        let out = minify(src);
        assert!(!out.contains("removed"), "comment leaked: {out:?}");
        assert!(out.contains("var") && out.contains("x") && out.contains("1"));
    }

    #[test]
    fn strips_block_comment() {
        let src = "var/* removed */x=1;";
        let out = minify(src);
        assert!(!out.contains("removed"), "comment leaked: {out:?}");
        // Must still have "var x" (space required between word tokens).
        assert!(out.contains("var x"), "missing word-separator: {out:?}");
    }

    #[test]
    fn collapses_whitespace_between_punctuation_and_word() {
        let src = "x  =   1 ;";
        let out = minify(src);
        assert_eq!(out, "x=1;");
    }

    #[test]
    fn keeps_space_between_two_keywords() {
        let src = "return    typeof    x;";
        let out = minify(src);
        assert!(out.contains("return typeof"), "got: {out:?}");
        assert!(out.contains("typeof x"), "got: {out:?}");
    }

    #[test]
    fn keeps_space_between_keyword_and_number() {
        // `return 1` is two word-like tokens; collapsing to
        // `return1` would re-tokenize as an identifier.
        let src = "return   1;";
        let out = minify(src);
        assert!(out.contains("return 1"), "got: {out:?}");
    }

    #[test]
    fn no_space_around_punctuation() {
        let src = "a + b ;";
        let out = minify(src);
        // Between identifier and `+` we have NO space (identifier
        // is word-like, `+` is not — only one side is word-like).
        // We accept either rendering as long as it round-trips
        // through the lexer; the conservative emitter we built
        // emits `a+b;`.
        assert_eq!(out, "a+b;");
    }

    #[test]
    fn preserves_string_literal_content() {
        // String literal contents are kept verbatim including
        // any whitespace inside them.
        let src = "var s = \"hello  world\";";
        let out = minify(src);
        assert!(out.contains("\"hello  world\""), "got: {out:?}");
    }

    #[test]
    fn multiline_input_becomes_single_line() {
        let src = "var\n  x\n  =\n  1\n;";
        let out = minify(src);
        assert!(!out.contains('\n'), "newlines should be stripped: {out:?}");
        assert!(out.contains("var x"));
    }

    #[test]
    fn nested_comments_and_whitespace_all_removed() {
        let src = r#"
            // top comment
            var /* inline */ x = 1; // trailing
            /* block
               with newline */
            var y = 2;
        "#;
        let out = minify(src);
        assert!(!out.contains("comment"));
        assert!(!out.contains("inline"));
        assert!(!out.contains("trailing"));
        assert!(!out.contains("block"));
        assert!(out.contains("var x"));
        assert!(out.contains("var y"));
    }

    #[test]
    fn error_display() {
        let e = MinifyError::LexError("bad input".into());
        assert!(e.to_string().contains("tokenizer failed"));
        let _: &dyn std::error::Error = &e;
    }

    // ---- gap-030 part 2: token-level ASI policy ------------

    /// The target fixture for gap-030. closurec must produce
    /// exactly upstream Closure v20240317's output:
    /// `function f(){return 1};`.
    #[test]
    fn gap030_function_decl_drops_inner_and_adds_trailing_semi() {
        assert_eq!(
            minify("function f(){return 1;}"),
            "function f(){return 1};"
        );
    }

    /// Empty function declaration still gets the trailing `;`
    /// (rule B). No `;` to drop inside since the body is empty.
    #[test]
    fn gap030_empty_function_decl_gets_trailing_semi() {
        assert_eq!(minify("function f(){}"), "function f(){};");
    }

    /// Function EXPRESSION (mid-expression, e.g.
    /// `var f=function(){};`) must NOT receive a synthetic
    /// trailing `;` after its `}` — the source's `;` outside
    /// the expression is the only terminator.
    #[test]
    fn gap030_function_expression_does_not_get_trailing_semi() {
        assert_eq!(
            minify("var f=function(){};"),
            "var f=function(){};"
        );
    }

    /// A block that is NOT a function-decl body (e.g. an
    /// if-block) must NOT receive a trailing `;` after its
    /// `}`. The inner `;` before `}` IS droppable because the
    /// last child is a terminator (`y()` expression).
    ///
    /// **gap-032 interaction**: As of CLOC12.42, single-
    /// statement if-bodies in body position are flattened
    /// (the `{` and `}` removed entirely), so this case now
    /// emits `if(x)y();` instead of `if(x){y()}`. Both
    /// outputs are semantically equivalent valid JS; the
    /// flattened form is what upstream Closure produces and
    /// is a strict improvement.
    #[test]
    fn gap030_if_block_drops_inner_semi_no_trailing() {
        assert_eq!(minify("if(x){y();}"), "if(x)y();");
    }

    /// Critical correctness gate (same shape as the
    /// security-review-caught bug in CLOC12.38, mirrored at
    /// the token level): the `;` in `if(x);` is structurally
    /// the body of the if-statement, NOT a terminator.
    /// Dropping it would produce `if(x)}` — SyntaxError.
    /// `body_position_next` flag suppresses the drop.
    #[test]
    fn gap030_does_not_drop_empty_body_of_if() {
        assert_eq!(
            minify("function f(){if(x);}"),
            "function f(){if(x);};"
        );
    }

    /// Same defense for `while(x);` body shape.
    #[test]
    fn gap030_does_not_drop_empty_body_of_while() {
        assert_eq!(
            minify("function f(){while(x);}"),
            "function f(){while(x);};"
        );
    }

    /// `for(;;);` — the `;` tokens inside `for(...)` are
    /// grammar parts of the for-head and are emitted as-is.
    /// The `;` after `)` is the for-loop's EmptyStatement
    /// body, which `body_position_next` protects from drop.
    #[test]
    fn gap030_for_loop_empty_body_preserved() {
        assert_eq!(
            minify("function f(){for(;;);}"),
            "function f(){for(;;);};"
        );
    }

    /// Source `;` immediately following a synthetic trailing
    /// `;` is deduped (rule C). Otherwise
    /// `function f(){};var g=1;` would emit `};;var g=1;`.
    #[test]
    fn gap030_dedup_source_semi_after_synthetic() {
        assert_eq!(
            minify("function f(){};var g=1;"),
            "function f(){};var g=1;"
        );
    }

    /// Multi-statement function body: only the FINAL `;`
    /// before `}` is dropped; intermediate `;`s between
    /// sibling statements are preserved.
    #[test]
    fn gap030_multi_stmt_body_drops_only_last_semi() {
        assert_eq!(
            minify("function f(){a;b;}"),
            "function f(){a;b};"
        );
    }

    /// Top-level `var x=1;` is untouched — no function-decl
    /// context, no `;`-before-`}` situation.
    #[test]
    fn gap030_top_level_var_unchanged() {
        assert_eq!(minify("var x=1;"), "var x=1;");
    }

    // ---- gap-033: try/catch/finally trailing `;` ----------

    /// The target fixture for gap-033. A try/catch chain
    /// gets a trailing `;` after the LAST clause's `}`.
    /// Inner `;`s are correctly dropped by rule A from
    /// gap-030 (already working before this change).
    #[test]
    fn gap033_try_catch_gets_trailing_semi() {
        assert_eq!(
            minify("try{a();}catch(e){b();}"),
            "try{a()}catch(e){b()};"
        );
    }

    /// try-only (no catch, but a finally) gets `;` after
    /// `finally`'s `}`.
    #[test]
    fn gap033_try_finally_gets_trailing_semi() {
        assert_eq!(
            minify("try{a();}finally{c();}"),
            "try{a()}finally{c()};"
        );
    }

    /// try/catch/finally chain — only the FINAL `}` gets `;`.
    /// The brace stack pops TryChain three times; the first
    /// two pops suppress `;` because next non-trivia is
    /// `catch` or `finally`.
    #[test]
    fn gap033_try_catch_finally_only_final_semi() {
        assert_eq!(
            minify("try{a();}catch(e){b();}finally{c();}"),
            "try{a()}catch(e){b()}finally{c()};"
        );
    }

    /// Nested try/catch — each chain independently gets its
    /// own trailing `;`. Important regression: brace_stack
    /// must track depth, not a single boolean.
    #[test]
    fn gap033_nested_try_catch_each_gets_semi() {
        // try{ try{a;}catch(e){b;} }catch(f){c;}
        // After inner-catch `}`: peek is `}` (outer-try's),
        // not catch/finally — inner chain ends, emit `;`.
        // After outer-try-body `}`: peek is `catch`, chain
        // continues, no `;`.
        // After outer-catch `}`: peek is EOF, chain ends,
        // emit `;`.
        assert_eq!(
            minify("try{try{a;}catch(e){b;}}catch(f){c;}"),
            "try{try{a}catch(e){b};}catch(f){c};"
        );
    }

    /// Critical regression test: gap-030's function-decl
    /// trailing `;` is preserved when function-decl appears
    /// inside a try-block. brace_stack handles Function and
    /// TryChain as separate cases — they don't interfere.
    #[test]
    fn gap033_function_decl_inside_try_block_still_gets_semi() {
        // try{ function f(){} } catches no semi inside (rule A
        // wouldn't fire on the function-decl `}` because
        // brace_stack pop is Function not TryChain). The
        // function-decl gets its own `;`. The try-body's `}`
        // then pops TryChain.
        assert_eq!(
            minify("try{function f(){}}catch(e){b;}"),
            "try{function f(){};}catch(e){b};"
        );
    }

    /// catch-with-no-binding (ES2019 optional catch binding):
    /// `try{a;}catch{b;}` — no `(e)`. Should still work.
    #[test]
    fn gap033_optional_catch_binding() {
        assert_eq!(
            minify("try{a;}catch{b;}"),
            "try{a}catch{b};"
        );
    }

    /// Critical regression from pre-push security review:
    /// `try` as an OBJECT-LITERAL PROPERTY NAME must NOT arm
    /// the try-chain flag. Without this guard, the flag would
    /// leak forward and contaminate the next unrelated `{` —
    /// specifically breaking `do{...}while(...)` because the
    /// spurious `;` after the do-body's `}` would terminate
    /// the do-statement and turn `while(x)` into an
    /// independent while-loop, a grammar error.
    #[test]
    fn gap033_try_as_object_property_does_not_arm() {
        // The original failing input the security review
        // identified.
        assert_eq!(
            minify("var t={try:1};do{y}while(x);"),
            "var t={try:1};do{y}while(x);"
        );
    }

    /// Same family as the above: `try` as a property name
    /// followed by other constructs that have their own `{}`.
    #[test]
    fn gap033_try_as_property_then_function_decl_unchanged() {
        assert_eq!(
            minify("var t={try:1};function g(){};var z=1;"),
            "var t={try:1};function g(){};var z=1;"
        );
    }

    /// `obj.catch(...)` — `catch` as method name on an
    /// object. The lexer tags `catch` as KEYWORD even here,
    /// so the state machine must use the prev-emitted-token
    /// guard (must be `}`) to filter out this case.
    #[test]
    fn gap033_catch_as_method_does_not_arm() {
        assert_eq!(
            minify("obj.catch(err);"),
            "obj.catch(err);"
        );
    }

    /// Promise-style chain: `p.then().catch(f)`. Both `catch`
    /// occurrences are method calls, not statement clauses.
    /// Neither should arm the try-chain.
    #[test]
    fn gap033_promise_catch_chain_unchanged() {
        assert_eq!(
            minify("p.then(g).catch(f);"),
            "p.then(g).catch(f);"
        );
    }

    // ---- gap-031: empty `{}` body collapses to `;` --------

    /// Target fixture: empty for-body collapses to `;`.
    #[test]
    fn gap031_empty_for_body_collapses() {
        assert_eq!(
            minify("for(var i=0;i<10;i++){}"),
            "for(var i=0;i<10;i++);"
        );
    }

    /// Empty while-body collapses.
    #[test]
    fn gap031_empty_while_body_collapses() {
        assert_eq!(minify("while(x){}"), "while(x);");
    }

    /// Empty if-body collapses (with no else).
    #[test]
    fn gap031_empty_if_body_collapses() {
        assert_eq!(minify("if(x){}"), "if(x);");
    }

    /// Empty function-decl body is NOT in body-position
    /// (the `(...)` head of `function` is not a control-flow
    /// head). So `function f(){}` stays as is + gets the
    /// gap-030 trailing `;`. Important non-regression for
    /// gap-031's interaction with gap-030.
    #[test]
    fn gap031_function_empty_body_unaffected() {
        assert_eq!(minify("function f(){}"), "function f(){};");
    }

    /// A NON-empty for-body must NOT collapse via gap-031 —
    /// the empty-`{}` rule requires the next token to be `}`.
    /// (gap-032 may further flatten the single-statement
    /// body — see below.)
    ///
    /// **gap-032 interaction**: With single-statement
    /// flattening now in place, `for(...){a;}` flattens to
    /// `for(...)a;` per CLOC12.42. Both outputs are
    /// semantically equivalent valid JS; the flattened form
    /// matches upstream and is strictly more minimal.
    #[test]
    fn gap031_nonempty_for_body_unaffected() {
        assert_eq!(
            minify("for(var i=0;i<10;i++){a;}"),
            "for(var i=0;i<10;i++)a;"
        );
    }

    /// Top-level `{}` is a Block statement, NOT in body
    /// position. body_position_next is false there, so the
    /// rule does not fire. (Plain `{}` stays as `{}`.)
    #[test]
    fn gap031_top_level_empty_block_unaffected() {
        // Top-level `{}` followed by a statement. The rule's
        // body_position_next guard means we don't substitute.
        assert_eq!(minify("{}var x=1;"), "{}var x=1;");
    }

    /// Object literal `{}` is in expression position, NOT in
    /// body position. body_position_next would be false
    /// (it's set by control-flow `)` only). Non-regression
    /// for empty object literals.
    #[test]
    fn gap031_empty_object_literal_unaffected() {
        assert_eq!(minify("var x={};"), "var x={};");
    }

    /// Critical interaction with gap-033: a try-body that is
    /// empty `{}`. The try-body's `{` is opening a TryChain
    /// block — but it is NOT in body position (try doesn't
    /// have a `(...)` head). So gap-031 does NOT fire and
    /// gap-033 correctly processes the try-chain.
    #[test]
    fn gap031_try_empty_body_unaffected() {
        // `try{}catch(e){a;}` — try-body stays `{}` because
        // it's not in body position; catch-body works as in
        // gap-033.
        assert_eq!(
            minify("try{}catch(e){a;}"),
            "try{}catch(e){a};"
        );
    }

    /// Optimality regression test (caught by pre-push
    /// security review). A source `;` immediately after the
    /// `{}` collapse-target gets folded into the synthetic
    /// `;` via rule C dedup. Without the
    /// `last_emit_was_synthetic_semi = true` line in the
    /// gap-031 branch, this would emit `for(...);;var x=1;`
    /// — syntactically valid (the extra `;` is an
    /// EmptyStatement) but ugly and non-minimal.
    #[test]
    fn gap031_for_loop_with_trailing_semi_deduped() {
        assert_eq!(
            minify("for(var i=0;i<10;i++){};var x=1;"),
            "for(var i=0;i<10;i++);var x=1;"
        );
    }

    // ---- gap-032: single-statement block flattening ------

    /// Target fixture: if/else with single-statement bodies.
    #[test]
    fn gap032_if_else_single_stmts_flatten() {
        assert_eq!(
            minify("if(x){a();}else{b();}"),
            "if(x)a();else b();"
        );
    }

    /// if-only (no else) with single-statement body.
    #[test]
    fn gap032_if_single_stmt_flattens() {
        assert_eq!(minify("if(x){a();}"), "if(x)a();");
    }

    /// while-body with single statement flattens.
    #[test]
    fn gap032_while_single_stmt_flattens() {
        assert_eq!(minify("while(x){a();}"), "while(x)a();");
    }

    /// for-body with single statement flattens.
    #[test]
    fn gap032_for_single_stmt_flattens() {
        assert_eq!(
            minify("for(var i=0;i<10;i++){a();}"),
            "for(var i=0;i<10;i++)a();"
        );
    }

    /// Multi-statement body MUST NOT flatten — there's
    /// more than one `;` at depth 0 inside the block.
    /// Without this non-regression test, the rule could
    /// over-fire and produce semantically different output.
    #[test]
    fn gap032_multi_stmt_body_does_not_flatten() {
        assert_eq!(
            minify("if(x){a();b();}"),
            "if(x){a();b()}"
        );
    }

    /// Body containing a nested control-flow keyword
    /// (`if`, `while`, `for`, etc.) MUST NOT flatten. The
    /// conservative pre-emit pathway can't track the inner
    /// body's structure, so we punt back to the normal
    /// brace-preserving path. The else-inner is the danger
    /// case: `if(x){if(y)a;}else{b;}` would, if flattened
    /// naively, become `if(x)if(y)a;else b;` — but the
    /// `else` would bind to the INNER `if(y)`, not the
    /// outer one. We avoid this entirely by refusing to
    /// flatten any body with a control-flow keyword.
    #[test]
    fn gap032_nested_if_does_not_flatten() {
        // The OUTER if-body has `has_blocking_keyword=true`
        // (the inner `if` is in the keyword exclude list),
        // so the outer block stays wrapped. The INNER
        // if-body, however, IS a single statement and DOES
        // get flattened by gap-032 — so `{a();}` becomes
        // `a();`. The inner-flatten's `;` survives the
        // outer block's closing `}` because the synthetic
        // `;` from gap-032's pre-emit path bypasses rule A.
        // Output is `if(x){if(y)a();}` — valid JS, just
        // slightly less minimal than upstream might produce.
        assert_eq!(
            minify("if(x){if(y){a();}}"),
            "if(x){if(y)a();}"
        );
    }

    /// Body containing a nested `{}` block MUST NOT flatten.
    /// `has_nested_brace` catches this.
    #[test]
    fn gap032_nested_brace_does_not_flatten() {
        assert_eq!(
            minify("if(x){{a();}}"),
            "if(x){{a()}}"
        );
    }

    /// Body containing `function` keyword MUST NOT flatten.
    /// Function declarations have their own block structure
    /// and trailing-`;` handling that the pre-emit pathway
    /// would corrupt.
    #[test]
    fn gap032_body_with_function_does_not_flatten() {
        // Outer if-body has `function` in the blocking-
        // keyword set, so flatten is blocked. The function-
        // decl inside then gets its normal gap-030 trailing
        // `;` after its `}` — the output is
        // `if(x){function f(){};}` which is the correct
        // composition of gap-030 + gap-032's bail-out.
        assert_eq!(
            minify("if(x){function f(){}}"),
            "if(x){function f(){};}"
        );
    }

    /// Body containing `try` keyword MUST NOT flatten. The
    /// try/catch chain has its own state machine that the
    /// pre-emit pathway would corrupt.
    #[test]
    fn gap032_body_with_try_does_not_flatten() {
        assert_eq!(
            minify("if(x){try{a();}catch(e){b();}}"),
            "if(x){try{a()}catch(e){b()};}"
        );
    }

    /// Body with `var` declaration containing a string with
    /// `;` inside flattens correctly — depth tracking is
    /// based on tokenized `;`, and string literals are
    /// single tokens not affecting depth.
    #[test]
    fn gap032_body_with_var_decl_flattens() {
        assert_eq!(
            minify("if(x){var y=1;}"),
            "if(x)var y=1;"
        );
    }

    /// Top-level `{a;}` (not in body position) MUST NOT
    /// flatten — body_position_next is false. The block is
    /// a statement in its own right.
    #[test]
    fn gap032_top_level_block_does_not_flatten() {
        assert_eq!(minify("{a;}"), "{a}");
    }

    /// Function-decl body MUST NOT flatten — not in body
    /// position. function-body stays as `{...}` and gets
    /// the gap-030 trailing `;` after `}`.
    #[test]
    fn gap032_function_body_does_not_flatten() {
        assert_eq!(
            minify("function f(){a();}"),
            "function f(){a()};"
        );
    }

    /// Try-body MUST NOT flatten — try-body's `{` is not in
    /// body_position_next true context. The gap-033
    /// try-chain processing still works.
    #[test]
    fn gap032_try_body_does_not_flatten() {
        assert_eq!(
            minify("try{a();}catch(e){b();}"),
            "try{a()}catch(e){b()};"
        );
    }

    // ---- gap-034: class declaration trailing `;` --------

    /// Target fixture: class declaration gets a trailing `;`.
    #[test]
    fn gap034_class_declaration_trailing_semi() {
        assert_eq!(minify("class C{m(){}}"), "class C{m(){}};");
    }

    /// Empty-body class also gets the trailing `;`. Composes
    /// with the rule that `class C{}` is NOT a body slot for
    /// gap-031's empty-`{}` collapse — the class body must
    /// stay as `{}` to be a valid class declaration.
    #[test]
    fn gap034_class_with_empty_body_gets_trailing_semi() {
        assert_eq!(minify("class C{}"), "class C{};");
    }

    /// class expression (mid-expression position, e.g. as
    /// the RHS of an assignment) does NOT arm the
    /// `saw_class_kw_at_boundary` flag, so the body's `}`
    /// is BlockKind::Other and no synthetic `;` fires.
    /// The surrounding `var x=...;` provides its own
    /// terminator.
    #[test]
    fn gap034_class_expression_does_not_get_trailing_semi() {
        assert_eq!(
            minify("var x=class{};"),
            "var x=class{};"
        );
    }

    // ---- gap-035: var/let/const → `{`/`[` separator -----

    /// Target fixture: `var{a}=x;` round-trips with a space
    /// between `var` and `{`.
    #[test]
    fn gap035_var_destructuring_inserts_space() {
        assert_eq!(minify("var{a}=x;"), "var {a}=x;");
    }

    /// `let` destructuring shape.
    #[test]
    fn gap035_let_destructuring_inserts_space() {
        assert_eq!(minify("let{a}=x;"), "let {a}=x;");
    }

    /// `const` destructuring shape.
    #[test]
    fn gap035_const_destructuring_inserts_space() {
        assert_eq!(minify("const{a}=x;"), "const {a}=x;");
    }

    /// Array destructuring `var[a,b]=x;` gets the same
    /// separator.
    #[test]
    fn gap035_var_array_destructuring_inserts_space() {
        assert_eq!(minify("var[a]=x;"), "var [a]=x;");
    }

    /// Non-destructuring `var x=1;` is unaffected — the
    /// `var` keyword is followed by an identifier, not
    /// `{`/`[`.
    #[test]
    fn gap035_simple_var_decl_unchanged() {
        assert_eq!(minify("var x=1;"), "var x=1;");
    }

    // ---- gap-036: switch trailing `;` --------------------

    /// Target fixture: switch statement gets a trailing `;`.
    #[test]
    fn gap036_switch_trailing_semi() {
        assert_eq!(
            minify("switch(x){case 1:y();break;}"),
            "switch(x){case 1:y();break};"
        );
    }

    /// Switch with default clause also gets the trailing `;`.
    #[test]
    fn gap036_switch_with_default_trailing_semi() {
        assert_eq!(
            minify("switch(x){default:y();}"),
            "switch(x){default:y()};"
        );
    }

    /// **Regression for security-review-caught bug.** `class`
    /// as an OBJECT-LITERAL PROPERTY NAME must NOT arm the
    /// class-declaration flag. Without the guard, the flag
    /// leaks forward and contaminates the next unrelated
    /// `{` — specifically breaking `do{...}while(...)` by
    /// emitting `do{y};while(x);` which is a SyntaxError.
    /// Same defect family as gap-033's `try`-as-property bug.
    #[test]
    fn gap034_class_as_property_does_not_arm() {
        assert_eq!(
            minify("var o={class:1};do{y}while(x);"),
            "var o={class:1};do{y}while(x);"
        );
    }

    /// **Regression for security-review-caught bug.** `switch`
    /// as an OBJECT-LITERAL PROPERTY NAME must NOT arm the
    /// switch-head flag. Without the guard, the flag would
    /// leak forward and contaminate an unrelated `(`/`)`/`{`
    /// chain (e.g. `while(x){a;b;}`), emitting a spurious
    /// `;` after the while-body's `}`. Not a SyntaxError on
    /// its own (the extra `;` is an EmptyStatement), but
    /// still a parity divergence vs upstream.
    #[test]
    fn gap036_switch_as_property_does_not_arm() {
        assert_eq!(
            minify("var o={switch:1};while(x){a;b;}"),
            "var o={switch:1};while(x){a;b}"
        );
    }

    // ---- gap-037: async function trailing `;` ------------

    /// Target fixture: async function declaration gets a
    /// trailing `;` mirroring gap-030's plain-function rule.
    #[test]
    fn gap037_async_function_trailing_semi() {
        assert_eq!(
            minify("async function f(){await x;}"),
            "async function f(){await x};"
        );
    }

    /// Empty-body async function also gets the trailing `;`.
    #[test]
    fn gap037_empty_async_function_trailing_semi() {
        assert_eq!(
            minify("async function f(){}"),
            "async function f(){};"
        );
    }

    /// **Non-regression**: `async` as method-shorthand name
    /// in an object literal (`{async(){...}}`) must NOT arm
    /// the flag. The next token after `async` is the IDENT
    /// for the method name (or `(` for the bare shorthand),
    /// not `function`, so the guard correctly filters this.
    #[test]
    fn gap037_async_method_shorthand_does_not_arm() {
        // `{async f(){}}` — `async` shorthand prefix on a
        // method. Output: object literal preserved; no
        // trailing `;` injected.
        assert_eq!(
            minify("var o={async f(){}};"),
            "var o={async f(){}};"
        );
    }

    /// **Non-regression**: `async()=>x` (async arrow
    /// function) must NOT arm. Next token is `(`, not
    /// `function`.
    #[test]
    fn gap037_async_arrow_does_not_arm() {
        assert_eq!(
            minify("var f=async()=>1;"),
            "var f=async()=>1;"
        );
    }

    /// **Non-regression**: async function EXPRESSION (mid-
    /// expression position) doesn't get the trailing `;`.
    /// at_stmt_boundary is false in this position.
    #[test]
    fn gap037_async_function_expression_no_trailing_semi() {
        assert_eq!(
            minify("var f=async function(){};"),
            "var f=async function(){};"
        );
    }

    // ---- gap-038: number literal shortest-form normalisation

    /// Target fixture: hex literal `0xff` shortens to `255`.
    #[test]
    fn gap038_hex_short() {
        assert_eq!(minify("var x=0xff;"), "var x=255;");
    }

    /// Hex tie-break: `0xffffffff` (10 chars) → `4294967295`
    /// (10 chars). Upstream tie-breaks to DECIMAL.
    #[test]
    fn gap038_hex_tie_picks_decimal() {
        assert_eq!(
            minify("var x=0xffffffff;"),
            "var x=4294967295;"
        );
    }

    /// Hex too long for decimal: 14-char hex →
    /// 15-char decimal. Keep hex.
    #[test]
    fn gap038_hex_kept_when_shorter() {
        assert_eq!(
            minify("var x=0xffffffffffff;"),
            "var x=0xffffffffffff;"
        );
    }

    /// Octal literal: `0o777` → `511` (5 chars → 3).
    #[test]
    fn gap038_octal_short() {
        assert_eq!(minify("var x=0o777;"), "var x=511;");
    }

    /// Binary literal: `0b1010` → `10` (6 chars → 2).
    #[test]
    fn gap038_binary_short() {
        assert_eq!(minify("var x=0b1010;"), "var x=10;");
    }

    /// Uppercase prefix `0X` is equivalent to `0x` per
    /// §12.8.3 — same normalisation rule applies.
    #[test]
    fn gap038_uppercase_hex_prefix() {
        assert_eq!(minify("var x=0XFF;"), "var x=255;");
    }

    /// **Non-regression**: decimal literals are NOT touched.
    /// Decimal-floating-point shortest-form (e.g. `0.5` → `.5`)
    /// is a different rule and out of scope for this gap.
    #[test]
    fn gap038_decimal_unchanged() {
        assert_eq!(minify("var x=42;"), "var x=42;");
    }

    /// **Non-regression**: BigInt literals are NOT touched.
    /// `0xfn` would canonicalise to `15n` upstream, but
    /// that requires bigint arithmetic — left for a
    /// follow-up gap. Leaving verbatim is safer than
    /// truncating.
    #[test]
    fn gap038_bigint_left_verbatim() {
        assert_eq!(minify("var x=0xfn;"), "var x=0xfn;");
    }

    /// **Non-regression**: number that overflows u128 stays
    /// verbatim rather than panicking. Hex digits >>32 are
    /// rare in practice but we must not crash on input that
    /// happens to be very large.
    #[test]
    fn gap038_overflow_left_verbatim() {
        let big = "0x".to_string() + &"f".repeat(40); // u160 worth
        let src = format!("var x={};", big);
        assert_eq!(minify(&src), src);
    }
}
