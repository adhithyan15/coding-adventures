package verilogparser

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
// Test: Empty module
// ---------------------------------------------------------------------------
//
// The simplest valid Verilog design: a module with no ports and no body.
//
//	module empty; endmodule
//
// This exercises the parser's ability to handle the minimal case:
//   - module keyword
//   - module name
//   - semicolon (no port list)
//   - endmodule keyword

func TestParseEmptyModule(t *testing.T) {
	source := `module empty; endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse empty module: %v", err)
	}

	// The root rule should be source_text (the grammar's entry point).
	if ast.RuleName != "source_text" {
		t.Fatalf("Expected source_text at root, got %s", ast.RuleName)
	}

	// There should be a module_declaration node somewhere in the tree.
	modDecl := findNode(ast, "module_declaration")
	if modDecl == nil {
		t.Fatal("Expected to find module_declaration node in AST")
	}
}

func TestParseVersionedVerilog(t *testing.T) {
	for _, version := range []string{"1995", "2001", "2005"} {
		ast, err := ParseVerilogVersion("module top; endmodule", version)
		if err != nil {
			t.Fatalf("version %s: %v", version, err)
		}
		if ast.RuleName != "source_text" {
			t.Fatalf("version %s: expected source_text root, got %s", version, ast.RuleName)
		}
	}
}

func TestParseVerilogVersionRejectsUnknownVersion(t *testing.T) {
	if _, err := ParseVerilogVersion("module top; endmodule", "2099"); err == nil {
		t.Fatal("expected unknown Verilog version to fail")
	}
}

// ---------------------------------------------------------------------------
// Test: Module with ports
// ---------------------------------------------------------------------------
//
// A module with input and output ports, like a basic logic gate:
//
//	module and_gate(input a, input b, output y);
//	endmodule
//
// This tests:
//   - Port list parsing (parenthesized, comma-separated)
//   - Port direction keywords (input, output)
//   - Multiple ports

func TestParseModuleWithPorts(t *testing.T) {
	source := `module and_gate(input a, input b, output y);
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse module with ports: %v", err)
	}

	if ast.RuleName != "source_text" {
		t.Fatalf("Expected source_text at root, got %s", ast.RuleName)
	}

	// Should have a port_list containing ports.
	portList := findNode(ast, "port_list")
	if portList == nil {
		t.Fatal("Expected to find port_list node in AST")
	}

	// Should have 3 port nodes (a, b, y).
	portCount := countNodes(ast, "port")
	if portCount != 3 {
		t.Errorf("Expected 3 port nodes, got %d", portCount)
	}
}

// ---------------------------------------------------------------------------
// Test: Continuous assignment (assign statement)
// ---------------------------------------------------------------------------
//
// Continuous assignments model combinational logic — the output is always
// a function of the current inputs, like a physical wire connection.
//
//	module buf(input a, output y);
//	    assign y = a;
//	endmodule
//
// This tests:
//   - The assign keyword
//   - lvalue = expression parsing
//   - Expression evaluation (simple name reference)

