//! Text accents (L5c) — an opt-in recognition pass that folds an accent control sequence
//! and the character it accents into a single [`Node::Accent`].
//!
//! ## Why a separate pass
//!
//! [`parse`](crate::parse) (L1) already *round-trips* accents — `\'e` lexes to a control
//! symbol command `\'` followed by the text `e`, and rendering puts them back. What L1 does
//! **not** do is say "this is an é". This pass adds that **semantic** recognition on demand,
//! exactly like [`expand`](crate::expand) for macros, so L1's own round-trip is untouched.
//!
//! ## What it recognizes
//!
//! The standard LaTeX text accents, in both spellings:
//! - **control-symbol accents** `\'  \`  \^  \"  \~  \=  \.` — never capture a brace argument
//!   at L1 (control symbols take none), so they pair with the *next* node;
//! - **control-word accents** `\u \v \H \c \d \b \r \t` — when written `\c{e}` L1 captures the
//!   `{e}` as the command's argument; when written `\c e` the lexer absorbs the space and the
//!   `e` is the next sibling. Both are handled.
//!
//! The accented argument is a single following character (`\'e` → é over `e`) or a braced
//! group (`\'{...}`). Recognition recurses into groups, command arguments, and environment
//! bodies, and `Node::Accent::to_latex` renders the braced form `\'{e}`, which re-recognizes
//! to the same node — so `recognize_accents(parse(&n.to_latex())) == [n]` (AST-equality).
//!
//! Total and panic-free: an accent with no accent-able argument (e.g. at end of input, or
//! before a space/structural node) is left as a plain command, never dropped or mis-folded.

use crate::ast::{Node, NodeKind};
use crate::token::Span;

/// The smallest span covering both `a` and `b` — used to compose a synthesised node's span from
/// its constituents (S2: an accent's span is the exact union of the accent command and its
/// argument's real spans).
fn union(a: Span, b: Span) -> Span {
    Span::new(a.start.min(b.start), a.end.max(b.end))
}

/// The span covering a whole node sequence (empty → `fallback`). The union of every node's span.
fn seq_span(nodes: &[Node], fallback: Span) -> Span {
    nodes.iter().map(|n| n.span).reduce(union).unwrap_or(fallback)
}

/// Is `name` a known text-accent control sequence (symbol or word spelling)?
fn is_accent(name: &str) -> bool {
    matches!(
        name,
        // control-symbol accents
        "'" | "`" | "^" | "\"" | "~" | "=" | "."
        // control-word accents (breve, caron, double-acute, cedilla, under-dot,
        // under-bar, ring, tie)
        | "u" | "v" | "H" | "c" | "d" | "b" | "r" | "t"
    )
}

/// Fold accent control sequences in a document tree into [`Node::Accent`] nodes. The input is
/// typically [`parse`](crate::parse) output (optionally already [`expand`](crate::expand)ed).
pub fn recognize_accents(nodes: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    let mut i = 0;
    while i < nodes.len() {
        let cmd_span = nodes[i].span;
        match &nodes[i].kind {
            NodeKind::Command { name, optional, arguments } if optional.is_empty() && is_accent(name) => {
                // Case A: the accent captured its argument as `{group}` (e.g. `\c{e}`).
                if let Some(first) = arguments.first() {
                    let arg = recognize_accents(first.clone());
                    // The recognized accent covers the accent command through the end of its
                    // argument — the union of the command span and the recognized arg's real span
                    // (an empty `\c{}` falls back to `cmd_span`, so the union is a safe no-op).
                    let span = union(cmd_span, seq_span(&arg, cmd_span));
                    out.push(Node::new(NodeKind::Accent { accent: name.clone(), arg }, span));
                    // Any further captured groups were not part of the accent — keep them.
                    for extra in arguments.iter().skip(1) {
                        let sp = seq_span(extra, cmd_span);
                        out.push(Node::group(recognize_accents(extra.clone()), sp));
                    }
                    i += 1;
                    continue;
                }
                // Case B: no captured argument — pair with the immediately following node.
                match nodes.get(i + 1).map(|n| (&n.kind, n.span)) {
                    Some((NodeKind::Group(inner), grp_span)) => {
                        let arg = recognize_accents(inner.clone());
                        // Span covers the accent command through the paired group.
                        out.push(Node::new(
                            NodeKind::Accent { accent: name.clone(), arg },
                            union(cmd_span, grp_span),
                        ));
                        i += 2;
                    }
                    Some((NodeKind::Text(t), text_span)) if !t.is_empty() => {
                        // The accent applies to the first character; the rest stays as text.
                        let mut chars = t.chars();
                        let first = chars.next().expect("non-empty");
                        let first_len = first.len_utf8();
                        let rest: String = chars.collect();
                        // The accented character occupies the first `first_len` bytes of the text
                        // run; the accent node covers the command through that character.
                        let base_span = Span::new(text_span.start, text_span.start + first_len);
                        out.push(Node::new(
                            NodeKind::Accent {
                                accent: name.clone(),
                                arg: vec![Node::text(first.to_string(), base_span)],
                            },
                            union(cmd_span, base_span),
                        ));
                        if !rest.is_empty() {
                            out.push(Node::text(rest, Span::new(base_span.end, text_span.end)));
                        }
                        i += 2;
                    }
                    // Nothing accent-able follows — leave the command as-is.
                    _ => {
                        out.push(nodes[i].clone());
                        i += 1;
                    }
                }
            }
            // Any other node: recurse into its child sequences, then keep it.
            _ => {
                out.push(recurse(nodes[i].clone()));
                i += 1;
            }
        }
    }
    out
}

