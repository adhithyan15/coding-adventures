package irtojvmclassfile

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	jvmclassfile "github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file"
)

func TestLowerIRToJvmClassFileProducesParseableClass(t *testing.T) {
	artifact, err := LowerIRToJvmClassFile(simpleProgram(), JvmBackendConfig{
		ClassName:       "demo.Example",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	if artifact.ClassFilename() != "demo/Example.class" {
		t.Fatalf("unexpected class filename: %s", artifact.ClassFilename())
	}

	parsed, err := jvmclassfile.ParseClassFile(artifact.ClassBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if parsed.ThisClassName != "demo/Example" {
		t.Fatalf("expected internal class name demo/Example, got %q", parsed.ThisClassName)
	}
	if parsed.FindMethod("_start", "()I") == nil {
		t.Fatal("expected _start method")
	}
	if parsed.FindMethod("main", "([Ljava/lang/String;)V") == nil {
		t.Fatal("expected main wrapper")
	}
	if parsed.FindMethod("__ca_syscall", "(I)V") == nil {
		t.Fatal("expected syscall helper")
	}
}

func TestLowerProgramWithCallAndStaticDataToParseableClass(t *testing.T) {
	artifact, err := LowerIRToJvmClassFile(programWithCallAndStaticData(), JvmBackendConfig{
		ClassName:       "NibProgram",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	parsed, err := jvmclassfile.ParseClassFile(artifact.ClassBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if parsed.FindMethod("_start", "()I") == nil {
		t.Fatal("expected _start method")
	}
	if parsed.FindMethod("_fn_main", "()I") == nil {
		t.Fatal("expected _fn_main method")
	}
}

func TestWriteClassFileUsesClasspathLayout(t *testing.T) {
	artifact, err := LowerIRToJvmClassFile(simpleProgram(), JvmBackendConfig{
		ClassName:       "demo.Example",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}

	outputDir := t.TempDir()
	target, err := WriteClassFile(artifact, outputDir)
	if err != nil {
		t.Fatalf("write failed: %v", err)
	}
	canonicalOutputDir, err := filepath.EvalSymlinks(outputDir)
	if err != nil {
		t.Fatalf("canonicalize failed: %v", err)
	}
	expected := filepath.Join(canonicalOutputDir, "demo", "Example.class")
	if target != expected {
		t.Fatalf("expected %q, got %q", expected, target)
	}
	data, err := os.ReadFile(target)
	if err != nil {
		t.Fatalf("read failed: %v", err)
	}
	if string(data) != string(artifact.ClassBytes) {
		t.Fatal("written class file does not match artifact bytes")
	}
}

func TestInvalidClassNameIsRejected(t *testing.T) {
	_, err := LowerIRToJvmClassFile(simpleProgram(), JvmBackendConfig{ClassName: ".Example"})
	if err == nil || !strings.Contains(err.Error(), "legal Java binary name") {
		t.Fatalf("expected illegal class name error, got %v", err)
	}
}

func TestWriteClassFileRejectsEscapingArtifact(t *testing.T) {
	artifact := &JVMClassArtifact{
		ClassName:  ".Escape",
		ClassBytes: []byte("class-bytes"),
	}
	_, err := WriteClassFile(artifact, t.TempDir())
	if err == nil || !strings.Contains(err.Error(), "escapes the requested") {
		t.Fatalf("expected path escape error, got %v", err)
	}
}

func TestWriteClassFileRejectsSymlinkedParentDirectory(t *testing.T) {
	artifact, err := LowerIRToJvmClassFile(simpleProgram(), JvmBackendConfig{
		ClassName:       "demo.Example",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	root := t.TempDir()
	sink := t.TempDir()
	if err := os.Symlink(sink, filepath.Join(root, "demo")); err != nil {
		t.Fatalf("symlink failed: %v", err)
	}
	_, err = WriteClassFile(artifact, root)
	if err == nil || !strings.Contains(err.Error(), "symlinked or invalid directory component") {
		t.Fatalf("expected symlink rejection, got %v", err)
	}
}

func TestWriteClassFileRejectsOutputRootWithSymlinkedAncestor(t *testing.T) {
	artifact, err := LowerIRToJvmClassFile(simpleProgram(), JvmBackendConfig{
		ClassName:       "demo.Example",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	root := t.TempDir()
	sink := t.TempDir()
	if err := os.Symlink(sink, filepath.Join(root, "linked-root")); err != nil {
		t.Fatalf("symlink failed: %v", err)
	}
	_, err = WriteClassFile(artifact, filepath.Join(root, "linked-root", "nested"))
	if err == nil || !strings.Contains(err.Error(), "symlinked or invalid path component") {
		t.Fatalf("expected root symlink rejection, got %v", err)
	}
}

func TestLargeStaticDataIsRejected(t *testing.T) {
	program := simpleProgram()
	program.AddData(ir.IrDataDecl{Label: "huge", Size: maxStaticDataBytes + 1, Init: 1})
	_, err := LowerIRToJvmClassFile(program, JvmBackendConfig{ClassName: "TooMuchData"})
	if err == nil || !strings.Contains(err.Error(), "Total static data exceeds") {
		t.Fatalf("expected static data limit error, got %v", err)
	}
}

func TestBranchTargetEscapingCallableIsRejected(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpCall, Operands: []ir.IrOperand{ir.IrLabel{Name: "_fn_other"}}, ID: 0})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpJump, Operands: []ir.IrOperand{ir.IrLabel{Name: "_fn_other"}}, ID: 0})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpRet, ID: 1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_fn_other"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpRet, ID: 2})
	_, err := LowerIRToJvmClassFile(program, JvmBackendConfig{ClassName: "EscapeBranch"})
	if err == nil || !strings.Contains(err.Error(), "escapes callable") {
		t.Fatalf("expected escaping branch error, got %v", err)
	}
}

func TestDuplicateHelperCollisionIsRejected(t *testing.T) {
	program := ir.NewIrProgram("__ca_syscall")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "__ca_syscall"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpRet, ID: 0})
	_, err := LowerIRToJvmClassFile(program, JvmBackendConfig{ClassName: "Collision"})
	if err == nil || !strings.Contains(err.Error(), "collide with helper names") {
		t.Fatalf("expected helper collision error, got %v", err)
	}
}

func TestLowerProgramCoversArithmeticMemoryAndComparisonOps(t *testing.T) {
	artifact, err := LowerIRToJvmClassFile(programCoveringMostOps(), JvmBackendConfig{
		ClassName:       "OpsProgram",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	parsed, err := jvmclassfile.ParseClassFile(artifact.ClassBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if parsed.FindMethod("_start", "()I") == nil {
		t.Fatal("expected _start method")
	}
	if parsed.FindMethod("_fn_helper", "()I") == nil {
		t.Fatal("expected helper method")
	}
}

func TestImmediateOutsideInt32RangeIsRejected(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{
		Opcode:   ir.OpLoadImm,
		Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrImmediate{Value: maxInt32 + 1}},
		ID:       0,
	})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpHalt, ID: 1})
	_, err := LowerIRToJvmClassFile(program, JvmBackendConfig{ClassName: "TooWide"})
	if err == nil || !strings.Contains(err.Error(), "32-bit JVM integer range") {
		t.Fatalf("expected int32 validation error, got %v", err)
	}
}

func TestGeneratedBrainfuckClassRunsOnGraalVMJava(t *testing.T) {
	graalvmHome := graalVMHomeForTests(t)
	artifact, err := LowerIRToJvmClassFile(programThatWritesA(), JvmBackendConfig{
		ClassName:       "BrainfuckA",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	outputDir := t.TempDir()
	if _, err := WriteClassFile(artifact, outputDir); err != nil {
		t.Fatalf("write failed: %v", err)
	}
	javaBin := filepath.Join(graalvmHome, "bin", "java")
	command := exec.Command(javaBin, "-cp", outputDir, "BrainfuckA")
	result, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("java failed: %v\n%s", err, string(result))
	}
	if string(result) != "A" {
		t.Fatalf("expected output A, got %q", string(result))
	}
}

func TestGeneratedNibClassRunsOnGraalVMJavaViaDriver(t *testing.T) {
	graalvmHome := graalVMHomeForTests(t)
	artifact, err := LowerIRToJvmClassFile(programThatReturnsSeven(), JvmBackendConfig{
		ClassName:       "NibReturn",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	outputDir := t.TempDir()
	if _, err := WriteClassFile(artifact, outputDir); err != nil {
		t.Fatalf("write failed: %v", err)
	}
	driverPath := filepath.Join(outputDir, "InvokeNib.java")
	driverSource := strings.Join([]string{
		"public final class InvokeNib {",
		"    public static void main(String[] args) {",
		"        System.out.print(NibReturn._start());",
		"    }",
		"}",
	}, "\n")
	if err := os.WriteFile(driverPath, []byte(driverSource), 0o644); err != nil {
		t.Fatalf("driver write failed: %v", err)
	}
	javacBin := filepath.Join(graalvmHome, "bin", "javac")
	javaBin := filepath.Join(graalvmHome, "bin", "java")
	javac := exec.Command(javacBin, "-cp", outputDir, "InvokeNib.java")
	javac.Dir = outputDir
	if output, err := javac.CombinedOutput(); err != nil {
		t.Fatalf("javac failed: %v\n%s", err, string(output))
	}
	java := exec.Command(javaBin, "-cp", outputDir, "InvokeNib")
	java.Dir = outputDir
	result, err := java.CombinedOutput()
	if err != nil {
		t.Fatalf("java failed: %v\n%s", err, string(result))
	}
	if string(result) != "7" {
		t.Fatalf("expected output 7, got %q", string(result))
	}
}

func simpleProgram() *ir.IrProgram {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{
		Opcode:   ir.OpLabel,
		Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}},
		ID:       -1,
	})
	program.AddInstruction(ir.IrInstruction{
		Opcode:   ir.OpLoadImm,
		Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrImmediate{Value: 0}},
		ID:       0,
	})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpHalt, ID: 1})
	return program
}

func programWithCallAndStaticData() *ir.IrProgram {
	program := ir.NewIrProgram("_start")
	program.AddData(ir.IrDataDecl{Label: "x", Size: 4, Init: 7})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpCall, Operands: []ir.IrOperand{ir.IrLabel{Name: "_fn_main"}}, ID: 0})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpHalt, ID: 1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_fn_main"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadAddr, Operands: []ir.IrOperand{ir.IrRegister{Index: 2}, ir.IrLabel{Name: "x"}}, ID: 2})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 3}, ir.IrImmediate{Value: 0}}, ID: 3})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadWord, Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrRegister{Index: 2}, ir.IrRegister{Index: 3}}, ID: 4})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpRet, ID: 5})
	return program
}

