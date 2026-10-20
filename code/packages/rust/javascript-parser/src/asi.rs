//! Automatic Semicolon Insertion (ASI) — the `}` / end-of-input rule (Rule 2)
//! and the line-terminator rule (Rule 1).
//!
//! # Why this module exists
//!
//! ECMAScript lets you omit many statement-ending semicolons; the language
//! inserts them automatically (§12.10). closurec's grammar, however, spells
//! `SEMICOLON` out as a *required* terminal in every statement production, so
//! ordinary semicolon-light source fails to parse:
//!
//! ```text
//!   function f(x) { if (x) { return 1 } else { g() } }
//!                                     ^^^ no `;` before `}`  →  parse error
//! ```
//!
//! When that happens, closurec degrades the *whole program* to
//! `WHITESPACE_ONLY` — every optimization pass is skipped. ASI closes that gap.
//!
//! # The approach: retry-on-parse-error (byte-identical by construction)
//!
//! Two ASI rules are implemented (§12.10):
//!
//! * **Rule 2** (`}` / end-of-input) — a `;` is inserted before a `}` (or at end
//!   of input) that would otherwise be a syntax error. No line terminator needed.
//! * **Rule 1** (line terminator) — a `;` is inserted before an offending token
//!   that is **preceded by a line terminator**. The lexer discards newlines as
//!   trivia, but each token records the `line` it starts on, so a line
//!   terminator sits between two tokens exactly when the later one starts on a
//!   higher line (and its single-line predecessor did not itself span lines —
//!   see [`asi_applies_at`]). No shared-lexer change is required.
//!
//! Rather than guess insertion points from a lookahead table (the approach the
//! design spec first sketched), we drive insertion from the parser itself:
//!
//! 1. Parse the token stream.
//! 2. If it parses, we are done — **nothing is inserted**.
//! 3. If it fails *specifically because a `SEMICOLON` was expected* at a point
//!    where Rule 1 or Rule 2 applies, synthesize a `;` there and re-parse.
//! 4. Any other failure is returned unchanged (closurec then degrades as before).
//!
//! The decisive property: a semicolon is inserted **only when parsing genuinely
//! failed for lack of one**. Therefore ASI is a *no-op on any input that already
//! parses* — it can never change a valid program's parse. This is what makes the
//! transform safe to drop into the parse path: every existing fixture (which all
//! use explicit semicolons) is byte-for-byte unaffected, and the only programs
//! whose behaviour changes are ones that previously failed outright.
//!
//! The cost is re-parsing once per inserted semicolon (O(insertions × n)). For a
//! build-time minifier this is acceptable; a batched single-pass insertion is a
//! possible future optimization, but correctness-by-construction comes first.
//!
//! # Phase 3: restricted productions (Rule 3) — why retry-on-error is not enough
//!
//! Rules 1 and 2 only ever fire on a *parse failure*, which is what makes them
//! safe (a program that already parses is untouched). The **restricted
//! productions** are different and more dangerous, because the offending program
//! parses *successfully into the wrong tree*:
//!
//! ```text
//!   return
//!     a + b
//! ```
//!
//! ECMAScript §12.10.1 forbids a line terminator between `return` and its
//! argument, so this is `return; a + b;` (an empty return followed by an
//! expression statement) — **not** `return a + b;`. closurec's grammar spells
//! `return` as `RETURN expr? SEMICOLON` and is *blind to newlines* (the lexer
//! discards them as trivia), so it greedily parses `return a + b` and would
//! re-emit exactly that: a silent **miscompile** that changes what the program
//! returns.
//!
//! Because the bad parse *succeeds*, the retry-on-error loop never sees it. So
//! Rule 3 must run as a **proactive pre-pass** ([`force_restricted_semicolons`])
//! that scans the stream *before* the first parse and inserts a `;` immediately
//! after a restricted keyword whose argument is pushed onto the next line. The
//! five restricted keywords are `return`, `throw`, `break`, `continue`, `yield`.
//!
//! Safety still holds by the same lever as Rules 1/2 — **we only insert when a
//! line terminator actually follows the keyword** ([`TOKEN_PRECEDED_BY_NEWLINE`]
//! on the *next* token). A valid `return x;` keeps its argument on the same line,
//! so the flag is clear and we never touch it; the only streams whose parse we
//! change are exactly the ones JS semantics say we *must* change. Context guards
//! (below) keep a `return` that is really a *property name* (`a.return`,
//! `{return: 1}`) from being mistaken for the statement keyword.

use coding_adventures_javascript_tokens::EsVersion;
use lexer::token::{Token, TokenType, TOKEN_PRECEDED_BY_NEWLINE};
use parser::grammar_parser::{
    GrammarASTNode, GrammarParseError, GrammarParser, DEFAULT_MAX_RULE_DEPTH,
};

/// Parse `tokens` with Phase-1 ASI applied.
///
/// Returns the parsed tree on success, or the final [`GrammarParseError`] if the
/// program cannot be parsed even with `}`/EOF semicolons supplied (so the caller
/// still degrades exactly as it did before ASI existed).
// `GrammarParseError` is the parser's standard error type, returned identically
// across the public parsing API; boxing it just to shrink this Result would
// churn every caller's error handling for no real benefit.
#[allow(clippy::result_large_err)]
pub fn parse_with_asi(
    tokens: Vec<Token>,
    version: EsVersion,
) -> Result<GrammarASTNode, GrammarParseError> {
    parse_with_asi_tracked(tokens, version).map(|(ast, _)| ast)
}

