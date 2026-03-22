// visualization.rs -- Graph Visualization in Multiple Formats
// ===========================================================
//
// This module converts directed graphs into human-readable text formats.
// It supports three output formats, each serving a different purpose:
//
// 1. **DOT format** (Graphviz) -- the industry standard for graph
//    visualization. Paste the output into https://dreampuf.github.io/GraphvizOnline/
//    or pipe it to `dot -Tpng` to get a rendered image.
//
// 2. **Mermaid format** -- a lightweight alternative that renders directly
//    in GitHub Markdown, Notion, and many other tools. Wrap the output in
//    a ```mermaid code fence and it just works.
//
// 3. **ASCII table** -- a plain-text representation for terminal output.
//    For labeled graphs, this produces a transition table (like an FSM
//    state table). For unlabeled graphs, it produces an adjacency list.
//
// # Why three formats?
//
// Each format has a sweet spot:
//
//   - DOT is the most powerful: supports node shapes, colors, subgraphs.
//   - Mermaid is the most convenient: renders inline in documentation.
//   - ASCII tables are the most portable: work everywhere.
//
// # Function naming
//
// Since Rust doesn't have method overloading, we use separate functions
// for `Graph` and `LabeledDirectedGraph`:
//
//   - `to_dot` / `labeled_to_dot`
//   - `to_mermaid` / `labeled_to_mermaid`
//   - `to_ascii_table` / `labeled_to_ascii_table`

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::graph::Graph;
use crate::labeled_graph::LabeledDirectedGraph;

// ═══════════════════════════════════════════════════════════════════════
// DotOptions -- controls DOT output rendering
// ═══════════════════════════════════════════════════════════════════════
//
// - `name`: The graph name (appears as `digraph <name> { ... }`).
//   Defaults to "G".
//
// - `node_attrs`: Per-node DOT attributes. The outer map is keyed by
//   node name. The inner map is keyed by attribute name (e.g., "shape")
//   with values (e.g., "doublecircle").
//
// - `initial`: If `Some`, adds an invisible start node with an arrow
//   to this node. Standard way to mark the initial state in FSM diagrams.
//
// - `rankdir`: Layout direction. "LR" means left-to-right, "TB" means
//   top-to-bottom. Defaults to "LR".

/// Options for configuring DOT output.
pub struct DotOptions {
    /// The graph name in the DOT output. Defaults to "G".
    pub name: String,
    /// Per-node DOT attributes (e.g., shape, color).
    pub node_attrs: HashMap<String, HashMap<String, String>>,
    /// If set, adds an invisible start node pointing to this node.
    pub initial: Option<String>,
    /// Layout direction: "LR" (left-to-right) or "TB" (top-to-bottom).
    pub rankdir: String,
}

