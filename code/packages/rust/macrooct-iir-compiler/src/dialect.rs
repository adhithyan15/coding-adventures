//! # `MacroOctDialect` — the per-language half of MacroOct's preprocessor.
//!
//! The [`source_preprocessor`](coding_adventures_source_preprocessor) engine
//! owns everything that is the same in every preprocessor: the source map,
//! include resolution, the conditional stack and the resource bounds. This
//! module owns everything that is not — which for MacroOct is three questions:
//!
//! | Question | Answered by |
//! |---|---|
//! | Is this line a directive, and which? | [`MacroOctDialect::classify`] |
//! | What does `@if LED == 1` mean? | [`MacroOctDialect::eval_condition`] |
//! | How is an included file tokenized? | [`MacroOctDialect::lex`] |
//!
//! and two it deliberately declines to answer: `stringize` and `paste` are
//! left at the trait's `None` defaults. MacroOct has neither `#` nor `##`, and
//! that absence is itself a test — an engine that assumed every dialect had
//! them would not compile a MacroOct program at all.
//!
//! ## What makes this dialect worth writing
//!
//! Nothing here is C-shaped, on purpose. The directives are `@`-prefixed, the
//! conditional terminator is `@end` rather than `@endif`, and there is no
//! `defined()` operator because there are no macros yet. If the engine had
//! quietly hardcoded C's vocabulary anywhere, MacroOct would not work — which
//! is the point of proving the engine here rather than on C, the customer that
//! motivated it. A customer is exactly what should not get to design the
//! boundary.
//!
//! ## The two total-ness rules this module obeys
//!
//! The engine's contract is that **no input, however malformed, may cause a
//! panic, an abort, an out-of-bounds index or a non-terminating loop.** That
//! contract reaches into the dialect, because the dialect is what parses the
//! controlling expression. So:
//!
//! 1. **No recursion.** [`MacroOctDialect::eval_condition`] is an explicit
//!    two-stack (shunting-yard) evaluator, not a recursive-descent parser. In
//!    Rust a stack overflow is an *abort* — not a catchable panic, and not
//!    containable by `catch_unwind` in an embedding host — so a recursive
//!    evaluator would convert the engine's `condition_depth` bound from a
//!    diagnostic into a process kill. The engine does pre-scan the token slice
//!    for grouping depth before calling, but relying on that would make this
//!    module correct only by someone else's diligence.
//! 2. **No indexing that can be out of range, and no arithmetic that can
//!    overflow.** Integer literals are validated into `0..=255` on the way in
//!    (MacroOct is Oct, whose only integer type is the 8008's byte), so every
//!    value in flight fits an `i64` with room to spare and no operation here
//!    can wrap.

use coding_adventures_macrooct_lexer::try_tokenize_macrooct;
use coding_adventures_source_preprocessor::{
    Bounds, Dialect, Directive, FileId, IncludeRequest, PpError,
};
use lexer::token::{Token, TokenType};

/// MacroOct's directive syntax and conditional semantics.
///
/// Stateless: every answer is a pure function of the tokens handed in. The
/// engine owns all the state there is (the include stack, the conditional
/// stack, the spend counters), which is what lets one `MacroOctDialect` be
/// shared across translation units without leaking anything between them.
#[derive(Debug, Default, Clone, Copy)]
pub struct MacroOctDialect;

/// Drop the `GrammarLexer`'s trailing end-of-stream sentinel.
///
/// The lexer appends an `Eof` token to every stream it produces. That is
/// harmless for a whole-file parse and actively wrong in two places here:
///
/// - An `Eof` on the same physical line as a closing `@end` joins that
///   directive's logical line, so the directive would look like `@end` plus
///   trailing junk — which a source file with no final newline produces every
///   time.
/// - An `Eof` from an `@include`d file would be spliced into the **middle** of
///   the token stream, where Oct's parser would stop early and silently
///   compile a truncated program.
///
/// The second is the dangerous one, and neither is a case the engine can
/// handle for us: "does this language's lexer append a sentinel?" is exactly
/// the kind of per-language fact a generic engine must not know.
pub(crate) fn strip_eof(tokens: &mut Vec<Token>) {
    while tokens.last().is_some_and(|t| t.type_ == TokenType::Eof) {
        tokens.pop();
    }
}

/// The run of tokens the engine hands to `classify` may still carry the
/// sentinel if a caller preprocessed a stream without stripping it (this
/// crate's own pipeline does strip it, but `MacroOctDialect` is public API).
/// Ignoring it here makes the dialect total for that caller too, instead of
/// rejecting a perfectly good `@end` with a confusing complaint about junk.
fn without_eof(line: &[Token]) -> &[Token] {
    let mut end = line.len();
    while end > 0 && line[end - 1].type_ == TokenType::Eof {
        end -= 1;
    }
    &line[..end]
}

