package brainfuckwasmcompiler

import (
	"os"
	"path/filepath"
	"testing"

	wasmruntime "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-runtime"
)

type byteReader struct {
	bytes  []byte
	offset int
}

func newByteReader(text string) *byteReader {
	return &byteReader{bytes: []byte(text)}
}

func (r *byteReader) Read(count int) []byte {
	if r.offset >= len(r.bytes) {
		return nil
	}
	end := r.offset + count
	if end > len(r.bytes) {
		end = len(r.bytes)
	}
	chunk := append([]byte{}, r.bytes[r.offset:end]...)
	r.offset = end
	return chunk
}

func run(binary []byte, stdin string) ([]int, []string, error) {
	output := make([]string, 0)
	reader := newByteReader(stdin)
	runtime := wasmruntime.New(wasmruntime.NewWasiStubFromConfig(wasmruntime.WasiConfig{
		StdoutCallback: func(text string) { output = append(output, text) },
		StdinCallback:  reader.Read,
	}))
	result, err := runtime.LoadAndRun(binary, "_start", nil)
	return result, output, err
}

func TestCompileSourceReturnsPipelineArtifacts(t *testing.T) {
	result, err := CompileSource("+.")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
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
	hasStart := false
	for _, export := range result.Module.Exports {
		if export.Name == "_start" {
			hasStart = true
		}
	}
	if !hasStart {
		t.Fatal("expected _start export")
	}
	if result.Filename != "program.bf" {
		t.Fatalf("expected default filename program.bf, got %q", result.Filename)
	}
}

func TestPackSourceAliasesCompileSource(t *testing.T) {
	compiled, err := CompileSource("+.")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	packed, err := PackSource("+.")
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

	result, err := WriteWasmFile("+.", outputPath)
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

func TestCompiledProgramRunsInRuntime(t *testing.T) {
	result, err := CompileSource(stringsRepeat("+", 65) + ".")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	executionResult, output, err := run(result.Binary, "")
	if err != nil {
		t.Fatalf("runtime failed: %v", err)
	}
	if len(executionResult) != 1 || executionResult[0] != 0 {
		t.Fatalf("expected [0], got %v", executionResult)
	}
	if len(output) != 1 || output[0] != "A" {
		t.Fatalf("expected output A, got %v", output)
	}
}

func TestCompiledInputProgramRunsInRuntime(t *testing.T) {
	result, err := CompileSource(",.")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	executionResult, output, err := run(result.Binary, "Z")
	if err != nil {
		t.Fatalf("runtime failed: %v", err)
	}
	if len(executionResult) != 1 || executionResult[0] != 0 {
		t.Fatalf("expected [0], got %v", executionResult)
	}
	if len(output) != 1 || output[0] != "Z" {
		t.Fatalf("expected output Z, got %v", output)
	}
}

func TestCompiledCatProgramRunsInRuntime(t *testing.T) {
	result, err := CompileSource(",[.,]")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	executionResult, output, err := run(result.Binary, "Hi")
	if err != nil {
		t.Fatalf("runtime failed: %v", err)
	}
	if len(executionResult) != 1 || executionResult[0] != 0 {
		t.Fatalf("expected [0], got %v", executionResult)
	}
	if len(output) != 2 || output[0] != "H" || output[1] != "i" {
		t.Fatalf("expected output [H i], got %v", output)
	}
}

func TestCompilerHonorsCustomFilename(t *testing.T) {
	compiler := NewBrainfuckWasmCompiler(&BrainfuckWasmCompilerOptions{Filename: "hello.bf"})
	result, err := compiler.CompileSource("+")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.Filename != "hello.bf" {
		t.Fatalf("expected hello.bf, got %q", result.Filename)
	}
}

func stringsRepeat(value string, count int) string {
	result := ""
	for index := 0; index < count; index++ {
		result += value
	}
	return result
}