/// Where ASI inserts semicolons into `tokens` (#17840).
///
/// Runs exactly the same algorithm as [`parse_with_asi`] (restricted-production
/// pre-pass, then retry-on-error), and returns, in ascending order, the index
/// in `tokens` *before which* each synthetic `;` goes. Two insertions before
/// the same token report that index twice. An index equal to `tokens.len()`
/// means the end of the stream.
///
/// This is for consumers that keep their own token vector and can't take a
/// rewritten one. closurec's WHITESPACE_ONLY path maps correlation-vector ids
/// through pointers into its original tokens, and splices its own `;` at these
/// points. Without them it drops newline-ASI boundaries and glues
/// `a()⏎b()` into the invalid `a()b()`.
///
/// ```text
/// tokens:  a ( ) b ( ) EOF      (b preceded by a newline)
/// result:  [3, 6]               →  a ( ) ; b ( ) ; EOF
/// ```
///
/// Errors exactly when [`parse_with_asi`] does: the program cannot be parsed
/// even with semicolons supplied.
#[allow(clippy::result_large_err)]
pub fn asi_insertion_points(
    tokens: Vec<Token>,
    version: EsVersion,
) -> Result<Vec<usize>, GrammarParseError> {
    let (_, original) = parse_with_asi_tracked(tokens, version)?;
    // `original[k]` says whether the k-th final token came from the input.
    // A synthetic one goes before the next original, whose input index is the
    // number of originals seen so far.
    let mut seen = 0usize;
    let mut points = Vec::new();
    for is_original in original {
        if is_original {
            seen += 1;
        } else {
            points.push(seen);
        }
    }
    Ok(points)
}

/// [`parse_with_asi`] plus a record of which final tokens are original
/// (`true`) and which are ASI-synthesized (`false`), kept parallel to the
/// token vector through every insertion.
#[allow(clippy::result_large_err)]
fn parse_with_asi_tracked(
    tokens: Vec<Token>,
    version: EsVersion,
) -> Result<(GrammarASTNode, Vec<bool>), GrammarParseError> {
    let grammar = crate::_grammar::parser_grammar(version.as_str())
        .expect("compiled JavaScript parser grammar missing supported version");

    // Phase 3 (restricted productions) runs FIRST, as a proactive pre-pass: the
    // offending streams parse successfully into the wrong tree, so the
    // retry-on-error loop below would never see them. See the module docs.
    let (tokens, original) = force_restricted_semicolons(tokens);

    // #17840 (performance): every retry re-parses the whole stream, so a
    // semicolon-free file costs O(lines × parse). Most of those insertions
    // are certain in advance, so make them all up front in one linear pass
    // (`speculate_line_semicolons`), and leave only the rest to retry. If the
    // speculation doesn't lead to a parse, run the plain algorithm on the
    // unspeculated stream instead. Correctness never rests on the guess.
    let (spec_tokens, spec_original) = speculate_line_semicolons(&tokens, &original);
    if spec_tokens.len() != tokens.len() {
        if let Ok(done) = retry_on_error(spec_tokens, spec_original, &grammar) {
            return Ok(done);
        }
    }
    retry_on_error(tokens, original, &grammar)
}

/// The Rule 1 / Rule 2 retry-on-error loop of [`parse_with_asi`], over a
/// stream that already went through the pre-passes.
#[allow(clippy::result_large_err)]
fn retry_on_error(
    mut tokens: Vec<Token>,
    mut original: Vec<bool>,
    grammar: &grammar_tools::parser_grammar::ParserGrammar,
) -> Result<(GrammarASTNode, Vec<bool>), GrammarParseError> {
    // Each insertion adds exactly one token and (when it helps) lets the parser
    // advance past one more `}`/EOF, so the number of useful insertions is
    // bounded by the token count. The `+ 1` covers the final EOF position.
    let max_insertions = tokens.len() + 1;
    // Positions we have already tried inserting a `;` before. If a failure
    // recurs at a position we have already addressed — even non-consecutively
    // (an A→B→A oscillation) — the inserted `;` is not making progress, so we
    // stop rather than churn up to the hard cap. With correct insertion the
    // failure position strictly advances, so this set only guards pathological
    // input; the `max_insertions` cap is the hard backstop.
    let mut tried: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    // Work budget (#17840 review, DoS). Each attempt re-parses the whole stream,
    // so a long run of insertions the speculation didn't cover costs
    // O(insertions × n). Cap the total at max(64·n, 2M) token-parses. Ordinary
    // input, where speculation covers almost everything, never comes close. A
    // hostile file gives up with the parse error in hand, which both callers
    // already handle: SIMPLE fails cleanly, and WHITESPACE_ONLY skips the
    // insertion.
    let budget = tokens.len().saturating_mul(64).max(2_000_000);
    let mut spent = 0usize;

    for _ in 0..max_insertions {
        spent = spent.saturating_add(tokens.len());
        // Opt into the recursive-descent depth guard (the parser is unbounded
        // by default). closurec runs this on an ordinary ~2 MiB stack over
        // *untrusted* JS, so pathologically deep nesting (`((((…))))`,
        // `1+1+…`, `----…a`) would otherwise overflow the native stack — an
        // uncatchable process abort. `DEFAULT_MAX_RULE_DEPTH` (128) trips a
        // clean, recoverable parse error well below the ~200-frame overflow
        // point; closurec then degrades that input to WHITESPACE_ONLY (still
        // valid output) instead of crashing. Real JS never nests grouping this
        // deep, so no legitimate program is affected.
        let mut parser = GrammarParser::new(tokens.clone(), grammar.clone())
            .with_max_depth(DEFAULT_MAX_RULE_DEPTH);
        match parser.parse() {
            Ok(ast) => return Ok((ast, original)),
            Err(e) => {
                // A `;` must be among what the parser expected here, or this is
                // not a missing-semicolon situation at all.
                if !e.message.contains("SEMICOLON") {
                    return Err(e);
                }
                // Locate the offending token so we can inspect its predecessor
                // (the line-terminator rule needs it).
                let idx = match find_token_index(&tokens, &e.token) {
                    Some(i) => i,
                    // Not locatable (should not happen — full-identity match is
                    // unique); bail safely.
                    None => return Err(e),
                };
                if !asi_applies_at(&tokens, idx) {
                    return Err(e);
                }
                if spent > budget {
                    return Err(e);
                }
                let fail_pos = (e.token.line, e.token.column);
                if !tried.insert(fail_pos) {
                    // Already inserted a `;` for this position and parsing still
                    // fails here — give up with the real error.
                    return Err(e);
                }
                tokens.insert(idx, synthetic_semicolon(&e.token));
                original.insert(idx, false);
            }
        }
    }

    // Budget exhausted (pathological input). Return the final outcome so the
    // caller sees a real error rather than a silent wrong parse. Same opt-in
    // depth guard as the retry loop above.
    GrammarParser::new(tokens, grammar.clone())
        .with_max_depth(DEFAULT_MAX_RULE_DEPTH)
        .parse()
        .map(|ast| (ast, original))
}