/// Position a diagnostic at a token, for messages the engine cannot locate
/// itself. The engine attaches the directive's own position to anything we
/// return without one, so this is only about pointing at the *operand*.
fn describe(token: Option<&Token>) -> String {
    match token {
        // Quoted, not `{:?}`. Debug-escaping handles control characters, but
        // it does not TRUNCATE -- and it can expand a hostile spelling roughly
        // sixfold on the way. A token's value is bounded by the source, which
        // is bounded by the engine, but a diagnostic that renders a whole
        // 64 KiB token is still a denial-of-service and a disclosure channel
        // on a shared builder. `PpError::quote` does both jobs, and using it
        // here keeps this path consistent with every other one.
        Some(t) => format!(
            "{} at line {}, column {}",
            PpError::quote(&t.value, Bounds::default().diagnostic_quote_bytes),
            t.line,
            t.column
        ),
        None => "end of line".to_string(),
    }
}

/// Catch a misspelled directive before it is read as a correct one.
///
/// ## The trap this closes
///
/// The directive rules in `macrooct.tokens` are **literal** patterns, and the
/// lexer matches a literal by prefix with no word-boundary requirement. So
/// `@endif` — the spelling a C programmer types by reflex — does not fail to
/// lex. It lexes as `AT_END` followed by `NAME("if")`, and `@ifdef` lexes as
/// `AT_IF` followed by `NAME("def")`.
///
/// Some of those land somewhere safe by luck: `@endif` looks like `@end` with
/// an operand, which is already refused. `@ifdef FOO` looks like the condition
/// `def FOO`, two values in a row, also refused. But `@ifdef` **alone** is the
/// condition `def` — a single undefined name, which evaluates to 0 — so the
/// group is silently skipped with no diagnostic anywhere. `@if1` is worse
/// still: it reads as `@if 1` and silently *takes* the branch.
///
/// So this checks for a token glued directly to the directive's last character
/// with no space between, and refuses it. Adjacency is the whole test, which
/// is why it is done on columns: `@end if` is a legitimate mistake to report
/// as "takes no operands", while `@endif` is a misspelling and deserves to be
/// named as one.
///
/// Only an alphanumeric or `_` counts as glued. `@if(1)` and `@include"x.oct"`
/// are ordinary, correct MacroOct with the space omitted, and must keep
/// working.
///
/// ## Why here and not in the grammar
///
/// The grammar could say `/@if\b/` instead of `"@if"`, and the word boundary
/// would make `@ifdef` fail to lex at all. That is a defensible alternative
/// and was not chosen for two reasons: it would make the directive rules the
/// only regexes in a section of literals, for a reason invisible at the point
/// of definition; and a lex error would say "unexpected sequence `@`", which
/// is a worse message than the one below. A misspelled directive should be
/// told it is a misspelled directive.
fn glued_suffix<'a>(head: &Token, rest: &'a [Token]) -> Option<&'a Token> {
    let next = rest.first()?;
    if next.line != head.line {
        return None;
    }
    // `head.value` is one of five ASCII literals, so `len()` is its width.
    if next.column != head.column + head.value.len() {
        return None;
    }
    next.value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        .then_some(next)
}

impl Dialect for MacroOctDialect {
    /// Classify one logical line.
    ///
    /// The engine has already grouped the tokens sharing a source line (see
    /// the module docs on `macrooct-lexer` for why `Token::line` is what
    /// delimits a line, given that MacroOct skips newlines). All we do is look
    /// at the first token.
    ///
    /// `None` means ordinary source, which the engine passes through
    /// untouched — or drops, if the line sits inside a conditional group that
    /// is not being taken.
    fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>> {
        let line = without_eof(line);
        let head = line.first()?;
        let rest = &line[1..];

        // A misspelling is not a directive with an operand. Checked before the
        // dispatch below so `@ifdef` cannot reach the `@if` arm and be read as
        // a condition -- see `glued_suffix` for why that specific spelling is
        // the dangerous one.
        if matches!(head.value.as_str(), "@include" | "@define" | "@if" | "@else" | "@end") {
            if let Some(glued) = glued_suffix(head, rest) {
                let spelling = format!("{}{}", head.value, glued.value);
                let hint = if spelling == "@endif" {
                    // The spelling a C programmer types by reflex, and the one
                    // PREP01 chose `@end` specifically to diverge from. Worth
                    // its own sentence rather than a generic list.
                    " — MacroOct spells the conditional terminator `@end`, not `@endif`"
                } else {
                    " — MacroOct's directives are `@include`, `@define`, `@if`, `@else` and `@end`"
                };
                // Quoted for the length cap. There is no injection channel
                // here — `glued_suffix` only admits alphanumeric token starts —
                // but a 4000-character identifier still produced a 4 KB
                // diagnostic, and the cap is the point.
                let shown =
                    PpError::quote(&spelling, Bounds::default().diagnostic_quote_bytes);
                return Some(Err(PpError::new(format!(
                    "unknown directive `{shown}` at line {}, column {}{hint}",
                    head.line, head.column
                ))));
            }
        }

        // Dispatch on the token's *value*. It could equally dispatch on
        // `type_name` (`AT_IF` and friends), and the two agree by construction
        // — the directive rules in `macrooct.tokens` are literal patterns, so
        // a token of type `AT_IF` always has value `"@if"`. Value is used
        // because it is what a reader of this file can check against the
        // grammar without holding both open.
        let directive = match head.value.as_str() {
            "@include" => return Some(include_directive(rest)),
            "@if" => {
                if rest.is_empty() {
                    return Some(Err(PpError::new(
                        "`@if` needs a controlling expression on the same line",
                    )));
                }
                Directive::If(rest.to_vec())
            }
            "@else" => {
                if let Some(extra) = rest.first() {
                    return Some(Err(PpError::new(format!(
                        "`@else` takes no operands, but found {}",
                        describe(Some(extra))
                    ))));
                }
                Directive::Else
            }
            // `@end`, not `@endif`. See the module header: the divergence is
            // the point. An engine carrying a hardcoded C vocabulary would
            // handle `@if`/`@else` happily and then fail right here, in the
            // first dialect rather than the fourth.
            "@end" => {
                if let Some(extra) = rest.first() {
                    return Some(Err(PpError::new(format!(
                        "`@end` takes no operands, but found {}",
                        describe(Some(extra))
                    ))));
                }
                Directive::EndIf
            }
            "@define" => return Some(define_directive(rest)),
            // Everything else is ordinary Oct source.
            _ => return None,
        };
        Some(Ok(directive))
    }

