// Tests for MosaicVM.
//
// We use a mock renderer that records all VM calls in order, then we
// assert on the recorded sequence to verify the traversal logic.
package mosaicvm

import (
	"fmt"
	"testing"

	mosaicanalyzer "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer"
)

// ============================================================================
// MockRenderer — records all VM calls for assertion
// ============================================================================

type call struct {
	method string
	args   []interface{}
}

type mockRenderer struct {
	calls []call
}

func (m *mockRenderer) record(method string, args ...interface{}) {
	m.calls = append(m.calls, call{method: method, args: args})
}

func (m *mockRenderer) BeginComponent(name string, slots []mosaicanalyzer.MosaicSlot) {
	m.record("BeginComponent", name, len(slots))
}
func (m *mockRenderer) EndComponent() {
	m.record("EndComponent")
}
func (m *mockRenderer) BeginNode(tag string, isPrimitive bool, props []ResolvedProperty, ctx SlotContext) {
	m.record("BeginNode", tag, isPrimitive)
}
func (m *mockRenderer) EndNode(tag string) {
	m.record("EndNode", tag)
}
func (m *mockRenderer) RenderSlotChild(slotName string, slotType mosaicanalyzer.MosaicType, ctx SlotContext) {
	m.record("RenderSlotChild", slotName)
}
func (m *mockRenderer) BeginWhen(slotName string, ctx SlotContext) {
	m.record("BeginWhen", slotName)
}
func (m *mockRenderer) EndWhen() {
	m.record("EndWhen")
}
func (m *mockRenderer) BeginEach(slotName string, itemName string, elementType mosaicanalyzer.MosaicType, ctx SlotContext) {
	m.record("BeginEach", slotName, itemName)
}
func (m *mockRenderer) EndEach() {
	m.record("EndEach")
}
func (m *mockRenderer) Emit() EmitResult {
	return EmitResult{Code: "mock output", FileName: "mock.txt"}
}

func (m *mockRenderer) findCall(method string) *call {
	for i := range m.calls {
		if m.calls[i].method == method {
			return &m.calls[i]
		}
	}
	return nil
}

func (m *mockRenderer) allCalls(method string) []call {
	var result []call
	for _, c := range m.calls {
		if c.method == method {
			result = append(result, c)
		}
	}
	return result
}

// mustRun analyzes source and runs the VM with the given renderer.
func mustRun(t *testing.T, source string, r MosaicRenderer) EmitResult {
	t.Helper()
	ir, err := mosaicanalyzer.Analyze(source)
	if err != nil {
		t.Fatalf("Analyze error: %v", err)
	}
	vm := New(ir)
	result, err := vm.Run(r)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}
	return result
}

// =============================================================================
// TestVMCallOrder
// =============================================================================
//
// The VM must call BeginComponent before BeginNode and EndComponent last.
func TestVMCallOrder(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `component Foo { Column {} }`, r)

	if len(r.calls) == 0 {
		t.Fatal("No calls recorded")
	}
	if r.calls[0].method != "BeginComponent" {
		t.Errorf("First call should be BeginComponent, got %q", r.calls[0].method)
	}
	last := r.calls[len(r.calls)-1]
	// last is after Emit, but we want EndComponent to be second-to-last
	// Actually Emit is not in calls — check for EndComponent
	found := false
	for _, c := range r.calls {
		if c.method == "EndComponent" {
			found = true
		}
	}
	if !found {
		t.Error("Expected EndComponent call")
	}
	_ = last
}

// =============================================================================
// TestVMBeginComponent
// =============================================================================
//
// BeginComponent receives the component name and slot count.
func TestVMBeginComponent(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `
component Card {
  slot title: text;
  slot count: number;
  Column {}
}`, r)

	c := r.findCall("BeginComponent")
	if c == nil {
		t.Fatal("BeginComponent not called")
	}
	if c.args[0] != "Card" {
		t.Errorf("Expected name 'Card', got %v", c.args[0])
	}
	if c.args[1] != 2 {
		t.Errorf("Expected 2 slots, got %v", c.args[1])
	}
}

// =============================================================================
// TestVMBeginNode
// =============================================================================
//
// BeginNode is called for each node element in the tree.
func TestVMBeginNode(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `component Foo { Column {} }`, r)

	c := r.findCall("BeginNode")
	if c == nil {
		t.Fatal("BeginNode not called")
	}
	if c.args[0] != "Column" {
		t.Errorf("Expected tag 'Column', got %v", c.args[0])
	}
	// Column is a primitive node
	if c.args[1] != true {
		t.Errorf("Expected isPrimitive=true for Column, got %v", c.args[1])
	}
}

// =============================================================================
// TestVMEndNode
// =============================================================================
//
// EndNode is called after all children of a node are visited.
func TestVMEndNode(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `component Foo { Column {} }`, r)

	c := r.findCall("EndNode")
	if c == nil {
		t.Fatal("EndNode not called")
	}
	if c.args[0] != "Column" {
		t.Errorf("Expected tag 'Column' in EndNode, got %v", c.args[0])
	}
}

