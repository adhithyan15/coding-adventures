// Brainfuck parser tests.
//
// These tests verify that ParseBrainfuck correctly converts Brainfuck source
// text into an Abstract Syntax Tree (AST). The grammar has four rules:
//
//   program     = { instruction }
//   instruction = loop | command
//   loop        = LOOP_START { instruction } LOOP_END
//   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
//
// The parser tests work at a higher level than the lexer tests: we examine
// the shape and rule names of the produced AST nodes, not individual tokens.
// We use the following helper conventions:
//
//  - ast.RuleName — the grammar rule name ("program", "instruction", etc.)
//  - ast.Children — a slice of *ASTNode or lexer.Token children
//  - findChildNodes — collects all direct *ASTNode children matching a rule
//  - deepCount — counts nodes with a given rule name anywhere in the subtree
//
// Error tests verify that ParseBrainfuck returns a non-nil error for
// structurally invalid inputs (unmatched brackets).
package brainfuck

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// =============================================================================
// Tree inspection helpers
// =============================================================================

// findChildNodes returns all direct *ASTNode children of node whose RuleName
// matches ruleName. Token children are skipped (they have no RuleName).
func findChildNodes(node *parser.ASTNode, ruleName string) []*parser.ASTNode {
	var result []*parser.ASTNode
	for _, child := range node.Children {
		if astChild, ok := child.(*parser.ASTNode); ok {
			if astChild.RuleName == ruleName {
				result = append(result, astChild)
			}
		}
	}
	return result
}

// deepCount counts all ASTNodes with the given rule name anywhere in the
// subtree rooted at node, using depth-first traversal. Useful for verifying
// that a specific construct appears exactly N times in the full tree.
func deepCount(node *parser.ASTNode, ruleName string) int {
	count := 0
	if node.RuleName == ruleName {
		count++
	}
	for _, child := range node.Children {
		if astChild, ok := child.(*parser.ASTNode); ok {
			count += deepCount(astChild, ruleName)
		}
	}
	return count
}

// =============================================================================
// TestParseBrainfuck_EmptyProgram
// =============================================================================
//
// Verifies that an empty source string produces a valid AST with the root
// rule "program" and no children. The grammar rule:
//
//   program = { instruction }
//
// A zero-repetition match is valid. The resulting program node has no children
// because there are no instructions. An empty Brainfuck program is legal — it
// runs, does nothing, and exits.
func TestParseBrainfuck_EmptyProgram(t *testing.T) {
	source := ""
	ast, err := ParseBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error parsing empty program: %v", err)
	}

	// The root must be the "program" rule (first rule in brainfuck.grammar)
	if ast.RuleName != "program" {
		t.Errorf("Expected root RuleName 'program', got %q", ast.RuleName)
	}

	// An empty program has no instruction children
	instructionNodes := findChildNodes(ast, "instruction")
	if len(instructionNodes) != 0 {
		t.Errorf("Expected 0 instruction children for empty program, got %d", len(instructionNodes))
	}
}

// =============================================================================
// TestParseBrainfuck_SimpleCommands
// =============================================================================
//
// Verifies that a sequence of four commands produces a program with four
// instruction nodes, each wrapping a command node.
//
//   program
//     instruction → command: INC
//     instruction → command: INC
//     instruction → command: RIGHT
//     instruction → command: RIGHT
func TestParseBrainfuck_SimpleCommands(t *testing.T) {
	source := "++>>"
	ast, err := ParseBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error parsing %q: %v", source, err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root RuleName 'program', got %q", ast.RuleName)
	}

	// The program rule wraps each instruction in an instruction node.
	// With four commands we expect four instruction children.
	instructionNodes := findChildNodes(ast, "instruction")
	if len(instructionNodes) != 4 {
		t.Errorf("Expected 4 instruction nodes for %q, got %d", source, len(instructionNodes))
	}

	// Each instruction should resolve to a command node
	for i, instr := range instructionNodes {
		commandNodes := findChildNodes(instr, "command")
		if len(commandNodes) != 1 {
			t.Errorf("Instruction[%d]: expected 1 command child, got %d", i, len(commandNodes))
		}
	}
}