    /// Evaluate a conditional's controlling expression.
    ///
    /// See [`eval`] for the grammar and the algorithm.
    fn eval_condition(&self, tokens: &[Token]) -> Result<bool, PpError> {
        eval(without_eof(tokens)).map(|v| v != 0)
    }

    /// Lex an included file's text with MacroOct's own grammar.
    ///
    /// Recursively, in the useful sense: an included file may itself contain
    /// directives, and gets them. (The *engine* is not recursive — it pushes a
    /// frame onto an explicit stack — which is what keeps a deep include chain
    /// a diagnostic rather than a stack overflow.)
    ///
    /// The `Result` matters here rather than being boilerplate: the text of an
    /// included file is chosen by the program being compiled, so a lex failure
    /// in it is reachable from input and must be a diagnostic, never a panic.
    /// That is why this calls `try_tokenize_macrooct` and not the panicking
    /// `tokenize_macrooct` that mirrors `oct-lexer`'s shape.
    fn lex(&self, text: &str, _file: FileId) -> Result<Vec<Token>, PpError> {
        let mut tokens = try_tokenize_macrooct(text).map_err(PpError::new)?;
        strip_eof(&mut tokens);
        Ok(tokens)
    }

    // `stringize` and `paste` are deliberately left at the trait's `None`
    // defaults. MacroOct has no `#` and no `##`, and declining them is a
    // positive test that the engine does not assume every dialect has a
    // C-shaped macro facility. Slice 2 keeps this declined; the C dialect in
    // slice 4 is where those hooks get their first real implementation.
}

// ===========================================================================
// @include
// ===========================================================================

/// `@include "path"` — exactly one string-literal operand.
///
/// The quotes are stripped here, in the dialect, because they are *lexical*:
/// `STR_LIT` is a MacroOct token, and the engine has no idea what a string
/// looks like in any language. `IncludeRequest::from` is left `None`; the
/// engine overwrites it with the including file's id, which the dialect does
/// not know and should not.
///
/// `system` is `false` unconditionally. That field exists for C's `<…>` versus
/// `"…"` split, and MacroOct has only one form — a shape worth noticing, since
/// it means the request type is already carrying a distinction its first two
/// dialects do not use.
fn include_directive(rest: &[Token]) -> Result<Directive, PpError> {
    let Some(operand) = rest.first() else {
        return Err(PpError::new(
            "`@include` needs a quoted path, e.g. `@include \"ports.macrooct\"`",
        ));
    };
    if let Some(extra) = rest.get(1) {
        return Err(PpError::new(format!(
            "`@include` takes exactly one quoted path, but found {}",
            describe(Some(extra))
        )));
    }

    // Length >= 2 as well as the two quote checks: a lone `"` would otherwise
    // pass "starts with a quote and ends with a quote" and then panic on the
    // slice below. `STR_LIT` cannot produce one, but this function is reached
    // from `Dialect`, which anybody can call with anything.
    let text = &operand.value;
    let quoted = text.len() >= 2 && text.starts_with('"') && text.ends_with('"');
    if !quoted {
        return Err(PpError::new(format!(
            "`@include` needs a quoted path, but found {}",
            describe(Some(operand))
        )));
    }
    let spelling = text[1..text.len() - 1].to_string();

    // Path *safety* is not checked here, and that division is deliberate: the
    // engine hands this spelling to a `SourceFs`, and `RootedFs` is the single
    // auditable place where containment, symlinks, UNC paths, device names and
    // regular-file-ness are decided. A dialect that screened paths itself
    // would create a second, weaker gate that reviewers would mistake for the
    // real one.
    Ok(Directive::Include(IncludeRequest { spelling, from: None, system: false }))
}

// ===========================================================================
// @define
// ===========================================================================

