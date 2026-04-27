package dartmouthbasicircompiler

import (
	"strings"
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	dartmouthbasicparser "github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-parser"
)

func compileSource(t *testing.T, source string) *CompileResult {
	t.Helper()
	ast, err := dartmouthbasicparser.ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	result, err := CompileDartmouthBasic(ast, nil)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	return result
}

func TestCompileDartmouthBasicEmitsEntryAndLineLabels(t *testing.T) {
	result := compileSource(t, "10 REM HI\n20 END\n")
	labels := map[string]bool{}
	for _, instruction := range result.Program.Instructions {
		if instruction.Opcode != ir.OpLabel {
			continue
		}
		label := instruction.Operands[0].(ir.IrLabel)
		labels[label.Name] = true
	}
	for _, want := range []string{"_start", "_line_10", "_line_20"} {
		if !labels[want] {
			t.Fatalf("expected label %q in IR", want)
		}
	}
}

func TestCompileDartmouthBasicLowersMultiplyAndDivideWithHelperLabels(t *testing.T) {
	result := compileSource(t, strings.Join([]string{
		"10 LET A = 3 * 4",
		"20 LET B = 12 / 3",
		"30 END",
	}, "\n")+"\n")
	assemblyLike := []string{}
	for _, instruction := range result.Program.Instructions {
		if instruction.Opcode == ir.OpLabel {
			assemblyLike = append(assemblyLike, instruction.Operands[0].(ir.IrLabel).Name)
		}
	}
	joined := strings.Join(assemblyLike, "\n")
	if !strings.Contains(joined, "__db_mul_loop_") {
		t.Fatalf("expected multiply helper labels, got:\n%s", joined)
	}
	if !strings.Contains(joined, "__db_div_loop_") {
		t.Fatalf("expected divide helper labels, got:\n%s", joined)
	}
}

func TestCompileDartmouthBasicCompilesIfThenBranch(t *testing.T) {
	result := compileSource(t, strings.Join([]string{
		"10 LET A = 2",
		"20 IF A = 2 THEN 40",
		"30 END",
		"40 END",
	}, "\n")+"\n")
	found := false
	for _, instruction := range result.Program.Instructions {
		if instruction.Opcode != ir.OpBranchNz {
			continue
		}
		label := instruction.Operands[1].(ir.IrLabel)
		if label.Name == "_line_40" {
			found = true
			break
		}
	}
	if !found {
		t.Fatal("expected IF ... THEN to branch to _line_40")
	}
}

func TestCompileDartmouthBasicRejectsNumericPrintForNow(t *testing.T) {
	ast, err := dartmouthbasicparser.ParseDartmouthBasic("10 PRINT 7\n20 END\n")
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	_, err = CompileDartmouthBasic(ast, nil)
	if err == nil {
		t.Fatal("expected numeric PRINT compile error")
	}
	if !strings.Contains(err.Error(), "numeric PRINT") {
		t.Fatalf("expected numeric PRINT error, got %v", err)
	}
}