/// The five ECMAScript "restricted production" keywords (§12.10.1): a line
/// terminator is **not allowed** between the keyword and what follows it, so a
/// newline there forces ASI. `break`/`continue` take an optional label;
/// `return`/`throw`/`yield` take an optional (for `throw`, required) argument —
/// but the rule is identical for our purposes: a newline immediately after the
/// keyword ends the statement right there.
const RESTRICTED_KEYWORDS: [&str; 5] = ["return", "throw", "break", "continue", "yield"];

/// **Phase 3 — restricted productions.** Scan `tokens` and insert a `;`
/// immediately after any restricted keyword (`return`/`throw`/`break`/
/// `continue`/`yield`) whose *following* token begins on a new line
/// ([`TOKEN_PRECEDED_BY_NEWLINE`]). Runs as a pre-pass because the offending
/// stream parses *successfully into the wrong tree* (see module docs), so the
/// retry-on-error loop never gets a chance to fix it.
///
/// This is the one ASI rule that changes a parse the grammar already accepts, so
/// it is deliberately conservative. An insertion is made only when **all** hold:
///
/// 1. the token is genuinely the restricted *keyword* — matched on the lexer's
///    keyword classification (`type_name`/`type_`), not merely the text, so a
///    same-spelled identifier never qualifies;
/// 2. it is **not a property access** — the previous significant token is not
///    `.` or `?.` (`a.return`, `a?.return` name a property, not a statement);
/// 3. there *is* a next token and it carries `TOKEN_PRECEDED_BY_NEWLINE`; and
/// 4. that next token is not already a statement terminator / non-argument —
///    `;`, `}`, or `:` (a `:` means we are looking at a property key or label
///    such as `{return: 1}`, never a restricted statement, so we leave it).
///
/// Because (3) requires a real newline after the keyword, every valid
/// single-line `return x;` / `throw e;` is untouched: the lever that keeps
/// Rules 1/2 byte-identical keeps Rule 3 byte-identical too.
/// Also returns, parallel to the output, whether each token is original
/// (`true`) or an inserted `;` (`false`); see [`asi_insertion_points`].
fn force_restricted_semicolons(tokens: Vec<Token>) -> (Vec<Token>, Vec<bool>) {
    // Fast path: a stream with no restricted keyword at all (the common case for
    // expression-heavy minifier input) is returned untouched.
    if !tokens
        .iter()
        .any(|t| is_restricted_keyword_token(t) || is_newline_led_update(t))
    {
        let original = vec![true; tokens.len()];
        return (tokens, original);
    }

    let open_paren = paren_table(&tokens);
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len() + 2);
    let mut original: Vec<bool> = Vec::with_capacity(tokens.len() + 2);
    for (i, tok) in tokens.iter().enumerate() {
        // Postfix `++`/`--` is a restricted production too:
        // `LeftHandSideExpression [no LineTerminator here] ++`. So after
        // something that can end an expression, a newline-led `++`/`--` must
        // start a NEW statement as a prefix operator:
        //
        //     x = 1          →  x = 1;
        //     ++y               ++y;
        //
        // The grammar is blind to newlines and reads `1 ++` as postfix, then
        // chokes on `y`. Retry-on-error can't help, because `y` itself isn't
        // newline-led. After a token that can't end an expression
        // (`x =⏎++y`), the `++` is already prefix and nothing is inserted.
        if is_newline_led_update(tok) && i > 0 && ends_expression_before(&tokens, &open_paren, i - 1) {
            out.push(synthetic_semicolon(tok));
            original.push(false);
        }
        out.push(tok.clone());
        original.push(true);

        if !is_restricted_keyword_token(tok) {
            continue;
        }
        // Guard (2): a property name after `.`/`?.` is not the keyword.
        if i > 0 && is_member_access_dot(&tokens[i - 1]) {
            continue;
        }
        // Guards (3)+(4): need a next token, on a new line, that actually starts
        // an argument/statement (not `;` / `}` / `:`).
        match tokens.get(i + 1) {
            Some(next)
                if next.flags.unwrap_or(0) & TOKEN_PRECEDED_BY_NEWLINE != 0
                    && !ends_or_keys_statement(next) =>
            {
                out.push(synthetic_semicolon(next));
                original.push(false);
            }
            _ => {}
        }
    }
    (out, original)
}

/// A `++`/`--` token with a line terminator before it.
fn is_newline_led_update(tok: &Token) -> bool {
    (tok.value == "++" || tok.value == "--")
        && tok.flags.unwrap_or(0) & TOKEN_PRECEDED_BY_NEWLINE != 0
}