// =============================================================================
// TestParseBrainfuck_SimpleLoop
// =============================================================================
//
// Verifies that "[>]" produces:
//
//   program
//     instruction
//       loop
//         LOOP_START("[")
//         instruction
//           command: RIGHT(">")
//         LOOP_END("]")
//
// This is the simplest non-trivial Brainfuck program: a loop containing a
// single move-right command. It moves the data pointer right until cell 0
// is zero.
func TestParseBrainfuck_SimpleLoop(t *testing.T) {
	source := "[>]"
	ast, err := ParseBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error parsing %q: %v", source, err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root 'program', got %q", ast.RuleName)
	}

	// The program should have exactly 1 instruction child
	instructions := findChildNodes(ast, "instruction")
	if len(instructions) != 1 {
		t.Fatalf("Expected 1 instruction for '[>]', got %d", len(instructions))
	}

	// That instruction should be a loop, not a command
	loops := findChildNodes(instructions[0], "loop")
	if len(loops) != 1 {
		t.Fatalf("Expected instruction to contain 1 loop node, got %d", len(loops))
	}

	// The loop body should contain exactly 1 instruction
	loopBody := findChildNodes(loops[0], "instruction")
	if len(loopBody) != 1 {
		t.Fatalf("Expected loop to contain 1 instruction, got %d", len(loopBody))
	}

	// That instruction should be a command (RIGHT)
	commands := findChildNodes(loopBody[0], "command")
	if len(commands) != 1 {
		t.Fatalf("Expected loop body instruction to be a command, got %d commands", len(commands))
	}
}

// =============================================================================
// TestParseBrainfuck_NestedLoop
// =============================================================================
//
// Verifies that "[[]]" produces a correctly nested AST. This exercises the
// recursive nature of the grammar:
//
//   program
//     instruction
//       loop       ← outer loop
//         LOOP_START
//         instruction
//           loop   ← inner loop
//             LOOP_START
//             (empty body)
//             LOOP_END
//         LOOP_END
func TestParseBrainfuck_NestedLoop(t *testing.T) {
	source := "[[]]"
	ast, err := ParseBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error parsing nested loops %q: %v", source, err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root 'program', got %q", ast.RuleName)
	}

	// There should be exactly 2 loop nodes in the entire tree: outer and inner
	loopCount := deepCount(ast, "loop")
	if loopCount != 2 {
		t.Errorf("Expected 2 loop nodes for '[[]]', got %d", loopCount)
	}

	// The outer program should have 1 instruction
	instructions := findChildNodes(ast, "instruction")
	if len(instructions) != 1 {
		t.Fatalf("Expected 1 top-level instruction, got %d", len(instructions))
	}

	// The outer instruction should be a loop
	outerLoops := findChildNodes(instructions[0], "loop")
	if len(outerLoops) != 1 {
		t.Fatalf("Expected 1 outer loop, got %d", len(outerLoops))
	}

	// The outer loop should have 1 instruction (the inner loop)
	outerBody := findChildNodes(outerLoops[0], "instruction")
	if len(outerBody) != 1 {
		t.Fatalf("Expected 1 instruction inside outer loop, got %d", len(outerBody))
	}

	// The inner instruction should be a loop too
	innerLoops := findChildNodes(outerBody[0], "loop")
	if len(innerLoops) != 1 {
		t.Fatalf("Expected 1 inner loop, got %d", len(innerLoops))
	}
}

// =============================================================================
// TestParseBrainfuck_ClearCellIdiom
// =============================================================================
//
// Verifies that "[-]" — the canonical "clear cell" idiom — parses correctly.
// This loop decrements the current cell until it reaches zero, effectively
// setting it to zero regardless of its initial value. Every Brainfuck
// programmer learns this pattern first.
func TestParseBrainfuck_ClearCellIdiom(t *testing.T) {
	source := "[-]"
	ast, err := ParseBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error parsing clear-cell idiom %q: %v", source, err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root 'program', got %q", ast.RuleName)
	}

	// Should have 1 instruction → 1 loop → 1 instruction → 1 command (DEC)
	instructions := findChildNodes(ast, "instruction")
	if len(instructions) != 1 {
		t.Fatalf("Expected 1 top-level instruction, got %d", len(instructions))
	}

	loops := findChildNodes(instructions[0], "loop")
	if len(loops) != 1 {
		t.Fatalf("Expected 1 loop, got %d", len(loops))
	}

	bodyInstructions := findChildNodes(loops[0], "instruction")
	if len(bodyInstructions) != 1 {
		t.Fatalf("Expected 1 instruction inside loop, got %d", len(bodyInstructions))
	}

	commands := findChildNodes(bodyInstructions[0], "command")
	if len(commands) != 1 {
		t.Fatalf("Expected 1 DEC command inside loop, got %d", len(commands))
	}
}

