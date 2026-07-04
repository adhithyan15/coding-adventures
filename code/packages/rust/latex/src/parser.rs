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
//! ## Precise byte spans (LTXDOC02 S1)
//!
//! Every token the lexer emits already records a half-open byte [`Span`]. This parser threads
//! those spans straight onto the [`Node`]s it builds, so each node carries its **exact** source
//! byte range — `&src[node.span.start .. node.span.end]` is the node's own source substring:
//!
//! | node | span covers |
//! |------|-------------|
//! | `Text` (coalesced run) | first char token's start … last char token's end |
//! | `Group` | the `{` … the matching `}` (braces included) |
//! | `Command` | `\name` … the last captured argument's closing `}` (or just the control word) |
//! | `Environment` | `\begin{name}` … the closing `}` of `\end{name}` |
//! | `Math` island | the opening `$`/`$$`/`\[` … the closing delimiter |
//! | leaves (`Space`, `Par`, `Comment`, `Active`, `Verb`, …) | the token's own span |
//!
//! Composite spans are built from the **tracked** start/end of the covered tokens (a `[first,
//! last)` union) — never re-derived by scanning the source for a substring.
//!
//! ## Generic argument capture (and its deliberate limit)
//!
//! After a control word, L1 captures **one** optional `[…]` argument (if it immediately
//! follows) and then a greedy run of mandatory `{…}` groups. It has no per-command arity
//! table, so `\textbf{a}{b}` captures *two* arguments — the precise arity of each command
//! is a later layer's job. A space breaks the run (`\textbf{a} {b}` captures only `{a}`),
//! because the tokenizer already absorbed the space *after* a control word but not after a
//! group.

use crate::ast::{Node, NodeKind};
use crate::error::ParseError;
use crate::lexer::tokenize;
use crate::token::{Span, Token, TokenKind};

/// Parse a LaTeX document into a sequence of structural nodes.
pub fn parse(src: &str) -> Result<Vec<Node>, ParseError> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks: &toks, src, pos: 0, depth: 0 };
    let (nodes, _end) = p.parse_seq(Stop::Document)?;
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

/// A parsed node sequence plus the byte offset **just past** the delimiter that closed it — the
/// `}` for a group, the `]` for a bracket. For `Document`/`EnvBody` runs (which stop *before*
/// consuming their terminator) this is the offset where the run ended (the start of the stopping
/// token); the caller composes the enclosing node's span from there.
type Seq = (Vec<Node>, usize);

