//! The structural document parser (L1) — turns the flat [`crate::token`] stream into a
//! tree of [`Node`]s.
//!
//! This layer is about **document structure**, not math: it coalesces ordinary characters
//! into text, builds groups (`{…}`), captures command applications (`\cmd[opt]{arg}…`) and
//! environments (`\begin{env}…\end{env}`), and keeps each math island (`$…$`) as its raw
//! inner source for the math layer (L2) to parse later.
//!
//! It is a small recursive-descent parser over the token slice. It is **total and
//! panic-free**: every malformed structure (unbalanced braces, a `\begin{a}…\end{b}`
//! mismatch, an unterminated environment or math island) yields a spanned [`ParseError`].
//!
//! ## Generic argument capture (and its deliberate limit)
//!
//! After a control word, L1 captures **one** optional `[…]` argument (if it immediately
//! follows) and then a greedy run of mandatory `{…}` groups. It has no per-command arity
//! table, so `\textbf{a}{b}` captures *two* arguments — the precise arity of each command
//! is a later layer's job. A space breaks the run (`\textbf{a} {b}` captures only `{a}`),
//! because the tokenizer already absorbed the space *after* a control word but not after a
//! group.

use crate::ast::Node;
use crate::error::ParseError;
use crate::lexer::tokenize;
use crate::token::{Token, TokenKind};

/// Parse a LaTeX document into a sequence of structural nodes.
pub fn parse(src: &str) -> Result<Vec<Node>, ParseError> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks: &toks, src, pos: 0, depth: 0 };
    let nodes = p.parse_seq(Stop::Document)?;
    // After a document sequence the only thing left should be Eof; a `\end` here is a
    // close with no matching `\begin`.
    match &p.peek().kind {
        TokenKind::Eof => Ok(nodes),
        TokenKind::ControlWord(w) if w == "end" => {
            let sp = p.peek().span;
            Err(ParseError::new("\\end with no matching \\begin", sp.start, sp.end))
        }
        _ => {
            let sp = p.peek().span;
            Err(ParseError::new("unexpected token after document", sp.start, sp.end))
        }
    }
}

/// What terminates a [`Parser::parse_seq`] run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// Top level: ends at end-of-input.
    Document,
    /// An environment body: ends at `\end` (end-of-input is an error — unterminated env).
    EnvBody,
    /// A `{ … }` group: ends at `}`.
    Group,
    /// A `[ … ]` optional argument: ends at `]`.
    Bracket,
}

/// The optional `[…]` arguments and mandatory `{…}` arguments captured after a command —
/// each argument is itself a node sequence.
type CapturedArgs = (Vec<Vec<Node>>, Vec<Vec<Node>>);

/// Maximum group/environment nesting depth. Deeply nested input (`{{{…}}}`) drives the
/// recursive descent as deep as it nests; this bound turns a pathological input into a
/// spanned error instead of a stack overflow, keeping the parser total. Real documents
/// nest only a handful deep, so this is generous.
// Each `Token` carries a `TokenKind` whose largest variants now hold owned `String`s
// (`Verb`, `VerbatimEnv`, …), so recursive-descent frames are heavier than they were at L1.
// Keep the structural nesting cap low enough that pathological input (thousands of `{`) trips
// the guard well within even a small (2 MB test-thread) stack instead of overflowing — real
// documents never nest anywhere near this deep.
const MAX_DEPTH: usize = 256;

