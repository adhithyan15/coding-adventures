package dartmouthbasicriscvcompiler

import (
	"strings"
	"testing"
)

func TestCompileSourceReturnsPipelineArtifacts(t *testing.T) {
	result, err := CompileSource("10 PRINT \"HI\"\n20 END\n")
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
	if !strings.Contains(result.Assembly, "_line_10:") {
		t.Fatalf("expected assembly to contain BASIC line label, got:\n%s", result.Assembly)
	}
	if len(result.Binary) == 0 {
		t.Fatal("expected RISC-V binary bytes")
	}
}

func TestRunSourceExecutesStringPrintOnRiscVSimulator(t *testing.T) {
	run, err := RunSource("10 PRINT \"HI\"\n20 END\n")
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if !run.Halted {
		t.Fatal("expected simulator to halt")
	}
	if run.Output != "HI\n" {
		t.Fatalf("expected output %q, got %q", "HI\n", run.Output)
	}
}

func TestRunSourceExecutesMultiplyAndIfProgram(t *testing.T) {
	source := strings.Join([]string{
		"10 LET A = 2 * 3",
		"20 IF A = 6 THEN 50",
		"30 PRINT \"BAD\"",
		"40 END",
		"50 PRINT \"OK\"",
		"60 END",
	}, "\n") + "\n"
	run, err := RunSource(source)
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if !run.Halted {
		t.Fatal("expected simulator to halt")
	}
	if run.Output != "OK\n" {
		t.Fatalf("expected output %q, got %q", "OK\n", run.Output)
	}
}

func TestRunSourceExecutesGotoAndDivisionProgram(t *testing.T) {
	source := strings.Join([]string{
		"10 GOTO 40",
		"20 PRINT \"BAD\"",
		"30 END",
		"40 LET A = 8 / 2",
		"50 IF A = 4 THEN 70",
		"60 END",
		"70 PRINT \"DIV\"",
		"80 END",
	}, "\n") + "\n"
	run, err := RunSource(source)
	if err != nil {
		t.Fatalf("run failed: %v", err)
	}
	if !run.Halted {
		t.Fatal("expected simulator to halt")
	}
	if run.Output != "DIV\n" {
		t.Fatalf("expected output %q, got %q", "DIV\n", run.Output)
	}
}

func TestParseErrorsReportStage(t *testing.T) {
	_, err := CompileSource("10 PRINT")
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

func TestLowerBasicErrorsReportStage(t *testing.T) {
	_, err := CompileSource("10 PRINT 7\n20 END\n")
	if err == nil {
		t.Fatal("expected lower-basic failure")
	}
	packageErr, ok := err.(*PackageError)
	if !ok {
		t.Fatalf("expected PackageError, got %T", err)
	}
	if packageErr.Stage != "lower-basic" {
		t.Fatalf("expected lower-basic stage, got %q", packageErr.Stage)
	}
}