/// The optional `[…]` arguments and mandatory `{…}` arguments captured after a command — each
/// argument is itself a node sequence — plus the byte offset just past the last closing `}`
/// (or `]`) consumed, or `None` if no argument was captured (a bare control word).
type CapturedArgs = (Vec<Vec<Node>>, Vec<Vec<Node>>, Option<usize>);

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

    fn parse_seq(&mut self, stop: Stop) -> Result<Seq, ParseError> {
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

    fn parse_seq_inner(&mut self, stop: Stop) -> Result<Seq, ParseError> {
        let mut nodes = Vec::new();
        loop {
            let tok = self.peek();
            let sp = tok.span;
            match &tok.kind {
                TokenKind::Eof => match stop {
                    // Document runs stop *before* Eof; the run "ends" at Eof's start.
                    Stop::Document => return Ok((nodes, sp.start)),
                    Stop::Group => return Err(ParseError::new("unterminated group: missing '}'", sp.start, sp.end)),
                    Stop::Bracket => return Err(ParseError::new("unterminated optional argument: missing ']'", sp.start, sp.end)),
                    Stop::EnvBody => return Err(ParseError::new("unterminated environment: missing \\end", sp.start, sp.end)),
                },
                // `\end` ends a document/env-body run (the caller handles it); inside a
                // group/bracket it means an unbalanced delimiter.
                TokenKind::ControlWord(w) if w == "end" => match stop {
                    // Stop *before* `\end`; the run ends at the `\end` token's start.
                    Stop::Document | Stop::EnvBody => return Ok((nodes, sp.start)),
                    Stop::Group => return Err(ParseError::new("unterminated group: missing '}' before \\end", sp.start, sp.end)),
                    Stop::Bracket => return Err(ParseError::new("unterminated optional argument: missing ']' before \\end", sp.start, sp.end)),
                },
                TokenKind::EndGroup => match stop {
                    Stop::Group => {
                        // Consume the `}` and report the offset just past it, so the caller's
                        // `Group` span covers `{`…`}` inclusive.
                        let end = sp.end;
                        self.bump();
                        return Ok((nodes, end));
                    }
                    _ => return Err(ParseError::new("unexpected '}' (no open group)", sp.start, sp.end)),
                },
                TokenKind::Char(']') if stop == Stop::Bracket => {
                    // Consume the `]` and report the offset just past it.
                    let end = sp.end;
                    self.bump();
                    return Ok((nodes, end));
                }
                _ => nodes.push(self.parse_one(stop)?),
            }
        }
    }

    fn parse_one(&mut self, stop: Stop) -> Result<Node, ParseError> {
        let tok = self.peek();
        let sp = tok.span;
        match tok.kind.clone() {
            TokenKind::Char(_) => Ok(self.coalesce_text(stop)),
            TokenKind::Space => {
                self.bump();
                Ok(Node::space(sp))
            }
            TokenKind::Par => {
                self.bump();
                Ok(Node::par(sp))
            }
            TokenKind::Comment(c) => {
                self.bump();
                Ok(Node::new(NodeKind::Comment(c), sp))
            }
            TokenKind::Active(c) => {
                self.bump();
                Ok(Node::new(NodeKind::Active(c), sp))
            }
            TokenKind::Verb { star, delim, content } => {
                self.bump();
                Ok(Node::new(NodeKind::Verb { star, delim, content }, sp))
            }
            TokenKind::VerbatimEnv { env, content } => {
                self.bump();
                Ok(Node::new(NodeKind::VerbatimEnv { env, content }, sp))
            }
            // In text mode these four are literal characters; they only carry structural
            // meaning inside math/environments, which are handled elsewhere (L2/L3).
            TokenKind::AlignTab => { self.bump(); Ok(Node::text("&", sp)) }
            TokenKind::Parameter => { self.bump(); Ok(Node::text("#", sp)) }
            TokenKind::Superscript => { self.bump(); Ok(Node::text("^", sp)) }
            TokenKind::Subscript => { self.bump(); Ok(Node::text("_", sp)) }
            TokenKind::BeginGroup => {
                let open = self.bump().span; // consume `{`
                let (inner, end) = self.parse_seq(Stop::Group)?;
                // The group's span covers `{` … the matching `}` (inclusive).
                Ok(Node::group(inner, Span::new(open.start, end)))
            }
            TokenKind::ControlSymbol(c) => {
                self.bump();
                Ok(Node::command(c.to_string(), vec![], vec![], sp))
            }
            TokenKind::MathOn { display } => self.parse_math(display),
            TokenKind::MathOff { .. } => {
                Err(ParseError::new("unexpected end of math (no matching open)", sp.start, sp.end))
            }
            TokenKind::ControlWord(name) => {
                if name == "begin" {
                    self.parse_environment()
                } else {
                    let cmd = self.bump().span; // consume `\name`
                    let (optional, arguments, args_end) = self.capture_args()?;
                    // No captured args → span is just the control word; otherwise it extends to
                    // the last argument's closing `}` (or the optional's `]`).
                    let end = args_end.unwrap_or(cmd.end);
                    Ok(Node::command(name, optional, arguments, Span::new(cmd.start, end)))
                }
            }
            // `\end` and `}` are handled by parse_seq before reaching here; Eof likewise.
            TokenKind::EndGroup | TokenKind::Eof => {
                Err(ParseError::new("internal: unexpected delimiter", sp.start, sp.end))
            }
        }
    }

    /// Merge consecutive `Char` tokens into one `Text` node. In bracket context, stop
    /// before the `]` that closes the optional argument so it isn't swallowed. The node's span
    /// covers the first char token's start … the last char token's end.
    fn coalesce_text(&mut self, stop: Stop) -> Node {
        let mut s = String::new();
        // The caller only invokes this when the current token is a `Char`, so `start` is the
        // first character's byte offset and there is always at least one char consumed.
        let start = self.peek().span.start;
        let mut end = start;
        while let TokenKind::Char(c) = self.peek().kind {
            if stop == Stop::Bracket && c == ']' {
                break;
            }
            end = self.peek().span.end;
            s.push(c);
            self.bump();
        }
        Node::text(s, Span::new(start, end))
    }

    /// `$ … $` / `\( … \)` (or display): keep the inner source verbatim for L2. The node's span
    /// covers the opening delimiter … the closing delimiter (inclusive).
    fn parse_math(&mut self, display: bool) -> Result<Node, ParseError> {
        let on = self.bump().span; // consume MathOn
        let content_start = on.end;
        loop {
            let tok = self.peek();
            match &tok.kind {
                TokenKind::MathOff { .. } => {
                    let content = self.src[content_start..tok.span.start].to_string();
                    let off_end = tok.span.end;
                    self.bump(); // consume MathOff
                    return Ok(Node::new(
                        NodeKind::Math { display, content },
                        Span::new(on.start, off_end),
                    ));
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
    /// mandatory `{…}` groups. Returns the captured argument sequences plus the byte offset
    /// just past the last closing delimiter consumed (`None` if nothing was captured).
    fn capture_args(&mut self) -> Result<CapturedArgs, ParseError> {
        let mut optional = Vec::new();
        let mut arguments = Vec::new();
        let mut last_end: Option<usize> = None;
        if matches!(self.peek().kind, TokenKind::Char('[')) {
            self.bump(); // consume '['
            let (nodes, end) = self.parse_seq(Stop::Bracket)?;
            optional.push(nodes);
            last_end = Some(end);
        }
        while matches!(self.peek().kind, TokenKind::BeginGroup) {
            self.bump(); // consume '{'
            let (nodes, end) = self.parse_seq(Stop::Group)?;
            arguments.push(nodes);
            last_end = Some(end);
        }
        Ok((optional, arguments, last_end))
    }

    /// `\begin{name}[opt]{arg}… body \end{name}`. The node's span covers `\begin` … the closing
    /// `}` of `\end{name}` (inclusive).
    fn parse_environment(&mut self) -> Result<Node, ParseError> {
        let begin_sp = self.bump().span; // consume `\begin`
        let name = self.read_brace_name("\\begin")?;
        let (optional, arguments, _args_end) = self.capture_args()?;
        let (body, _body_end) = self.parse_seq(Stop::EnvBody)?;
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
        // `read_brace_name` consumed through the closing `}` of `\end{name}`; the token now at
        // `pos-1` is that `}` — its end is the environment's end. `read_brace_name` returned it,
        // so we recover it from the just-consumed token's span.
        let env_end = self.prev_end();
        Ok(Node::new(
            NodeKind::Environment { name, optional, arguments, body },
            Span::new(begin_sp.start, env_end),
        ))
    }

    /// The end byte offset of the token just consumed (`pos - 1`). Used to close a span on the
    /// last delimiter a sub-parse ate. Safe: every call site has consumed ≥1 token first.
    fn prev_end(&self) -> usize {
        let i = self.pos.saturating_sub(1).min(self.toks.len() - 1);
        self.toks[i].span.end
    }

    /// Read a `{name}` group of ordinary characters (used for `\begin{name}`/`\end{name}`).
    /// Consumes through the closing `}`.
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
    use crate::ast::{document_to_latex, NodeKind::*};

    fn p(src: &str) -> Vec<Node> {
        parse(src).expect("parse")
    }

    /// Assert a node's kind matches, ignoring its span (spans are checked separately below).
    fn kinds(nodes: &[Node]) -> Vec<NodeKind> {
        nodes.iter().map(|n| n.kind.clone()).collect()
    }

    /// Round-trip invariant: parsing the rendered AST yields the same AST (modulo spans, which
    /// `Node`'s `PartialEq` already ignores).
    fn assert_round_trips(src: &str) {
        let ast = parse(src).expect("parse");
        let rendered = document_to_latex(&ast);
        let reparsed = parse(&rendered).expect("re-parse");
        assert_eq!(ast, reparsed, "round-trip mismatch: {src:?} -> {rendered:?}");
    }

    #[test]
    fn plain_text_and_spaces() {
        assert_eq!(kinds(&p("ab c")), vec![Text("ab".into()), Space, Text("c".into())]);
    }

    #[test]
    fn paragraph_break() {
        assert_eq!(kinds(&p("a\n\nb")), vec![Text("a".into()), Par, Text("b".into())]);
    }

    #[test]
    fn group() {
        assert_eq!(kinds(&p("{ab}")), vec![Group(vec![Node::text("ab", Span::new(1, 3))])]);
    }

    #[test]
    fn bare_command() {
        assert_eq!(kinds(&p(r"\alpha")), vec![Command { name: "alpha".into(), optional: vec![], arguments: vec![] }]);
    }

    #[test]
    fn command_with_one_arg() {
        assert_eq!(
            kinds(&p(r"\textbf{hi}")),
            vec![Command { name: "textbf".into(), optional: vec![], arguments: vec![vec![Node::text("hi", Span::new(8, 10))]] }]
        );
    }

    #[test]
    fn command_with_optional_and_arg() {
        assert_eq!(
            kinds(&p(r"\sqrt[3]{x}")),
            vec![Command {
                name: "sqrt".into(),
                optional: vec![vec![Node::text("3", Span::new(6, 7))]],
                arguments: vec![vec![Node::text("x", Span::new(9, 10))]],
            }]
        );
    }

    #[test]
    fn greedy_args_are_broken_by_a_space() {
        // `\textbf{a} {b}` → textbf takes only {a}; {b} is a separate group.
        assert_eq!(
            kinds(&p(r"\textbf{a} {b}")),
            vec![
                Command { name: "textbf".into(), optional: vec![], arguments: vec![vec![Node::text("a", Span::new(8, 9))]] },
                Space,
                Group(vec![Node::text("b", Span::new(12, 13))]),
            ]
        );
    }

    #[test]
    fn control_symbol_is_an_argless_command() {
        assert_eq!(kinds(&p(r"\,")), vec![Command { name: ",".into(), optional: vec![], arguments: vec![] }]);
    }

    #[test]
    fn inline_and_display_math_keep_raw_content() {
        assert_eq!(kinds(&p(r"$x+1$")), vec![Math { display: false, content: "x+1".into() }]);
        assert_eq!(kinds(&p(r"$$x+1$$")), vec![Math { display: true, content: "x+1".into() }]);
        // `\(...\)` is inline; content captured verbatim.
        assert_eq!(kinds(&p(r"\(a b\)")), vec![Math { display: false, content: "a b".into() }]);
    }

    #[test]
    fn environment_with_body() {
        assert_eq!(
            kinds(&p(r"\begin{center}hi\end{center}")),
            vec![Environment {
                name: "center".into(),
                optional: vec![],
                arguments: vec![],
                body: vec![Node::text("hi", Span::new(14, 16))],
            }]
        );
    }

    #[test]
    fn nested_environments() {
        let ast = p(r"\begin{a}\begin{b}x\end{b}\end{a}");
        match &ast[0].kind {
            Environment { name, body, .. } => {
                assert_eq!(name, "a");
                assert!(matches!(&body[0].kind, Environment { name, .. } if name == "b"));
            }
            other => panic!("expected env, got {other:?}"),
        }
    }

    #[test]
    fn environment_with_args() {
        // tabular takes a mandatory column spec.
        let ast = p(r"\begin{tabular}{cc}x\end{tabular}");
        assert!(matches!(&ast[0].kind, Environment { name, arguments, .. }
            if name == "tabular" && arguments.len() == 1));
    }

    #[test]
    fn a_realistic_paragraph() {
        let ast = p(r"Let $x$ be \textbf{positive}.");
        assert_eq!(
            kinds(&ast),
            vec![
                Text("Let".into()), Space,
                Math { display: false, content: "x".into() }, Space,
                Text("be".into()), Space,
                Command { name: "textbf".into(), optional: vec![], arguments: vec![vec![Node::text("positive", Span::new(19, 27))]] },
                Text(".".into()),
            ]
        );
    }

    // ---- precise byte spans (LTXDOC02 S1) -------------------------------------
    //
    // Each representative top-level node slices back to its EXACT source substring, proving the
    // parser threaded the tokens' spans through faithfully (not re-derived by substring search).

    /// `&src[node.span]` — the node's own source substring.
    fn slice<'a>(src: &'a str, n: &Node) -> &'a str {
        &src[n.span.start..n.span.end]
    }

    #[test]
    fn command_span_slices_back_to_source() {
        let src = r"\textbf{x}";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), r"\textbf{x}");
    }

    #[test]
    fn command_with_optional_span_covers_through_last_arg() {
        let src = r"\sqrt[3]{x}";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), r"\sqrt[3]{x}");
    }

    #[test]
    fn bare_command_span_is_just_the_control_word() {
        let src = r"\alpha next";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), r"\alpha");
    }

    #[test]
    fn group_span_includes_braces() {
        let src = r"{ab}";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), "{ab}");
    }

    #[test]
    fn math_span_includes_delimiters() {
        let src = r"$x+1$";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), "$x+1$");
        // display math keeps both `$$` delimiters
        let src2 = r"$$y=mx$$";
        let ast2 = p(src2);
        assert_eq!(slice(src2, &ast2[0]), "$$y=mx$$");
    }

    #[test]
    fn text_run_span_is_exactly_its_characters() {
        let src = "hello world";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), "hello"); // first run, up to the space
    }

    #[test]
    fn environment_span_covers_begin_to_end() {
        let src = r"\begin{center}hi\end{center}";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), r"\begin{center}hi\end{center}");
    }

    #[test]
    fn environment_with_arg_span_covers_begin_to_end() {
        let src = r"\begin{tabular}{cc}a\end{tabular}";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), r"\begin{tabular}{cc}a\end{tabular}");
    }

    /// Containment: every node's span ⊆ its parent's ⊆ the whole-source range `0..src.len()`.
    fn assert_contained(src: &str, nodes: &[Node], parent: Span) {
        for n in nodes {
            assert!(
                parent.start <= n.span.start && n.span.end <= parent.end,
                "span {:?} not within parent {:?} (src {src:?})",
                n.span,
                parent
            );
            assert!(n.span.start <= n.span.end, "span end < start: {:?}", n.span);
            // Recurse into every child node-list this node carries.
            for child_list in child_lists(n) {
                assert_contained(src, child_list, n.span);
            }
        }
    }

    /// Every child node-sequence a node carries (for the containment walk).
    fn child_lists(n: &Node) -> Vec<&[Node]> {
        match &n.kind {
            NodeKind::Group(inner) => vec![inner.as_slice()],
            NodeKind::Command { optional, arguments, .. } => {
                optional.iter().chain(arguments.iter()).map(Vec::as_slice).collect()
            }
            NodeKind::Environment { optional, arguments, body, .. } => optional
                .iter()
                .chain(arguments.iter())
                .map(Vec::as_slice)
                .chain(std::iter::once(body.as_slice()))
                .collect(),
            _ => vec![],
        }
    }

    #[test]
    fn every_node_span_is_contained_in_the_source() {
        for src in [
            r"Let $x$ be \textbf{positive}.",
            r"\begin{tabular}{cc}a&b\end{tabular}",
            r"nested {groups {deep}} ok",
            r"\sqrt[3]{x} + \frac{1}{2}",
            r"a~b and \, thin",
        ] {
            let ast = p(src);
            assert_contained(src, &ast, Span::new(0, src.len()));
        }
    }

    #[test]
    fn totality_malformed_but_parseable_still_spanned_no_panic() {
        // A control word with a trailing letter, a stray `#`, an active char, a comment: all
        // parse, and every resulting node carries a well-formed span (end >= start, in range).
        let src = "a#~b% note\n\\c";
        let ast = p(src);
        assert_contained(src, &ast, Span::new(0, src.len()));
        // `Span::new` guards `end < start` (a leaf's own span is never inverted).
        for n in &ast {
            assert!(n.span.start <= n.span.end);
        }
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
            kinds(&p(r"\verb|a{b}$c|")),
            vec![Verb { star: false, delim: '|', content: "a{b}$c".into() }]
        );
        // starred variant
        assert_eq!(
            kinds(&p(r"\verb*!x!")),
            vec![Verb { star: true, delim: '!', content: "x".into() }]
        );
    }

    #[test]
    fn verb_span_slices_back_to_source() {
        let src = r"\verb|a{b}$c|";
        let ast = p(src);
        assert_eq!(slice(src, &ast[0]), src);
    }

    #[test]
    fn verb_does_not_disturb_surrounding_text() {
        assert_eq!(
            kinds(&p(r"a\verb|b|c")),
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
            kinds(&p("\\begin{verbatim}let x = {1};\n$y$\\end{verbatim}")),
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
