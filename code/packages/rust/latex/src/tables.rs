//! Document-mode **table & list** recognition (D1) — an opt-in pass that folds the *generic*
//! environments [`parse`](crate::parse) (L1) produces for `tabular`/`tabular*` and
//! `itemize`/`enumerate`/`description` into the structured [`Node::Tabular`] and [`Node::List`].
//!
//! ## Why a separate opt-in pass
//!
//! Exactly like [`recognize_structure`](crate::recognize_structure) (sectioning) and
//! [`recognize_accents`](crate::recognize_accents) (accents), L1 already parses these
//! environments — `\begin{tabular}{lcr}a & b\end{tabular}` is a perfectly good generic
//! [`Node::Environment`], and it round-trips. What L1 deliberately does **not** do is say "this
//! is a 1×2 table with column spec `lcr`", splitting the body on the `&` alignment tab and the
//! `\\` row break. This pass adds that recognition **on demand**, so L1's own tree and
//! round-trip stay untouched (run it, or don't).
//!
//! ## What it recognizes (a table)
//!
//! ```text
//!   \begin{tabular}{l c r}          col_spec = Some("l c r")
//!     a & b & c \\                  row 0 = [ [a] , [b] , [c] ]     (& splits cells)
//!     d & e & f                     row 1 = [ [d] , [e] , [f] ]     (\\ splits rows)
//!   \end{tabular}
//! ```
//!
//! A cell is the run of nodes *between* alignment tabs; a row is the run of cells *between* row
//! breaks. `tabular*` carries a leading `{width}` argument before its `{colspec}` — we keep the
//! **last** mandatory argument as the column spec and drop the width (a width is not a column
//! spec; folding it away is faithful to the grid, and the node round-trips as a plain `tabular`).
//!
//! Lists split their body on `\item`: each item owns the nodes after its `\item` up to the next
//! one, and an `\item[term]` optional becomes the item's `label`.
//!
//! ## Totality (leave-as-is on doubt)
//!
//! The pass is **total and infallible** — it returns `Vec<Node>`, never a `Result`, and never
//! panics. Splitting on `&`/`\\`/`\item` is a pure *regrouping* of already-parsed sibling nodes,
//! so no source is dropped: a ragged grid (rows of differing cell counts) is preserved exactly,
//! with each row keeping its own length — **not** an error. A list environment whose body has
//! stray content *before* the first `\item` is left as a recognized-body [`Node::Environment`]
//! rather than folded, because folding questionable input would misrepresent it. Truly malformed
//! input (unbalanced braces, `\begin{a}…\end{b}` mismatch) never reaches here: the L1 parser
//! already rejects it with a spanned [`ParseError`](crate::ParseError). So the spec's
//! "spanned errors on malformed grids" guarantee lives in **L1**, upstream of this total pass.
//!
//! ## Bounded recursion, no brace counting
//!
//! Recursion descends only into child node-lists that L1 already produced; its depth is the L1
//! tree depth, which the parser caps (`MAX_DEPTH`), so adversarial nesting cannot overflow here
//! (no *new* unbounded recursion). The column spec is read off the already-parsed argument nodes
//! via [`Node::to_latex`] — there is **no raw brace counting** in this module.

use crate::ast::{ListItem, ListKind, Node};

/// Map a list-environment name to its [`ListKind`], or `None` if `name` is not one.
fn list_kind(name: &str) -> Option<ListKind> {
    Some(match name {
        "itemize" => ListKind::Itemize,
        "enumerate" => ListKind::Enumerate,
        "description" => ListKind::Description,
        _ => return None,
    })
}

/// Recognize document-mode tables and lists in `nodes` into [`Node::Tabular`] / [`Node::List`].
/// The input is typically [`parse`](crate::parse) output (optionally already
/// [`recognize_structure`](crate::recognize_structure)-folded — the passes are independent and
/// compose, since each only touches its own construct and recurses through the rest).
pub fn recognize_tables(nodes: Vec<Node>) -> Vec<Node> {
    nodes.into_iter().map(recurse).collect()
}