// =============================================================================
// TestParseBrainfuck_EmptyLoop
// =============================================================================
//
// Verifies that "[]" — an empty loop — is valid and parses without error.
// An empty loop is a legal but unusual construct: if the current cell is
// nonzero when [] is reached, it would be an infinite loop (since DEC never
// runs). In practice, programmers use [] only when the cell is known to be
// zero, making it a no-op. The parser must accept it regardless.
func TestParseBrainfuck_EmptyLoop(t *testing.T) {
	source := "[]"
	ast, err := ParseBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error parsing empty loop %q: %v", source, err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root 'program', got %q", ast.RuleName)
	}

	// Should have 1 instruction → 1 loop with 0 body instructions
	instructions := findChildNodes(ast, "instruction")
	if len(instructions) != 1 {
		t.Fatalf("Expected 1 instruction for '[]', got %d", len(instructions))
	}

	loops := findChildNodes(instructions[0], "loop")
	if len(loops) != 1 {
		t.Fatalf("Expected 1 loop, got %d", len(loops))
	}

	// The loop body is empty — no instruction children inside the loop
	bodyInstructions := findChildNodes(loops[0], "instruction")
	if len(bodyInstructions) != 0 {
		t.Errorf("Expected 0 instructions inside empty loop, got %d", len(bodyInstructions))
	}
}

// =============================================================================
// TestParseBrainfuck_UnmatchedOpen
// =============================================================================
//
// Verifies that "[[" — two opening brackets with no matching closing brackets
// — causes a parse error. The parser must detect unmatched brackets and
// return a non-nil error; it must not silently succeed with a partial tree.
func TestParseBrainfuck_UnmatchedOpen(t *testing.T) {
	source := "[["
	_, err := ParseBrainfuck(source)
	if err == nil {
		t.Errorf("Expected parse error for unmatched open brackets %q, but got nil error", source)
	}
}

// =============================================================================
// TestParseBrainfuck_UnmatchedClose
// =============================================================================
//
// Verifies that "]" — a closing bracket with no matching opening bracket
// — causes a parse error. A lone ] is not a valid instruction (it is not
// listed under the command rule, and loop requires a preceding [).
func TestParseBrainfuck_UnmatchedClose(t *testing.T) {
	source := "]"
	_, err := ParseBrainfuck(source)
	if err == nil {
		t.Errorf("Expected parse error for unmatched close bracket %q, but got nil error", source)
	}
}

