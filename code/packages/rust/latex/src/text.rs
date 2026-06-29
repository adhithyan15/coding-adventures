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

use crate::ast::Node;

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
        match &nodes[i] {
            Node::Command { name, optional, arguments } if optional.is_empty() && is_accent(name) => {
                // Case A: the accent captured its argument as `{group}` (e.g. `\c{e}`).
                if let Some(first) = arguments.first() {
                    let arg = recognize_accents(first.clone());
                    out.push(Node::Accent { accent: name.clone(), arg });
                    // Any further captured groups were not part of the accent — keep them.
                    for extra in arguments.iter().skip(1) {
                        out.push(Node::Group(recognize_accents(extra.clone())));
                    }
                    i += 1;
                    continue;
                }
                // Case B: no captured argument — pair with the immediately following node.
                match nodes.get(i + 1) {
                    Some(Node::Group(inner)) => {
                        let arg = recognize_accents(inner.clone());
                        out.push(Node::Accent { accent: name.clone(), arg });
                        i += 2;
                    }
                    Some(Node::Text(t)) if !t.is_empty() => {
                        // The accent applies to the first character; the rest stays as text.
                        let mut chars = t.chars();
                        let first = chars.next().expect("non-empty");
                        let rest: String = chars.collect();
                        out.push(Node::Accent {
                            accent: name.clone(),
                            arg: vec![Node::Text(first.to_string())],
                        });
                        if !rest.is_empty() {
                            out.push(Node::Text(rest));
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
    match node {
        Node::Group(inner) => Node::Group(recognize_accents(inner)),
        Node::Command { name, optional, arguments } => Node::Command {
            name,
            optional: optional.into_iter().map(recognize_accents).collect(),
            arguments: arguments.into_iter().map(recognize_accents).collect(),
        },
        Node::Environment { name, optional, arguments, body } => Node::Environment {
            name,
            optional: optional.into_iter().map(recognize_accents).collect(),
            arguments: arguments.into_iter().map(recognize_accents).collect(),
            body: recognize_accents(body),
        },
        // leaves (Text, Space, Par, Math, Comment, Verb, VerbatimEnv, Active, Unsupported,
        // and already-built Accent) carry no further accent structure to fold here
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document_to_latex, parse};

    /// Parse, recognize accents, and render back to LaTeX for easy assertions.
    fn acc(src: &str) -> Vec<Node> {
        recognize_accents(parse(src).expect("parse"))
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
            acc(r"\'e"),
            vec![Node::Accent { accent: "'".into(), arg: vec![Node::Text("e".into())] }]
        );
    }

    #[test]
    fn accent_takes_only_first_char_rest_stays() {
        // \~nada → Accent(~, [n]) then Text("ada").
        assert_eq!(
            acc(r"\~nada"),
            vec![
                Node::Accent { accent: "~".into(), arg: vec![Node::Text("n".into())] },
                Node::Text("ada".into()),
            ]
        );
    }

    #[test]
    fn braced_accent_argument() {
        // \"{o} → Accent("\"", [o]).
        assert_eq!(
            acc(r#"\"{o}"#),
            vec![Node::Accent { accent: "\"".into(), arg: vec![Node::Text("o".into())] }]
        );
    }

    #[test]
    fn control_word_accent_with_braced_arg() {
        // \c{c} → Accent("c", [c]) (cedilla).
        assert_eq!(
            acc(r"\c{c}"),
            vec![Node::Accent { accent: "c".into(), arg: vec![Node::Text("c".into())] }]
        );
    }

    #[test]
    fn control_word_accent_bare_arg() {
        // \v s → lexer absorbs the space, so `s` is the next sibling → Accent("v", [s]).
        assert_eq!(
            acc(r"\v s"),
            vec![Node::Accent { accent: "v".into(), arg: vec![Node::Text("s".into())] }]
        );
    }

    #[test]
    fn accent_inside_a_group_is_recognized() {
        let n = acc(r"{\'e}");
        match &n[0] {
            Node::Group(inner) => assert!(matches!(inner[0], Node::Accent { .. })),
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn non_accent_command_untouched() {
        // \textbf{x} is not an accent.
        assert_eq!(
            acc(r"\textbf{x}"),
            vec![Node::Command {
                name: "textbf".into(),
                optional: vec![],
                arguments: vec![vec![Node::Text("x".into())]],
            }]
        );
    }

    #[test]
    fn dangling_accent_left_as_command() {
        // `\'` with nothing accent-able after it stays a plain command (no panic, no drop).
        assert_eq!(
            acc(r"\'"),
            vec![Node::Command { name: "'".into(), optional: vec![], arguments: vec![] }]
        );
    }

    #[test]
    fn round_trips_cover_accents() {
        for s in [r"\'e", r#"caf\'e"#, r#"\"o"#, r"\~n", r"\^o", r"\c{c}", r"\=e", r"\.z"] {
            round_trips(s);
        }
    }
}
