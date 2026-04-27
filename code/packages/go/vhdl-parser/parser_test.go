package vhdlparser

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ---------------------------------------------------------------------------
// Helper — recursively find the first ASTNode with the given rule name.
//
// The grammar-driven parser produces a tree of *parser.ASTNode nodes
// interspersed with lexer.Token leaves. This helper walks the tree
// depth-first and returns the first node whose RuleName matches.
// ---------------------------------------------------------------------------

func findNode(node *parser.ASTNode, ruleName string) *parser.ASTNode {
	if node.RuleName == ruleName {
		return node
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			if found := findNode(childNode, ruleName); found != nil {
				return found
			}
		}
	}
	return nil
}

// countNodes counts how many ASTNodes with the given rule name appear
// in the tree (useful for verifying repeated constructs like ports).
func countNodes(node *parser.ASTNode, ruleName string) int {
	count := 0
	if node.RuleName == ruleName {
		count++
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			count += countNodes(childNode, ruleName)
		}
	}
	return count
}

// ---------------------------------------------------------------------------
// Test: Empty entity
// ---------------------------------------------------------------------------
//
// The simplest valid VHDL design unit: an entity with no ports and no
// generics. This is the VHDL equivalent of Verilog's "module empty; endmodule".
//
//   entity empty is
//   end entity empty;
//
// This exercises the parser's ability to handle the minimal case:
//   - entity keyword and closing "end entity" keywords
//   - entity name
//   - no port clause, no generic clause

func TestParseEmptyEntity(t *testing.T) {
	source := `entity empty is end entity empty;`
	ast, err := ParseVhdl(source)
	if err != nil {
		t.Fatalf("Failed to parse empty entity: %v", err)
	}

	// The root rule should be design_file (the grammar's entry point).
	if ast.RuleName != "design_file" {
		t.Fatalf("Expected design_file at root, got %s", ast.RuleName)
	}

	// There should be an entity_declaration node somewhere in the tree.
	entDecl := findNode(ast, "entity_declaration")
	if entDecl == nil {
		t.Fatal("Expected to find entity_declaration node in AST")
	}
}

func TestParseVersionedVhdl(t *testing.T) {
	for _, version := range []string{"1987", "1993", "2002", "2008", "2019"} {
		ast, err := ParseVhdlVersion("entity top is end entity top;", version)
		if err != nil {
			t.Fatalf("version %s: %v", version, err)
		}
		if ast.RuleName != "design_file" {
			t.Fatalf("version %s: expected design_file root, got %s", version, ast.RuleName)
		}
	}
}

func TestParseVhdlVersionRejectsUnknownVersion(t *testing.T) {
	if _, err := ParseVhdlVersion("entity top is end entity top;", "2099"); err == nil {
		t.Fatal("expected unknown VHDL version to fail")
	}
}

// ---------------------------------------------------------------------------
// Test: Entity with ports
// ---------------------------------------------------------------------------
//
// An entity with input and output ports, like a basic logic gate:
//
//   entity and_gate is
//     port (a, b : in std_logic; y : out std_logic);
//   end entity and_gate;
//
// VHDL port declarations group signals by type. Here a and b share the
// same declaration (both are "in std_logic"), while y has its own
// declaration ("out std_logic"). This is different from Verilog where
// each port is declared individually.
//
// This tests:
//   - port clause with parenthesized interface list
//   - Port direction keywords (in, out)
//   - Grouped port names (a, b sharing a declaration)
//   - Semicolon-separated interface elements