/// `@define NAME body…`
///
/// Classified, and then **refused by the engine** — slice 1 has no macro
/// table, and the engine answers `Directive::Define` with a located "not
/// supported yet" diagnostic.
///
/// Recognising a directive the engine will refuse looks redundant and is not.
/// If the dialect returned `None` here, `@define LED 1` would be classified as
/// ordinary Oct source, handed to Oct's parser, and rejected with a syntax
/// error about `@` — a message that points at the lexer rather than at the
/// feature the author was reaching for. The refusal says what is actually
/// true: the directive exists and is not implemented yet.
fn define_directive(rest: &[Token]) -> Result<Directive, PpError> {
    let Some(name) = rest.first() else {
        return Err(PpError::new("`@define` needs a macro name"));
    };
    // A macro name is an identifier. Rejecting `@define 1 2` here rather than
    // letting slice 2 discover it keeps the diagnostic close to the mistake.
    let is_identifier = name
        .value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !is_identifier {
        return Err(PpError::new(format!(
            "`@define` needs an identifier for a macro name, but found {}",
            describe(Some(name))
        )));
    }
    Ok(Directive::Define { name: name.value.clone(), body: rest[1..].to_vec() })
}

// ===========================================================================
// Controlling expressions
// ===========================================================================

/// Binary operators MacroOct's conditionals understand, and nothing else.
///
/// Note what is *absent*: no `+`, no `&`, no `!`, no `defined()`. Slice 1's
/// conditionals exist to select between alternatives, and every operator added
/// here is one more thing that has to agree with Oct's own semantics forever.
/// Anything unsupported is a `PpError` naming the token — never a guess, and
/// never a silent zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    /// Not an operator — the grouping sentinel that stops the stack unwind at
    /// an open parenthesis.
    LParen,
}

impl Op {
    /// Binding power. Higher binds tighter.
    ///
    /// The layering matches Oct's own expression grammar, which matters more
    /// than it might look: `@if x == 1 && y == 2` has to mean what the same
    /// text means inside an Oct `if`, or a program would select a branch its
    /// author read as the other one.
    fn precedence(self) -> u8 {
        match self {
            Op::LParen => 0,
            Op::Or => 1,
            Op::And => 2,
            Op::Eq | Op::Ne | Op::Lt | Op::Gt | Op::Le | Op::Ge => 3,
        }
    }

    fn from_token(value: &str) -> Option<Op> {
        Some(match value {
            "||" => Op::Or,
            "&&" => Op::And,
            "==" => Op::Eq,
            "!=" => Op::Ne,
            "<" => Op::Lt,
            ">" => Op::Gt,
            "<=" => Op::Le,
            ">=" => Op::Ge,
            _ => return None,
        })
    }

    /// Apply to two already-evaluated operands.
    ///
    /// Total by construction: both inputs are in `0..=255` (see
    /// [`operand_value`]), so nothing here can overflow, divide by zero or
    /// index anything. `&&`/`||` are *not* short-circuiting, and cannot be:
    /// by the time an operator is applied its right operand has already been
    /// evaluated. That is invisible in slice 1 — evaluating an operand has no
    /// side effects and cannot fail — but it is a real difference from Oct's
    /// runtime `&&`, and the place it would start to matter is a future
    /// `defined()`-style operator.
    fn apply(self, left: i64, right: i64) -> i64 {
        let b = |v: bool| i64::from(v);
        match self {
            Op::Or => b(left != 0 || right != 0),
            Op::And => b(left != 0 && right != 0),
            Op::Eq => b(left == right),
            Op::Ne => b(left != right),
            Op::Lt => b(left < right),
            Op::Gt => b(left > right),
            Op::Le => b(left <= right),
            Op::Ge => b(left >= right),
            // Unreachable by construction: `LParen` is never pushed as an
            // applicable operator, it is popped as a sentinel. Written as a
            // value rather than `unreachable!()` because this module's whole
            // job is to be total, and a panic here would be one an attacker
            // only has to find one way to reach.
            Op::LParen => 0,
        }
    }
}

