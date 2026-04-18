package nibjvmcompiler

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

func TestCompileSourceReturnsPipelineArtifacts(t *testing.T) {
	result, err := CompileSource("static x: u4 = 7; fn main() { let y: u4 = x; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.ClassName != "NibProgram" {
		t.Fatalf("expected default class name NibProgram, got %q", result.ClassName)
	}
	if len(result.RawIR.Instructions) == 0 {
		t.Fatal("expected raw IR instructions")
	}
	if len(result.OptimizedIR.Instructions) == 0 {
		t.Fatal("expected optimized IR instructions")
	}
	if len(result.ClassBytes) == 0 {
		t.Fatal("expected class bytes")
	}
	if result.ParsedClass.FindMethod("_start", "()I") == nil {
		t.Fatal("expected _start method")
	}
	if result.ParsedClass.FindMethod("_fn_main", "()I") == nil {
		t.Fatal("expected _fn_main method")
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
	if string(compiled.ClassBytes) != string(packed.ClassBytes) {
		t.Fatal("expected pack source to match compile source")
	}
}

func TestWriteClassFileWritesClasspathLayout(t *testing.T) {
	outputDir := t.TempDir()
	result, err := WriteClassFile("fn main() -> u4 { return 7; }", outputDir)
	if err != nil {
		t.Fatalf("write failed: %v", err)
	}
	canonicalOutputDir, err := filepath.EvalSymlinks(outputDir)
	if err != nil {
		t.Fatalf("canonicalize failed: %v", err)
	}
	expected := filepath.Join(canonicalOutputDir, "NibProgram.class")
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

func TestCompileSourceReturnsTypeCheckStageError(t *testing.T) {
	_, err := CompileSource("fn main() { let x: bool = 1 +% 2; }")
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

func TestCompilerHonorsCustomDefaults(t *testing.T) {
	emitMainWrapper := false
	compiler := NewNibJvmCompiler(&NibJvmCompilerOptions{
		ClassName:       "custom.NibProgram",
		EmitMainWrapper: &emitMainWrapper,
	})
	result, err := compiler.CompileSource("fn main() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.ClassName != "custom.NibProgram" {
		t.Fatalf("expected custom.NibProgram, got %q", result.ClassName)
	}
	if result.ParsedClass.FindMethod("main", "([Ljava/lang/String;)V") != nil {
		t.Fatal("did not expect main wrapper when disabled")
	}
}

func TestCompiledProgramRunsOnGraalVMJavaViaDriver(t *testing.T) {
	graalvmHome := graalVMHomeForTests(t)
	result, err := CompileSource("fn main() -> u4 { return 7; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	outputDir := t.TempDir()
	if _, err := WriteClassFile("fn main() -> u4 { return 7; }", outputDir); err != nil {
		t.Fatalf("write failed: %v", err)
	}
	driverPath := filepath.Join(outputDir, "InvokeNib.java")
	driverSource := "public final class InvokeNib {\n" +
		"    public static void main(String[] args) {\n" +
		"        System.out.print(" + result.ClassName + "._start());\n" +
		"    }\n" +
		"}\n"
	if err := os.WriteFile(driverPath, []byte(driverSource), 0o644); err != nil {
		t.Fatalf("driver write failed: %v", err)
	}
	javac := exec.Command(filepath.Join(graalvmHome, "bin", "javac"), "-cp", outputDir, "InvokeNib.java")
	javac.Dir = outputDir
	if output, err := javac.CombinedOutput(); err != nil {
		t.Fatalf("javac failed: %v\n%s", err, string(output))
	}
	java := exec.Command(filepath.Join(graalvmHome, "bin", "java"), "-cp", outputDir, "InvokeNib")
	java.Dir = outputDir
	output, err := java.CombinedOutput()
	if err != nil {
		t.Fatalf("java failed: %v\n%s", err, string(output))
	}
	if string(output) != "7" {
		t.Fatalf("expected output 7, got %q", string(output))
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