struct Parser<'a> {
    toks: &'a [Token],
    src: &'a str,
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token {
        // The token stream always ends in Eof and we never advance past it, so this is
        // always in bounds.
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn bump(&mut self) -> &Token {
        let t = &self.toks[self.pos.min(self.toks.len() - 1)];
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn parse_seq(&mut self, stop: Stop) -> Result<Vec<Node>, ParseError> {
        // Guard recursion depth (groups/environments/arguments recurse through here) so a
        // pathologically nested input is a clean error, not a stack overflow.
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            let sp = self.peek().span;
            self.depth -= 1;
            return Err(ParseError::new(
                format!("nesting too deep (>{MAX_DEPTH})"),
                sp.start,
                sp.end,
            ));
        }
        let result = self.parse_seq_inner(stop);
        self.depth -= 1;
        result
    }

    fn parse_seq_inner(&mut self, stop: Stop) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        loop {
            let tok = self.peek();
            let sp = tok.span;
            match &tok.kind {
                TokenKind::Eof => match stop {
                    Stop::Document => break,
                    Stop::Group => return Err(ParseError::new("unterminated group: missing '}'", sp.start, sp.end)),
                    Stop::Bracket => return Err(ParseError::new("unterminated optional argument: missing ']'", sp.start, sp.end)),
                    Stop::EnvBody => return Err(ParseError::new("unterminated environment: missing \\end", sp.start, sp.end)),
                },
                // `\end` ends a document/env-body run (the caller handles it); inside a
                // group/bracket it means an unbalanced delimiter.
                TokenKind::ControlWord(w) if w == "end" => match stop {
                    Stop::Document | Stop::EnvBody => break,
                    Stop::Group => return Err(ParseError::new("unterminated group: missing '}' before \\end", sp.start, sp.end)),
                    Stop::Bracket => return Err(ParseError::new("unterminated optional argument: missing ']' before \\end", sp.start, sp.end)),
                },
                TokenKind::EndGroup => match stop {
                    Stop::Group => {
                        self.bump();
                        break;
                    }
                    _ => return Err(ParseError::new("unexpected '}' (no open group)", sp.start, sp.end)),
                },
                TokenKind::Char(']') if stop == Stop::Bracket => {
                    self.bump();
                    break;
                }
                _ => nodes.push(self.parse_one(stop)?),
            }
        }
        Ok(nodes)
    }

    fn parse_one(&mut self, stop: Stop) -> Result<Node, ParseError> {
        let tok = self.peek();
        let sp = tok.span;
        match tok.kind.clone() {
            TokenKind::Char(_) => Ok(self.coalesce_text(stop)),
            TokenKind::Space => {
                self.bump();
                Ok(Node::Space)
            }
            TokenKind::Par => {
                self.bump();
                Ok(Node::Par)
            }
            TokenKind::Comment(c) => {
                self.bump();
                Ok(Node::Comment(c))
            }
            TokenKind::Active(c) => {
                self.bump();
                Ok(Node::Active(c))
            }
            TokenKind::Verb { star, delim, content } => {
                self.bump();
                Ok(Node::Verb { star, delim, content })
            }
            TokenKind::VerbatimEnv { env, content } => {
                self.bump();
                Ok(Node::VerbatimEnv { env, content })
            }
            // In text mode these four are literal characters; they only carry structural
            // meaning inside math/environments, which are handled elsewhere (L2/L3).
            TokenKind::AlignTab => { self.bump(); Ok(Node::Text("&".into())) }
            TokenKind::Parameter => { self.bump(); Ok(Node::Text("#".into())) }
            TokenKind::Superscript => { self.bump(); Ok(Node::Text("^".into())) }
            TokenKind::Subscript => { self.bump(); Ok(Node::Text("_".into())) }
            TokenKind::BeginGroup => {
                self.bump();
                let inner = self.parse_seq(Stop::Group)?;
                Ok(Node::Group(inner))
            }
            TokenKind::ControlSymbol(c) => {
                self.bump();
                Ok(Node::Command { name: c.to_string(), optional: vec![], arguments: vec![] })
            }
            TokenKind::MathOn { display } => self.parse_math(display),
            TokenKind::MathOff { .. } => {
                Err(ParseError::new("unexpected end of math (no matching open)", sp.start, sp.end))
            }
            TokenKind::ControlWord(name) => {
                if name == "begin" {
                    self.parse_environment()
                } else {
                    self.bump();
                    let (optional, arguments) = self.capture_args()?;
                    Ok(Node::Command { name, optional, arguments })
                }
            }
            // `\end` and `}` are handled by parse_seq before reaching here; Eof likewise.
            TokenKind::EndGroup | TokenKind::Eof => {
                Err(ParseError::new("internal: unexpected delimiter", sp.start, sp.end))
            }
        }
    }

    /// Merge consecutive `Char` tokens into one `Text` node. In bracket context, stop
    /// before the `]` that closes the optional argument so it isn't swallowed.
    fn coalesce_text(&mut self, stop: Stop) -> Node {
        let mut s = String::new();
        while let TokenKind::Char(c) = self.peek().kind {
            if stop == Stop::Bracket && c == ']' {
                break;
            }
            s.push(c);
            self.bump();
        }
        Node::Text(s)
    }

    /// `$ … $` / `\( … \)` (or display): keep the inner source verbatim for L2.
    fn parse_math(&mut self, display: bool) -> Result<Node, ParseError> {
        let on = self.bump().span; // consume MathOn
        let content_start = on.end;
        loop {
            let tok = self.peek();
            match &tok.kind {
                TokenKind::MathOff { .. } => {
                    let content = self.src[content_start..tok.span.start].to_string();
                    self.bump(); // consume MathOff
                    return Ok(Node::Math { display, content });
                }
                TokenKind::Eof => {
                    return Err(ParseError::new("unterminated math (missing closing delimiter)", on.start, tok.span.end));
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// Capture one optional `[…]` (if it immediately follows) then a greedy run of
    /// mandatory `{…}` groups.
    fn capture_args(&mut self) -> Result<CapturedArgs, ParseError> {
        let mut optional = Vec::new();
        let mut arguments = Vec::new();
        if matches!(self.peek().kind, TokenKind::Char('[')) {
            self.bump(); // consume '['
            optional.push(self.parse_seq(Stop::Bracket)?);
        }
        while matches!(self.peek().kind, TokenKind::BeginGroup) {
            self.bump(); // consume '{'
            arguments.push(self.parse_seq(Stop::Group)?);
        }
        Ok((optional, arguments))
    }

    /// `\begin{name}[opt]{arg}… body \end{name}`.
    fn parse_environment(&mut self) -> Result<Node, ParseError> {
        let begin_sp = self.bump().span; // consume `\begin`
        let name = self.read_brace_name("\\begin")?;
        let (optional, arguments) = self.capture_args()?;
        let body = self.parse_seq(Stop::EnvBody)?;
        // Expect `\end{name}`.
        match &self.peek().kind {
            TokenKind::ControlWord(w) if w == "end" => {
                self.bump();
            }
            _ => {
                let sp = self.peek().span;
                return Err(ParseError::new(
                    format!("unterminated environment \\begin{{{name}}} (expected \\end)"),
                    begin_sp.start,
                    sp.end,
                ));
            }
        }
        let end_name_sp = self.peek().span;
        let end_name = self.read_brace_name("\\end")?;
        if end_name != name {
            return Err(ParseError::new(
                format!("environment mismatch: \\begin{{{name}}} closed by \\end{{{end_name}}}"),
                begin_sp.start,
                end_name_sp.end,
            ));
        }
        Ok(Node::Environment { name, optional, arguments, body })
    }

    /// Read a `{name}` group of ordinary characters (used for `\begin{name}`/`\end{name}`).
    fn read_brace_name(&mut self, ctx: &str) -> Result<String, ParseError> {
        let sp = self.peek().span;
        if !matches!(self.peek().kind, TokenKind::BeginGroup) {
            return Err(ParseError::new(format!("{ctx} must be followed by {{name}}"), sp.start, sp.end));
        }
        self.bump(); // consume '{'
        let mut name = String::new();
        loop {
            let tok = self.peek();
            match tok.kind {
                TokenKind::Char(c) => {
                    name.push(c);
                    self.bump();
                }
                TokenKind::EndGroup => {
                    self.bump();
                    return Ok(name);
                }
                TokenKind::Eof => {
                    return Err(ParseError::new("unterminated environment name (missing '}')", sp.start, tok.span.end));
                }
                _ => {
                    let bad = tok.span;
                    return Err(ParseError::new("invalid character in environment name", bad.start, bad.end));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{document_to_latex, Node::*};

    fn p(src: &str) -> Vec<Node> {
        parse(src).expect("parse")
    }

    /// Round-trip invariant: parsing the rendered AST yields the same AST.
    fn assert_round_trips(src: &str) {
        let ast = parse(src).expect("parse");
        let rendered = document_to_latex(&ast);
        let reparsed = parse(&rendered).expect("re-parse");
        assert_eq!(ast, reparsed, "round-trip mismatch: {src:?} -> {rendered:?}");
    }

    #[test]
    fn plain_text_and_spaces() {
        assert_eq!(p("ab c"), vec![Text("ab".into()), Space, Text("c".into())]);
    }

    #[test]
    fn paragraph_break() {
        assert_eq!(p("a\n\nb"), vec![Text("a".into()), Par, Text("b".into())]);
    }

    #[test]
    fn group() {
        assert_eq!(p("{ab}"), vec![Group(vec![Text("ab".into())])]);
    }

    #[test]
    fn bare_command() {
        assert_eq!(p(r"\alpha"), vec![Command { name: "alpha".into(), optional: vec![], arguments: vec![] }]);
    }

    #[test]
    fn command_with_one_arg() {
        assert_eq!(
            p(r"\textbf{hi}"),
            vec![Command { name: "textbf".into(), optional: vec![], arguments: vec![vec![Text("hi".into())]] }]
        );
    }

    #[test]
    fn command_with_optional_and_arg() {
        assert_eq!(
            p(r"\sqrt[3]{x}"),
            vec![Command {
                name: "sqrt".into(),
                optional: vec![vec![Text("3".into())]],
                arguments: vec![vec![Text("x".into())]],
            }]
        );
    }

    #[test]
    fn greedy_args_are_broken_by_a_space() {
        // `\textbf{a} {b}` → textbf takes only {a}; {b} is a separate group.
        assert_eq!(
            p(r"\textbf{a} {b}"),
            vec![
                Command { name: "textbf".into(), optional: vec![], arguments: vec![vec![Text("a".into())]] },
                Space,
                Group(vec![Text("b".into())]),
            ]
        );
    }

    #[test]
    fn control_symbol_is_an_argless_command() {
        assert_eq!(p(r"\,"), vec![Command { name: ",".into(), optional: vec![], arguments: vec![] }]);
    }

    #[test]
    fn inline_and_display_math_keep_raw_content() {
        assert_eq!(p(r"$x+1$"), vec![Math { display: false, content: "x+1".into() }]);
        assert_eq!(p(r"$$x+1$$"), vec![Math { display: true, content: "x+1".into() }]);
        // `\(...\)` is inline; content captured verbatim.
        assert_eq!(p(r"\(a b\)"), vec![Math { display: false, content: "a b".into() }]);
    }

    #[test]
    fn environment_with_body() {
        assert_eq!(
            p(r"\begin{center}hi\end{center}"),
            vec![Environment {
                name: "center".into(),
                optional: vec![],
                arguments: vec![],
                body: vec![Text("hi".into())],
            }]
        );
    }

    #[test]
    fn nested_environments() {
        let ast = p(r"\begin{a}\begin{b}x\end{b}\end{a}");
        match &ast[0] {
            Environment { name, body, .. } => {
                assert_eq!(name, "a");
                assert!(matches!(&body[0], Environment { name, .. } if name == "b"));
            }
            other => panic!("expected env, got {other:?}"),
        }
    }

    #[test]
    fn environment_with_args() {
        // tabular takes a mandatory column spec.
        let ast = p(r"\begin{tabular}{cc}x\end{tabular}");
        assert!(matches!(&ast[0], Environment { name, arguments, .. }
            if name == "tabular" && arguments.len() == 1));
    }

    #[test]
    fn a_realistic_paragraph() {
        let ast = p(r"Let $x$ be \textbf{positive}.");
        assert_eq!(
            ast,
            vec![
                Text("Let".into()), Space,
                Math { display: false, content: "x".into() }, Space,
                Text("be".into()), Space,
                Command { name: "textbf".into(), optional: vec![], arguments: vec![vec![Text("positive".into())]] },
                Text(".".into()),
            ]
        );
    }

    // ---- error cases (spanned, never panic) -----------------------------------
    #[test]
    fn unbalanced_open_brace_errors() {
        let e = parse("a{b").unwrap_err();
        assert!(e.message.contains("unterminated group"));
    }

    #[test]
    fn unbalanced_close_brace_errors() {
        let e = parse("a}b").unwrap_err();
        assert!(e.message.contains("no open group"));
    }

    #[test]
    fn environment_mismatch_errors() {
        let e = parse(r"\begin{a}x\end{b}").unwrap_err();
        assert!(e.message.contains("environment mismatch"), "{}", e.message);
    }

    #[test]
    fn unterminated_environment_errors() {
        let e = parse(r"\begin{a}x").unwrap_err();
        assert!(e.message.contains("unterminated environment"));
    }

    #[test]
    fn unterminated_math_errors() {
        let e = parse(r"$x+1").unwrap_err();
        assert!(e.message.contains("unterminated math"));
    }

    #[test]
    fn stray_end_errors() {
        let e = parse(r"x\end{a}").unwrap_err();
        assert!(e.message.contains("no matching \\begin"));
    }

    #[test]
    fn pathological_nesting_errors_instead_of_overflowing() {
        // 5000 nested groups would overflow a naive recursive descent; the depth guard
        // turns it into a clean spanned error (no panic, no stack overflow).
        let deep = "{".repeat(5000);
        let e = parse(&deep).unwrap_err();
        assert!(e.message.contains("nesting too deep"), "{}", e.message);
    }

    // ---- round-trips ----------------------------------------------------------
    #[test]
    fn round_trips_cover_the_constructs() {
        for src in [
            "Hello, world.",
            "a\n\nb",
            r"\textbf{bold} and \emph{italic}",
            r"\sqrt[3]{x} + \frac{1}{2}",
            r"Let $x$ be $$y=mx+b$$ done",
            r"\begin{itemize}\item one\item two\end{itemize}",
            r"\begin{tabular}{cc}a&b\end{tabular}",
            r"a~b and \, thin",
            r"nested {groups {deep}} ok",
            r"use \verb|x{y}$z| inline",
            r"\verb*+two words+ here",
        ] {
            assert_round_trips(src);
        }
    }

    #[test]
    fn verb_is_a_node_with_raw_body() {
        // `\verb|...|` becomes a Verb node whose body keeps catcode-significant chars literal.
        assert_eq!(
            p(r"\verb|a{b}$c|"),
            vec![Verb { star: false, delim: '|', content: "a{b}$c".into() }]
        );
        // starred variant
        assert_eq!(
            p(r"\verb*!x!"),
            vec![Verb { star: true, delim: '!', content: "x".into() }]
        );
    }

    #[test]
    fn verb_does_not_disturb_surrounding_text() {
        assert_eq!(
            p(r"a\verb|b|c"),
            vec![
                Text("a".into()),
                Verb { star: false, delim: '|', content: "b".into() },
                Text("c".into()),
            ]
        );
    }

    #[test]
    fn verbatim_environment_is_a_node() {
        assert_eq!(
            p("\\begin{verbatim}let x = {1};\n$y$\\end{verbatim}"),
            vec![VerbatimEnv {
                env: "verbatim".into(),
                content: "let x = {1};\n$y$".into(),
            }]
        );
    }

    #[test]
    fn verbatim_environment_round_trips() {
        assert_round_trips("before \\begin{verbatim}raw {code} $here$\\end{verbatim} after");
        assert_round_trips(r"\begin{verbatim*}v s\end{verbatim*}");
    }
}
