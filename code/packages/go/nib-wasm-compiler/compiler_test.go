package nibwasmcompiler

import (
	"errors"
	"os"
	"path/filepath"
	"testing"

	nibparser "github.com/adhithyan15/coding-adventures/code/packages/go/nib-parser"
	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
	wasmruntime "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-runtime"
)

func run(binary []byte, entry string, args []int) ([]int, error) {
	return wasmruntime.New(nil).LoadAndRun(binary, entry, args)
}

func TestCompileSourceReturnsPipelineArtifacts(t *testing.T) {
	result, err := CompileSource("fn answer() -> u4 { return 7; }")
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
	if len(result.Binary) == 0 {
		t.Fatal("expected wasm binary bytes")
	}
	hasAnswer := false
	for _, export := range result.Module.Exports {
		if export.Name == "answer" {
			hasAnswer = true
		}
	}
	if !hasAnswer {
		t.Fatal("expected answer export")
	}
}

func TestPackSourceAliasesCompileSource(t *testing.T) {
	compiled, err := CompileSource("fn answer() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	packed, err := PackSource("fn answer() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("pack failed: %v", err)
	}
	if string(packed.Binary) != string(compiled.Binary) {
		t.Fatal("expected pack source to match compile source")
	}
}

func TestWriteWasmFileWritesOutput(t *testing.T) {
	outputDir := t.TempDir()
	outputPath := filepath.Join(outputDir, "program.wasm")

	result, err := WriteWasmFile("fn answer() -> u4 { return 7; }", outputPath)
	if err != nil {
		t.Fatalf("write failed: %v", err)
	}
	data, err := os.ReadFile(outputPath)
	if err != nil {
		t.Fatalf("read failed: %v", err)
	}
	if string(data) != string(result.Binary) {
		t.Fatal("written file does not match binary output")
	}
}

func TestCompiledFunctionRunsInRuntime(t *testing.T) {
	result, err := CompileSource("fn answer() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	values, err := run(result.Binary, "answer", nil)
	if err != nil {
		t.Fatalf("runtime failed: %v", err)
	}
	if len(values) != 1 || values[0] != 7 {
		t.Fatalf("expected [7], got %v", values)
	}
}

func TestCompiledEntrypointRunsInRuntime(t *testing.T) {
	source := "fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }"
	result, err := CompileSource(source)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	values, err := run(result.Binary, "_start", nil)
	if err != nil {
		t.Fatalf("runtime failed: %v", err)
	}
	if len(values) != 1 || values[0] != 7 {
		t.Fatalf("expected [7], got %v", values)
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

func TestWriteErrorsReportStage(t *testing.T) {
	outputDir := t.TempDir()
	_, err := WriteWasmFile("fn answer() -> u4 { return 7; }", outputDir)
	if err == nil {
		t.Fatal("expected write failure")
	}
	packageErr, ok := err.(*PackageError)
	if !ok {
		t.Fatalf("expected PackageError, got %T", err)
	}
	if packageErr.Stage != "write" {
		t.Fatalf("expected write stage, got %q", packageErr.Stage)
	}
}

func TestCompilerOptionsCanDisableOptimization(t *testing.T) {
	optimize := false
	result, err := NewNibWasmCompiler(&NibWasmCompilerOptions{OptimizeIR: &optimize}).
		CompileSource("fn answer() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.Optimization.InstructionsBefore != result.Optimization.InstructionsAfter {
		t.Fatalf("expected no-op optimizer counts to match, got before=%d after=%d",
			result.Optimization.InstructionsBefore,
			result.Optimization.InstructionsAfter)
	}
}

func TestExtractSignaturesHandlesEmptyTypedAST(t *testing.T) {
	if signatures := extractSignatures(nil); len(signatures) != 1 || signatures[0].Label != "_start" {
		t.Fatalf("expected only _start signature for nil typed AST, got %#v", signatures)
	}
}

func TestExtractSignaturesCountsParameters(t *testing.T) {
	ast, err := nibparser.ParseNib("fn add(a: u4, b: u4) -> u4 { return a +% b; }")
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	checked := nibtypechecker.CheckNib(ast)
	if !checked.OK {
		t.Fatalf("type check failed: %#v", checked.Errors)
	}
	signatures := extractSignatures(checked.TypedAST)

	if len(signatures) != 2 {
		t.Fatalf("expected _start plus add signature, got %#v", signatures)
	}
	if signatures[1].Label != "_fn_add" || signatures[1].ParamCount != 2 || signatures[1].ExportName != "add" {
		t.Fatalf("unexpected add signature: %#v", signatures[1])
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
