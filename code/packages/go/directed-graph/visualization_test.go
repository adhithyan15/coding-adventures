// visualization_test.go -- Tests for Graph Visualization Functions
// ================================================================
//
// We test all six visualization functions with both Graph and LabeledGraph
// inputs. Tests verify correct format structure, edge labels, options,
// determinism, and edge cases.

package directedgraph

import (
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// Helper: create a turnstile FSM
// ---------------------------------------------------------------------------

func newTurnstile() *LabeledGraph {
	lg := NewLabeledGraphAllowSelfLoops()
	lg.AddEdge("locked", "unlocked", "coin")
	lg.AddEdge("locked", "locked", "push")
	lg.AddEdge("unlocked", "locked", "push")
	lg.AddEdge("unlocked", "unlocked", "coin")
	return lg
}

func newSimpleDag() *Graph {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("A", "C")
	g.AddEdge("B", "D")
	g.AddEdge("C", "D")
	return g
}

// ===========================================================================
// ToDot -- Unlabeled Graph
// ===========================================================================

func TestToDotEmptyGraph(t *testing.T) {
	g := New()
	dot := ToDot(g, nil)
	if !strings.Contains(dot, "digraph G {") {
		t.Error("expected 'digraph G {' in output")
	}
	if !strings.Contains(dot, "rankdir=LR;") {
		t.Error("expected 'rankdir=LR;' in output")
	}
}

func TestToDotSingleNode(t *testing.T) {
	g := New()
	g.AddNode("A")
	dot := ToDot(g, nil)
	if !strings.Contains(dot, "    A;") {
		t.Error("expected node A in output")
	}
}

func TestToDotSingleEdge(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	dot := ToDot(g, nil)
	if !strings.Contains(dot, "    A -> B;") {
		t.Error("expected edge A -> B in output")
	}
}

func TestToDotDiamondDag(t *testing.T) {
	g := newSimpleDag()
	dot := ToDot(g, nil)
	for _, expected := range []string{"A -> B;", "A -> C;", "B -> D;", "C -> D;"} {
		if !strings.Contains(dot, expected) {
			t.Errorf("expected %q in output", expected)
		}
	}
}

func TestToDotCustomName(t *testing.T) {
	g := New()
	dot := ToDot(g, &DotOptions{Name: "MyGraph"})
	if !strings.Contains(dot, "digraph MyGraph {") {
		t.Error("expected custom name in output")
	}
}

func TestToDotTBRankdir(t *testing.T) {
	g := New()
	dot := ToDot(g, &DotOptions{Rankdir: "TB"})
	if !strings.Contains(dot, "rankdir=TB;") {
		t.Error("expected TB rankdir")
	}
}

func TestToDotNodeAttrs(t *testing.T) {
	g := New()
	g.AddNode("A")
	attrs := map[string]map[string]string{
		"A": {"shape": "circle"},
	}
	dot := ToDot(g, &DotOptions{NodeAttrs: attrs})
	if !strings.Contains(dot, "A [shape=circle];") {
		t.Error("expected node attributes")
	}
}

func TestToDotMultipleNodeAttrs(t *testing.T) {
	g := New()
	g.AddNode("A")
	attrs := map[string]map[string]string{
		"A": {"shape": "circle", "color": "red"},
	}
	dot := ToDot(g, &DotOptions{NodeAttrs: attrs})
	if !strings.Contains(dot, "A [color=red, shape=circle];") {
		t.Error("expected sorted node attributes")
	}
}

func TestToDotInitialState(t *testing.T) {
	g := New()
	g.AddNode("start")
	dot := ToDot(g, &DotOptions{Initial: "start"})
	if !strings.Contains(dot, `"" [shape=none];`) {
		t.Error("expected invisible start node")
	}
	if !strings.Contains(dot, `"" -> start;`) {
		t.Error("expected arrow from start node")
	}
}

func TestToDotIsolatedNodes(t *testing.T) {
	g := New()
	g.AddNode("X")
	g.AddNode("Y")
	dot := ToDot(g, nil)
	if !strings.Contains(dot, "    X;") || !strings.Contains(dot, "    Y;") {
		t.Error("expected isolated nodes in output")
	}
}

func TestToDotDeterministic(t *testing.T) {
	g := newSimpleDag()
	dot1 := ToDot(g, nil)
	dot2 := ToDot(g, nil)
	if dot1 != dot2 {
		t.Error("output should be deterministic")
	}
}

// ===========================================================================
// LabeledToDot
// ===========================================================================

func TestLabeledToDotEmpty(t *testing.T) {
	lg := NewLabeledGraph()
	dot := LabeledToDot(lg, nil)
	if !strings.Contains(dot, "digraph G {") {
		t.Error("expected digraph header")
	}
}

func TestLabeledToDotSingleEdge(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	dot := LabeledToDot(lg, nil)
	if !strings.Contains(dot, `A -> B [label="compile"];`) {
		t.Error("expected labeled edge")
	}
}

func TestLabeledToDotMultipleLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	dot := LabeledToDot(lg, nil)
	if !strings.Contains(dot, `A -> B [label="compile, test"];`) {
		t.Error("expected combined labels")
	}
}