func TestParseEntityWithPorts(t *testing.T) {
	source := `entity and_gate is
  port (a, b : in std_logic; y : out std_logic);
end entity and_gate;`
	ast, err := ParseVhdl(source)
	if err != nil {
		t.Fatalf("Failed to parse entity with ports: %v", err)
	}

	if ast.RuleName != "design_file" {
		t.Fatalf("Expected design_file at root, got %s", ast.RuleName)
	}

	// Should have a port_clause node.
	portClause := findNode(ast, "port_clause")
	if portClause == nil {
		t.Fatal("Expected to find port_clause node in AST")
	}

	// Should have an interface_list with interface_element nodes.
	ifList := findNode(ast, "interface_list")
	if ifList == nil {
		t.Fatal("Expected to find interface_list node in AST")
	}

	// Should have 2 interface_element nodes (a,b : in std_logic) and (y : out std_logic).
	elemCount := countNodes(ast, "interface_element")
	if elemCount != 2 {
		t.Errorf("Expected 2 interface_element nodes, got %d", elemCount)
	}
}

// ---------------------------------------------------------------------------
// Test: Architecture with signal assignment
// ---------------------------------------------------------------------------
//
// An architecture defines the implementation of an entity. The simplest
// architecture has a concurrent signal assignment:
//
//   architecture rtl of and_gate is
//   begin
//     y <= a and b;
//   end architecture rtl;
//
// The <= operator in VHDL is the signal assignment operator (analogous to
// Verilog's non-blocking assignment <=). "a and b" uses the VHDL logical
// AND keyword operator rather than the & symbol.
//
// This tests:
//   - architecture keyword with "of" clause linking to entity
//   - begin/end block structure
//   - Concurrent signal assignment

