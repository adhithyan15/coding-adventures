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
    // gap-061 needs to INSERT a `(`/`)` pair into the token
    // stream (the arg-bearing new-expr wrap can't reuse the
    // empty-args reorder trick). We clone one real `(` and one
    // real `)` from the source as synthetic grouping parens —
    // declared BEFORE `kept` so they outlive it, letting `kept`
    // (a `Vec<&Token>`) hold references to them. Only `.value`
    // matters for re-stitching, so the cloned line/column are
    // irrelevant. If the source has no parens, gap-061 can't
    // fire anyway, so `None` is fine.
    let synth_open: Option<lexer::token::Token> = tokens
        .iter()
        .find(|t| is_structural_punct(t, "("))
        .cloned();
    let synth_close: Option<lexer::token::Token> = tokens
        .iter()
        .find(|t| is_structural_punct(t, ")"))
        .cloned();

    // gap-067 needs a synthetic `;` to terminate a flattened
    // labeled body that had no trailing `;` of its own (the block
    // braces it dropped were doing the termination). Cloned from
    // any source token (the stream always has ≥1, the EOF
    // sentinel) and re-typed to a Semicolon — only `.value`
    // matters for re-stitching. Declared before `kept` so it
    // outlives the `&Token` reference inserted into it.
    let synth_semi: Option<lexer::token::Token> = tokens.first().map(|t| {
        let mut s = t.clone();
        s.value = ";".to_string();
        s.type_ = lexer::token::TokenType::Semicolon;
        s.type_name = None;
        s
    });

    // gap-093 needs a synthetic `(`/`)` pair to WRAP a numeric
    // literal that is the object of a member access — `1 .x` and
    // `1..toString()` must become `(1).x` / `(1).toString()`,
    // because a bare `1.x` is invalid JS (the `.` binds as the
    // number's decimal point, leaving a dangling property). Unlike
    // gap-061's `synth_open`/`synth_close`, the source here often
    // has NO parens at all (`1 .x`), so we can't clone one from the
    // stream — we clone any token (the first) and re-type it, the
    // same trick `synth_semi` uses. Only `.value`/`.type_` matter
    // for re-stitching; the cloned line/column are irrelevant.
    let synth_num_open: Option<lexer::token::Token> = tokens.first().map(|t| {
        let mut s = t.clone();
        s.value = "(".to_string();
        s.type_ = lexer::token::TokenType::LParen;
        s.type_name = None;
        s
    });
    let synth_num_close: Option<lexer::token::Token> = tokens.first().map(|t| {
        let mut s = t.clone();
        s.value = ")".to_string();
        s.type_ = lexer::token::TokenType::RParen;
        s.type_name = None;
        s
    });

    let mut kept: Vec<_> = tokens
        .iter()
        .filter(|t| !is_trivia(t))
        .filter(|t| !is_eof(t))
        .collect();

    // gap-093 + gap-098: NUMBER-followed-by-DOT normalisation. The
    // lexer splits a number's trailing dot off into its own DOT token
    // (`1 .x` and `5.` both become NUMBER + DOT + …), so the dot's
    // meaning has to be recovered from what FOLLOWS it:
    //
    // gap-093 — the dot IS a member access (a property name follows).
    // Upstream parenthesises the number so the dot can't be misread as
    // the number's decimal point:
    //   `1 .x`           -> `(1).x`
    //   `1.5.toString()` -> `(1.5).toString()`
    //   `1..toString()`  -> `(1).toString()`   (NB: ONE dot survives)
    //
    // gap-098 — the dot is NOT a member access (the follower is `;`, an
    // operator, `)`, `,`, EOF — anything that can't be a property name).
    // Then the dot is a redundant trailing decimal point and is dropped:
    //   `5.`             -> `5`
    //   `5.+1`           -> `5+1`
    //
    // but an INDEX access or anything else is left alone:
    //   `1[0]`           -> `1[0]`   (next token is `[`, not `.`)
    //   `(1).x`          -> `(1).x`  (already parenthesised; the
    //                                 number's follower is `)`)
    //
    // WHY a bare `1.x` is wrong: the lexer splits `1 .x` into
    // NUMBER(`1`) DOT NAME(`x`); re-stitched verbatim that prints
    // `1.x`, which a JS parser reads as the float `1.` followed by
    // a stray `x` — a SyntaxError. The parens force `1` to be a
    // complete primary expression, after which `.x` is plain member
    // access. This is a CORRECTNESS fix, not just byte-parity.
    //
    // THE DOUBLE-DOT CASE: `1..toString()` lexes as
    // NUMBER(`1`) DOT DOT NAME(`toString`). The FIRST dot is the
    // (split-off) decimal point that made `1.` a float; the SECOND
    // is the member operator. Once we parenthesise the `1`, the
    // decimal-point dot is redundant, so we keep exactly ONE dot.
    //
    // We rebuild `kept` in a single forward pass, emitting
    // `( <number> )` in place of `<number>` whenever its immediate
    // follower is a structural `.` AND the token after the dot(s)
    // is a property name (word-like). The synthetic parens are the
    // `synth_num_open`/`synth_num_close` declared above.
    if let (Some(np_open), Some(np_close)) =
        (synth_num_open.as_ref(), synth_num_close.as_ref())
    {
        let mut wrapped: Vec<&lexer::token::Token> = Vec::with_capacity(kept.len() + 8);
        let mut i = 0;
        while i < kept.len() {
            let tok = kept[i];
            // A NUMBER (or BigInt) whose next token is a member `.`.
            let followed_by_dot =
                is_number_literal(tok) && kept.get(i + 1).is_some_and(|t| is_structural_punct(t, "."));
            if followed_by_dot {
                // One dot or two? Two means the first is the
                // decimal-point dot that the lexer split off.
                let double_dot =
                    kept.get(i + 2).is_some_and(|t| is_structural_punct(t, "."));
                // The property name sits after the dot(s).
                let name_idx = if double_dot { i + 3 } else { i + 2 };
                let is_member = kept.get(name_idx).is_some_and(|t| is_word_like(t));
                if is_member {
                    // gap-093: emit `( <number> )`, then resume so the
                    // member dot is re-emitted by the normal path.
                    wrapped.push(np_open);
                    wrapped.push(tok);
                    wrapped.push(np_close);
                    // Skip the number; for the double-dot form also
                    // skip the redundant decimal-point dot so only
                    // the member dot survives.
                    i += if double_dot { 2 } else { 1 };
                    continue;
                } else if !double_dot {
                    // gap-098: TRAILING BARE DECIMAL POINT. A single `.`
                    // after a NUMBER whose own follower is NOT a property
                    // name (it's `;`, an operator, `)`, `,`, EOF, …) is a
                    // redundant trailing decimal point — the lexer split
                    // `5.` into NUMBER `5` + DOT `.`. Upstream drops it:
                    //   `5.`    -> `5`      `5.+1`  -> `5+1`
                    //   `50.`   -> `50`     `b=5.`  -> `b=5`
                    // This is the exact complement of gap-093's member
                    // case above: there the dot IS member access (post-dot
                    // token word-like) and the number gets parenthesised;
                    // here the dot canNOT be member access (a member needs
                    // a name after it), so it is pure decimal-point cruft
                    // and is simply removed. A genuine float like `5.5` is
                    // a single NUMBER token (no separate DOT) and never
                    // reaches here. Emit the number, skip the dot.
                    wrapped.push(tok);
                    i += 2; // past the number and the redundant dot
                    continue;
                }
            }
            wrapped.push(tok);
            i += 1;
        }
        kept = wrapped;
    }

    // gap-088: EMPTY-STATEMENT elimination. Upstream Closure drops a
    // `;` that is an EmptyStatement — a `;` in statement-list position
    // with no statement before it:
    //   `;;var x=1;`            -> `var x=1;`   (leading)
    //   `var x=1;;;`            -> `var x=1;`   (trailing — the first
    //                                          `;` is the real
    //                                          terminator; the rest are
    //                                          empty statements)
    //   `var a=1;;var b=2;`     -> `var a=1;var b=2;`  (between)
    //   `;;;`                   -> ``           (all empty)
    //   `function f(){;;x();}`  -> `function f(){x()}`
    //
    // A `;` is an empty statement (droppable) exactly when the token
    // BEFORE it is a `{` (start of a block / program body), another
    // `;` (a preceding empty statement or terminator), OR nothing (the
    // `;` is the very first token). In every other position the `;`
    // either terminates a real statement (`a;` — preceded by a value)
    // or is the BODY of a control-flow header (`while(a);`, `if(a);`,
    // `for(;;);`, `do;while(a)` — preceded by `)`/`do`/`else`), and
    // MUST be kept.
    //
    // The ONE hazard is the `for( … )` header, whose `;` SEPARATORS are
    // not statements: in `for(;;)` the second `;` IS preceded by the
    // first `;` and would otherwise look droppable. So we track a
    // bracket stack and refuse to drop a `;` whose innermost enclosing
    // bracket is a `for(` paren. (`for`'s `(` is detected by the
    // preceding `for` keyword, excluding a `.for(`/`?.for(` property
    // call.) Every other `;` inside a `for(...)` header is preceded by
    // a value and already non-droppable.
    {
        // Stack of open brackets; the bool is `true` only for a
        // `for( … )` header paren.
        let mut stack: Vec<bool> = Vec::new();
        let mut drops: Vec<usize> = Vec::new();
        for i in 0..kept.len() {
            let t = kept[i];
            if is_structural_punct(t, ";") {
                let droppable_pos = i == 0
                    || is_structural_punct(kept[i - 1], "{")
                    || is_structural_punct(kept[i - 1], ";");
                let in_for_header = matches!(stack.last(), Some(true));
                if droppable_pos && !in_for_header {
                    drops.push(i);
                }
            } else if is_structural_punct(t, "(") {
                // for-header `(` iff directly preceded by the `for`
                // keyword (and that `for` is not a `.for` property).
                let is_for = i > 0
                    && is_word_like(kept[i - 1])
                    && kept[i - 1].value == "for"
                    && !(i >= 2
                        && (is_structural_punct(kept[i - 2], ".")
                            || is_structural_punct(kept[i - 2], "?.")));
                stack.push(is_for);
            } else if is_structural_punct(t, "[") || is_structural_punct(t, "{") {
                stack.push(false);
            } else if is_structural_punct(t, ")")
                || is_structural_punct(t, "]")
                || is_structural_punct(t, "}")
            {
                stack.pop();
            }
        }
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // gap-089: empty `new` call-paren drop for a MEMBER-expression
    // callee — `new a.b()` → `new a.b`, `new a.b.c()` → `new a.b.c`,
    // `new a[x]()` → `new a[x]`. gap-050 (in the emit loop) already
    // drops the empty `()` of a `new` with a BARE-IDENTIFIER callee
    // (`new A()` → `new A`); this pre-pass extends that to a callee
    // that is a member expression (a `.IDENT` / `[ … ]` chain rooted
    // at the identifier after `new`).
    //
    // We scan FORWARD: locate a `new` keyword, parse its MemberCallee
    // (the base identifier followed by zero-or-more `.IDENT` or
    // balanced `[ … ]` accessors), and if the callee is *immediately*
    // followed by an empty `( )`, drop that pair — UNLESS the token
    // after `)` would syntactically re-bind the result (the SAME
    // safety gate as gap-050): a following `(`, `.`, `[`, or a
    // template `` ` `` makes `new a.b()` ≠ `new a.b` (call / member /
    // index / tagged-template all bind tighter than the
    // NewExpression). Those blocked-follower cases are left untouched
    // and are handled by the existing new-expr member-wrap pass
    // (`new a.b().c` → `(new a.b).c`). Every other follower (`;`, `,`,
    // `}`, EOF, an infix operator, …) binds looser and is safe.
    //
    // The callee MUST contain at least one accessor (a `.`/`[`) — a
    // bare `new IDENT()` is left to gap-050, so the two passes never
    // both fire on the same `()`.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i < kept.len() {
            // Anchor on a `new` keyword token.
            if !(is_word_like(kept[i]) && kept[i].value == "new") {
                i += 1;
                continue;
            }
            // Parse the callee: base identifier ...
            let base = i + 1;
            if !is_simple_identifier_token(kept.get(base).copied()) {
                i += 1;
                continue;
            }
            // ... then a chain of `.IDENT` or balanced `[ … ]`.
            let mut j = base + 1;
            let mut saw_accessor = false;
            loop {
                if j < kept.len() && is_structural_punct(kept[j], ".") {
                    // `.IDENT`
                    if is_simple_identifier_token(kept.get(j + 1).copied()) {
                        saw_accessor = true;
                        j += 2;
                        continue;
                    }
                    break;
                } else if j < kept.len() && is_structural_punct(kept[j], "[") {
                    // Balanced `[ … ]` (structural depth scan).
                    let mut depth: i32 = 1;
                    let mut k = j + 1;
                    let mut close = None;
                    while k < kept.len() {
                        let t = kept[k];
                        if is_structural_punct(t, "[")
                            || is_structural_punct(t, "(")
                            || is_structural_punct(t, "{")
                        {
                            depth += 1;
                        } else if is_structural_punct(t, "]") {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(k);
                                break;
                            }
                        } else if is_structural_punct(t, ")")
                            || is_structural_punct(t, "}")
                        {
                            depth -= 1;
                        }
                        k += 1;
                    }
                    match close {
                        Some(c) => {
                            saw_accessor = true;
                            j = c + 1;
                        }
                        None => break, // unbalanced — give up
                    }
                } else {
                    break;
                }
            }
            // Require at least one accessor (bare `new IDENT()` is
            // gap-050's job) and an immediately-following empty `( )`.
            let empty_call = saw_accessor
                && j + 1 < kept.len()
                && is_structural_punct(kept[j], "(")
                && is_structural_punct(kept[j + 1], ")");
            if empty_call {
                // Follower gate (identical to gap-050).
                let blocks = kept.get(j + 2).is_some_and(|t| {
                    is_structural_punct(t, "(")
                        || is_structural_punct(t, ".")
                        || is_structural_punct(t, "[")
                        || t.value.starts_with('`')
                });
                if !blocks {
                    drops.push(j);
                    drops.push(j + 1);
                    i = j + 2;
                    continue;
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

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
    // gap-053 + gap-084: paren elision around var-init RHS, to a
    // FIXPOINT.
    // Pattern: `= ( ... ) ;` or `= ( ... ) ,` where the
    // contents inside `(...)` have no top-level `,` (would
    // be a comma operator that would change to multiple
    // declarators if parens dropped) and don't start with
    // `function` (could be IIFE — keeping conservative).
    // Drop both the `(` and the matching `)`.
    //
    // gap-084 (the FIXPOINT): one pass only strips the OUTERMOST
    // layer of a `=(…)` RHS — `var x=((a));` becomes `var x=(a);`
    // and the exposed `(a)` is never revisited (the inner loop
    // already advanced past `close_idx`). Upstream fully strips
    // `((a))` → `a`. Running this elision repeatedly until a whole
    // pass drops nothing peels every redundant layer:
    //   ((a))   -> (a)   -> a
    //   (((a))) -> ((a)) -> (a) -> a
    //   ((a+b)) -> (a+b) -> a+b   (each layer is the whole RHS)
    // while the top-level-comma guard still halts at the meaningful
    // layer:  ((a,b)) -> (a,b)  (the inner `(a,b)` is a comma
    // operator — its single paren layer is load-bearing and stays).
    // Each iteration removes ≥2 tokens or makes no change and
    // breaks, so the loop always terminates.
    loop {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 2 < kept.len() {
            if kept[i].value == "=" && kept[i + 1].value == "(" {
                let open_idx = i + 1;
                let mut depth: i32 = 1;
                let mut has_top_level_comma = false;
                let mut starts_with_function =
                    kept.get(open_idx + 1).map(|t| t.value.as_str()) == Some("function");
                let _ = &mut starts_with_function;
                let mut close_idx: Option<usize> = None;
                let mut j = open_idx + 1;
                while j < kept.len() {
                    match kept[j].value.as_str() {
                        "(" | "[" | "{" => depth += 1,
                        ")" => {
                            depth -= 1;
                            if depth == 0 {
                                close_idx = Some(j);
                                break;
                            }
                        }
                        "]" | "}" => depth -= 1,
                        "," if depth == 1 => {
                            has_top_level_comma = true;
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if let Some(close_idx) = close_idx {
                    let next_after = kept.get(close_idx + 1).map(|t| t.value.as_str());
                    let next_terminates =
                        matches!(next_after, Some(";") | Some(",") | None);
                    if next_terminates
                        && !has_top_level_comma
                        && !starts_with_function
                    {
                        drops.push(open_idx);
                        drops.push(close_idx);
                        i = close_idx + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        if drops.is_empty() {
            break;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // gap-054 + gap-070: paren elision around a unary-keyword
    // operand. Pattern: `void (E)` / `typeof (E)` / `delete (E)`
    // where `E` is a "safe" operand whose grouping parens are
    // redundant. Drop both parens.
    //
    // Two shapes of safe operand are recognised:
    //
    //   (gap-054) a SINGLE safe token — a numeric literal, a
    //     string literal, or a simple identifier:
    //       typeof(x)   -> typeof x        delete(a) -> delete a
    //       typeof(1)   -> typeof 1        void(0)   -> void 0
    //
    //   (gap-070) a MEMBER-REFERENCE CHAIN — an identifier base
    //     followed by any run of `.name` / `?.name` / `[…]`
    //     accessors, with NO top-level operators, calls, or
    //     commas:
    //       delete(a.b)    -> delete a.b      delete(a[b]) -> delete a[b]
    //       typeof(a.b.c)  -> typeof a.b.c    void(a.b)    -> void a.b
    //
    // Both shapes are higher-precedence than the prefix unary
    // operator and self-delimiting, so the grouping parens carry
    // no meaning — `OP(REF)` and `OP REF` parse identically, and
    // whatever follows the close paren re-associates the same way
    // (member access binds tighter than the unary op). Operands
    // that contain a top-level binary operator (`void(a+b)`,
    // `delete(a-1)`) are LEFT ALONE: there `OP(a+b)` ≠ `OP a+b`.
    //
    // PROPERTY GUARD (gap-070 correctness fix): the keyword must
    // be the genuine unary OPERATOR, not a PROPERTY of the same
    // name. `o.delete(a)` is a method call (Map/Set#delete), NOT
    // a `delete` expression — stripping its call parens would
    // corrupt it into the invalid `o.delete a`. A property is
    // preceded by a `.`/`?.` accessor; the operator is at the
    // statement start or preceded by anything else. (Before this
    // guard, `o.delete(a)` mis-emitted as `o.delete a`.)
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 1 < kept.len() {
            // The keyword must be a real word-like token (never a
            // string literal `"delete"` whose `.value` matches).
            //
            // gap-071: `instanceof` joins the set. It is a BINARY
            // operator (`a instanceof(B)`), but the right-operand
            // paren elision is mechanically identical to the prefix
            // unary cases — the left operand sits at `kept[i-1]` and
            // is irrelevant to whether the RIGHT operand's grouping
            // parens are redundant. `instanceof` binds looser than
            // member access, so `a instanceof(B.c)` ≡ `a instanceof
            // B.c` and whatever follows the close paren re-associates
            // identically (same as the unary cases). The property
            // guard below also covers `o.instanceof(x)` — a property
            // access whose call parens must be preserved.
            let is_unary_kw = is_word_like(kept[i])
                && matches!(
                    kept[i].value.as_str(),
                    "void" | "typeof" | "delete" | "instanceof"
                );
            // Property guard: skip `o.delete(`, `o?.typeof(`, …
            let is_property = i >= 1
                && (is_structural_punct(kept[i - 1], ".")
                    || is_structural_punct(kept[i - 1], "?."));
            if is_unary_kw
                && !is_property
                && is_structural_punct(kept[i + 1], "(")
            {
                let open = i + 1;
                // Find the matching close paren (structural depth
                // scan — string/regex literals whose content holds
                // a bracket char never perturb the count).
                let mut depth: i32 = 1;
                let mut close: Option<usize> = None;
                let mut j = open + 1;
                while j < kept.len() {
                    let t = kept[j];
                    if is_structural_punct(t, "(")
                        || is_structural_punct(t, "[")
                        || is_structural_punct(t, "{")
                    {
                        depth += 1;
                    } else if is_structural_punct(t, ")") {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(j);
                            break;
                        }
                    } else if is_structural_punct(t, "]")
                        || is_structural_punct(t, "}")
                    {
                        depth -= 1;
                    }
                    j += 1;
                }
                if let Some(close) = close {
                    let span = &kept[open + 1..close];
                    // gap-054/070 + gap-101: the operand is "safe" if it
                    // is a single token, a member-reference chain
                    // (gap-054/070), a leading SYMBOL/KEYWORD unary chain,
                    // or a call/member chain (gap-101). All bind tighter
                    // than (or, for `instanceof`, are re-associated the
                    // same by) the operator, so the grouping parens are
                    // redundant. A parenthesised binary operand is still
                    // rejected and keeps its parens.
                    if is_safe_unary_kw_operand(span) {
                        drops.push(open);
                        drops.push(close);
                        i = close + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // gap-075 + gap-078: paren elision around a SYMBOL operator's
    // parenthesised RIGHT operand.
    //
    // gap-075 (the prefix-unary symbols `-`/`+`/`!`/`~`):
    //   `-(a)` → `-a`, `!(a)` → `!a`, `~(a)` → `~a`, and the
    //   same-sign nesting `-(-a)` → `- -a`, `+(+a)` → `+ +a`.
    //
    // gap-078 (the remaining BINARY symbol operators — comparison,
    // logical, arithmetic, bitwise):
    //   `a==(b)` → `a==b`, `a||(b)` → `a||b`, `a*(b)` → `a*b`,
    //   `a<<(b)` → `a<<b`, … (verified against the JAR for the full
    //   set `== != === !== < > <= >= && || ?? * / % ** & | ^ << >>
    //   >>>`).
    //
    // All are anchored on the PUNCTUATION operator
    // (`is_structural_punct`-gated, so a string/regex literal whose
    // CONTENT is e.g. `"=="` never matches) followed by `(`. There is
    // NO prefix-vs-binary distinction to make: stripping a grouping
    // paren around a SELF-DELIMITING operand is sound whether the
    // operator is a prefix unary (`-(a)`) or a binary operator whose
    // RIGHT operand is parenthesised (`a-(b)` → `a-b`, `a==(b)` →
    // `a==b`).
    //
    // The operand check (`is_safe_unary_paren_operand`) is the single
    // safety gate: it accepts ONLY a self-delimiting operand (a single
    // safe token, a member-reference chain, or a leading prefix-symbol
    // unary chain) and rejects anything containing a top-level binary
    // operator. An atomic operand has NO precedence interaction with
    // the outer operator, so the strip is always sound. The fuller,
    // precedence-aware elision the JAR also does (`a==(b+c)` →
    // `a==b+c`, since `+` binds tighter than `==`, while `a*(b+c)`
    // KEEPS its parens) is DEFERRED — it needs an operator-precedence
    // table; here `a==(b+c)` conservatively keeps its parens (valid,
    // just not yet byte-identical).
    //
    // SAME-SIGN SAFETY: when the operand begins with the SAME sign
    // (`-(-a)` / `+(+a)`), gluing the two operators would form the
    // spurious `--`/`++` (decrement / increment). The gap-063
    // `needs_separator` rule inserts a separating space once the
    // parens are gone, so this pre-pass only drops them. `!`/`~`
    // are prefix-only and carry no binary ambiguity; `--`/`++`
    // (single tokens whose `.value` is `"--"`/`"++"`) never match
    // the bare-`-`/`+` anchor.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 1 < kept.len() {
            // gap-075 prefix-unary symbol anchors.
            let is_sym_unary = is_structural_punct(kept[i], "-")
                || is_structural_punct(kept[i], "+")
                || is_structural_punct(kept[i], "!")
                || is_structural_punct(kept[i], "~");
            // gap-078 binary symbol-operator anchors (comparison /
            // logical / arithmetic / bitwise). `-`/`+` are already
            // covered above (they double as additive binary ops).
            let is_binary_sym = is_structural_punct(kept[i], "==")
                || is_structural_punct(kept[i], "!=")
                || is_structural_punct(kept[i], "===")
                || is_structural_punct(kept[i], "!==")
                || is_structural_punct(kept[i], "<")
                || is_structural_punct(kept[i], ">")
                || is_structural_punct(kept[i], "<=")
                || is_structural_punct(kept[i], ">=")
                || is_structural_punct(kept[i], "&&")
                || is_structural_punct(kept[i], "||")
                || is_structural_punct(kept[i], "??")
                || is_structural_punct(kept[i], "*")
                || is_structural_punct(kept[i], "/")
                || is_structural_punct(kept[i], "%")
                || is_structural_punct(kept[i], "**")
                || is_structural_punct(kept[i], "&")
                || is_structural_punct(kept[i], "|")
                || is_structural_punct(kept[i], "^")
                || is_structural_punct(kept[i], "<<")
                || is_structural_punct(kept[i], ">>")
                || is_structural_punct(kept[i], ">>>");
            if (is_sym_unary || is_binary_sym)
                && is_structural_punct(kept[i + 1], "(")
            {
                let open = i + 1;
                // Find the matching close paren (structural scan).
                let mut depth: i32 = 1;
                let mut close: Option<usize> = None;
                let mut j = open + 1;
                while j < kept.len() {
                    let t = kept[j];
                    if is_structural_punct(t, "(")
                        || is_structural_punct(t, "[")
                        || is_structural_punct(t, "{")
                    {
                        depth += 1;
                    } else if is_structural_punct(t, ")") {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(j);
                            break;
                        }
                    } else if is_structural_punct(t, "]")
                        || is_structural_punct(t, "}")
                    {
                        depth -= 1;
                    }
                    j += 1;
                }
                if let Some(close) = close {
                    let span = &kept[open + 1..close];
                    if is_safe_unary_paren_operand(span) {
                        drops.push(open);
                        drops.push(close);
                        i = close + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // gap-077 + gap-081: paren elision around a binary operator's
    // parenthesised LEFT operand, OR a ternary `?:` CONDITION.
    //   gap-077: `(a)+b` → `a+b`, `(a)*b` → `a*b`, `(a.b)+c` →
    //     `a.b+c`, `(a)||b` → `a||b` — the LEFT-hand mirror of
    //     gap-075/078 (which strip the RIGHT operand `a-(b)` → `a-b`,
    //     `a==(b)` → `a==b`).
    //   gap-081: `(a)?b:c` → `a?b:c`, `(a.b)?c:d` → `a.b?c:d` — the
    //     CONDITION-side mirror of gap-055's ternary-ARM elision. The
    //     parenthesised span sits to the LEFT of the `?`, so the same
    //     starts-an-expression + atomic-operand machinery applies.
    //
    // A `(` is a GROUPING paren (eligible) only when it STARTS a
    // sub-expression — i.e. the token before it does NOT produce a
    // value. A CALL / member paren (`f(a)`, `x[i](a)`, `(g)(a)`) is
    // preceded by a value-producing token (a word-like name/literal,
    // a string literal, or a `)`/`]`/`}` close) and must NEVER be
    // stripped — dropping it would corrupt `f(a)+b` into `fa+b`.
    //
    // The eligible `(`'s matching `)` must be IMMEDIATELY followed by
    // a BINARY operator (so the parenthesised span is that operator's
    // LEFT operand — `)` followed by `.`/`?.`/`(`/`[` is a member /
    // call and is left to gap-057 / the callee passes). The span
    // itself must pass `is_safe_unary_paren_operand` (a
    // self-delimiting atomic operand). An operand with a top-level
    // binary operator (`(a+b)*c`) or comma (`(a,b)+c`) is rejected and
    // keeps its parens — the precedence / comma-operator safety,
    // identical to gap-075/078.
    //
    // Every bracket / operator test routes through `is_structural_punct`
    // so a string/regex literal whose CONTENT is a bracket or operator
    // can never corrupt the depth scan or the anchor.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i < kept.len() {
            // The `(` must START an expression: it is either the first
            // token, or the preceding token does not produce a value.
            let starts_expr = i == 0 || {
                let p = kept[i - 1];
                !(is_word_like(p)
                    || is_string_literal(p)
                    || is_structural_punct(p, ")")
                    || is_structural_punct(p, "]")
                    || is_structural_punct(p, "}"))
            };
            if !(starts_expr && is_structural_punct(kept[i], "(")) {
                i += 1;
                continue;
            }
            // Match the `(` … `)` (structural depth scan).
            let open = i;
            let mut depth: i32 = 1;
            let mut close: Option<usize> = None;
            let mut j = open + 1;
            while j < kept.len() {
                let t = kept[j];
                if is_structural_punct(t, "(")
                    || is_structural_punct(t, "[")
                    || is_structural_punct(t, "{")
                {
                    depth += 1;
                } else if is_structural_punct(t, ")") {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(j);
                        break;
                    }
                } else if is_structural_punct(t, "]")
                    || is_structural_punct(t, "}")
                {
                    depth -= 1;
                }
                j += 1;
            }
            let Some(close) = close else {
                i += 1;
                continue;
            };
            // The token after `)` must be a BINARY operator (then the
            // parenthesised span is that operator's LEFT operand) OR a
            // ternary `?` (gap-081: then the span is the `?:`
            // CONDITION — `(a)?b:c` → `a?b:c`, the condition-side
            // mirror of gap-055's ternary-ARM elision). The optional-
            // chain `?.` lexes as a single `"?."` token, so
            // `is_structural_punct(t, "?")` matches ONLY the bare
            // ternary `?` and never `(a)?.b`.
            let after = close + 1;
            let is_binary_or_cond_after = after < kept.len() && {
                let t = kept[after];
                is_structural_punct(t, "+")
                    || is_structural_punct(t, "-")
                    || is_structural_punct(t, "*")
                    || is_structural_punct(t, "/")
                    || is_structural_punct(t, "%")
                    || is_structural_punct(t, "**")
                    || is_structural_punct(t, "==")
                    || is_structural_punct(t, "!=")
                    || is_structural_punct(t, "===")
                    || is_structural_punct(t, "!==")
                    || is_structural_punct(t, "<")
                    || is_structural_punct(t, ">")
                    || is_structural_punct(t, "<=")
                    || is_structural_punct(t, ">=")
                    || is_structural_punct(t, "&&")
                    || is_structural_punct(t, "||")
                    || is_structural_punct(t, "??")
                    || is_structural_punct(t, "&")
                    || is_structural_punct(t, "|")
                    || is_structural_punct(t, "^")
                    || is_structural_punct(t, "<<")
                    || is_structural_punct(t, ">>")
                    || is_structural_punct(t, ">>>")
                    // gap-081: ternary CONDITION.
                    || is_structural_punct(t, "?")
            };
            if is_binary_or_cond_after {
                let span = &kept[open + 1..close];
                // EXPONENTIATION HAZARD: `**` forbids an
                // UNPARENTHESISED unary LEFT operand — `-a**b` is a
                // SyntaxError (ECMAScript: the left side of `**` must
                // be an `UpdateExpression`, not a `UnaryExpression`).
                // So `(-a)**b`, `(!a)**b`, `(typeof a)**b`, … must
                // KEEP their parens even though the operand is
                // otherwise "safe". Only the `**` case is affected;
                // every other binary operator accepts a unary left
                // operand unparenthesised.
                let exp_unary_hazard = is_structural_punct(
                    kept[after],
                    "**",
                ) && span.first().is_some_and(|t| {
                    is_structural_punct(t, "-")
                        || is_structural_punct(t, "+")
                        || is_structural_punct(t, "!")
                        || is_structural_punct(t, "~")
                        || (is_word_like(t)
                            && matches!(
                                t.value.as_str(),
                                "typeof" | "void" | "delete" | "await"
                            ))
                });
                if !exp_unary_hazard
                    && is_safe_unary_paren_operand(span)
                {
                    drops.push(open);
                    drops.push(close);
                    i = close + 1;
                    continue;
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // gap-087: paren elision inside a COMPUTED-MEMBER index —
    // `a[(b)]` → `a[b]`, `a[(b+c)]` → `a[b+c]`, `a[(b,c)]` →
    // `a[b,c]`, `x()[(b)]` → `x()[b]`, `a[b[(c)]]` → `a[b[c]]`.
    //
    // A computed-member `[` (the subscript operator) is preceded by
    // a VALUE-producing token (a word-like name/literal, a string, or
    // a `)`/`]`/`}` close) — exactly the tokens that make a following
    // `(` a CALL paren. That is the mirror of the gap-077
    // `starts_expr` test: here we REQUIRE the `[` to follow a value
    // (so it is a subscript, not an ARRAY LITERAL `[`). The
    // distinction matters because inside an array literal a top-level
    // comma is an ELEMENT separator (`[(a,b)]` must KEEP its parens,
    // else it becomes the two-element `[a,b]`), whereas inside a
    // subscript the brackets already delimit a SINGLE expression, so
    // even a comma operator is safe to expose (`a[(b,c)]` → `a[b,c]`).
    // Array-literal element parens are the comma-guarded gap-086
    // family and are handled separately.
    //
    // Eligibility: the `[` is immediately followed by `(`, and that
    // `(`'s matching `)` is immediately followed by the matching `]`
    // — i.e. the parens wrap the WHOLE index. A partial paren
    // (`a[(b)+c]`) is already handled by the gap-077 left-operand
    // pass; a call/group that is not the whole index (`a[(f)(b)]`)
    // has its `)` followed by `(`, not `]`, so it is left alone. No
    // comma / atomic-operand guard is needed: any single expression
    // is safe once the enclosing `[ … ]` is preserved.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i < kept.len() {
            // Subscript `[`: preceded by a value-producing token.
            let is_subscript = i > 0 && {
                let p = kept[i - 1];
                is_word_like(p)
                    || is_string_literal(p)
                    || is_structural_punct(p, ")")
                    || is_structural_punct(p, "]")
                    || is_structural_punct(p, "}")
            };
            // Need `[` `(` … directly adjacent.
            if !(is_subscript
                && is_structural_punct(kept[i], "[")
                && i + 1 < kept.len()
                && is_structural_punct(kept[i + 1], "("))
            {
                i += 1;
                continue;
            }
            let open = i + 1; // the `(`
            // Match the `(` … `)` (structural depth scan, identical to
            // the gap-077 scan so string/regex bracket content can
            // never corrupt the depth).
            let mut depth: i32 = 1;
            let mut close: Option<usize> = None;
            let mut j = open + 1;
            while j < kept.len() {
                let t = kept[j];
                if is_structural_punct(t, "(")
                    || is_structural_punct(t, "[")
                    || is_structural_punct(t, "{")
                {
                    depth += 1;
                } else if is_structural_punct(t, ")") {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(j);
                        break;
                    }
                } else if is_structural_punct(t, "]")
                    || is_structural_punct(t, "}")
                {
                    depth -= 1;
                }
                j += 1;
            }
            let Some(close) = close else {
                i += 1;
                continue;
            };
            // The parens wrap the WHOLE index iff the token after `)`
            // is the matching `]` of this subscript.
            if close + 1 < kept.len()
                && is_structural_punct(kept[close + 1], "]")
            {
                drops.push(open);
                drops.push(close);
                i = close + 2; // past `… ) ]`
                continue;
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // gap-086: paren elision around a whole CALL ARGUMENT —
    // `f((a))` → `f(a)`, `f((a+b))` → `f(a+b)`, `f((a),(b))` →
    // `f(a,b)`, `f((a),b)` → `f(a,b)`, `f(g((a)))` → `f(g(a))`.
    //
    // We anchor on the CALL-OPEN paren — a `(` immediately preceded by
    // a VALUE-producing token (a word-like name/literal, a string, or a
    // `)`/`]`/`}` close). That is exactly the paren that opens an
    // ARGUMENT LIST (`f(`, `a.b(`, `x[i](`, `g)(`), as opposed to a
    // GROUPING paren (handled by gap-077 etc.). Anchoring on the call
    // open — rather than on a bare `,`/`(` before a candidate group —
    // keeps ARRAY LITERALS out of scope entirely: `[(a,b)]` is never
    // reached, so its load-bearing element-paren is preserved.
    //
    // Walking the arg list at relative depth 1, each ARGUMENT is the
    // token span between two boundaries (the call `(`, each depth-1
    // `,`, the closing `)`). An argument that is ENTIRELY `( … )`
    // (its first token is `(` whose structural match is the token just
    // before the boundary) has that wrapping paren stripped — UNLESS
    // the inner span contains a TOP-LEVEL COMMA. That is the one
    // load-bearing case: `f((a,b))` is a SINGLE argument whose value is
    // the comma-operator `a,b`; dropping the parens would resplit it
    // into the TWO arguments `a,b`. (Precedence is irrelevant here —
    // an argument position accepts any AssignmentExpression, so unlike
    // the operand passes we do NOT need the atomic-operand guard;
    // `f((a+b))` → `f(a+b)` is safe.) An arrow whose param list is
    // parenthesised (`f((a)=>a)`) is skipped automatically: the `)` of
    // `(a)` is followed by `=>`, not a `,`/`)` boundary.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i < kept.len() {
            // Locate a call-open paren.
            let is_call_open = is_structural_punct(kept[i], "(") && i > 0 && {
                let p = kept[i - 1];
                is_word_like(p)
                    || is_string_literal(p)
                    || is_structural_punct(p, ")")
                    || is_structural_punct(p, "]")
                    || is_structural_punct(p, "}")
            };
            if !is_call_open {
                i += 1;
                continue;
            }
            // Walk the argument list at relative depth 1. `arg_start`
            // is the index of the first token of the current argument.
            let mut depth: i32 = 1;
            let mut arg_start = i + 1;
            let mut j = i + 1;
            while j < kept.len() {
                let t = kept[j];
                if is_structural_punct(t, "(")
                    || is_structural_punct(t, "[")
                    || is_structural_punct(t, "{")
                {
                    depth += 1;
                } else if is_structural_punct(t, ")")
                    || is_structural_punct(t, "]")
                    || is_structural_punct(t, "}")
                {
                    depth -= 1;
                    if depth == 0 {
                        // Call close: the final argument is
                        // `kept[arg_start..j]`.
                        maybe_strip_arg_paren(&kept, arg_start, j, &mut drops);
                        break;
                    }
                } else if is_structural_punct(t, ",") && depth == 1 {
                    // Argument boundary: `kept[arg_start..j]`.
                    maybe_strip_arg_paren(&kept, arg_start, j, &mut drops);
                    arg_start = j + 1;
                }
                j += 1;
            }
            // Continue the OUTER scan from the next token; nested calls
            // are reached as their own call-open anchors.
            i += 1;
        }
        drops.sort_unstable();
        drops.dedup();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // gap-055: paren elision around a *whole-arm* sub-
    // expression that follows `?` or `:`. Upstream Closure
    // strips redundant grouping parens around:
    //   - ternary then-arm:   `x?(E):y`   → `x?E:y`
    //   - ternary else-arm:   `x?y:(E)`   → `x?y:E`
    //   - object-literal val:  `{a:(E)}`  → `{a:E}`
    //   - label / `case` body: `foo:(E);` → `foo:E;`
    //
    // SAFETY — three guards, all necessary:
    //
    //   1. WHOLE-ARM: the token AFTER the matching `)` must be
    //      a delimiter that ends the sub-expression (`:` for a
    //      then-arm, or one of `, ; ) ] }` / EOF for an else-
    //      arm / value). Otherwise dropping changes precedence,
    //      e.g. `x?(a=1)+2:c` must stay (next-after-`)` is `+`).
    //
    //   2. NO TOP-LEVEL COMMA: `(a,b)` is a comma operator —
    //      semantically distinct from `a,b` in these positions.
    //      Upstream keeps it; so do we.
    //
    //   3. STRUCTURAL-ONLY token matching: a string / regex /
    //      template literal stores its *content* in `.value`
    //      (delimiters stripped), so a one-char string `")"`
    //      has `.value == ")"`. We MUST gate every bracket /
    //      comma / `?` / `:` comparison on `is_structural_punct`
    //      — else a bracket char inside a literal corrupts the
    //      depth counter and the wrong token gets removed,
    //      producing broken output like `x?):y` from
    //      `x?(")"):y`.
    //
    // `?.` lexes as a single OPTIONAL_CHAIN token, so the
    // `prev == "?"` test never fires on optional chaining
    // (`a?.(b)` is left untouched).
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 1;
        while i + 1 < kept.len() {
            // ---- prefix classification (gap-055 + gap-056) ----
            // gap-055 prefixes: ternary `?` / `:` (also covers
            // object-literal value and label/case bodies).
            let prev = &kept[i - 1];
            let is_arm_prefix = is_structural_punct(prev, "?")
                || is_structural_punct(prev, ":");
            // gap-056 prefixes: concise-arrow body (`=>`) and the
            // statement keywords `return` / `throw`.
            let is_arrow_prefix = is_structural_punct(prev, "=>");
            // `return` / `throw` must be the STATEMENT keyword,
            // not a property name (`gen.throw(e)`, `it.return()`).
            // A property name is preceded by a member accessor
            // (`.` or `?.`); a statement keyword is at the start
            // of the token stream or preceded by anything else.
            let is_ret_throw_prefix = (prev.value == "return"
                || prev.value == "throw")
                && !is_string_literal(prev)
                && (i < 2
                    || (!is_structural_punct(&kept[i - 2], ".")
                        && !is_structural_punct(&kept[i - 2], "?.")));
            // gap-102: `yield` operand. `yield` takes an
            // AssignmentExpression (binding looser than every binary
            // operator), so a grouping paren around the operand is
            // redundant — exactly like `return`/`throw`. The same
            // property guard applies: `yield` must be the genuine
            // generator keyword, not a property name (`o.yield(x)` is a
            // method call). `yield*` delegate is excluded for free: the
            // token after `yield` is then `*`, not `(`, so the pass
            // never fires. The shared top-level-comma guard keeps
            // `yield(a,b)` wrapped (`yield a,b` ≡ `(yield a),b`).
            let is_yield_prefix = prev.value == "yield"
                && !is_string_literal(prev)
                && (i < 2
                    || (!is_structural_punct(&kept[i - 2], ".")
                        && !is_structural_punct(&kept[i - 2], "?.")));
            let prefix_matches = is_arm_prefix
                || is_arrow_prefix
                || is_ret_throw_prefix
                || is_yield_prefix;
            if prefix_matches && is_structural_punct(&kept[i], "(") {
                let open_idx = i;
                let mut depth: i32 = 1;
                let mut has_top_level_comma = false;
                let mut close_idx: Option<usize> = None;
                let mut j = open_idx + 1;
                while j < kept.len() {
                    let t = &kept[j];
                    if is_structural_punct(t, "(")
                        || is_structural_punct(t, "[")
                        || is_structural_punct(t, "{")
                    {
                        depth += 1;
                    } else if is_structural_punct(t, ")") {
                        depth -= 1;
                        if depth == 0 {
                            close_idx = Some(j);
                            break;
                        }
                    } else if is_structural_punct(t, "]")
                        || is_structural_punct(t, "}")
                    {
                        depth -= 1;
                    } else if depth == 1 && is_structural_punct(t, ",") {
                        has_top_level_comma = true;
                    }
                    j += 1;
                }
                if let Some(close_idx) = close_idx {
                    let arm_complete = match kept.get(close_idx + 1) {
                        None => true,
                        Some(t) => {
                            is_structural_punct(t, ":")
                                || is_structural_punct(t, ";")
                                || is_structural_punct(t, ",")
                                || is_structural_punct(t, ")")
                                || is_structural_punct(t, "]")
                                || is_structural_punct(t, "}")
                        }
                    };
                    // gap-056 arrow guard: a concise arrow body
                    // that starts with `{` is AMBIGUOUS — bare
                    // `x=>{...}` parses as a function BLOCK body,
                    // not an object literal, so `x=>({a:1})` must
                    // keep its parens. (After `?`/`:`/`return`/
                    // `throw` the operand is unambiguously an
                    // expression, so `{` is fine there.)
                    let arrow_brace_body = is_arrow_prefix
                        && kept
                            .get(open_idx + 1)
                            .map(|t| is_structural_punct(t, "{"))
                            .unwrap_or(false);
                    if arm_complete && !has_top_level_comma && !arrow_brace_body
                    {
                        drops.push(open_idx);
                        drops.push(close_idx);
                        i = close_idx + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-057: member-object paren elision -----------------
    // `(a).b` → `a.b`. Upstream Closure strips the grouping parens
    // around a member-expression's OBJECT when the object is a
    // single identifier, because `.` (member access) binds tighter
    // than anything the parens could be protecting — for a lone
    // identifier there is nothing to protect.
    //
    // Three guards make this provably safe:
    //
    //   1. GROUPING, NOT CALL: the `(` must be a *grouping* paren,
    //      never a call/index paren. `f(a).b` is a CALL on `f`;
    //      stripping would corrupt it to `fa.b`. A `(` is a
    //      call/index paren exactly when it directly follows a
    //      *value* (identifier / literal / `)` / `]`) or the
    //      optional-call operator `?.`. So we require the token
    //      BEFORE `(` to be a punctuation/operator token that is
    //      NOT `)`, `]`, or `?.` — or for `(` to start the stream.
    //      `x?.(a).b` (optional call) is left untouched.
    //
    //   2. SINGLE PLAIN IDENTIFIER: the parens must wrap exactly
    //      one token, and that token must be a plain identifier
    //      (`is_plain_identifier`). Numbers are excluded (`(1).x`
    //      → `1.x` mis-lexes), as are keywords/literals/regex.
    //
    //   3. MEMBER ACCESS FOLLOWS: the token after `)` must be `.`
    //      — this is the member-object position. (`(a)[i]` and
    //      `(a)(x)` are also safe for a lone identifier but are
    //      left to a follow-up; the byte-identity fixture only
    //      exercises `.`.)
    //
    // Every bracket/operator comparison goes through the
    // structural-punct guards so a literal whose content looks
    // like punctuation (e.g. the string `")"`) can never be
    // mistaken for a delimiter.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 3 < kept.len() {
            if is_structural_punct(&kept[i], "(") {
                let grouping = match i.checked_sub(1).and_then(|k| kept.get(k)) {
                    None => true,
                    Some(p) => {
                        is_punct(p)
                            && !is_structural_punct(p, ")")
                            && !is_structural_punct(p, "]")
                            && !is_structural_punct(p, "?.")
                    }
                };
                let inner_ok = is_plain_identifier(&kept[i + 1]);
                let close_ok = is_structural_punct(&kept[i + 2], ")");
                let member_follows = is_structural_punct(&kept[i + 3], ".");
                if grouping && inner_ok && close_ok && member_follows {
                    drops.push(i); // open `(`
                    drops.push(i + 2); // close `)`
                    i += 3;
                    continue;
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-065: callee paren elision ------------------------
    // `(f)(x)` → `f(x)`. Sibling of gap-057: upstream strips the
    // grouping parens around the CALLEE of a call / tagged
    // template when the callee is a *simple reference* — a plain
    // identifier optionally followed by a `.IDENT` member chain:
    //
    //   `(f)(x)`      →  `f(x)`
    //   `(a.b)(x)`    →  `a.b(x)`
    //   `(f)`tpl``    →  `f`tpl``   (tagged template)
    //
    // `.` and `(`/`` ` `` all bind tighter than anything a
    // grouping paren could be protecting around a bare reference,
    // so removal is meaning-preserving. The guards mirror gap-057:
    //
    //   1. GROUPING, NOT CALL: the `(` must be a grouping paren
    //      (prev token is punct other than `)`/`]`/`?.`, or start
    //      of stream) — never a call/index paren. `f(g)(x)` keeps
    //      its parens (the `(g)` is f's call).
    //   2. SIMPLE REFERENCE: the inner is a plain identifier plus
    //      zero-or-more `.IDENT` accessors. NO commas, operators,
    //      computed `[...]`, or calls inside — `(a,b)(x)` (a
    //      sequence) and `(a+b)(x)` keep their parens because
    //      unwrapping would change meaning. (The scan simply
    //      stops at the first non-`.IDENT` token; if that isn't
    //      the matching `)`, nothing is dropped.)
    //   3. CALL / TAG FOLLOWS: the token after `)` must be a real
    //      `(` call paren or a template literal (tagged template).
    //
    // All bracket checks route through `is_structural_punct`; the
    // tagged-template follower is gated on `is_word_like` so a
    // string whose content starts with `` ` `` can't trigger it.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 2 < kept.len() {
            if is_structural_punct(&kept[i], "(")
                && is_plain_identifier(&kept[i + 1])
            {
                let grouping = match i.checked_sub(1).and_then(|k| kept.get(k)) {
                    None => true,
                    Some(p) => {
                        is_punct(p)
                            && !is_structural_punct(p, ")")
                            && !is_structural_punct(p, "]")
                            && !is_structural_punct(p, "?.")
                    }
                };
                if grouping {
                    // Consume the `.IDENT` member chain.
                    let mut p = i + 1;
                    while p + 2 < kept.len()
                        && is_structural_punct(&kept[p + 1], ".")
                        && is_plain_identifier(&kept[p + 2])
                    {
                        p += 2;
                    }
                    // Expect the matching `)` immediately, then a
                    // call `(` or a tagged-template literal.
                    let close_ok = kept
                        .get(p + 1)
                        .map(|t| is_structural_punct(t, ")"))
                        .unwrap_or(false);
                    let call_or_tag = kept
                        .get(p + 2)
                        .map(|t| {
                            is_structural_punct(t, "(")
                                || (is_word_like(t) && t.value.starts_with('`'))
                        })
                        .unwrap_or(false);
                    if close_ok && call_or_tag {
                        drops.push(i); // open `(`
                        drops.push(p + 1); // close `)`
                        i = p + 2;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-099: computed-member object paren elision ---------
    // `(b)[c]` → `b[c]`. The `[index]` sibling of gap-065 (callee)
    // and gap-057 (`.member`): upstream strips the grouping parens
    // around the OBJECT of a computed-member `[…]` access when the
    // object is a *simple reference* — a plain identifier plus
    // zero-or-more `.IDENT` accessors. `[` binds tighter than any
    // operator a grouping paren could be protecting around a bare
    // reference, so removal is meaning-preserving:
    //
    //   `(b)[c]`     →  `b[c]`
    //   `(b.c)[d]`   →  `b.c[d]`
    //   `(a)[b]=c`   →  `a[b]=c`
    //
    // The guards mirror gap-065 exactly; only the FOLLOWER differs:
    //
    //   1. GROUPING, NOT CALL/INDEX: the `(` must be a grouping
    //      paren (prev token is punct other than `)`/`]`/`?.`, or
    //      start of stream) — never a call/index paren. `f(a)[b]`
    //      keeps its parens (the `(a)` is f's call).
    //   2. SIMPLE REFERENCE: the inner is a plain identifier plus a
    //      `.IDENT` chain. NO operators/commas/computed members —
    //      `(a+b)[c]` and `(b||c)[d]` keep their parens (the scan
    //      stops at the first non-`.IDENT` token; if that isn't the
    //      matching `)`, nothing is dropped).
    //   3. COMPUTED INDEX FOLLOWS: the token after `)` must be a
    //      real `[` index bracket.
    //
    // Distinct from gap-087, which elided parens INSIDE the index
    // (`a[(b)]` → `a[b]`); gap-099 is the object side. All bracket
    // checks route through `is_structural_punct`.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 2 < kept.len() {
            if is_structural_punct(&kept[i], "(")
                && is_plain_identifier(&kept[i + 1])
            {
                let grouping = match i.checked_sub(1).and_then(|k| kept.get(k)) {
                    None => true,
                    Some(p) => {
                        is_punct(p)
                            && !is_structural_punct(p, ")")
                            && !is_structural_punct(p, "]")
                            && !is_structural_punct(p, "?.")
                    }
                };
                if grouping {
                    // Consume the `.IDENT` member chain.
                    let mut p = i + 1;
                    while p + 2 < kept.len()
                        && is_structural_punct(&kept[p + 1], ".")
                        && is_plain_identifier(&kept[p + 2])
                    {
                        p += 2;
                    }
                    // Expect the matching `)` immediately, then a
                    // computed-index `[`.
                    let close_ok = kept
                        .get(p + 1)
                        .map(|t| is_structural_punct(t, ")"))
                        .unwrap_or(false);
                    let index_follows = kept
                        .get(p + 2)
                        .map(|t| is_structural_punct(t, "["))
                        .unwrap_or(false);
                    if close_ok && index_follows {
                        drops.push(i); // open `(`
                        drops.push(p + 1); // close `)`
                        i = p + 2;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-100: function/class-expression paren elision in
    //      EXPRESSION position ---------------------------------
    // `a=(function(){})()` → `a=function(){}()`. A parenthesised
    // `function`/`class` EXPRESSION only needs its wrapping parens
    // at STATEMENT position — there the leading `function`/`class`
    // keyword would otherwise begin a *declaration*. In expression
    // position the grouping parens are redundant:
    //
    //   `a=(function(){})()`       → `a=function(){}()`
    //   `a=(class{})()`            → `a=class{}()`
    //   `a=(async function(){})()` → `a=async function(){}()`
    //   `(function(){})();`        → unchanged (statement position)
    //
    // MINIMAL SAFE SLICE: fire only when the `(` is preceded by `=`
    // or `,` — the clearest expression-context delimiters, and the
    // ones the byte-identity fixtures exercise. This conservatively
    // EXCLUDES the load-bearing statement-position IIFE
    // `(function(){})();` (whose `(` is preceded by `;`/`{`/`}`/
    // start-of-stream, never `=`/`,`), which MUST keep its parens to
    // avoid being reparsed as a function declaration. Broader
    // expression contexts (after `(`/`[`/`return`/`=>`/operators)
    // are left to a follow-up.
    //
    // The matching `)` is located by a structural paren-depth scan
    // (the function's own param `()` nest and re-balance inside);
    // all bracket checks route through `is_structural_punct` so a
    // string/regex whose content looks like a paren can't fool it.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 1 < kept.len() {
            if is_structural_punct(&kept[i], "(") {
                // Expression-context predecessor: `=` or `,`.
                let expr_ctx = match i.checked_sub(1).and_then(|k| kept.get(k)) {
                    Some(p) if is_structural_punct(p, "=") => {
                        // Assignment `=`: require the target (i-2) to be a
                        // plain identifier at a STATEMENT boundary (i-3 is
                        // `;`/`{`/`}` or start-of-stream). This excludes a
                        // default-PARAMETER `=` (inside a `(...)` param
                        // list, where i-3 is `(`/`,`) — unwrapping THERE
                        // exposes the function body's `}` to the
                        // function-decl trailing-`;` pass and corrupts the
                        // output (`g(a=function(){};())`).
                        i >= 2
                            && is_plain_identifier(&kept[i - 2])
                            && match i.checked_sub(3).and_then(|k| kept.get(k)) {
                                None => true,
                                Some(b) => {
                                    is_structural_punct(b, ";")
                                        || is_structural_punct(b, "{")
                                        || is_structural_punct(b, "}")
                                }
                            }
                    }
                    Some(p) => is_structural_punct(p, ","),
                    None => false,
                };
                // The inner must START a function/class expression.
                let inner = &kept[i + 1];
                let is_fn_or_class = is_word_like(inner)
                    && (inner.value == "function"
                        || inner.value == "class"
                        || (inner.value == "async"
                            && kept
                                .get(i + 2)
                                .is_some_and(|t| is_word_like(t) && t.value == "function")));
                if expr_ctx && is_fn_or_class {
                    // Structural paren-depth scan for the matching `)`.
                    let mut depth: i32 = 1;
                    let mut j = i + 1;
                    let mut close: Option<usize> = None;
                    while j < kept.len() {
                        let t = &kept[j];
                        if is_structural_punct(t, "(") {
                            depth += 1;
                        } else if is_structural_punct(t, ")") {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(j);
                                break;
                            }
                        }
                        j += 1;
                    }
                    if let Some(c) = close {
                        drops.push(i); // grouping open `(`
                        drops.push(c); // grouping close `)`
                        i = c + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-066: redundant parens after `extends` ------------
    // `class A extends(B){}` → `class A extends B{}`. After the
    // `extends` keyword a parenthesized simple reference is
    // redundant — the class body `{` delimits the heritage
    // clause, so `extends B{` is unambiguous.
    //
    // Minimal safe slice: the inner must be a *simple reference*
    // — a plain identifier plus zero-or-more `.IDENT` accessors
    // (mirroring gap-065). The scan stops at the first non-
    // `.IDENT` token, so:
    //   - `extends(B||C)` keeps its parens — `B||C` is NOT a
    //     LeftHandSideExpression, so `extends B||C` would be
    //     INVALID JS (upstream strips it anyway, producing
    //     arguably-invalid output; we stay safe and conservative).
    //   - `extends(f())` (call-chain inner) is left for a
    //     follow-up — not yet fixtured.
    //
    // Guards:
    //   - the `(` must directly follow the `extends` KEYWORD, not
    //     a string literal whose content is `extends` and not a
    //     PROPERTY named `extends` (`obj.extends(x)` is a method
    //     call — prev-prev must not be `.`/`?.`),
    //   - all bracket/accessor checks via `is_structural_punct`.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 1;
        while i + 1 < kept.len() {
            let after_extends = kept[i - 1].value == "extends"
                && !is_string_literal(kept[i - 1])
                && match i.checked_sub(2).and_then(|k| kept.get(k)) {
                    None => true,
                    Some(pp) => {
                        !is_structural_punct(pp, ".")
                            && !is_structural_punct(pp, "?.")
                    }
                };
            if after_extends
                && is_structural_punct(&kept[i], "(")
                && is_plain_identifier(&kept[i + 1])
            {
                // Consume the `.IDENT` member chain.
                let mut p = i + 1;
                while p + 2 < kept.len()
                    && is_structural_punct(&kept[p + 1], ".")
                    && is_plain_identifier(&kept[p + 2])
                {
                    p += 2;
                }
                let close_ok = kept
                    .get(p + 1)
                    .map(|t| is_structural_punct(t, ")"))
                    .unwrap_or(false);
                if close_ok {
                    drops.push(i); // open `(`
                    drops.push(p + 1); // close `)`
                    i = p + 2;
                    continue;
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-068: redundant parens around a `new` callee ------
    // `new(f)()` → `new f`, `new(a.b)` → `new a.b`. After the
    // `new` keyword a parenthesized simple reference is redundant
    // — member access (`.`) binds tighter than an argument-less
    // `new`, so `new a.b` constructs `a.b` just like `new(a.b)`.
    //
    // This pass only STRIPS the parens around the callee. For the
    // call form `new(f)()` the trailing empty `()` is then
    // dropped by the existing gap-050 empty-paren elision in the
    // emit loop (which runs after this pre-pass): `new f()` →
    // `new f`.
    //
    // Minimal safe slice (mirrors gap-066): the inner must be a
    // *simple reference* — a plain identifier plus zero-or-more
    // `.IDENT` accessors. The scan stops at the first non-`.IDENT`
    // token, so an operator inner `new(a+b)` keeps its parens
    // (`new a+b` would parse as `(new a)+b` — a different
    // program). Computed `[...]` and call-chain inners are
    // deferred.
    //
    // Guards:
    //   - operator `new`, not a PROPERTY named `new`
    //     (`o.new(f)` is a method call — prev-prev must not be
    //     `.`/`?.`), and not a string literal whose content is
    //     `new`,
    //   - all bracket/accessor checks via `is_structural_punct`.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 1;
        while i + 1 < kept.len() {
            let after_new = kept[i - 1].value == "new"
                && !is_string_literal(kept[i - 1])
                && match i.checked_sub(2).and_then(|k| kept.get(k)) {
                    None => true,
                    Some(pp) => {
                        !is_structural_punct(pp, ".")
                            && !is_structural_punct(pp, "?.")
                    }
                };
            if after_new
                && is_structural_punct(&kept[i], "(")
                && is_plain_identifier(&kept[i + 1])
            {
                // Consume the `.IDENT` member chain.
                let mut p = i + 1;
                while p + 2 < kept.len()
                    && is_structural_punct(&kept[p + 1], ".")
                    && is_plain_identifier(&kept[p + 2])
                {
                    p += 2;
                }
                let close_ok = kept
                    .get(p + 1)
                    .map(|t| is_structural_punct(t, ")"))
                    .unwrap_or(false);
                if close_ok {
                    drops.push(i); // open `(`
                    drops.push(p + 1); // close `)`
                    i = p + 2;
                    continue;
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-059: member/call on a `new` expression ----------
    // `new A().b` → `(new A).b`. When a NewExpression is the
    // OBJECT of a member access (`.`/`[`) or the CALLEE of a
    // call (`(`), upstream Closure wraps the whole `new …` in
    // parens — because `new A.b` would parse as `new (A.b)`
    // (member access binds tighter than argument-less `new`),
    // which is a DIFFERENT program. It also drops the empty
    // `()` arg list (cf. gap-050, which only drops it when no
    // member/call follows — exactly the cases this pass does
    // NOT handle, so the two are complementary).
    //
    // We implement the wrap WITHOUT synthesising tokens: the
    // empty arg-list already contributes a `(` and a `)` to the
    // stream, so we just REORDER them — move the `(` to before
    // `new`, leaving the `)` after the identifier:
    //
    //   `new A ( ) .`   →   `( new A ) .`
    //   [new,A,(,),.]       [(,new,A,),.]
    //
    // Safe slice (matches `minify_new_member_chain` (gap-059)
    // AND `minify_new_member_callee` (gap-060)):
    //   - the CALLEE is a plain identifier optionally followed
    //     by `.IDENT` member accessors: `new A` (gap-059) or
    //     `new a.b.C` (gap-060). Computed `[...]` callees are a
    //     deferred follow-up.
    //   - EMPTY arg list `()` (arg-bearing `new A(y).b` →
    //     `(new A(y)).b` is a deferred follow-up — it can't use
    //     the reorder trick since the args aren't empty),
    //   - followed by `.`, `[`, or `(`.
    //
    // Guards:
    //   - operator `new`, not the property name `.new`/`?.new`
    //     (a property `new` is preceded by a member accessor),
    //   - all bracket / accessor checks go through
    //     `is_structural_punct` so a string/regex/template whose
    //     value looks like a bracket can never trigger it.
    {
        let mut i = 0;
        while i + 1 < kept.len() {
            let is_operator_new = kept[i].value == "new"
                && !is_string_literal(kept[i])
                && (i == 0
                    || (!is_structural_punct(&kept[i - 1], ".")
                        && !is_structural_punct(&kept[i - 1], "?.")));
            if is_operator_new
                && is_simple_identifier_token(kept.get(i + 1).copied())
            {
                // Scan the callee extent: the leading identifier
                // (at i+1) plus zero or more `.IDENT` accessors.
                let mut p = i + 2;
                while p + 1 < kept.len()
                    && is_structural_punct(&kept[p], ".")
                    && is_simple_identifier_token(Some(kept[p + 1]))
                {
                    p += 2;
                }
                // Expect EMPTY args `( )` at p, p+1 then a
                // member/call follower at p+2.
                if p + 2 < kept.len()
                    && is_structural_punct(&kept[p], "(")
                    && is_structural_punct(&kept[p + 1], ")")
                    && (is_structural_punct(&kept[p + 2], ".")
                        || is_structural_punct(&kept[p + 2], "[")
                        || is_structural_punct(&kept[p + 2], "("))
                {
                    // Reorder: pull the `(` (empty arg-list open)
                    // out from p and re-insert it before `new`.
                    // The `)` stays put — now closing the wrap.
                    let open = kept.remove(p);
                    kept.insert(i, open);
                    // Past `( new <callee> )`; the follower is
                    // now at p+2 and re-scanned next iteration.
                    i = p + 2;
                    continue;
                }
            }
            i += 1;
        }
    }

    // ---- gap-062: redundant double-paren collapse ------------
    // `((a+b))*c` → `(a+b)*c`. When a GROUPING `(` is directly
    // followed by another `(`, and the inner group's matching
    // `)` is directly followed by the outer `)`, the OUTER layer
    // is a redundant grouping — strip it.
    //
    // Minimal safe slice (matches `minify_double_paren_arith`):
    //   - the outer `(` must be a GROUPING paren, never a call /
    //     index paren. `f((a))` (outer is f's call) and
    //     `a[(x)]`-style are left to a follow-up — otherwise we
    //     could turn `f((a,b))` (one comma-operator arg) into
    //     `f(a,b)` (two args), changing the program.
    //   - the two open parens must be ADJACENT (`( (`),
    //   - the inner group's `)` must be IMMEDIATELY followed by
    //     the outer `)` (so the parens nest with nothing between
    //     them — purely redundant),
    //   - NO top-level comma inside the inner group (extra
    //     conservatism; comma-operator grouping is deferred).
    //
    // Upstream actually eliminates parens more aggressively
    // (`((a))` → `a`, `(a)+(b)` → `a+b`); this slice only strips
    // ONE directly-nested grouping layer. All bracket / comma
    // checks route through `is_structural_punct` so a string /
    // regex / template whose value looks like a bracket can
    // never trigger the collapse.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 1 < kept.len() {
            if is_structural_punct(&kept[i], "(")
                && is_structural_punct(&kept[i + 1], "(")
            {
                let grouping = match i.checked_sub(1).and_then(|k| kept.get(k)) {
                    None => true,
                    Some(p) => {
                        is_punct(p)
                            && !is_structural_punct(p, ")")
                            && !is_structural_punct(p, "]")
                            && !is_structural_punct(p, "?.")
                    }
                };
                if grouping {
                    // Depth-scan from the inner `(` (at i+1) to
                    // its matching `)`, tracking a top-level
                    // comma.
                    let mut depth: i32 = 1;
                    let mut inner_close: Option<usize> = None;
                    let mut has_comma = false;
                    let mut j = i + 2;
                    while j < kept.len() {
                        let t = &kept[j];
                        if is_structural_punct(t, "(")
                            || is_structural_punct(t, "[")
                            || is_structural_punct(t, "{")
                        {
                            depth += 1;
                        } else if is_structural_punct(t, ")") {
                            depth -= 1;
                            if depth == 0 {
                                inner_close = Some(j);
                                break;
                            }
                        } else if is_structural_punct(t, "]")
                            || is_structural_punct(t, "}")
                        {
                            depth -= 1;
                        } else if depth == 1 && is_structural_punct(t, ",") {
                            has_comma = true;
                        }
                        j += 1;
                    }
                    if let Some(jc) = inner_close {
                        let outer_close_follows = kept
                            .get(jc + 1)
                            .map(|t| is_structural_punct(t, ")"))
                            .unwrap_or(false);
                        if outer_close_follows && !has_comma {
                            drops.push(i); // outer `(`
                            drops.push(jc + 1); // outer `)`
                            i = jc + 2;
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-061: arg-bearing new-expression member wrap -----
    // `new A(y).b` → `(new A(y)).b`. Like gap-059/060, a
    // NewExpression that is the object/callee of a following
    // `.`/`[`/`(` must be wrapped in parens. But here the arg
    // list is NON-EMPTY (`(y)`), so we can't REORDER the empty
    // parens — there are none spare. Instead we INSERT a
    // synthetic `(` before `new` and a synthetic `)` after the
    // arg-list's `)`.
    //
    // Match shape: `new <callee> ( <non-empty args> ) FOLLOWER`
    //   - callee: identifier + zero-or-more `.IDENT` (same scan
    //     as gap-060),
    //   - NON-EMPTY args (at least one token between `(` `)` —
    //     the empty case is gap-059/060's reorder),
    //   - FOLLOWER ∈ {`.`,`[`,`(`}.
    //
    // Guards: operator `new` only (not `.new`/`?.new`); all
    // bracket/accessor checks via `is_structural_punct`; the
    // arg-list close is found by a depth-balanced scan so nested
    // calls (`new A(f(x)).b`) are handled.
    if let (Some(so), Some(sc)) = (synth_open.as_ref(), synth_close.as_ref()) {
        let mut i = 0;
        while i + 1 < kept.len() {
            let is_operator_new = kept[i].value == "new"
                && !is_string_literal(kept[i])
                && (i == 0
                    || (!is_structural_punct(&kept[i - 1], ".")
                        && !is_structural_punct(&kept[i - 1], "?.")));
            if is_operator_new
                && is_simple_identifier_token(kept.get(i + 1).copied())
            {
                // Scan the callee extent (identifier + `.IDENT`).
                let mut p = i + 2;
                while p + 1 < kept.len()
                    && is_structural_punct(&kept[p], ".")
                    && is_simple_identifier_token(Some(kept[p + 1]))
                {
                    p += 2;
                }
                // `kept[p]` must be the arg-list `(` and the
                // first arg token must NOT be `)` (non-empty).
                if p + 1 < kept.len()
                    && is_structural_punct(&kept[p], "(")
                    && !is_structural_punct(&kept[p + 1], ")")
                {
                    // Depth-balanced scan for the matching `)`.
                    let mut depth: i32 = 1;
                    let mut close: Option<usize> = None;
                    let mut j = p + 1;
                    while j < kept.len() {
                        let t = &kept[j];
                        if is_structural_punct(t, "(")
                            || is_structural_punct(t, "[")
                            || is_structural_punct(t, "{")
                        {
                            depth += 1;
                        } else if is_structural_punct(t, ")") {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(j);
                                break;
                            }
                        } else if is_structural_punct(t, "]")
                            || is_structural_punct(t, "}")
                        {
                            depth -= 1;
                        }
                        j += 1;
                    }
                    if let Some(q) = close {
                        let follower_wraps = matches!(
                            kept.get(q + 1).map(|t| t.value.as_str()),
                            Some(".") | Some("[") | Some("(")
                        ) && kept
                            .get(q + 1)
                            .map(|t| is_structural_punct(t, ".")
                                || is_structural_punct(t, "[")
                                || is_structural_punct(t, "("))
                            .unwrap_or(false);
                        if follower_wraps {
                            // Insert `)` after the arg-list close
                            // FIRST (higher index, no shift to
                            // earlier positions), then `(` before
                            // `new`.
                            kept.insert(q + 1, sc);
                            kept.insert(i, so);
                            // `( new <callee> ( args ) )` now
                            // spans i..=q+2; the follower sits at
                            // q+3. Re-scan from there.
                            i = q + 3;
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }
    }

    // ---- gap-067: labeled single-statement block flatten ------
    // `label:{break label}` → `label:break label;`. A labeled
    // statement whose body is a single-statement block drops the
    // braces.
    //
    // PROVABLY-SAFE minimal slice (matches the byte-identity
    // fixture `minify_label_block_flatten`):
    //   - the label `IDENT :` must sit at a hard STATEMENT
    //     boundary — the token before the identifier is `;`, `}`,
    //     or start-of-stream. `{` is DELIBERATELY EXCLUDED: a `{`
    //     is ambiguous (block vs object literal), so an inner
    //     `x:{...}` of an object (`{x:{break:1}}`, whose `x` is
    //     preceded by the object's `{`) must never be touched.
    //   - the block body's FIRST token is a COMPLETION keyword:
    //     `break`/`continue`/`return`/`throw`. These are
    //     unambiguously STATEMENTS — never an object value, never
    //     a `let`/`const`/`class`/`function` declaration — which
    //     proves the `{` is a block and `IDENT:` is a label.
    //   - the block is a SINGLE statement: no top-level `;`
    //     separates a second statement (a lone trailing `;`
    //     before `}` is fine).
    //
    // The `{`/`}` pair is then dropped; the emit loop supplies
    // the statement-terminating `;`. Multi-statement bodies
    // (`label:{a();break label}`) and non-completion-keyword
    // bodies keep their braces. All bracket checks route through
    // `is_structural_punct` so a literal can never be mistaken
    // for a delimiter.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 3 < kept.len() {
            let at_stmt_boundary = match i.checked_sub(1).and_then(|k| kept.get(k)) {
                None => true,
                Some(p) => {
                    is_structural_punct(p, ";") || is_structural_punct(p, "}")
                }
            };
            if at_stmt_boundary
                && is_plain_identifier(&kept[i])
                && is_structural_punct(&kept[i + 1], ":")
                && is_structural_punct(&kept[i + 2], "{")
                && !is_string_literal(&kept[i + 3])
                && matches!(
                    kept[i + 3].value.as_str(),
                    "break" | "continue" | "return" | "throw"
                )
            {
                // Depth-scan to the matching `}`, flagging a
                // statement-separating top-level `;` (one
                // followed by something other than the `}`).
                let mut depth: i32 = 1;
                let mut close: Option<usize> = None;
                let mut multi = false;
                let mut j = i + 3;
                while j < kept.len() {
                    let t = &kept[j];
                    if is_structural_punct(t, "{")
                        || is_structural_punct(t, "(")
                        || is_structural_punct(t, "[")
                    {
                        depth += 1;
                    } else if is_structural_punct(t, "}") {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(j);
                            break;
                        }
                    } else if is_structural_punct(t, ")")
                        || is_structural_punct(t, "]")
                    {
                        depth -= 1;
                    } else if depth == 1 && is_structural_punct(t, ";") {
                        let next_is_close = kept
                            .get(j + 1)
                            .map(|n| is_structural_punct(n, "}"))
                            .unwrap_or(false);
                        if !next_is_close {
                            multi = true;
                        }
                    }
                    j += 1;
                }
                if let Some(c) = close {
                    if !multi {
                        // Drop the opening `{`. The closing `}`
                        // becomes the statement terminator: if the
                        // body already ends in a top-level `;`,
                        // just drop the `}`; otherwise REPLACE the
                        // `}` in place with a synthetic `;` so the
                        // flattened statement stays terminated
                        // (`label:break label` → `label:break
                        // label;`).
                        let body_ends_with_semi = c
                            .checked_sub(1)
                            .and_then(|k| kept.get(k))
                            .map(|t| is_structural_punct(t, ";"))
                            .unwrap_or(false);
                        drops.push(i + 2); // block `{`
                        if body_ends_with_semi {
                            drops.push(c); // block `}`
                        } else if let Some(semi) = synth_semi.as_ref() {
                            kept[c] = semi; // `}` → `;`
                        } else {
                            // No synthetic `;` available (cannot
                            // happen — the stream always has a
                            // token to clone). Bail conservatively.
                            drops.pop();
                            i += 1;
                            continue;
                        }
                        i = c + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-074 + gap-076 + gap-079: header-keyword body flatten -
    // `for(...){S}` / `while(...){S}` / `with(...){S}` / `if(...){S}`
    // whose body `{S}` is a SINGLE statement with NO trailing `;` →
    // `for(...)S;`. Sibling of gap-067 (which flattens a *labeled*
    // block).
    //
    //   l:for(;;){continue l}  →  l:for(;;)continue l;   (gap-074)
    //   for(;;){break}         →  for(;;)break;          (gap-074)
    //   while(x){g()}          →  while(x)g();           (gap-074)
    //   for(a in o){h(a)}      →  for(a in o)h(a);       (gap-074)
    //   with(o){a()}           →  with(o)a();            (gap-076)
    //   if(x){y()}             →  if(x)y();              (gap-079)
    //
    // The `{` immediately following a `for(...)`/`while(...)`/
    // `with(...)`/`if(...)` header is UNAMBIGUOUSLY the statement
    // body — never an object literal — so (unlike gap-067) no
    // completion-keyword guard is needed. The body is dropped braces
    // + a synthetic `;` terminator.
    //
    // gap-079 (the `if` arm) — DANGLING-ELSE SAFETY. Stripping the
    // braces around an `if` consequent is unsound exactly when the
    // body contains a nested un-`else`-d `if` AND the outer `if` has
    // an `else`:  `if(a){if(b)c()}else d()` must KEEP its braces —
    // flattening to `if(a)if(b)c();else d()` would re-bind the `else`
    // to the INNER `if(b)` (the JAR keeps the braces too, verified).
    // We get this for free: any body containing a nested `if` (or any
    // other control-flow keyword) sets `has_blocking_keyword` and is
    // therefore NOT flattened — so the dangling-else case can never
    // reach the brace-drop. A consequent that is a single non-control
    // statement (`{y()}`) has no such hazard. `else`-arm flatten
    // (`else{z()}` → `else z()`) is the separate gap-080.
    //
    // PROVABLY-SAFE minimal slice (matches `minify_loop_body_flatten`,
    // `minify_with_body_flatten`, and `minify_if_body_flatten`):
    //   - anchor on a `for`/`while`/`with`/`if` STATEMENT keyword
    //     (word-like, and NOT a property — `o.while(x){…}` is a
    //     method call, so a `.`/`?.` look-behind disqualifies it);
    //   - the keyword's `(`…`)` header is matched by a structural
    //     depth scan, and the token AFTER `)` must be a `{`;
    //   - the body has NO nested `{` and NO control-flow keyword at
    //     depth 1 (this is also the dangling-else guard for `if`),
    //     and EXACTLY ZERO top-level `;` (i.e. a single,
    //     un-terminated statement). Bodies that already end in `;`
    //     are left to the gap-032 emit-time flatten; multi-statement
    //     bodies (`{a();b()}`) and empty bodies (`{}`) are untouched.
    //
    // All bracket checks route through `is_structural_punct`, so a
    // literal `"{"`/`")"` can never be mistaken for a delimiter.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 1 < kept.len() {
            // gap-076: `with` joins the anchor set. A `with(o){…}`
            // statement has the same `keyword (…) {body}` shape as a
            // loop, and a `{` immediately after the `with(…)` header
            // is unambiguously the with-body — so the identical
            // single-statement flatten applies (`with(o){a()}` →
            // `with(o)a();`).
            let is_loop_kw = is_word_like(&kept[i])
                && matches!(kept[i].value.as_str(), "for" | "while" | "with" | "if");
            let is_property = i >= 1
                && (is_structural_punct(&kept[i - 1], ".")
                    || is_structural_punct(&kept[i - 1], "?."));
            if !(is_loop_kw
                && !is_property
                && is_structural_punct(&kept[i + 1], "("))
            {
                i += 1;
                continue;
            }
            // Match the header `(` … `)`.
            let mut depth: i32 = 1;
            let mut header_close: Option<usize> = None;
            let mut j = i + 2;
            while j < kept.len() {
                let t = &kept[j];
                if is_structural_punct(t, "(")
                    || is_structural_punct(t, "[")
                    || is_structural_punct(t, "{")
                {
                    depth += 1;
                } else if is_structural_punct(t, ")") {
                    depth -= 1;
                    if depth == 0 {
                        header_close = Some(j);
                        break;
                    }
                } else if is_structural_punct(t, "]")
                    || is_structural_punct(t, "}")
                {
                    depth -= 1;
                }
                j += 1;
            }
            let Some(hc) = header_close else {
                i += 1;
                continue;
            };
            // The body must be a `{` immediately after the header.
            let body_open = hc + 1;
            if body_open >= kept.len()
                || !is_structural_punct(&kept[body_open], "{")
            {
                i += 1;
                continue;
            }
            // Scan the body to its matching `}`, gathering
            // eligibility info (no nested brace, no blocking
            // keyword, zero top-level `;`).
            let mut bdepth: i32 = 1;
            let mut body_close: Option<usize> = None;
            let mut has_nested_brace = false;
            let mut has_blocking_keyword = false;
            let mut top_semis: u32 = 0;
            let mut k = body_open + 1;
            while k < kept.len() {
                let t = &kept[k];
                if is_structural_punct(t, "{") {
                    has_nested_brace = true;
                    bdepth += 1;
                } else if is_structural_punct(t, "(")
                    || is_structural_punct(t, "[")
                {
                    bdepth += 1;
                } else if is_structural_punct(t, "}") {
                    bdepth -= 1;
                    if bdepth == 0 {
                        body_close = Some(k);
                        break;
                    }
                } else if is_structural_punct(t, ")")
                    || is_structural_punct(t, "]")
                {
                    bdepth -= 1;
                } else if bdepth == 1 {
                    if is_structural_punct(t, ";") {
                        top_semis += 1;
                    } else if is_word_like(t)
                        && matches!(
                            t.value.as_str(),
                            "function"
                                | "try"
                                | "if"
                                | "while"
                                | "for"
                                | "do"
                                | "switch"
                                | "class"
                        )
                    {
                        has_blocking_keyword = true;
                    }
                }
                k += 1;
            }
            if let Some(bc) = body_close {
                let non_empty = bc > body_open + 1;
                if non_empty
                    && !has_nested_brace
                    && !has_blocking_keyword
                    && top_semis == 0
                {
                    if let Some(semi) = synth_semi.as_ref() {
                        drops.push(body_open); // loop-body `{`
                        kept[bc] = semi; // `}` → synthetic `;`
                        i = bc + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
        }
    }

    // ---- gap-080: else-body single-statement block flatten ------
    // `else{S}` whose body is a SINGLE un-terminated statement →
    // `else S;`. The `else`-arm sibling of gap-079 (if-body flatten).
    //
    //   if(x)a();else{b()}  →  if(x)a();else b();
    //
    // Unlike gap-074/079 the `else` keyword has NO `(…)` header — its
    // body `{` follows IMMEDIATELY. An `else` directly followed by `{`
    // is UNAMBIGUOUSLY the alternate block: `else` is a reserved word
    // (so `else{…}` can never be an object literal or a labelled
    // block), and the only grammar that admits `else { … }` is the
    // alternate of an `if`. `else if(…)` is NOT matched here (the
    // token after `else` is `if`, not `{`) — its inner consequent is
    // flattened by the gap-079 `if` arm instead.
    //
    // Same provably-safe minimal slice as gap-074/079 (matches
    // `minify_else_body_flatten`): the body has NO nested `{`, NO
    // control-flow keyword at depth 1, and EXACTLY ZERO top-level `;`
    // (a single un-terminated statement). Multi-statement bodies
    // (`else{a();b()}`) and empty bodies (`else{}`) keep their braces;
    // a body containing a nested control-flow keyword (`else{if(y)…}`)
    // is conservatively left for a follow-up (output stays valid).
    // The body is dropped braces + a synthetic `;` terminator,
    // reusing gap-067's `synth_semi`.
    {
        let mut drops: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + 1 < kept.len() {
            let is_else =
                is_word_like(&kept[i]) && kept[i].value.as_str() == "else";
            // `else` is reserved, so a `.else`/`?.else` member access
            // can never legally be followed by a `{` block — but keep
            // the look-behind for symmetry with gap-074's guard.
            let is_property = i >= 1
                && (is_structural_punct(&kept[i - 1], ".")
                    || is_structural_punct(&kept[i - 1], "?."));
            if !(is_else
                && !is_property
                && is_structural_punct(&kept[i + 1], "{"))
            {
                i += 1;
                continue;
            }
            // The body `{` follows the `else` directly (no header).
            let body_open = i + 1;
            // Scan the body to its matching `}`, gathering the same
            // eligibility info as gap-074.
            let mut bdepth: i32 = 1;
            let mut body_close: Option<usize> = None;
            let mut has_nested_brace = false;
            let mut has_blocking_keyword = false;
            let mut top_semis: u32 = 0;
            let mut k = body_open + 1;
            while k < kept.len() {
                let t = &kept[k];
                if is_structural_punct(t, "{") {
                    has_nested_brace = true;
                    bdepth += 1;
                } else if is_structural_punct(t, "(")
                    || is_structural_punct(t, "[")
                {
                    bdepth += 1;
                } else if is_structural_punct(t, "}") {
                    bdepth -= 1;
                    if bdepth == 0 {
                        body_close = Some(k);
                        break;
                    }
                } else if is_structural_punct(t, ")")
                    || is_structural_punct(t, "]")
                {
                    bdepth -= 1;
                } else if bdepth == 1 {
                    if is_structural_punct(t, ";") {
                        top_semis += 1;
                    } else if is_word_like(t)
                        && matches!(
                            t.value.as_str(),
                            "function"
                                | "try"
                                | "if"
                                | "while"
                                | "for"
                                | "do"
                                | "switch"
                                | "class"
                        )
                    {
                        has_blocking_keyword = true;
                    }
                }
                k += 1;
            }
            if let Some(bc) = body_close {
                let non_empty = bc > body_open + 1;
                if non_empty
                    && !has_nested_brace
                    && !has_blocking_keyword
                    && top_semis == 0
                {
                    if let Some(semi) = synth_semi.as_ref() {
                        drops.push(body_open); // else-body `{`
                        kept[bc] = semi; // `}` → synthetic `;`
                        i = bc + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        drops.sort_unstable();
        for &drop_idx in drops.iter().rev() {
            kept.remove(drop_idx);
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
            // gap-064: the close-paren check MUST go through
            // `is_structural_punct`, not a bare `.value == ")"`.
            // A string/regex/template literal stores its CONTENT
            // in `.value` (delimiters stripped), so a one-char
            // string `")"` has `.value == ")"`. Without this
            // guard `new A(")")` (a NON-empty arg list whose sole
            // arg is the string `")"`) was misread as the empty
            // arg list `new A()`, dropping the `(` and the string
            // and leaving a stray real `)` — `new A);` (mangled,
            // invalid JS). `is_structural_punct` matches only a
            // genuine `)` punctuator token, so a string argument
            // can never trigger the empty-paren elision.
            && kept
                .get(idx + 1)
                .map(|t| is_structural_punct(t, ")"))
                .unwrap_or(false)
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
        //
        // gap-094 (CORRECTNESS): a trailing comma is droppable
        // ONLY when it follows a REAL element. When it follows a
        // HOLE — a preceding `,` (`[1,,]`) or the opening `[`
        // (`[,]`) — the comma is load-bearing: `[1,,]` has length
        // 2 (element + one trailing hole), and dropping the last
        // comma yields `[1,]` (length 1), silently changing the
        // array. So we additionally require the token BEFORE the
        // comma to be a value-producing element, i.e. NOT a `,`
        // and NOT a `[`. (Both checks route through
        // `is_structural_punct` so a string/regex literal whose
        // CONTENT is `,`/`[` is treated as a real element, not a
        // hole marker.)
        if val == ","
            && kept.get(idx + 1).map(|t| t.value.as_str()) == Some("]")
            && idx > 0
            && !is_structural_punct(kept[idx - 1], ",")
            && !is_structural_punct(kept[idx - 1], "[")
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
            if needs_separator(prev, tok)
                || new_paren_needs_space(&kept, idx)
                || get_set_computed_needs_space(&kept, idx)
                || async_gen_method_needs_space(&kept, idx)
            {
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
                // gap-052: extend trailing-`;` to Other-blocks
                // at EOF. Upstream Closure emits `;` after ANY
                // top-level `}` at EOF (`if(x){...};`,
                // `foo:{...};`, `for(;;){...};`, bare
                // `{a;b;};`). Mid-stream Other-`}` blocks
                // still don't get a `;` — that would change
                // statement boundaries inside expressions or
                // produce stray `;` inside multi-statement
                // sequences.
                BlockKind::Other => next_val.is_none(),
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
        // First strip any `_` separators (gap-048) — they are pure
        // lexical sugar regardless of radix.
        let cleaned_body = if body.contains('_') {
            body.replace('_', "")
        } else {
            body.to_string()
        };
        // gap-091: a RADIX BigInt literal (`0xFFn`, `0o17n`, `0b101n`)
        // is canonicalised to its shortest DECIMAL form (`255n`,
        // `15n`, `5n`), exactly as gap-038 does for non-BigInt
        // hex/oct/bin numbers. We parse the radix body into a `u128`
        // (which covers the common range); a BigInt whose magnitude
        // exceeds `u128::MAX` would need real bigint arithmetic and is
        // left verbatim (a residual). A DECIMAL BigInt body has no
        // radix prefix, so it falls through unchanged (already
        // shortest — `255n` stays `255n`).
        let radix_value: Option<u128> = if let Some(rest) = cleaned_body
            .strip_prefix("0x")
            .or_else(|| cleaned_body.strip_prefix("0X"))
        {
            u128::from_str_radix(rest, 16).ok()
        } else if let Some(rest) = cleaned_body
            .strip_prefix("0o")
            .or_else(|| cleaned_body.strip_prefix("0O"))
        {
            u128::from_str_radix(rest, 8).ok()
        } else if let Some(rest) = cleaned_body
            .strip_prefix("0b")
            .or_else(|| cleaned_body.strip_prefix("0B"))
        {
            u128::from_str_radix(rest, 2).ok()
        } else {
            None
        };
        if let Some(n) = radix_value {
            return format!("{}n", n);
        }
        // No radix prefix (decimal BigInt) or out-of-u128 magnitude:
        // emit the separator-stripped body unchanged.
        if body.contains('_') {
            return format!("{}n", cleaned_body);
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
        // Has a `.` or `e`/`E` or other non-integer character.
        //
        // gap-082: a decimal float / scientific literal that
        // denotes a non-negative INTEGER value fitting in u128
        // (`1e3` = 1000, `1.5e10` = 15000000000, `1.0` = 1,
        // `100.00` = 100, `1.23e2` = 123) is routed through the
        // same shortest-form integer logic as a bare integer —
        // decimal vs scientific, uppercase `E`, tie → decimal
        // (`1e3` → `1E3`, `1.5e10` → `15E9`, `1.5e3` → `1500`).
        // `decimal_float_as_u128` returns the exact integer or
        // `None` when the literal is FRACTIONAL (`0.5`, `1e-5`)
        // or its magnitude overflows u128 (`1e100`); those are
        // left as the separator-stripped source (deferred — the
        // full V8 fractional shortest-form needs Grisu/Ryu and
        // is a separate gap).
        //
        // The ES2021 `_` numeric separator is PURELY LEXICAL
        // sugar, so it is stripped regardless (gap-058):
        //   `1_000.5` → `1000.5`,  `1_0e3` → `10E3` (now also
        //   integer-normalised since `1_0e3` = 10000).
        if let Some(n) = decimal_float_as_u128(&cleaned) {
            let decimal = n.to_string();
            let scientific = scientific_form_of(n);
            let slen = scientific.as_ref().map(|s| s.len()).unwrap_or(usize::MAX);
            return if decimal.len() <= slen {
                decimal
            } else {
                scientific.unwrap()
            };
        }
        return cleaned;
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

/// gap-082: interpret a separator-free decimal float / scientific
/// literal as a non-negative INTEGER, when it denotes one exactly
/// and fits in `u128`. Returns `None` for fractional values or
/// magnitudes outside `u128`'s range (callers leave those forms
/// untouched).
///
/// The argument `s` is the literal with every ES2021 `_` separator
/// already removed and NO radix prefix (`0x`/`0o`/`0b`) and NO
/// trailing BigInt `n` — those are handled by earlier branches.
/// There is no sign: a leading `-`/`+` is a *separate* token in the
/// stream, never part of a NUMBER literal.
///
/// ## How an integer value is recovered
///
/// A decimal literal has the shape `INT[.FRAC][(e|E)EXP]`. Writing
/// `digits = INT ++ FRAC` (the significant digits as one integer)
/// and `eff_exp = EXP − len(FRAC)`, the value is exactly
///
/// ```text
///   value = digits × 10^eff_exp
/// ```
///
/// because moving the decimal point right past `len(FRAC)` digits
/// multiplies by `10^len(FRAC)`, which we subtract back out of the
/// explicit exponent. Worked examples:
///
/// | literal   | digits | EXP | FRAC | eff_exp | value        |
/// |-----------|--------|-----|------|---------|--------------|
/// | `1e3`     | `1`    | 3   | ``   | 3       | 1000         |
/// | `1.5e10`  | `15`   | 10  | `5`  | 9       | 15000000000  |
/// | `1.23e2`  | `123`  | 2   | `23` | 0       | 123          |
/// | `100.00`  | `10000`| 0   | `00` | −2      | 100          |
/// | `0.5`     | `5`    | 0   | `5`  | −1      | 0.5 (None)   |
///
/// When `eff_exp ≥ 0` the value is `digits × 10^eff_exp` (an
/// integer, checked for overflow). When `eff_exp < 0` the value is
/// an integer **iff** `digits` is divisible by `10^(−eff_exp)`
/// (`100.00` → 10000 / 100 = 100); otherwise it is genuinely
/// fractional (`0.5`, `1.23`) and we return `None`.
fn decimal_float_as_u128(s: &str) -> Option<u128> {
    // Split off the exponent, if any. `EXP` may carry a sign
    // (`1e-5`), so it is parsed as a *signed* integer.
    let (mantissa, exp): (&str, i32) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i32>().ok()?),
        None => (s, 0),
    };
    // Split the mantissa into integer and fractional digits.
    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };
    // Both halves must be pure ASCII digits — anything else means
    // this isn't a plain decimal literal we can reason about.
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    // The significant digits as one integer. Leading zeros parse
    // fine (`007` → 7); an empty string would not, so guard it.
    let digits_str = format!("{}{}", int_part, frac_part);
    let digits: u128 = digits_str.parse().ok()?;
    let eff_exp = exp.checked_sub(frac_part.len() as i32)?;
    if eff_exp >= 0 {
        let pow = 10u128.checked_pow(eff_exp as u32)?;
        digits.checked_mul(pow)
    } else {
        // `eff_exp` is negative; take its magnitude as u32.
        // `i32::unsigned_abs` is used (NOT `(-eff_exp) as u32`)
        // so that `eff_exp == i32::MIN` — reachable from a crafted
        // literal like `1e-2147483648` — does not overflow-panic
        // (`-(i32::MIN)` is undefined in i32). The huge magnitude
        // then makes `checked_pow` return None, so the literal is
        // left verbatim rather than crashing the compiler.
        let pow = 10u128.checked_pow(eff_exp.unsigned_abs())?;
        if digits % pow == 0 {
            Some(digits / pow)
        } else {
            None
        }
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
    // gap-063: same-sign `+`/`-` adjacency (CORRECTNESS). If the
    // previous operator token ENDS with `+`/`-` and the next
    // operator token STARTS with the SAME sign character, gluing
    // them together would form a spurious compound operator:
    //
    //   `-` `-a`  →  `--a`   (decrement, NOT double negation!)
    //   `+` `+a`  →  `++a`   (increment)
    //   `--` `-a` →  `---a`,  `a` `+` `++b` → `a+++b`, …
    //
    // Insert a space to keep the operators distinct. Different
    // signs (`+` then `-`) are unambiguous (`+-`) and need none.
    //
    // CRITICAL GUARD: both sides must be real PUNCTUATOR tokens
    // (`is_punct`), never string/regex/template literals. A
    // one-char string `"-"` stores `.value == "-"` (delimiters
    // stripped), so without this gate `"a-"` followed by a `-`
    // operator — emitted as `"a-"-…` where the char before `-`
    // is the closing quote, not `-` — would wrongly get a space.
    // `is_punct` excludes literals, so only genuine operators
    // whose emitted text equals `.value` are considered.
    if is_punct(a) && is_punct(b) {
        let a_last = a.value.chars().last();
        let b_first = b.value.chars().next();
        if (a_last == Some('+') && b_first == Some('+'))
            || (a_last == Some('-') && b_first == Some('-'))
        {
            return true;
        }
    }
    false
}

/// gap-069: decide whether the token at `kept[idx]` — known to be
/// the one about to be emitted — needs a single leading space
/// because it is a `(` GROUPING paren immediately following the
/// `new` KEYWORD. Upstream Closure emits `new (a+b)` (with a
/// space), not `new(a+b)`.
///
/// Why this lives HERE (a two-token look-behind) and not in
/// `needs_separator` (which sees only the adjacent pair):
/// distinguishing the genuine NewExpression keyword `new` from a
/// PROPERTY named `new` requires the token *before* `new`. The
/// JavaScript lexer is context-free, so it types `new` identically
/// in `new X` and `o.new(f)` — only the preceding `.`/`?.` member
/// accessor tells them apart:
///
///   `new(a+b)`   → kept[..] = … new ( …      → SPACE  (NewExpr)
///   `o.new(f)`   → kept[..] = … . new ( …     → NO SPACE (method call)
///   `a.b(x)`     → kept[..] = … b ( …         → rule doesn't fire
///
/// The companion case `new(f)()` (simple-reference callee) never
/// reaches here: gap-068's pre-pass has already ELIDED the parens
/// (`new f`) before the emit loop runs, so no `new (` survives.
///
/// GUARDS (all required, fail-closed):
///   - `kept[idx]` is a *structural* `(` (not a one-char string
///     `"("` whose `.value` merely equals `(`).
///   - `kept[idx-1]` is word-like AND its value is exactly `new`
///     (so a string literal `"new"` can never trigger it; `new`
///     is reserved, so no identifier can collide either).
///   - `kept[idx-2]`, if present, is NOT a `.`/`?.` member
///     accessor — i.e. `new` is a real unary keyword, not a
///     property. When `new` is the very first token (idx < 2)
///     there is no accessor, so it is unambiguously a NewExpression.
fn new_paren_needs_space(kept: &[&lexer::token::Token], idx: usize) -> bool {
    if idx == 0 {
        return false; // need a `new` token before the `(`
    }
    let open = kept[idx];
    let prev = kept[idx - 1];
    if !is_structural_punct(open, "(") {
        return false;
    }
    if !(is_word_like(prev) && prev.value == "new") {
        return false;
    }
    // Property guard: reject `o.new(` / `o?.new(` (method call).
    if idx >= 2 {
        let before_new = kept[idx - 2];
        if is_structural_punct(before_new, ".") || is_structural_punct(before_new, "?.") {
            return false;
        }
    }
    true
}

/// gap-073: decide whether the `[` token at `kept[idx]` needs a
/// single leading space because it is the COMPUTED KEY of a
/// `get`/`set` accessor — `{get[k](){…}}` → `{get [k](){…}}`,
/// matching upstream Closure. Without the space `get[k]` re-reads
/// as a member access on a variable named `get` rather than a
/// getter with a computed name.
///
/// The hazard mirrors gap-069: `get`/`set` are *contextual*
/// keywords — accessors only inside an object/class body, plain
/// identifiers everywhere else. The JS lexer types them
/// identically, so two tokens of look-behind plus a forward check
/// are needed to tell a real accessor from member access /
/// variable indexing:
///
///   {get[k](){…}}     → { get [ … ] (   → SPACE    (accessor)
///   {a:1,set[k](v){}} → , set [ … ] (   → SPACE    (accessor)
///   o.get[k];         → . get [ … ] ;   → NO SPACE (member access)
///   get[k](x);        → ^ get [ … ] (   → NO SPACE (var index+call)
///
/// GUARDS (all required, fail-closed):
///   - `kept[idx]` is a structural `[` (not a string `"["`).
///   - `kept[idx-1]` is a word-like `get` or `set` keyword (so a
///     string literal `"get"` can never trigger it).
///   - `kept[idx-2]` is a structural `{` or `,` — an object-literal
///     property-start position. This is the decisive disambiguator:
///     it excludes member access (`.`/`?.`) and statement-level
///     variable indexing (preceded by `;` / start). (Class-body
///     accessors after a previous member — preceded by `}` /
///     `static` — are deferred; the object-literal form is the
///     gap-073 fixture.)
///   - after the matching `]`, the next token is a structural `(` —
///     i.e. the accessor's parameter list. This confirms a
///     method/accessor shape and rejects a bare computed member.
fn get_set_computed_needs_space(kept: &[&lexer::token::Token], idx: usize) -> bool {
    if idx < 2 {
        return false; // need property-start + keyword before `[`
    }
    let open = kept[idx];
    if !is_structural_punct(open, "[") {
        return false;
    }
    let kw = kept[idx - 1];
    if !(is_word_like(kw) && (kw.value == "get" || kw.value == "set")) {
        return false;
    }
    // Property-start context: an object-literal `{` or `,`.
    let before_kw = kept[idx - 2];
    if !(is_structural_punct(before_kw, "{") || is_structural_punct(before_kw, ",")) {
        return false;
    }
    // Find the matching `]` (structural depth scan).
    let mut depth: i32 = 1;
    let mut close: Option<usize> = None;
    let mut j = idx + 1;
    while j < kept.len() {
        let t = kept[j];
        if is_structural_punct(t, "[") {
            depth += 1;
        } else if is_structural_punct(t, "]") {
            depth -= 1;
            if depth == 0 {
                close = Some(j);
                break;
            }
        }
        j += 1;
    }
    // The accessor's parameter list `(` must immediately follow.
    match close {
        Some(c) => kept
            .get(c + 1)
            .is_some_and(|t| is_structural_punct(t, "(")),
        None => false,
    }
}

/// gap-097: True iff the token at `idx` is the `*` of an ASYNC GENERATOR
/// METHOD (`async *m(){}`), so a separating space is needed between the
/// preceding `async` and this `*`.
///
/// The trap is that `async*x` is ALSO valid as a multiplication — `async`
/// is only a contextual keyword, so `a=async*b` means `async * b`. Upstream
/// adds the space ONLY for the method form and leaves the arithmetic form
/// alone:
///
///   o={async*m(){}}      ->  o={async *m(){}}     (method   — space)
///   class A{async*m(){}} ->  class A{async *m(){}}(method   — space)
///   a=async*b            ->  a=async*b            (multiply — no space)
///   a=async*f()          ->  a=async*f()          (multiply — no space)
///   a=b,async*c          ->  a=b,async*c          (multiply — no space)
///   o={async*[x](){}}    ->  o={async*[x](){}}    (computed — no space:
///                                                  `*[` can't merge)
///
/// The reliable discriminator is the full method SIGNATURE: an async
/// generator method is `async * NAME ( <params> ) {` — a *named* method
/// (identifier name, not `[computed]`) with a parameter list AND a body
/// `{`. The multiplication forms above all lack the trailing `){` body
/// (`async*f()` is followed by `;`/operator, never `{`), so checking for
/// it cleanly separates the two. We mirror `get_set_computed_needs_space`'s
/// structural depth-scan to find the param-list's matching `)`.
fn async_gen_method_needs_space(kept: &[&lexer::token::Token], idx: usize) -> bool {
    // This `*` must sit between `async` and an identifier method name.
    if idx == 0 {
        return false;
    }
    if !is_structural_punct(kept[idx], "*") {
        return false;
    }
    let prev = kept[idx - 1];
    if !(is_word_like(prev) && prev.value == "async") {
        return false;
    }
    // The method NAME: a word-like identifier (not `[computed]`, not a
    // string — those forms don't merge with `*` so upstream omits the
    // space).
    let name = match kept.get(idx + 1) {
        Some(t) if is_simple_identifier_token(Some(t)) => t,
        _ => return false,
    };
    let _ = name;
    // The parameter list `(` follows the name.
    if !kept.get(idx + 2).is_some_and(|t| is_structural_punct(t, "(")) {
        return false;
    }
    // Scan for the matching `)` (structural paren depth).
    let mut depth: i32 = 1;
    let mut close: Option<usize> = None;
    let mut j = idx + 3;
    while j < kept.len() {
        let t = kept[j];
        if is_structural_punct(t, "(") {
            depth += 1;
        } else if is_structural_punct(t, ")") {
            depth -= 1;
            if depth == 0 {
                close = Some(j);
                break;
            }
        }
        j += 1;
    }
    // A method BODY `{` must immediately follow the `)`. This is what an
    // arithmetic `async*f()` lacks, so it is the deciding signal.
    match close {
        Some(c) => kept
            .get(c + 1)
            .is_some_and(|t| is_structural_punct(t, "{")),
        None => false,
    }
}

/// gap-054 + gap-070: True iff `span` (the tokens BETWEEN a unary
/// keyword's grouping parens, exclusive of the parens) is a "safe"
/// operand whose parens are pure grouping and can be dropped.
///
/// Two accepted shapes (see the call site for the full rationale):
///
///   1. A SINGLE safe token — an identifier, a numeric literal, or
///      a string literal. (gap-054, the original case.)
///   2. A MEMBER-REFERENCE CHAIN — an identifier base followed by
///      any run of `.name` / `?.name` / `[…]` accessors and nothing
///      else at the top level. (gap-070.)
///
/// Both are self-delimiting and bind tighter than a prefix unary
/// operator, so `OP(span)` ≡ `OP span`. Anything with a top-level
/// binary operator, comma, or call `(` is rejected — there the
/// parens change meaning.
/// gap-075: True iff `span` is a safe operand for a PREFIX SYMBOL
/// unary operator's grouping parens — i.e. `OP(span)` ≡ `OP span`.
///
/// Accepts everything `is_safe_unary_operand` does (a single token
/// or a member-reference chain), PLUS a leading chain of prefix
/// SYMBOL unary operators (`-`/`+`/`!`/`~`) applied to such an
/// operand: `-a`, `!a`, `~a.b`, `- -a`. That extra shape is what
/// makes `-(-a)` → `- -a` strippable — the inner `-a` is itself a
/// UnaryExpression, which `is_safe_unary_operand` alone rejects.
///
/// All shapes are higher-precedence than (or equal to) a unary
/// operator and self-delimiting, so the grouping parens are pure
/// grouping and whatever follows the close paren re-associates the
/// same way. Anything with a top-level BINARY operator, comma, or
/// call is rejected (`is_safe_unary_operand` handles that), so
/// `-(a+b)` correctly keeps its parens. `--`/`++` (decrement /
/// increment) are single tokens whose `.value` is `"--"`/`"++"`,
/// never `"-"`/`"+"`, so they never satisfy the leading-operator
/// test — `-(--a)` is left alone.
/// gap-086 helper. An argument occupies `kept[arg_start..boundary]`
/// (the `boundary` index is the argument's terminating `,` or the
/// call's closing `)`). If that argument is ENTIRELY parenthesised —
/// its first token is `(` whose structural match is exactly the token
/// just before `boundary` — and the parenthesised span carries NO
/// TOP-LEVEL COMMA, the wrapping `(`/`)` indices are pushed onto
/// `drops`. The top-level-comma guard preserves the one load-bearing
/// case `f((a,b))` (a single comma-operator argument that would
/// otherwise resplit into two arguments).
fn maybe_strip_arg_paren(
    kept: &[&lexer::token::Token],
    arg_start: usize,
    boundary: usize,
    drops: &mut Vec<usize>,
) {
    // Need at least `( )` before the boundary.
    if boundary < 2 || arg_start + 1 >= boundary {
        return;
    }
    if !is_structural_punct(kept[arg_start], "(") {
        return;
    }
    // Structural match of the opening `(` at `arg_start`.
    let mut depth: i32 = 1;
    let mut k = arg_start + 1;
    let mut close: Option<usize> = None;
    while k < boundary {
        let t = kept[k];
        if is_structural_punct(t, "(")
            || is_structural_punct(t, "[")
            || is_structural_punct(t, "{")
        {
            depth += 1;
        } else if is_structural_punct(t, ")") {
            depth -= 1;
            if depth == 0 {
                close = Some(k);
                break;
            }
        } else if is_structural_punct(t, "]") || is_structural_punct(t, "}") {
            depth -= 1;
        }
        k += 1;
    }
    let Some(close) = close else { return };
    // The parens must wrap the WHOLE argument: the matching `)` is the
    // token immediately before the boundary.
    if close != boundary - 1 {
        return;
    }
    // Reject a top-level comma inside the span (the `f((a,b))` case).
    let inner = &kept[arg_start + 1..close];
    let mut d: i32 = 0;
    for t in inner {
        if is_structural_punct(t, "(")
            || is_structural_punct(t, "[")
            || is_structural_punct(t, "{")
        {
            d += 1;
        } else if is_structural_punct(t, ")")
            || is_structural_punct(t, "]")
            || is_structural_punct(t, "}")
        {
            d -= 1;
        } else if is_structural_punct(t, ",") && d == 0 {
            return; // top-level comma — keep the parens
        }
    }
    drops.push(arg_start);
    drops.push(close);
}

fn is_safe_unary_paren_operand(span: &[&lexer::token::Token]) -> bool {
    if is_safe_unary_operand(span) {
        return true;
    }
    if let Some((first, rest)) = span.split_first() {
        if !rest.is_empty()
            && (is_structural_punct(first, "-")
                || is_structural_punct(first, "+")
                || is_structural_punct(first, "!")
                || is_structural_punct(first, "~"))
        {
            return is_safe_unary_paren_operand(rest);
        }
    }
    false
}

/// gap-101: True iff `span` (the tokens BETWEEN a unary OPERATOR's
/// grouping parens) is a "safe" operand for that operator — i.e.
/// `OP(span)` ≡ `OP span`. A strict SUPERSET of
/// `is_safe_unary_paren_operand` that additionally accepts the two
/// higher-arity operand shapes upstream also unwraps:
///
///   (a) a leading KEYWORD unary operator (`typeof`/`void`/`delete`)
///       applied recursively to a safe operand:
///         typeof(void 0)     -> typeof void 0
///         typeof(typeof b)   -> typeof typeof b
///         void(void 0)       -> void void 0
///
///   (b) a CALL / member chain — an identifier base followed by any
///       run of `.name` / `?.name` / `[…]` / `(…)` accessors:
///         typeof(b())        -> typeof b()
///         typeof(a.b())      -> typeof a.b()
///
/// Every prefix unary operator (and the binary `instanceof`) binds
/// LOOSER than member access, call, and any prefix unary, so a
/// parenthesised operand that is itself a UnaryExpression or a
/// CallExpression re-associates identically with or without the
/// grouping parens. A parenthesised BINARY operand (`typeof(b+c)`,
/// `void(a,b)`, `typeof(a=b)`) binds looser than the would-be
/// adjacency and is still REJECTED — `is_safe_unary_paren_operand`
/// handles the single-token / member-chain / leading-symbol-unary
/// shapes, the keyword-unary branch below adds (a), and
/// `is_call_ref_chain` adds (b).
fn is_safe_unary_kw_operand(span: &[&lexer::token::Token]) -> bool {
    // Single token, member-reference chain, or a leading SYMBOL
    // prefix-unary chain (`-b`, `!b`, `~a.b`).
    if is_safe_unary_paren_operand(span) {
        return true;
    }
    // (a) A leading KEYWORD unary operator applied to a safe operand.
    // The keyword must be a genuine word-like token (never a string
    // literal `"typeof"` whose `.value` matches) and must be FOLLOWED
    // by more tokens (a bare `typeof` is not an operand).
    if let Some((first, rest)) = span.split_first() {
        if !rest.is_empty()
            && is_word_like(first)
            && matches!(first.value.as_str(), "typeof" | "void" | "delete")
        {
            return is_safe_unary_kw_operand(rest);
        }
    }
    // (b) A call / member reference chain (identifier base + accessors,
    // where the accessors may include a call `(…)`).
    is_call_ref_chain(span)
}

/// gap-101 helper. True iff `span` is an identifier base followed by
/// a (possibly empty) run of member / call accessors — `.name`,
/// `?.name`, `[…]`, `(…)` — and NOTHING else at the top level. This
/// is the `is_safe_unary_operand` shape-2 reference-chain walk
/// EXTENDED to also accept call accessors `(…)`, so `b()` / `a.b()` /
/// `a().b` / `a[0]()` qualify as self-delimiting operands of a unary
/// operator. A top-level binary operator, comma, or assignment makes
/// it return false (the grouping parens are then meaningful).
fn is_call_ref_chain(span: &[&lexer::token::Token]) -> bool {
    if span.is_empty() {
        return false;
    }
    let base = span[0];
    if is_string_literal(base) || !is_word_like(base) {
        return false;
    }
    if !base
        .value
        .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
    {
        return false;
    }
    let mut k = 1;
    while k < span.len() {
        let t = span[k];
        if is_structural_punct(t, ".") || is_structural_punct(t, "?.") {
            let Some(name) = span.get(k + 1) else {
                return false;
            };
            if is_string_literal(name) || !is_word_like(name) {
                return false;
            }
            k += 2;
        } else if is_structural_punct(t, "[") || is_structural_punct(t, "(") {
            // Balanced `[ … ]` / `( … )` accessor. A depth scan over
            // ALL bracket kinds keeps a nested mismatched pair from
            // ending the accessor early; well-formed JS guarantees the
            // matching close has the right kind. An empty `()` call is
            // valid (unlike an empty `[]` subscript), so no emptiness
            // check is applied here.
            let mut depth: i32 = 1;
            let mut j = k + 1;
            while j < span.len() {
                let u = span[j];
                if is_structural_punct(u, "(")
                    || is_structural_punct(u, "[")
                    || is_structural_punct(u, "{")
                {
                    depth += 1;
                } else if is_structural_punct(u, ")")
                    || is_structural_punct(u, "]")
                    || is_structural_punct(u, "}")
                {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            if depth != 0 {
                return false; // unbalanced
            }
            k = j + 1;
        } else {
            // Top-level operator / comma / assignment → not a pure
            // call-reference chain; the parens carry meaning.
            return false;
        }
    }
    true
}

fn is_safe_unary_operand(span: &[&lexer::token::Token]) -> bool {
    if span.is_empty() {
        return false;
    }
    // Shape 1: a single safe token (identifier / number / string).
    if span.len() == 1 {
        let t = span[0];
        if is_string_literal(t) {
            return true;
        }
        let v = t.value.as_str();
        return !v.is_empty()
            && v.starts_with(|c: char| {
                c.is_ascii_alphabetic()
                    || c == '_'
                    || c == '$'
                    || c.is_ascii_digit()
            });
    }
    // Shape 2: a member-reference chain. The base must be a plain
    // word-like identifier/keyword token (`a`, `this`) — never a
    // string, number, or operator. (A number base like `1` is
    // rejected: `1.toString` would re-lex the `.` as part of the
    // number.)
    let base = span[0];
    if is_string_literal(base) || !is_word_like(base) {
        return false;
    }
    if !base
        .value
        .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
    {
        return false;
    }
    // Walk the accessors after the base.
    let mut k = 1;
    while k < span.len() {
        let t = span[k];
        if is_structural_punct(t, ".") || is_structural_punct(t, "?.") {
            // A `.`/`?.` must be followed by a property name — a
            // word-like identifier or keyword (`a.if` is legal).
            let Some(name) = span.get(k + 1) else {
                return false;
            };
            if is_string_literal(name) || !is_word_like(name) {
                return false;
            }
            k += 2;
        } else if is_structural_punct(t, "[") {
            // Balanced computed-member subscript `[ … ]`. The
            // contents are a complete sub-expression; we only
            // require the brackets balance and are non-empty.
            let mut depth: i32 = 1;
            let mut j = k + 1;
            while j < span.len() {
                let u = span[j];
                if is_structural_punct(u, "[") {
                    depth += 1;
                } else if is_structural_punct(u, "]") {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            if depth != 0 || j == k + 1 {
                return false; // unbalanced or empty `[]`
            }
            k = j + 1;
        } else {
            // Top-level operator, call `(`, comma, etc. → not a
            // pure reference chain; the parens are meaningful.
            return false;
        }
    }
    true
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
/// True iff `tok` is a *structural punctuator* whose `.value`
/// equals `val` — i.e. a real operator/bracket token and NOT a
/// literal that merely happens to CONTAIN that character.
///
/// This matters because a string/regex/template literal stores
/// its *content* in `.value` (the lexer strips the delimiters),
/// so a one-character string `")"` has `.value == ")"`. Any
/// token-level peephole that depth-tracks brackets by comparing
/// `.value` against `"("` / `")"` / `","` MUST gate on this, or
/// a bracket char inside a literal corrupts the depth count and
/// the wrong token gets matched/removed.
///
/// Word-like tokens (REGEX, TEMPLATE*, BIGINT, NAME, …) and
/// string literals are the only kinds that can smuggle a bracket
/// character into `.value`; excluding both leaves exactly the
/// punctuator tokens.
fn is_structural_punct(tok: &lexer::token::Token, val: &str) -> bool {
    tok.value == val && !is_string_literal(tok) && !is_word_like(tok)
}

/// True for any punctuation/operator token — i.e. a token that is
/// neither word-like (identifier/number/keyword/regex/template/…)
/// nor a string literal. We use this to recognise the *category*
/// of a token (operator vs value) without caring which operator
/// it is. Like `is_structural_punct`, it deliberately rejects
/// string literals whose `.value` happens to look like an
/// operator (e.g. a one-char string `"+"`).
fn is_punct(tok: &lexer::token::Token) -> bool {
    !is_word_like(tok) && !is_string_literal(tok)
}

/// gap-057: True only for a *plain identifier* token (`a`, `_x`,
/// `$y`, `#priv`) — the one shape that is unconditionally safe to
/// expose as a bare member-expression object when its grouping
/// parens are stripped (`(a).b` → `a.b`).
///
/// Deliberately EXCLUDES:
///   - NUMBER — `(1).toString()` must NOT become `1.toString()`
///     (`1.` lexes as a malformed number; the `.` would be eaten).
///   - KEYWORD — `this`/`super`/etc. are safe in principle but
///     out of scope for the minimal fixture; left for a follow-up.
///   - REGEX / TEMPLATE / STRING — their `.value` carries content,
///     and member access on them is a separate concern.
fn is_plain_identifier(tok: &lexer::token::Token) -> bool {
    if is_string_literal(tok) {
        return false;
    }
    if let Some(name) = &tok.type_name {
        let upper = name.to_ascii_uppercase();
        return matches!(
            upper.as_str(),
            "NAME" | "IDENT" | "IDENTIFIER" | "PRIVATE_NAME"
        );
    }
    matches!(tok.type_, lexer::token::TokenType::Name)
}

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

    // ---- gap-063: same-sign `+`/`-` adjacency (CORRECTNESS) ----

    /// gap-063: `- -a` (double negation) must NOT collapse to
    /// `--a` (pre-decrement) — a different program.
    #[test]
    fn gap063_double_minus_keeps_space() {
        assert_eq!(minify("var x=- -a;"), "var x=- -a;");
    }

    /// gap-063: `+ +a` must NOT collapse to `++a`.
    #[test]
    fn gap063_double_plus_keeps_space() {
        assert_eq!(minify("var x=+ +a;"), "var x=+ +a;");
    }

    /// gap-063: binary minus then unary minus — `a- -b` must stay
    /// `a- -b`, not become `a--b`.
    #[test]
    fn gap063_binary_then_unary_minus() {
        assert_eq!(minify("var x=a- -b;"), "var x=a- -b;");
    }

    /// gap-063: `- --a` (negate a pre-decrement) keeps the space
    /// between the `-` and `--` operators.
    #[test]
    fn gap063_minus_then_predecrement() {
        assert_eq!(minify("var x=- --a;"), "var x=- --a;");
    }

    /// gap-063: DIFFERENT signs are unambiguous — `a+ -b` joins
    /// to `a+-b` (no spurious operator possible).
    #[test]
    fn gap063_different_signs_join() {
        assert_eq!(minify("var x=a+ -b;"), "var x=a+-b;");
    }

    /// gap-063 GUARD: a string literal whose content ends in `-`
    /// must NOT trigger the rule against a following `-` operator
    /// — the emitted char before the operator is the closing
    /// quote, not `-`. `"a-"-1` stays `"a-"-1` (no spurious
    /// space). This is why the rule gates on `is_punct`.
    #[test]
    fn gap063_string_ending_in_sign_not_spaced() {
        assert_eq!(minify("var x=\"a-\"-1;"), "var x=\"a-\"-1;");
        // ...but the two REAL `-` operators after it still space:
        assert_eq!(minify("var y=\"a-\"- -b;"), "var y=\"a-\"- -b;");
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
            "if(x){a();b()};"
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
            "if(x){if(y)a()};"
        );
    }

    /// Body containing a nested `{}` block MUST NOT flatten.
    /// `has_nested_brace` catches this.
    #[test]
    fn gap032_nested_brace_does_not_flatten() {
        assert_eq!(
            minify("if(x){{a();}}"),
            "if(x){{a()}};"
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
        assert_eq!(minify("{a;}"), "{a};");
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
            "var o={switch:1};while(x){a;b};"
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
    /// gap-091 (was gap-038's deferred BigInt future): a hex BigInt is
    /// now decimalised — `0xfn` → `15n`.
    #[test]
    fn gap038_bigint_left_verbatim() {
        assert_eq!(minify("var x=0xfn;"), "var x=15n;");
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

    /// gap-048 + gap-091: a hex BigInt with a `_` separator is both
    /// separator-stripped AND decimalised — `0x1_FFFn` → `8191n`
    /// (0x1FFF = 8191).
    #[test]
    fn gap048_bigint_hex_separator_stripped() {
        assert_eq!(
            minify("var a=0x1_FFFn;"),
            "var a=8191n;"
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

    /// gap-091: a hex BigInt without separators is decimalised, just
    /// like regular `0xff` → `255` (gap-038): `0xfn` → `15n`. (Was
    /// deferred under gap-048; closed by CLOC12.96.)
    #[test]
    fn gap048_bigint_hex_no_separator_unchanged() {
        assert_eq!(minify("var x=0xfn;"), "var x=15n;");
    }

    /// gap-091: BigInt radix literals canonicalise to decimal across
    /// all three radices (hex/oct/bin), case-insensitive prefix.
    #[test]
    fn gap091_bigint_radix_to_decimal() {
        assert_eq!(minify("var x=0xFFn;"), "var x=255n;");
        assert_eq!(minify("var x=0XFFn;"), "var x=255n;");
        assert_eq!(minify("var x=0o17n;"), "var x=15n;");
        assert_eq!(minify("var x=0b101n;"), "var x=5n;");
        assert_eq!(minify("var x=0n;"), "var x=0n;");
    }

    /// gap-091 non-regression: a DECIMAL BigInt (no radix prefix) is
    /// left as-is — it is already shortest. A magnitude beyond u128
    /// stays verbatim (real bigint arithmetic is a residual).
    #[test]
    fn gap091_decimal_and_overflow_bigint_kept() {
        assert_eq!(minify("var x=255n;"), "var x=255n;");
        assert_eq!(
            minify("var x=9007199254740993n;"),
            "var x=9007199254740993n;"
        );
        // 35 hex digits = 140 bits > u128 — left verbatim.
        assert_eq!(
            minify("var x=0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFn;"),
            "var x=0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFn;"
        );
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

    // ---- gap-058: numeric separator in float literals ----

    /// gap-058: a `_` separator in a FLOAT literal is purely
    /// lexical sugar and must be stripped, even though full
    /// float shortest-form (`0.5` → `.5`) stays deferred.
    /// `1_000.5` → `1000.5` (matches upstream).
    #[test]
    fn gap058_float_separator_stripped() {
        assert_eq!(minify("var x=1_000.5;"), "var x=1000.5;");
    }

    /// gap-058 + gap-082: separator in the mantissa of a
    /// scientific literal is stripped, and since `1_0e3` =
    /// `10e3` = 10000 is an INTEGER value it is now further
    /// canonicalised to shortest scientific form (`1E4`) by
    /// gap-082. JAR-verified: `1_0e3` → `1E4`. (Before gap-082
    /// the float branch only stripped the separator and left
    /// `10e3`, which was never checked against the JAR and was
    /// in fact wrong — the JAR always emits `1E4`.)
    #[test]
    fn gap058_scientific_mantissa_separator_stripped() {
        assert_eq!(minify("var x=1_0e3;"), "var x=1E4;");
    }

    /// gap-058 non-regression: a float with NO separator is
    /// returned verbatim (no spurious shortest-form rewrite).
    #[test]
    fn gap058_plain_float_unchanged() {
        assert_eq!(minify("var x=3.14;"), "var x=3.14;");
    }

    // ---- gap-082: integer-valued float/scientific canon ----
    //
    // A decimal float / scientific literal that denotes a
    // non-negative INTEGER fitting in u128 is routed through the
    // shortest-form integer logic (decimal vs uppercase-`E`
    // scientific, tie → decimal). Every assertion below is
    // JAR-verified (`--compilation_level WHITESPACE_ONLY`).
    // Fractional values and >u128 magnitudes stay deferred.

    /// THE fixture: `1e3` = 1000 → sci `1E3` (3) beats decimal
    /// `1000` (4). Lowercase `e` is normalised to uppercase.
    #[test]
    fn gap082_exp_to_uppercase_scientific() {
        assert_eq!(minify("var x=1e3;"), "var x=1E3;");
    }

    /// `1.0` = 1 → `1` (trailing `.0` dropped; sci helper
    /// returns None for e==0 so decimal wins).
    #[test]
    fn gap082_trailing_dot_zero_dropped() {
        assert_eq!(minify("var x=1.0;"), "var x=1;");
    }

    /// `1.5e10` = 15000000000 → sci `15E9` (4) beats the
    /// 11-digit decimal. Mantissa fractional digit folds into
    /// the exponent: `15 × 10^9`.
    #[test]
    fn gap082_fractional_mantissa_scientific() {
        assert_eq!(minify("var x=1.5e10;"), "var x=15E9;");
    }

    /// `1.23e2` = 123 → plain decimal `123` (sci `123E0` would
    /// be longer; helper returns None for e==0).
    #[test]
    fn gap082_scientific_resolves_to_small_int() {
        assert_eq!(minify("var x=1.23e2;"), "var x=123;");
    }

    /// `100.00` = 100 → `100`. eff_exp is NEGATIVE (−2) but the
    /// digits `10000` are divisible by `10^2`, so the value is
    /// an exact integer.
    #[test]
    fn gap082_trailing_zeros_after_point() {
        assert_eq!(minify("var x=100.00;"), "var x=100;");
    }

    /// `1.5e3` = 1500 → decimal `1500` (4) ties sci `15E2` (4);
    /// tie breaks to decimal.
    #[test]
    fn gap082_tie_breaks_to_decimal() {
        assert_eq!(minify("var x=1.5e3;"), "var x=1500;");
    }

    /// `1e21` = 10^21 fits in u128 (< 3.4e38) → `1E21`.
    #[test]
    fn gap082_large_but_in_range() {
        assert_eq!(minify("var x=1e21;"), "var x=1E21;");
    }

    /// **Deferred (residual gap-085)**: a FRACTIONAL value is
    /// left as the separator-stripped source. `0.5` is not an
    /// integer (5 not divisible by 10), so `decimal_float_as_u128`
    /// returns None and the literal is unchanged. (The JAR emits
    /// `.5`; matching it needs the V8 fractional formatter.)
    #[test]
    fn gap082_fractional_left_verbatim() {
        assert_eq!(minify("var x=0.5;"), "var x=0.5;");
    }

    /// **Deferred (residual gap-085)**: a negative-exponent
    /// (sub-1) value stays verbatim. `1e-5` = 0.00001 is not an
    /// integer.
    #[test]
    fn gap082_negative_exponent_left_verbatim() {
        assert_eq!(minify("var x=1e-5;"), "var x=1e-5;");
    }

    /// **Deferred**: a magnitude beyond u128 overflows the
    /// `checked_pow`/`checked_mul` and is left verbatim. `1e100`
    /// = 10^100 ≫ u128::MAX.
    #[test]
    fn gap082_overflow_left_verbatim() {
        assert_eq!(minify("var x=1e100;"), "var x=1e100;");
    }

    /// `decimal_float_as_u128` unit checks — exercise the helper
    /// directly across the boundary cases, including the ones
    /// the lexer never feeds it as a single NUMBER token (`5.`).
    #[test]
    fn gap082_helper_recovers_integers() {
        assert_eq!(decimal_float_as_u128("1e3"), Some(1000));
        assert_eq!(decimal_float_as_u128("1.5e10"), Some(15_000_000_000));
        assert_eq!(decimal_float_as_u128("1.0"), Some(1));
        assert_eq!(decimal_float_as_u128("100.00"), Some(100));
        assert_eq!(decimal_float_as_u128("5."), Some(5));
        assert_eq!(decimal_float_as_u128("1.23e2"), Some(123));
    }

    /// `decimal_float_as_u128` returns None for fractional and
    /// out-of-range inputs.
    #[test]
    fn gap082_helper_rejects_non_integers() {
        assert_eq!(decimal_float_as_u128("0.5"), None);
        assert_eq!(decimal_float_as_u128("1e-5"), None);
        assert_eq!(decimal_float_as_u128("1.23"), None);
        assert_eq!(decimal_float_as_u128("1e100"), None);
    }

    /// **Security regression**: pathological exponents must not
    /// panic (DoS-by-crash on crafted JS input). `1e-2147483648`
    /// parses to `exp == i32::MIN`; the old `(-eff_exp) as u32`
    /// overflow-panicked on it. `1e2147483647` / `1e99999999999`
    /// exercise the positive over-range and the i32-overflowing
    /// exponent parse. All must return None (left verbatim), not
    /// crash, and `minify` must round-trip the source unchanged.
    #[test]
    fn gap082_pathological_exponents_do_not_panic() {
        assert_eq!(decimal_float_as_u128("1e-2147483648"), None);
        assert_eq!(decimal_float_as_u128("1e2147483647"), None);
        assert_eq!(decimal_float_as_u128("1e99999999999"), None);
        assert_eq!(minify("var x=1e-2147483648;"), "var x=1e-2147483648;");
        assert_eq!(minify("var x=1e99999999999;"), "var x=1e99999999999;");
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

    /// `(x)` followed by `.` is a member access on a
    /// parenthesised expression — NOT an arrow (the token at
    /// idx+3 is `.`, not `=>`), so gap-045's arrow logic must
    /// leave it alone. As of gap-057 (CLOC12.66), the SEPARATE
    /// member-object pre-pass legitimately strips the redundant
    /// grouping parens around the single identifier `a`, so the
    /// canonical output is now `var x=a.b;` (matching upstream).
    #[test]
    fn gap057_member_object_paren_stripped() {
        assert_eq!(minify("var x=(a).b;"), "var x=a.b;");
    }

    // ---- gap-065: callee paren elision -------------------

    /// gap-065: `(f)(x)` → `f(x)` — parens around a bare
    /// identifier callee are redundant.
    #[test]
    fn gap065_call_callee_paren_stripped() {
        assert_eq!(minify("(f)(x);"), "f(x);");
    }

    /// gap-065: `(a.b)(x)` → `a.b(x)` — member-chain callee.
    #[test]
    fn gap065_member_callee_paren_stripped() {
        assert_eq!(minify("(a.b)(x);"), "a.b(x);");
    }

    /// gap-065: `` (f)`t` `` → `` f`t` `` — tagged-template callee.
    #[test]
    fn gap065_tagged_callee_paren_stripped() {
        assert_eq!(minify("(f)`t`;"), "f`t`;");
    }

    /// gap-065 boundary: a SEQUENCE callee keeps its parens —
    /// `(a,b)(x)` must NOT become `a,b(x)` (different program).
    #[test]
    fn gap065_sequence_callee_keeps_parens() {
        assert_eq!(minify("(a,b)(x);"), "(a,b)(x);");
    }

    /// gap-065 boundary: an OPERATOR callee keeps its parens —
    /// `(a+b)(x)` must NOT become `a+b(x)`.
    #[test]
    fn gap065_operator_callee_keeps_parens() {
        assert_eq!(minify("(a+b)(x);"), "(a+b)(x);");
    }

    /// gap-065 safety: a CALL paren must never be stripped —
    /// `f(g)(x)` keeps `(g)` (it is f's call, not a grouping).
    #[test]
    fn gap065_call_paren_not_stripped() {
        assert_eq!(minify("f(g)(x);"), "f(g)(x);");
    }

    // ---- gap-099: computed-member object paren elision ----

    /// gap-099: grouping parens around a computed-member OBJECT are
    /// dropped — `(b)[c]` → `b[c]`, `(b.c)[d]` → `b.c[d]`, including
    /// deep chains and assignment targets.
    #[test]
    fn gap099_computed_member_paren_stripped() {
        assert_eq!(minify("a=(b)[c];"), "a=b[c];");
        assert_eq!(minify("a=(b.c)[d];"), "a=b.c[d];");
        assert_eq!(minify("a=(b.c.d)[e];"), "a=b.c.d[e];");
        assert_eq!(minify("(a)[b]=c;"), "a[b]=c;");
    }

    /// Trailing accessors ride along after the unwrap.
    #[test]
    fn gap099_computed_member_then_chain() {
        assert_eq!(minify("a=(b)[c][d];"), "a=b[c][d];");
        assert_eq!(minify("a=(b)[c].d;"), "a=b[c].d;");
    }

    /// **Non-regression**: a non-trivial operand keeps its parens —
    /// `(a+b)[c]` and `(b||c)[d]` are NOT simple references.
    #[test]
    fn gap099_operator_operand_keeps_parens() {
        assert_eq!(minify("x=(a+b)[c];"), "x=(a+b)[c];");
        assert_eq!(minify("a=(b||c)[d];"), "a=(b||c)[d];");
    }

    /// **Non-regression**: a CALL paren is never treated as grouping —
    /// `f(b)[c]` keeps `(b)` (it is f's call), not stripped to `fb[c]`.
    #[test]
    fn gap099_call_paren_not_stripped() {
        assert_eq!(minify("a=f(b)[c];"), "a=f(b)[c];");
    }

    // ---- gap-100: function/class-expr paren in expr position ----

    /// gap-100: a parenthesised function/class EXPRESSION in
    /// expression position drops its grouping parens.
    #[test]
    fn gap100_funcexpr_paren_stripped() {
        assert_eq!(minify("a=(function(){})();"), "a=function(){}();");
        assert_eq!(minify("a=(class{})();"), "a=class{}();");
        assert_eq!(
            minify("a=(async function(){})();"),
            "a=async function(){}();"
        );
        assert_eq!(minify("b=1,(function(){})();"), "b=1,function(){}();");
    }

    /// **CRITICAL non-regression**: an IIFE at STATEMENT position
    /// MUST keep its parens — `(function(){})();` would otherwise be
    /// reparsed as a function *declaration*.
    #[test]
    fn gap100_statement_iife_keeps_parens() {
        assert_eq!(minify("(function(){})();"), "(function(){})();");
        assert_eq!(minify("(function(){}());"), "(function(){})();");
    }

    /// **Non-regression**: a DEFAULT-PARAMETER default value is NOT
    /// unwrapped — the `=` there is a param default, not a statement
    /// assignment (its target sits after a `(`, not a statement
    /// boundary), and unwrapping would expose the body `}` to the
    /// function-decl trailing-`;` rule.
    #[test]
    fn gap100_default_param_not_unwrapped() {
        // The grouping parens around the default-value function
        // expression are preserved (gap-100 leaves it alone).
        assert!(minify("function g(a=(function(){})()){}").contains("(function(){}"));
    }

    // ---- gap-066: redundant parens after `extends` -------

    /// gap-066: `class A extends(B){}` → `class A extends B{}`.
    #[test]
    fn gap066_extends_paren_stripped() {
        assert_eq!(minify("class A extends(B){}"), "class A extends B{};");
    }

    /// gap-066: member-chain superclass — `extends(a.b)` →
    /// `extends a.b`.
    #[test]
    fn gap066_extends_member_chain() {
        assert_eq!(minify("class A extends(a.b){}"), "class A extends a.b{};");
    }

    /// gap-066: class EXPRESSION heritage also stripped.
    #[test]
    fn gap066_class_expression_extends() {
        assert_eq!(minify("var x=class extends(B){};"), "var x=class extends B{};");
    }

    /// gap-066 SAFETY: an operator superclass keeps its parens —
    /// `extends(B||C)` must NOT become `extends B||C` (which is
    /// INVALID JS — `B||C` is not a LeftHandSideExpression).
    /// closurec stays safe here even though upstream strips it.
    #[test]
    fn gap066_operator_heritage_keeps_parens() {
        assert_eq!(minify("class A extends(B||C){}"), "class A extends(B||C){};");
    }

    /// gap-066 SAFETY: a PROPERTY named `extends` is a method
    /// call, not a class heritage — `o.extends(x)` must NOT have
    /// its call parens stripped.
    #[test]
    fn gap066_extends_property_not_stripped() {
        assert_eq!(minify("o.extends(x);"), "o.extends(x);");
    }

    // ---- gap-068: redundant parens around a `new` callee ----

    /// gap-068: `new(f)()` → `new f` — strip the callee parens
    /// (then the empty `()` is dropped by gap-050).
    #[test]
    fn gap068_new_call_paren_stripped() {
        assert_eq!(minify("new(f)();"), "new f;");
    }

    /// gap-068: `new(a.b)` → `new a.b` — member-chain callee.
    #[test]
    fn gap068_new_member_paren_stripped() {
        assert_eq!(minify("new(a.b);"), "new a.b;");
        assert_eq!(minify("new(a.b.c);"), "new a.b.c;");
    }

    /// gap-068 SAFETY: an operator callee keeps its parens —
    /// `new(a+b)` must NOT become `new a+b` (which parses as
    /// `(new a)+b` — a different program).
    #[test]
    fn gap068_operator_callee_keeps_parens() {
        // closurec keeps the parens; it does not strip (the
        // emit-time `new`/`(` spacing is a separate concern).
        assert!(minify("new(a+b);").contains("(a+b)"));
    }

    /// gap-068 SAFETY: a PROPERTY named `new` is a method call,
    /// not a NewExpression — `o.new(f)` must NOT be stripped.
    #[test]
    fn gap068_new_property_not_stripped() {
        assert_eq!(minify("o.new(f);"), "o.new(f);");
    }

    // ---- gap-069: `new (` emit-adjacency separating space ----

    /// gap-069: a `new` keyword followed by a KEPT grouping paren
    /// (compound callee) gets a separating space — `new(a+b)` →
    /// `new (a+b)`, matching upstream Closure.
    #[test]
    fn gap069_new_paren_gets_space() {
        assert_eq!(minify("new(a+b);"), "new (a+b);");
        assert_eq!(minify("new(a,b);"), "new (a,b);");
    }

    /// gap-069 SAFETY: a PROPERTY named `new` (`o.new(f)`) is a
    /// method call, NOT a NewExpression — the preceding `.`
    /// accessor must suppress the space. (Also covered by
    /// gap068_new_property_not_stripped; this asserts the space
    /// specifically does not creep in for a compound argument.)
    #[test]
    fn gap069_property_new_no_space() {
        assert_eq!(minify("o.new(a+b);"), "o.new(a+b);");
        assert_eq!(minify("o?.new(a+b);"), "o?.new(a+b);");
    }

    /// gap-069 boundary: `new X()` (simple identifier callee) does
    /// NOT trigger the space — `new` is followed by the IDENT `X`,
    /// not a `(` — and gap-050 still drops the empty `()`.
    #[test]
    fn gap069_new_ident_unaffected() {
        assert_eq!(minify("new X();"), "new X;");
    }

    // ---- gap-070: unary-keyword operand member-chain paren elision ----

    /// gap-070: `delete`/`typeof`/`void` followed by a parenthesised
    /// MEMBER-REFERENCE CHAIN drops the redundant parens.
    #[test]
    fn gap070_member_chain_paren_elided() {
        assert_eq!(minify("delete(a.b);"), "delete a.b;");
        assert_eq!(minify("delete(a.b.c);"), "delete a.b.c;");
        assert_eq!(minify("delete(a[b]);"), "delete a[b];");
        assert_eq!(minify("typeof(a.b);"), "typeof a.b;");
        assert_eq!(minify("void(a.b);"), "void a.b;");
    }

    /// gap-070 SAFETY (correctness fix): a PROPERTY named
    /// `delete`/`typeof`/`void` is a method call, NOT a unary
    /// operator — its call parens must be preserved. Before the
    /// property guard, `o.delete(a)` mis-emitted as the invalid
    /// `o.delete a`.
    #[test]
    fn gap070_property_keyword_not_elided() {
        assert_eq!(minify("o.delete(a);"), "o.delete(a);");
        assert_eq!(minify("o.typeof(x);"), "o.typeof(x);");
        assert_eq!(minify("o?.delete(a);"), "o?.delete(a);");
    }

    /// gap-070 boundary: an operand containing a top-level binary
    /// operator is NOT a pure reference chain — `delete(a+b)` must
    /// keep its parens (`delete a+b` ≠ `delete(a+b)`).
    #[test]
    fn gap070_operator_operand_kept() {
        assert_eq!(minify("delete(a+b);"), "delete(a+b);");
        assert_eq!(minify("typeof(a||b);"), "typeof(a||b);");
    }

    /// gap-054 non-regression: the single-token cases still elide.
    #[test]
    fn gap070_single_token_still_elided() {
        assert_eq!(minify("delete(a);"), "delete a;");
        assert_eq!(minify("typeof(x);"), "typeof x;");
        assert_eq!(minify("void(0);"), "void 0;");
    }

    // ---- gap-071: instanceof right-operand paren elision ----

    /// gap-071: `instanceof` followed by a parenthesised simple
    /// reference (single token or member chain) drops the parens.
    #[test]
    fn gap071_instanceof_operand_elided() {
        assert_eq!(minify("var x=a instanceof(B);"), "var x=a instanceof B;");
        assert_eq!(
            minify("var x=a instanceof(b.c);"),
            "var x=a instanceof b.c;"
        );
        assert_eq!(
            minify("var x=a instanceof(b[c]);"),
            "var x=a instanceof b[c];"
        );
    }

    /// gap-071 boundary: an operand containing a top-level binary
    /// operator keeps its parens — `a instanceof(B||C)` ≠
    /// `a instanceof B||C`.
    #[test]
    fn gap071_operator_operand_kept() {
        assert_eq!(
            minify("var x=a instanceof(B||C);"),
            "var x=a instanceof(B||C);"
        );
    }

    /// gap-071 SAFETY: a PROPERTY named `instanceof`
    /// (`o.instanceof(x)`) is a method call, NOT the operator — its
    /// call parens must be preserved.
    #[test]
    fn gap071_property_instanceof_not_elided() {
        assert_eq!(minify("o.instanceof(x);"), "o.instanceof(x);");
    }

    // ---- gap-075: prefix-unary symbol operand paren elision ----

    /// gap-075: `-`/`+`/`!`/`~` followed by a parenthesised simple
    /// reference drops the parens.
    #[test]
    fn gap075_symbol_unary_operand_elided() {
        assert_eq!(minify("var x=-(a);"), "var x=-a;");
        assert_eq!(minify("var x=!(a);"), "var x=!a;");
        assert_eq!(minify("var x=~(a);"), "var x=~a;");
        assert_eq!(minify("var x=-(a.b);"), "var x=-a.b;");
    }

    /// gap-075 same-sign: `-(-a)` / `+(+a)` strip the parens but a
    /// separating space (gap-063) prevents the `--`/`++` glue.
    #[test]
    fn gap075_same_sign_gets_space() {
        assert_eq!(minify("var x=-(-a);"), "var x=- -a;");
        assert_eq!(minify("var x=+(+a);"), "var x=+ +a;");
    }

    /// gap-075 boundary: an operator operand keeps its parens
    /// (`-(a+b)` ≠ `-a+b`); a binary `-` with a simple right operand
    /// also strips (`a-(b)` → `a-b`).
    #[test]
    fn gap075_operator_operand_kept_binary_simple_stripped() {
        assert_eq!(minify("var x=-(a+b);"), "var x=-(a+b);");
        assert_eq!(minify("var x=a-(b);"), "var x=a-b;");
        assert_eq!(minify("var x=a-(b+c);"), "var x=a-(b+c);");
    }

    // ---- gap-101: unary operator + higher-arity operand elision ---

    /// gap-101: a unary KEYWORD operator with a parenthesised
    /// UNARY-expression operand drops the grouping parens. The
    /// separator follows the usual word-like rule — a word-like
    /// inner operator keeps the space (`typeof void 0`), a symbol
    /// inner operator collapses it (`typeof-b`, `typeof!b`).
    #[test]
    fn gap101_unary_keyword_operand_elided() {
        assert_eq!(minify("a=typeof(void 0);"), "a=typeof void 0;");
        assert_eq!(minify("a=typeof(typeof b);"), "a=typeof typeof b;");
        assert_eq!(minify("a=void(void 0);"), "a=void void 0;");
        assert_eq!(minify("a=typeof(-b);"), "a=typeof-b;");
        assert_eq!(minify("a=typeof(!b);"), "a=typeof!b;");
        assert_eq!(minify("a=typeof(~b);"), "a=typeof~b;");
    }

    /// gap-101: a unary operator with a parenthesised CALL / member
    /// operand drops the parens (calls bind tighter than every prefix
    /// unary and than `instanceof`).
    #[test]
    fn gap101_unary_call_operand_elided() {
        assert_eq!(minify("a=typeof(b());"), "a=typeof b();");
        assert_eq!(minify("a=typeof(a.b());"), "a=typeof a.b();");
        assert_eq!(minify("a=void(b());"), "a=void b();");
        assert_eq!(
            minify("var x=a instanceof(C());"),
            "var x=a instanceof C();"
        );
        assert_eq!(
            minify("var x=a instanceof(typeof c);"),
            "var x=a instanceof typeof c;"
        );
    }

    /// gap-101 boundary: a parenthesised BINARY / comma / assignment /
    /// ternary operand binds looser than the would-be adjacency and
    /// MUST keep its parens.
    #[test]
    fn gap101_binary_operand_kept() {
        assert_eq!(minify("a=typeof(b+c);"), "a=typeof(b+c);");
        assert_eq!(minify("a=typeof(b||c);"), "a=typeof(b||c);");
        assert_eq!(minify("a=typeof(b,c);"), "a=typeof(b,c);");
        assert_eq!(minify("a=typeof(a=b);"), "a=typeof(a=b);");
        assert_eq!(minify("a=typeof(b?c:d);"), "a=typeof(b?c:d);");
        assert_eq!(minify("a=void(b=c);"), "a=void(b=c);");
    }

    // ---- gap-102: yield operand paren elision ------------------

    /// gap-102: a `yield` operand's grouping parens are redundant and
    /// dropped (like `return`/`throw`, gap-056), since `yield` takes an
    /// AssignmentExpression. Covers ident, member-chain, binary, and
    /// assignment-RHS operands; the separator follows the usual rule
    /// (`yield a` keeps the space, `yield-a` collapses it).
    #[test]
    fn gap102_yield_operand_elided() {
        assert_eq!(minify("function*g(){yield(a);}"), "function*g(){yield a};");
        assert_eq!(
            minify("function*g(){yield(a.b);}"),
            "function*g(){yield a.b};"
        );
        assert_eq!(
            minify("function*g(){yield(a+b);}"),
            "function*g(){yield a+b};"
        );
        assert_eq!(
            minify("function*g(){a=yield(b);}"),
            "function*g(){a=yield b};"
        );
        assert_eq!(
            minify("function*g(){yield(-a);}"),
            "function*g(){yield-a};"
        );
    }

    /// gap-102 boundary: a comma-operator operand keeps its parens
    /// (`yield a,b` ≡ `(yield a),b`), and the `yield*` delegate form is
    /// left untouched (the token after `yield` is `*`, not `(`).
    #[test]
    fn gap102_yield_comma_and_delegate_kept() {
        assert_eq!(
            minify("function*g(){yield(a,b);}"),
            "function*g(){yield(a,b)};"
        );
        assert_eq!(minify("function*g(){yield*a;}"), "function*g(){yield*a};");
    }

    /// gap-102 SAFETY: a PROPERTY named `yield` (`o.yield(x)`) is a
    /// method call, NOT the generator keyword — its call parens must be
    /// preserved.
    #[test]
    fn gap102_property_yield_not_elided() {
        assert_eq!(minify("o.yield(a);"), "o.yield(a);");
        assert_eq!(minify("a=b.yield(c);"), "a=b.yield(c);");
    }

    // ---- gap-078: binary symbol-operator right-operand elision --

    /// gap-078: a binary comparison / logical / arithmetic / bitwise
    /// symbol operator with a parenthesised ATOMIC right operand
    /// drops the parens (`a==(b)` → `a==b`, …).
    #[test]
    fn gap078_binary_operand_paren_stripped() {
        assert_eq!(minify("var x=a==(b);"), "var x=a==b;");
        assert_eq!(minify("var x=a!=(b);"), "var x=a!=b;");
        assert_eq!(minify("var x=a===(b);"), "var x=a===b;");
        assert_eq!(minify("var x=a<(b);"), "var x=a<b;");
        assert_eq!(minify("var x=a||(b);"), "var x=a||b;");
        assert_eq!(minify("var x=a&&(b);"), "var x=a&&b;");
        assert_eq!(minify("var x=a??(b);"), "var x=a??b;");
        assert_eq!(minify("var x=a*(b);"), "var x=a*b;");
        assert_eq!(minify("var x=a%(b);"), "var x=a%b;");
        assert_eq!(minify("var x=a<<(b);"), "var x=a<<b;");
        assert_eq!(minify("var x=a>>>(b);"), "var x=a>>>b;");
        assert_eq!(minify("var x=a&(b);"), "var x=a&b;");
    }

    /// gap-078: a member-chain right operand also strips
    /// (`a==(b.c)` → `a==b.c`).
    #[test]
    fn gap078_member_chain_operand_stripped() {
        assert_eq!(minify("var x=a==(b.c);"), "var x=a==b.c;");
    }

    /// gap-078 boundary (DEFERRED precedence-aware case): an operand
    /// containing a top-level binary operator KEEPS its parens — the
    /// conservative atomic-operand guard does not yet do the full
    /// precedence analysis the JAR does (`a==(b+c)` → upstream
    /// `a==b+c`). Output stays valid; just not byte-identical.
    #[test]
    fn gap078_operator_operand_kept() {
        assert_eq!(minify("var x=a==(b+c);"), "var x=a==(b+c);");
        assert_eq!(minify("var x=a*(b+c);"), "var x=a*(b+c);");
    }

    /// gap-078 SAFETY: a string/regex literal whose CONTENT is an
    /// operator must never be treated as the operator token — and a
    /// CALL paren (`f(a)`) is not a grouping paren.
    #[test]
    fn gap078_literal_and_call_safe() {
        // `"=="` is a string literal, not the `==` operator.
        assert_eq!(minify("var x=\"==\"+(b);"), "var x=\"==\"+b;");
        // `f(a)` is a call, never stripped.
        assert_eq!(minify("var x=a==f(b);"), "var x=a==f(b);");
    }

    // ---- gap-077: binary LEFT-operand paren elision -----------

    /// gap-077: a binary operator's parenthesised ATOMIC LEFT operand
    /// drops the parens (`(a)+b` → `a+b`, the mirror of gap-075/078).
    #[test]
    fn gap077_left_operand_paren_stripped() {
        assert_eq!(minify("var x=(a)+b;"), "var x=a+b;");
        assert_eq!(minify("var x=(a)*b;"), "var x=a*b;");
        assert_eq!(minify("var x=(a)==b;"), "var x=a==b;");
        assert_eq!(minify("var x=(a)||b;"), "var x=a||b;");
        assert_eq!(minify("var x=(a.b)+c;"), "var x=a.b+c;");
    }

    /// gap-077 PRECEDENCE SAFETY: a LEFT operand containing a
    /// top-level binary operator keeps its parens — `(a+b)*c` must
    /// NOT become `a+b*c`.
    #[test]
    fn gap077_operator_left_operand_kept() {
        assert_eq!(minify("var x=(a+b)*c;"), "var x=(a+b)*c;");
        assert_eq!(minify("var x=(a||b)&&c;"), "var x=(a||b)&&c;");
    }

    /// gap-077 SAFETY: a CALL paren is NOT a grouping paren — `f(a)+b`
    /// must stay (dropping it would corrupt to `fa+b`). Likewise a
    /// comma-operator group `(a,b)+c` keeps its parens.
    #[test]
    fn gap077_call_and_comma_kept() {
        assert_eq!(minify("var x=f(a)+b;"), "var x=f(a)+b;");
        assert_eq!(minify("var x=(a,b)+c;"), "var x=(a,b)+c;");
    }

    /// gap-077 EXPONENTIATION HAZARD: `**` forbids an unparenthesised
    /// unary LEFT operand — `-a**b` is a SyntaxError. So `(-a)**b`
    /// MUST keep its parens (matches the JAR), even though `(-a)` is
    /// otherwise a "safe" operand. A plain `(a)**b` still strips.
    #[test]
    fn gap077_exponent_unary_left_kept() {
        assert_eq!(minify("var x=(-a)**b;"), "var x=(-a)**b;");
        assert_eq!(minify("var x=(!a)**b;"), "var x=(!a)**b;");
        assert_eq!(minify("var x=(a)**b;"), "var x=a**b;");
    }

    // ---- gap-081: ternary CONDITION paren elision -------------

    /// gap-081: a grouping paren around a ternary `?:` CONDITION
    /// elides — `(a)?b:c` → `a?b:c`, `(a.b)?c:d` → `a.b?c:d`. The
    /// condition-side mirror of gap-055 (ternary arms).
    #[test]
    fn gap081_ternary_condition_paren_stripped() {
        assert_eq!(minify("var x=(a)?b:c;"), "var x=a?b:c;");
        assert_eq!(minify("var x=(a.b)?c:d;"), "var x=a.b?c:d;");
    }

    /// gap-081 SAFETY: a CALL paren is NOT a grouping paren
    /// (`f(a)?b:c` must stay), and a comma-operator condition keeps
    /// its parens (`(a,b)?c:d` — the atomic-operand guard rejects the
    /// top-level comma). The operator-condition `(a||b)?c:d` is the
    /// deferred precedence-aware gap-083 — closurec keeps it (valid).
    #[test]
    fn gap081_call_comma_and_operator_kept() {
        assert_eq!(minify("var x=f(a)?b:c;"), "var x=f(a)?b:c;");
        assert_eq!(minify("var x=(a,b)?c:d;"), "var x=(a,b)?c:d;");
        assert_eq!(minify("var x=(a||b)?c:d;"), "var x=(a||b)?c:d;");
    }

    /// gap-081: `?.` is a single OPTIONAL_CHAIN token, NOT a bare
    /// ternary `?`, so gap-081 never mis-fires on `(a)?.b` (the token
    /// after `)` is `?.`). closurec leaves it as `(a)?.b` — upstream's
    /// `a?.b` is a separate optional-member paren elision (deferred),
    /// not a ternary condition. The key property: gap-081 does NOT
    /// corrupt or wrongly strip the optional-chain form.
    #[test]
    fn gap081_optional_chain_not_ternary() {
        assert_eq!(minify("var x=(a)?.b;"), "var x=(a)?.b;");
    }

    // ---- gap-088: empty-statement elimination -----------------

    /// gap-088: a `;` whose predecessor is `{`, `;`, or start-of-input
    /// is an EmptyStatement and is dropped. Leading, trailing, between,
    /// sole, and block-internal empties all go; the first `;` after a
    /// real statement is its terminator and stays.
    #[test]
    fn gap088_empty_statements_dropped() {
        assert_eq!(minify(";;var x=1;"), "var x=1;");
        assert_eq!(minify("var x=1;;;"), "var x=1;");
        assert_eq!(minify("var a=1;;var b=2;"), "var a=1;var b=2;");
        assert_eq!(minify(";;;"), "");
        assert_eq!(minify(";"), "");
        assert_eq!(minify(";x();"), "x();");
        assert_eq!(minify("a;;b;"), "a;b;");
        assert_eq!(minify("function f(){;;x();}"), "function f(){x()};");
    }

    /// gap-088 SAFETY — a `;` that is the BODY of a control-flow header
    /// is NOT an empty statement and must be kept: `while(a);`,
    /// `if(a);`, `for(;;);`, `do;while(a);`. Each such `;` follows a
    /// `)`/`do`, never a `{`/`;`/start, so the predecessor test already
    /// excludes it.
    #[test]
    fn gap088_control_flow_body_semicolon_kept() {
        assert_eq!(minify("while(a);"), "while(a);");
        assert_eq!(minify("if(a);"), "if(a);");
        assert_eq!(minify("for(;;);"), "for(;;);");
        assert_eq!(minify("do;while(a);"), "do;while(a);");
    }

    /// gap-088 SAFETY — the `for( … )` header separators must survive.
    /// In `for(;;)` the SECOND `;` is preceded by the first `;`, so the
    /// predecessor test alone would drop it; the bracket-stack for-guard
    /// keeps it. A genuine `for` loop is distinguished from a `.for(`
    /// property call (`a.for(b)` is untouched and never mistaken for a
    /// for-header).
    #[test]
    fn gap088_for_header_separators_kept() {
        assert_eq!(minify("for(;;)x();"), "for(;;)x();");
        assert_eq!(
            minify("for(var i=0;i<3;i++)x();"),
            "for(var i=0;i<3;i++)x();"
        );
        assert_eq!(minify("a.for(b);"), "a.for(b);");
    }

    // ---- gap-086: call-argument paren elision -----------------

    /// gap-086: a paren wrapping a WHOLE call argument elides. Argument
    /// position accepts any AssignmentExpression, so — unlike the
    /// operand passes — there is NO atomic / precedence guard: a binary
    /// (`f((a+b))` → `f(a+b)`), logical (`f((a||b))` → `f(a||b)`), or
    /// string (`f(("s"))` → `f("s")`) argument all strip. Multiple and
    /// nested arguments each strip independently.
    #[test]
    fn gap086_call_arg_paren_stripped() {
        assert_eq!(minify("f((a));"), "f(a);");
        assert_eq!(minify("f((a+b));"), "f(a+b);");
        assert_eq!(minify("f((a||b));"), "f(a||b);");
        assert_eq!(minify("f((a),(b));"), "f(a,b);");
        assert_eq!(minify("f((a),b);"), "f(a,b);");
        assert_eq!(minify("f(a,(b));"), "f(a,b);");
        assert_eq!(minify("f(g((a)));"), "f(g(a));");
    }

    /// gap-086 SAFETY — the ONE load-bearing case. A single
    /// comma-operator argument `f((a,b))` must KEEP its parens: dropping
    /// them resplits the one argument into the two arguments `a,b`. The
    /// top-level-comma guard preserves it, including when mixed with
    /// other args (`f((a,b),c)` keeps the first arg's parens).
    #[test]
    fn gap086_comma_operator_argument_kept() {
        assert_eq!(minify("f((a,b));"), "f((a,b));");
        assert_eq!(minify("f((a,b),c);"), "f((a,b),c);");
    }

    /// gap-086 SAFETY — a parenthesised ARROW PARAMETER list is not a
    /// grouping paren and must be left alone: `f((a,b)=>a)` keeps its
    /// `(a,b)` (the `)` is followed by `=>`, not an arg boundary).
    /// (A single-param `(a)=>a` is reduced to `a=>a` by gap-045, which
    /// is orthogonal.)
    #[test]
    fn gap086_arrow_param_list_kept() {
        assert_eq!(minify("f((a,b)=>a);"), "f((a,b)=>a);");
    }

    /// gap-086: the anchor is the CALL-open paren (preceded by a value),
    /// so member calls (`a.b((c))` → `a.b(c)`), computed-member calls
    /// (`x[i]((a))` → `x[i](a)`), and `new` calls (`new C((a))` →
    /// `new C(a)`) are all covered.
    #[test]
    fn gap086_member_and_new_call_args() {
        assert_eq!(minify("a.b((c));"), "a.b(c);");
        assert_eq!(minify("x[i]((a));"), "x[i](a);");
        assert_eq!(minify("new C((a));"), "new C(a);");
    }

    // ---- gap-087: computed-member index paren elision ---------

    /// gap-087: a paren wrapping the WHOLE index of a computed-member
    /// subscript elides. The brackets already delimit a single
    /// expression, so no comma / atomic-operand guard is needed — even
    /// a comma operator (`a[(b,c)]` → `a[b,c]`) or an assignment
    /// (`a[(b=c)]` → `a[b=c]`) is safe to expose.
    #[test]
    fn gap087_index_paren_stripped() {
        assert_eq!(minify("a[(b)];"), "a[b];");
        assert_eq!(minify("a[(b+c)];"), "a[b+c];");
        assert_eq!(minify("a[(b,c)];"), "a[b,c];");
        assert_eq!(minify("a[(b=c)];"), "a[b=c];");
    }

    /// gap-087: the subscripted object may itself end in a `)`/`]`
    /// (`x()[(b)]` → `x()[b]`), and nested subscripts each strip
    /// independently (`a[b[(c)]]` → `a[b[c]]`).
    #[test]
    fn gap087_index_paren_value_object_and_nested() {
        assert_eq!(minify("x()[(b)];"), "x()[b];");
        assert_eq!(minify("a[b[(c)]];"), "a[b[c]];");
    }

    /// gap-087 SAFETY — must NOT mistake an ARRAY-LITERAL `[` for a
    /// subscript. An array literal `[` is NOT preceded by a value, and
    /// inside it a top-level comma is an ELEMENT separator: `[(a,b)]`
    /// keeps its parens (dropping them would split one element into
    /// two). Only a value-preceded subscript `[` is eligible.
    #[test]
    fn gap087_array_literal_comma_element_kept() {
        assert_eq!(minify("var x=[(a,b)];"), "var x=[(a,b)];");
    }

    /// gap-087 SAFETY — the parens must wrap the WHOLE index. A partial
    /// paren (`a[(b)+c]`) is left to the gap-077 left-operand pass
    /// (which yields `a[b+c]`), and a non-grouping call inside the
    /// index (`a[f(b)]`) is untouched by this pass.
    #[test]
    fn gap087_partial_and_call_index_safe() {
        assert_eq!(minify("a[(b)+c];"), "a[b+c];");
        assert_eq!(minify("a[f(b)];"), "a[f(b)];");
    }

    // ---- gap-073: get/set computed-key separating space ----

    /// gap-073: a `get`/`set` accessor before a COMPUTED key `[k]`
    /// in an object literal gets a separating space.
    #[test]
    fn gap073_accessor_computed_key_spaced() {
        assert_eq!(
            minify("var o={get[k](){return 1}};"),
            "var o={get [k](){return 1}};"
        );
        assert_eq!(minify("var o={set[k](v){}};"), "var o={set [k](v){}};");
        assert_eq!(
            minify("var o={a:1,get[k](){return 2}};"),
            "var o={a:1,get [k](){return 2}};"
        );
    }

    /// gap-073 SAFETY: a member access `o.get[k]` or a variable
    /// index+call `get[k](x)` must NOT gain a space — `get`/`set`
    /// there are plain identifiers, not accessors.
    #[test]
    fn gap073_member_access_not_spaced() {
        assert_eq!(minify("o.get[k];"), "o.get[k];");
        assert_eq!(minify("get[k](x);"), "get[k](x);");
    }

    // ---- gap-067: labeled single-statement block flatten ----

    /// gap-067: `label:{break label}` → `label:break label;` —
    /// the single-statement block's braces drop and a synthetic
    /// `;` terminates the flattened statement.
    #[test]
    fn gap067_labeled_block_flattens() {
        assert_eq!(minify("label:{break label}"), "label:break label;");
    }

    /// gap-067: a body that already ends in `;` reuses it (no
    /// double `;`).
    #[test]
    fn gap067_labeled_block_trailing_semi() {
        assert_eq!(minify("label:{break label;}"), "label:break label;");
        assert_eq!(minify("label:{throw e}"), "label:throw e;");
    }

    /// gap-067 boundary: a MULTI-statement labeled block keeps
    /// its braces.
    #[test]
    fn gap067_multi_statement_keeps_braces() {
        assert_eq!(minify("label:{a();break label}"), "label:{a();break label};");
    }

    /// gap-067 SAFETY: an object literal whose nested value
    /// resembles `IDENT:{...}` must NOT be flattened — the inner
    /// `{` is an object, preceded by the outer object `{` (which
    /// the pass excludes as a statement boundary).
    #[test]
    fn gap067_object_literal_not_flattened() {
        assert_eq!(minify("var o={x:{break:1}};"), "var o={x:{break:1}};");
    }

    /// gap-067 SAFETY: a ternary `a?b:{c}` is not a labeled
    /// block — must be untouched.
    #[test]
    fn gap067_ternary_not_flattened() {
        assert_eq!(minify("a?b:{c};"), "a?b:{c};");
    }

    // ---- gap-074: loop-body single-statement block flatten ----

    /// gap-074: a `for`/`while` body that is a single un-terminated
    /// statement drops its braces; a synthetic `;` terminates it.
    #[test]
    fn gap074_loop_body_flattens() {
        assert_eq!(minify("l:for(;;){continue l}"), "l:for(;;)continue l;");
        assert_eq!(minify("for(;;){break}"), "for(;;)break;");
        assert_eq!(minify("while(x){g()}"), "while(x)g();");
        assert_eq!(minify("for(a in o){h(a)}"), "for(a in o)h(a);");
        assert_eq!(minify("for(a of o){h(a)}"), "for(a of o)h(a);");
    }

    /// gap-074 boundary: a MULTI-statement loop body keeps braces.
    #[test]
    fn gap074_multi_statement_keeps_braces() {
        assert_eq!(minify("for(;;){a();b()}"), "for(;;){a();b()};");
    }

    /// gap-074 SAFETY: a PROPERTY method named `while`/`for`
    /// (`o.while(x){…}`) is a method call, NOT a loop — its block
    /// must not be flattened.
    #[test]
    fn gap074_property_method_not_flattened() {
        assert_eq!(minify("o.while(x){f()}"), "o.while(x){f()};");
    }

    /// gap-074 conservative deferral: a body containing a nested
    /// control-flow keyword keeps its braces (left for follow-up).
    #[test]
    fn gap074_nested_control_flow_kept() {
        assert_eq!(minify("for(;;){if(x)a()}"), "for(;;){if(x)a()};");
    }

    // ---- gap-076: with-body single-statement block flatten ----

    /// gap-076: a `with` body that is a single un-terminated
    /// statement drops its braces (`with`-sibling of gap-074).
    #[test]
    fn gap076_with_body_flattens() {
        assert_eq!(minify("with(o){a()}"), "with(o)a();");
    }

    /// gap-076 boundary: a MULTI-statement `with` body keeps braces;
    /// a PROPERTY method named `with` (`o.with(x){…}`) is untouched.
    #[test]
    fn gap076_with_body_guards() {
        assert_eq!(minify("with(o){a();b()}"), "with(o){a();b()};");
        assert_eq!(minify("o.with(x){f()}"), "o.with(x){f()};");
    }

    // ---- gap-079: if-body single-statement block flatten ------

    /// gap-079: an `if` consequent that is a single un-terminated
    /// statement drops its braces (`if`-sibling of gap-074/076).
    #[test]
    fn gap079_if_body_flattens() {
        assert_eq!(minify("if(x){y()}"), "if(x)y();");
    }

    /// gap-079 boundary: a MULTI-statement `if` body keeps braces;
    /// an empty body is untouched.
    #[test]
    fn gap079_if_body_guards() {
        assert_eq!(minify("if(x){a();b()}"), "if(x){a();b()};");
    }

    /// gap-079 DANGLING-ELSE SAFETY: an `if` whose body contains a
    /// nested un-`else`-d `if` and is followed by an `else` must KEEP
    /// its braces — flattening `if(a){if(b)c()}else d()` to
    /// `if(a)if(b)c();else d()` would re-bind the `else` to the inner
    /// `if(b)`. The existing no-control-flow-keyword guard prevents
    /// the brace-drop (the body contains `if`). The decisive property
    /// is that the braces survive (verified against the JAR, which
    /// keeps them too — the only diff is an unrelated EOF trailing
    /// `;` after the bare `else` arm).
    #[test]
    fn gap079_dangling_else_kept() {
        assert_eq!(
            minify("if(a){if(b)c()}else d()"),
            "if(a){if(b)c()}else d()"
        );
    }

    /// gap-079: an `else if(...)` chain flattens the inner body
    /// (`else if(c){d()}` → `else if(c)d();`).
    #[test]
    fn gap079_else_if_chain_inner_flattens() {
        assert_eq!(
            minify("if(a)b();else if(c){d()}"),
            "if(a)b();else if(c)d();"
        );
    }

    // ---- gap-080: else-body single-statement block flatten ----

    /// gap-080: an `else` alternate that is a single un-terminated
    /// statement drops its braces (`else`-sibling of gap-079).
    #[test]
    fn gap080_else_body_flattens() {
        assert_eq!(minify("if(x)a();else{b()}"), "if(x)a();else b();");
    }

    /// gap-080 boundary: a MULTI-statement `else` body keeps braces.
    #[test]
    fn gap080_else_body_multi_keeps_braces() {
        assert_eq!(
            minify("if(x)a();else{b();c()}"),
            "if(x)a();else{b();c()};"
        );
    }

    /// gap-080 conservative deferral: an `else` body containing a
    /// nested control-flow keyword keeps its braces (left for
    /// follow-up; output stays valid). `else if(...)` is a DIFFERENT
    /// shape — `else` is followed by `if`, not `{`, so it is not
    /// matched here; its inner consequent flattens via gap-079.
    #[test]
    fn gap080_else_nested_control_flow_kept() {
        assert_eq!(
            minify("if(x)a();else{if(y)b()}"),
            "if(x)a();else{if(y)b()};"
        );
    }

    /// gap-080 SAFETY: `else` as an object-literal property KEY is
    /// NOT an alternate block — `{else:1}` has `else` followed by
    /// `:`, never `{`, so the anchor never fires.
    #[test]
    fn gap080_else_property_key_untouched() {
        assert_eq!(minify("var o={else:1};"), "var o={else:1};");
    }

    /// gap-057 safety: a CALL paren must never be stripped —
    /// `f(a).b` is a call on `f`, not a grouping paren. Dropping
    /// the parens would corrupt it to `fa.b`.
    #[test]
    fn gap057_call_paren_preserved() {
        assert_eq!(minify("var x=f(a).b;"), "var x=f(a).b;");
    }

    /// gap-057 safety: an OPTIONAL CALL `x?.(a)` is a call, not a
    /// grouping paren — must stay (else `x?.a.b` changes meaning).
    #[test]
    fn gap057_optional_call_preserved() {
        assert_eq!(minify("var x=y?.(a).b;"), "var x=y?.(a).b;");
    }

    /// gap-057 safety: a NUMBER object must keep its parens —
    /// `(1).toString()` → `1.toString()` would mis-lex `1.`.
    #[test]
    fn gap057_number_object_preserved() {
        assert_eq!(
            minify("var x=(1).toString();"),
            "var x=(1).toString();"
        );
    }

    /// gap-057 safety: a multi-token object is NOT a single
    /// identifier — `(a+b).c` must keep its parens (precedence).
    #[test]
    fn gap057_compound_object_preserved() {
        assert_eq!(minify("var x=(a+b).c;"), "var x=(a+b).c;");
    }

    /// gap-099 + structural-punct safety: `(a)["\")\""]` — the grouping
    /// parens around the computed-member object `a` are now stripped by
    /// gap-099 (`(a)[…]` → `a[…]`), while the string literal `")"` is
    /// preserved INTACT — its content must not be mistaken for a
    /// structural bracket by the depth scan. (Before gap-099 this stayed
    /// `(a)[")"]`; upstream JAR confirms `a[")"]`.)
    #[test]
    fn gap099_string_content_not_bracket() {
        assert_eq!(minify("var x=(a)[\")\"];"), "var x=a[\")\"];");
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

    /// gap-094 (CORRECTNESS, was a wrong gap-046 assertion): `[1,,]`
    /// is `[1, <hole>]` (length 2). The comma before `]` follows a
    /// HOLE (the preceding `,`), so it is load-bearing and must be
    /// KEPT — dropping it to `[1,]` (length 1) silently changes the
    /// array. A fresh JAR probe confirms upstream keeps `[1,,]`
    /// verbatim. (The original test asserted the buggy `[1,]` and even
    /// noted the rule was "technically WRONG" — gap-094 fixes it.)
    #[test]
    fn gap046_elision_with_trailing_normalised() {
        assert_eq!(minify("var a=[1,,];"), "var a=[1,,];");
    }

    /// gap-094: a trailing comma after a REAL element still drops
    /// (`[1,2,]` → `[1,2]`, `[[1],]` → `[[1]]`, `[f(),]` → `[f()]`),
    /// while every hole form is preserved (`[,]`, `[,,]`, `[1,2,,]`).
    #[test]
    fn gap094_hole_vs_real_trailing_comma() {
        assert_eq!(minify("var x=[,];"), "var x=[,];");
        assert_eq!(minify("var x=[,,];"), "var x=[,,];");
        assert_eq!(minify("var x=[1,2,,];"), "var x=[1,2,,];");
        assert_eq!(minify("var x=[[1],];"), "var x=[[1]];");
        assert_eq!(minify("var x=[f(),];"), "var x=[f()];");
    }

    // ---- gap-093: number-literal `.member` paren wrap ----

    /// Target case (CORRECTNESS): an integer literal followed by a
    /// member `.` must be wrapped — bare `1.x` does not parse, so we
    /// emit `(1).x`. The lexer splits `1 .x` into NUMBER `1` + DOT +
    /// NAME, which re-stitched verbatim would print the invalid `1.x`.
    #[test]
    fn gap093_integer_member_prop_wrapped() {
        assert_eq!(minify("a=1 .x;"), "a=(1).x;");
    }

    /// A method call on an integer wraps the receiver: `255 .toString(16)`
    /// → `(255).toString(16)`.
    #[test]
    fn gap093_integer_member_method_wrapped() {
        assert_eq!(minify("a=255 .toString(16);"), "a=(255).toString(16);");
    }

    /// A float literal whose own decimal point is internal still wraps
    /// for member access: `1.5.toString()` → `(1.5).toString()`.
    #[test]
    fn gap093_float_member_method_wrapped() {
        assert_eq!(minify("a=1.5.toString();"), "a=(1.5).toString();");
    }

    /// The DOUBLE-DOT form `1..toString()` lexes as NUMBER `1` + DOT +
    /// DOT + NAME. The first dot is the (split-off) decimal point; the
    /// second is the member operator. After wrapping the `1`, exactly
    /// ONE dot survives: `(1).toString()`.
    #[test]
    fn gap093_double_dot_collapses_to_one() {
        assert_eq!(minify("a=1..toString();"), "a=(1).toString();");
        assert_eq!(minify("a=1..x;"), "a=(1).x;");
    }

    /// A member chain only wraps the head number; trailing accessors
    /// ride along: `1 .x.y` → `(1).x.y`.
    #[test]
    fn gap093_member_chain_wraps_head_only() {
        assert_eq!(minify("a=1 .x.y;"), "a=(1).x.y;");
    }

    /// gap-082 normalisation runs first, so the wrapped value is the
    /// canonical number: `1.5e3.toFixed(2)` → `(1500).toFixed(2)` and
    /// `0xff .toString()` → `(255).toString()`.
    #[test]
    fn gap093_wraps_normalised_value() {
        assert_eq!(minify("a=1.5e3.toFixed(2);"), "a=(1500).toFixed(2);");
        assert_eq!(minify("a=0xff .toString();"), "a=(255).toString();");
    }

    /// **Non-regression**: an INDEX access is left alone — the follower
    /// is `[`, not `.`, so `1[0]` stays `1[0]` (no spurious parens).
    #[test]
    fn gap093_index_access_not_wrapped() {
        assert_eq!(minify("a=1[0];"), "a=1[0];");
    }

    /// **Non-regression**: an already-parenthesised number is untouched
    /// (its follower is `)`, not `.`) — `(1).x` stays `(1).x`, no
    /// double-wrap.
    #[test]
    fn gap093_already_parenthesised_untouched() {
        assert_eq!(minify("a=(1).x;"), "a=(1).x;");
    }

    /// **Non-regression**: a number as an object key (`{1:2}`, follower
    /// `:`) or in arithmetic (`1+2`) is never wrapped.
    #[test]
    fn gap093_non_member_numbers_untouched() {
        assert_eq!(minify("x={1:2};"), "x={1:2};");
        assert_eq!(minify("a=1+2;"), "a=1+2;");
        assert_eq!(minify("a=f(1);"), "a=f(1);");
    }

    /// **Non-regression**: identifier member access is still handled by
    /// gap-057 (paren elision), unaffected by the number wrap: `(foo).x`
    /// → `foo.x`, `b.c.d` → `b.c.d`.
    #[test]
    fn gap093_identifier_member_unaffected() {
        assert_eq!(minify("a=(foo).x;"), "a=foo.x;");
        assert_eq!(minify("a=b.c.d;"), "a=b.c.d;");
    }

    // ---- gap-098: trailing bare decimal point drop ----

    /// Target case: a trailing bare `.` on an integer (the float `5.0`)
    /// is a redundant decimal point — drop it. The lexer splits `5.`
    /// into NUMBER `5` + DOT `.`; the dot's follower here is `;` (not a
    /// property name), so the dot cannot be member access.
    #[test]
    fn gap098_trailing_dot_before_semicolon_dropped() {
        assert_eq!(minify("a=5.;"), "a=5;");
        assert_eq!(minify("a=50.;"), "a=50;");
    }

    /// The dot also drops before an operator (`5.+1` -> `5+1`,
    /// `5.*2` -> `5*2`) — the follower is a punctuator, never a name.
    #[test]
    fn gap098_trailing_dot_before_operator_dropped() {
        assert_eq!(minify("a=5.+1;"), "a=5+1;");
        assert_eq!(minify("a=5.*2;"), "a=5*2;");
        assert_eq!(minify("a=5.===5;"), "a=5===5;");
    }

    /// Drops at end-of-expression contexts too: assignment chain,
    /// comma, call argument, array element.
    #[test]
    fn gap098_trailing_dot_various_contexts() {
        assert_eq!(minify("a=b=5.;"), "a=b=5;");
        assert_eq!(minify("a=5.,b=6;"), "a=5,b=6;");
        assert_eq!(minify("f(5.);"), "f(5);");
        assert_eq!(minify("a=[5.];"), "a=[5];");
    }

    /// `5.[0]` — the dot is followed by `[` (an index access, not a
    /// property name), so gap-098 drops the redundant decimal point,
    /// leaving the bare index `5[0]`.
    #[test]
    fn gap098_trailing_dot_before_index() {
        assert_eq!(minify("a=5.[0];"), "a=5[0];");
    }

    /// **Non-regression**: a genuine float `5.5` is a SINGLE NUMBER
    /// token (the lexer keeps the fraction), so there is no separate
    /// DOT and gap-098 never fires.
    #[test]
    fn gap098_genuine_float_untouched() {
        assert_eq!(minify("a=5.5;"), "a=5.5;");
        assert_eq!(minify("a=.5;"), "a=.5;");
    }

    /// **Non-regression**: gap-098 is the complement of gap-093, not a
    /// replacement — a number followed by a `.member` access still
    /// parenthesises (`1 .x` -> `(1).x`, `1..toString()` ->
    /// `(1).toString()`), and an index on a plain integer (`1[0]`) is
    /// untouched.
    #[test]
    fn gap098_does_not_disturb_gap093() {
        assert_eq!(minify("a=1 .x;"), "a=(1).x;");
        assert_eq!(minify("a=1..toString();"), "a=(1).toString();");
        assert_eq!(minify("a=1[0];"), "a=1[0];");
    }

    // ---- gap-097: async generator method `async`/`*` separator ----

    /// Target case: an async generator method needs a space between
    /// `async` and `*`, in both object literals and class bodies.
    #[test]
    fn gap097_async_gen_method_gets_space() {
        assert_eq!(minify("o={async*m(){}};"), "o={async *m(){}};");
        assert_eq!(minify("class A{async*m(){}}"), "class A{async *m(){}};");
    }

    /// `static async*m(){}` and a method followed by another method
    /// both keep the space.
    #[test]
    fn gap097_async_gen_static_and_chained() {
        assert_eq!(
            minify("class A{static async*m(){}}"),
            "class A{static async *m(){}};"
        );
        assert_eq!(
            minify("class A{async*m(){}b(){}}"),
            "class A{async *m(){}b(){}};"
        );
    }

    /// Params (incl. destructuring with defaults) don't fool the
    /// matching-`)` scan: the body `{` is still found.
    #[test]
    fn gap097_async_gen_method_with_params() {
        assert_eq!(minify("o={async*m(a,b){}};"), "o={async *m(a,b){}};");
        assert_eq!(
            minify("o={async*m({a}={}){}};"),
            "o={async *m({a}={}){}};"
        );
    }

    /// **CRITICAL non-regression**: `async*x` is also valid as
    /// MULTIPLICATION (`async` is only a contextual keyword). The
    /// arithmetic forms must NOT gain a space — they lack the `){`
    /// method body that the helper requires.
    #[test]
    fn gap097_multiplication_not_spaced() {
        assert_eq!(minify("a=async*b;"), "a=async*b;");
        assert_eq!(minify("a=async*b*c;"), "a=async*b*c;");
        assert_eq!(minify("a=async*f();"), "a=async*f();");
        assert_eq!(minify("a=async*f()*g;"), "a=async*f()*g;");
        assert_eq!(minify("a=b,async*c;"), "a=b,async*c;");
    }

    /// **Non-regression**: a COMPUTED method name (`async*[x](){}`)
    /// gets no space — `*[` can't merge, so upstream omits it.
    #[test]
    fn gap097_computed_method_name_not_spaced() {
        assert_eq!(minify("o={async*[x](){}};"), "o={async*[x](){}};");
    }

    /// **Non-regression**: `async function*f(){}` already has the
    /// `function` keyword between `async` and `*`, so no extra space.
    #[test]
    fn gap097_async_generator_function_unchanged() {
        assert_eq!(minify("async function*f(){}"), "async function*f(){};");
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
    /// gap-059 (was gap-050 "keeps_parens"): `new Foo().bar`
    /// is now WRAPPED to `(new Foo).bar`. Dropping the `()`
    /// without wrapping would mis-parse as `new(Foo.bar)`, so
    /// upstream wraps the NewExpression. The old behavior
    /// (leaving `new Foo().bar` intact) diverged from upstream;
    /// gap-059 makes it byte-identical.
    #[test]
    fn gap059_new_call_then_member_wraps() {
        assert_eq!(minify("var x=new Foo().bar;"), "var x=(new Foo).bar;");
    }

    /// gap-060: member-CALLEE new-expr — `new a.b.C().d` →
    /// `(new a.b.C).d`. The callee scan consumes the `.IDENT`
    /// accessor chain (`a.b.C`) before the empty `()`.
    #[test]
    fn gap060_member_callee_new_wraps() {
        assert_eq!(minify("var x=new a.b.C().d;"), "var x=(new a.b.C).d;");
    }

    /// gap-060: member callee + computed-member follower —
    /// `new a.b().c[0]` → `(new a.b).c[0]` (only the new-expr
    /// is wrapped; the trailing chain is untouched).
    #[test]
    fn gap060_member_callee_then_member_chain() {
        assert_eq!(minify("var x=new a.b().c;"), "var x=(new a.b).c;");
    }

    /// gap-089 (was a gap-060 "deferred" non-regression): a
    /// member-callee `new` with NO following member/call and empty
    /// args now drops the `()` — `new a.b.C()` → `new a.b.C`. gap-050
    /// only handled single-identifier callees; gap-089's forward
    /// pre-pass extends the empty-paren drop to member-expression
    /// callees.
    #[test]
    fn gap060_member_callee_standalone_unchanged() {
        assert_eq!(minify("var x=new a.b.C();"), "var x=new a.b.C;");
    }

    /// gap-064 (CORRECTNESS): a string argument whose content is
    /// `)` must NOT be mistaken for the empty-arg close paren.
    /// `new A(")")` keeps its string arg (was mangled to
    /// `new A);` before the `is_structural_punct` guard).
    #[test]
    fn gap064_string_paren_arg_not_dropped() {
        assert_eq!(minify("var z=new A(\")\");"), "var z=new A(\")\");");
    }

    // ---- gap-089: new member-callee empty-paren drop ----------

    /// gap-089: the empty `()` of a `new` with a MEMBER-expression
    /// callee is dropped — dotted (`new a.b()` → `new a.b`,
    /// `new a.b.c()` → `new a.b.c`) and computed (`new a[x]()` →
    /// `new a[x]`). Extends gap-050 (bare-identifier callees) to a
    /// `.IDENT` / `[ … ]` member chain rooted at `new`.
    #[test]
    fn gap089_new_member_empty_paren_dropped() {
        assert_eq!(minify("new a.b();"), "new a.b;");
        assert_eq!(minify("new a.b.c();"), "new a.b.c;");
        assert_eq!(minify("x=new a.b();"), "x=new a.b;");
        assert_eq!(minify("new a[x]();"), "new a[x];");
    }

    /// gap-089 SAFETY — a follower that re-binds the result blocks the
    /// drop (the same gate as gap-050). `new a.b().c`, `new a.b()()`,
    /// `new a.b()[0]` are handled by the new-expr member-wrap pass
    /// (`(new a.b).c` etc.), NOT by dropping the `()`. A benign
    /// operator follower is safe (`new a.b()+1` → `new a.b+1`).
    #[test]
    fn gap089_blocked_followers_not_dropped() {
        assert_eq!(minify("new a.b().c;"), "(new a.b).c;");
        assert_eq!(minify("new a.b()();"), "(new a.b)();");
        assert_eq!(minify("new a.b()[0];"), "(new a.b)[0];");
        assert_eq!(minify("new a.b()+1;"), "new a.b+1;");
    }

    /// gap-089 boundary: a bare-identifier callee stays gap-050's job
    /// (`new A()` → `new A`), a non-`new` member call is untouched
    /// (`a.b()` stays), and a non-empty arg list is kept
    /// (`new a.b(1)` stays).
    #[test]
    fn gap089_boundaries() {
        assert_eq!(minify("new A();"), "new A;");
        assert_eq!(minify("a.b();"), "a.b();");
        assert_eq!(minify("new a.b(1);"), "new a.b(1);");
    }

    /// gap-064: same string-`)` arg under the arg-bearing member
    /// wrap — `new A(")").b` → `(new A(")")).b`, not `(new A)).b`.
    #[test]
    fn gap064_string_paren_arg_member_wrap() {
        assert_eq!(
            minify("var z=new A(\")\").b;"),
            "var z=(new A(\")\")).b;"
        );
    }

    /// gap-064 non-regression: the genuine empty-paren drop still
    /// fires (`new A()` → `new A`), and a real non-empty arg
    /// (`new A(x)`) is preserved.
    #[test]
    fn gap064_genuine_empty_paren_still_drops() {
        assert_eq!(minify("var z=new A();"), "var z=new A;");
        assert_eq!(minify("var z=new A(x);"), "var z=new A(x);");
    }

    /// gap-059: `new Foo()[0]` → `(new Foo)[0]` (computed
    /// member triggers the same wrap).
    #[test]
    fn gap059_new_call_then_bracket_wraps() {
        assert_eq!(minify("var x=new Foo()[0];"), "var x=(new Foo)[0];");
    }

    /// gap-059: `new Foo()()` → `(new Foo)()` (call on the
    /// constructed object triggers the wrap; the chained call
    /// is preserved).
    #[test]
    fn gap059_new_call_then_call_wraps() {
        assert_eq!(minify("var x=new Foo()();"), "var x=(new Foo)();");
    }

    /// gap-059 non-regression: standalone `new Foo()` (no
    /// member/call follows) is still just `new Foo` (gap-050),
    /// NOT wrapped.
    #[test]
    fn gap059_standalone_new_not_wrapped() {
        assert_eq!(minify("var x=new Foo();"), "var x=new Foo;");
    }

    /// gap-059 guard: a property named `new` (`a.new()`) is
    /// NOT the `new` operator, so no wrap.
    #[test]
    fn gap059_property_new_not_wrapped() {
        assert_eq!(minify("var x=a.new().b;"), "var x=a.new().b;");
    }

    /// gap-061: arg-bearing new-expr member — `new Foo(y).b` →
    /// `(new Foo(y)).b`. Synthetic parens are inserted since the
    /// arg list is non-empty (no spare parens to reorder). The
    /// old behavior (left unchanged) was deferred; gap-061 makes
    /// it byte-identical with upstream.
    #[test]
    fn gap061_arg_bearing_new_wraps() {
        assert_eq!(minify("var x=new Foo(y).b;"), "var x=(new Foo(y)).b;");
    }

    /// gap-061: member callee + multiple args + member follower
    /// — `new a.b.C(y,z).d` → `(new a.b.C(y,z)).d`.
    #[test]
    fn gap061_member_callee_multi_arg_wraps() {
        assert_eq!(
            minify("var x=new a.b.C(y,z).d;"),
            "var x=(new a.b.C(y,z)).d;"
        );
    }

    /// gap-061: nested call in the arg list — the depth-balanced
    /// scan finds the OUTER arg-list close. `new A(f(x)).b` →
    /// `(new A(f(x))).b`.
    #[test]
    fn gap061_nested_call_args_wraps() {
        assert_eq!(minify("var x=new A(f(x)).b;"), "var x=(new A(f(x))).b;");
    }

    /// gap-061 non-regression: arg-bearing new with NO following
    /// member/call is NOT wrapped (`new A(y)` stays — nothing to
    /// disambiguate).
    #[test]
    fn gap061_standalone_arg_new_unchanged() {
        assert_eq!(minify("var x=new A(y);"), "var x=new A(y);");
    }

    /// gap-061 guard: property `new` (`a.new(x).b`) is not the
    /// operator — must NOT wrap.
    #[test]
    fn gap061_property_new_not_wrapped() {
        assert_eq!(minify("var x=a.new(x).b;"), "var x=a.new(x).b;");
    }

    // ---- gap-062: redundant double-paren collapse --------

    /// gap-062: `((a+b))*c` → `(a+b)*c` — one redundant
    /// directly-nested grouping-paren layer is stripped.
    #[test]
    fn gap062_double_paren_collapses_one_layer() {
        assert_eq!(minify("var x=((a+b))*c;"), "var x=(a+b)*c;");
    }

    /// gap-062 safety: a CALL paren must never be stripped —
    /// `f((a,b))` (one comma-operator arg) must NOT become
    /// `f(a,b)` (two args). The outer `(` follows `f` (a
    /// callable), so the grouping guard skips it.
    #[test]
    fn gap062_call_paren_with_comma_preserved() {
        assert_eq!(minify("f((a,b));"), "f((a,b));");
    }

    /// gap-062 / gap-075 / gap-077 interaction: in `g((a)+(b))` the
    /// gap-075 symbol-operand pass strips the RIGHT `+` operand's
    /// grouping parens (`+(b)` → `+b`) and the gap-077 left-operand
    /// pass strips the LEFT `(a)` (its `)` is followed by the binary
    /// `+`), so BOTH grouping layers now go — matching upstream
    /// (`g(a+b)`). Verified against the JAR.
    #[test]
    fn gap062_call_arg_grouping_preserved() {
        assert_eq!(minify("g((a)+(b));"), "g(a+b);");
    }

    /// gap-062 non-regression: a single grouping layer is NOT
    /// touched (`(a+b)*c` stays — nothing redundant to strip).
    #[test]
    fn gap062_single_paren_unchanged() {
        assert_eq!(minify("var x=(a+b)*c;"), "var x=(a+b)*c;");
    }

    // ---- gap-084: nested double-paren var-init full strip -----

    /// gap-084: the gap-053 var-init elision runs to a FIXPOINT, so a
    /// nested double (or deeper) paren around a whole RHS strips every
    /// redundant layer — `((a))` → `a`, `(((a)))` → `a`, `((a+b))` →
    /// `a+b` (each layer is the whole RHS).
    #[test]
    fn gap084_nested_double_paren_varinit_fully_strips() {
        assert_eq!(minify("var x=((a));"), "var x=a;");
        assert_eq!(minify("var x=(((a)));"), "var x=a;");
        assert_eq!(minify("var x=((a+b));"), "var x=a+b;");
        assert_eq!(minify("x=((a));"), "x=a;");
    }

    /// gap-084 SAFETY: the top-level-comma guard still halts the
    /// fixpoint at the load-bearing layer — `((a,b))` keeps ONE paren
    /// (`(a,b)` is a comma operator; a bare `a,b` RHS would become two
    /// declarators). Matches the JAR.
    #[test]
    fn gap084_comma_operator_keeps_one_layer() {
        assert_eq!(minify("var x=((a,b));"), "var x=(a,b);");
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

    /// **Non-regression (gap-050) + gap-069**: `new` keyword
    /// followed by a non-identifier (e.g. `new (expr)`) is NOT
    /// targeted by the gap-050 empty-paren peephole — kept[idx-1]
    /// isn't a simple identifier, so the trailing constructor
    /// `()` survives. Since CLOC12.78, the `new (` adjacency also
    /// carries the gap-069 separating space.
    #[test]
    fn gap050_new_with_paren_expr_unchanged() {
        // `new (Foo||Bar)()` — paren expression for
        // constructor selection. The empty `()` after the
        // paren-close is NOT what gap-050 targets (idx-1 is `)`,
        // not an identifier), so it is preserved. gap-069 keeps
        // the `new (` space (the grouping parens are kept).
        assert_eq!(
            minify("var x=new (Foo||Bar)();"),
            "var x=new (Foo||Bar)();"
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
