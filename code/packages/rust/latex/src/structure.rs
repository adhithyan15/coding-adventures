//! Document structure recognition (L5d) — an opt-in pass that classifies the *generic*
//! command nodes [`parse`](crate::parse) (L1) produces into **semantic** structure nodes:
//! sectioning, cross-references, preamble directives, and argument-form font commands.
//!
//! ## Why a separate pass
//!
//! Just like [`expand`](crate::expand) (macros) and [`recognize_accents`](crate::recognize_accents)
//! (accents), L1 already *round-trips* every one of these — `\section{Intro}` lexes to a
//! generic [`Node::Command`] and renders straight back. What L1 deliberately does **not** do
//! is say "this is a level-2 heading titled *Intro*", or "this is a citation of key *foo*".
//! This pass adds that **semantic** recognition on demand, so L1's own structure and
//! round-trip stay untouched (run it, or don't — the L1 tree is unchanged either way).
//!
//! ## What it recognizes
//!
//! | Source | Recognized node |
//! |--------|-----------------|
//! | `\section{T}`, `\section*{T}`, `\section[s]{T}` (+ the `\part`…`\subparagraph` family) | [`Node::Section`] |
//! | `\label{k}`, `\ref{k}`, `\eqref{k}`, `\cite[note]{k}`, … | [`Node::CrossRef`] |
//! | `\documentclass[opt]{article}`, `\usepackage[opt]{amsmath}`, `\RequirePackage{…}` | [`Node::Preamble`] |
//! | `\textbf{x}`, `\emph{x}`, `\underline{x}`, … (argument-form font cmds) | [`Node::Styled`] |
//!
//! ## The starred-form subtlety
//!
//! `\section{T}` captures its `{T}` as the command's argument at L1, so we read the title
//! straight off the command. But in `\section*{T}` the `*` (an *other* character) sits between
//! the control word and the brace, so L1 captures **no** argument — the tree is
//! `[Command("section"), Text("*"), Group([T])]`. This pass folds that `Text("*")` sibling and
//! the following group into one starred [`Node::Section`]. (Standard starred sections take no
//! optional TOC title, so the starred branch never carries `short`.)
//!
//! ## Totality
//!
//! Every branch either folds a well-formed construct or leaves the original command untouched
//! (recursing into its children) — a sectioning command with no title, a cross-ref with no
//! key, a styled command with the wrong argument count are all left as plain commands. Nothing
//! is dropped, nothing panics. Recursion is bounded by the tree depth, which the L1 parser
//! already caps (`MAX_DEPTH`), so adversarial nesting cannot overflow here.
//!
//! ## Font *declarations* stay plain
//!
//! Only the **argument-form** font commands (`\textbf{…}`) become [`Node::Styled`]. The
//! *declaration* forms (`\bfseries`, `\itshape`, `\large`, …) are intentionally **not**
//! recognized: their effect runs from the declaration to the end of the enclosing group, so
//! they have no single wrapped argument — modeling them as an argument node would misrepresent
//! their scope. They round-trip fine as plain commands.

use crate::ast::{Node, NodeKind, SectionLevel};
use crate::token::Span;

/// The smallest span covering both `a` and `b` — composes a synthesised node's span from its
/// constituents (S1: union-of-children; precise per-construct spans are S2's job).
fn union(a: Span, b: Span) -> Span {
    Span::new(a.start.min(b.start), a.end.max(b.end))
}

/// The span covering a whole node sequence (empty → `fallback`): the union of every node's span.
fn seq_span(nodes: &[Node], fallback: Span) -> Span {
    nodes.iter().map(|n| n.span).reduce(union).unwrap_or(fallback)
}

/// Map a sectioning control word to its [`SectionLevel`], or `None` if `name` is not a
/// sectioning command.
fn section_level(name: &str) -> Option<SectionLevel> {
    Some(match name {
        "part" => SectionLevel::Part,
        "chapter" => SectionLevel::Chapter,
        "section" => SectionLevel::Section,
        "subsection" => SectionLevel::Subsection,
        "subsubsection" => SectionLevel::Subsubsection,
        "paragraph" => SectionLevel::Paragraph,
        "subparagraph" => SectionLevel::Subparagraph,
        _ => return None,
    })
}

/// Is `name` a cross-reference / citation command (one mandatory key argument, optional note)?
fn is_crossref(name: &str) -> bool {
    matches!(
        name,
        "label" | "ref" | "eqref" | "pageref" | "autoref" | "nameref" | "cite" | "citep" | "citet"
    )
}

/// Is `name` a preamble directive (`[options]` + one mandatory name argument)?
fn is_preamble(name: &str) -> bool {
    matches!(name, "documentclass" | "usepackage" | "RequirePackage")
}