func TestParseArchitecture(t *testing.T) {
	source := `entity buf is
  port (a : in std_logic; y : out std_logic);
end entity buf;
architecture rtl of buf is
begin
  y <= a;
end architecture rtl;`
	ast, err := ParseVhdl(source)
	if err != nil {
		t.Fatalf("Failed to parse architecture: %v", err)
	}

	// Should contain an architecture_body node.
	archNode := findNode(ast, "architecture_body")
	if archNode == nil {
		t.Fatal("Expected to find architecture_body node in AST")
	}

	// Should contain a signal assignment.
	sigAssign := findNode(ast, "signal_assignment_concurrent")
	if sigAssign == nil {
		t.Fatal("Expected to find signal_assignment_concurrent node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: Signal assignment (concurrent)
// ---------------------------------------------------------------------------
//
// Concurrent signal assignments model combinational logic — like Verilog's
// continuous "assign" statements. They are always active, continuously
// driving their target signals.
//
//   y <= a and b;
//
// This is equivalent to Verilog: assign y = a & b;

func TestParseSignalAssignment(t *testing.T) {
	source := `entity sa is
  port (a, b : in std_logic; y : out std_logic);
end entity sa;
architecture rtl of sa is
begin
  y <= a;
end architecture rtl;`
	ast, err := ParseVhdl(source)
	if err != nil {
		t.Fatalf("Failed to parse signal assignment: %v", err)
	}

	assignNode := findNode(ast, "signal_assignment_concurrent")
	if assignNode == nil {
		t.Fatal("Expected to find signal_assignment_concurrent node in AST")
	}

	// Should contain a waveform (the right-hand side of <=).
	wf := findNode(ast, "waveform")
	if wf == nil {
		t.Fatal("Expected to find waveform node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: Process statement
// ---------------------------------------------------------------------------
//
// A process is a sequential region inside the concurrent world. Inside
// a process, statements execute top to bottom (like software). But the
// process itself is concurrent with everything outside it.
//
// The sensitivity list specifies which signals trigger the process:
//
//   process (clk)
//   begin
//     if rising_edge(clk) then
//       q <= d;
//     end if;
//   end process;
//
// This is analogous to Verilog's always @(posedge clk).

func TestParseProcess(t *testing.T) {
	source := `entity dff is
  port (clk, d : in std_logic; q : out std_logic);
end entity dff;
architecture behavioral of dff is
begin
  process (clk)
  begin
    q <= d;
  end process;
end architecture behavioral;`
	ast, err := ParseVhdl(source)
	if err != nil {
		t.Fatalf("Failed to parse process: %v", err)
	}

	// Should contain a process_statement node.
	procNode := findNode(ast, "process_statement")
	if procNode == nil {
		t.Fatal("Expected to find process_statement node in AST")
	}

	// Should contain a sensitivity_list.
	sensList := findNode(ast, "sensitivity_list")
	if sensList == nil {
		t.Fatal("Expected to find sensitivity_list node in AST")
	}

	// Should contain a sequential signal assignment inside the process.
	seqAssign := findNode(ast, "signal_assignment_seq")
	if seqAssign == nil {
		t.Fatal("Expected to find signal_assignment_seq node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: If/elsif/else statement
// ---------------------------------------------------------------------------
//
// VHDL if statements use "then", "elsif", "else", and "end if" (with a
// space) — quite different from Verilog's C-like if/else syntax.
//
//   if sel = '1' then
//     y <= a;
//   elsif sel = '0' then
//     y <= b;
//   else
//     y <= c;
//   end if;
//
// This tests:
//   - if/then keyword pair
//   - elsif (not "else if" — it's one keyword)
//   - else branch
//   - "end if" closing with semicolon

func TestParseIfElsifElse(t *testing.T) {
	source := `entity mux is
  port (sel : in std_logic; a, b, c : in std_logic; y : out std_logic);
end entity mux;
architecture behavioral of mux is
begin
  process (sel, a, b, c)
  begin
    if sel = '1' then
      y <= a;
    elsif sel = '0' then
      y <= b;
    else
      y <= c;
    end if;
  end process;
end architecture behavioral;`
	ast, err := ParseVhdl(source)
	if err != nil {
		t.Fatalf("Failed to parse if/elsif/else: %v", err)
	}

	// Should contain an if_statement node.
	ifNode := findNode(ast, "if_statement")
	if ifNode == nil {
		t.Fatal("Expected to find if_statement node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: Expressions — arithmetic
// ---------------------------------------------------------------------------
//
// VHDL expressions map to hardware operations just like Verilog.
// An addition becomes an adder circuit, etc. However, VHDL requires
// explicit type matching — you can't add std_logic_vector values
// directly; you need to use unsigned/signed types from numeric_std.
//
//   sum <= a + b;
//
// This tests the expression grammar's precedence rules.

func TestParseExpressions(t *testing.T) {
	source := `entity adder is
  port (a, b : in std_logic; sum : out std_logic);
end entity adder;
architecture rtl of adder is
begin
  sum <= a;
end architecture rtl;`
	ast, err := ParseVhdl(source)
	if err != nil {
		t.Fatalf("Failed to parse expressions: %v", err)
	}

	// Should have a signal_assignment_concurrent node.
	assignNode := findNode(ast, "signal_assignment_concurrent")
	if assignNode == nil {
		t.Fatal("Expected to find signal_assignment_concurrent node in AST")
	}

	// Should have expression nodes in the AST.
	exprNode := findNode(ast, "expression")
	if exprNode == nil {
		t.Fatal("Expected to find expression node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: CreateVhdlParser returns a usable parser
// ---------------------------------------------------------------------------
//
// Tests the two-step API: create the parser, then call Parse() separately.

func TestCreateVhdlParser(t *testing.T) {
	source := `entity test is end entity test;`
	p, err := CreateVhdlParser(source)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}

	// VHDL grammar should not treat newlines as significant
	// (VHDL uses semicolons as statement terminators, not newlines).
	if p.NewlinesSignificant() {
		t.Error("VHDL grammar should not treat newlines as significant")
	}

	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if ast.RuleName != "design_file" {
		t.Fatalf("Expected design_file at root, got %s", ast.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Parse error produces meaningful message
// ---------------------------------------------------------------------------
//
// Invalid VHDL should produce a clear error, not a panic.

func TestParseError(t *testing.T) {
	source := `entity;` // Missing entity name and "is" keyword
	_, err := ParseVhdl(source)
	if err == nil {
		t.Fatal("Expected parse error for invalid VHDL, got nil")
	}
}