/// Rebuild one node, recursing `recognize_tables` into every child node-list, and — for the
/// table/list environments — folding it into the structured variant.
fn recurse(node: Node) -> Node {
    match node {
        Node::Environment { name, optional, arguments, body } => {
            // Recurse into the argument/optional lists and the body regardless, then decide
            // whether this environment is a table, a list, or just a recursed environment.
            let optional: Vec<Vec<Node>> = optional.into_iter().map(recognize_tables).collect();
            let arguments: Vec<Vec<Node>> = arguments.into_iter().map(recognize_tables).collect();
            let body = recognize_tables(body);

            if name == "tabular" || name == "tabular*" {
                return fold_tabular(&arguments, body);
            }
            if let Some(kind) = list_kind(&name) {
                return fold_list(kind, name, optional, arguments, body);
            }
            Node::Environment { name, optional, arguments, body }
        }

        // --- plain structural recursion into every other node's child lists -------------------
        Node::Group(inner) => Node::Group(recognize_tables(inner)),
        Node::Command { name, optional, arguments } => Node::Command {
            name,
            optional: optional.into_iter().map(recognize_tables).collect(),
            arguments: arguments.into_iter().map(recognize_tables).collect(),
        },
        Node::Section { level, starred, short, title } => Node::Section {
            level,
            starred,
            short: short.map(recognize_tables),
            title: recognize_tables(title),
        },
        Node::CrossRef { command, note, target } => Node::CrossRef {
            command,
            note: note.map(recognize_tables),
            target: recognize_tables(target),
        },
        Node::Preamble { command, options, name } => Node::Preamble {
            command,
            options: options.map(recognize_tables),
            name: recognize_tables(name),
        },
        Node::Styled { command, content } => {
            Node::Styled { command, content: recognize_tables(content) }
        }
        // Recurse into already-recognized tables/lists too, so the pass is idempotent and
        // composes with itself (e.g. a list nested inside a table cell).
        Node::Tabular { col_spec, rows } => Node::Tabular {
            col_spec,
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(recognize_tables).collect())
                .collect(),
        },
        Node::List { kind, items } => Node::List {
            kind,
            items: items
                .into_iter()
                .map(|it| ListItem {
                    label: it.label.map(recognize_tables),
                    body: recognize_tables(it.body),
                })
                .collect(),
        },
        // leaves (Text, Space, Par, Math, Comment, Verb, VerbatimEnv, Accent, Active,
        // Unsupported) carry no further child lists to fold here
        other => other,
    }
}

/// Fold a (already-recursed) `tabular`/`tabular*` body into a [`Node::Tabular`].
///
/// `col_spec` = the **last** mandatory argument rendered back to source (the only argument for
/// `tabular`; the `{colspec}` after `{width}` for `tabular*`), or `None` if there were none.
/// Rows are the body split on the `\\` row break; cells are each row split on the `&` tab.
fn fold_tabular(arguments: &[Vec<Node>], body: Vec<Node>) -> Node {
    let col_spec = arguments.last().map(|arg| crate::document_to_latex(arg));
    let rows = split_rows(body);
    Node::Tabular { col_spec, rows }
}

/// Split a tabular body into rows (on `\\`), each row into cells (on `Text("&")`).
///
/// A trailing all-empty row produced by a trailing `\\` (e.g. `a & b \\`) is dropped, so the
/// grid has one row per line of content — but a genuinely empty *cell* is kept, and ragged rows
/// keep their own cell counts (no padding, no error — faithfulness over regularization).
fn split_rows(body: Vec<Node>) -> Vec<Vec<Vec<Node>>> {
    // First split on the `\\` row break command.
    let mut rows: Vec<Vec<Node>> = Vec::new();
    let mut current: Vec<Node> = Vec::new();
    for node in body {
        if is_row_break(&node) {
            rows.push(std::mem::take(&mut current));
        } else {
            current.push(node);
        }
    }
    rows.push(current);

    // Drop a single trailing empty row from a trailing `\\` (`a & b \\` → one row, not two).
    // "Empty" means the row's nodes are only insignificant whitespace/comments.
    if rows.len() > 1 && rows.last().map(|r| is_blank_row(r)).unwrap_or(false) {
        rows.pop();
    }

    // Then split each row on the `&` alignment tab into cells.
    rows.into_iter().map(split_cells).collect()
}

/// Split one row's node run into cells at each `Text("&")` node. The `&` node itself is dropped
/// (it is the separator); everything else — including Space nodes inside a cell — is kept.
fn split_cells(row: Vec<Node>) -> Vec<Vec<Node>> {
    let mut cells: Vec<Vec<Node>> = Vec::new();
    let mut cell: Vec<Node> = Vec::new();
    for node in row {
        if is_align_tab(&node) {
            cells.push(std::mem::take(&mut cell));
        } else {
            cell.push(node);
        }
    }
    cells.push(cell);
    cells
}

/// Is this node the `\\` row-break command (`Command { name: "\\", .. }`)?
fn is_row_break(node: &Node) -> bool {
    matches!(node, Node::Command { name, optional, arguments }
        if name == "\\" && optional.is_empty() && arguments.is_empty())
}

