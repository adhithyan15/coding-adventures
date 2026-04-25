package registervm

import "fmt"

// NewContext creates a lexical context with slot_count undefined slots.
func NewContext(parent *Context, slotCount int) *Context {
	if slotCount < 0 {
		slotCount = 0
	}
	slots := make([]VMValue, slotCount)
	for i := range slots {
		slots[i] = Undefined
	}
	return &Context{Slots: slots, Parent: parent}
}

// GetSlot reads a value from a context at depth/index.
func GetSlot(ctx *Context, depth int, index int) (VMValue, error) {
	target, err := walkContext(ctx, depth)
	if err != nil {
		return nil, err
	}
	if index < 0 || index >= len(target.Slots) {
		return nil, fmt.Errorf("context slot index %d out of range", index)
	}
	return target.Slots[index], nil
}

// SetSlot writes a value into a context at depth/index.
func SetSlot(ctx *Context, depth int, index int, value VMValue) error {
	target, err := walkContext(ctx, depth)
	if err != nil {
		return err
	}
	if index < 0 || index >= len(target.Slots) {
		return fmt.Errorf("context slot index %d out of range", index)
	}
	target.Slots[index] = value
	return nil
}

func walkContext(ctx *Context, depth int) (*Context, error) {
	if ctx == nil {
		return nil, fmt.Errorf("context is nil")
	}
	if depth < 0 {
		return nil, fmt.Errorf("context depth %d out of range", depth)
	}
	current := ctx
	for i := 0; i < depth; i++ {
		if current.Parent == nil {
			return nil, fmt.Errorf("context chain shorter than depth %d", depth)
		}
		current = current.Parent
	}
	return current, nil
}