/// Value of a single operand token, or an error naming it.
///
/// Recognised:
///
/// | Token | Value |
/// |---|---|
/// | `0`…`255` | itself |
/// | `0xFF`, `0b1010` | itself — Oct's own literal spellings |
/// | `true` / `false` | 1 / 0 |
/// | any other identifier | **0** — undefined |
///
/// **Undefined is 0, and that is a placeholder, not a design.** It is the C
/// rule, and it is the only answer available in a slice with no macro table:
/// until `@define` lands there is nothing that could ever make a name defined,
/// so "undefined" is the *only* state a name can be in. Slice 2 replaces this
/// with a real lookup.
///
/// `true`/`false` are handled explicitly rather than falling through to the
/// undefined-identifier rule, because falling through would make `@if true`
/// quietly false — a trap with no diagnostic, in a language whose `true` is a
/// keyword the author can see the compiler understands everywhere else.
fn operand_value(token: &Token) -> Result<i64, PpError> {
    let text = token.value.as_str();
    match text {
        "true" => return Ok(1),
        "false" => return Ok(0),
        _ => {}
    }

    let parsed = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(i64::from_str_radix(hex, 16))
    } else if let Some(bin) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        Some(i64::from_str_radix(bin, 2))
    } else if text.starts_with(|c: char| c.is_ascii_digit()) {
        Some(text.parse::<i64>())
    } else {
        None
    };

    if let Some(parsed) = parsed {
        // A literal that does not fit a byte is refused rather than truncated.
        // Oct's only integer type is the 8008's byte and Oct itself rejects an
        // out-of-range literal at compile time; a conditional that silently
        // read `300` as `44` would make `@if PORT == 300` select a branch for
        // a reason nobody could find. `parse` failing (overflowing `i64`,
        // trailing junk) lands in the same arm, which is why this matches on
        // the `Result` rather than unwrapping it.
        return match parsed {
            Ok(v) if (0..=255).contains(&v) => Ok(v),
            _ => Err(PpError::new(format!(
                "integer literal {} is out of range for MacroOct's u8 conditionals (0-255)",
                describe(Some(token))
            ))),
        };
    }

    // An identifier. Undefined until slice 2 gives names a way to be defined.
    if text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Ok(0);
    }

    Err(PpError::new(format!(
        "{} is not a value MacroOct's conditionals understand — expected an \
         integer 0-255, `true`, `false`, or a name",
        describe(Some(token))
    )))
}

