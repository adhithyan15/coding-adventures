package garbagecollector

import (
	"reflect"
	"testing"
)

func TestAllocateAndDeref(t *testing.T) {
	gc := NewMarkAndSweepGC()

	first := gc.Allocate(NewConsCell(42, nil))
	second := gc.Allocate(NewSymbol("next"))

	if first != InitialAddress || second != InitialAddress+1 {
		t.Fatalf("expected monotonic addresses from %#x, got %#x and %#x", InitialAddress, first, second)
	}
	if gc.HeapSize() != 2 {
		t.Fatalf("expected heap size 2, got %d", gc.HeapSize())
	}

	obj, err := gc.Deref(first)
	if err != nil {
		t.Fatalf("deref failed: %v", err)
	}
	if obj.TypeName() != "ConsCell" {
		t.Fatalf("expected ConsCell, got %s", obj.TypeName())
	}
	if !gc.IsValidAddress(second) {
		t.Fatal("second allocation should be valid")
	}
}

func TestDerefInvalidOrFreedObject(t *testing.T) {
	gc := NewMarkAndSweepGC()
	address := gc.Allocate(NewSymbol("gone"))

	if _, err := gc.Deref(address); err != nil {
		t.Fatalf("fresh address should dereference: %v", err)
	}
	if freed := gc.Collect(nil); freed != 1 {
		t.Fatalf("expected 1 freed object, got %d", freed)
	}
	if _, err := gc.Deref(address); err == nil {
		t.Fatal("expected deref of freed address to fail")
	}
	if gc.IsValidAddress(address) {
		t.Fatal("freed address should not be valid")
	}
}

func TestCollectKeepsTransitiveReferencesAlive(t *testing.T) {
	gc := NewMarkAndSweepGC()
	tail := gc.Allocate(NewSymbol("tail"))
	middle := gc.Allocate(NewConsCell(tail, nil))
	head := gc.Allocate(NewConsCell(1, middle))

	if freed := gc.Collect([]any{head}); freed != 0 {
		t.Fatalf("expected no freed objects, got %d", freed)
	}
	if gc.HeapSize() != 3 {
		t.Fatalf("expected heap size 3, got %d", gc.HeapSize())
	}
	if !gc.IsValidAddress(tail) {
		t.Fatal("tail should remain live")
	}
}

func TestCollectsUnreachableCycles(t *testing.T) {
	gc := NewMarkAndSweepGC()
	left := NewConsCell(nil, nil)
	right := NewConsCell(nil, nil)
	leftAddress := gc.Allocate(left)
	rightAddress := gc.Allocate(right)
	left.Cdr = rightAddress
	right.Cdr = leftAddress

	if freed := gc.Collect(nil); freed != 2 {
		t.Fatalf("expected 2 freed objects, got %d", freed)
	}
	if gc.HeapSize() != 0 {
		t.Fatalf("expected empty heap, got %d", gc.HeapSize())
	}
}

func TestScansNestedRootArraysAndMaps(t *testing.T) {
	gc := NewMarkAndSweepGC()
	fromSlice := gc.Allocate(NewSymbol("slice-root"))
	fromMap := gc.Allocate(NewSymbol("map-root"))
	unreachable := gc.Allocate(NewSymbol("unreachable"))

	roots := []any{[]int{fromSlice}, map[string]any{"global": fromMap, "literal": 42}}
	if freed := gc.Collect(roots); freed != 1 {
		t.Fatalf("expected one freed object, got %d", freed)
	}
	if !gc.IsValidAddress(fromSlice) || !gc.IsValidAddress(fromMap) {
		t.Fatal("nested roots should remain live")
	}
	if gc.IsValidAddress(unreachable) {
		t.Fatal("unreachable object should be collected")
	}
}

func TestStats(t *testing.T) {
	gc := NewMarkAndSweepGC()
	root := gc.Allocate(NewSymbol("root"))
	gc.Allocate(NewSymbol("temp"))

	if freed := gc.Collect([]any{root}); freed != 1 {
		t.Fatalf("expected 1 freed object, got %d", freed)
	}

	expected := GCStats{TotalAllocations: 2, TotalCollections: 1, TotalFreed: 1, HeapSize: 1}
	if stats := gc.Stats(); stats != expected {
		t.Fatalf("expected stats %+v, got %+v", expected, stats)
	}
}

func TestHeapObjectReferences(t *testing.T) {
	closure := NewLispClosure("lambda", map[string]any{"x": InitialAddress, "y": "plain", "z": 17.5}, []string{"arg"})

	if refs := NewConsCell(InitialAddress, "tail").References(); !reflect.DeepEqual(refs, []int{InitialAddress}) {
		t.Fatalf("unexpected cons refs: %#v", refs)
	}
	if refs := NewSymbol("plain").References(); refs != nil {
		t.Fatalf("symbol should not report refs, got %#v", refs)
	}
	if refs := closure.References(); !reflect.DeepEqual(refs, []int{InitialAddress}) {
		t.Fatalf("unexpected closure refs: %#v", refs)
	}
	if closure.TypeName() != "LispClosure" || closure.Code != "lambda" || len(closure.Params) != 1 {
		t.Fatal("closure fields were not preserved")
	}
}

func TestSymbolTableInternLookupAndAllSymbols(t *testing.T) {
	gc := NewMarkAndSweepGC()
	table := NewSymbolTable(gc)

	first := table.Intern("foo")
	second := table.Intern("foo")
	other := table.Intern("bar")

	if first != second {
		t.Fatalf("same symbol name should reuse address: %d != %d", first, second)
	}
	if first == other {
		t.Fatalf("different symbol names should not share address: %d", first)
	}
	if address, ok := table.Lookup("foo"); !ok || address != first {
		t.Fatalf("lookup returned %d, %v; expected %d, true", address, ok, first)
	}

	expected := map[string]int{"foo": first, "bar": other}
	if symbols := table.AllSymbols(); !reflect.DeepEqual(symbols, expected) {
		t.Fatalf("expected symbols %#v, got %#v", expected, symbols)
	}
}

func TestSymbolTableReallocatesAfterCollection(t *testing.T) {
	gc := NewMarkAndSweepGC()
	table := NewSymbolTable(gc)
	original := table.Intern("foo")

	if freed := gc.Collect(nil); freed != 1 {
		t.Fatalf("expected one symbol to be collected, got %d", freed)
	}
	if _, ok := table.Lookup("foo"); ok {
		t.Fatal("lookup should ignore collected symbols")
	}

	fresh := table.Intern("foo")
	if fresh == original {
		t.Fatalf("expected a fresh address after collection, got %d", fresh)
	}
	expected := map[string]int{"foo": fresh}
	if symbols := table.AllSymbols(); !reflect.DeepEqual(symbols, expected) {
		t.Fatalf("expected symbols %#v, got %#v", expected, symbols)
	}
}
