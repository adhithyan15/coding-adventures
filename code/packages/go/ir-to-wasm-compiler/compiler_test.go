package irtowasmcompiler

import (
	"testing"

	brainfuck "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck"
	brainfuckircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler"
)

func TestIrToWasmCompilerLowersBrainfuckIR(t *testing.T) {
	ast, err := brainfuck.ParseBrainfuck(",.")
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	compiled, err := brainfuckircompiler.Compile(ast, "echo.bf", brainfuckircompiler.ReleaseConfig())
	if err != nil {
		t.Fatalf("ir compile failed: %v", err)
	}

	module, err := NewIrToWasmCompiler().Compile(compiled.Program)
	if err != nil {
		t.Fatalf("lowering failed: %v", err)
	}

	if len(module.Memories) != 1 {
		t.Fatalf("expected one memory, got %d", len(module.Memories))
	}

	hasMemoryExport := false
	hasStartExport := false
	importNames := make([]string, len(module.Imports))
	for index, entry := range module.Imports {
		importNames[index] = entry.Name
	}
	for _, export := range module.Exports {
		if export.Name == "memory" {
			hasMemoryExport = true
		}
		if export.Name == "_start" {
			hasStartExport = true
		}
	}

	if !hasMemoryExport {
		t.Fatal("expected memory export")
	}
	if !hasStartExport {
		t.Fatal("expected _start export")
	}
	if len(importNames) != 2 || importNames[0] != "fd_write" || importNames[1] != "fd_read" {
		t.Fatalf("expected fd_write/fd_read imports, got %v", importNames)
	}
}