func programThatWritesA() *ir.IrProgram {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 4}, ir.IrImmediate{Value: 65}}, ID: 0})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrImmediate{Value: 0}}, ID: 1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpSyscall, Operands: []ir.IrOperand{ir.IrImmediate{Value: 1}}, ID: 2})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpHalt, ID: 3})
	return program
}

func programThatReturnsSeven() *ir.IrProgram {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrImmediate{Value: 7}}, ID: 0})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpHalt, ID: 1})
	return program
}

func programCoveringMostOps() *ir.IrProgram {
	program := ir.NewIrProgram("_start")
	program.AddData(ir.IrDataDecl{Label: "tape", Size: 8, Init: 0})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpCall, Operands: []ir.IrOperand{ir.IrLabel{Name: "_fn_helper"}}, ID: 0})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadAddr, Operands: []ir.IrOperand{ir.IrRegister{Index: 2}, ir.IrLabel{Name: "tape"}}, ID: 1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 3}, ir.IrImmediate{Value: 0}}, ID: 2})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpStoreByte, Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrRegister{Index: 2}, ir.IrRegister{Index: 3}}, ID: 3})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadByte, Operands: []ir.IrOperand{ir.IrRegister{Index: 4}, ir.IrRegister{Index: 2}, ir.IrRegister{Index: 3}}, ID: 4})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAddImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 5}, ir.IrRegister{Index: 4}, ir.IrImmediate{Value: 1}}, ID: 5})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpSub, Operands: []ir.IrOperand{ir.IrRegister{Index: 6}, ir.IrRegister{Index: 5}, ir.IrRegister{Index: 1}}, ID: 6})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAndImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 7}, ir.IrRegister{Index: 6}, ir.IrImmediate{Value: 0xFF}}, ID: 7})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAnd, Operands: []ir.IrOperand{ir.IrRegister{Index: 8}, ir.IrRegister{Index: 7}, ir.IrRegister{Index: 1}}, ID: 8})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpCmpEq, Operands: []ir.IrOperand{ir.IrRegister{Index: 9}, ir.IrRegister{Index: 8}, ir.IrRegister{Index: 1}}, ID: 9})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpBranchZ, Operands: []ir.IrOperand{ir.IrRegister{Index: 9}, ir.IrLabel{Name: "check"}}, ID: 10})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrImmediate{Value: 0}}, ID: 11})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpJump, Operands: []ir.IrOperand{ir.IrLabel{Name: "done"}}, ID: 12})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "check"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpCmpNe, Operands: []ir.IrOperand{ir.IrRegister{Index: 10}, ir.IrRegister{Index: 5}, ir.IrRegister{Index: 6}}, ID: 13})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpCmpLt, Operands: []ir.IrOperand{ir.IrRegister{Index: 11}, ir.IrRegister{Index: 6}, ir.IrRegister{Index: 5}}, ID: 14})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpCmpGt, Operands: []ir.IrOperand{ir.IrRegister{Index: 12}, ir.IrRegister{Index: 5}, ir.IrRegister{Index: 6}}, ID: 15})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAdd, Operands: []ir.IrOperand{ir.IrRegister{Index: 13}, ir.IrRegister{Index: 10}, ir.IrRegister{Index: 11}}, ID: 16})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAdd, Operands: []ir.IrOperand{ir.IrRegister{Index: 14}, ir.IrRegister{Index: 13}, ir.IrRegister{Index: 12}}, ID: 17})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpStoreWord, Operands: []ir.IrOperand{ir.IrRegister{Index: 14}, ir.IrRegister{Index: 2}, ir.IrRegister{Index: 3}}, ID: 18})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadWord, Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrRegister{Index: 2}, ir.IrRegister{Index: 3}}, ID: 19})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "done"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpHalt, ID: 20})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_fn_helper"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 1}, ir.IrImmediate{Value: 65}}, ID: 21})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpRet, ID: 22})
	return program
}

func graalVMHomeForTests(t *testing.T) string {
	t.Helper()
	value := os.Getenv("GRAALVM_HOME")
	if value == "" {
		t.Skip("GRAALVM_HOME is not set for local runtime smoke tests")
	}
	return value
}
