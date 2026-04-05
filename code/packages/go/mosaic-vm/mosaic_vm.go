// Package mosaicvm is a generic tree-walking driver for Mosaic compiler backends.
//
// The MosaicVM is the fourth stage of the Mosaic compiler pipeline:
//
//	Source text → Lexer → Parser → Analyzer → MosaicIR → VM → Backend → Target code
//
// The VM's responsibilities:
//  1. Traverse the MosaicIR tree depth-first.
//  2. Normalize every MosaicValue into a ResolvedValue (hex → RGBA, dimension → value+unit).
//  3. Track the slot context (component slots + active each-loop scopes).
//  4. Call MosaicRenderer methods in strict open-before-close order.
//
// The VM is agnostic about output format — it has no knowledge of React, Web
// Components, or any other platform. Backends own the output; the VM only
// drives the traversal and normalizes values.
//
// Traversal Order
// ---------------
//
// The VM visits nodes depth-first, calling beginComponent before children and
// endComponent after. The exact call sequence for a component is:
//
//	BeginComponent(name, slots)
//	  BeginNode(tag, isPrimitive, resolvedProps, ctx)
//	    // for each child of root in source order:
//	    BeginNode(child, ...) ... EndNode(child)   ← child nodes
//	    RenderSlotChild(...)                        ← @slotName; children
//	    BeginWhen(...)                              ← when blocks
//	      [when children]
//	    EndWhen()
//	    BeginEach(...)                              ← each blocks
//	      [each children — with loop scope pushed]
//	    EndEach()
//	  EndNode(root)
//	EndComponent()
//	emit() → EmitResult
//
// Usage:
//
//	ir, _ := mosaicanalyzer.Analyze(source)
//	vm := mosaicvm.New(ir)
//	result, err := vm.Run(myRenderer)
package mosaicvm

import (
	"fmt"
	"strconv"

	mosaicanalyzer "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer"
)

// ============================================================================
// Resolved Value Types
// ============================================================================

// ResolvedValue is a normalized property value after VM processing.
//
// The VM transforms raw MosaicValue instances:
//   - color_hex strings → parsed RGBA integers (Kind="color")
//   - dimension strings → separate NumValue + Unit
//   - ident values → folded into string
//   - slot_ref → enriched with type info and loop-variable flag
//
// Kind values: "string", "number", "bool", "dimension", "color", "slot_ref", "enum"
type ResolvedValue struct {
	Kind      string
	StrValue  string  // for "string", "ident" (folded into string)
	NumValue  float64 // for "number", "dimension"
	Unit      string  // for "dimension"; namespace for "enum"
	Member    string  // for "enum": the right side of the dot
	BoolValue bool    // for "bool"
	R, G, B   int     // for "color"
	A         int     // for "color"; defaults to 255
	SlotName  string  // for "slot_ref"
	SlotType  *mosaicanalyzer.MosaicType // for "slot_ref": the slot's type
	IsLoopVar bool    // for "slot_ref": true if this is a loop variable from each
}

// ResolvedProperty is a property with its value fully normalized.
type ResolvedProperty struct {
	Name  string
	Value ResolvedValue
}

// ============================================================================
// Slot Context
// ============================================================================

// LoopScope represents one active each-loop scope.
// When inside `each @items as item { ... }`, the scope allows @item to resolve.
type LoopScope struct {
	// ItemName is the loop variable name (the "item" in `each @items as item`).
	ItemName string
	// ElementType is the type of each element in the list.
	ElementType mosaicanalyzer.MosaicType
}

// SlotContext carries the slot bindings available during tree traversal.
// A new inner context is created when entering an each block; it's discarded
// when leaving (Go's value semantics handle cleanup automatically).
type SlotContext struct {
	// ComponentSlots maps slot name → MosaicSlot for the component being compiled.
	ComponentSlots map[string]*mosaicanalyzer.MosaicSlot

	// LoopScopes is the stack of active each-loop scopes, innermost last.
	LoopScopes []LoopScope
}

// ============================================================================
// Emit Result
// ============================================================================

