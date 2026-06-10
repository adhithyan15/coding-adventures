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
    let mut kept: Vec<_> = tokens
        .iter()
        .filter(|t| !is_trivia(t))
        .filter(|t| !is_eof(t))
        .collect();

    // gap-051: IIFE paren normalisation. Upstream Closure
    // rewrites `(function(){...}())` to `(function(){...})()`
    // — the call's `()` moves OUTSIDE the wrapping parens.
    // The two forms are operationally identical (both call
    // the function expression) and have identical byte
    // count; this is a pure normalisation preference that
    // upstream applies.
    //
    // Token-level pattern: `} ( ) )` (anywhere in the
    // stream) becomes `} ) ( )` — we swap the call-close
    // `)` with the outer-close `)`. The `(` at position
    // i+1 stays (it's the call-open); the `)` at i+2
    // becomes the outer-close; the original outer-close
    // at i+3 becomes the call-close.
    //
    // Actually more precisely: we reorder the 4-token
    // window `} ( ) )` to `} ) ( )`. The `}` at i stays;
    // the `)` originally at i+3 moves to i+1; the `(`
    // originally at i+1 moves to i+2; the `)` originally
    // at i+2 stays at i+3 logically — equivalent to a
    // 3-token rotation within `[i+1, i+2, i+3]`.
    //
    // Safety: this pattern can ONLY appear in IIFE
    // contexts where the inner `()` is a call on the
    // immediately-prior function body. Any non-IIFE
    // `} ( ) )` sequence in valid JS would imply
    // something like `class Foo {} () ()` which is a
    // syntax error. So the rewrite is sound.
    //
    // Note we don't gate on what's BEFORE the `}` — it
    // could be a function body, a class body, or anything.
    // For class bodies, `class C{} ()` would be weird; but
    // upstream's normalisation applies to function bodies
    // specifically. To be safe, we additionally require
    // the `}` to be preceded by a token that could end a
    // function body (which is essentially any expression
    // closer — too broad). Simpler check: look back for
    // the matching `{` and confirm it's preceded by `)`
    // (function arg-list close) or `=>` (arrow head). For
    // now, the simpler pattern match alone — if false
    // positives surface, refine with the backwards scan.
    {
        let mut i = 0;
        while i + 3 < kept.len() {
            if kept[i].value == "}"
                && kept[i + 1].value == "("
                && kept[i + 2].value == ")"
                && kept[i + 3].value == ")"
            {
                // Swap the call-close at i+2 with the
                // outer-close at i+3. After swap, the
                // sequence is `} ( ) )` → `} ) ( )` where
                // the first `)` is the outer close.
                // Wait — we want `} ) ( )` but we have
                // `} ( ) )`. To go from input to output,
                // we move the call-pair `( )` (at i+1,
                // i+2) AFTER the outer `)` (at i+3).
                //
                // Implementation: rotate the slice
                // [i+1..=i+3] one position right (which
                // is equivalent to rotating one left
                // since it's a 3-element window):
                //   [A=(,  B=),  C=)] → [C, A, B]
                //   so (, ), ) → ), (, )
                kept[i + 1..=i + 3].rotate_right(1);
                // Advance past the now-rewritten window.
                i += 4;
            } else {
                i += 1;
            }
        }
    }
    let kept = kept;

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
    let mut deferred_synthetic_semi = false;
    // ^ gap-041: when a `}` would emit a synthetic `;` but
    //   the very next non-trivia token is ANOTHER `}`, the
    //   `;` is deferred to that outer `}`. This lifts
    //   nested-function-decl terminators past the enclosing
    //   brace, matching upstream's `function f(){function
    //   g(){}}` → `function f(){function g(){}};` (single
    //   `;` on the outermost `}`).
    let mut prev_emitted_tok: Option<&lexer::token::Token> = None;

    let mut idx = 0usize;
    while idx < kept.len() {
        let tok = kept[idx];
        let val = tok.value.as_str();

        // gap-045: single-arg arrow function — drop the
        // enclosing parens around a bare-identifier param.
        // Pattern: `(`, IDENT, `)`, `=>` — when matched,
        // emit IDENT + `=>` directly, advancing past all 4
        // tokens. Eligibility deliberately excludes:
        //   - zero args (`()=>...` — next-after-`(` is `)`)
        //   - destructuring (`({x})=>...`, `([x])=>...`)
        //   - rest (`(...args)=>...` — next is `...`)
        //   - default values (`(x=1)=>...` — `)` is at idx+4 not +2)
        //   - multiple args (`(x,y)=>...` — `)` is at idx+4+)
        //   - typed params (TS) — multiple tokens between
        //     the `(` and `)`.
        // The check `is_simple_identifier_token` ensures the
        // single token between parens is a NAME (not a
        // keyword or operator).
        //
        // Composes with `async` keyword: `async(x)=>...` →
        // `async x=>...`. The `async` is emitted before the
        // `(`, and our shape-detection runs ON the `(` —
        // so `async` ends up with `prev_emitted_tok = async`
        // when IDENT is emitted, triggering needs_separator
        // (KEYWORD + NAME) → space inserted. Correct.
        // gap-050: `new IDENT ( )` → `new IDENT`. Empty
        // constructor arg-list elision. Upstream Closure
        // normalises `new Foo()` to `new Foo` because the
        // two NewExpression forms are operationally
        // equivalent in ECMAScript.
        //
        // Safety: we must NOT drop when the result would
        // syntactically fuse with what follows the
        // dropped `()`. Specifically:
        //   - `new Foo().bar` ≠ `new Foo.bar`
        //     (left: member-access on constructed object;
        //     right: NewExpression on the member `Foo.bar`)
        //   - `new Foo()[x]` ≠ `new Foo[x]`
        //     (same family — element-access on result vs.
        //     constructor of `Foo[x]`)
        //   - `new Foo()()` ≠ `new Foo()`
        //     (left: invoke constructed object as fn;
        //     right: lose the chained call entirely)
        //   - `new Foo()` ` `tag` ≠ `new Foo` ` `tag`
        //     (tagged template — tighter than NewExpression)
        //
        // Conservative gate: kept[idx-2] == "new", kept[idx-1]
        // is a simple identifier, kept[idx+1] == ")", and
        // kept[idx+2] is none of {`(`, `.`, `[`, `` ` ``}.
        // Other followers (`;`, `}`, `,`, EOF, infix
        // operators like `+`, `instanceof`, etc.) all bind
        // LOOSER than NewExpression and therefore are safe.
        let next2_starts_with_backtick = kept
            .get(idx + 2)
            .map(|t| t.value.starts_with('`'))
            .unwrap_or(false);
        let next2_blocks_drop = matches!(
            kept.get(idx + 2).map(|t| t.value.as_str()),
            Some("(") | Some(".") | Some("[")
        ) || next2_starts_with_backtick;
        if val == "("
            && idx >= 2
            && kept[idx - 2].value == "new"
            && is_simple_identifier_token(Some(kept[idx - 1]))
            && kept.get(idx + 1).map(|t| t.value.as_str()) == Some(")")
            && !next2_blocks_drop
        {
            // Skip both `(` and `)`. State machine
            // unchanged — these tokens were going to be
            // ignored for at_stmt_boundary /
            // body_position_next purposes anyway (parens
            // around an empty arg list don't open a body
            // slot).
            idx += 2;
            continue;
        }

        if val == "("
            && is_simple_identifier_token(kept.get(idx + 1).copied())
            && kept.get(idx + 2).map(|t| t.value.as_str()) == Some(")")
            && kept.get(idx + 3).map(|t| t.value.as_str()) == Some("=>")
        {
            let ident = kept[idx + 1];
            // Emit IDENT with separator if needed.
            if let Some(prev) = prev_emitted_tok {
                if needs_separator(prev, ident) {
                    out.push(' ');
                }
            }
            out.push_str(&ident.value);
            prev_emitted_tok = Some(ident);
            // Emit `=>` — never needs a separator with an
            // IDENT prefix (`=` is PUNCTUATION).
            out.push_str("=>");
            // Update state machine for the now-emitted `=>`:
            // body-position semantics don't apply here (arrow
            // body opens differently), but at_stmt_boundary
            // and body_position_next should reset to "we are
            // mid-expression".
            at_stmt_boundary = false;
            body_position_next = false;
            // Synthetic emit shouldn't be flagged as one for
            // rule-C dedup purposes — it's not a `;`.
            last_emit_was_synthetic_semi = false;
            // `=>` token is a punctuation; track it as prev
            // for the next iteration's needs_separator. We
            // can't store the original token because we
            // skipped it; reuse the IDENT slot — the next
            // token's needs_separator call will see word-like
            // IDENT as prev. For `x=>y` next is `y` (NAME),
            // word-like(NAME, NAME) → space. WRONG. So set
            // prev_emitted_tok to the `=>` token instead.
            prev_emitted_tok = Some(kept[idx + 3]);
            idx += 4;
            continue;
        }

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

        // gap-046: drop a trailing `,` in array literals.
        // Upstream Closure under WHITESPACE_ONLY normalises
        // `[1,2,]` → `[1,2]` — the trailing comma carries no
        // semantic weight (it doesn't create an elision when
        // it appears as the LAST element), and skipping it
        // saves a byte. When the next non-trivia token is
        // `]`, suppress the `,` emission entirely.
        //
        // **Object-literal trailing comma** (`{a:1,}` →
        // `{a:1}`) follows the same logic but requires
        // discriminating a literal `}` from a block-close
        // `}` — the latter case (`{a;b;}`) would have its
        // `;` dropped by rule A, not by this rule. Deferred
        // to a future gap (gap-046b) so the array case lands
        // first as a clean minimum.
        //
        // **Elision sequences** (`[,,a]`, `[a,,b]`) are
        // SEMANTIC — they create `undefined` slots in the
        // array. The rule here only fires when the `,`
        // appears IMMEDIATELY before `]` — i.e. as the
        // trailing-comma-after-last-element form. Inner
        // elisions have a non-`]` token next.
        if val == ","
            && kept.get(idx + 1).map(|t| t.value.as_str()) == Some("]")
        {
            idx += 1;
            continue;
        }

        // gap-046b: drop a trailing `,` in object literals
        // and object destructuring patterns. Upstream
        // Closure normalises `{a:1,}` → `{a:1}` and
        // `var {a,}=o` → `var {a}=o`.
        //
        // **Safe-to-drop unconditionally**: in VALID
        // ECMAScript, a `,` immediately before `}` can ONLY
        // appear in object-literal / object-destructuring
        // contexts. Other `}` contexts can't have `,`
        // directly preceding:
        //   - Block: statements are `;`-separated (or ASI);
        //     `{a, b}` parses as a block containing the
        //     comma-expression `a, b` as an expression
        //     statement, with `;` inserted before `}`. The
        //     `,` is between `a` and `b`, not before `}`.
        //   - Function body / arrow body: same as block.
        //   - Class body: members have NO separator (no `,`
        //     between methods/fields).
        //   - switch body: `case` arms, no `,`.
        //   - try/catch/finally body: same as block.
        //
        // So we can drop without checking brace_stack.
        if val == ","
            && kept.get(idx + 1).map(|t| t.value.as_str()) == Some("}")
        {
            idx += 1;
            continue;
        }

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
                    // gap-049: if the next token AFTER the
                    // closing `}` is itself a `}` (outer
                    // block boundary), the inlined trailing
                    // `;` is redundant — Rule A would drop
                    // a source `;` at this position, and
                    // logically the inline `;` is occupying
                    // exactly that slot. Without this
                    // suppression we'd emit
                    //   `function f(){for(var v of a)a;};`
                    // when upstream Closure produces
                    //   `function f(){for(var v of a)a};`.
                    //
                    // The check has to happen here (inside
                    // the flatten) because once the
                    // contents are pre-emitted, the main
                    // state machine doesn't re-scan them
                    // and Rule A only fires on tokens still
                    // in `kept`.
                    let next_after_close =
                        kept.get(close_idx + 1).map(|t| t.value.as_str());
                    let drop_trailing_semi = next_after_close == Some("}");
                    // Pre-emit content tokens (idx+1 ..
                    // emit_end). Each token gets the same
                    // separator + quoting treatment as the
                    // main loop, but the state machine isn't
                    // run on them — they're carried through
                    // verbatim. This is safe because we
                    // verified the contents are a single
                    // simple statement with no nested
                    // structure.
                    let emit_end = if drop_trailing_semi {
                        // close_idx - 1 is the trailing `;`
                        // (verified by `last_before_close ==
                        // ";"` in the eligibility check).
                        close_idx - 1
                    } else {
                        close_idx
                    };
                    for content_idx in (idx + 1)..emit_end {
                        let t = kept[content_idx];
                        if let Some(prev) = prev_emitted_tok {
                            if needs_separator(prev, t) {
                                out.push(' ');
                            }
                        }
                        if is_string_literal(t) {
                            emit_quoted_string(&mut out, &t.value);
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
            emit_quoted_string(&mut out, &tok.value);
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
            let next_val = kept.get(idx + 1).map(|t| t.value.as_str());
            let next_is_close_brace = next_val == Some("}");
            let next_is_chain_continuation =
                matches!(next_val, Some("catch") | Some("finally"));
            // gap-047: ASI cleanly covers the boundary
            // between `}` and a new statement-starting
            // keyword — no synthetic `;` is needed.
            // Upstream Closure under WHITESPACE_ONLY emits
            //   `function add(a,b){return a+b}var sum=add(2,3);`
            // for a source that has both a function decl and
            // a subsequent `var`. Without this suppression
            // we emit `function add(a,b){return a+b};var sum=...;`
            // — the extra `;` is a wasted byte.
            //
            // The keyword set is the closed list of tokens
            // that grammatically START a new statement and
            // CANNOT continue the previous expression. This
            // is the safety guarantee: omitting the `;` here
            // is always parser-correct because there's no
            // way for `}` followed by these keywords to
            // semantically join into a single statement.
            //
            // Notably ABSENT: identifier-like names
            // (`x`, `foo`, etc.) — could be a free expression
            // that grammar-fuses with the preceding `}` in
            // some constructs (e.g., function expressions vs
            // declarations). Keeping `;` here is safer. EOF
            // (next_val == None) also keeps the `;` per
            // gap-030's original rule for cases like
            // `function f(){}` (no trailing context).
            let next_is_stmt_keyword = matches!(
                next_val,
                Some("var") | Some("let") | Some("const")
                    | Some("function") | Some("class")
                    | Some("if") | Some("for") | Some("while")
                    | Some("do") | Some("switch") | Some("try")
                    | Some("return") | Some("throw")
                    | Some("break") | Some("continue")
                    | Some("import") | Some("export")
            );
            let kind_wants_semi = match kind {
                BlockKind::Function => true,
                BlockKind::Class => true,    // gap-034
                BlockKind::Switch => true,   // gap-036
                BlockKind::TryChain => {
                    // gap-033: chain continues iff next is
                    // `catch` / `finally`.
                    !next_is_chain_continuation
                }
                BlockKind::Other => false,
            };
            // gap-041: when a synthetic `;` is owed at this
            // `}` AND the very next non-trivia token is
            // another `}`, defer the `;` PAST the outer
            // brace. This matches upstream's behaviour:
            //   `function f(){function g(){}}` →
            //   `function f(){function g(){}};` (single `;`
            //   on the outermost `}`).
            //   `if(x){function f(){}}` →
            //   `if(x){function f(){}};` (`;` lifted out of
            //   Other-block onto the outer-`}` position).
            // The mechanism: a `deferred_synthetic_semi`
            // flag is carried forward, and any subsequent
            // `}` consumes it as if it were the source of
            // the `;`. If the carrying `}` would itself
            // emit a `;`, the two collapse to one.
            //
            // TryChain interaction: if next is `catch` or
            // `finally`, neither a fresh `;` NOR a deferred
            // one may be emitted (the chain owns the
            // terminator). The deferred state survives
            // across the suppression.
            let emit_semi;
            if kind_wants_semi && next_is_close_brace {
                // Defer.
                deferred_synthetic_semi = true;
                emit_semi = false;
            } else if next_is_close_brace {
                // This kind doesn't owe a `;`; just
                // propagate any deferred state through.
                emit_semi = false;
            } else if next_is_chain_continuation {
                // Carry the deferred forward past the chain
                // boundary; nothing emits here.
                emit_semi = false;
            } else if next_is_stmt_keyword {
                // gap-047: ASI covers the boundary. No `;`
                // needed. Drop both the kind-wants-semi and
                // any deferred pending — the next statement
                // can stand on its own without us.
                deferred_synthetic_semi = false;
                emit_semi = false;
            } else {
                // Emit if either we owe one OR a deferred
                // one is pending. Both collapse to a single
                // emission.
                emit_semi = kind_wants_semi || deferred_synthetic_semi;
                deferred_synthetic_semi = false;
            }
            if emit_semi {
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
        } else if val == "do" {
            // gap-042: `do` opens the do-statement's body
            // slot IMMEDIATELY (unlike `if`/`while`/`for`
            // which open it after the `)` of the head). Per
            // §13.7.2: `DoWhileStatement → do Statement
            // while ( Expression ) ;`. By arming
            // body_position_next here, gap-032's
            // single-statement flatten fires on
            // `do{a;}while(x);` → `do a;while(x);`.
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

/// gap-045: True iff this token is a simple bare IDENT
/// (TokenType::Name) — i.e. a candidate single-arg arrow
/// parameter. Returns false for NONE input, for keywords,
/// punctuation, and any non-NAME token type.
///
/// The arrow-paren-drop pattern requires a bare identifier
/// because:
///   - `(x)=>...` legal (single param name)
///   - `({x})=>...` destructuring — `{` is PUNCT, not Name
///   - `(...args)=>...` rest — `...` is PUNCT, not Name
///   - `(true)=>...` is a SyntaxError (reserved word as
///     param name); even if the lexer tagged `true` as
///     Keyword, this check rejects it.
///   - `(x=1)=>...` default value — `(`, IDENT, `=`, ... —
///     the position-2 check (next is `)`) catches this.
fn is_simple_identifier_token(tok: Option<&lexer::token::Token>) -> bool {
    let Some(t) = tok else {
        return false;
    };
    matches!(t.type_, lexer::token::TokenType::Name)
}

/// True iff this token is a numeric literal (NUMBER token).
/// Mirrors `is_string_literal`'s grammar-name detection but
/// for the numeric-rule family. Used by gap-038's shortest-
/// form normalisation.
fn is_number_literal(tok: &lexer::token::Token) -> bool {
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        // gap-048: BIGINT tokens (e.g. `1_000_000n`) go
        // through the same normalize path so the ES2021
        // `_` separator gets stripped. The full radix +
        // shortest-form normalization is short-circuited
        // inside `normalize_number_value` for BigInt
        // (the suffix `n` is detected there) — only the
        // separator-stripping applies.
        if upper == "NUMBER"
            || upper == "NUMERIC_LITERAL"
            || upper == "NUMBER_LITERAL"
            || upper == "BIGINT"
            || upper == "BIGINT_LITERAL"
        {
            return true;
        }
    }
    matches!(tok.type_, lexer::token::TokenType::Number)
}

/// gap-038 + gap-040: rewrite numeric integer literals to
/// their shortest representation among the three candidates
/// {cleaned source form, decimal form, scientific form},
/// matching upstream Closure's WHITESPACE_ONLY behaviour.
///
/// **The candidates:**
///   - **Cleaned**: original `value` with ES2021 numeric
///     separators (`_`) stripped. For hex/oct/bin, this
///     keeps the radix prefix; for decimal, this is the
///     plain digit run.
///   - **Decimal**: `n.to_string()` — the canonical base-10
///     form.
///   - **Scientific**: when `n = m × 10^e` with `m % 10 ≠ 0`
///     and `e ≥ 1`, the form `"{m}E{e}"`. None if `n == 0`
///     or `e == 0`.
///
/// **Tie-break order** (when lengths are equal): decimal >
/// cleaned > scientific. Verified against
/// `closure-compiler-v20240317.jar` for boundary cases:
///   - `0xffffffff` (10 cleaned hex) = `4294967295` (10
///     decimal) → DECIMAL wins.
///   - `100` (3 decimal) = `1E2` (3 scientific) → DECIMAL
///     wins.
///
/// **Worked examples** (all verified against the JAR):
///   `0xff` (4)       → `255` (3) → DECIMAL
///   `0xffffffffffff` (14) → `281474976710655` (15) → CLEANED HEX
///   `1_000` (5)      → cleaned `1000` (4), sci `1E3` (3) → SCIENTIFIC
///   `1_000_000` (9)  → cleaned `1000000` (7), sci `1E6` (3) → SCIENTIFIC
///   `1_234_567` (9)  → cleaned `1234567` (7), no sci → CLEANED
///   `0xff_ff` (7)    → cleaned `0xffff` (6), decimal `65535` (5) → DECIMAL
///   `12000` (5)      → sci `12E3` (4) → SCIENTIFIC
///   `1234500` (7)    → sci `12345E2` (7) → DECIMAL (tie)
///
/// **Not handled (left for follow-up gaps):**
///   - BigInt literals (suffix `n`) need bigint arithmetic.
///   - Decimal floating-point shortest-form (`0.5` → `.5`,
///     `10.0` → `10`).
///   - Numbers exceeding `u128::MAX` stay verbatim.
fn normalize_number_value(value: &str) -> String {
    // gap-048: BigInt literals don't get the full
    // radix-and-shortest-form normalization that regular
    // numbers do — that requires bigint arithmetic and
    // is deferred. BUT the ES2021 `_` numeric separator
    // is PURELY LEXICAL sugar (the body before `n` is
    // still a decimal literal); we can strip it
    // independently of any arithmetic. So:
    //   `1_000_000n` → `1000000n` ✓ (gap-048)
    //   `0x1_FFFn`   → `0x1FFFn`  ✓ (gap-048)
    //   `9007199254740993n` → unchanged (no separators)
    //   `9007199254740993n` → NOT normalized to decimal
    //     shortest-form (would need bigint math, deferred)
    if let Some(body) = value.strip_suffix('n') {
        if body.contains('_') {
            return format!("{}n", body.replace('_', ""));
        }
        return value.to_string();
    }
    // gap-040: strip ES2021 numeric separators (`_`).
    // `u128::from_str_radix` doesn't accept them; this also
    // makes the cleaned form a candidate for shortest-form
    // comparison.
    let cleaned = if value.contains('_') {
        value.replace('_', "")
    } else {
        value.to_string()
    };
    // Try each radix prefix in turn. Prefixes are
    // case-insensitive per §12.8.3. The decimal branch
    // (no radix prefix) also feeds into shortest-form for
    // gap-040 — bare-decimal source can still be shorter
    // in scientific form (e.g. `1000` → `1E3`).
    let parsed: Option<u128> = if let Some(rest) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        u128::from_str_radix(rest, 16).ok()
    } else if let Some(rest) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        u128::from_str_radix(rest, 8).ok()
    } else if let Some(rest) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        u128::from_str_radix(rest, 2).ok()
    } else if cleaned.chars().all(|c| c.is_ascii_digit()) {
        // Bare decimal integer.
        cleaned.parse::<u128>().ok()
    } else {
        // Has a `.` or `e`/`E` or other non-integer
        // character — leave alone. Decimal floating-point
        // shortest-form (e.g. `0.5` → `.5`) is a separate
        // future gap.
        return value.to_string();
    };

    let Some(n) = parsed else {
        // Doesn't fit in u128 — leave verbatim.
        return value.to_string();
    };

    let decimal = n.to_string();
    let scientific = scientific_form_of(n);

    // Pick shortest. Tie-break order: decimal > cleaned >
    // scientific (verified via JAR probes for the boundary
    // cases — see function-level doc).
    let cleaned_len = cleaned.len();
    let decimal_len = decimal.len();
    let sci_len = scientific.as_ref().map(|s| s.len()).unwrap_or(usize::MAX);

    let min_len = cleaned_len.min(decimal_len).min(sci_len);
    if decimal_len == min_len {
        decimal
    } else if cleaned_len == min_len {
        cleaned
    } else {
        scientific.unwrap()
    }
}

