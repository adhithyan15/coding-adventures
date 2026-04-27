package brainfuckircompiler

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck"
	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ──────────────────────────────────────────────────────────────────────────────
// Test helpers
// ──────────────────────────────────────────────────────────────────────────────

// compileSource is a convenience function that tokenizes, parses, and
// compiles a Brainfuck source string with the given config.
func compileSource(source string, config BuildConfig) (*CompileResult, error) {
	ast, err := brainfuck.ParseBrainfuck(source)
	if err != nil {
		return nil, err
	}
	return Compile(ast, "test.bf", config)
}

// mustCompile is compileSource that panics on error — for tests where
// parsing/compiling should always succeed.
func mustCompile(t *testing.T, source string, config BuildConfig) *CompileResult {
	t.Helper()
	result, err := compileSource(source, config)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	return result
}

// countOpcode counts how many instructions with the given opcode appear.
func countOpcode(program *ir.IrProgram, opcode ir.IrOp) int {
	count := 0
	for _, instr := range program.Instructions {
		if instr.Opcode == opcode {
			count++
		}
	}
	return count
}

// hasLabel checks if the program contains a label with the given name.
func hasLabel(program *ir.IrProgram, name string) bool {
	for _, instr := range program.Instructions {
		if instr.Opcode == ir.OpLabel && len(instr.Operands) > 0 {
			if label, ok := instr.Operands[0].(ir.IrLabel); ok && label.Name == name {
				return true
			}
		}
	}
	return false
}

// ──────────────────────────────────────────────────────────────────────────────
// BuildConfig tests
// ──────────────────────────────────────────────────────────────────────────────

func TestDebugConfig(t *testing.T) {
	cfg := DebugConfig()
	if !cfg.InsertBoundsChecks {
		t.Error("debug config should have bounds checks")
	}
	if !cfg.InsertDebugLocs {
		t.Error("debug config should have debug locs")
	}
	if !cfg.MaskByteArithmetic {
		t.Error("debug config should have byte masking")
	}
	if cfg.TapeSize != 30000 {
		t.Errorf("expected tape size 30000, got %d", cfg.TapeSize)
	}
}