/// Can `tok` be the last token of an expression, so that a `++`/`--` right
/// after it would be read as *postfix*? Identifiers, numbers, strings,
/// closing brackets and the literal keywords qualify. Operators, `=`, `(`, `,`
/// and other keywords don't: after them `++` can only be a prefix operator.
fn can_end_expression(tok: &Token) -> bool {
    match tok.type_ {
        TokenType::Name if tok.type_name.is_none() => true,
        TokenType::Number | TokenType::String => true,
        TokenType::Keyword => matches!(
            tok.value.as_str(),
            "this" | "true" | "false" | "null" | "super"
        ),
        _ => matches!(tok.value.as_str(), ")" | "]" | "}"),
    }
}

/// Insert, in one linear pass, the line-terminator semicolons that ASI is
/// certain to insert (#17840, performance). Returns the new stream and its
/// original/synthetic flags.
///
/// A `;` goes before token `i` when all of these hold:
/// - `i` is preceded by a line terminator;
/// - token `i - 1` ends an expression ([`ends_expression_before`]), and is not
///   a block-closing `}` or one of the contextual words `get`, `set`,
///   `static`, `async`, `of`, `let`, `yield`, `await`;
/// - token `i` can only start a new statement: a plain identifier other than
///   `of`, a number or string, or a statement/primary keyword.
///
/// Rule 2's commonest case is speculated too: a `;` before a `}` that closes
/// a **block** when the statement before it ends unterminated
/// (`function f(){ a() }`). The token before the matching `{` decides
/// whether it's a block:
/// - `)`, `else`, `do`, `try` or `finally` open a body;
/// - `;`, `{`, `}` or the start of input open a bare block.
///
/// Anything else is left to retry, including `=>`. That covers `=`, `(`, `:`,
/// `,` and a class name, which may open an object literal or a class body
/// that takes no `;`. `=>` is excluded because the grammar buckets
/// `() => {a:1}` as an object literal, and a speculated `;` would change
/// which parse wins.
///
/// Such a pair can never continue one expression (`a b`, `f() var`), so the
/// parse is bound to fail exactly at `i`, and Rule 1 would insert this `;` on
/// some retry anyway. Doing it up front turns O(lines × parse) into roughly
/// one parse for ordinary semicolon-free code. Anything this misses, such as
/// a `;` before `}` (Rule 2), is still found by retry-on-error.
fn speculate_line_semicolons(tokens: &[Token], original: &[bool]) -> (Vec<Token>, Vec<bool>) {
    const CONTEXTUAL: [&str; 8] = ["get", "set", "static", "async", "of", "let", "yield", "await"];
    const STATEMENT_START_KEYWORDS: [&str; 25] = [
        "var", "let", "const", "if", "for", "while", "do", "return", "throw", "function", "class",
        "switch", "try", "break", "continue", "new", "this", "typeof", "void", "delete", "true",
        "false", "null", "debugger", "super",
    ];
    let starts_statement_only = |t: &Token| match t.type_ {
        // `of`, `from` and `as` continue `for (x of …)`, `import x from …` and
        // `import {a as b}`, so they never start a statement here.
        TokenType::Name if t.type_name.is_none() => !matches!(t.value.as_str(), "of" | "from" | "as"),
        TokenType::Number | TokenType::String => true,
        TokenType::Keyword => STATEMENT_START_KEYWORDS.contains(&t.value.as_str()),
        _ => false,
    };
    let open_paren = paren_table(tokens);
    // `}` index → its `{` index, in one pass.
    let mut open_of: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut stack: Vec<usize> = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        match t.value.as_str() {
            "{" => stack.push(i),
            "}" => {
                if let Some(o) = stack.pop() {
                    open_of.insert(i, o);
                }
            }
            _ => {}
        }
    }
    let opens_block = |o: usize| {
        o == 0 || {
            let before = &tokens[o - 1];
            matches!(
                before.value.as_str(),
                ")" | "else" | "do" | "try" | "finally" | ";" | "{" | "}"
            )
        }
    };
    let ends_statement_unterminated = |k: usize| {
        let t = &tokens[k];
        (t.value != "}" && ends_expression_before(tokens, &open_paren, k))
            || (t.type_ == TokenType::Keyword
                && matches!(t.value.as_str(), "break" | "continue" | "return" | "debugger"))
    };
    let mut out = Vec::with_capacity(tokens.len() + tokens.len() / 8);
    let mut flags = Vec::with_capacity(out.capacity());
    for (i, tok) in tokens.iter().enumerate() {
        if tok.value == "}"
            && i > 0
            && open_of.get(&i).is_some_and(|&o| o + 1 < i && opens_block(o))
            && ends_statement_unterminated(i - 1)
        {
            out.push(synthetic_semicolon(tok));
            flags.push(false);
            out.push(tok.clone());
            flags.push(original[i]);
            continue;
        }
        if i > 0
            && tok.flags.unwrap_or(0) & TOKEN_PRECEDED_BY_NEWLINE != 0
            && starts_statement_only(tok)
        {
            let prev = &tokens[i - 1];
            // A `}` ends an expression only when it closes an object literal:
            // the `{` follows `=`, `(`, `,`, `:`, `?`, `[`, `return` or an
            // operator (`x = {}⏎y()`). A block or class body `}` doesn't.
            let ends_object_literal = prev.value == "}"
                && open_of.get(&(i - 1)).is_some_and(|&o| {
                    o > 0
                        && matches!(
                            tokens[o - 1].value.as_str(),
                            "=" | "(" | "," | ":" | "?" | "[" | "return" | "+" | "-" | "||"
                                | "&&" | "??" | "!"
                        )
                });
            if ((prev.value != "}" && ends_expression_before(tokens, &open_paren, i - 1))
                || ends_object_literal)
                && !CONTEXTUAL.contains(&prev.value.as_str())
            {
                out.push(synthetic_semicolon(tok));
                flags.push(false);
            }
        }
        out.push(tok.clone());
        flags.push(original[i]);
    }
    (out, flags)
}

