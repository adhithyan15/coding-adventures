//! The document AST — what the structural parser ([`crate::parser`], L1) produces.
//!
//! A LaTeX document is a sequence of [`Node`]s: runs of text, groups (`{…}`), command
//! applications (`\cmd[opt]{arg}…`), environments (`\begin{env}…\end{env}`), math islands
//! (`$…$`), comments, and a few specials. Math bodies are kept **raw** at this layer (the
//! exact inner source string) — the math grammar is the next layer's job (L2), which keeps
//! this layer about *document structure* only.
//!
//! Every node round-trips: [`Node::to_latex`] renders a node back to LaTeX, and
//! `parse(&render(ast)) == ast` (AST-equality, not byte-equality — surface spacing and the
//! `$…$` vs `\(…\)` delimiter choice are normalized).

/// A sectioning level, in document-hierarchy order from coarsest (`\part`) to finest
/// (`\subparagraph`). Produced by the L5d [`recognize_structure`](crate::recognize_structure)
/// pass as part of [`Node::Section`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionLevel {
    /// `\part`
    Part,
    /// `\chapter`
    Chapter,
    /// `\section`
    Section,
    /// `\subsection`
    Subsection,
    /// `\subsubsection`
    Subsubsection,
    /// `\paragraph` (a run-in heading, *not* a paragraph break — see [`Node::Par`])
    Paragraph,
    /// `\subparagraph`
    Subparagraph,
}

impl SectionLevel {
    /// The LaTeX control word for this level (without the leading backslash), used to render a
    /// [`Node::Section`] back to source.
    pub fn command(self) -> &'static str {
        match self {
            SectionLevel::Part => "part",
            SectionLevel::Chapter => "chapter",
            SectionLevel::Section => "section",
            SectionLevel::Subsection => "subsection",
            SectionLevel::Subsubsection => "subsubsection",
            SectionLevel::Paragraph => "paragraph",
            SectionLevel::Subparagraph => "subparagraph",
        }
    }
}

/// One node of the document tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A run of ordinary text (consecutive ordinary characters, coalesced).
    Text(String),
    /// Significant inter-word space.
    Space,
    /// A paragraph break (a blank line).
    Par,
    /// An explicit group: `{ … }`.
    Group(Vec<Node>),
    /// A command application: `\name[opt]…{arg}…`. `optional` holds bracketed optional
    /// arguments; `arguments` holds the brace groups that immediately follow. (L1 captures
    /// generically — the precise arity of each command is a later layer's command table.)
    Command {
        name: String,
        optional: Vec<Vec<Node>>,
        arguments: Vec<Vec<Node>>,
    },
    /// An environment: `\begin{name}[opt]{arg}… body \end{name}`.
    Environment {
        name: String,
        optional: Vec<Vec<Node>>,
        arguments: Vec<Vec<Node>>,
        body: Vec<Node>,
    },
    /// A math island. `content` is the exact inner LaTeX source (parsed by L2). `display`
    /// distinguishes `$$…$$`/`\[…\]` from `$…$`/`\(…\)`.
    Math { display: bool, content: String },
    /// A comment (text without the `%` or trailing newline).
    Comment(String),
    /// Inline verbatim (`\verb<delim>…<delim>` / `\verb*…`): the body is the **raw** inner
    /// text (catcodes suspended), kept verbatim. `star` is the visible-space variant; `delim`
    /// is the chosen delimiter, preserved so the node round-trips.
    Verb { star: bool, delim: char, content: String },
    /// A verbatim *environment* (`\begin{verbatim}…\end{verbatim}` / `verbatim*`): the body is
    /// the **raw** inner text (catcodes suspended, newlines kept). `env` is the environment
    /// name verbatim, preserved so the node round-trips.
    VerbatimEnv { env: String, content: String },
    /// A text accent applied to its argument (L5c): `\'e` → é, `\c{c}` → ç. `accent` is the
    /// control-sequence name verbatim (`'`, `"`, `c`, `u`, …); `arg` is the accented content
    /// (a single-character text run, or a braced group's nodes). Produced only by the opt-in
    /// [`recognize_accents`](crate::recognize_accents) pass, not by L1.
    Accent { accent: String, arg: Vec<Node> },
    /// A sectioning heading (L5d): `\section{Title}`, `\section*{Title}`, `\section[Short]{Title}`
    /// (and the `\part`…`\subparagraph` family). `starred` is the no-number `*` form; `short` is
    /// the optional TOC/running-head title; `title` is the heading body. Produced only by the
    /// opt-in [`recognize_structure`](crate::recognize_structure) pass, not by L1.
    Section { level: SectionLevel, starred: bool, short: Option<Vec<Node>>, title: Vec<Node> },
    /// A cross-reference or citation (L5d): `\label{k}`, `\ref{k}`, `\eqref{k}`, `\cite[note]{k}`,
    /// … `command` is the control-word verbatim; `note` is the optional bracketed argument (the
    /// citation note); `target` is the mandatory key argument. Produced only by the opt-in
    /// [`recognize_structure`](crate::recognize_structure) pass, not by L1.
    CrossRef { command: String, note: Option<Vec<Node>>, target: Vec<Node> },
    /// A preamble directive (L5d): `\documentclass[opt]{article}`, `\usepackage[opt]{pkg}`,
    /// `\RequirePackage{pkg}`. `command` is the control-word verbatim; `options` is the optional
    /// bracketed package/class options; `name` is the mandatory class/package name. Produced only
    /// by the opt-in [`recognize_structure`](crate::recognize_structure) pass, not by L1.
    Preamble { command: String, options: Option<Vec<Node>>, name: Vec<Node> },
    /// An argument-form text font command (L5d): `\textbf{x}`, `\emph{x}`, `\underline{x}`, …
    /// `command` is the control-word verbatim; `content` is the wrapped argument. (Declaration-
    /// form font commands like `\bfseries` stay plain [`Node::Command`]s — their effect is
    /// positional, not a wrapped argument.) Produced only by the opt-in
    /// [`recognize_structure`](crate::recognize_structure) pass, not by L1.
    Styled { command: String, content: Vec<Node> },
    /// An active character that acts like a command — `~`.
    Active(char),
    /// A construct deliberately out of scope (the TeX-programmability asymptote — e.g.
    /// runtime `\catcode`). Not produced by L1; reserved so later layers can surface an
    /// honest "unsupported" rather than mis-parse.
    Unsupported { construct: String, span: (usize, usize) },
}