func TestReleaseConfig(t *testing.T) {
	cfg := ReleaseConfig()
	if cfg.InsertBoundsChecks {
		t.Error("release config should NOT have bounds checks")
	}
	if cfg.InsertDebugLocs {
		t.Error("release config should NOT have debug locs")
	}
	if !cfg.MaskByteArithmetic {
		t.Error("release config should have byte masking")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Empty program
// ──────────────────────────────────────────────────────────────────────────────

func TestCompileEmptyProgram(t *testing.T) {
	result := mustCompile(t, "", ReleaseConfig())

	// Should have prologue + HALT
	if !hasLabel(result.Program, "_start") {
		t.Error("expected _start label")
	}
	if countOpcode(result.Program, ir.OpHalt) != 1 {
		t.Error("expected exactly 1 HALT instruction")
	}
	if result.Program.Version != 1 {
		t.Errorf("expected version 1, got %d", result.Program.Version)
	}
	if result.Program.EntryLabel != "_start" {
		t.Errorf("expected entry _start, got %s", result.Program.EntryLabel)
	}
}

func TestCompileEmptyProgramHasData(t *testing.T) {
	result := mustCompile(t, "", ReleaseConfig())

	if len(result.Program.Data) != 1 {
		t.Fatalf("expected 1 data decl, got %d", len(result.Program.Data))
	}
	if result.Program.Data[0].Label != "tape" {
		t.Error("expected 'tape' data label")
	}
	if result.Program.Data[0].Size != 30000 {
		t.Errorf("expected tape size 30000, got %d", result.Program.Data[0].Size)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Single commands
// ──────────────────────────────────────────────────────────────────────────────

func TestCompileIncrement(t *testing.T) {
	result := mustCompile(t, "+", ReleaseConfig())

	// INC produces: LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
	if countOpcode(result.Program, ir.OpLoadByte) < 1 {
		t.Error("expected at least 1 LOAD_BYTE for INC")
	}
	if countOpcode(result.Program, ir.OpStoreByte) < 1 {
		t.Error("expected at least 1 STORE_BYTE for INC")
	}
	if countOpcode(result.Program, ir.OpAndImm) < 1 {
		t.Error("expected AND_IMM for byte masking")
	}
}

func TestCompileIncrementNoMask(t *testing.T) {
	config := ReleaseConfig()
	config.MaskByteArithmetic = false
	result := mustCompile(t, "+", config)

	// Without masking, no AND_IMM (except in prologue which doesn't have one)
	if countOpcode(result.Program, ir.OpAndImm) != 0 {
		t.Error("expected no AND_IMM when masking is disabled")
	}
}

func TestCompileDecrement(t *testing.T) {
	result := mustCompile(t, "-", ReleaseConfig())

	// DEC should have ADD_IMM with value -1
	found := false
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpAddImm && len(instr.Operands) >= 3 {
			if imm, ok := instr.Operands[2].(ir.IrImmediate); ok && imm.Value == -1 {
				found = true
				break
			}
		}
	}
	if !found {
		t.Error("expected ADD_IMM with -1 for DEC command")
	}
}

func TestCompileRight(t *testing.T) {
	result := mustCompile(t, ">", ReleaseConfig())

	// RIGHT produces: ADD_IMM v1, v1, 1
	found := false
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpAddImm && len(instr.Operands) >= 3 {
			reg, isReg := instr.Operands[0].(ir.IrRegister)
			imm, isImm := instr.Operands[2].(ir.IrImmediate)
			if isReg && isImm && reg.Index == regTapePtr && imm.Value == 1 {
				found = true
				break
			}
		}
	}
	if !found {
		t.Error("expected ADD_IMM v1, v1, 1 for RIGHT command")
	}
}

func TestCompileLeft(t *testing.T) {
	result := mustCompile(t, "<", ReleaseConfig())

	// LEFT produces: ADD_IMM v1, v1, -1
	found := false
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpAddImm && len(instr.Operands) >= 3 {
			reg, isReg := instr.Operands[0].(ir.IrRegister)
			imm, isImm := instr.Operands[2].(ir.IrImmediate)
			if isReg && isImm && reg.Index == regTapePtr && imm.Value == -1 {
				found = true
				break
			}
		}
	}
	if !found {
		t.Error("expected ADD_IMM v1, v1, -1 for LEFT command")
	}
}

func TestCompileOutput(t *testing.T) {
	result := mustCompile(t, ".", ReleaseConfig())

	// OUTPUT produces: LOAD_BYTE + ADD_IMM copy to arg reg + SYSCALL 1
	if countOpcode(result.Program, ir.OpSyscall) < 1 {
		t.Error("expected SYSCALL for OUTPUT")
	}
	foundCopy := false
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpAddImm && len(instr.Operands) == 3 {
			dst, dstOK := instr.Operands[0].(ir.IrRegister)
			src, srcOK := instr.Operands[1].(ir.IrRegister)
			imm, immOK := instr.Operands[2].(ir.IrImmediate)
			if dstOK && srcOK && immOK &&
				dst.Index == regSysArg &&
				src.Index == regTemp &&
				imm.Value == 0 {
				foundCopy = true
				break
			}
		}
	}
	if !foundCopy {
		t.Error("expected ADD_IMM copy into syscall argument register")
	}

	// Verify syscall number is 1 (write)
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpSyscall && len(instr.Operands) > 0 {
			if imm, ok := instr.Operands[0].(ir.IrImmediate); ok && imm.Value == syscallWrite {
				return // found it
			}
		}
	}
	t.Error("expected SYSCALL 1 (write) for OUTPUT")
}

func TestCompileInput(t *testing.T) {
	result := mustCompile(t, ",", ReleaseConfig())

	// INPUT produces: SYSCALL 2 + STORE_BYTE
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpSyscall && len(instr.Operands) > 0 {
			if imm, ok := instr.Operands[0].(ir.IrImmediate); ok && imm.Value == syscallRead {
				return // found it
			}
		}
	}
	t.Error("expected SYSCALL 2 (read) for INPUT")
}

// ──────────────────────────────────────────────────────────────────────────────
// Loop compilation
// ──────────────────────────────────────────────────────────────────────────────

func TestCompileSimpleLoop(t *testing.T) {
	result := mustCompile(t, "[-]", ReleaseConfig())

	// Should have loop labels
	if !hasLabel(result.Program, "loop_0_start") {
		t.Error("expected loop_0_start label")
	}
	if !hasLabel(result.Program, "loop_0_end") {
		t.Error("expected loop_0_end label")
	}

	// Should have BRANCH_Z and JUMP for the loop structure
	if countOpcode(result.Program, ir.OpBranchZ) < 1 {
		t.Error("expected BRANCH_Z for loop entry")
	}
	if countOpcode(result.Program, ir.OpJump) < 1 {
		t.Error("expected JUMP for loop back-edge")
	}
}

func TestCompileNestedLoops(t *testing.T) {
	result := mustCompile(t, "[>[+<-]]", ReleaseConfig())

	// Should have two sets of loop labels
	if !hasLabel(result.Program, "loop_0_start") {
		t.Error("expected loop_0_start")
	}
	if !hasLabel(result.Program, "loop_1_start") {
		t.Error("expected loop_1_start")
	}
}