/// Does `tokens[k]` end an expression, so that a newline-led `++`/`--` after
/// it would be read as postfix?
///
/// [`can_end_expression`] answers by token kind, with one exception it can't
/// see on its own: a `)` that closes a **statement header**. In
/// `if (x)⏎++y` the `++y` is the statement's body, a prefix update, and a
/// `;` there would empty the body and pull the update out of the condition
/// or loop. So for `)`, find its `(`. If `if`, `while`, `for` or `with`
/// precedes it, and that keyword is not a `.`/`?.` property, it's a header.
///
/// A `do…while`'s `while (c)` also counts as a header here, and nothing is
/// inserted. The retry-on-error loop still adds the `;` its statement needs,
/// because the newline-led `++` that follows is exactly Rule 1's offending
/// token.
fn ends_expression_before(tokens: &[Token], open_paren: &[usize], k: usize) -> bool {
    let tok = &tokens[k];
    if !can_end_expression(tok) {
        return false;
    }
    if tok.value != ")" {
        return true;
    }
    let j = open_paren[k];
    if j == usize::MAX {
        return true; // unbalanced `)`; not a header we can see
    }
    let keyword = |i: usize, words: &[&str]| {
        tokens[i].type_ == TokenType::Keyword && words.contains(&tokens[i].value.as_str())
    };
    let not_property = |i: usize| !(i > 0 && is_member_access_dot(&tokens[i - 1]));
    // `if (…)`, `while (…)`, `for (…)`, `with (…)`, and `for await (…)`.
    let header = j > 0
        && ((keyword(j - 1, &["if", "while", "for", "with"]) && not_property(j - 1))
            || (tokens[j - 1].value == "await"
                && j > 1
                && keyword(j - 2, &["for"])
                && not_property(j - 2)));
    !header
}

/// For every `)` index, the index of its matching `(`, or `usize::MAX`. One
/// stack pass, so [`ends_expression_before`] is O(1) per call.
fn paren_table(tokens: &[Token]) -> Vec<usize> {
    let mut open_paren = vec![usize::MAX; tokens.len()];
    let mut stack = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        match t.value.as_str() {
            "(" => stack.push(i),
            ")" => {
                if let Some(o) = stack.pop() {
                    open_paren[i] = o;
                }
            }
            _ => {}
        }
    }
    open_paren
}

/// Is `tok` one of the restricted keywords, *classified as a keyword by the
/// lexer* (`TokenType::Keyword`) — not a same-spelled identifier, string, or
/// number? The JavaScript lexer tags every reserved word with
/// `TokenType::Keyword`, so a precise type check is enough and is what keeps a
/// variable or property literally named `return` from ever qualifying.
///
/// Note `yield` is only a reserved word inside a generator; where the lexer
/// classifies it as an ordinary `Name` (identifier) this returns `false` and we
/// do nothing — which is correct, because there the retry-on-error loop already
/// splits `yield`⏎`x` as two identifier statements via Rule 1. We only force the
/// split when `yield` is a genuine keyword.
fn is_restricted_keyword_token(tok: &Token) -> bool {
    tok.type_ == TokenType::Keyword && RESTRICTED_KEYWORDS.contains(&tok.value.as_str())
}

/// A member-access dot (`.`) or optional-chaining (`?.`) — the token that, when
/// it *precedes* a restricted keyword, demotes it to an ordinary property name.
fn is_member_access_dot(tok: &Token) -> bool {
    tok.value == "." || tok.value == "?." || matches!(tok.type_, TokenType::Dot)
}

/// Does `tok` already terminate the statement (`;`, `}`) or mark the keyword as a
/// property key / label (`:`)? In any of these cases the restricted-production
/// insertion is unnecessary or wrong, so we decline it.
fn ends_or_keys_statement(tok: &Token) -> bool {
    matches!(tok.type_, TokenType::Semicolon | TokenType::RBrace | TokenType::Colon)
        || tok.value == ";"
        || tok.value == "}"
        || tok.value == ":"
}

/// Does an ASI insertion apply *before the token at `idx`* (which the parser
/// rejected while expecting a `SEMICOLON`)? Two ECMAScript §12.10 rules:
///
/// * **Rule 2** — the offending token is a `}` or end-of-input. A statement may
///   always be terminated there; no line terminator is required.
/// * **Rule 1** — the offending token is **preceded by a line terminator**. The
///   lexer records this precisely on the token's `flags`
///   ([`TOKEN_PRECEDED_BY_NEWLINE`]) — set only when a line terminator was
///   consumed as *trivia* before the token, so it is correct even after a
///   multi-line string/template (a newline *inside* a token does not count).
///   This supersedes the earlier start-line-arithmetic heuristic and its
///   multi-line caveats.
///
/// Soundness: this is only ever consulted on a genuine parse *failure*, so a
/// program that already parses is untouched. Requiring an actual line
/// terminator for the non-`}`/EOF case is what keeps a true one-line syntax
/// error (`a=1 b=2`) from being silently "recovered".
fn asi_applies_at(tokens: &[Token], idx: usize) -> bool {
    let off = &tokens[idx];

    // Rule 2: before a `}` or EOF.
    if off.type_ == TokenType::RBrace
        || off.type_name.as_deref() == Some("RBRACE")
        || off.value == "}"
        || off.type_ == TokenType::Eof
    {
        return true;
    }

    // Rule 1: the offending token is preceded by a line terminator.
    off.flags.unwrap_or(0) & TOKEN_PRECEDED_BY_NEWLINE != 0
}

/// A synthesized `;` token positioned at the offending token. The parser matches
/// tokens by `type_name` first, so `type_name = "SEMICOLON"` is what makes this
/// satisfy a `SEMICOLON` grammar reference; `type_` and `value` are filled in to
/// match a real semicolon for any consumer that inspects them.
fn synthetic_semicolon(offending: &Token) -> Token {
    Token {
        type_: TokenType::Semicolon,
        value: ";".to_string(),
        line: offending.line,
        column: offending.column,
        type_name: Some("SEMICOLON".to_string()),
        flags: None,
        // Synthetic (ASI-inserted) token: it corresponds to no source bytes, so
        // it carries no correlation-vector id.
        cv: None,
    }
}