impl Default for DotOptions {
    /// Create default DOT options: name "G", rankdir "LR", no special attrs.
    fn default() -> Self {
        DotOptions {
            name: "G".to_string(),
            node_attrs: HashMap::new(),
            initial: None,
            rankdir: "LR".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: format DOT attributes
// ---------------------------------------------------------------------------

/// Convert a map of DOT attributes to the bracketed format.
///
/// Example: `{"shape": "circle", "color": "red"}` becomes `[color=red, shape=circle]`
///
/// Attributes are sorted by key for deterministic output.
fn format_dot_attrs(attrs: &HashMap<String, String>) -> String {
    // Use BTreeMap for sorted iteration.
    let sorted: BTreeMap<&String, &String> = attrs.iter().collect();
    let parts: Vec<String> = sorted
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    format!("[{}]", parts.join(", "))
}

// ---------------------------------------------------------------------------
// Helper: collect edge labels for a labeled graph
// ---------------------------------------------------------------------------

/// Given a labeled graph, build a lookup from (from, to) to a combined
/// label string like "coin, push".
///
/// We sort the labels alphabetically so the output is deterministic --
/// important for tests and diffing.
fn collect_edge_labels(graph: &LabeledDirectedGraph) -> BTreeMap<(String, String), String> {
    let mut grouped: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();

    for (from, to, label) in graph.edges() {
        grouped
            .entry((from, to))
            .or_insert_with(BTreeSet::new)
            .insert(label);
    }

    grouped
        .into_iter()
        .map(|(key, labels)| {
            let combined: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            (key, combined.join(", "))
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════
// to_dot -- Graphviz DOT format for unlabeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// The DOT language is the standard input format for Graphviz. A DOT file
// describes a graph using a simple text syntax:
//
//   digraph G {
//       A -> B;
//       B -> C;
//   }
//
// Nodes are declared explicitly so that isolated nodes (with no edges)
// still appear in the output.

/// Convert an unlabeled `Graph` to Graphviz DOT format.
///
/// # Example
///
/// ```
/// use directed_graph::Graph;
/// use directed_graph::visualization::{to_dot, DotOptions};
///
/// let mut g = Graph::new();
/// g.add_edge("A", "B").unwrap();
/// let dot = to_dot(&g, &DotOptions::default());
/// assert!(dot.contains("A -> B"));
/// ```
pub fn to_dot(graph: &Graph, opts: &DotOptions) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push(format!("digraph {} {{", opts.name));
    lines.push(format!("    rankdir={};", opts.rankdir));

    // Initial state marker: invisible node with arrow.
    if let Some(ref initial) = opts.initial {
        lines.push("    \"\" [shape=none];".to_string());
        lines.push(format!("    \"\" -> {};", initial));
    }

    // Node declarations (sorted for deterministic output).
    for node in graph.nodes() {
        if let Some(attrs) = opts.node_attrs.get(&node) {
            lines.push(format!("    {} {};", node, format_dot_attrs(attrs)));
        } else {
            lines.push(format!("    {};", node));
        }
    }

    // Edge declarations (already sorted by Graph::edges()).
    for (from, to) in graph.edges() {
        lines.push(format!("    {} -> {};", from, to));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════
// labeled_to_dot -- Graphviz DOT format for labeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// For labeled graphs, edges get [label="..."] attributes. If multiple
// labels exist on the same (from, to) pair, they are combined as
// "a, b" in a single label attribute.

/// Convert a `LabeledDirectedGraph` to Graphviz DOT format.
///
/// # Example
///
/// ```
/// use directed_graph::LabeledDirectedGraph;
/// use directed_graph::visualization::{labeled_to_dot, DotOptions};
///
/// let mut lg = LabeledDirectedGraph::new_allow_self_loops();
/// lg.add_edge("locked", "unlocked", "coin").unwrap();
/// lg.add_edge("locked", "locked", "push").unwrap();
/// let dot = labeled_to_dot(&lg, &DotOptions::default());
/// assert!(dot.contains(r#"[label="coin"]"#));
/// ```
pub fn labeled_to_dot(graph: &LabeledDirectedGraph, opts: &DotOptions) -> String {
    let mut lines: Vec<String> = Vec::new();
    let edge_labels = collect_edge_labels(graph);

    lines.push(format!("digraph {} {{", opts.name));
    lines.push(format!("    rankdir={};", opts.rankdir));

    // Initial state marker.
    if let Some(ref initial) = opts.initial {
        lines.push("    \"\" [shape=none];".to_string());
        lines.push(format!("    \"\" -> {};", initial));
    }

    // Node declarations.
    for node in graph.nodes() {
        if let Some(attrs) = opts.node_attrs.get(&node) {
            lines.push(format!("    {} {};", node, format_dot_attrs(attrs)));
        } else {
            lines.push(format!("    {};", node));
        }
    }

    // Edge declarations with labels.
    // The edge_labels BTreeMap is already sorted.
    for ((from, to), label) in &edge_labels {
        lines.push(format!("    {} -> {} [label=\"{}\"];", from, to, label));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════
// to_mermaid -- Mermaid flowchart format for unlabeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// Mermaid is a JavaScript-based diagramming tool that renders directly
// in Markdown. The syntax for a flowchart is:
//
//   graph LR
//       A --> B
//       B --> C

/// Convert an unlabeled `Graph` to Mermaid flowchart format.
///
/// The `direction` parameter controls flow direction: "LR" (left-to-right)
/// or "TD" (top-down). Pass an empty string for the default ("LR").
pub fn to_mermaid(graph: &Graph, direction: &str) -> String {
    let dir = if direction.is_empty() { "LR" } else { direction };

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("graph {}", dir));

    for (from, to) in graph.edges() {
        lines.push(format!("    {} --> {}", from, to));
    }

    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════
// labeled_to_mermaid -- Mermaid flowchart for labeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// For labeled edges, Mermaid uses the -->|label| syntax:
//
//   A -->|coin| B

/// Convert a `LabeledDirectedGraph` to Mermaid flowchart format.
///
/// The `direction` parameter controls flow direction: "LR" or "TD".
/// Pass an empty string for the default ("LR").
pub fn labeled_to_mermaid(graph: &LabeledDirectedGraph, direction: &str) -> String {
    let dir = if direction.is_empty() { "LR" } else { direction };
    let edge_labels = collect_edge_labels(graph);

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("graph {}", dir));

    for ((from, to), label) in &edge_labels {
        lines.push(format!("    {} -->|{}| {}", from, label, to));
    }

    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════
// to_ascii_table -- Plain text adjacency list for unlabeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// For unlabeled graphs, we produce a two-column table:
//
//   Node    | Successors
//   --------+-----------
//   A       | B, C
//   B       | D
//   C       | D
//   D       | -

/// Convert an unlabeled `Graph` to a plain-text adjacency table.
///
/// Each row shows a node and its sorted list of successors.
/// A dash "-" indicates no successors.
pub fn to_ascii_table(graph: &Graph) -> String {
    let nodes = graph.nodes();

    // Build successor strings for each node.
    let mut succ_strings: BTreeMap<String, String> = BTreeMap::new();
    for node in &nodes {
        let succs = graph.successors(node).unwrap_or_default();
        if succs.is_empty() {
            succ_strings.insert(node.clone(), "-".to_string());
        } else {
            succ_strings.insert(node.clone(), succs.join(", "));
        }
    }

    // Calculate column widths.
    let node_col_width = std::cmp::max(
        "Node".len(),
        nodes.iter().map(|n| n.len()).max().unwrap_or(0),
    );
    let succ_col_width = std::cmp::max(
        "Successors".len(),
        succ_strings.values().map(|s| s.len()).max().unwrap_or(0),
    );

    // Build the table.
    let mut lines: Vec<String> = Vec::new();

    // Header.
    lines.push(format!(
        "{:<node_w$} | {:<succ_w$}",
        "Node",
        "Successors",
        node_w = node_col_width,
        succ_w = succ_col_width
    ));

    // Separator.
    lines.push(format!(
        "{}-+-{}",
        "-".repeat(node_col_width),
        "-".repeat(succ_col_width)
    ));

    // Data rows.
    for node in &nodes {
        let succ_str = &succ_strings[node];
        lines.push(format!(
            "{:<node_w$} | {:<succ_w$}",
            node,
            succ_str,
            node_w = node_col_width,
            succ_w = succ_col_width
        ));
    }

    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════
// labeled_to_ascii_table -- Transition table for labeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// For labeled graphs, we produce a transition table where rows are nodes
// (states), columns are unique labels (input symbols), and cells are
// destination nodes (next states):
//
//   State      | coin      | push
//   -----------+-----------+----------
//   locked     | unlocked  | locked
//   unlocked   | unlocked  | locked

/// Convert a `LabeledDirectedGraph` to a plain-text transition table.
///
/// Rows are nodes (states), columns are unique labels, cells are
/// destination nodes. A dash "-" means no transition exists.
pub fn labeled_to_ascii_table(graph: &LabeledDirectedGraph) -> String {
    let nodes = graph.nodes();
    let edges = graph.edges();

    // Step 1: Collect all unique labels.
    let mut label_set: BTreeSet<String> = BTreeSet::new();
    for (_, _, label) in &edges {
        label_set.insert(label.clone());
    }
    let labels: Vec<String> = label_set.into_iter().collect();

    // Handle edge case: no labels.
    if labels.is_empty() {
        let state_col_width = std::cmp::max(
            "State".len(),
            nodes.iter().map(|n| n.len()).max().unwrap_or(0),
        );
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("{:<w$}", "State", w = state_col_width));
        lines.push("-".repeat(state_col_width));
        for node in &nodes {
            lines.push(format!("{:<w$}", node, w = state_col_width));
        }
        return lines.join("\n");
    }

    // Step 2: Build transition map.
    // transitions[(node, label)] = sorted list of destination nodes.
    let mut transitions: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();

    for (from, to, label) in &edges {
        transitions
            .entry((from.clone(), label.clone()))
            .or_insert_with(BTreeSet::new)
            .insert(to.clone());
    }

    // Step 3: Calculate column widths.
    let state_col_width = std::cmp::max(
        "State".len(),
        nodes.iter().map(|n| n.len()).max().unwrap_or(0),
    );

    let label_col_widths: Vec<usize> = labels
        .iter()
        .map(|label| {
            let max_cell = nodes
                .iter()
                .map(|node| {
                    let key = (node.clone(), label.clone());
                    match transitions.get(&key) {
                        Some(dests) => {
                            let joined: Vec<&str> =
                                dests.iter().map(|s| s.as_str()).collect();
                            joined.join(", ").len()
                        }
                        None => 1, // "-" is 1 char
                    }
                })
                .max()
                .unwrap_or(1);
            std::cmp::max(label.len(), max_cell)
        })
        .collect();

    // Step 4: Build the formatted table.
    let mut lines: Vec<String> = Vec::new();

    // Header row.
    let mut header = format!("{:<w$}", "State", w = state_col_width);
    for (i, label) in labels.iter().enumerate() {
        header.push_str(&format!(" | {:<w$}", label, w = label_col_widths[i]));
    }
    lines.push(header);

    // Separator row.
    let mut sep = "-".repeat(state_col_width);
    for width in &label_col_widths {
        sep.push_str(&format!("-+-{}", "-".repeat(*width)));
    }
    lines.push(sep);

    // Data rows.
    for node in &nodes {
        let mut row = format!("{:<w$}", node, w = state_col_width);
        for (i, label) in labels.iter().enumerate() {
            let key = (node.clone(), label.clone());
            let cell_text = match transitions.get(&key) {
                Some(dests) => {
                    let joined: Vec<&str> = dests.iter().map(|s| s.as_str()).collect();
                    joined.join(", ")
                }
                None => "-".to_string(),
            };
            row.push_str(&format!(" | {:<w$}", cell_text, w = label_col_widths[i]));
        }
        lines.push(row);
    }

    lines.join("\n")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // Helper: build a turnstile FSM (used in many tests)
    // ═══════════════════════════════════════════════════════════════════════
    //
    // The turnstile is a classic two-state FSM:
    //
    //   locked --coin--> unlocked
    //   locked --push--> locked
    //   unlocked --coin--> unlocked
    //   unlocked --push--> locked

    fn turnstile() -> LabeledDirectedGraph {
        let mut lg = LabeledDirectedGraph::new_allow_self_loops();
        lg.add_edge("locked", "unlocked", "coin").unwrap();
        lg.add_edge("locked", "locked", "push").unwrap();
        lg.add_edge("unlocked", "locked", "push").unwrap();
        lg.add_edge("unlocked", "unlocked", "coin").unwrap();
        lg
    }

    fn simple_dag() -> Graph {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("A", "C").unwrap();
        g.add_edge("B", "D").unwrap();
        g.add_edge("C", "D").unwrap();
        g
    }

    // ═══════════════════════════════════════════════════════════════════════
    // to_dot -- unlabeled graph tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_to_dot_empty_graph() {
        let g = Graph::new();
        let dot = to_dot(&g, &DotOptions::default());
        assert!(dot.contains("digraph G {"));
        assert!(dot.contains("rankdir=LR;"));
        assert!(dot.ends_with('}'));
    }

    #[test]
    fn test_to_dot_single_node() {
        let mut g = Graph::new();
        g.add_node("A");
        let dot = to_dot(&g, &DotOptions::default());
        assert!(dot.contains("    A;"));
    }

    #[test]
    fn test_to_dot_single_edge() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        let dot = to_dot(&g, &DotOptions::default());
        assert!(dot.contains("    A -> B;"));
    }

    #[test]
    fn test_to_dot_diamond_dag() {
        let g = simple_dag();
        let dot = to_dot(&g, &DotOptions::default());
        assert!(dot.contains("A -> B;"));
        assert!(dot.contains("A -> C;"));
        assert!(dot.contains("B -> D;"));
        assert!(dot.contains("C -> D;"));
    }

    #[test]
    fn test_to_dot_custom_name() {
        let g = Graph::new();
        let opts = DotOptions {
            name: "MyGraph".to_string(),
            ..Default::default()
        };
        let dot = to_dot(&g, &opts);
        assert!(dot.contains("digraph MyGraph {"));
    }

    #[test]
    fn test_to_dot_tb_rankdir() {
        let g = Graph::new();
        let opts = DotOptions {
            rankdir: "TB".to_string(),
            ..Default::default()
        };
        let dot = to_dot(&g, &opts);
        assert!(dot.contains("rankdir=TB;"));
    }

    #[test]
    fn test_to_dot_node_attrs() {
        let mut g = Graph::new();
        g.add_node("A");
        let mut node_attrs = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert("shape".to_string(), "circle".to_string());
        node_attrs.insert("A".to_string(), attrs);

        let opts = DotOptions {
            node_attrs,
            ..Default::default()
        };
        let dot = to_dot(&g, &opts);
        assert!(dot.contains("A [shape=circle];"));
    }

    #[test]
    fn test_to_dot_initial_state() {
        let mut g = Graph::new();
        g.add_node("start");
        let opts = DotOptions {
            initial: Some("start".to_string()),
            ..Default::default()
        };
        let dot = to_dot(&g, &opts);
        assert!(dot.contains("\"\" [shape=none];"));
        assert!(dot.contains("\"\" -> start;"));
    }

    #[test]
    fn test_to_dot_multiple_node_attrs() {
        let mut g = Graph::new();
        g.add_node("A");
        let mut node_attrs = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert("shape".to_string(), "circle".to_string());
        attrs.insert("color".to_string(), "red".to_string());
        node_attrs.insert("A".to_string(), attrs);

        let opts = DotOptions {
            node_attrs,
            ..Default::default()
        };
        let dot = to_dot(&g, &opts);
        assert!(dot.contains("A [color=red, shape=circle];"));
    }

    #[test]
    fn test_to_dot_nodes_without_attrs_still_listed() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        let mut node_attrs = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert("shape".to_string(), "circle".to_string());
        node_attrs.insert("A".to_string(), attrs);

        let opts = DotOptions {
            node_attrs,
            ..Default::default()
        };
        let dot = to_dot(&g, &opts);
        assert!(dot.contains("    B;"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // labeled_to_dot tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_labeled_to_dot_empty() {
        let lg = LabeledDirectedGraph::new();
        let dot = labeled_to_dot(&lg, &DotOptions::default());
        assert!(dot.contains("digraph G {"));
    }

    #[test]
    fn test_labeled_to_dot_single_edge() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        let dot = labeled_to_dot(&lg, &DotOptions::default());
        assert!(dot.contains(r#"A -> B [label="compile"];"#));
    }

    #[test]
    fn test_labeled_to_dot_multiple_labels() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        let dot = labeled_to_dot(&lg, &DotOptions::default());
        assert!(dot.contains(r#"A -> B [label="compile, test"];"#));
    }

    #[test]
    fn test_labeled_to_dot_turnstile() {
        let lg = turnstile();
        let dot = labeled_to_dot(&lg, &DotOptions::default());
        assert!(dot.contains(r#"locked -> locked [label="push"];"#));
        assert!(dot.contains(r#"locked -> unlocked [label="coin"];"#));
        assert!(dot.contains(r#"unlocked -> locked [label="push"];"#));
        assert!(dot.contains(r#"unlocked -> unlocked [label="coin"];"#));
    }

    #[test]
    fn test_labeled_to_dot_initial_state() {
        let lg = turnstile();
        let opts = DotOptions {
            initial: Some("locked".to_string()),
            ..Default::default()
        };
        let dot = labeled_to_dot(&lg, &opts);
        assert!(dot.contains("\"\" [shape=none];"));
        assert!(dot.contains("\"\" -> locked;"));
    }

    #[test]
    fn test_labeled_to_dot_node_attrs() {
        let lg = turnstile();
        let mut node_attrs = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert("shape".to_string(), "doublecircle".to_string());
        node_attrs.insert("unlocked".to_string(), attrs);

        let opts = DotOptions {
            node_attrs,
            ..Default::default()
        };
        let dot = labeled_to_dot(&lg, &opts);
        assert!(dot.contains("unlocked [shape=doublecircle];"));
    }

    #[test]
    fn test_labeled_to_dot_custom_name() {
        let lg = LabeledDirectedGraph::new();
        let opts = DotOptions {
            name: "FSM".to_string(),
            ..Default::default()
        };
        let dot = labeled_to_dot(&lg, &opts);
        assert!(dot.contains("digraph FSM {"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // to_mermaid -- unlabeled graph tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_to_mermaid_empty() {
        let g = Graph::new();
        let m = to_mermaid(&g, "");
        assert_eq!(m, "graph LR");
    }

    #[test]
    fn test_to_mermaid_single_edge() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        let m = to_mermaid(&g, "");
        assert!(m.contains("A --> B"));
    }

    #[test]
    fn test_to_mermaid_diamond() {
        let g = simple_dag();
        let m = to_mermaid(&g, "LR");
        assert!(m.contains("graph LR"));
        assert!(m.contains("A --> B"));
        assert!(m.contains("A --> C"));
        assert!(m.contains("B --> D"));
        assert!(m.contains("C --> D"));
    }

    #[test]
    fn test_to_mermaid_td_direction() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        let m = to_mermaid(&g, "TD");
        assert!(m.contains("graph TD"));
    }

    #[test]
    fn test_to_mermaid_default_direction() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        let m = to_mermaid(&g, "");
        assert!(m.contains("graph LR"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // labeled_to_mermaid tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_labeled_to_mermaid_empty() {
        let lg = LabeledDirectedGraph::new();
        let m = labeled_to_mermaid(&lg, "");
        assert_eq!(m, "graph LR");
    }

    #[test]
    fn test_labeled_to_mermaid_single_edge() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        let m = labeled_to_mermaid(&lg, "LR");
        assert!(m.contains("A -->|compile| B"));
    }

    #[test]
    fn test_labeled_to_mermaid_multiple_labels() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        let m = labeled_to_mermaid(&lg, "");
        assert!(m.contains("A -->|compile, test| B"));
    }

    #[test]
    fn test_labeled_to_mermaid_turnstile() {
        let lg = turnstile();
        let m = labeled_to_mermaid(&lg, "LR");
        assert!(m.contains("locked -->|coin| unlocked"));
        assert!(m.contains("locked -->|push| locked"));
        assert!(m.contains("unlocked -->|coin| unlocked"));
        assert!(m.contains("unlocked -->|push| locked"));
    }

    #[test]
    fn test_labeled_to_mermaid_td_direction() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "dep").unwrap();
        let m = labeled_to_mermaid(&lg, "TD");
        assert!(m.contains("graph TD"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // to_ascii_table -- unlabeled graph tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_to_ascii_table_empty() {
        let g = Graph::new();
        let table = to_ascii_table(&g);
        assert!(table.contains("Node"));
        assert!(table.contains("Successors"));
    }

    #[test]
    fn test_to_ascii_table_single_node() {
        let mut g = Graph::new();
        g.add_node("A");
        let table = to_ascii_table(&g);
        assert!(table.contains("A"));
        assert!(table.contains("-"));
    }

    #[test]
    fn test_to_ascii_table_single_edge() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        let table = to_ascii_table(&g);
        assert!(table.contains("A"));
        assert!(table.contains("B"));
    }

    #[test]
    fn test_to_ascii_table_diamond() {
        let g = simple_dag();
        let table = to_ascii_table(&g);
        // A has successors B, C
        assert!(table.contains("B, C"));
        // D has no successors
        let lines: Vec<&str> = table.lines().collect();
        let d_line = lines.iter().find(|l| l.starts_with("D")).unwrap();
        assert!(d_line.contains("-"));
    }

    #[test]
    fn test_to_ascii_table_header_separator() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        let table = to_ascii_table(&g);
        let lines: Vec<&str> = table.lines().collect();
        assert!(lines[0].contains("Node"));
        assert!(lines[0].contains("Successors"));
        assert!(lines[1].contains("-+-"));
    }

    #[test]
    fn test_to_ascii_table_column_alignment() {
        let mut g = Graph::new();
        g.add_edge("short", "very_long_name").unwrap();
        let table = to_ascii_table(&g);
        // The column should be wide enough for the longest name.
        assert!(table.contains("very_long_name"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // labeled_to_ascii_table tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_labeled_to_ascii_table_empty() {
        let lg = LabeledDirectedGraph::new();
        let table = labeled_to_ascii_table(&lg);
        assert!(table.contains("State"));
    }

    #[test]
    fn test_labeled_to_ascii_table_single_edge() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "dep").unwrap();
        let table = labeled_to_ascii_table(&lg);
        assert!(table.contains("State"));
        assert!(table.contains("dep"));
        assert!(table.contains("B"));
    }

    #[test]
    fn test_labeled_to_ascii_table_turnstile() {
        let lg = turnstile();
        let table = labeled_to_ascii_table(&lg);
        // Check headers.
        assert!(table.contains("State"));
        assert!(table.contains("coin"));
        assert!(table.contains("push"));
        // Check transitions.
        let lines: Vec<&str> = table.lines().collect();
        let locked_line = lines.iter().find(|l| l.starts_with("locked ")).unwrap();
        assert!(locked_line.contains("unlocked"));
    }

    #[test]
    fn test_labeled_to_ascii_table_separator() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "dep").unwrap();
        let table = labeled_to_ascii_table(&lg);
        let lines: Vec<&str> = table.lines().collect();
        assert!(lines[1].contains("-+-"));
    }

    #[test]
    fn test_labeled_to_ascii_table_missing_transition() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "x").unwrap();
        lg.add_node("C");
        let table = labeled_to_ascii_table(&lg);
        let lines: Vec<&str> = table.lines().collect();
        // Node C should have "-" for the "x" column.
        let c_line = lines.iter().find(|l| l.starts_with("C")).unwrap();
        assert!(c_line.contains("-"));
    }

    #[test]
    fn test_labeled_to_ascii_table_only_nodes_no_edges() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_node("A");
        lg.add_node("B");
        let table = labeled_to_ascii_table(&lg);
        assert!(table.contains("State"));
        assert!(table.contains("A"));
        assert!(table.contains("B"));
    }

    #[test]
    fn test_labeled_to_ascii_table_multiple_labels() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "x").unwrap();
        lg.add_edge("A", "C", "y").unwrap();
        let table = labeled_to_ascii_table(&lg);
        assert!(table.contains("x"));
        assert!(table.contains("y"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Determinism tests -- output should be identical across runs
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_dot_deterministic() {
        let g = simple_dag();
        let dot1 = to_dot(&g, &DotOptions::default());
        let dot2 = to_dot(&g, &DotOptions::default());
        assert_eq!(dot1, dot2);
    }

    #[test]
    fn test_mermaid_deterministic() {
        let g = simple_dag();
        let m1 = to_mermaid(&g, "LR");
        let m2 = to_mermaid(&g, "LR");
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_ascii_table_deterministic() {
        let g = simple_dag();
        let t1 = to_ascii_table(&g);
        let t2 = to_ascii_table(&g);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_labeled_dot_deterministic() {
        let lg = turnstile();
        let d1 = labeled_to_dot(&lg, &DotOptions::default());
        let d2 = labeled_to_dot(&lg, &DotOptions::default());
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_labeled_mermaid_deterministic() {
        let lg = turnstile();
        let m1 = labeled_to_mermaid(&lg, "LR");
        let m2 = labeled_to_mermaid(&lg, "LR");
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_labeled_ascii_table_deterministic() {
        let lg = turnstile();
        let t1 = labeled_to_ascii_table(&lg);
        let t2 = labeled_to_ascii_table(&lg);
        assert_eq!(t1, t2);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Edge cases and complex scenarios
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_to_dot_isolated_nodes() {
        let mut g = Graph::new();
        g.add_node("X");
        g.add_node("Y");
        let dot = to_dot(&g, &DotOptions::default());
        assert!(dot.contains("    X;"));
        assert!(dot.contains("    Y;"));
    }

    #[test]
    fn test_to_mermaid_chain() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        g.add_edge("C", "D").unwrap();
        let m = to_mermaid(&g, "LR");
        assert!(m.contains("A --> B"));
        assert!(m.contains("B --> C"));
        assert!(m.contains("C --> D"));
    }

    #[test]
    fn test_labeled_to_dot_three_labels_same_edge() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "x").unwrap();
        lg.add_edge("A", "B", "y").unwrap();
        lg.add_edge("A", "B", "z").unwrap();
        let dot = labeled_to_dot(&lg, &DotOptions::default());
        assert!(dot.contains(r#"A -> B [label="x, y, z"];"#));
    }

    #[test]
    fn test_labeled_to_mermaid_three_labels() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "x").unwrap();
        lg.add_edge("A", "B", "y").unwrap();
        lg.add_edge("A", "B", "z").unwrap();
        let m = labeled_to_mermaid(&lg, "");
        assert!(m.contains("A -->|x, y, z| B"));
    }

    #[test]
    fn test_to_ascii_table_long_successor_list() {
        let mut g = Graph::new();
        g.add_edge("hub", "A").unwrap();
        g.add_edge("hub", "B").unwrap();
        g.add_edge("hub", "C").unwrap();
        g.add_edge("hub", "D").unwrap();
        let table = to_ascii_table(&g);
        assert!(table.contains("A, B, C, D"));
    }

    #[test]
    fn test_labeled_ascii_table_wide_state_name() {
        let mut lg = LabeledDirectedGraph::new_allow_self_loops();
        lg.add_edge("very_long_state_name", "short", "go").unwrap();
        lg.add_edge("short", "short", "stay").unwrap();
        let table = labeled_to_ascii_table(&lg);
        assert!(table.contains("very_long_state_name"));
    }

    #[test]
    fn test_format_dot_attrs_single() {
        let mut attrs = HashMap::new();
        attrs.insert("shape".to_string(), "circle".to_string());
        assert_eq!(format_dot_attrs(&attrs), "[shape=circle]");
    }

    #[test]
    fn test_format_dot_attrs_multiple_sorted() {
        let mut attrs = HashMap::new();
        attrs.insert("shape".to_string(), "circle".to_string());
        attrs.insert("color".to_string(), "red".to_string());
        assert_eq!(format_dot_attrs(&attrs), "[color=red, shape=circle]");
    }
}
