// visualization.go -- Graph Visualization in Multiple Formats
// ===========================================================
//
// This file converts directed graphs into human-readable text formats.
// It supports three output formats, each serving a different purpose:
//
// 1. DOT format (Graphviz) -- the industry standard for graph visualization.
//    Paste the output into https://dreampuf.github.io/GraphvizOnline/
//    or pipe it to `dot -Tpng` to get a rendered image.
//
// 2. Mermaid format -- a lightweight alternative that renders directly
//    in GitHub Markdown, Notion, and many other tools.
//
// 3. ASCII table -- a plain-text representation for terminal output.
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
// Since Go doesn't have method overloading or union types, we use
// separate functions for Graph and LabeledGraph:
//
//   - ToDot / LabeledToDot
//   - ToMermaid / LabeledToMermaid
//   - ToAsciiTable / LabeledToAsciiTable
//
// This is idiomatic Go -- explicit is better than implicit.

package directedgraph

import (
	"fmt"
	"sort"
	"strings"
)

// ═══════════════════════════════════════════════════════════════════════
// DotOptions controls DOT output rendering.
// ═══════════════════════════════════════════════════════════════════════
//
// - Name: The graph name (appears as `digraph <name> { ... }`).
//   Defaults to "G" if empty.
//
// - NodeAttrs: Per-node DOT attributes. The outer map is keyed by node
//   name. The inner map is keyed by attribute name (e.g., "shape") with
//   values (e.g., "doublecircle").
//
// - Initial: If non-empty, adds an invisible start node with an arrow
//   to this node. This is the standard way to mark the initial state
//   in FSM diagrams.
//
// - Rankdir: Layout direction. "LR" means left-to-right, "TB" means
//   top-to-bottom. Defaults to "LR" if empty.

type DotOptions struct {
	Name      string
	NodeAttrs map[string]map[string]string
	Initial   string
	Rankdir   string
}

// defaultDotOptions fills in zero-value fields with sensible defaults.
func defaultDotOptions(opts *DotOptions) DotOptions {
	result := DotOptions{
		Name:      "G",
		Rankdir:   "LR",
		NodeAttrs: nil,
		Initial:   "",
	}
	if opts != nil {
		if opts.Name != "" {
			result.Name = opts.Name
		}
		if opts.Rankdir != "" {
			result.Rankdir = opts.Rankdir
		}
		if opts.NodeAttrs != nil {
			result.NodeAttrs = opts.NodeAttrs
		}
		result.Initial = opts.Initial
	}
	return result
}

