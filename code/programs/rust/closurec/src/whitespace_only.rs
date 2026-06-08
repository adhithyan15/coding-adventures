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
    let mut brace_stack: Vec<bool> = Vec::new();
    // ^ for each open `{`, true iff this `{` opened a
    //   function-declaration body. Popped on matching `}`.
    let mut paren_stack: Vec<bool> = Vec::new();
    // ^ for each open `(`, true iff this `(` was the head of
    //   an `if` / `while` / `for` (control-flow construct
    //   whose body slot follows the close-paren). Popped on
    //   matching `)`.
    let mut saw_function_kw_at_boundary = false;
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

        // Rule B: a `}` that closes a function-declaration
        // body gets a synthetic `;` appended.
        if val == "}" {
            let was_function_decl = brace_stack.pop().unwrap_or(false);
            if was_function_decl {
                out.push(';');
                last_emit_was_synthetic_semi = true;
            }
            at_stmt_boundary = true;
            body_position_next = false;
        } else if val == "{" {
            brace_stack.push(saw_function_kw_at_boundary);
            saw_function_kw_at_boundary = false;
            // A `{` either opens a Block-as-body (consumes
            // body_position_next) or opens a function-decl
            // body (independent of body_position). Either
            // way the body slot is filled by this brace.
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
}