impl Node {
    /// If this is a [`Node::Math`] island, parse its raw content into the math AST
    /// ([`crate::MathNode`]); otherwise `None`. The structural tree is unchanged (L1
    /// round-trip intact) — parsed math is produced on demand here.
    pub fn parsed_math(&self) -> Option<Result<crate::MathNode, crate::ParseError>> {
        match self {
            Node::Math { content, .. } => Some(crate::parse_math(content)),
            _ => None,
        }
    }

    /// Render this node back to LaTeX source. `parse(&node.to_latex()) == [node]` up to the
    /// normalizations noted in the module docs.
    pub fn to_latex(&self) -> String {
        let mut s = String::new();
        self.write_latex(&mut s);
        s
    }

    fn write_latex(&self, out: &mut String) {
        match self {
            Node::Text(t) => out.push_str(t),
            Node::Space => out.push(' '),
            Node::Par => out.push_str("\n\n"),
            Node::Group(nodes) => {
                out.push('{');
                render_seq(nodes, out);
                out.push('}');
            }
            Node::Command { name, optional, arguments } => {
                out.push('\\');
                out.push_str(name);
                for opt in optional {
                    out.push('[');
                    render_seq(opt, out);
                    out.push(']');
                }
                for arg in arguments {
                    out.push('{');
                    render_seq(arg, out);
                    out.push('}');
                }
                // A bare control *word* (all letters, no args) needs a trailing space so a
                // following letter doesn't fuse into the command name (`\alpha`+`x`).
                let is_word = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic());
                if is_word && optional.is_empty() && arguments.is_empty() {
                    out.push(' ');
                }
            }
            Node::Environment { name, optional, arguments, body } => {
                out.push_str("\\begin{");
                out.push_str(name);
                out.push('}');
                for opt in optional {
                    out.push('[');
                    render_seq(opt, out);
                    out.push(']');
                }
                for arg in arguments {
                    out.push('{');
                    render_seq(arg, out);
                    out.push('}');
                }
                render_seq(body, out);
                out.push_str("\\end{");
                out.push_str(name);
                out.push('}');
            }
            Node::Math { display, content } => {
                let delim = if *display { "$$" } else { "$" };
                out.push_str(delim);
                out.push_str(content);
                out.push_str(delim);
            }
            Node::Comment(c) => {
                out.push('%');
                out.push_str(c);
                out.push('\n');
            }
            Node::Verb { star, delim, content } => {
                out.push_str("\\verb");
                if *star {
                    out.push('*');
                }
                out.push(*delim);
                out.push_str(content);
                out.push(*delim);
            }
            Node::VerbatimEnv { env, content } => {
                out.push_str("\\begin{");
                out.push_str(env);
                out.push('}');
                out.push_str(content);
                out.push_str("\\end{");
                out.push_str(env);
                out.push('}');
            }
            Node::Accent { accent, arg } => {
                // Render the braced form `\<accent>{arg}` — it re-recognizes to the same node
                // whether the source wrote `\'e` or `\'{e}`.
                out.push('\\');
                out.push_str(accent);
                out.push('{');
                render_seq(arg, out);
                out.push('}');
            }
            Node::Section { level, starred, short, title } => {
                // `\section` (+ `*` if starred) (+ `[short]` if present) `{title}`. Renders back
                // to the exact shape `recognize_structure` folds, so it re-recognizes equal.
                out.push('\\');
                out.push_str(level.command());
                if *starred {
                    out.push('*');
                }
                if let Some(short) = short {
                    out.push('[');
                    render_seq(short, out);
                    out.push(']');
                }
                out.push('{');
                render_seq(title, out);
                out.push('}');
            }
            Node::CrossRef { command, note, target } => {
                out.push('\\');
                out.push_str(command);
                if let Some(note) = note {
                    out.push('[');
                    render_seq(note, out);
                    out.push(']');
                }
                out.push('{');
                render_seq(target, out);
                out.push('}');
            }
            Node::Preamble { command, options, name } => {
                out.push('\\');
                out.push_str(command);
                if let Some(options) = options {
                    out.push('[');
                    render_seq(options, out);
                    out.push(']');
                }
                out.push('{');
                render_seq(name, out);
                out.push('}');
            }
            Node::Styled { command, content } => {
                out.push('\\');
                out.push_str(command);
                out.push('{');
                render_seq(content, out);
                out.push('}');
            }
            Node::Active(c) => out.push(*c),
            Node::Unsupported { construct, .. } => out.push_str(construct),
        }
    }
}

/// Render a sequence of nodes back to LaTeX.
pub fn render_seq(nodes: &[Node], out: &mut String) {
    for n in nodes {
        n.write_latex(out);
    }
}

/// Render a whole document (a node sequence) to a LaTeX string.
pub fn document_to_latex(nodes: &[Node]) -> String {
    let mut s = String::new();
    render_seq(nodes, &mut s);
    s
}