// formatDotAttrs converts a map of DOT attributes to the bracketed format.
//
// Example: {"shape": "circle", "color": "red"} → `[color=red, shape=circle]`
//
// Attributes are sorted by key for deterministic output.
func formatDotAttrs(attrs map[string]string) string {
	keys := make([]string, 0, len(attrs))
	for k := range attrs {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	parts := make([]string, 0, len(keys))
	for _, k := range keys {
		parts = append(parts, fmt.Sprintf("%s=%s", k, attrs[k]))
	}
	return "[" + strings.Join(parts, ", ") + "]"
}

// ═══════════════════════════════════════════════════════════════════════
// ToDot -- Graphviz DOT format for unlabeled graphs
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
// still appear in the output. Edges are listed after nodes.

// ToDot converts an unlabeled Graph to Graphviz DOT format.
func ToDot(g *Graph, opts *DotOptions) string {
	o := defaultDotOptions(opts)
	var sb strings.Builder

	sb.WriteString(fmt.Sprintf("digraph %s {\n", o.Name))
	sb.WriteString(fmt.Sprintf("    rankdir=%s;\n", o.Rankdir))

	// Initial state marker: invisible node with arrow.
	if o.Initial != "" {
		sb.WriteString("    \"\" [shape=none];\n")
		sb.WriteString(fmt.Sprintf("    \"\" -> %s;\n", o.Initial))
	}

	// Node declarations (sorted for deterministic output).
	nodes := g.Nodes()
	for _, node := range nodes {
		if o.NodeAttrs != nil {
			if attrs, ok := o.NodeAttrs[node]; ok {
				sb.WriteString(fmt.Sprintf("    %s %s;\n", node, formatDotAttrs(attrs)))
				continue
			}
		}
		sb.WriteString(fmt.Sprintf("    %s;\n", node))
	}

	// Edge declarations (sorted for deterministic output).
	edges := g.Edges()
	for _, edge := range edges {
		sb.WriteString(fmt.Sprintf("    %s -> %s;\n", edge[0], edge[1]))
	}

	sb.WriteString("}")
	return sb.String()
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledToDot -- Graphviz DOT format for labeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// For labeled graphs, edges get [label="..."] attributes. If multiple
// labels exist on the same (from, to) pair, they are combined as
// "a, b" in a single label attribute.

// LabeledToDot converts a LabeledGraph to Graphviz DOT format.
func LabeledToDot(lg *LabeledGraph, opts *DotOptions) string {
	o := defaultDotOptions(opts)
	var sb strings.Builder

	sb.WriteString(fmt.Sprintf("digraph %s {\n", o.Name))
	sb.WriteString(fmt.Sprintf("    rankdir=%s;\n", o.Rankdir))

	// Initial state marker.
	if o.Initial != "" {
		sb.WriteString("    \"\" [shape=none];\n")
		sb.WriteString(fmt.Sprintf("    \"\" -> %s;\n", o.Initial))
	}

	// Node declarations.
	nodes := lg.Nodes()
	for _, node := range nodes {
		if o.NodeAttrs != nil {
			if attrs, ok := o.NodeAttrs[node]; ok {
				sb.WriteString(fmt.Sprintf("    %s %s;\n", node, formatDotAttrs(attrs)))
				continue
			}
		}
		sb.WriteString(fmt.Sprintf("    %s;\n", node))
	}

	// Build combined edge labels.
	// We need to group edges by (from, to) and combine their labels.
	edgeLabels := collectLabeledEdgeLabels(lg)

	// Get unique structural edges in sorted order.
	structuralEdges := uniqueStructuralEdges(lg)

	for _, edge := range structuralEdges {
		key := [2]string{edge[0], edge[1]}
		if label, ok := edgeLabels[key]; ok {
			sb.WriteString(fmt.Sprintf("    %s -> %s [label=\"%s\"];\n", edge[0], edge[1], label))
		} else {
			sb.WriteString(fmt.Sprintf("    %s -> %s;\n", edge[0], edge[1]))
		}
	}

	sb.WriteString("}")
	return sb.String()
}

// collectLabeledEdgeLabels groups edge labels by (from, to) pair.
//
// For each unique (from, to) pair in the labeled graph, we collect all
// labels and join them with ", " (sorted alphabetically for determinism).
//
// Example: if edge A→B has labels "compile" and "test", the result
// contains key [2]string{"A","B"} with value "compile, test".
func collectLabeledEdgeLabels(lg *LabeledGraph) map[[2]string]string {
	result := make(map[[2]string]string)
	edges := lg.Edges() // [][3]string sorted

	// Group labels by (from, to).
	grouped := make(map[[2]string][]string)
	for _, edge := range edges {
		key := [2]string{edge[0], edge[1]}
		grouped[key] = append(grouped[key], edge[2])
	}

	// Sort and join labels.
	for key, labels := range grouped {
		sort.Strings(labels)
		result[key] = strings.Join(labels, ", ")
	}

	return result
}

// uniqueStructuralEdges extracts the unique (from, to) pairs from a
// labeled graph's edges, sorted deterministically.
func uniqueStructuralEdges(lg *LabeledGraph) [][2]string {
	seen := make(map[[2]string]bool)
	var result [][2]string

	for _, edge := range lg.Edges() {
		key := [2]string{edge[0], edge[1]}
		if !seen[key] {
			seen[key] = true
			result = append(result, key)
		}
	}

	sort.Slice(result, func(i, j int) bool {
		if result[i][0] != result[j][0] {
			return result[i][0] < result[j][0]
		}
		return result[i][1] < result[j][1]
	})

	return result
}

// ═══════════════════════════════════════════════════════════════════════
// ToMermaid -- Mermaid flowchart format for unlabeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// Mermaid is a JavaScript-based diagramming tool that renders directly
// in Markdown. The syntax for a flowchart is:
//
//   graph LR
//       A --> B
//       B --> C
//
// Mermaid is popular because GitHub, GitLab, and many documentation tools
// render it natively -- no external tools needed.

// ToMermaid converts an unlabeled Graph to Mermaid flowchart format.
//
// The direction parameter controls flow direction: "LR" (left-to-right)
// or "TD" (top-down). Defaults to "LR" if empty.
func ToMermaid(g *Graph, direction string) string {
	if direction == "" {
		direction = "LR"
	}

	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("graph %s\n", direction))

	edges := g.Edges()
	for _, edge := range edges {
		sb.WriteString(fmt.Sprintf("    %s --> %s\n", edge[0], edge[1]))
	}

	return strings.TrimRight(sb.String(), "\n")
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledToMermaid -- Mermaid flowchart format for labeled graphs
// ═══════════════════════════════════════════════════════════════════════
//
// For labeled edges, Mermaid uses the -->|label| syntax:
//
//   A -->|coin| B
//
// If multiple labels exist on the same edge, we combine them:
//
//   A -->|coin, push| B

// LabeledToMermaid converts a LabeledGraph to Mermaid flowchart format.
//
// The direction parameter controls flow direction: "LR" (left-to-right)
// or "TD" (top-down). Defaults to "LR" if empty.
func LabeledToMermaid(lg *LabeledGraph, direction string) string {
	if direction == "" {
		direction = "LR"
	}

	edgeLabels := collectLabeledEdgeLabels(lg)
	structuralEdges := uniqueStructuralEdges(lg)

	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("graph %s\n", direction))

	for _, edge := range structuralEdges {
		key := [2]string{edge[0], edge[1]}
		if label, ok := edgeLabels[key]; ok {
			sb.WriteString(fmt.Sprintf("    %s -->|%s| %s\n", edge[0], label, edge[1]))
		} else {
			sb.WriteString(fmt.Sprintf("    %s --> %s\n", edge[0], edge[1]))
		}
	}

	return strings.TrimRight(sb.String(), "\n")
}

// ═══════════════════════════════════════════════════════════════════════
// ToAsciiTable -- Plain text adjacency list for unlabeled graphs
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
//
// The dash "-" indicates no successors. This is simple and readable.

// ToAsciiTable converts an unlabeled Graph to a plain-text adjacency table.
func ToAsciiTable(g *Graph) string {
	nodes := g.Nodes()

	// Build successor strings for each node.
	succStrings := make(map[string]string, len(nodes))
	for _, node := range nodes {
		succs, _ := g.Successors(node)
		sort.Strings(succs)
		if len(succs) > 0 {
			succStrings[node] = strings.Join(succs, ", ")
		} else {
			succStrings[node] = "-"
		}
	}

	// Calculate column widths.
	nodeColWidth := len("Node")
	for _, node := range nodes {
		if len(node) > nodeColWidth {
			nodeColWidth = len(node)
		}
	}

	succColWidth := len("Successors")
	for _, s := range succStrings {
		if len(s) > succColWidth {
			succColWidth = len(s)
		}
	}

	// Build the table.
	var sb strings.Builder

	// Header.
	sb.WriteString(fmt.Sprintf("%-*s | %-*s\n", nodeColWidth, "Node", succColWidth, "Successors"))
	// Separator.
	sb.WriteString(fmt.Sprintf("%s-+-%s\n", strings.Repeat("-", nodeColWidth), strings.Repeat("-", succColWidth)))
	// Data rows.
	for _, node := range nodes {
		sb.WriteString(fmt.Sprintf("%-*s | %-*s\n", nodeColWidth, node, succColWidth, succStrings[node]))
	}

	return strings.TrimRight(sb.String(), "\n")
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledToAsciiTable -- Transition table for labeled graphs
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
//
// This is the standard representation of a finite state machine.
// A "-" in a cell means no transition exists for that (state, label) pair.

// LabeledToAsciiTable converts a LabeledGraph to a plain-text transition table.
func LabeledToAsciiTable(lg *LabeledGraph) string {
	nodes := lg.Nodes()
	edges := lg.Edges() // [][3]string sorted

	// Step 1: Collect all unique labels.
	labelSet := make(map[string]bool)
	for _, edge := range edges {
		labelSet[edge[2]] = true
	}
	labels := make([]string, 0, len(labelSet))
	for l := range labelSet {
		labels = append(labels, l)
	}
	sort.Strings(labels)

	// Handle edge case: no labels.
	if len(labels) == 0 {
		stateColWidth := len("State")
		for _, node := range nodes {
			if len(node) > stateColWidth {
				stateColWidth = len(node)
			}
		}
		var sb strings.Builder
		sb.WriteString(fmt.Sprintf("%-*s\n", stateColWidth, "State"))
		sb.WriteString(fmt.Sprintf("%s\n", strings.Repeat("-", stateColWidth)))
		for _, node := range nodes {
			sb.WriteString(fmt.Sprintf("%-*s\n", stateColWidth, node))
		}
		return strings.TrimRight(sb.String(), "\n")
	}

	// Step 2: Build transition map.
	// transitions[node][label] = sorted list of destination nodes.
	type nodeLabel struct {
		node, label string
	}
	transitions := make(map[nodeLabel][]string)

	for _, edge := range edges {
		key := nodeLabel{edge[0], edge[2]}
		dests := transitions[key]
		// Only add if not already present.
		found := false
		for _, d := range dests {
			if d == edge[1] {
				found = true
				break
			}
		}
		if !found {
			dests = append(dests, edge[1])
			sort.Strings(dests)
			transitions[key] = dests
		}
	}

	// Step 3: Calculate column widths.
	stateColWidth := len("State")
	for _, node := range nodes {
		if len(node) > stateColWidth {
			stateColWidth = len(node)
		}
	}

	labelColWidths := make([]int, len(labels))
	for i, label := range labels {
		labelColWidths[i] = len(label)
		for _, node := range nodes {
			key := nodeLabel{node, label}
			dests := transitions[key]
			cellText := "-"
			if len(dests) > 0 {
				cellText = strings.Join(dests, ", ")
			}
			if len(cellText) > labelColWidths[i] {
				labelColWidths[i] = len(cellText)
			}
		}
	}

	// Step 4: Build the formatted table.
	var sb strings.Builder

	// Header row.
	sb.WriteString(fmt.Sprintf("%-*s", stateColWidth, "State"))
	for i, label := range labels {
		sb.WriteString(fmt.Sprintf(" | %-*s", labelColWidths[i], label))
	}
	sb.WriteString("\n")

	// Separator row.
	sb.WriteString(strings.Repeat("-", stateColWidth))
	for i := range labels {
		sb.WriteString("-+-")
		sb.WriteString(strings.Repeat("-", labelColWidths[i]))
	}
	sb.WriteString("\n")

	// Data rows.
	for _, node := range nodes {
		sb.WriteString(fmt.Sprintf("%-*s", stateColWidth, node))
		for i, label := range labels {
			key := nodeLabel{node, label}
			dests := transitions[key]
			cellText := "-"
			if len(dests) > 0 {
				cellText = strings.Join(dests, ", ")
			}
			sb.WriteString(fmt.Sprintf(" | %-*s", labelColWidths[i], cellText))
		}
		sb.WriteString("\n")
	}

	return strings.TrimRight(sb.String(), "\n")
}