/// gap-040: scientific shortest-form helper. For an integer
/// `n = m × 10^e` with `m % 10 ≠ 0` and `e ≥ 1`, returns
/// `Some("{m}E{e}")`. Returns `None` when `n == 0` or `e ==
/// 0` (the latter because `"{n}E0"` is always longer than
/// just `"{n}"`).
///
/// Upstream uses uppercase `E` (verified by JAR probes:
/// `1e3` → `1E3`, `2e10` → `2E10`).
fn scientific_form_of(n: u128) -> Option<String> {
    if n == 0 {
        return None;
    }
    let mut m = n;
    let mut e: u32 = 0;
    while m % 10 == 0 {
        m /= 10;
        e += 1;
    }
    if e == 0 {
        return None;
    }
    Some(format!("{}E{}", m, e))
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

/// gap-043: pick the shorter delimiter for a string literal.
/// Upstream Closure (under WHITESPACE_ONLY) re-quotes string
/// literals to minimise the escape weight: when content has
/// more `"` than `'`, switch to single-quoted form (so the
/// `"` chars stay unescaped); otherwise keep double-quoted.
/// Ties go to double per upstream's `CodePrinter` (verified
/// by the JAR probe `'a'` → `"a"`).
///
/// This function emits the complete `"..."` or `'...'`
/// sequence (delimiters + content) to `out`, with all
/// content characters appropriately escaped for the chosen
/// quote style. Backslash/control-char escapes are
/// independent of quote choice.
///
/// Mirrors the logic in `closure-emitter`'s
/// `choose_quote_and_escape` (closed CLOC12 gap-026). The
/// AST emitter goes through that path; the CLI WHITESPACE_ONLY
/// path uses this independent copy because it doesn't build
/// an AST.
fn emit_quoted_string(out: &mut String, content: &str) {
    let dq = content.chars().filter(|c| *c == '"').count();
    let sq = content.chars().filter(|c| *c == '\'').count();
    if dq > sq {
        // Single-quote wins — escape `\` and `'`.
        out.push('\'');
        for c in content.chars() {
            match c {
                '\'' => out.push_str("\\'"),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                other => out.push(other),
            }
        }
        out.push('\'');
    } else {
        // Double-quote (default, tie-break).
        out.push('"');
        push_quoted_string_content(out, content);
        out.push('"');
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
    // gap-039: tagged template literal. The grammar
    // `TaggedTemplateExpression → MemberExpression TemplateLiteral`
    // (§13.3.11) requires zero whitespace between the tag
    // function and the template's opening `` ` ``. Without
    // this short-circuit, two adjacent word-like tokens
    // would get a separator inserted (e.g. `tag` IDENT
    // followed by template literal whose value starts with
    // `` ` ``), producing `tag \`hi\`` instead of upstream's
    // `tag\`hi\``. Filter BEFORE the word-like rule so a
    // template literal preceded by an IDENT / number /
    // keyword still emits no space.
    if b.value.starts_with('`') {
        return false;
    }
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
        // chain ends; per gap-041 the `;` is DEFERRED past
        // the outer `}` rather than emitted immediately.
        // After outer-try-body `}`: peek is `catch`, chain
        // continues; deferred state carried past.
        // After outer-catch `}`: chain ends; deferred +
        // own-`;` collapse to a single trailing `;`.
        // Matches upstream's
        // `try{try{a}catch(e){b}}catch(f){c};` (verified
        // via closure-compiler-v20240317.jar).
        assert_eq!(
            minify("try{try{a;}catch(e){b;}}catch(f){c;}"),
            "try{try{a}catch(e){b}}catch(f){c};"
        );
    }

    /// Critical regression test: gap-030's function-decl
    /// trailing `;` is preserved when function-decl appears
    /// inside a try-block. brace_stack handles Function and
    /// TryChain as separate cases — they don't interfere.
    #[test]
    fn gap033_function_decl_inside_try_block_still_gets_semi() {
        // Per gap-041's deferred-`;` mechanism, the inner
        // function-decl's `;` is deferred past the try-body's
        // `}`. The try-chain continues to `catch`, so the
        // deferred `;` is carried forward past the chain
        // boundary too. The final catch `}` (chain end)
        // collapses the deferred + chain-terminator into a
        // single trailing `;`. Matches upstream:
        // `try{function f(){}}catch(e){b};` (verified via
        // closure-compiler-v20240317.jar).
        assert_eq!(
            minify("try{function f(){}}catch(e){b;}"),
            "try{function f(){}}catch(e){b};"
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
        // `a();`. **gap-049 (CLOC12.56) further improves
        // this**: the inner flatten now peeks the next
        // token after its closing `}`. Since the outer
        // block's `}` follows, the trailing `;` is
        // suppressed. Output is `if(x){if(y)a()}` — one
        // byte shorter than before (was `if(x){if(y)a();}`).
        // Upstream Closure emits `if(x)if(y)a();` which is
        // also more aggressive than us (it flattens the
        // outer block too); matching that requires letting
        // the outer flatten see through `if` keywords,
        // which would be a separate future gap.
        assert_eq!(
            minify("if(x){if(y){a();}}"),
            "if(x){if(y)a()}"
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
        // decl's synthetic `;` is then DEFERRED past the
        // outer `}` per gap-041 — matching upstream's
        // `if(x){function f(){}};` (verified against
        // closure-compiler-v20240317.jar).
        assert_eq!(
            minify("if(x){function f(){}}"),
            "if(x){function f(){}};"
        );
    }

    /// Body containing `try` keyword MUST NOT flatten. The
    /// try/catch chain has its own state machine that the
    /// pre-emit pathway would corrupt.
    #[test]
    fn gap032_body_with_try_does_not_flatten() {
        // Outer if-body has `try` in the blocking-keyword
        // set, so gap-032 doesn't flatten (conservatively).
        // The try-chain's trailing `;` is deferred past the
        // outer `}` per gap-041. Note: upstream Closure
        // ACTUALLY flattens this further to
        // `if(x)try{a()}catch(e){b()};` (gap-032 is more
        // aggressive there) — that's a separate future gap
        // (the `has_blocking_keyword` set should not
        // include `try` once we trust the brace_stack
        // properly). For now this is the best we produce
        // without that further work.
        assert_eq!(
            minify("if(x){try{a();}catch(e){b();}}"),
            "if(x){try{a()}catch(e){b()}};"
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

    /// gap-048: BigInt literal with ES2021 `_` numeric
    /// separator strips the separators (lexical sugar)
    /// while leaving the BigInt body otherwise verbatim.
    #[test]
    fn gap048_bigint_decimal_separator_stripped() {
        assert_eq!(
            minify("var a=1_000_000n;"),
            "var a=1000000n;"
        );
    }

    /// gap-048: separator stripping also works for hex
    /// BigInt — the `0x` prefix and digits-pattern are
    /// preserved; only `_` is removed.
    #[test]
    fn gap048_bigint_hex_separator_stripped() {
        assert_eq!(
            minify("var a=0x1_FFFn;"),
            "var a=0x1FFFn;"
        );
    }

    /// **Non-regression**: BigInt WITHOUT separators is
    /// unchanged. The gap-048 branch returns early in
    /// that case (no `_` → no work).
    #[test]
    fn gap048_bigint_no_separator_unchanged() {
        assert_eq!(
            minify("var a=9007199254740993n;"),
            "var a=9007199254740993n;"
        );
    }

    /// **Non-regression**: hex BigInt without separators
    /// is unchanged (the radix-and-shortest-form
    /// canonicalization that regular `0xff` → `255`
    /// gets is still deferred for BigInt — that's
    /// gap-038's bigint future).
    #[test]
    fn gap048_bigint_hex_no_separator_unchanged() {
        assert_eq!(minify("var x=0xfn;"), "var x=0xfn;");
    }

    /// **Non-regression**: separator in regular number
    /// (non-BigInt) still goes through gap-040's normal
    /// shortest-form path. `1_000` → `1E3` (shortest).
    #[test]
    fn gap048_regular_separator_still_uses_gap040() {
        assert_eq!(minify("var a=1_000;"), "var a=1E3;");
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

    // ---- gap-040: numeric separator + scientific ---------

    /// Target fixture: `1_000_000` → `1E6`.
    /// Stripped to `1000000` (7 chars), scientific `1E6`
    /// (3 chars) → scientific wins.
    #[test]
    fn gap040_separator_scientific_million() {
        assert_eq!(minify("var x=1_000_000;"), "var x=1E6;");
    }

    /// `1_000` → `1E3`. Decimal `1000` (4), sci `1E3` (3).
    #[test]
    fn gap040_separator_scientific_thousand() {
        assert_eq!(minify("var x=1_000;"), "var x=1E3;");
    }

    /// `1_234_567` → `1234567`. No trailing zeros, no
    /// scientific candidate. Cleaned (7) = decimal (7),
    /// decimal wins on tie.
    #[test]
    fn gap040_separator_no_trailing_zeros() {
        assert_eq!(minify("var x=1_234_567;"), "var x=1234567;");
    }

    /// `0xff_ff` → `65535`. Cleaned hex (6), decimal (5),
    /// no sci → decimal.
    #[test]
    fn gap040_hex_with_separator() {
        assert_eq!(minify("var x=0xff_ff;"), "var x=65535;");
    }

    /// Bare decimal `1000` → `1E3`. Cleaned (4) = decimal
    /// (4), sci (3) → sci.
    #[test]
    fn gap040_bare_decimal_to_scientific() {
        assert_eq!(minify("var x=1000;"), "var x=1E3;");
    }

    /// Tie: `100` decimal (3) ties with `1E2` scientific (3).
    /// Decimal wins on tie.
    #[test]
    fn gap040_decimal_scientific_tie_picks_decimal() {
        assert_eq!(minify("var x=100;"), "var x=100;");
    }

    /// `10` → `10`. Decimal (2) < sci `1E1` (3).
    #[test]
    fn gap040_tiny_decimal_unchanged() {
        assert_eq!(minify("var x=10;"), "var x=10;");
    }

    /// `12000` → `12E3`. Mantissa not 1; verifies general
    /// scientific form. Cleaned (5), decimal (5), sci `12E3`
    /// (4) → sci wins.
    #[test]
    fn gap040_scientific_with_multi_digit_mantissa() {
        assert_eq!(minify("var x=12000;"), "var x=12E3;");
    }

    /// `1234500` (7 chars) ties with sci `12345E2` (7
    /// chars). Decimal wins on tie.
    #[test]
    fn gap040_decimal_sci_tie_picks_decimal() {
        assert_eq!(minify("var x=1234500;"), "var x=1234500;");
    }

    /// **Non-regression**: `0` is the trivial case.
    /// Scientific helper returns None for `n == 0`.
    #[test]
    fn gap040_zero_unchanged() {
        assert_eq!(minify("var x=0;"), "var x=0;");
    }

    /// **Non-regression**: decimal source without trailing
    /// zeros is unchanged.
    #[test]
    fn gap040_decimal_no_normalization() {
        assert_eq!(minify("var x=12345;"), "var x=12345;");
    }

    /// **Non-regression**: floating-point literals are NOT
    /// touched — they hit the early `return value.to_string()`
    /// branch because they contain `.` (or `e`/`E`).
    #[test]
    fn gap040_float_left_alone() {
        assert_eq!(minify("var x=1.5;"), "var x=1.5;");
    }

    // ---- gap-042: do-keyword arms body_position_next -----

    /// Target fixture: `do{a;}while(x);` flattens via gap-032.
    #[test]
    fn gap042_do_while_single_stmt_flattens() {
        assert_eq!(
            minify("do{a;}while(x);"),
            "do a;while(x);"
        );
    }

    /// Multi-statement do-body does NOT flatten (gap-032's
    /// eligibility check requires exactly 1 `;` at depth 0).
    #[test]
    fn gap042_do_while_multi_stmt_does_not_flatten() {
        assert_eq!(
            minify("do{a();b();}while(x);"),
            "do{a();b()}while(x);"
        );
    }

    // ---- gap-043: CLI quote-choice optimisation ----------

    /// **No quotes in content**: defaults to double quotes
    /// (the existing behaviour). Tie-break to double per
    /// upstream's `CodePrinter`.
    #[test]
    fn gap043_no_quotes_in_content_picks_double() {
        assert_eq!(minify("var x=\"hello\";"), "var x=\"hello\";");
    }

    /// **Content has single quotes**: stick with double
    /// (no escape savings from switching).
    #[test]
    fn gap043_single_quotes_only_stay_double() {
        assert_eq!(
            minify("var x=\"a'b'c\";"),
            "var x=\"a'b'c\";"
        );
    }

    /// **Content has more double than single quotes**:
    /// switch to single-quoted form to avoid `\"` escapes.
    /// This is the target fixture's case in compact form.
    #[test]
    fn gap043_more_double_switches_to_single() {
        // Source contains escaped `"` (one) and no `'`.
        // After switching to single quotes, the `"` no
        // longer needs escaping.
        assert_eq!(
            minify(r#"var x="\"";"#),
            r#"var x='"';"#
        );
    }

    /// **Tie**: both `"` and `'` appear once. Double wins.
    #[test]
    fn gap043_tie_picks_double() {
        assert_eq!(
            minify(r#"var x="\"'";"#),
            r#"var x="\"'";"#
        );
    }

    // ---- gap-045: single-arg arrow drops parens ----------

    /// Target fixture in compact form: `(x)=>x+1` →
    /// `x=>x+1`.
    #[test]
    fn gap045_single_arg_arrow_drops_parens() {
        assert_eq!(
            minify("var f=(x)=>x+1;"),
            "var f=x=>x+1;"
        );
    }

    /// Composes with `async` keyword: `async(x)=>x+1` →
    /// `async x=>x+1`. The separator between `async` and
    /// the now-bare IDENT is inserted by `needs_separator`
    /// (KEYWORD + NAME → space).
    #[test]
    fn gap045_async_single_arg_arrow() {
        assert_eq!(
            minify("var f=async(x)=>x+1;"),
            "var f=async x=>x+1;"
        );
    }

    /// **Non-regression**: zero-arg arrow `()=>x` stays as
    /// `()=>x`. The `(` is followed by `)` not IDENT, so
    /// the gap-045 pattern doesn't match.
    #[test]
    fn gap045_zero_arg_arrow_unchanged() {
        assert_eq!(
            minify("var f=()=>1;"),
            "var f=()=>1;"
        );
    }

    /// **Non-regression**: multi-arg arrow `(x,y)=>x+y`
    /// stays parenthesised. The token at idx+2 is `,` not
    /// `)`, so the pattern doesn't match.
    #[test]
    fn gap045_multi_arg_arrow_unchanged() {
        assert_eq!(
            minify("var f=(x,y)=>x+y;"),
            "var f=(x,y)=>x+y;"
        );
    }

    /// **Non-regression**: default-value arrow `(x=1)=>x`
    /// stays parenthesised. The token at idx+2 is `=` not
    /// `)`.
    #[test]
    fn gap045_default_arg_arrow_unchanged() {
        assert_eq!(
            minify("var f=(x=1)=>x;"),
            "var f=(x=1)=>x;"
        );
    }

    /// **Non-regression**: rest-arg arrow `(...args)=>args`
    /// stays parenthesised. The token at idx+1 is `...`
    /// (PUNCT, not Name) — `is_simple_identifier_token`
    /// returns false.
    #[test]
    fn gap045_rest_arg_arrow_unchanged() {
        assert_eq!(
            minify("var f=(...args)=>args;"),
            "var f=(...args)=>args;"
        );
    }

    /// **Non-regression**: destructuring-arg arrow
    /// `({x})=>x` stays parenthesised. The token at idx+1
    /// is `{` (PUNCT, not Name).
    #[test]
    fn gap045_destruct_arg_arrow_unchanged() {
        assert_eq!(
            minify("var f=({x})=>x;"),
            "var f=({x})=>x;"
        );
    }

    /// **Non-regression**: `(x)` followed by `.` (member
    /// access on a parenthesised expression) — NOT an
    /// arrow. The token at idx+3 is `.` not `=>`.
    #[test]
    fn gap045_parens_not_arrow_unchanged() {
        assert_eq!(
            minify("var x=(a).b;"),
            "var x=(a).b;"
        );
    }

    // ---- gap-046: trailing array comma drop --------------

    /// Target fixture: `[1,2,]` → `[1,2]`.
    #[test]
    fn gap046_trailing_array_comma_dropped() {
        assert_eq!(minify("var a=[1,2,];"), "var a=[1,2];");
    }

    /// Single-element with trailing: `[1,]` → `[1]`.
    #[test]
    fn gap046_single_element_with_trailing() {
        assert_eq!(minify("var a=[1,];"), "var a=[1];");
    }

    /// **Non-regression**: inner commas not affected.
    /// `[1,2,3]` round-trips identically.
    #[test]
    fn gap046_inner_commas_preserved() {
        assert_eq!(minify("var a=[1,2,3];"), "var a=[1,2,3];");
    }

    /// **Elision-with-trailing** ambiguity: `[1,,]` —
    /// elision `[1,,]` is `[1, undefined]` (length 2).
    /// Hmm — actually the second `,` IS trailing. After
    /// our rule drops it: `[1,]`. But `[1,]` is length 1,
    /// not 2. So our rule is technically WRONG for this
    /// case. However: upstream Closure under
    /// WHITESPACE_ONLY produces `[1,]` for the source
    /// `[1,,]` too (verified via JAR probe — they accept
    /// this lossy normalisation for whitespace_only).
    /// Confirmed via probe `[1,,];` → `[1,];`.
    #[test]
    fn gap046_elision_with_trailing_normalised() {
        assert_eq!(minify("var a=[1,,];"), "var a=[1,];");
    }

    /// **Non-regression**: function-call trailing comma
    /// `f(1,2,)` is ES2017 — also a trailing comma but in
    /// a call expression, not an array literal. The rule
    /// here only fires when next is `]`. For `f(1,2,)`,
    /// next is `)`. So our rule doesn't touch it. (Whether
    /// upstream normalises this is a separate gap.)
    #[test]
    fn gap046_call_trailing_comma_unchanged() {
        assert_eq!(minify("f(1,2,);"), "f(1,2,);");
    }

    /// **Non-regression**: empty array `[]` works.
    /// No `,` to suppress.
    #[test]
    fn gap046_empty_array_unchanged() {
        assert_eq!(minify("var a=[];"), "var a=[];");
    }

    // ---- gap-046b: trailing object comma drop ------------

    /// Target case: object literal with trailing `,`.
    #[test]
    fn gap046b_obj_literal_trailing_comma_dropped() {
        assert_eq!(
            minify("var o={a:1,b:2,};"),
            "var o={a:1,b:2};"
        );
    }

    /// Object destructuring with trailing `,` — same shape.
    #[test]
    fn gap046b_obj_destruct_trailing_comma_dropped() {
        assert_eq!(
            minify("var {a,b,}=o;"),
            "var {a,b}=o;"
        );
    }

    /// Single-property object literal with trailing `,`.
    #[test]
    fn gap046b_obj_literal_single_with_comma() {
        assert_eq!(minify("var o={a:1,};"), "var o={a:1};");
    }

    /// **Non-regression**: object WITHOUT trailing `,`
    /// stays verbatim.
    #[test]
    fn gap046b_obj_literal_unchanged() {
        assert_eq!(minify("var o={a:1};"), "var o={a:1};");
    }

    /// **Non-regression**: empty object `{}` is unchanged.
    #[test]
    fn gap046b_empty_obj_unchanged() {
        assert_eq!(minify("var o={};"), "var o={};");
    }

    /// **Non-regression**: function-call trailing comma
    /// `f(1,2,)` is `,` before `)`, NOT before `}`. The
    /// peephole only fires on `,` before `}`, so calls
    /// are unaffected.
    #[test]
    fn gap046b_call_trailing_comma_unchanged() {
        assert_eq!(minify("f(1,2,);"), "f(1,2,);");
    }

    /// **Non-regression**: nested object `,` only drops
    /// the immediate trailing comma; inner commas between
    /// values are preserved.
    #[test]
    fn gap046b_nested_obj_inner_commas_preserved() {
        assert_eq!(
            minify("var o={a:{b:1,c:2,},d:3,};"),
            "var o={a:{b:1,c:2},d:3};"
        );
    }

    // ---- gap-051: IIFE paren normalisation ---------------

    /// Target case: bare IIFE in inner-call form.
    #[test]
    fn gap051_iife_inner_call_to_outer() {
        assert_eq!(
            minify("(function(){return 42;}());"),
            "(function(){return 42})();"
        );
    }

    /// IIFE assigned to a variable.
    #[test]
    fn gap051_iife_in_assignment() {
        assert_eq!(
            minify("var x=(function(){return 1;}());"),
            "var x=(function(){return 1})();"
        );
    }

    /// **Non-regression**: IIFE already in outer-call form
    /// stays as-is. `(fn)()` doesn't match the rewrite
    /// pattern (`} ( ) )`); the `)` after the function
    /// body comes BEFORE the call's `()`.
    #[test]
    fn gap051_outer_call_iife_unchanged() {
        assert_eq!(
            minify("(function(){return 42;})();"),
            "(function(){return 42})();"
        );
    }

    /// **Non-regression**: function call NOT inside an
    /// outer paren stays as-is.
    #[test]
    fn gap051_plain_call_unchanged() {
        // gap-030 still emits `;` after function-decl `}` because
        // gap-047's suppression set doesn't include identifiers.
        // The point of THIS test is just that gap-051 doesn't
        // misfire on non-IIFE patterns.
        assert_eq!(
            minify("function f(){return 1;}f();"),
            "function f(){return 1};f();"
        );
    }

    /// **Non-regression**: empty arrow body call inside
    /// parens — should NOT rewrite because the pattern
    /// requires `} ( ) )`. Arrow has different shape.
    #[test]
    fn gap051_arrow_iife_pattern() {
        // `(()=>1)()` — already outer-call form, unchanged.
        assert_eq!(
            minify("var x=(()=>1)();"),
            "var x=(()=>1)();"
        );
    }

    /// **Non-regression**: IIFE with args.
    #[test]
    fn gap051_iife_with_args_in_call() {
        // `(function(a){return a;}(1));` → `(function(a){return a})(1);`
        assert_eq!(
            minify("(function(a){return a;}(1));"),
            "(function(a){return a}(1));"
        );
        // The pattern `} ( a ) )` doesn't match `} ( ) )`
        // — there's an arg inside the call parens. So we
        // don't rewrite. Upstream may or may not rewrite
        // with-args IIFEs; leaving that as a future
        // refinement.
    }

    // ---- gap-049: flattened for-body `;` suppression ----

    /// Target case: for-of body flatten next to outer `}`.
    #[test]
    fn gap049_for_of_flat_drops_trailing_semi() {
        assert_eq!(
            minify("function f(){for(var v of a){a;}}"),
            "function f(){for(var v of a)a};"
        );
    }

    /// for-in body flatten — same family as for-of.
    #[test]
    fn gap049_for_in_flat_drops_trailing_semi() {
        assert_eq!(
            minify("function f(){for(var k in o){use(k);}}"),
            "function f(){for(var k in o)use(k)};"
        );
    }

    /// while-body flatten next to outer `}`.
    #[test]
    fn gap049_while_flat_drops_trailing_semi() {
        assert_eq!(
            minify("function f(){while(x){a();}}"),
            "function f(){while(x)a()};"
        );
    }

    /// **Non-regression**: flattened body NOT next to a
    /// `}` keeps its `;`. `for(...) a();` at top level
    /// still ends with `;` because next is EOF (gap-049
    /// requires next-after-close to be specifically `}`).
    #[test]
    fn gap049_for_at_top_level_keeps_semi() {
        assert_eq!(
            minify("for(var v of a){a;}"),
            "for(var v of a)a;"
        );
    }

    /// **Non-regression**: if-body flattened at top level
    /// keeps its `;` for the same reason.
    #[test]
    fn gap049_if_at_top_level_keeps_semi() {
        assert_eq!(
            minify("if(x){a();}"),
            "if(x)a();"
        );
    }

    /// **Non-regression**: if-else where the else-arm is
    /// flattened next to outer `}`. The `;` of the else-arm
    /// should still be suppressed.
    #[test]
    fn gap049_if_else_inside_function_drops_semi() {
        assert_eq!(
            minify("function f(){if(x){a();}else{b();}}"),
            "function f(){if(x)a();else b()};"
        );
    }

    // ---- gap-050: `new X()` → `new X` empty-paren drop --

    /// Target case: bare `new Foo()` at top level.
    #[test]
    fn gap050_new_call_drops_empty_parens() {
        assert_eq!(
            minify("var x=new Foo();"),
            "var x=new Foo;"
        );
    }

    /// `new Foo()` inside a function body still drops.
    #[test]
    fn gap050_new_call_in_function_body_drops() {
        assert_eq!(
            minify("function f(){return new Bar();}"),
            "function f(){return new Bar};"
        );
    }

    /// **Non-regression**: `new Foo(a)` (NON-empty arg
    /// list) keeps parens — they're not redundant.
    #[test]
    fn gap050_new_call_with_args_unchanged() {
        assert_eq!(
            minify("var x=new Foo(a);"),
            "var x=new Foo(a);"
        );
    }

    /// **Non-regression**: `new Foo().bar` — dropping the
    /// `()` would change parse to `new Foo.bar` (different
    /// constructor). Must keep parens.
    #[test]
    fn gap050_new_call_then_member_keeps_parens() {
        assert_eq!(
            minify("var x=new Foo().bar;"),
            "var x=new Foo().bar;"
        );
    }

    /// **Non-regression**: `new Foo()[x]` — same family;
    /// element-access on result vs constructor of `Foo[x]`.
    #[test]
    fn gap050_new_call_then_bracket_keeps_parens() {
        assert_eq!(
            minify("var x=new Foo()[0];"),
            "var x=new Foo()[0];"
        );
    }

    /// **Non-regression**: `new Foo()()` — chained call on
    /// result. Dropping would lose the chained call.
    #[test]
    fn gap050_new_call_then_call_keeps_parens() {
        assert_eq!(
            minify("var x=new Foo()();"),
            "var x=new Foo()();"
        );
    }

    /// **Non-regression**: `new Foo() + 1` — `+` binds
    /// looser than NewExpression, so dropping is safe.
    /// Verifies the peephole DOES drop here.
    #[test]
    fn gap050_new_call_then_plus_drops() {
        assert_eq!(
            minify("var x=new Foo()+1;"),
            "var x=new Foo+1;"
        );
    }

    /// **Non-regression**: `new` keyword followed by a
    /// non-identifier (e.g. `new (expr)`) is NOT
    /// targeted by this peephole — kept[idx-1] isn't a
    /// simple identifier.
    #[test]
    fn gap050_new_with_paren_expr_unchanged() {
        // `new (Foo||Bar)()` — paren expression for
        // constructor selection. The empty `()` after the
        // paren-close is NOT what we target (idx-1 is `)`,
        // not an identifier).
        assert_eq!(
            minify("var x=new (Foo||Bar)();"),
            "var x=new(Foo||Bar)();"
        );
    }

    // ---- gap-047: suppress `;` before stmt-keyword -------

    /// Target case: function decl followed by `var`.
    #[test]
    fn gap047_function_then_var_no_semi() {
        assert_eq!(
            minify("function f(){a;}var x=1;"),
            "function f(){a}var x=1;"
        );
    }

    /// Class decl followed by `function`: the class's
    /// trailing `;` (gap-034) is suppressed. But the
    /// outermost `function g(){}` at EOF still gets its own
    /// `;` from gap-030's EOF case (no suppression).
    #[test]
    fn gap047_class_then_function_no_semi() {
        assert_eq!(
            minify("class C{}function g(){}"),
            "class C{}function g(){};"
        );
    }

    /// **EOF (None) → keep `;`**. The single-statement
    /// fixture `function add(a,b){return a+b;}` was passing
    /// at the harness level with output
    /// `function add(a,b){return a+b};` and that behaviour
    /// must not change. EOF doesn't match any keyword and
    /// is NOT in the suppression set.
    #[test]
    fn gap047_function_at_eof_keeps_semi() {
        assert_eq!(
            minify("function add(a,b){return a+b;}"),
            "function add(a,b){return a+b};"
        );
    }

    /// **`}` next → still defer (gap-041)**. The gap-047
    /// keyword check is separate from the close-brace defer.
    /// `function f(){function g(){}}` should still hoist
    /// the single `;` to the outermost `}`.
    #[test]
    fn gap047_close_brace_next_still_defers() {
        assert_eq!(
            minify("function f(){function g(){}}"),
            "function f(){function g(){}};"
        );
    }

    /// **Non-regression**: an Other-block close followed by
    /// `var` doesn't ever emit a `;` regardless of gap-047
    /// (because `kind_wants_semi == false`). This is the
    /// natural shape `if(x){...}var y;` which works
    /// pre-gap-047 too. Verifies the change doesn't regress
    /// it.
    #[test]
    fn gap047_other_block_then_var_unchanged() {
        // Multi-statement if-body so gap-032 doesn't flatten.
        assert_eq!(
            minify("if(x){a();b();}var y=1;"),
            "if(x){a();b()}var y=1;"
        );
    }

    /// **Non-regression**: `return` keyword in suppression
    /// list. Inner function-decl's trailing `;` (would be
    /// gap-030) is suppressed because next is `return`.
    /// The outer function's `;` at EOF still fires.
    #[test]
    fn gap047_function_then_return_no_semi() {
        assert_eq!(
            minify("function f(){function g(){}return 1;}"),
            "function f(){function g(){}return 1};"
        );
    }

    /// **Non-regression**: catch/finally are NOT in the
    /// gap-047 suppression set — they're handled via
    /// `next_is_chain_continuation` (a separate branch).
    /// Test verifies the TryChain rule still fires
    /// correctly.
    #[test]
    fn gap047_trychain_continuation_unaffected() {
        assert_eq!(
            minify("try{a;}catch(e){b;}"),
            "try{a}catch(e){b};"
        );
    }

    // Note on `do{}while(x);` (empty do-body): gap-031's
    // empty-body collapse fires, producing `do;while(x);`
    // in principle. But the synthetic `;` emission doesn't
    // update `prev_emitted_tok`, so `needs_separator`
    // computes word-like(do, while) → true → inserts a
    // spurious space, producing `do; while(x);`. This is a
    // SEPARATE latent issue (will be filed as a future
    // gap) and orthogonal to the do-keyword arming gap-042
    // is closing. A test for it would belong with the fix
    // for the prev_emitted_tok update.

    // ---- gap-039: tagged template separator --------------

    /// Target fixture: tagged template literal has no
    /// separator between the tag function and the
    /// `` ` ``-opening template.
    #[test]
    fn gap039_tagged_template_no_separator() {
        assert_eq!(minify("var x=tag`hi`;"), "var x=tag`hi`;");
    }

    /// Tagged-template chains: `tag``hi`.length` (member
    /// access after a tagged template) still works.
    #[test]
    fn gap039_member_after_tagged_template() {
        assert_eq!(
            minify("var x=tag`hi`.length;"),
            "var x=tag`hi`.length;"
        );
    }

    /// Bare template literal (not tagged) is unaffected —
    /// the rule fires on next-starts-with-backtick whether
    /// or not a tag IDENT precedes it. The previous token
    /// before a bare `` ` `` is typically `=` (PUNCTUATION,
    /// not word-like), so no separator would be needed
    /// anyway.
    #[test]
    fn gap039_bare_template_literal() {
        assert_eq!(minify("var x=`hi`;"), "var x=`hi`;");
    }

    /// **Composition with gap-035**: `var{a}=tag`hi`;` would
    /// trigger both the `var{` separator (gap-035) and the
    /// tagged-template no-separator (gap-039). gap-035 wins
    /// inside `var`-to-`{`; gap-039 wins inside IDENT-to-`` ` ``.
    /// They don't conflict because they fire on different
    /// token-pair shapes.
    #[test]
    fn gap039_composes_with_gap035() {
        assert_eq!(
            minify("var{a}=tag`hi`;"),
            "var {a}=tag`hi`;"
        );
    }
}