func TestLabeledToDotTurnstile(t *testing.T) {
	lg := newTurnstile()
	dot := LabeledToDot(lg, nil)
	expected := []string{
		`locked -> locked [label="push"];`,
		`locked -> unlocked [label="coin"];`,
		`unlocked -> locked [label="push"];`,
		`unlocked -> unlocked [label="coin"];`,
	}
	for _, exp := range expected {
		if !strings.Contains(dot, exp) {
			t.Errorf("expected %q in output", exp)
		}
	}
}

func TestLabeledToDotInitialState(t *testing.T) {
	lg := newTurnstile()
	dot := LabeledToDot(lg, &DotOptions{Initial: "locked"})
	if !strings.Contains(dot, `"" -> locked;`) {
		t.Error("expected initial state arrow")
	}
}

func TestLabeledToDotNodeAttrs(t *testing.T) {
	lg := newTurnstile()
	attrs := map[string]map[string]string{
		"unlocked": {"shape": "doublecircle"},
	}
	dot := LabeledToDot(lg, &DotOptions{NodeAttrs: attrs})
	if !strings.Contains(dot, "unlocked [shape=doublecircle];") {
		t.Error("expected node attrs")
	}
}

func TestLabeledToDotCustomName(t *testing.T) {
	lg := NewLabeledGraph()
	dot := LabeledToDot(lg, &DotOptions{Name: "FSM"})
	if !strings.Contains(dot, "digraph FSM {") {
		t.Error("expected custom name")
	}
}

func TestLabeledToDotThreeLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "x")
	lg.AddEdge("A", "B", "y")
	lg.AddEdge("A", "B", "z")
	dot := LabeledToDot(lg, nil)
	if !strings.Contains(dot, `A -> B [label="x, y, z"];`) {
		t.Error("expected three combined labels")
	}
}

func TestLabeledToDotDeterministic(t *testing.T) {
	lg := newTurnstile()
	d1 := LabeledToDot(lg, nil)
	d2 := LabeledToDot(lg, nil)
	if d1 != d2 {
		t.Error("output should be deterministic")
	}
}

// ===========================================================================
// ToMermaid -- Unlabeled Graph
// ===========================================================================

func TestToMermaidEmpty(t *testing.T) {
	g := New()
	m := ToMermaid(g, "")
	if m != "graph LR" {
		t.Errorf("expected 'graph LR', got %q", m)
	}
}

func TestToMermaidSingleEdge(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	m := ToMermaid(g, "")
	if !strings.Contains(m, "A --> B") {
		t.Error("expected A --> B")
	}
}

func TestToMermaidDiamond(t *testing.T) {
	g := newSimpleDag()
	m := ToMermaid(g, "LR")
	for _, edge := range []string{"A --> B", "A --> C", "B --> D", "C --> D"} {
		if !strings.Contains(m, edge) {
			t.Errorf("expected %q in output", edge)
		}
	}
}

func TestToMermaidTDDirection(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	m := ToMermaid(g, "TD")
	if !strings.Contains(m, "graph TD") {
		t.Error("expected TD direction")
	}
}

func TestToMermaidDefaultDirection(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	m := ToMermaid(g, "")
	if !strings.Contains(m, "graph LR") {
		t.Error("expected default LR direction")
	}
}

func TestToMermaidChain(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	g.AddEdge("C", "D")
	m := ToMermaid(g, "")
	for _, edge := range []string{"A --> B", "B --> C", "C --> D"} {
		if !strings.Contains(m, edge) {
			t.Errorf("expected %q in output", edge)
		}
	}
}

func TestToMermaidDeterministic(t *testing.T) {
	g := newSimpleDag()
	m1 := ToMermaid(g, "LR")
	m2 := ToMermaid(g, "LR")
	if m1 != m2 {
		t.Error("output should be deterministic")
	}
}

// ===========================================================================
// LabeledToMermaid
// ===========================================================================