/// Is `name` an **argument-form** text font command (wraps exactly one mandatory argument)?
fn is_style(name: &str) -> bool {
    matches!(
        name,
        "textbf"
            | "textit"
            | "texttt"
            | "textrm"
            | "textsf"
            | "textsc"
            | "textsl"
            | "textmd"
            | "textup"
            | "textnormal"
            | "emph"
            | "underline"
    )
}

/// Recognize document-structure commands in `nodes` into semantic [`Node`]s. The input is
/// typically [`parse`](crate::parse) output (optionally already [`expand`](crate::expand)ed or
/// [`recognize_accents`](crate::recognize_accents)-folded — the passes are independent).
pub fn recognize_structure(nodes: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    let mut i = 0;
    while i < nodes.len() {
        // These recognized nodes fold an existing `Node::Command` (and, for the starred form,
        // its `*` and group siblings). The synthesised node keeps the command's own span — it
        // already covers `\name[opt]{arg}` — extended over any folded siblings.
        let cmd_span = nodes[i].span;
        if let NodeKind::Command { name, optional, arguments } = &nodes[i].kind {
            // --- Sectioning: \section{T} / \section*{T} / \section[s]{T} -------------------
            if let Some(level) = section_level(name) {
                if let Some(first) = arguments.first() {
                    // Captured-argument form (no `*` intervened): \section[short]{title}.
                    let short = optional.first().map(|o| recognize_structure(o.clone()));
                    let title = recognize_structure(first.clone());
                    out.push(Node::new(
                        NodeKind::Section { level, starred: false, short, title },
                        cmd_span,
                    ));
                    // Any further captured groups were not part of the heading — keep them.
                    for extra in arguments.iter().skip(1) {
                        let sp = seq_span(extra, cmd_span);
                        out.push(Node::group(recognize_structure(extra.clone()), sp));
                    }
                    i += 1;
                    continue;
                }
                // No captured argument: try the starred form `\section*{title}`.
                if optional.is_empty() {
                    if let Some(title) = starred_title(&nodes, i) {
                        // Span covers the command through the folded `*` and title group.
                        let span = union(cmd_span, nodes[i + 2].span);
                        out.push(Node::new(
                            NodeKind::Section { level, starred: true, short: None, title },
                            span,
                        ));
                        i += 3; // command + Text("*") + Group
                        continue;
                    }
                }
                // Not a well-formed heading — leave the command as-is.
                out.push(recurse(nodes[i].clone()));
                i += 1;
                continue;
            }

            // --- Cross-references / citations: \ref{k}, \cite[note]{k} --------------------
            if is_crossref(name) {
                if let Some(first) = arguments.first() {
                    let note = optional.first().map(|o| recognize_structure(o.clone()));
                    let target = recognize_structure(first.clone());
                    out.push(Node::new(
                        NodeKind::CrossRef { command: name.clone(), note, target },
                        cmd_span,
                    ));
                    for extra in arguments.iter().skip(1) {
                        let sp = seq_span(extra, cmd_span);
                        out.push(Node::group(recognize_structure(extra.clone()), sp));
                    }
                    i += 1;
                    continue;
                }
                out.push(recurse(nodes[i].clone()));
                i += 1;
                continue;
            }

            // --- Preamble: \documentclass[opt]{cls}, \usepackage[opt]{pkg} ----------------
            if is_preamble(name) {
                if let Some(first) = arguments.first() {
                    let options = optional.first().map(|o| recognize_structure(o.clone()));
                    let name_arg = recognize_structure(first.clone());
                    out.push(Node::new(
                        NodeKind::Preamble { command: name.clone(), options, name: name_arg },
                        cmd_span,
                    ));
                    for extra in arguments.iter().skip(1) {
                        let sp = seq_span(extra, cmd_span);
                        out.push(Node::group(recognize_structure(extra.clone()), sp));
                    }
                    i += 1;
                    continue;
                }
                out.push(recurse(nodes[i].clone()));
                i += 1;
                continue;
            }

            // --- Argument-form font commands: \textbf{x}, \emph{x} -----------------------
            if is_style(name) && optional.is_empty() && arguments.len() == 1 {
                let content = recognize_structure(arguments[0].clone());
                out.push(Node::new(NodeKind::Styled { command: name.clone(), content }, cmd_span));
                i += 1;
                continue;
            }
        }

        // Any other node (or a command that did not match): recurse into its children.
        out.push(recurse(nodes[i].clone()));
        i += 1;
    }
    out
}

/// If `nodes[i+1]` is exactly `Text("*")` and `nodes[i+2]` is a group, return the recognized
/// group as the starred heading's title; otherwise `None` (so the caller leaves the command
/// untouched). Only the canonical `\section*{title}` shape folds — unusual spacings such as
/// `\section* foo` are deliberately left alone, since they round-trip fine as plain nodes.
fn starred_title(nodes: &[Node], i: usize) -> Option<Vec<Node>> {
    match nodes.get(i + 1).map(|n| &n.kind) {
        Some(NodeKind::Text(star)) if star == "*" => match nodes.get(i + 2).map(|n| &n.kind) {
            Some(NodeKind::Group(inner)) => Some(recognize_structure(inner.clone())),
            _ => None,
        },
        _ => None,
    }
}

