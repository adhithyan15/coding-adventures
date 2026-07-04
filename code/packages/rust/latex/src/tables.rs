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

use crate::ast::{ListItem, ListKind, Node, NodeKind};
use crate::token::Span;

/// The smallest span covering both `a` and `b` — folds a synthesised node's span from its parts.
fn union(a: Span, b: Span) -> Span {
    Span::new(a.start.min(b.start), a.end.max(b.end))
}

/// The span covering a whole node sequence (empty → `fallback`): the union of every node's span.
/// Used to fold a cell's / row's / item's content spans into one range (S2).
fn seq_span(nodes: &[Node], fallback: Span) -> Span {
    nodes.iter().map(|n| n.span).reduce(union).unwrap_or(fallback)
}

/// The span of a whole grid: the union of every cell's content span, folded onto `anchor`
/// (the `\begin{tabular}…\end{tabular}` environment span, which already brackets the grid). An
/// empty grid keeps `anchor`, so the result is never smaller than the environment delimiters.
fn grid_span(anchor: Span, rows: &[Vec<Vec<Node>>]) -> Span {
    let mut span = anchor;
    for row in rows {
        for cell in row {
            span = union(span, seq_span(cell, anchor));
        }
    }
    span
}

/// The span of a whole list: the union of every item's label + body span, folded onto `anchor`
/// (the `\begin{env}…\end{env}` environment span, which already brackets the items).
fn list_span(anchor: Span, items: &[ListItem]) -> Span {
    let mut span = anchor;
    for item in items {
        if let Some(label) = &item.label {
            span = union(span, seq_span(label, anchor));
        }
        span = union(span, seq_span(&item.body, anchor));
    }
    span
}

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
    // The node's own span already covers its full source extent; recursion/folding regroups its
    // children without changing that extent (a `tabular`/list environment folds `\begin`…`\end`,
    // which is exactly the environment node's span), so we keep it.
    let span = node.span;
    let kind = match node.kind {
        NodeKind::Environment { name, optional, arguments, body } => {
            // Recurse into the argument/optional lists and the body regardless, then decide
            // whether this environment is a table, a list, or just a recursed environment.
            let optional: Vec<Vec<Node>> = optional.into_iter().map(recognize_tables).collect();
            let arguments: Vec<Vec<Node>> = arguments.into_iter().map(recognize_tables).collect();
            let body = recognize_tables(body);

            if name == "tabular" || name == "tabular*" {
                let kind = fold_tabular(&arguments, body);
                // S2: the Tabular's span is the union of `\begin{tabular}…\end{tabular}` (the
                // env span) with every cell's content span — the exact grid extent.
                let tab_span = match &kind {
                    NodeKind::Tabular { rows, .. } => grid_span(span, rows),
                    _ => span,
                };
                return Node::new(kind, tab_span);
            }
            if let Some(kind) = list_kind(&name) {
                let folded = fold_list(kind, name, optional, arguments, body, span);
                // S2: a folded List's span is the union of `\begin{env}…\end{env}` with every
                // item's span; an un-folded (stray-content) Environment keeps its own env span.
                let list_span = match &folded {
                    NodeKind::List { items, .. } => list_span(span, items),
                    _ => span,
                };
                return Node::new(folded, list_span);
            }
            NodeKind::Environment { name, optional, arguments, body }
        }

        // --- plain structural recursion into every other node's child lists -------------------
        NodeKind::Group(inner) => NodeKind::Group(recognize_tables(inner)),
        NodeKind::Command { name, optional, arguments } => NodeKind::Command {
            name,
            optional: optional.into_iter().map(recognize_tables).collect(),
            arguments: arguments.into_iter().map(recognize_tables).collect(),
        },
        NodeKind::Section { level, starred, short, title } => NodeKind::Section {
            level,
            starred,
            short: short.map(recognize_tables),
            title: recognize_tables(title),
        },
        NodeKind::CrossRef { command, note, target } => NodeKind::CrossRef {
            command,
            note: note.map(recognize_tables),
            target: recognize_tables(target),
        },
        NodeKind::Preamble { command, options, name } => NodeKind::Preamble {
            command,
            options: options.map(recognize_tables),
            name: recognize_tables(name),
        },
        NodeKind::Styled { command, content } => {
            NodeKind::Styled { command, content: recognize_tables(content) }
        }
        // Recurse into already-recognized tables/lists too, so the pass is idempotent and
        // composes with itself (e.g. a list nested inside a table cell).
        NodeKind::Tabular { col_spec, rows } => NodeKind::Tabular {
            col_spec,
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(recognize_tables).collect())
                .collect(),
        },
        NodeKind::List { kind, items } => NodeKind::List {
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
    };
    Node::new(kind, span)
}