/// Recurse accent recognition into a node's child sequences without treating the node itself
/// as an accent.
fn recurse(node: Node) -> Node {
    // Recursion regroups a node's children but never changes its extent, so the node keeps its
    // own span.
    let span = node.span;
    let kind = match node.kind {
        NodeKind::Group(inner) => NodeKind::Group(recognize_accents(inner)),
        NodeKind::Command { name, optional, arguments } => NodeKind::Command {
            name,
            optional: optional.into_iter().map(recognize_accents).collect(),
            arguments: arguments.into_iter().map(recognize_accents).collect(),
        },
        NodeKind::Environment { name, optional, arguments, body } => NodeKind::Environment {
            name,
            optional: optional.into_iter().map(recognize_accents).collect(),
            arguments: arguments.into_iter().map(recognize_accents).collect(),
            body: recognize_accents(body),
        },
        // leaves (Text, Space, Par, Math, Comment, Verb, VerbatimEnv, Active, Unsupported,
        // and already-built Accent) carry no further accent structure to fold here
        other => other,
    };
    Node::new(kind, span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document_to_latex, parse};

    /// Parse, recognize accents, and render back to LaTeX for easy assertions.
    fn acc(src: &str) -> Vec<Node> {
        recognize_accents(parse(src).expect("parse"))
    }

    /// The node kinds only (spans checked separately) — for structural assertions.
    fn kinds(nodes: &[Node]) -> Vec<NodeKind> {
        nodes.iter().map(|n| n.kind.clone()).collect()
    }

    /// A dummy span for building expected `NodeKind` trees (equality ignores spans).
    fn z() -> Span {
        Span::new(0, 0)
    }

    /// The L5c round-trip: rendering a recognized tree and re-recognizing yields the same tree.
    fn round_trips(src: &str) {
        let a = acc(src);
        let rendered = document_to_latex(&a);
        let b = recognize_accents(parse(&rendered).expect("re-parse"));
        assert_eq!(a, b, "accent round-trip: {src:?} -> {rendered:?}");
    }

    #[test]
    fn control_symbol_accent_over_next_char() {
        // \'e  → Accent("'", [e]), no leftover.
        assert_eq!(
            kinds(&acc(r"\'e")),
            vec![NodeKind::Accent { accent: "'".into(), arg: vec![Node::text("e", z())] }]
        );
    }

    #[test]
    fn accent_takes_only_first_char_rest_stays() {
        // \~nada → Accent(~, [n]) then Text("ada").
        assert_eq!(
            kinds(&acc(r"\~nada")),
            vec![
                NodeKind::Accent { accent: "~".into(), arg: vec![Node::text("n", z())] },
                NodeKind::Text("ada".into()),
            ]
        );
    }

    #[test]
    fn braced_accent_argument() {
        // \"{o} → Accent("\"", [o]).
        assert_eq!(
            kinds(&acc(r#"\"{o}"#)),
            vec![NodeKind::Accent { accent: "\"".into(), arg: vec![Node::text("o", z())] }]
        );
    }

    #[test]
    fn control_word_accent_with_braced_arg() {
        // \c{c} → Accent("c", [c]) (cedilla).
        assert_eq!(
            kinds(&acc(r"\c{c}")),
            vec![NodeKind::Accent { accent: "c".into(), arg: vec![Node::text("c", z())] }]
        );
    }

    #[test]
    fn control_word_accent_bare_arg() {
        // \v s → lexer absorbs the space, so `s` is the next sibling → Accent("v", [s]).
        assert_eq!(
            kinds(&acc(r"\v s")),
            vec![NodeKind::Accent { accent: "v".into(), arg: vec![Node::text("s", z())] }]
        );
    }

    #[test]
    fn accent_span_slices_back_to_source() {
        // The recognized accent covers its full source extent (command through base).
        let src = r"caf\'e";
        let n = acc(src);
        // nodes: Text("caf"), Accent("'", [e])
        let accent = &n[1];
        assert!(matches!(accent.kind, NodeKind::Accent { .. }));
        assert_eq!(&src[accent.span.start..accent.span.end], r"\'e");
    }

    #[test]
    fn accent_span_covers_command_plus_control_symbol_arg() {
        // `\'e` — the Accent span slices back to the accent command through its base character.
        let src = r"\'e";
        let n = acc(src);
        assert!(matches!(n[0].kind, NodeKind::Accent { .. }));
        assert_eq!(&src[n[0].span.start..n[0].span.end], r"\'e");
    }

    #[test]
    fn accent_span_covers_braced_control_word_arg() {
        // `\c{c}` — the Accent span slices back to the whole `\c{c}` command+argument.
        let src = r"\c{c}";
        let n = acc(src);
        assert!(matches!(n[0].kind, NodeKind::Accent { .. }));
        assert_eq!(&src[n[0].span.start..n[0].span.end], r"\c{c}");
    }

    #[test]
    fn accent_span_contains_its_argument() {
        // Containment: the accent's span ⊇ its argument node's span, and both ⊆ `0..len`.
        let src = r"caf\'{e}";
        let n = acc(src);
        let accent = n.iter().find(|x| matches!(x.kind, NodeKind::Accent { .. })).expect("accent");
        assert!(accent.span.end <= src.len());
        if let NodeKind::Accent { arg, .. } = &accent.kind {
            for a in arg {
                assert!(
                    accent.span.start <= a.span.start && a.span.end <= accent.span.end,
                    "arg span {:?} not ⊆ accent span {:?}",
                    a.span,
                    accent.span
                );
            }
        }
    }

    #[test]
    fn accent_span_fixed_point_modulo_re_recognition() {
        let src = r"caf\'e";
        let a = acc(src);
        let rendered = document_to_latex(&a);
        let b = recognize_accents(parse(&rendered).expect("re-parse"));
        assert_eq!(a, b, "accent tree equal modulo spans");
        let accent = b.iter().find(|x| matches!(x.kind, NodeKind::Accent { .. })).expect("accent");
        // The re-recognized accent's span still slices back to a `\<accent>{…}` extent.
        assert!(rendered[accent.span.start..accent.span.end].starts_with('\\'));
    }

    #[test]
    fn accent_inside_a_group_is_recognized() {
        let n = acc(r"{\'e}");
        match &n[0].kind {
            NodeKind::Group(inner) => assert!(matches!(inner[0].kind, NodeKind::Accent { .. })),
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn non_accent_command_untouched() {
        // \textbf{x} is not an accent.
        assert_eq!(
            kinds(&acc(r"\textbf{x}")),
            vec![NodeKind::Command {
                name: "textbf".into(),
                optional: vec![],
                arguments: vec![vec![Node::text("x", z())]],
            }]
        );
    }

    #[test]
    fn dangling_accent_left_as_command() {
        // `\'` with nothing accent-able after it stays a plain command (no panic, no drop).
        assert_eq!(
            kinds(&acc(r"\'")),
            vec![NodeKind::Command { name: "'".into(), optional: vec![], arguments: vec![] }]
        );
    }

    #[test]
    fn round_trips_cover_accents() {
        for s in [r"\'e", r#"caf\'e"#, r#"\"o"#, r"\~n", r"\^o", r"\c{c}", r"\=e", r"\.z"] {
            round_trips(s);
        }
    }
}