// =============================================================================
// TestParseBrainfuck_CanonicalProgram
// =============================================================================
//
// Verifies the full AST structure for "++[>+<-]". This is the most thorough
// structural test: it checks every level of the tree.
//
// Expected tree (abbreviated):
//
//   program
//     instruction → command: INC         (+)
//     instruction → command: INC         (+)
//     instruction → loop:
//       LOOP_START                        ([)
//       instruction → command: RIGHT      (>)
//       instruction → command: INC        (+)
//       instruction → command: LEFT       (<)
//       instruction → command: DEC        (-)
//       LOOP_END                          (])
func TestParseBrainfuck_CanonicalProgram(t *testing.T) {
	source := "++[>+<-]"
	ast, err := ParseBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error parsing canonical program %q: %v", source, err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root 'program', got %q", ast.RuleName)
	}

	// The top-level program should have 3 instructions: INC, INC, loop
	topInstructions := findChildNodes(ast, "instruction")
	if len(topInstructions) != 3 {
		t.Fatalf("Expected 3 top-level instructions for %q, got %d", source, len(topInstructions))
	}

	// First two top-level instructions should be commands (INC)
	for i := 0; i < 2; i++ {
		cmds := findChildNodes(topInstructions[i], "command")
		loops := findChildNodes(topInstructions[i], "loop")
		if len(cmds) != 1 {
			t.Errorf("Instruction[%d]: expected 1 command, got %d", i, len(cmds))
		}
		if len(loops) != 0 {
			t.Errorf("Instruction[%d]: expected 0 loops, got %d", i, len(loops))
		}
	}

	// Third top-level instruction should be a loop
	thirdInstruction := topInstructions[2]
	topLoops := findChildNodes(thirdInstruction, "loop")
	if len(topLoops) != 1 {
		t.Fatalf("Third instruction: expected 1 loop, got %d", len(topLoops))
	}

	// The loop should contain 4 instructions: RIGHT, INC, LEFT, DEC
	loopBody := findChildNodes(topLoops[0], "instruction")
	if len(loopBody) != 4 {
		t.Fatalf("Loop body: expected 4 instructions, got %d", len(loopBody))
	}

	// Each loop body instruction should be a command
	for i, bodyInstr := range loopBody {
		cmds := findChildNodes(bodyInstr, "command")
		if len(cmds) != 1 {
			t.Errorf("Loop body instruction[%d]: expected 1 command, got %d", i, len(cmds))
		}
	}

	// Summary counts: 5 instructions total inside the loop body and top level
	// deepCount for "instruction" = 3 top-level + 4 inside loop = 7
	instrCount := deepCount(ast, "instruction")
	if instrCount != 7 {
		t.Errorf("Expected 7 total instruction nodes in tree, got %d", instrCount)
	}

	// One loop in the entire tree
	loopCount := deepCount(ast, "loop")
	if loopCount != 1 {
		t.Errorf("Expected 1 loop node in tree, got %d", loopCount)
	}
}

// =============================================================================
// TestCreateBrainfuckParser_ReturnsParser
// =============================================================================
//
// Verifies that CreateBrainfuckParser returns a non-nil GrammarParser and
// that calling Parse() on it produces a valid AST. This tests the two-step
// API (create then parse) as opposed to the one-shot ParseBrainfuck function.
func TestCreateBrainfuckParser_ReturnsParser(t *testing.T) {
	source := "+-"
	bfParser, err := CreateBrainfuckParser(source)
	if err != nil {
		t.Fatalf("Unexpected error from CreateBrainfuckParser: %v", err)
	}

	if bfParser == nil {
		t.Fatal("CreateBrainfuckParser returned nil parser")
	}

	// Parsing should succeed and produce a program node
	ast, err := bfParser.Parse()
	if err != nil {
		t.Fatalf("Unexpected error from Parse: %v", err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root 'program', got %q", ast.RuleName)
	}

	// Two commands: INC and DEC → two instructions
	instructions := findChildNodes(ast, "instruction")
	if len(instructions) != 2 {
		t.Errorf("Expected 2 instructions for '+-', got %d", len(instructions))
	}
}

// =============================================================================
// TestOperationResultFactory_FailPath
// =============================================================================
//
// Verifies that the ResultFactory.Fail method produces an OperationResult
// with the expected failure fields. This exercises the Fail code path in
// gen_capabilities.go, which is used when the grammar file cannot be read
// or parsed.
func TestOperationResultFactory_FailPath(t *testing.T) {
	// Use StartNew with a callback that explicitly calls rf.Fail.
	// This is how CreateBrainfuckLexer and CreateBrainfuckParser report errors.
	_, err := StartNew[int]("test.FailPath", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Fail(0, &_capabilityViolationError{
				category:  "fs",
				action:    "read",
				requested: "/test/path",
			})
		}).GetResult()

	if err == nil {
		t.Fatal("Expected non-nil error from Fail result, got nil")
	}

	// The error returned by GetResult should be the typed error we passed to Fail,
	// preserving its type for errors.As checks.
	var capViolErr *_capabilityViolationError
	if !isCapabilityViolation(err, &capViolErr) {
		t.Fatalf("Expected *_capabilityViolationError from GetResult, got %T: %v", err, err)
	}
}