// EmitResult is the output produced by a MosaicRenderer after `Emit()`.
type EmitResult struct {
	// FileName is the suggested output file name (e.g., "profile-card.tsx").
	FileName string

	// Code is the generated source code string.
	Code string

	// Metadata holds optional renderer-specific key-value data.
	Metadata map[string]string
}

// ============================================================================
// MosaicRenderer Interface
// ============================================================================

// MosaicRenderer is the interface that Mosaic compiler backends must implement.
//
// The VM calls these methods in depth-first order as it traverses the IR tree.
// Renderers accumulate state during the traversal, then finalize output in Emit().
type MosaicRenderer interface {
	// BeginComponent is called once at the start of traversal with the component
	// name and its declared slots. Use this to initialize the output buffer.
	BeginComponent(name string, slots []mosaicanalyzer.MosaicSlot)

	// EndComponent is called once after all nodes have been visited.
	// Use this to finalize the component wrapper (e.g., close the function body).
	EndComponent()

	// BeginNode is called when entering a node element. The tag, isPrimitive flag,
	// and fully resolved properties are provided. ctx carries the current slot bindings.
	BeginNode(tag string, isPrimitive bool, props []ResolvedProperty, ctx SlotContext)

	// EndNode is called when leaving a node element (after all children are visited).
	EndNode(tag string)

	// RenderSlotChild is called for slot references used as children (`@slotName;`).
	// The slotType describes what kind of content to project.
	RenderSlotChild(slotName string, slotType mosaicanalyzer.MosaicType, ctx SlotContext)

	// BeginWhen is called when entering a conditional block (`when @show { ... }`).
	// The slotName is the bool slot controlling visibility.
	BeginWhen(slotName string, ctx SlotContext)

	// EndWhen is called after all children of a when block have been visited.
	EndWhen()

	// BeginEach is called when entering an iteration block (`each @items as item { ... }`).
	// elementType is the type of each loop element (the T in list<T>).
	BeginEach(slotName string, itemName string, elementType mosaicanalyzer.MosaicType, ctx SlotContext)

	// EndEach is called after all children of an each block have been visited.
	EndEach()

	// Emit finalizes the output and returns the generated code.
	Emit() EmitResult
}

// ============================================================================
// MosaicVM
// ============================================================================

// MosaicVMError is returned when the VM encounters a runtime invariant violation.
// These indicate a bug in the analyzer (undefined slot or invalid type reference).
type MosaicVMError struct {
	Message string
}

func (e *MosaicVMError) Error() string {
	return fmt.Sprintf("mosaic vm error: %s", e.Message)
}

// MosaicVM is the generic tree-walking driver for Mosaic compiler backends.
//
// Construct a VM with a MosaicIR, then call Run(renderer) with any backend
// that implements MosaicRenderer. The VM returns the EmitResult from the renderer.
//
// A single MosaicVM instance can be run against multiple renderers — one for
// React, another for Web Components, etc. The VM is stateless between Run() calls.
type MosaicVM struct {
	ir *mosaicanalyzer.MosaicIR
}

// New creates a new MosaicVM for the given MosaicIR.
func New(ir *mosaicanalyzer.MosaicIR) *MosaicVM {
	return &MosaicVM{ir: ir}
}

// Run traverses the IR tree, calling renderer methods in depth-first order.
// Returns the EmitResult produced by renderer.Emit().
func (vm *MosaicVM) Run(renderer MosaicRenderer) (EmitResult, error) {
	// Build the root SlotContext from the component's slot declarations.
	// componentSlots is a map for O(1) lookups during traversal.
	componentSlots := make(map[string]*mosaicanalyzer.MosaicSlot)
	for i := range vm.ir.Component.Slots {
		s := &vm.ir.Component.Slots[i]
		componentSlots[s.Name] = s
	}

	ctx := SlotContext{
		ComponentSlots: componentSlots,
		LoopScopes:     []LoopScope{},
	}

	renderer.BeginComponent(vm.ir.Component.Name, vm.ir.Component.Slots)
	if err := vm.walkNode(vm.ir.Component.Tree, ctx, renderer); err != nil {
		return EmitResult{}, err
	}
	renderer.EndComponent()
	return renderer.Emit(), nil
}

