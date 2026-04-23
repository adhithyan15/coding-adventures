package nibriscvcompiler

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestCompileSourceReturnsPipelineArtifacts(t *testing.T) {
	result, err := CompileSource("fn main() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.TypedAST == nil {
		t.Fatal("expected typed AST")
	}
	if len(result.RawIR.Instructions) == 0 {
		t.Fatal("expected raw IR instructions")
	}
	if len(result.OptimizedIR.Instructions) == 0 {
		t.Fatal("expected optimized IR instructions")
	}
	if !strings.Contains(result.Assembly, "_fn_main:") {
		t.Fatalf("expected assembly to contain main label, got:\n%s", result.Assembly)
	}
	if len(result.Binary) == 0 {
		t.Fatal("expected RISC-V binary bytes")
	}
}

func TestPackSourceAliasesCompileSource(t *testing.T) {
	compiled, err := CompileSource("fn main() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	packed, err := PackSource("fn main() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("pack failed: %v", err)
	}
	if string(packed.Binary) != string(compiled.Binary) {
		t.Fatal("expected pack source to match compile source")
	}
}

func TestRunSourceExecutesNibOnRiscVSimulator(t *testing.T) {
	run, err := RunSource("fn main() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if !run.Halted {
		t.Fatal("expected simulator to halt")
	}
	if run.ReturnValue != 7 {
		t.Fatalf("expected return value 7, got %d", run.ReturnValue)
	}
}

func TestRunSourceExecutesNestedNibCallsOnRiscVSimulator(t *testing.T) {
	source := strings.Join([]string{
		"fn inner() -> u4 { return 7; }",
		"fn middle() -> u4 { return inner(); }",
		"fn main() -> u4 { return middle(); }",
	}, " ")
	run, err := RunSource(source)
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if !run.Halted {
		t.Fatal("expected simulator to halt")
	}
	if run.ReturnValue != 7 {
		t.Fatalf("expected nested call return value 7, got %d", run.ReturnValue)
	}
	if !strings.Contains(run.Package.Assembly, "sw ra, 0(sp)") ||
		!strings.Contains(run.Package.Assembly, "lw ra, 0(sp)") {
		t.Fatalf("expected call frame save/restore in assembly, got:\n%s", run.Package.Assembly)
	}
}

func TestRunSourcePreservesCallerLocalsAcrossNibCalls(t *testing.T) {
	source := strings.Join([]string{
		"fn add_one(value: u4) -> u4 {",
		"  let scratch: u4 = value +% 1;",
		"  return scratch;",
		"}",
		"fn main() -> u4 {",
		"  let kept: u4 = 5;",
		"  let called: u4 = add_one(6);",
		"  return kept + called;",
		"}",
	}, " ")
	run, err := RunSource(source)
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if !run.Halted {
		t.Fatal("expected simulator to halt")
	}
	if run.ReturnValue != 12 {
		t.Fatalf("expected caller local plus call result to be 12, got %d", run.ReturnValue)
	}
	if !strings.Contains(run.Package.Assembly, "sw x12, ") ||
		!strings.Contains(run.Package.Assembly, "lw x12, ") {
		t.Fatalf("expected caller-save spill/reload in assembly, got:\n%s", run.Package.Assembly)
	}
}

func TestTypeErrorsReportStage(t *testing.T) {
	_, err := CompileSource("fn main() { let flag: bool = 1; }")
	if err == nil {
		t.Fatal("expected type-check failure")
	}
	packageErr, ok := err.(*PackageError)
	if !ok {
		t.Fatalf("expected PackageError, got %T", err)
	}
	if packageErr.Stage != "type-check" {
		t.Fatalf("expected type-check stage, got %q", packageErr.Stage)
	}
}

func TestParseErrorsReportStage(t *testing.T) {
	_, err := CompileSource("fn main(")
	if err == nil {
		t.Fatal("expected parse failure")
	}
	packageErr, ok := err.(*PackageError)
	if !ok {
		t.Fatalf("expected PackageError, got %T", err)
	}
	if packageErr.Stage != "parse" {
		t.Fatalf("expected parse stage, got %q", packageErr.Stage)
	}
}

func TestWriteAssemblyFileWritesOutput(t *testing.T) {
	outputPath := filepath.Join(t.TempDir(), "program.s")
	result, err := WriteAssemblyFile("fn main() -> u4 { return 7; }", outputPath)
	if err != nil {
		t.Fatalf("write failed: %v", err)
	}
	data, err := os.ReadFile(outputPath)
	if err != nil {
		t.Fatalf("read failed: %v", err)
	}
	if string(data) != result.Assembly {
		t.Fatal("written assembly does not match result")
	}
}

func TestCompilerOptionsCanDisableOptimization(t *testing.T) {
	optimize := false
	result, err := NewNibRiscVCompiler(&NibRiscVCompilerOptions{OptimizeIR: &optimize}).
		CompileSource("fn main() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.Optimization.InstructionsBefore != result.Optimization.InstructionsAfter {
		t.Fatalf("expected no-op optimizer counts to match, got before=%d after=%d",
			result.Optimization.InstructionsBefore,
			result.Optimization.InstructionsAfter)
	}
}

func TestPackageErrorPreservesCause(t *testing.T) {
	cause := errors.New("boom")
	err := wrapStage("stage", cause)
	packageErr, ok := err.(*PackageError)
	if !ok {
		t.Fatalf("expected PackageError, got %T", err)
	}
	if packageErr.Error() != "boom" {
		t.Fatalf("unexpected error message %q", packageErr.Error())
	}
	if !errors.Is(packageErr, cause) {
		t.Fatal("expected errors.Is to see wrapped cause")
	}
	if wrapStage("stage", packageErr) != packageErr {
		t.Fatal("expected existing package errors to pass through")
	}
}