func TestLabeledToMermaidEmpty(t *testing.T) {
	lg := NewLabeledGraph()
	m := LabeledToMermaid(lg, "")
	if m != "graph LR" {
		t.Errorf("expected 'graph LR', got %q", m)
	}
}

func TestLabeledToMermaidSingleEdge(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	m := LabeledToMermaid(lg, "LR")
	if !strings.Contains(m, "A -->|compile| B") {
		t.Error("expected labeled mermaid edge")
	}
}

func TestLabeledToMermaidMultipleLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	m := LabeledToMermaid(lg, "")
	if !strings.Contains(m, "A -->|compile, test| B") {
		t.Error("expected combined labels")
	}
}

func TestLabeledToMermaidTurnstile(t *testing.T) {
	lg := newTurnstile()
	m := LabeledToMermaid(lg, "LR")
	expected := []string{
		"locked -->|coin| unlocked",
		"locked -->|push| locked",
		"unlocked -->|coin| unlocked",
		"unlocked -->|push| locked",
	}
	for _, exp := range expected {
		if !strings.Contains(m, exp) {
			t.Errorf("expected %q in output", exp)
		}
	}
}

func TestLabeledToMermaidTDDirection(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "dep")
	m := LabeledToMermaid(lg, "TD")
	if !strings.Contains(m, "graph TD") {
		t.Error("expected TD direction")
	}
}

func TestLabeledToMermaidThreeLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "x")
	lg.AddEdge("A", "B", "y")
	lg.AddEdge("A", "B", "z")
	m := LabeledToMermaid(lg, "")
	if !strings.Contains(m, "A -->|x, y, z| B") {
		t.Error("expected three labels combined")
	}
}

func TestLabeledToMermaidDeterministic(t *testing.T) {
	lg := newTurnstile()
	m1 := LabeledToMermaid(lg, "LR")
	m2 := LabeledToMermaid(lg, "LR")
	if m1 != m2 {
		t.Error("output should be deterministic")
	}
}

// ===========================================================================
// ToAsciiTable -- Unlabeled Graph
// ===========================================================================

func TestToAsciiTableEmpty(t *testing.T) {
	g := New()
	table := ToAsciiTable(g)
	if !strings.Contains(table, "Node") || !strings.Contains(table, "Successors") {
		t.Error("expected header columns")
	}
}

func TestToAsciiTableSingleNode(t *testing.T) {
	g := New()
	g.AddNode("A")
	table := ToAsciiTable(g)
	if !strings.Contains(table, "A") || !strings.Contains(table, "-") {
		t.Error("expected node with dash")
	}
}

func TestToAsciiTableSingleEdge(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	table := ToAsciiTable(g)
	if !strings.Contains(table, "A") || !strings.Contains(table, "B") {
		t.Error("expected both nodes")
	}
}

func TestToAsciiTableDiamond(t *testing.T) {
	g := newSimpleDag()
	table := ToAsciiTable(g)
	if !strings.Contains(table, "B, C") {
		t.Error("expected A's successors")
	}
	lines := strings.Split(table, "\n")
	for _, line := range lines {
		if strings.HasPrefix(line, "D") {
			if !strings.Contains(line, "-") {
				t.Error("D should have no successors")
			}
		}
	}
}

func TestToAsciiTableHeaderSeparator(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	table := ToAsciiTable(g)
	lines := strings.Split(table, "\n")
	if !strings.Contains(lines[1], "-+-") {
		t.Error("expected separator")
	}
}

func TestToAsciiTableColumnAlignment(t *testing.T) {
	g := New()
	g.AddEdge("short", "very_long_name")
	table := ToAsciiTable(g)
	if !strings.Contains(table, "very_long_name") {
		t.Error("expected long name")
	}
}

func TestToAsciiTableHubNode(t *testing.T) {
	g := New()
	g.AddEdge("hub", "A")
	g.AddEdge("hub", "B")
	g.AddEdge("hub", "C")
	g.AddEdge("hub", "D")
	table := ToAsciiTable(g)
	if !strings.Contains(table, "A, B, C, D") {
		t.Error("expected hub successors")
	}
}

func TestToAsciiTableDeterministic(t *testing.T) {
	g := newSimpleDag()
	t1 := ToAsciiTable(g)
	t2 := ToAsciiTable(g)
	if t1 != t2 {
		t.Error("output should be deterministic")
	}
}

// ===========================================================================
// LabeledToAsciiTable
// ===========================================================================

func TestLabeledToAsciiTableEmpty(t *testing.T) {
	lg := NewLabeledGraph()
	table := LabeledToAsciiTable(lg)
	if !strings.Contains(table, "State") {
		t.Error("expected State header")
	}
}