// walkNode traverses a single node: resolves properties, calls BeginNode,
// walks children, calls EndNode.
func (vm *MosaicVM) walkNode(node *mosaicanalyzer.MosaicNode, ctx SlotContext, r MosaicRenderer) error {
	// Resolve all properties before calling BeginNode so the renderer
	// receives fully normalized values.
	resolved := make([]ResolvedProperty, 0, len(node.Properties))
	for _, p := range node.Properties {
		rv, err := vm.resolveValue(p.Value, ctx)
		if err != nil {
			return err
		}
		resolved = append(resolved, ResolvedProperty{Name: p.Name, Value: rv})
	}

	r.BeginNode(node.Tag, node.IsPrimitive, resolved, ctx)

	for _, child := range node.Children {
		if err := vm.walkChild(child, ctx, r); err != nil {
			return err
		}
	}

	r.EndNode(node.Tag)
	return nil
}

// walkChild dispatches a single child to the appropriate renderer method.
func (vm *MosaicVM) walkChild(child mosaicanalyzer.MosaicChild, ctx SlotContext, r MosaicRenderer) error {
	switch child.Kind {
	case "node":
		return vm.walkNode(child.Node, ctx, r)

	case "slot_ref":
		// Slot used as a child: Column { @header; }
		// Look up the slot type so the renderer knows what kind of content to project.
		slot := vm.resolveSlot(child.SlotName, ctx)
		if slot == nil {
			return &MosaicVMError{Message: fmt.Sprintf("unknown slot: @%s", child.SlotName)}
		}
		r.RenderSlotChild(child.SlotName, slot.Type, ctx)

	case "when":
		// Conditional block: when @show { ... }
		r.BeginWhen(child.SlotName, ctx)
		for _, bodyNode := range child.Body {
			if err := vm.walkNode(bodyNode, ctx, r); err != nil {
				return err
			}
		}
		r.EndWhen()

	case "each":
		// Iteration block: each @items as item { ... }
		// 1. Find the list slot and its element type.
		listSlot := ctx.ComponentSlots[child.SlotName]
		if listSlot == nil {
			return &MosaicVMError{Message: fmt.Sprintf("unknown list slot: @%s", child.SlotName)}
		}
		if listSlot.Type.Kind != "list" {
			return &MosaicVMError{Message: fmt.Sprintf(
				"each block references @%s but it is not a list type", child.SlotName)}
		}
		elementType := *listSlot.Type.Element

		// 2. Call BeginEach on the renderer.
		r.BeginEach(child.SlotName, child.ItemName, elementType, ctx)

		// 3. Build inner context with loop scope pushed.
		innerCtx := SlotContext{
			ComponentSlots: ctx.ComponentSlots,
			LoopScopes:     append(append([]LoopScope{}, ctx.LoopScopes...), LoopScope{
				ItemName:    child.ItemName,
				ElementType: elementType,
			}),
		}

		// 4. Walk the body with the loop scope active.
		for _, bodyNode := range child.Body {
			if err := vm.walkNode(bodyNode, innerCtx, r); err != nil {
				return err
			}
		}

		// 5. Close the each block.
		r.EndEach()
	}
	return nil
}

// resolveValue normalizes a MosaicValue into a ResolvedValue.
//
// The main transformations are:
//   - color_hex → parsed RGBA integers
//   - dimension → {NumValue, Unit}
//   - ident → folded into string
//   - slot_ref → enriched with slot type info and loop-variable flag
//   - enum → {StrValue=namespace, Member=member}
func (vm *MosaicVM) resolveValue(v mosaicanalyzer.MosaicValue, ctx SlotContext) (ResolvedValue, error) {
	switch v.Kind {
	case "string":
		return ResolvedValue{Kind: "string", StrValue: v.StrValue}, nil
	case "number":
		return ResolvedValue{Kind: "number", NumValue: v.NumValue}, nil
	case "bool":
		return ResolvedValue{Kind: "bool", BoolValue: v.BoolValue}, nil
	case "ident":
		// Bare identifiers (e.g., align: center) fold into "string" for the renderer.
		return ResolvedValue{Kind: "string", StrValue: v.StrValue}, nil
	case "dimension":
		return ResolvedValue{Kind: "dimension", NumValue: v.NumValue, Unit: v.Unit}, nil
	case "color_hex":
		return parseColor(v.StrValue)
	case "enum":
		return ResolvedValue{Kind: "enum", StrValue: v.StrValue, Member: v.Unit}, nil
	case "slot_ref":
		return vm.resolveSlotRef(v.SlotName, ctx)
	}
	return ResolvedValue{}, &MosaicVMError{Message: fmt.Sprintf("unknown value kind: %q", v.Kind)}
}

