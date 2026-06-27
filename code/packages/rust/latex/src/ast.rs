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
    /// An active character that acts like a command — `~`.
    Active(char),
    /// A construct deliberately out of scope (the TeX-programmability asymptote — e.g.
    /// runtime `\catcode`). Not produced by L1; reserved so later layers can surface an
    /// honest "unsupported" rather than mis-parse.
    Unsupported { construct: String, span: (usize, usize) },
}

impl Node {
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