/// Is this node the `&` alignment tab (its own `Text("&")` node between cells)?
fn is_align_tab(node: &Node) -> bool {
    matches!(node, Node::Text(t) if t == "&")
}

/// Does a row consist only of insignificant whitespace (Space/Par) and comments? Such a row is
/// what a trailing `\\` leaves behind, and is dropped so `a & b \\` is one row, not two.
fn is_blank_row(row: &[Node]) -> bool {
    row.iter()
        .all(|n| matches!(n, Node::Space | Node::Par | Node::Comment(_)))
}

/// Fold a (already-recursed) list environment body into a [`Node::List`], **unless** the body
/// has stray content before the first `\item` — in which case we leave it as a recognized-body
/// [`Node::Environment`] (totality: don't fold questionable input).
fn fold_list(
    kind: ListKind,
    name: String,
    optional: Vec<Vec<Node>>,
    arguments: Vec<Vec<Node>>,
    body: Vec<Node>,
) -> Node {
    // Ignoring leading insignificant whitespace/comments, does the body start with an `\item`?
    let first_significant = body
        .iter()
        .find(|n| !matches!(n, Node::Space | Node::Par | Node::Comment(_)));
    let starts_with_item = matches!(
        first_significant,
        Some(Node::Command { name, .. }) if name == "item"
    );
    if !starts_with_item {
        // Stray content before the first item — leave it as a generic (recursed) environment.
        return Node::Environment { name, optional, arguments, body };
    }

    // Split the body at each `\item`; each item's label = its optional term, body = the nodes
    // after it up to the next `\item`. Any leading whitespace before the first `\item` is
    // attached to the (soon-to-open) first item's *pre-body* — but since we verified the body
    // starts with `\item` (modulo whitespace), that leading whitespace is harmless and rare; we
    // keep it faithfully as leading nodes of the first item so nothing is dropped.
    let mut items: Vec<ListItem> = Vec::new();
    let mut pending_label: Option<Vec<Node>> = None;
    let mut current: Vec<Node> = Vec::new();
    let mut open = false; // are we inside an item yet?
    for node in body {
        match &node {
            Node::Command { name, optional, .. } if name == "item" => {
                // Close the previous item (if any) and open a new one.
                if open {
                    items.push(ListItem { label: pending_label.take(), body: std::mem::take(&mut current) });
                }
                pending_label = optional.first().cloned();
                current.clear();
                open = true;
            }
            _ => {
                // Content before the very first `\item` is attached as leading nodes of the
                // first item (only reachable for pure whitespace, per the guard above).
                current.push(node);
            }
        }
    }
    if open {
        items.push(ListItem { label: pending_label.take(), body: current });
    }

    Node::List { kind, items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document_to_latex, parse};

    /// Parse and recognize tables/lists, for concise assertions.
    fn tb(src: &str) -> Vec<Node> {
        recognize_tables(parse(src).expect("parse"))
    }

    /// The D1 round-trip: rendering a recognized node and re-recognizing yields the same node.
    fn round_trips(src: &str) {
        let a = tb(src);
        let rendered = document_to_latex(&a);
        let b = recognize_tables(parse(&rendered).expect("re-parse"));
        assert_eq!(a, b, "table/list round-trip: {src:?} -> {rendered:?}");
    }

    #[test]
    fn basic_2x2_tabular_with_col_spec() {
        match &tb(r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}")[0] {
            Node::Tabular { col_spec, rows } => {
                assert_eq!(col_spec.as_deref(), Some("lc"));
                assert_eq!(rows.len(), 2, "two rows");
                assert_eq!(rows[0].len(), 2, "row 0 has 2 cells");
                assert_eq!(rows[1].len(), 2, "row 1 has 2 cells");
                // Cell contents include the `a` text (plus surrounding Space nodes kept verbatim).
                assert!(rows[0][0].iter().any(|n| matches!(n, Node::Text(t) if t == "a")));
                assert!(rows[1][1].iter().any(|n| matches!(n, Node::Text(t) if t == "d")));
            }
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn no_col_spec_is_none() {
        match &tb(r"\begin{tabular}a & b\end{tabular}")[0] {
            Node::Tabular { col_spec, rows } => {
                assert_eq!(*col_spec, None);
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2);
            }
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn tabular_star_drops_width_keeps_colspec() {
        // `tabular*` carries {width}{colspec}; the LAST argument is the column spec.
        match &tb(r"\begin{tabular*}{2cm}{lr}a & b\end{tabular*}")[0] {
            Node::Tabular { col_spec, rows } => {
                assert_eq!(col_spec.as_deref(), Some("lr"));
                assert_eq!(rows[0].len(), 2);
            }
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn ragged_rows_preserved_not_error() {
        // Row 0 has 2 cells, row 1 has 3 — kept as-is, no padding, no error.
        match &tb(r"\begin{tabular}{c}a & b \\ c & d & e\end{tabular}")[0] {
            Node::Tabular { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[1].len(), 3);
            }
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn trailing_row_break_dropped() {
        // A trailing `\\` should not leave a spurious empty final row.
        match &tb(r"\begin{tabular}{c}a & b \\\end{tabular}")[0] {
            Node::Tabular { rows, .. } => assert_eq!(rows.len(), 1, "one row, trailing \\\\ dropped"),
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn itemize_two_items() {
        match &tb(r"\begin{itemize}\item one\item two\end{itemize}")[0] {
            Node::List { kind, items } => {
                assert_eq!(*kind, ListKind::Itemize);
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].label, None);
                assert!(items[0].body.iter().any(|n| matches!(n, Node::Text(t) if t == "one")));
                assert!(items[1].body.iter().any(|n| matches!(n, Node::Text(t) if t == "two")));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn item_label_captured() {
        match &tb(r"\begin{description}\item[Term] definition\end{description}")[0] {
            Node::List { kind, items } => {
                assert_eq!(*kind, ListKind::Description);
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].label, Some(vec![Node::Text("Term".into())]));
                assert!(items[0].body.iter().any(|n| matches!(n, Node::Text(t) if t == "definition")));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn enumerate_recognized() {
        match &tb(r"\begin{enumerate}\item a\item b\end{enumerate}")[0] {
            Node::List { kind, items } => {
                assert_eq!(*kind, ListKind::Enumerate);
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn nested_list_inside_item() {
        // An itemize inside an enumerate item is itself recognized.
        let src = r"\begin{enumerate}\item outer\begin{itemize}\item inner\end{itemize}\end{enumerate}";
        match &tb(src)[0] {
            Node::List { kind, items } => {
                assert_eq!(*kind, ListKind::Enumerate);
                assert_eq!(items.len(), 1);
                assert!(
                    items[0].body.iter().any(|n| matches!(n, Node::List { kind, .. } if *kind == ListKind::Itemize)),
                    "inner itemize should be a recognized List: {:#?}", items[0].body
                );
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn stray_content_before_first_item_stays_environment() {
        // Real content (not whitespace) before the first `\item` — leave as an Environment.
        match &tb(r"\begin{itemize}oops\item one\end{itemize}")[0] {
            Node::Environment { name, body, .. } => {
                assert_eq!(name, "itemize");
                assert!(body.iter().any(|n| matches!(n, Node::Text(t) if t == "oops")));
            }
            other => panic!("expected Environment (stray content), got {other:?}"),
        }
    }

    #[test]
    fn recurses_into_section_body() {
        // A tabular inside a (recognized) section title/body is itself recognized.
        let src = r"\begin{itemize}\item x\end{itemize}";
        let nodes = recognize_tables(
            crate::recognize_structure(parse(&format!(r"\section{{H}} {src}")).expect("parse")),
        );
        assert!(matches!(nodes[0], Node::Section { .. }));
        assert!(
            nodes.iter().any(|n| matches!(n, Node::List { .. })),
            "list after a section should be recognized: {nodes:#?}"
        );
    }

    #[test]
    fn recurses_into_group() {
        // A tabular wrapped in a brace group is still recognized inside the group.
        match &tb(r"{\begin{tabular}{c}a\end{tabular}}")[0] {
            Node::Group(inner) => {
                assert!(inner.iter().any(|n| matches!(n, Node::Tabular { .. })));
            }
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn idempotent() {
        let once = tb(r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}");
        let twice = recognize_tables(once.clone());
        assert_eq!(once, twice);
    }

    #[test]
    fn round_trips_cover_tables_and_lists() {
        for s in [
            r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}",
            r"\begin{tabular}{lcr}a & b & c\end{tabular}",
            r"\begin{tabular*}{2cm}{lr}a & b\end{tabular*}",
            r"\begin{tabular}a & b\end{tabular}",
            r"\begin{itemize}\item one\item two\end{itemize}",
            r"\begin{enumerate}\item a\item b\item c\end{enumerate}",
            r"\begin{description}\item[Term] def\item[Two] more\end{description}",
            r"\begin{enumerate}\item outer\begin{itemize}\item inner\end{itemize}\end{enumerate}",
        ] {
            round_trips(s);
        }
    }
}
