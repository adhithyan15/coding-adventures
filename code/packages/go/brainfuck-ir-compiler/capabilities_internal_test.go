package brainfuckircompiler

import (
	"errors"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

func TestResultFactoryHelpers(t *testing.T) {
	rf := &ResultFactory[int]{}

	okResult := rf.Generate(true, false, 7)
	if !okResult.DidSucceed || okResult.ReturnValue != 7 {
		t.Fatalf("unexpected generated result: %+v", okResult)
	}

	failErr := errors.New("boom")
	failResult := rf.Fail(9, failErr)
	if failResult.DidSucceed || failResult.Err != failErr || failResult.ReturnValue != 9 {
		t.Fatalf("unexpected failed result: %+v", failResult)
	}
}

func TestOperationGetResultSuccessAndProperties(t *testing.T) {
	op := StartNew("success", 0, func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
		if got := op.propertyBag["mode"]; got != "test" {
			t.Fatalf("expected property bag to contain mode=test, got %v", got)
		}
		return rf.Generate(true, false, 42)
	})
	op.AddProperty("mode", "test")

	got, err := op.GetResult()
	if err != nil {
		t.Fatalf("expected success, got %v", err)
	}
	if got != 42 {
		t.Fatalf("expected 42, got %d", got)
	}
}

func TestOperationGetResultErrorAndPanicPaths(t *testing.T) {
	expectedErr := errors.New("expected failure")
	failOp := StartNew("fail", 1, func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
		return rf.Fail(5, expectedErr)
	})

	got, err := failOp.GetResult()
	if !errors.Is(err, expectedErr) || got != 5 {
		t.Fatalf("expected explicit failure, got value=%d err=%v", got, err)
	}

	panicOp := StartNew("panic", 11, func(_ *Operation[int], _ *ResultFactory[int]) *OperationResult[int] {
		panic("unexpected")
	})
	got, err = panicOp.GetResult()
	if got != 11 || err == nil || !strings.Contains(err.Error(), "failed unexpectedly") {
		t.Fatalf("expected panic fallback and unexpected error, got value=%d err=%v", got, err)
	}

	repanicOp := StartNew("panic-repanic", 13, func(_ *Operation[int], _ *ResultFactory[int]) *OperationResult[int] {
		panic("repanic")
	}).PanicOnUnexpected()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected PanicOnUnexpected to repanic")
		}
	}()
	_, _ = repanicOp.GetResult()
}

func TestCapabilityViolationErrorMessage(t *testing.T) {
	err := (&_capabilityViolationError{
		category:  "parser",
		action:    "read",
		requested: "disk",
	}).Error()

	if !strings.Contains(err, "capability violation: parser:read") || !strings.Contains(err, "\"disk\"") {
		t.Fatalf("unexpected error string: %s", err)
	}
}

func TestExtractTokenFindsLeafAndRawToken(t *testing.T) {
	c := &compiler{}
	tok := lexer.Token{Type: lexer.TokenPlus, Value: "+"}

	leaf := &parser.ASTNode{RuleName: "command", Children: []interface{}{tok}}
	if got := c.extractToken(leaf); got == nil || got.Value != "+" {
		t.Fatalf("expected direct leaf token, got %#v", got)
	}

	nested := &parser.ASTNode{
		RuleName: "wrapper",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "inner",
				Children: []interface{}{tok},
			},
		},
	}
	if got := c.extractToken(nested); got == nil || got.Value != "+" {
		t.Fatalf("expected nested token, got %#v", got)
	}

	raw := &parser.ASTNode{RuleName: "raw", Children: []interface{}{tok, "ignored"}}
	if got := c.extractToken(raw); got == nil || got.Value != "+" {
		t.Fatalf("expected raw child token, got %#v", got)
	}

	empty := &parser.ASTNode{RuleName: "empty"}
	if got := c.extractToken(empty); got != nil {
		t.Fatalf("expected nil token for empty node, got %#v", got)
	}
}