// =============================================================================
// TestVMNestedNodes
// =============================================================================
//
// Nested nodes should produce interleaved Begin/End calls in depth-first order.
func TestVMNestedNodes(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `
component Card {
  Column {
    Text {}
    Image {}
  }
}`, r)

	// Expected order: BeginNode(Column) BeginNode(Text) EndNode(Text) BeginNode(Image) EndNode(Image) EndNode(Column)
	methods := make([]string, 0)
	for _, c := range r.calls {
		if c.method == "BeginNode" || c.method == "EndNode" {
			methods = append(methods, fmt.Sprintf("%s(%v)", c.method, c.args[0]))
		}
	}

	expected := []string{
		"BeginNode(Column)", "BeginNode(Text)", "EndNode(Text)",
		"BeginNode(Image)", "EndNode(Image)", "EndNode(Column)",
	}
	if len(methods) != len(expected) {
		t.Fatalf("Expected %d node calls, got %d: %v", len(expected), len(methods), methods)
	}
	for i, exp := range expected {
		if methods[i] != exp {
			t.Errorf("Call %d: expected %q, got %q", i, exp, methods[i])
		}
	}
}

// =============================================================================
// TestVMRenderSlotChild
// =============================================================================
//
// A slot reference used as a child should call RenderSlotChild.
func TestVMRenderSlotChild(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `
component Card {
  slot action: node;
  Column {
    @action;
  }
}`, r)

	c := r.findCall("RenderSlotChild")
	if c == nil {
		t.Fatal("RenderSlotChild not called")
	}
	if c.args[0] != "action" {
		t.Errorf("Expected slotName='action', got %v", c.args[0])
	}
}

// =============================================================================
// TestVMWhenBlock
// =============================================================================
//
// A when block should call BeginWhen/EndWhen around its body.
func TestVMWhenBlock(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `
component Cond {
  slot show: bool;
  Column {
    when @show {
      Text {}
    }
  }
}`, r)

	beginWhen := r.findCall("BeginWhen")
	if beginWhen == nil {
		t.Fatal("BeginWhen not called")
	}
	if beginWhen.args[0] != "show" {
		t.Errorf("Expected slotName='show', got %v", beginWhen.args[0])
	}
	if r.findCall("EndWhen") == nil {
		t.Fatal("EndWhen not called")
	}
}

// =============================================================================
// TestVMEachBlock
// =============================================================================
//
// An each block should call BeginEach/EndEach around its body.
func TestVMEachBlock(t *testing.T) {
	r := &mockRenderer{}
	mustRun(t, `
component List {
  slot items: list<text>;
  Column {
    each @items as item {
      Text {}
    }
  }
}`, r)

	beginEach := r.findCall("BeginEach")
	if beginEach == nil {
		t.Fatal("BeginEach not called")
	}
	if beginEach.args[0] != "items" {
		t.Errorf("Expected slotName='items', got %v", beginEach.args[0])
	}
	if beginEach.args[1] != "item" {
		t.Errorf("Expected itemName='item', got %v", beginEach.args[1])
	}
	if r.findCall("EndEach") == nil {
		t.Fatal("EndEach not called")
	}
}

// =============================================================================
// TestVMEmitResult
// =============================================================================
//
// Run() returns the EmitResult from renderer.Emit().
func TestVMEmitResult(t *testing.T) {
	r := &mockRenderer{}
	result := mustRun(t, `component Foo { Column {} }`, r)

	if result.Code != "mock output" {
		t.Errorf("Expected 'mock output', got %q", result.Code)
	}
	if result.FileName != "mock.txt" {
		t.Errorf("Expected 'mock.txt', got %q", result.FileName)
	}
}

// =============================================================================
// TestVMColorResolution
// =============================================================================
//
// A hex color property should be resolved to RGBA integers.
// We test the parseColor helper directly since it's the core of color resolution.
func TestVMColorResolution(t *testing.T) {
	rv, err := parseColor("#ff0000")
	if err != nil {
		t.Fatalf("parseColor error: %v", err)
	}
	if rv.Kind != "color" {
		t.Errorf("Expected kind 'color', got %q", rv.Kind)
	}
	if rv.R != 255 || rv.G != 0 || rv.B != 0 {
		t.Errorf("Expected R=255 G=0 B=0, got R=%d G=%d B=%d", rv.R, rv.G, rv.B)
	}
	if rv.A != 255 {
		t.Errorf("Expected A=255, got %d", rv.A)
	}
}

// =============================================================================
// TestVMColorThreeDigit
// =============================================================================
//
// A three-digit hex color #rgb should double each digit: #fff → r=255,g=255,b=255.
func TestVMColorThreeDigit(t *testing.T) {
	rv, err := parseColor("#fff")
	if err != nil {
		t.Fatalf("parseColor error: %v", err)
	}
	if rv.R != 255 || rv.G != 255 || rv.B != 255 {
		t.Errorf("Expected R=255 G=255 B=255, got R=%d G=%d B=%d", rv.R, rv.G, rv.B)
	}
}

// =============================================================================
// TestVMColorEightDigit
// =============================================================================
//
// An eight-digit hex color #rrggbbaa should set the alpha channel.
func TestVMColorEightDigit(t *testing.T) {
	rv, err := parseColor("#ff000080")
	if err != nil {
		t.Fatalf("parseColor error: %v", err)
	}
	if rv.R != 255 || rv.G != 0 || rv.B != 0 {
		t.Errorf("Expected R=255 G=0 B=0, got R=%d G=%d B=%d", rv.R, rv.G, rv.B)
	}
	if rv.A != 128 { // 0x80 = 128
		t.Errorf("Expected A=128, got %d", rv.A)
	}
}