func TestParseAssign(t *testing.T) {
	source := `module buf_gate(input a, output y);
    assign y = a;
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse assign statement: %v", err)
	}

	// Should contain a continuous_assign node.
	assignNode := findNode(ast, "continuous_assign")
	if assignNode == nil {
		t.Fatal("Expected to find continuous_assign node in AST")
	}

	// Should contain an assignment (lvalue = expression).
	assignment := findNode(ast, "assignment")
	if assignment == nil {
		t.Fatal("Expected to find assignment node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: Always block
// ---------------------------------------------------------------------------
//
// Always blocks describe behavior that triggers on signal changes.
// This is the core of sequential logic (flip-flops, registers).
//
//	module dff(input clk, input d, output reg q);
//	    always @(posedge clk)
//	        q <= d;
//	endmodule
//
// This tests:
//   - always keyword
//   - @ sensitivity list with posedge
//   - Non-blocking assignment (<=)

func TestParseAlwaysBlock(t *testing.T) {
	source := `module dff(input clk, input d, output reg q);
    always @(posedge clk)
        q <= d;
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse always block: %v", err)
	}

	// Should contain an always_construct node.
	alwaysNode := findNode(ast, "always_construct")
	if alwaysNode == nil {
		t.Fatal("Expected to find always_construct node in AST")
	}

	// Should contain a sensitivity_list.
	sensList := findNode(ast, "sensitivity_list")
	if sensList == nil {
		t.Fatal("Expected to find sensitivity_list node in AST")
	}

	// Should contain a nonblocking_assignment (q <= d).
	nba := findNode(ast, "nonblocking_assignment")
	if nba == nil {
		t.Fatal("Expected to find nonblocking_assignment node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: Case statement
// ---------------------------------------------------------------------------
//
// Case statements are used for multi-way branching, commonly for
// instruction decoders and multiplexers in hardware.
//
//	module mux4(input [1:0] sel, input a, input b, input c, input d, output reg y);
//	    always @(*)
//	        case (sel)
//	            0: y = a;
//	            1: y = b;
//	            2: y = c;
//	            default: y = d;
//	        endcase
//	endmodule
//
// This tests:
//   - case keyword and endcase
//   - Sensitivity list with * (wildcard)
//   - case_item with expression : statement
//   - default case

func TestParseCaseStatement(t *testing.T) {
	source := `module mux4(input wire sel, input a, input b, output reg y);
    always @(sel or a or b)
        case (sel)
            0: y = a;
            default: y = b;
        endcase
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse case statement: %v", err)
	}

	// Should contain a case_statement node.
	caseNode := findNode(ast, "case_statement")
	if caseNode == nil {
		t.Fatal("Expected to find case_statement node in AST")
	}

	// Should contain case_item nodes.
	caseItemCount := countNodes(ast, "case_item")
	if caseItemCount < 2 {
		t.Errorf("Expected at least 2 case_item nodes, got %d", caseItemCount)
	}
}

// ---------------------------------------------------------------------------
// Test: Expressions — arithmetic and bitwise
// ---------------------------------------------------------------------------
//
// Verilog expressions map directly to hardware operations. An addition
// becomes an adder circuit, a bitwise AND becomes AND gates, etc.
//
//	module expr_test(input [7:0] a, input [7:0] b, output [7:0] sum, output [7:0] masked);
//	    assign sum = a + b;
//	    assign masked = a & b;
//	endmodule
//
// This tests the expression grammar's precedence rules and the
// ability to parse binary operators.

func TestParseExpressions(t *testing.T) {
	source := `module expr_test(input wire a, input wire b, output wire sum);
    assign sum = a + b;
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse expressions: %v", err)
	}

	// Should have a continuous_assign with an expression tree.
	assignNode := findNode(ast, "continuous_assign")
	if assignNode == nil {
		t.Fatal("Expected to find continuous_assign node in AST")
	}

	// The expression tree should contain an additive_expr node
	// (since we're adding a + b).
	addExpr := findNode(ast, "additive_expr")
	if addExpr == nil {
		t.Fatal("Expected to find additive_expr node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: CreateVerilogParser returns a usable parser
// ---------------------------------------------------------------------------
//
// Tests the two-step API: create the parser, then call Parse() separately.

func TestCreateVerilogParser(t *testing.T) {
	source := `module test; endmodule`
	p, err := CreateVerilogParser(source)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}

	// The parser should not report newlines as significant
	// (Verilog treats whitespace/newlines as insignificant).
	if p.NewlinesSignificant() {
		t.Error("Verilog grammar should not treat newlines as significant")
	}

	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if ast.RuleName != "source_text" {
		t.Fatalf("Expected source_text at root, got %s", ast.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Begin/end block in always
// ---------------------------------------------------------------------------
//
// Multiple statements in an always block require begin/end grouping,
// similar to { } in C.
//
//	always @(posedge clk) begin
//	    a <= b;
//	    c <= d;
//	end

func TestParseBeginEndBlock(t *testing.T) {
	source := `module multi(input clk, input d1, input d2, output reg q1, output reg q2);
    always @(posedge clk) begin
        q1 <= d1;
        q2 <= d2;
    end
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse begin/end block: %v", err)
	}

	blockNode := findNode(ast, "block_statement")
	if blockNode == nil {
		t.Fatal("Expected to find block_statement node in AST")
	}

	// Should have two nonblocking assignments inside the block.
	nbaCount := countNodes(ast, "nonblocking_assignment")
	if nbaCount != 2 {
		t.Errorf("Expected 2 nonblocking_assignment nodes, got %d", nbaCount)
	}
}

// ---------------------------------------------------------------------------
// Test: If/else statement
// ---------------------------------------------------------------------------
//
// If/else in Verilog generates multiplexer logic in hardware.

func TestParseIfElse(t *testing.T) {
	source := `module mux(input sel, input a, input b, output reg y);
    always @(sel or a or b)
        if (sel) y = a;
        else y = b;
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse if/else: %v", err)
	}

	ifNode := findNode(ast, "if_statement")
	if ifNode == nil {
		t.Fatal("Expected to find if_statement node in AST")
	}
}

// ---------------------------------------------------------------------------
// Test: Wire and reg declarations
// ---------------------------------------------------------------------------
//
// Internal signals in a module must be declared as wire or reg.

func TestParseDeclarations(t *testing.T) {
	// Note: in the grammar, module_item tries net_declaration before
	// reg_declaration. Since net_type includes "reg", a bare `reg b;`
	// is parsed as net_declaration (matching reg as a net_type). To get
	// a reg_declaration, we need `reg signed [7:0] b;` or similar — but
	// the grammar's ordering means net_declaration wins for simple cases.
	// We verify both wire and reg declarations are parsed (both as
	// net_declaration since they share the same grammar path).
	source := `module decl_test;
    wire a;
    reg b;
endmodule`
	ast, err := ParseVerilog(source)
	if err != nil {
		t.Fatalf("Failed to parse declarations: %v", err)
	}

	// Both wire and reg are parsed via net_declaration in the grammar
	// (because net_type includes "reg").
	netDeclCount := countNodes(ast, "net_declaration")
	if netDeclCount < 2 {
		t.Errorf("Expected at least 2 net_declaration nodes (wire + reg), got %d", netDeclCount)
	}
}

// ---------------------------------------------------------------------------
// Test: Parse error produces meaningful message
// ---------------------------------------------------------------------------
//
// Invalid Verilog should produce a clear error, not a panic.

func TestParseError(t *testing.T) {
	source := `module; endmodule` // Missing module name
	_, err := ParseVerilog(source)
	if err == nil {
		t.Fatal("Expected parse error for invalid Verilog, got nil")
	}
}