func TestLabeledToAsciiTableSingleEdge(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "dep")
	table := LabeledToAsciiTable(lg)
	if !strings.Contains(table, "dep") || !strings.Contains(table, "B") {
		t.Error("expected label and destination")
	}
}

func TestLabeledToAsciiTableTurnstile(t *testing.T) {
	lg := newTurnstile()
	table := LabeledToAsciiTable(lg)
	if !strings.Contains(table, "coin") || !strings.Contains(table, "push") {
		t.Error("expected label columns")
	}
	lines := strings.Split(table, "\n")
	found := false
	for _, line := range lines {
		if strings.HasPrefix(line, "locked ") {
			found = true
			if !strings.Contains(line, "unlocked") {
				t.Error("locked row should show unlocked transition")
			}
		}
	}
	if !found {
		t.Error("expected locked row")
	}
}

func TestLabeledToAsciiTableSeparator(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "dep")
	table := LabeledToAsciiTable(lg)
	lines := strings.Split(table, "\n")
	if !strings.Contains(lines[1], "-+-") {
		t.Error("expected separator")
	}
}

func TestLabeledToAsciiTableMissingTransition(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "x")
	lg.AddNode("C")
	table := LabeledToAsciiTable(lg)
	lines := strings.Split(table, "\n")
	for _, line := range lines {
		if strings.HasPrefix(line, "C") {
			if !strings.Contains(line, "-") {
				t.Error("C should have dash for missing transition")
			}
		}
	}
}

func TestLabeledToAsciiTableNodesWithoutEdges(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddNode("A")
	lg.AddNode("B")
	table := LabeledToAsciiTable(lg)
	if !strings.Contains(table, "A") || !strings.Contains(table, "B") {
		t.Error("expected both nodes")
	}
}

func TestLabeledToAsciiTableMultipleLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "x")
	lg.AddEdge("A", "C", "y")
	table := LabeledToAsciiTable(lg)
	if !strings.Contains(table, "x") || !strings.Contains(table, "y") {
		t.Error("expected both label columns")
	}
}

func TestLabeledToAsciiTableWideName(t *testing.T) {
	lg := NewLabeledGraphAllowSelfLoops()
	lg.AddEdge("very_long_state_name", "short", "go")
	lg.AddEdge("short", "short", "stay")
	table := LabeledToAsciiTable(lg)
	if !strings.Contains(table, "very_long_state_name") {
		t.Error("expected long state name")
	}
}

func TestLabeledToAsciiTableDeterministic(t *testing.T) {
	lg := newTurnstile()
	t1 := LabeledToAsciiTable(lg)
	t2 := LabeledToAsciiTable(lg)
	if t1 != t2 {
		t.Error("output should be deterministic")
	}
}

// ===========================================================================
// formatDotAttrs tests
// ===========================================================================

func TestFormatDotAttrsSingle(t *testing.T) {
	attrs := map[string]string{"shape": "circle"}
	result := formatDotAttrs(attrs)
	if result != "[shape=circle]" {
		t.Errorf("expected [shape=circle], got %q", result)
	}
}

func TestFormatDotAttrsMultipleSorted(t *testing.T) {
	attrs := map[string]string{"shape": "circle", "color": "red"}
	result := formatDotAttrs(attrs)
	if result != "[color=red, shape=circle]" {
		t.Errorf("expected sorted attrs, got %q", result)
	}
}

// ===========================================================================
// Full output verification
// ===========================================================================

func TestToDotFullTurnstile(t *testing.T) {
	lg := newTurnstile()
	attrs := map[string]map[string]string{
		"locked":   {"shape": "circle"},
		"unlocked": {"shape": "doublecircle"},
	}
	dot := LabeledToDot(lg, &DotOptions{
		Name:      "Turnstile",
		Rankdir:   "LR",
		Initial:   "locked",
		NodeAttrs: attrs,
	})
	if !strings.Contains(dot, "digraph Turnstile {") {
		t.Error("expected graph name")
	}
	if !strings.Contains(dot, `"" [shape=none];`) {
		t.Error("expected invisible start node")
	}
	if !strings.Contains(dot, `"" -> locked;`) {
		t.Error("expected initial arrow")
	}
	if !strings.Contains(dot, "locked [shape=circle];") {
		t.Error("expected locked attrs")
	}
	if !strings.Contains(dot, "unlocked [shape=doublecircle];") {
		t.Error("expected unlocked attrs")
	}
}