/// Recurse structure recognition into a node's child sequences without treating the node
/// itself as a structure command (mirrors `text::recurse`).
fn recurse(node: Node) -> Node {
    // Recursion regroups children but never changes the node's extent, so its span is kept.
    let span = node.span;
    let kind = match node.kind {
        NodeKind::Group(inner) => NodeKind::Group(recognize_structure(inner)),
        NodeKind::Command { name, optional, arguments } => NodeKind::Command {
            name,
            optional: optional.into_iter().map(recognize_structure).collect(),
            arguments: arguments.into_iter().map(recognize_structure).collect(),
        },
        NodeKind::Environment { name, optional, arguments, body } => NodeKind::Environment {
            name,
            optional: optional.into_iter().map(recognize_structure).collect(),
            arguments: arguments.into_iter().map(recognize_structure).collect(),
            body: recognize_structure(body),
        },
        // Recurse into the parts of already-recognized structure nodes too, so the pass is
        // idempotent and composes with itself (e.g. a section title containing a `\ref`).
        NodeKind::Section { level, starred, short, title } => NodeKind::Section {
            level,
            starred,
            short: short.map(recognize_structure),
            title: recognize_structure(title),
        },
        NodeKind::CrossRef { command, note, target } => NodeKind::CrossRef {
            command,
            note: note.map(recognize_structure),
            target: recognize_structure(target),
        },
        NodeKind::Preamble { command, options, name } => NodeKind::Preamble {
            command,
            options: options.map(recognize_structure),
            name: recognize_structure(name),
        },
        NodeKind::Styled { command, content } => NodeKind::Styled {
            command,
            content: recognize_structure(content),
        },
        // leaves (Text, Space, Par, Math, Comment, Verb, VerbatimEnv, Accent, Active,
        // Unsupported) carry no further structure to fold here
        other => other,
    };
    Node::new(kind, span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document_to_latex, parse};

    /// Parse and recognize structure, for concise assertions.
    fn st(src: &str) -> Vec<Node> {
        recognize_structure(parse(src).expect("parse"))
    }

    /// The node kinds only (spans checked separately) — for structural assertions.
    fn kinds(nodes: &[Node]) -> Vec<NodeKind> {
        nodes.iter().map(|n| n.kind.clone()).collect()
    }

    /// A dummy span for building expected `NodeKind` trees (equality ignores spans).
    fn z() -> Span {
        Span::new(0, 0)
    }

    /// The L5d round-trip: rendering a recognized tree and re-recognizing yields the same tree.
    fn round_trips(src: &str) {
        let a = st(src);
        let rendered = document_to_latex(&a);
        let b = recognize_structure(parse(&rendered).expect("re-parse"));
        assert_eq!(a, b, "structure round-trip: {src:?} -> {rendered:?}");
    }

    #[test]
    fn plain_section() {
        assert_eq!(
            kinds(&st(r"\section{Intro}")),
            vec![NodeKind::Section {
                level: SectionLevel::Section,
                starred: false,
                short: None,
                title: vec![Node::text("Intro", z())],
            }]
        );
    }

    #[test]
    fn starred_section_folds_the_star() {
        // \section*{Intro} → starred heading; the intervening Text("*") is consumed.
        assert_eq!(
            kinds(&st(r"\section*{Intro}")),
            vec![NodeKind::Section {
                level: SectionLevel::Section,
                starred: true,
                short: None,
                title: vec![Node::text("Intro", z())],
            }]
        );
    }

    #[test]
    fn section_span_slices_back_to_source() {
        // A recognized section covers its full source extent, incl. the starred `*` and group.
        let src = r"\section*{Intro}";
        let n = st(src);
        assert!(matches!(n[0].kind, NodeKind::Section { starred: true, .. }));
        assert_eq!(&src[n[0].span.start..n[0].span.end], src);
    }

    #[test]
    fn section_with_short_toc_title() {
        // \section[Intro]{Introduction} → optional short TOC title preserved.
        assert_eq!(
            kinds(&st(r"\section[Intro]{Introduction}")),
            vec![NodeKind::Section {
                level: SectionLevel::Section,
                starred: false,
                short: Some(vec![Node::text("Intro", z())]),
                title: vec![Node::text("Introduction", z())],
            }]
        );
    }

    #[test]
    fn all_seven_levels_recognized() {
        let cases = [
            (r"\part{a}", SectionLevel::Part),
            (r"\chapter{a}", SectionLevel::Chapter),
            (r"\section{a}", SectionLevel::Section),
            (r"\subsection{a}", SectionLevel::Subsection),
            (r"\subsubsection{a}", SectionLevel::Subsubsection),
            (r"\paragraph{a}", SectionLevel::Paragraph),
            (r"\subparagraph{a}", SectionLevel::Subparagraph),
        ];
        for (src, level) in cases {
            match &st(src)[0].kind {
                NodeKind::Section { level: got, .. } => assert_eq!(*got, level, "{src}"),
                other => panic!("expected Section for {src}, got {other:?}"),
            }
        }
    }

    #[test]
    fn section_without_title_stays_command() {
        // A bare `\section` (nothing accent-able / no title) is left untouched.
        assert_eq!(
            kinds(&st(r"\section")),
            vec![NodeKind::Command { name: "section".into(), optional: vec![], arguments: vec![] }]
        );
    }

    #[test]
    fn cross_references() {
        assert_eq!(
            kinds(&st(r"\label{eq:1}")),
            vec![NodeKind::CrossRef {
                command: "label".into(),
                note: None,
                target: vec![Node::text("eq:1", z())],
            }]
        );
        assert_eq!(
            kinds(&st(r"\ref{eq:1}")),
            vec![NodeKind::CrossRef {
                command: "ref".into(),
                note: None,
                target: vec![Node::text("eq:1", z())],
            }]
        );
    }

    #[test]
    fn citation_with_note() {
        // \cite[p.~5]{foo} keeps the optional note.
        match &st(r"\cite[p]{foo}")[0].kind {
            NodeKind::CrossRef { command, note, target } => {
                assert_eq!(command, "cite");
                assert_eq!(note, &Some(vec![Node::text("p", z())]));
                assert_eq!(target, &vec![Node::text("foo", z())]);
            }
            other => panic!("expected CrossRef, got {other:?}"),
        }
    }

    #[test]
    fn preamble_directives() {
        assert_eq!(
            kinds(&st(r"\documentclass[a4paper]{article}")),
            vec![NodeKind::Preamble {
                command: "documentclass".into(),
                options: Some(vec![Node::text("a4paper", z())]),
                name: vec![Node::text("article", z())],
            }]
        );
        assert_eq!(
            kinds(&st(r"\usepackage{amsmath}")),
            vec![NodeKind::Preamble {
                command: "usepackage".into(),
                options: None,
                name: vec![Node::text("amsmath", z())],
            }]
        );
    }

    #[test]
    fn styled_font_commands() {
        assert_eq!(
            kinds(&st(r"\textbf{bold}")),
            vec![NodeKind::Styled {
                command: "textbf".into(),
                content: vec![Node::text("bold", z())],
            }]
        );
        assert_eq!(
            kinds(&st(r"\emph{x}")),
            vec![NodeKind::Styled { command: "emph".into(), content: vec![Node::text("x", z())] }]
        );
    }

    #[test]
    fn font_declaration_stays_command() {
        // \bfseries is a declaration (no wrapped argument) — left as a plain command.
        assert_eq!(
            kinds(&st(r"\bfseries")),
            vec![NodeKind::Command { name: "bfseries".into(), optional: vec![], arguments: vec![] }]
        );
    }

    #[test]
    fn recognition_recurses_into_groups_and_titles() {
        // A \ref inside a section title is itself recognized.
        match &st(r"\section{see \ref{x}}")[0].kind {
            NodeKind::Section { title, .. } => {
                assert!(title.iter().any(|n| matches!(n.kind, NodeKind::CrossRef { .. })));
            }
            other => panic!("expected Section, got {other:?}"),
        }
    }

    #[test]
    fn surrounding_text_is_preserved() {
        // Heading then a paragraph: the text after the heading stays put.
        let nodes = st("\\section{Intro}\nhello");
        assert!(matches!(nodes[0].kind, NodeKind::Section { .. }));
        assert!(nodes.iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t.contains("hello"))));
    }

    #[test]
    fn idempotent() {
        // Running the pass twice changes nothing.
        let once = st(r"\section*{Intro} \textbf{x} \cite[p]{k}");
        let twice = recognize_structure(once.clone());
        assert_eq!(once, twice);
    }

    #[test]
    fn round_trips_cover_structure() {
        for s in [
            r"\section{Intro}",
            r"\section*{Intro}",
            r"\subsection[Short]{Long title}",
            r"\chapter{One}",
            r"\label{eq:1}",
            r"\ref{fig:2}",
            r"\eqref{e}",
            r"\cite{foo}",
            r"\cite[p]{foo}",
            r"\documentclass[a4paper]{article}",
            r"\usepackage{amsmath}",
            r"\textbf{bold}",
            r"\emph{x}",
            r"\underline{y}",
        ] {
            round_trips(s);
        }
    }
}