/// Locate the offending token in the stream. We match on **full token
/// identity** — kind (`type_`), text (`value`), and position (`line`,
/// `column`) — not position alone. This matters because the synthetic `;` we
/// insert is deliberately stamped with the offending token's own `(line,
/// column)` (good for diagnostics/source maps), so a position-only match could
/// later find that injected `;` instead of a real `}`/EOF. A `;` differs from a
/// `}`/EOF in `type_` and `value`, so including those fields keeps the match
/// unambiguous even when positions collide. Returns the index to insert the
/// synthetic `;` *before*.
fn find_token_index(tokens: &[Token], offending: &Token) -> Option<usize> {
    tokens.iter().position(|t| {
        t.line == offending.line
            && t.column == offending.column
            && t.type_ == offending.type_
            && t.value == offending.value
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(src: &str) -> Vec<Token> {
        coding_adventures_javascript_lexer::tokenize_javascript_typed(src, EsVersion::Es2025)
            .expect("tokenize")
    }

    /// Count how many SEMICOLON tokens ASI would add by diffing the stream the
    /// parser ultimately accepts against the original. We re-run the insertion
    /// loop indirectly via `parse_with_asi`; here we just assert parse success.
    fn parses_with_asi(src: &str) -> bool {
        parse_with_asi(tokenize(src), EsVersion::Es2025).is_ok()
    }

    fn parses_without_asi(src: &str) -> bool {
        let grammar = crate::_grammar::parser_grammar("es2025").unwrap();
        GrammarParser::new(tokenize(src), grammar).parse().is_ok()
    }

    #[test]
    fn return_before_close_brace_is_recovered() {
        // No `;` before the `}` — fails without ASI, parses with it.
        let src = "function f(){return 1}";
        assert!(!parses_without_asi(src), "precondition: should fail raw");
        assert!(parses_with_asi(src), "ASI should insert a semicolon before the close brace");
    }

    #[test]
    fn call_before_close_brace_is_recovered() {
        let src = "function f(){g()}";
        assert!(!parses_without_asi(src));
        assert!(parses_with_asi(src));
    }

    #[test]
    fn expression_statement_at_eof_is_recovered() {
        // Top-level expression statement with no trailing `;` at end of input.
        let src = "a()";
        assert!(!parses_without_asi(src));
        assert!(parses_with_asi(src));
    }

    #[test]
    fn already_valid_input_is_a_noop_passthrough() {
        // With explicit semicolons the program already parses; ASI must not
        // change anything — it parses, and the FIRST parse (no insertion) wins.
        let src = "function f(){return 1;}";
        assert!(parses_without_asi(src), "precondition: valid as-is");
        assert!(parses_with_asi(src));
    }

    #[test]
    fn idempotent_on_recovered_input() {
        // Running ASI is stable: a stream that needed one insertion still parses
        // (and the inserted `;` doesn't trigger further spurious insertions).
        let src = "function f(){if(x){return 1}else{g()}}";
        assert!(parses_with_asi(src));
    }

    #[test]
    fn genuine_syntax_error_is_not_papered_over() {
        // A real error (mismatched paren) is NOT a missing-semicolon case, so
        // ASI must return the error rather than loop or wrongly "recover".
        let src = "function f({)}";
        assert!(!parses_with_asi(src), "non-ASI syntax error must still fail");
    }

    #[test]
    fn empty_block_is_not_given_a_semicolon() {
        // `function f(){}` already parses; ASI must not insert into `{}`.
        let src = "function f(){}";
        assert!(parses_without_asi(src));
        assert!(parses_with_asi(src));
    }

    // --- Phase 2: the line-terminator rule (ECMAScript Rule 1) ------------

    #[test]
    fn statements_separated_by_a_newline_are_recovered() {
        // `a = 1` then `b = 2` on the next line: the offending token `b` is
        // preceded by a line terminator, so ASI inserts a `;` after `1`.
        let src = "a = 1\nb = 2";
        assert!(!parses_without_asi(src), "precondition: should fail raw");
        assert!(parses_with_asi(src), "ASI Rule 1 should split across the newline");
    }

    #[test]
    fn two_statements_on_one_line_are_not_recovered() {
        // No line terminator between `1` and `b`: this is a genuine syntax
        // error (ASI does NOT apply mid-line), so it must stay a failure — we
        // must not paper it over.
        let src = "a = 1 b = 2";
        assert!(!parses_with_asi(src), "one-line `a=1 b=2` is a real error, not ASI");
    }

    #[test]
    fn newline_after_assignment_keyword_call() {
        // A multi-line sequence of statements with no semicolons anywhere.
        let src = "var x = f()\nvar y = g()\nh(x, y)";
        assert!(!parses_without_asi(src));
        assert!(parses_with_asi(src));
    }

    #[test]
    fn valid_multiline_program_is_a_noop() {
        // Already valid (explicit semicolons), spanning lines: ASI must not
        // change the parse — it succeeds on the first try.
        let src = "var x = 1;\nvar y = 2;\nh(x, y);";
        assert!(parses_without_asi(src), "precondition: valid as-is");
        assert!(parses_with_asi(src));
    }

    #[test]
    fn continuation_on_next_line_is_not_split() {
        // A binary expression continued on the next line is ONE statement; the
        // `+` can continue it, so the parser does not fail at `b` and ASI never
        // fires. (`a + b` is a single expression statement.) This guards against
        // wrongly splitting a legal multi-line expression.
        let src = "var a = 1, b = 2;\nvar c = a\n+ b;";
        assert!(parses_with_asi(src));
    }

    #[test]
    fn statement_ending_in_a_string_before_a_newline_is_recovered() {
        // Previously a documented limitation (the start-line heuristic could not
        // trust a string predecessor). Now that the lexer flags the offending
        // token directly, a statement ending in a string literal before a
        // newline recovers correctly — the limitation is gone.
        let src = "var s = \"x\"\nlog(s)";
        assert!(!parses_without_asi(src), "precondition: should fail raw");
        assert!(
            parses_with_asi(src),
            "string-ending statement before a newline now recovers via the flag"
        );
    }

    // --- Phase 3: restricted productions (Rule 3) ------------------------
    //
    // These are the dangerous cases: the offending stream parses *successfully
    // into the wrong tree*, so we assert on the inserted-token stream directly
    // (not just "does it parse"), and we count synthetic semicolons.

    /// How many `;` does the Phase-3 pre-pass force into `src`?
    fn forced_semis(src: &str) -> usize {
        let before = tokenize(src);
        let (after, _) = force_restricted_semicolons(before.clone());
        after.len() - before.len()
    }

    #[test]
    fn return_then_newline_argument_is_split() {
        // `return ⏎ a + b` is `return; a + b` in JS, NOT `return a + b`. The
        // grammar is newline-blind and would mis-merge it, so Phase 3 must force
        // a `;` right after `return`.
        let src = "function f(){return\na + b}";
        assert_eq!(forced_semis(src), 1, "exactly one `;` forced after return");
        // And the split makes the (otherwise mis-parsing-into-wrong-tree) input
        // parse as two statements.
        assert!(parses_with_asi(src));
    }

    #[test]
    fn return_with_same_line_argument_is_untouched() {
        // The byte-identity lever: a valid single-line `return a + b;` has no
        // newline after `return`, so Phase 3 forces nothing.
        let src = "function f(){return a + b;}";
        assert_eq!(forced_semis(src), 0, "valid same-line return is untouched");
        assert!(parses_without_asi(src), "precondition: already valid");
    }

    #[test]
    fn throw_break_continue_after_newline_are_split() {
        // All five restricted keywords behave alike: a newline immediately after
        // ends the statement. (`throw;` is itself illegal, but forcing the split
        // is still correct — closurec then declines to mis-merge and the illegal
        // input degrades to a byte-identical passthrough.)
        assert_eq!(forced_semis("throw\ne"), 1, "throw split");
        assert_eq!(forced_semis("for(;;){break\nx}"), 1, "break split");
        assert_eq!(forced_semis("for(;;){continue\nx}"), 1, "continue split");
    }

    #[test]
    fn member_access_named_return_is_not_split() {
        // `a.return` names a *property*; the `.` demotes the keyword. Even with a
        // newline after it, Phase 3 must not insert.
        assert_eq!(forced_semis("a.return\nb"), 0, "property access, not a keyword");
        assert_eq!(forced_semis("a?.return\nb"), 0, "optional-chained property");
    }

    #[test]
    fn property_key_named_return_is_not_split() {
        // `{return: 1}` uses `return` as an object key — the following `:` marks
        // it, so Phase 3 declines (guard 4).
        assert_eq!(forced_semis("x = {return:\n1}"), 0, "object key, not a keyword");
    }

    #[test]
    fn return_then_close_brace_is_not_double_inserted() {
        // `return ⏎ }` already terminates at the `}` (Rule 2 covers it); Phase 3
        // must not also force a `;`, which would be redundant.
        assert_eq!(forced_semis("function f(){return\n}"), 0, "close brace already terminates");
        // Still parses (empty return via Rule 2 on the `}`).
        assert!(parses_with_asi("function f(){return\n}"));
    }

    #[test]
    fn restricted_split_is_idempotent() {
        // Running the pre-pass twice inserts no more than running it once: after
        // the first `;`, the keyword's next token is the synthetic `;` (which
        // `ends_or_keys_statement` declines), so the second pass is a no-op.
        let (once, _) = force_restricted_semicolons(tokenize("function f(){return\na}"));
        let (twice, _) = force_restricted_semicolons(once.clone());
        assert_eq!(once.len(), twice.len(), "Phase 3 pre-pass is idempotent");
    }

    #[test]
    fn no_restricted_keyword_is_an_allocation_free_passthrough() {
        // The fast path: a stream with no restricted keyword returns unchanged.
        let src = "var a = 1\nvar b = 2";
        assert_eq!(forced_semis(src), 0);
    }

    // ---- #17840: insertion points for token-preserving consumers ---------

    /// The input index of every token, for readable assertions.
    fn insertion_points(src: &str) -> (Vec<usize>, Vec<String>) {
        let tokens = tokenize(src);
        let values = tokens.iter().map(|t| t.value.clone()).collect();
        let points = asi_insertion_points(tokens, EsVersion::Es2025).expect("parses");
        (points, values)
    }

    #[test]
    fn insertion_points_before_a_newline_led_statement_and_at_eof() {
        // a()⏎b() → a();b(); — one `;` before `b`, one at the end.
        let (points, values) = insertion_points("a()\nb()");
        let b = values.iter().position(|v| v == "b").unwrap();
        assert_eq!(points.first(), Some(&b));
        assert_eq!(points.len(), 2, "{points:?} over {values:?}");
    }

    #[test]
    fn insertion_points_are_empty_for_a_fully_terminated_program() {
        let (points, _) = insertion_points("a();b();");
        assert!(points.is_empty(), "{points:?}");
    }

    #[test]
    fn insertion_points_cover_restricted_productions() {
        // return⏎a → return;a — the pre-pass `;` goes right after `return`.
        let (points, values) = insertion_points("function f(){return\na}");
        let a = values.iter().position(|v| v == "a").unwrap();
        assert!(points.contains(&a), "{points:?} over {values:?}");
    }

    #[test]
    fn insertion_points_match_what_parse_with_asi_inserts() {
        // Re-inserting at the reported points reproduces a stream the plain
        // parser accepts with no further help.
        let src = "var x = 1\nvar y = 2\nif (x) { y++ }\nfunction g() { return\nx }";
        let tokens = tokenize(src);
        let points = asi_insertion_points(tokens.clone(), EsVersion::Es2025).unwrap();
        let mut rebuilt = Vec::new();
        let mut p = points.iter().peekable();
        for (i, t) in tokens.iter().enumerate() {
            while p.peek() == Some(&&i) {
                rebuilt.push(synthetic_semicolon(t));
                p.next();
            }
            rebuilt.push(t.clone());
        }
        let grammar = crate::_grammar::parser_grammar("es2025").unwrap();
        assert!(GrammarParser::new(rebuilt, grammar).parse().is_ok());
    }

    #[test]
    fn newline_led_increment_after_an_expression_starts_a_new_statement() {
        // x = 1⏎++y → x = 1; ++y (postfix `++` can't follow a line break).
        assert!(parses_with_asi("x = 1\n++y"));
        assert_eq!(forced_semis("x = 1\n++y"), 1);
        assert_eq!(forced_semis("a()\n--b"), 1);
        assert_eq!(forced_semis("a[0]\n++b"), 1);
    }

    #[test]
    fn increment_after_an_operator_or_on_the_same_line_is_untouched() {
        // `x =⏎++y` is `x = ++y`; `x++` on one line is ordinary postfix.
        assert_eq!(forced_semis("x =\n++y"), 0);
        assert_eq!(forced_semis("f(\n++y)"), 0);
        assert_eq!(forced_semis("x++"), 0);
        assert_eq!(forced_semis("x ++\ny"), 0);
    }

    #[test]
    fn increment_after_a_statement_header_is_the_body() {
        // `if (x)⏎++y` — `++y` is the if's body (security review of #17840).
        for src in ["if (x)\n++y", "while (x)\n--y", "for (;;)\n++i", "with (o)\n++a",
                    "if (a) b(); else if (c)\n++d"] {
            assert_eq!(forced_semis(src), 0, "{src}");
            assert!(parses_with_asi(src), "{src}");
        }
        // A call's `)` still ends an expression.
        assert_eq!(forced_semis("f(x)\n++y"), 1);
        // `do…while`: nothing forced, but retry-on-error still splits it.
        assert_eq!(forced_semis("do a(); while (x)\n++y"), 0);
        assert!(parses_with_asi("do a(); while (x)\n++y"));
    }

    /// The speculation is an accelerator only: on every input, the final
    /// token stream must equal what plain retry-on-error produces.
    #[test]
    fn speculation_agrees_with_plain_retry() {
        let grammar = crate::_grammar::parser_grammar("es2025").unwrap();
        for src in [
            "a()\nb()\nc = 1\nvar d = 2\nif (d) e()\nfunction f() { return g() }\nf()",
            "class P { m() { a() }\nn() { b() } }\nnew P()",
            "x = 1\n++y\nwhile (x)\n--y\nfor (;;) { break }",
            "var o = { a: 1, b: function () { return 2 } }\no.b()\nthrow o",
            "try { a() } catch (e) { b() } finally { c() }\nlabel: for (;;) { continue label }",
            "do a()\nwhile (b)\nswitch (a) { case 1: b()\ndefault: c() }",
            "let s = `t`\nlet r = /x/g\nconst q = a ? b : c\nx = () => { y() }",
            "async function f(y) { for await (const x of y)\ng(x) }\nx = {}\ny()\nimport z\nfrom \"m\"",
        ] {
            let (tokens, original) = force_restricted_semicolons(tokenize(src));
            let (plain, _) = retry_on_error(tokens.clone(), original.clone(), &grammar).expect("plain parses");
            let (spec, spec_original) = speculate_line_semicolons(&tokens, &original);
            let (fast, _) = retry_on_error(spec, spec_original, &grammar).expect("speculated parses");
            assert_eq!(fast, plain, "{src}");
        }
    }

    #[test]
    fn for_await_header_is_a_header() {
        // #17840 review: `for await (…)⏎g(x)` — `g(x)` is the loop's body.
        for src in [
            "async function f(y){for await (const x of y)\ng(x)}",
            "async function f(y){for await (const x of y)\n++n}",
        ] {
            let (tokens, original) = force_restricted_semicolons(tokenize(src));
            assert_eq!(tokens.len(), original.len());
            assert_eq!(forced_semis(src), 0, "{src}");
            let (spec, _) = speculate_line_semicolons(&tokens, &original);
            assert_eq!(spec.len(), tokens.len() + 1, "only the body-final `;` before `}}`: {src}");
        }
    }

    #[test]
    fn contextual_from_and_as_are_not_statement_starts() {
        for src in ["import x\nfrom \"m\"", "import {a\nas b} from \"m\""] {
            let tokens = tokenize(src);
            let original = vec![true; tokens.len()];
            let (spec, _) = speculate_line_semicolons(&tokens, &original);
            assert_eq!(spec.len(), tokens.len(), "{src}");
        }
    }

    #[test]
    fn object_literal_close_brace_ends_an_expression() {
        // `x = {}⏎y()` needs the `;`; a block `}` doesn't get one.
        let tokens = tokenize("x = {}\ny()");
        let original = vec![true; tokens.len()];
        assert_eq!(speculate_line_semicolons(&tokens, &original).0.len(), tokens.len() + 1);
        let tokens = tokenize("if (a) {}\ny()");
        let original = vec![true; tokens.len()];
        assert_eq!(speculate_line_semicolons(&tokens, &original).0.len(), tokens.len());
    }
}