/// Fold a (already-recursed) `tabular`/`tabular*` body into a [`Node::Tabular`].
///
/// `col_spec` = the **last** mandatory argument rendered back to source (the only argument for
/// `tabular`; the `{colspec}` after `{width}` for `tabular*`), or `None` if there were none.
/// Rows are the body split on the `\\` row break; cells are each row split on the `&` tab.
fn fold_tabular(arguments: &[Vec<Node>], body: Vec<Node>) -> NodeKind {
    let col_spec = arguments.last().map(|arg| crate::document_to_latex(arg));
    let rows = split_rows(body);
    NodeKind::Tabular { col_spec, rows }
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
    matches!(&node.kind, NodeKind::Command { name, optional, arguments }
        if name == "\\" && optional.is_empty() && arguments.is_empty())
}

/// Is this node the `&` alignment tab (its own `Text("&")` node between cells)?
fn is_align_tab(node: &Node) -> bool {
    matches!(&node.kind, NodeKind::Text(t) if t == "&")
}

/// Does a row consist only of insignificant whitespace (Space/Par) and comments? Such a row is
/// what a trailing `\\` leaves behind, and is dropped so `a & b \\` is one row, not two.
fn is_blank_row(row: &[Node]) -> bool {
    row.iter()
        .all(|n| matches!(n.kind, NodeKind::Space | NodeKind::Par | NodeKind::Comment(_)))
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
    _env_span: Span,
) -> NodeKind {
    // Ignoring leading insignificant whitespace/comments, does the body start with an `\item`?
    let first_significant = body
        .iter()
        .find(|n| !matches!(n.kind, NodeKind::Space | NodeKind::Par | NodeKind::Comment(_)));
    let starts_with_item = matches!(
        first_significant.map(|n| &n.kind),
        Some(NodeKind::Command { name, .. }) if name == "item"
    );
    if !starts_with_item {
        // Stray content before the first item — leave it as a generic (recursed) environment.
        return NodeKind::Environment { name, optional, arguments, body };
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
        match &node.kind {
            NodeKind::Command { name, optional, .. } if name == "item" => {
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

    NodeKind::List { kind, items }
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
        match &tb(r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}")[0].kind {
            NodeKind::Tabular { col_spec, rows } => {
                assert_eq!(col_spec.as_deref(), Some("lc"));
                assert_eq!(rows.len(), 2, "two rows");
                assert_eq!(rows[0].len(), 2, "row 0 has 2 cells");
                assert_eq!(rows[1].len(), 2, "row 1 has 2 cells");
                // Cell contents include the `a` text (plus surrounding Space nodes kept verbatim).
                assert!(rows[0][0].iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t == "a")));
                assert!(rows[1][1].iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t == "d")));
            }
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn tabular_span_slices_back_to_source() {
        // The recognized tabular covers `\begin{tabular}…\end{tabular}` exactly.
        let src = r"\begin{tabular}{lc}a & b\end{tabular}";
        let n = tb(src);
        assert!(matches!(n[0].kind, NodeKind::Tabular { .. }));
        assert_eq!(&src[n[0].span.start..n[0].span.end], src);
    }

    #[test]
    fn list_span_slices_back_to_source() {
        let src = r"\begin{itemize}\item one\item two\end{itemize}";
        let n = tb(src);
        assert!(matches!(n[0].kind, NodeKind::List { .. }));
        assert_eq!(&src[n[0].span.start..n[0].span.end], src);
    }

    #[test]
    fn tabular_span_is_exact_grid_extent() {
        // A 2×2 grid: the Tabular span slices back to `\begin{tabular}…\end{tabular}` exactly,
        // and every cell's content span is contained within it (S2 grid_span union).
        let src = r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}";
        let n = tb(src);
        assert_eq!(&src[n[0].span.start..n[0].span.end], src);
        if let NodeKind::Tabular { rows, .. } = &n[0].kind {
            for row in rows {
                for cell in row {
                    for node in cell {
                        assert!(
                            n[0].span.start <= node.span.start && node.span.end <= n[0].span.end,
                            "cell node span {:?} not ⊆ tabular span {:?}",
                            node.span,
                            n[0].span
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn itemize_span_and_per_item_extents_slice_back() {
        // The List span covers `\begin{itemize}…\end{itemize}`; each item's body span (the union
        // of its content nodes) slices back to that item's own `\item …` extent.
        let src = r"\begin{itemize}\item one\item two\end{itemize}";
        let n = tb(src);
        assert_eq!(&src[n[0].span.start..n[0].span.end], src);
        if let NodeKind::List { items, .. } = &n[0].kind {
            assert_eq!(items.len(), 2);
            // item 0 body = ` one` (leading space the lexer kept after `\item`), item 1 = ` two`.
            let body0 = seq_span(&items[0].body, n[0].span);
            let body1 = seq_span(&items[1].body, n[0].span);
            assert_eq!(&src[body0.start..body0.end], "one");
            assert_eq!(&src[body1.start..body1.end], "two");
            // Each item body ⊆ the list span.
            for b in [body0, body1] {
                assert!(n[0].span.start <= b.start && b.end <= n[0].span.end);
            }
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn description_item_label_and_body_spans_slice_back() {
        let src = r"\begin{description}\item[Term] definition\end{description}";
        let n = tb(src);
        if let NodeKind::List { items, .. } = &n[0].kind {
            let label = items[0].label.as_ref().expect("label");
            let lsp = seq_span(label, n[0].span);
            assert_eq!(&src[lsp.start..lsp.end], "Term");
            // `\item[Term] definition`: the `]` ends the control word, so the following Space is
            // kept as the body's leading node — the body span slices to ` definition` (with space).
            let bsp = seq_span(&items[0].body, n[0].span);
            assert_eq!(&src[bsp.start..bsp.end], " definition");
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn tabular_and_list_spans_fixed_point_modulo_re_recognition() {
        for src in [
            r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}",
            r"\begin{itemize}\item one\item two\end{itemize}",
        ] {
            let a = tb(src);
            let rendered = document_to_latex(&a);
            let b = recognize_tables(parse(&rendered).expect("re-parse"));
            assert_eq!(a, b, "table/list tree equal modulo spans: {src:?}");
            // The top node still slices back to the freshly-rendered whole source.
            assert_eq!(&rendered[b[0].span.start..b[0].span.end], rendered.as_str());
        }
    }

    #[test]
    fn no_col_spec_is_none() {
        match &tb(r"\begin{tabular}a & b\end{tabular}")[0].kind {
            NodeKind::Tabular { col_spec, rows } => {
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
        match &tb(r"\begin{tabular*}{2cm}{lr}a & b\end{tabular*}")[0].kind {
            NodeKind::Tabular { col_spec, rows } => {
                assert_eq!(col_spec.as_deref(), Some("lr"));
                assert_eq!(rows[0].len(), 2);
            }
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn ragged_rows_preserved_not_error() {
        // Row 0 has 2 cells, row 1 has 3 — kept as-is, no padding, no error.
        match &tb(r"\begin{tabular}{c}a & b \\ c & d & e\end{tabular}")[0].kind {
            NodeKind::Tabular { rows, .. } => {
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
        match &tb(r"\begin{tabular}{c}a & b \\\end{tabular}")[0].kind {
            NodeKind::Tabular { rows, .. } => assert_eq!(rows.len(), 1, "one row, trailing \\\\ dropped"),
            other => panic!("expected Tabular, got {other:?}"),
        }
    }

    #[test]
    fn itemize_two_items() {
        match &tb(r"\begin{itemize}\item one\item two\end{itemize}")[0].kind {
            NodeKind::List { kind, items } => {
                assert_eq!(*kind, ListKind::Itemize);
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].label, None);
                assert!(items[0].body.iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t == "one")));
                assert!(items[1].body.iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t == "two")));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn item_label_captured() {
        match &tb(r"\begin{description}\item[Term] definition\end{description}")[0].kind {
            NodeKind::List { kind, items } => {
                assert_eq!(*kind, ListKind::Description);
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].label, Some(vec![Node::text("Term", Span::new(0, 0))]));
                assert!(items[0].body.iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t == "definition")));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn enumerate_recognized() {
        match &tb(r"\begin{enumerate}\item a\item b\end{enumerate}")[0].kind {
            NodeKind::List { kind, items } => {
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
        match &tb(src)[0].kind {
            NodeKind::List { kind, items } => {
                assert_eq!(*kind, ListKind::Enumerate);
                assert_eq!(items.len(), 1);
                assert!(
                    items[0].body.iter().any(|n| matches!(&n.kind, NodeKind::List { kind, .. } if *kind == ListKind::Itemize)),
                    "inner itemize should be a recognized List: {:#?}", items[0].body
                );
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn stray_content_before_first_item_stays_environment() {
        // Real content (not whitespace) before the first `\item` — leave as an Environment.
        match &tb(r"\begin{itemize}oops\item one\end{itemize}")[0].kind {
            NodeKind::Environment { name, body, .. } => {
                assert_eq!(name, "itemize");
                assert!(body.iter().any(|n| matches!(&n.kind, NodeKind::Text(t) if t == "oops")));
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
        assert!(matches!(nodes[0].kind, NodeKind::Section { .. }));
        assert!(
            nodes.iter().any(|n| matches!(n.kind, NodeKind::List { .. })),
            "list after a section should be recognized: {nodes:#?}"
        );
    }

    #[test]
    fn recurses_into_group() {
        // A tabular wrapped in a brace group is still recognized inside the group.
        match &tb(r"{\begin{tabular}{c}a\end{tabular}}")[0].kind {
            NodeKind::Group(inner) => {
                assert!(inner.iter().any(|n| matches!(n.kind, NodeKind::Tabular { .. })));
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
