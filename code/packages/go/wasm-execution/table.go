// table.go --- WASM table implementation for indirect function calls.
//
// A WASM table is an array of opaque references — in WASM 1.0, these are
// always function references (funcref).  Tables enable indirect function
// calls via call_indirect: look up a function by index in the table at
// runtime, verify its type, and call it.
//
// Table entries are either a valid function index or -1 (uninitialized).
// Accessing an uninitialized entry via call_indirect causes a trap.
package wasmexecution

import "fmt"

// Table is a resizable array of nullable function indices.
type Table struct {
	elements []int // -1 means uninitialized (null)
	maxSize  int   // -1 means no limit
}

// NewTable creates a table with initialSize entries, all set to -1 (null).
func NewTable(initialSize int, maxSize int) *Table {
	elements := make([]int, initialSize)
	for i := range elements {
		elements[i] = -1
	}
	return &Table{elements: elements, maxSize: maxSize}
}

// Get returns the function index at the given table index.
// Returns -1 if the entry is uninitialized.
// Panics with TrapError if the index is out of bounds.
func (t *Table) Get(index int) int {
	if index < 0 || index >= len(t.elements) {
		panic(NewTrapError(fmt.Sprintf(
			"out of bounds table access: index=%d, table size=%d",
			index, len(t.elements))))
	}
	return t.elements[index]
}

// Set stores a function index at the given table index.
// Use funcIndex = -1 to clear the entry.
func (t *Table) Set(index int, funcIndex int) {
	if index < 0 || index >= len(t.elements) {
		panic(NewTrapError(fmt.Sprintf(
			"out of bounds table access: index=%d, table size=%d",
			index, len(t.elements))))
	}
	t.elements[index] = funcIndex
}

// Size returns the current table size.
func (t *Table) Size() int {
	return len(t.elements)
}

// Grow adds delta entries (initialized to -1).
// Returns the old size on success, or -1 on failure.
func (t *Table) Grow(delta int) int {
	oldSize := len(t.elements)
	newSize := oldSize + delta
	if t.maxSize >= 0 && newSize > t.maxSize {
		return -1
	}
	for i := 0; i < delta; i++ {
		t.elements = append(t.elements, -1)
	}
	return oldSize
}
