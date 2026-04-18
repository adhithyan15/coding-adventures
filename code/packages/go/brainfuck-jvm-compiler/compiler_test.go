package brainfuckjvmcompiler

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestCompileSourceReturnsPipelineArtifacts(t *testing.T) {
	result, err := CompileSource("+.")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.Filename != "program.bf" {
		t.Fatalf("expected default filename program.bf, got %q", result.Filename)
	}
	if result.ClassName != "BrainfuckProgram" {
		t.Fatalf("expected default class name BrainfuckProgram, got %q", result.ClassName)
	}
	if len(result.RawIR.Instructions) == 0 {
		t.Fatal("expected raw IR instructions")
	}
	if len(result.OptimizedIR.Instructions) == 0 {
		t.Fatal("expected optimized IR instructions")
	}
	if len(result.ClassBytes) == 0 {
		t.Fatal("expected class-file bytes")
	}
	if result.ParsedClass.FindMethod("_start", "()I") == nil {
		t.Fatal("expected _start method")
	}
	if result.ParsedClass.FindMethod("main", "([Ljava/lang/String;)V") == nil {
		t.Fatal("expected main wrapper")
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
	if string(packed.ClassBytes) != string(compiled.ClassBytes) {
		t.Fatal("expected pack source to match compile source")
	}
}

func TestWriteClassFileWritesClasspathLayout(t *testing.T) {
	outputDir := t.TempDir()
	result, err := WriteClassFile("+.", outputDir)
	if err != nil {
		t.Fatalf("write failed: %v", err)
	}
	canonicalOutputDir, err := filepath.EvalSymlinks(outputDir)
	if err != nil {
		t.Fatalf("canonicalize failed: %v", err)
	}
	expected := filepath.Join(canonicalOutputDir, "BrainfuckProgram.class")
	if result.ClassFilePath != expected {
		t.Fatalf("expected %q, got %q", expected, result.ClassFilePath)
	}
	data, err := os.ReadFile(result.ClassFilePath)
	if err != nil {
		t.Fatalf("read failed: %v", err)
	}
	if string(data) != string(result.ClassBytes) {
		t.Fatal("written file does not match class bytes")
	}
}

func TestCompileSourceReturnsParseStageError(t *testing.T) {
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

func TestCompilerHonorsCustomDefaults(t *testing.T) {
	emitMainWrapper := false
	compiler := NewBrainfuckJvmCompiler(&BrainfuckJvmCompilerOptions{
		Filename:        "hello.bf",
		ClassName:       "custom.Program",
		EmitMainWrapper: &emitMainWrapper,
	})
	result, err := compiler.CompileSource("+")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.Filename != "hello.bf" {
		t.Fatalf("expected hello.bf, got %q", result.Filename)
	}
	if result.ClassName != "custom.Program" {
		t.Fatalf("expected custom.Program, got %q", result.ClassName)
	}
	if result.ParsedClass.FindMethod("main", "([Ljava/lang/String;)V") != nil {
		t.Fatal("did not expect main wrapper when disabled")
	}
}

func TestCompileSourceReturnsLowerStageErrorForInvalidClassName(t *testing.T) {
	compiler := NewBrainfuckJvmCompiler(&BrainfuckJvmCompilerOptions{ClassName: ".bad"})
	_, err := compiler.CompileSource("+.")
	if err == nil {
		t.Fatal("expected lower-jvm failure")
	}
	packageErr, ok := err.(*PackageError)
	if !ok {
		t.Fatalf("expected PackageError, got %T", err)
	}
	if packageErr.Stage != "lower-jvm" {
		t.Fatalf("expected lower-jvm stage, got %q", packageErr.Stage)
	}
}

func TestWriteClassFileReturnsWriteStageError(t *testing.T) {
	root := t.TempDir()
	filePath := filepath.Join(root, "not-a-directory")
	if err := os.WriteFile(filePath, []byte("x"), 0o644); err != nil {
		t.Fatalf("seed file failed: %v", err)
	}
	_, err := WriteClassFile("+.", filePath)
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

func TestCompiledProgramRunsOnGraalVMJava(t *testing.T) {
	graalvmHome := graalVMHomeForTests(t)
	result, err := CompileSource(strings.Repeat("+", 65) + ".")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	outputDir := t.TempDir()
	if _, err := WriteClassFile(strings.Repeat("+", 65)+".", outputDir); err != nil {
		t.Fatalf("write failed: %v", err)
	}
	java := exec.Command(filepath.Join(graalvmHome, "bin", "java"), "-cp", outputDir, result.ClassName)
	output, err := java.CombinedOutput()
	if err != nil {
		t.Fatalf("java failed: %v\n%s", err, string(output))
	}
	if string(output) != "A" {
		t.Fatalf("expected output A, got %q", string(output))
	}
}

func graalVMHomeForTests(t *testing.T) string {
	t.Helper()
	value := os.Getenv("GRAALVM_HOME")
	if value == "" {
		t.Skip("GRAALVM_HOME is not set for local runtime smoke tests")
	}
	return value
}