// isCapabilityViolation is a helper that checks whether err is (or wraps)
// a *_capabilityViolationError.
func isCapabilityViolation(err error, target **_capabilityViolationError) bool {
	if capViol, ok := err.(*_capabilityViolationError); ok {
		*target = capViol
		return true
	}
	return false
}

// =============================================================================
// TestGetResult_PanicRecovery
// =============================================================================
//
// Verifies that GetResult recovers from a panic inside the callback and
// returns an error describing the unexpected failure, rather than crashing.
// This exercises the panic recovery path in gen_capabilities.go:GetResult.
//
// The capability framework is designed to be defensive: even if a callback
// panics due to a programming bug, the caller gets a descriptive error
// rather than a process crash.
func TestGetResult_PanicRecovery(t *testing.T) {
	_, err := StartNew[int]("test.PanicRecovery", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			panic("intentional panic for testing")
		}).GetResult()

	// GetResult must catch the panic and return a non-nil error
	if err == nil {
		t.Fatal("Expected error from GetResult when callback panics, got nil")
	}

	// The error message should mention the operation name and "unexpectedly"
	errMsg := err.Error()
	if errMsg == "" {
		t.Error("Expected non-empty error message from panic recovery")
	}
}

// =============================================================================
// TestGetResult_PanicOnUnexpected_Repanics
// =============================================================================
//
// Verifies that when PanicOnUnexpected() is set and the callback panics,
// GetResult re-panics instead of returning an error. This exercises the
// rePanic branch in GetResult.
func TestGetResult_PanicOnUnexpected_Repanics(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Error("Expected PanicOnUnexpected to re-panic, but no panic occurred")
		}
	}()

	StartNew[int]("test.RePanic", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			panic("intentional panic for PanicOnUnexpected test")
		}).PanicOnUnexpected().GetResult() //nolint:errcheck
}

// =============================================================================
// TestParseBrainfuck_CommentsIgnored
// =============================================================================
//
// Verifies that embedded comments do not affect the AST structure. The
// programs:
//
//   "++[>+<-]"
//   "increment ++ enter loop [ move right > increment + move left < decrement - ] done"
//
// must produce structurally identical trees: same number of instructions,
// same nesting depth, same number of loops. Comments are stripped by the
// lexer before the parser ever sees the token stream.
func TestParseBrainfuck_CommentsIgnored(t *testing.T) {
	// Clean program: no comments
	cleanSource := "++[>+<-]"
	// Same program with extensive commentary
	commentedSource := "increment ++ enter loop [ move right > increment + move left < decrement - ] done"

	cleanAST, err := ParseBrainfuck(cleanSource)
	if err != nil {
		t.Fatalf("Unexpected error parsing clean program: %v", err)
	}

	commentedAST, err := ParseBrainfuck(commentedSource)
	if err != nil {
		t.Fatalf("Unexpected error parsing commented program: %v", err)
	}

	// Both ASTs must have the same root rule
	if cleanAST.RuleName != commentedAST.RuleName {
		t.Errorf("Root rule mismatch: clean=%q, commented=%q",
			cleanAST.RuleName, commentedAST.RuleName)
	}

	// Both ASTs must have the same number of instruction nodes
	cleanInstrCount := deepCount(cleanAST, "instruction")
	commentedInstrCount := deepCount(commentedAST, "instruction")
	if cleanInstrCount != commentedInstrCount {
		t.Errorf("Instruction count mismatch: clean=%d, commented=%d",
			cleanInstrCount, commentedInstrCount)
	}

	// Both ASTs must have the same number of loop nodes
	cleanLoopCount := deepCount(cleanAST, "loop")
	commentedLoopCount := deepCount(commentedAST, "loop")
	if cleanLoopCount != commentedLoopCount {
		t.Errorf("Loop count mismatch: clean=%d, commented=%d",
			cleanLoopCount, commentedLoopCount)
	}

	// Both ASTs must have the same number of command nodes
	cleanCmdCount := deepCount(cleanAST, "command")
	commentedCmdCount := deepCount(commentedAST, "command")
	if cleanCmdCount != commentedCmdCount {
		t.Errorf("Command count mismatch: clean=%d, commented=%d",
			cleanCmdCount, commentedCmdCount)
	}
}
