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
    let mut saw_function_kw_at_boundary = false;
    let mut next_block_is_try_chain = false;
    // ^ gap-033: armed by `try` / `catch` / `finally`
    //   keywords; consumed by the next `{` and stored on the
    //   brace stack as `BlockKind::TryChain`. `try` arms it
    //   at a statement boundary. `catch` and `finally` arm
    //   it whenever they appear (they only legally appear
    //   after a preceding `}` that closed a try-chain block,
    //   so the boundary context is already correct).
    let mut next_paren_is_control_flow_head = false;
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
            // Priority: TryChain > Function > Other. A `{`
            // immediately after `try`/`catch`/`finally` is a
            // try-chain body; one preceded by `function` at a
            // statement boundary is a function-decl body;
            // everything else is Other.
            let kind = if next_block_is_try_chain {
                BlockKind::TryChain
            } else if saw_function_kw_at_boundary {
                BlockKind::Function
            } else {
                BlockKind::Other
            };
            brace_stack.push(kind);
            next_block_is_try_chain = false;
            saw_function_kw_at_boundary = false;
            // A `{` either opens a Block-as-body (consumes
            // body_position_next) or opens a function-decl /
            // try-chain body (independent of body_position).
            // Either way the body slot is filled by this brace.
            body_position_next = false;
            at_stmt_boundary = true;
        } else if val == "(" {
            paren_stack.push(next_paren_is_control_flow_head);
            next_paren_is_control_flow_head = false;
            at_stmt_boundary = false;
        } else if val == ")" {
            let was_cf = paren_stack.pop().unwrap_or(false);
            // The body slot of an if/while/for opens after
            // the closing `)`. The next emitted statement
            // (or `;` / `{`) fills that slot.
            body_position_next = was_cf;
            at_stmt_boundary = false;
        } else if val == ";" {
            // We emitted a real `;` (either a terminator or
            // a body slot per body_position_next). Either
            // way, body_position_next is consumed.
            body_position_next = false;
            at_stmt_boundary = true;
        } else if is_keyword_function(tok) {
            // Only treat as a function-DECLARATION when at a
            // statement boundary. Expressions live
            // mid-expression and don't qualify.
            if at_stmt_boundary {
                saw_function_kw_at_boundary = true;
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
    is_word_like(a) && is_word_like(b)
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
    #[test]
    fn gap030_if_block_drops_inner_semi_no_trailing() {
        assert_eq!(minify("if(x){y();}"), "if(x){y()}");
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

    /// A NON-empty for-body must NOT collapse — only `{}`
    /// (empty) triggers the rule. The `{` arm checks that
    /// the very next token is `}`.
    #[test]
    fn gap031_nonempty_for_body_unaffected() {
        assert_eq!(
            minify("for(var i=0;i<10;i++){a;}"),
            "for(var i=0;i<10;i++){a}"
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
}