// parseColor parses a hex color string into RGBA integer components.
//
// Three-digit hex: #rgb → doubles each digit → #rrggbb (alpha=255)
// Six-digit hex: #rrggbb → alpha=255
// Eight-digit hex: #rrggbbaa → all four channels explicit
func parseColor(hex string) (ResolvedValue, error) {
	h := hex[1:] // strip leading '#'
	r, g, b, a := 0, 0, 0, 255

	parseHex := func(s string) (int, error) {
		n, err := strconv.ParseInt(s, 16, 64)
		return int(n), err
	}

	switch len(h) {
	case 3:
		// Three-digit shorthand: #rgb → #rrggbb
		rv, err := parseHex(string(h[0]) + string(h[0]))
		if err != nil {
			return ResolvedValue{}, err
		}
		gv, err := parseHex(string(h[1]) + string(h[1]))
		if err != nil {
			return ResolvedValue{}, err
		}
		bv, err := parseHex(string(h[2]) + string(h[2]))
		if err != nil {
			return ResolvedValue{}, err
		}
		r, g, b = rv, gv, bv
	case 6:
		rv, err := parseHex(h[0:2])
		if err != nil {
			return ResolvedValue{}, err
		}
		gv, err := parseHex(h[2:4])
		if err != nil {
			return ResolvedValue{}, err
		}
		bv, err := parseHex(h[4:6])
		if err != nil {
			return ResolvedValue{}, err
		}
		r, g, b = rv, gv, bv
	case 8:
		rv, err := parseHex(h[0:2])
		if err != nil {
			return ResolvedValue{}, err
		}
		gv, err := parseHex(h[2:4])
		if err != nil {
			return ResolvedValue{}, err
		}
		bv, err := parseHex(h[4:6])
		if err != nil {
			return ResolvedValue{}, err
		}
		av, err := parseHex(h[6:8])
		if err != nil {
			return ResolvedValue{}, err
		}
		r, g, b, a = rv, gv, bv, av
	default:
		return ResolvedValue{}, &MosaicVMError{Message: fmt.Sprintf("invalid color hex: %s", hex)}
	}

	return ResolvedValue{Kind: "color", R: r, G: g, B: b, A: a}, nil
}

// resolveSlotRef looks up a slot reference in the current context.
// It checks loop scopes innermost-first, then falls back to component slots.
func (vm *MosaicVM) resolveSlotRef(slotName string, ctx SlotContext) (ResolvedValue, error) {
	// 1. Check active loop scopes, innermost first.
	for i := len(ctx.LoopScopes) - 1; i >= 0; i-- {
		scope := ctx.LoopScopes[i]
		if scope.ItemName == slotName {
			return ResolvedValue{
				Kind:      "slot_ref",
				SlotName:  slotName,
				SlotType:  &scope.ElementType,
				IsLoopVar: true,
			}, nil
		}
	}

	// 2. Fall back to component slots.
	slot := ctx.ComponentSlots[slotName]
	if slot == nil {
		return ResolvedValue{}, &MosaicVMError{Message: fmt.Sprintf("unresolved slot reference: @%s", slotName)}
	}
	return ResolvedValue{
		Kind:      "slot_ref",
		SlotName:  slotName,
		SlotType:  &slot.Type,
		IsLoopVar: false,
	}, nil
}

// resolveSlot looks up a named slot for RenderSlotChild.
// Returns nil if the slot is not found.
func (vm *MosaicVM) resolveSlot(slotName string, ctx SlotContext) *mosaicanalyzer.MosaicSlot {
	return ctx.ComponentSlots[slotName]
}