func TestCompileEmptyLoop(t *testing.T) {
	result := mustCompile(t, "[]", ReleaseConfig())

	// Empty loop should still have start/end labels + branch
	if !hasLabel(result.Program, "loop_0_start") {
		t.Error("expected loop_0_start")
	}
	if !hasLabel(result.Program, "loop_0_end") {
		t.Error("expected loop_0_end")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Debug mode: bounds checking
// ──────────────────────────────────────────────────────────────────────────────

func TestCompileWithBoundsChecks(t *testing.T) {
	result := mustCompile(t, ">", DebugConfig())

	// Debug mode should add CMP_GT + BRANCH_NZ before the pointer move
	if countOpcode(result.Program, ir.OpCmpGt) < 1 {
		t.Error("expected CMP_GT for right bounds check")
	}
	if countOpcode(result.Program, ir.OpBranchNz) < 1 {
		t.Error("expected BRANCH_NZ for bounds trap")
	}
	// Should have the __trap_oob label
	if !hasLabel(result.Program, "__trap_oob") {
		t.Error("expected __trap_oob trap handler label")
	}
}

func TestCompileWithBoundsChecksLeft(t *testing.T) {
	result := mustCompile(t, "<", DebugConfig())

	// Debug mode should add CMP_LT for left bounds check
	if countOpcode(result.Program, ir.OpCmpLt) < 1 {
		t.Error("expected CMP_LT for left bounds check")
	}
}

func TestCompileNoBoundsChecksInRelease(t *testing.T) {
	result := mustCompile(t, "><", ReleaseConfig())

	if countOpcode(result.Program, ir.OpCmpGt) != 0 {
		t.Error("release mode should not have CMP_GT bounds checks")
	}
	if countOpcode(result.Program, ir.OpCmpLt) != 0 {
		t.Error("release mode should not have CMP_LT bounds checks")
	}
	if hasLabel(result.Program, "__trap_oob") {
		t.Error("release mode should not have __trap_oob handler")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Source map tests
// ──────────────────────────────────────────────────────────────────────────────

func TestSourceMapBasic(t *testing.T) {
	result := mustCompile(t, "+.", ReleaseConfig())

	// Should have 2 SourceToAst entries (one for +, one for .)
	if len(result.SourceMap.SourceToAst.Entries) != 2 {
		t.Fatalf("expected 2 SourceToAst entries, got %d",
			len(result.SourceMap.SourceToAst.Entries))
	}

	// First entry: "+" at column 1
	entry0 := result.SourceMap.SourceToAst.Entries[0]
	if entry0.Pos.Column != 1 {
		t.Errorf("expected '+' at column 1, got %d", entry0.Pos.Column)
	}

	// Second entry: "." at column 2
	entry1 := result.SourceMap.SourceToAst.Entries[1]
	if entry1.Pos.Column != 2 {
		t.Errorf("expected '.' at column 2, got %d", entry1.Pos.Column)
	}
}

func TestSourceMapAstToIr(t *testing.T) {
	result := mustCompile(t, "+", ReleaseConfig())

	// Should have 1 AstToIr entry for the "+" command
	if len(result.SourceMap.AstToIr.Entries) != 1 {
		t.Fatalf("expected 1 AstToIr entry, got %d",
			len(result.SourceMap.AstToIr.Entries))
	}

	// "+" produces 4 IR instructions: LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
	entry := result.SourceMap.AstToIr.Entries[0]
	if len(entry.IrIDs) != 4 {
		t.Errorf("expected 4 IR IDs for '+', got %d: %v", len(entry.IrIDs), entry.IrIDs)
	}
}

func TestSourceMapFileName(t *testing.T) {
	result := mustCompile(t, "+", ReleaseConfig())

	for _, entry := range result.SourceMap.SourceToAst.Entries {
		if entry.Pos.File != "test.bf" {
			t.Errorf("expected file 'test.bf', got %q", entry.Pos.File)
		}
	}
}

func TestSourceMapLoopHasEntry(t *testing.T) {
	result := mustCompile(t, "[-]", ReleaseConfig())

	// Loop and the - command should both have source map entries
	// SourceToAst: loop + command = 2 entries
	if len(result.SourceMap.SourceToAst.Entries) < 2 {
		t.Errorf("expected at least 2 SourceToAst entries for '[-]', got %d",
			len(result.SourceMap.SourceToAst.Entries))
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// IR text output (printer integration)
// ──────────────────────────────────────────────────────────────────────────────

func TestCompiledIRIsPrintable(t *testing.T) {
	result := mustCompile(t, "+.", ReleaseConfig())

	text := ir.Print(result.Program)
	if !strings.Contains(text, ".version 1") {
		t.Error("printed IR should contain .version 1")
	}
	if !strings.Contains(text, ".data tape 30000 0") {
		t.Error("printed IR should contain .data tape 30000 0")
	}
	if !strings.Contains(text, ".entry _start") {
		t.Error("printed IR should contain .entry _start")
	}
	if !strings.Contains(text, "LOAD_BYTE") {
		t.Error("printed IR should contain LOAD_BYTE")
	}
	if !strings.Contains(text, "HALT") {
		t.Error("printed IR should contain HALT")
	}
}

func TestCompiledIRRoundtrip(t *testing.T) {
	result := mustCompile(t, "++[-].", ReleaseConfig())

	text := ir.Print(result.Program)
	parsed, err := ir.Parse(text)
	if err != nil {
		t.Fatalf("roundtrip parse failed: %v\n\nText:\n%s", err, text)
	}

	// Verify instruction count matches
	if len(parsed.Instructions) != len(result.Program.Instructions) {
		t.Errorf("roundtrip: instruction count mismatch: got %d, want %d",
			len(parsed.Instructions), len(result.Program.Instructions))
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Complex programs
// ──────────────────────────────────────────────────────────────────────────────

func TestCompileHelloWorldSubset(t *testing.T) {
	// A simplified "Hello World" fragment: set cell 0 to 72 ('H') and output it
	// 72 = 8 * 9, so: ++++++++ [>+++++++++<-] >.
	source := "++++++++[>+++++++++<-]>."
	result := mustCompile(t, source, ReleaseConfig())

	// Should have at least one loop
	if !hasLabel(result.Program, "loop_0_start") {
		t.Error("expected loop_0_start for Hello World fragment")
	}

	// Should have output syscall
	foundOutput := false
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpSyscall && len(instr.Operands) > 0 {
			if imm, ok := instr.Operands[0].(ir.IrImmediate); ok && imm.Value == syscallWrite {
				foundOutput = true
			}
		}
	}
	if !foundOutput {
		t.Error("expected SYSCALL 1 (output) in Hello World fragment")
	}
}

func TestCompileCatProgram(t *testing.T) {
	// Cat program: ,[.,]
	result := mustCompile(t, ",[.,]", ReleaseConfig())

	// Should have both read and write syscalls
	foundRead := false
	foundWrite := false
	for _, instr := range result.Program.Instructions {
		if instr.Opcode == ir.OpSyscall && len(instr.Operands) > 0 {
			if imm, ok := instr.Operands[0].(ir.IrImmediate); ok {
				if imm.Value == syscallRead {
					foundRead = true
				}
				if imm.Value == syscallWrite {
					foundWrite = true
				}
			}
		}
	}
	if !foundRead {
		t.Error("expected SYSCALL 2 (read) in cat program")
	}
	if !foundWrite {
		t.Error("expected SYSCALL 1 (write) in cat program")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Custom tape size
// ──────────────────────────────────────────────────────────────────────────────

func TestCustomTapeSize(t *testing.T) {
	config := ReleaseConfig()
	config.TapeSize = 1000
	result := mustCompile(t, "", config)

	if result.Program.Data[0].Size != 1000 {
		t.Errorf("expected tape size 1000, got %d", result.Program.Data[0].Size)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Instruction ID uniqueness
// ──────────────────────────────────────────────────────────────────────────────

func TestInstructionIDsAreUnique(t *testing.T) {
	result := mustCompile(t, "++[>+<-].", ReleaseConfig())

	seen := make(map[int]bool)
	for _, instr := range result.Program.Instructions {
		if instr.ID == -1 {
			continue // labels have -1
		}
		if seen[instr.ID] {
			t.Errorf("duplicate instruction ID: %d", instr.ID)
		}
		seen[instr.ID] = true
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Error cases
// ──────────────────────────────────────────────────────────────────────────────

func TestCompileInvalidAST(t *testing.T) {
	// Manually create an AST with wrong root node type
	ast := &parser.ASTNode{RuleName: "not_a_program"}
	_, err := Compile(ast, "test.bf", ReleaseConfig())
	if err == nil {
		t.Error("expected error for non-program AST node")
	}
}

func TestCompileZeroTapeSize(t *testing.T) {
	ast, err := brainfuck.ParseBrainfuck("")
	if err != nil {
		t.Fatal(err)
	}
	config := ReleaseConfig()
	config.TapeSize = 0
	_, err = Compile(ast, "test.bf", config)
	if err == nil {
		t.Error("expected error for zero tape size")
	}
}

func TestCompileNegativeTapeSize(t *testing.T) {
	ast, err := brainfuck.ParseBrainfuck("")
	if err != nil {
		t.Fatal(err)
	}
	config := ReleaseConfig()
	config.TapeSize = -1
	_, err = Compile(ast, "test.bf", config)
	if err == nil {
		t.Error("expected error for negative tape size")
	}
}