/// Evaluate a controlling expression to an integer.
///
/// The grammar, in the order the two stacks below enforce:
///
/// ```text
///   or      := and ( "||" and )*
///   and     := cmp ( "&&" cmp )*
///   cmp     := operand ( ("=="|"!="|"<"|">"|"<="|">=") operand )*
///   operand := INT | HEX | BIN | "true" | "false" | NAME | "(" or ")"
/// ```
///
/// ## Why shunting-yard and not recursive descent
///
/// Recursive descent is the obvious way to write this and the wrong one here.
/// A dialect parses attacker-influenced text, and in Rust a stack overflow is
/// an **abort** — uncatchable, and fatal to the embedding host. The engine does
/// pre-scan the slice's grouping depth before calling, precisely so a dialect
/// need not defend itself, but a dialect whose safety depends on a check in
/// somebody else's crate is one refactor away from unsafe. Two `Vec`s and a
/// `for` loop cannot overflow the machine stack no matter what they are fed.
///
/// ## The state machine, which is the part that catches malformed input
///
/// `expect_operand` alternates. It starts `true`, becomes `false` after a
/// value or a `)`, and returns to `true` after an operator or a `(`. Every
/// malformed shape falls out of it without a special case:
/// `1 2` (two operands), `1 ==` (ends expecting one), `== 1` (starts with an
/// operator), `()` (empty group), `(1` and `1)` (unbalanced).
fn eval(tokens: &[Token]) -> Result<i64, PpError> {
    if tokens.is_empty() {
        return Err(PpError::new("empty controlling expression"));
    }

    let mut values: Vec<i64> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();
    let mut expect_operand = true;

    // Pop one operator and apply it. Returns an error rather than panicking on
    // an operand shortfall; the state machine makes that unreachable, but
    // "unreachable" and "cannot happen" differ by one future edit.
    fn reduce(values: &mut Vec<i64>, ops: &mut Vec<Op>) -> Result<(), PpError> {
        let op = ops.pop().ok_or_else(|| PpError::new("malformed controlling expression"))?;
        let right = values.pop().ok_or_else(|| PpError::new("malformed controlling expression"))?;
        let left = values.pop().ok_or_else(|| PpError::new("malformed controlling expression"))?;
        values.push(op.apply(left, right));
        Ok(())
    }

    for token in tokens {
        match token.value.as_str() {
            "(" => {
                if !expect_operand {
                    return Err(PpError::new(format!(
                        "unexpected `(` after a value, at line {}, column {}",
                        token.line, token.column
                    )));
                }
                ops.push(Op::LParen);
            }
            ")" => {
                if expect_operand {
                    return Err(PpError::new(format!(
                        "unexpected `)` — the group has no value, at line {}, column {}",
                        token.line, token.column
                    )));
                }
                // Unwind to the matching `(`. Bounded by `ops.len()`, which
                // only ever shrinks here, so this loop terminates.
                loop {
                    match ops.last() {
                        Some(Op::LParen) => {
                            ops.pop();
                            break;
                        }
                        Some(_) => reduce(&mut values, &mut ops)?,
                        None => {
                            return Err(PpError::new(format!(
                                "unbalanced `)` at line {}, column {}",
                                token.line, token.column
                            )))
                        }
                    }
                }
            }
            other => {
                if let Some(op) = Op::from_token(other) {
                    if expect_operand {
                        return Err(PpError::new(format!(
                            "operator {} has no left operand",
                            describe(Some(token))
                        )));
                    }
                    // Left-associative, so `>=` (not `>`) is the reduce
                    // condition: `1 < 2 < 3` groups as `(1 < 2) < 3`, matching
                    // Oct's own left-to-right reading.
                    while ops.last().is_some_and(|top| top.precedence() >= op.precedence())
                        && ops.last() != Some(&Op::LParen)
                    {
                        reduce(&mut values, &mut ops)?;
                    }
                    ops.push(op);
                    expect_operand = true;
                } else {
                    if !expect_operand {
                        return Err(PpError::new(format!(
                            "unexpected {} — two values in a row, with no operator between them",
                            describe(Some(token))
                        )));
                    }
                    values.push(operand_value(token)?);
                    expect_operand = false;
                }
                continue;
            }
        }
        // Only the `(` and `)` arms reach here; both leave the machine
        // expecting what the bracket implies.
        expect_operand = token.value == "(";
    }

    if expect_operand {
        return Err(PpError::new(
            "controlling expression ends after an operator, with nothing to apply it to",
        ));
    }
    while !ops.is_empty() {
        if ops.last() == Some(&Op::LParen) {
            return Err(PpError::new("unclosed `(` in the controlling expression"));
        }
        reduce(&mut values, &mut ops)?;
    }
    match values.as_slice() {
        [only] => Ok(*only),
        _ => Err(PpError::new("malformed controlling expression")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_macrooct_lexer::tokenize_macrooct;

    /// Lex a fragment the way the engine would hand it to the dialect: the run
    /// of tokens on one line, sentinel removed.
    fn line(src: &str) -> Vec<Token> {
        let mut tokens = tokenize_macrooct(src);
        strip_eof(&mut tokens);
        tokens
    }

    fn classify(src: &str) -> Option<Result<Directive, PpError>> {
        MacroOctDialect.classify(&line(src))
    }

    fn cond(src: &str) -> Result<bool, PpError> {
        MacroOctDialect.eval_condition(&line(src))
    }

    // --- classify ----------------------------------------------------------

    #[test]
    fn ordinary_oct_source_is_not_a_directive() {
        assert!(classify("fn main() { out(1, 42); }").is_none());
        assert!(classify("let x: u8 = 1;").is_none());
        // Including a line whose first token merely *contains* a directive
        // word. `iffy` is a NAME, not `@if`.
        assert!(classify("iffy = 1;").is_none());
    }

    #[test]
    fn each_directive_is_recognised() {
        assert!(matches!(classify("@if 1"), Some(Ok(Directive::If(_)))));
        assert!(matches!(classify("@else"), Some(Ok(Directive::Else))));
        assert!(matches!(classify("@end"), Some(Ok(Directive::EndIf))));
        assert!(matches!(classify("@include \"x.macrooct\""), Some(Ok(Directive::Include(_)))));
        assert!(matches!(classify("@define A 1"), Some(Ok(Directive::Define { .. }))));
    }

    #[test]
    fn the_terminator_is_end_not_endif() {
        // The deliberate divergence from C, and the check that MacroOct does
        // not quietly accept C's spelling anyway. It would be easy to: `@end`
        // is a literal pattern, so `@endif` lexes as `@end` + `if` rather than
        // failing, and a dialect that only looked at the first token would
        // have taken it.
        assert!(matches!(classify("@end"), Some(Ok(Directive::EndIf))));
        let Some(Err(e)) = classify("@endif") else {
            panic!("`@endif` must be refused, not silently accepted or ignored");
        };
        let text = e.to_string();
        assert!(text.contains("unknown directive `@endif`"), "{text}");
        assert!(text.contains("`@end`"), "the message must name the right spelling: {text}");
    }

    #[test]
    fn a_misspelled_directive_is_refused_rather_than_read_as_a_correct_one() {
        // The silent traps, each one a real consequence of matching directive
        // literals without a word boundary:
        //
        //   `@ifdef`  → `@if` + `def`, one undefined name → 0 → group SKIPPED
        //   `@if1`    → `@if` + `1`                       → 1 → group TAKEN
        //   `@elsewhere` → `@else` + `where`
        //
        // Each of these used to reach a branch decision with no diagnostic.
        for src in ["@ifdef", "@ifdef FOO", "@if1", "@elsewhere", "@includes \"x\"", "@defines A 1"] {
            let Some(Err(e)) = classify(src) else {
                panic!("{src} must be refused as a misspelling");
            };
            assert!(
                e.to_string().contains("unknown directive"),
                "{src} -> {e}"
            );
        }
    }

    #[test]
    fn omitting_the_space_after_a_directive_still_works() {
        // The misspelling check keys on a glued *alphanumeric*, so a glued
        // bracket or quote -- ordinary MacroOct with the space left out -- has
        // to keep working. Being too aggressive here would break correct code.
        assert_eq!(cond("(1)"), Ok(true));
        assert!(matches!(classify("@if(1)"), Some(Ok(Directive::If(_)))));
        let Some(Ok(Directive::Include(r))) = classify("@include\"ports.macrooct\"") else {
            panic!("a glued quote is not a misspelling");
        };
        assert_eq!(r.spelling, "ports.macrooct");
    }

    #[test]
    fn a_word_starting_with_a_directive_name_is_still_ordinary_source() {
        // The other direction: the check must not fire on source that merely
        // *begins* with something directive-shaped but is not glued to one.
        // `@` cannot start an Oct token at all, so the realistic case is a
        // NAME like `endif` on its own.
        assert!(classify("endif = 1;").is_none());
        assert!(classify("ifdef();").is_none());
    }

    #[test]
    fn include_strips_the_quotes_and_leaves_resolution_to_the_engine() {
        let Some(Ok(Directive::Include(request))) = classify("@include \"sub/ports.macrooct\"")
        else {
            panic!("expected an include");
        };
        assert_eq!(request.spelling, "sub/ports.macrooct");
        // `from` is the engine's to fill in -- the dialect does not know which
        // file it is reading.
        assert_eq!(request.from, None);
        // MacroOct has one include form, not C's `"…"` / `<…>` pair.
        assert!(!request.system);
    }

    #[test]
    fn include_rejects_a_missing_extra_or_unquoted_operand() {
        for src in [
            "@include",                                  // nothing
            "@include \"a.macrooct\" \"b.macrooct\"",    // two paths
            "@include ports",                            // bare identifier
            "@include 42",                               // a number
        ] {
            assert!(
                matches!(classify(src), Some(Err(_))),
                "{src} must be refused with a diagnostic"
            );
        }
    }

    #[test]
    fn else_and_end_reject_trailing_operands() {
        // A line-oriented directive with junk after it is a typo, not a
        // conditional. Accepting it silently would make `@end fn main() {}`
        // discard the function.
        assert!(matches!(classify("@else 1"), Some(Err(_))));
        assert!(matches!(classify("@end 1"), Some(Err(_))));
    }

    #[test]
    fn if_without_a_condition_is_refused() {
        assert!(matches!(classify("@if"), Some(Err(_))));
    }

    #[test]
    fn define_is_classified_even_though_slice_one_refuses_it() {
        // Classifying a directive the engine will refuse is deliberate: it is
        // what turns "syntax error near @" into "macro definitions are not
        // supported yet".
        let Some(Ok(Directive::Define { name, body })) = classify("@define LED_PORT 1") else {
            panic!("expected a define");
        };
        assert_eq!(name, "LED_PORT");
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].value, "1");
    }

    #[test]
    fn define_requires_an_identifier_name() {
        assert!(matches!(classify("@define"), Some(Err(_))));
        assert!(matches!(classify("@define 1 2"), Some(Err(_))));
    }

    #[test]
    fn a_trailing_eof_sentinel_does_not_turn_a_directive_into_junk() {
        // A source file with no final newline puts the lexer's `Eof` on the
        // same line as `@end`. This crate's pipeline strips the sentinel, but
        // `MacroOctDialect` is public and must not punish a caller that did
        // not know to.
        let with_sentinel = tokenize_macrooct("@end");
        assert!(with_sentinel.iter().any(|t| t.type_ == TokenType::Eof));
        assert!(matches!(
            MacroOctDialect.classify(&with_sentinel),
            Some(Ok(Directive::EndIf))
        ));
    }

    // --- eval_condition ----------------------------------------------------

    #[test]
    fn a_bare_literal_is_true_when_nonzero() {
        assert_eq!(cond("1"), Ok(true));
        assert_eq!(cond("0"), Ok(false));
        assert_eq!(cond("255"), Ok(true));
    }

    #[test]
    fn octs_own_literal_spellings_are_understood() {
        assert_eq!(cond("0xFF == 255"), Ok(true));
        assert_eq!(cond("0b1010 == 10"), Ok(true));
        assert_eq!(cond("true"), Ok(true));
        assert_eq!(cond("false"), Ok(false));
    }

    #[test]
    fn true_is_not_treated_as_an_undefined_name() {
        // The trap this guards: `true` is an identifier-shaped token, so a
        // dialect that only knew "digits are numbers, everything else is an
        // undefined name" would evaluate `@if true` to FALSE and take the
        // `@else` branch, with no diagnostic anywhere.
        assert_eq!(cond("true"), Ok(true));
        assert_eq!(cond("true == 1"), Ok(true));
    }

    #[test]
    fn every_comparison_operator_works_in_both_directions() {
        for (src, want) in [
            ("1 == 1", true), ("1 == 2", false),
            ("1 != 2", true), ("1 != 1", false),
            ("1 < 2", true), ("2 < 1", false),
            ("2 > 1", true), ("1 > 2", false),
            ("1 <= 1", true), ("2 <= 1", false),
            ("1 >= 1", true), ("1 >= 2", false),
        ] {
            assert_eq!(cond(src), Ok(want), "{src}");
        }
    }

    #[test]
    fn comparisons_bind_tighter_than_and_which_binds_tighter_than_or() {
        // Precedence has to match Oct's own, or a program selects the branch
        // its author read as the other one. `0 && 0 || 1` is `(0 && 0) || 1`
        // = true; a flat left-to-right reading would give `0 && (0 || 1)` =
        // false, so this single case distinguishes them.
        assert_eq!(cond("0 && 0 || 1"), Ok(true));
        assert_eq!(cond("1 || 0 && 0"), Ok(true));
        assert_eq!(cond("1 == 1 && 2 == 2"), Ok(true));
        assert_eq!(cond("1 == 1 && 2 == 3"), Ok(false));
        assert_eq!(cond("1 == 2 || 3 == 3"), Ok(true));
    }

    #[test]
    fn parentheses_override_precedence() {
        // A pair that actually distinguishes the two readings: `&&` binds
        // tighter, so the unparenthesised form is `1 || (0 && 0)` = true,
        // while the parenthesised one is `(1 || 0) && 0` = false. Flipping
        // either would make both expressions agree and the test vacuous.
        assert_eq!(cond("1 || 0 && 0"), Ok(true));
        assert_eq!(cond("(1 || 0) && 0"), Ok(false));
        assert_eq!(cond("((((1))))"), Ok(true));
    }

    #[test]
    fn an_undefined_name_is_zero_for_now() {
        // Slice 1 has no macro table, so "undefined" is the only state a name
        // can be in. Slice 2 replaces this with a real lookup.
        assert_eq!(cond("LED_PORT"), Ok(false));
        assert_eq!(cond("LED_PORT == 0"), Ok(true));
        assert_eq!(cond("LED_PORT == 1"), Ok(false));
    }

    #[test]
    fn an_out_of_range_literal_is_refused_rather_than_truncated() {
        // `300` must not quietly become `44`. Oct's only integer type is the
        // 8008 byte, and Oct itself rejects the literal; a conditional that
        // wrapped it would select a branch for a reason nobody could find.
        let e = cond("300 == 44").unwrap_err();
        assert!(e.to_string().contains("out of range"), "{e}");
    }

    #[test]
    fn unsupported_operators_are_refused_not_guessed() {
        // The dialect declines rather than inventing semantics. Each of these
        // is a real Oct operator that simply has no meaning in a slice-1
        // conditional; a silent `0` would be far worse than a diagnostic.
        for src in ["1 + 1", "1 & 1", "1 ^ 1", "~1", "!1"] {
            assert!(cond(src).is_err(), "{src} must be refused");
        }
    }

    #[test]
    fn malformed_expressions_produce_diagnostics_and_never_panic() {
        for src in [
            "1 2",      // two values, no operator
            "1 ==",     // ends after an operator
            "== 1",     // starts with an operator
            "(1",       // unclosed group
            "1)",       // unbalanced close
            "()",       // empty group
            "(",
            ")",
        ] {
            assert!(cond(src).is_err(), "{src:?} must be a diagnostic");
        }
        // And the genuinely empty slice, which `classify` prevents but
        // `eval_condition` is public and must survive.
        assert!(MacroOctDialect.eval_condition(&[]).is_err());
    }

    #[test]
    fn deep_grouping_terminates_without_touching_the_machine_stack() {
        // The engine pre-scans grouping depth before calling, so in the real
        // pipeline this input never arrives. The evaluator must survive it
        // anyway: relying on a check in another crate is one refactor away
        // from an uncatchable abort. 100_000 levels would overflow any
        // recursive-descent parser on a default stack.
        let depth = 100_000;
        let src = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        assert_eq!(cond(&src), Ok(true));
    }

    #[test]
    fn a_long_flat_expression_terminates() {
        // The other direction: no nesting, but a very long operator chain.
        let src = vec!["1"; 20_000].join(" && ");
        assert_eq!(cond(&src), Ok(true));
    }

    // --- lex ---------------------------------------------------------------

    #[test]
    fn lex_of_an_included_file_drops_the_eof_sentinel() {
        // The sentinel from an included file would be spliced into the MIDDLE
        // of the stream, where Oct's parser stops early -- silently compiling
        // a truncated program. This is the single most consequential line in
        // the dialect.
        let mut fs = coding_adventures_source_preprocessor::MemoryFs::new();
        let id = fs.insert("x", "");
        let tokens = MacroOctDialect.lex("fn helper() { }", id).unwrap();
        assert!(!tokens.is_empty());
        assert!(tokens.iter().all(|t| t.type_ != TokenType::Eof));
    }

    #[test]
    fn a_lex_failure_in_an_included_file_is_a_diagnostic_not_a_panic() {
        let mut fs = coding_adventures_source_preprocessor::MemoryFs::new();
        let id = fs.insert("x", "");
        assert!(MacroOctDialect.lex("fn main() { $ }", id).is_err());
    }

    // --- the declined hooks ------------------------------------------------

    #[test]
    fn macrooct_declines_stringize_and_paste() {
        // Not an omission: MacroOct genuinely has neither `#` nor `##`, and
        // declining them is a positive test that the engine does not assume
        // every dialect carries a C-shaped macro facility.
        let tokens = line("1");
        assert!(MacroOctDialect.stringize(&tokens).is_none());
        assert!(MacroOctDialect.paste(&tokens[0], &tokens[0]).is_none());
    }
}
