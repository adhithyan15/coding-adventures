package brainfuckriscvcompiler

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestCompileSourceReturnsAssemblyAndBytes(t *testing.T) {
	result, err := CompileSource("+++[>++<-]>.")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.AST == nil {
		t.Fatal("expected AST")
	}
	if len(result.RawIR.Instructions) == 0 {
		t.Fatal("expected raw IR instructions")
	}
	if len(result.OptimizedIR.Instructions) == 0 {
		t.Fatal("expected optimized IR instructions")
	}
	if !strings.Contains(result.Assembly, "_start:") {
		t.Fatalf("expected assembly to contain _start label, got:\n%s", result.Assembly)
	}
	if !strings.Contains(result.Assembly, "tape:") {
		t.Fatalf("expected assembly to contain tape data, got:\n%s", result.Assembly)
	}
	if len(result.Binary) == 0 {
		t.Fatal("expected RISC-V binary bytes")
	}
}

func TestPackSourceAliasesCompileSource(t *testing.T) {
	compiled, err := CompileSource("+")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	packed, err := PackSource("+")
	if err != nil {
		t.Fatalf("pack failed: %v", err)
	}
	if string(packed.Binary) != string(compiled.Binary) {
		t.Fatal("expected pack source to match compile source")
	}
}

func TestRunSourceExecutesBrainfuckOutputOnRiscVSimulator(t *testing.T) {
	run, err := RunSource("+++++.")
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if !run.Halted {
		t.Fatal("expected simulator to halt")
	}
	if !run.HostExited {
		t.Fatal("expected host exit syscall")
	}
	if got := run.Output; len(got) != 1 || got[0] != 5 {
		t.Fatalf("expected single output byte 5, got %v", got)
	}
}

func TestRunSourceWithInputEchoesByte(t *testing.T) {
	run, err := RunSourceWithInput(",.", []byte("Z"))
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if got := run.OutputString(); got != "Z" {
		t.Fatalf("expected output %q, got %q", "Z", got)
	}
}

func TestCompilerOptionsCanDisableOptimization(t *testing.T) {
	optimize := false
	result, err := NewBrainfuckRiscVCompiler(&BrainfuckRiscVCompilerOptions{OptimizeIR: &optimize}).
		CompileSource("+++")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.Optimization.InstructionsBefore != result.Optimization.InstructionsAfter {
		t.Fatalf("expected no-op optimizer counts to match, got before=%d after=%d",
			result.Optimization.InstructionsBefore,
			result.Optimization.InstructionsAfter)
	}
}

func TestParseErrorsReportStage(t *testing.T) {
	_, err := CompileSource("[")
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
	result, err := WriteAssemblyFile("+.", outputPath)
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
