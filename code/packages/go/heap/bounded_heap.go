package heap

import "errors"

// The checked API reports invalid construction and bounded-capacity failures
// as stable errors so callers can branch with errors.Is.
var (
	ErrNegativeLimit     = errors.New("heap: limit must not be negative")
	ErrNilComparator     = errors.New("heap: comparator is required")
	ErrInputExceedsLimit = errors.New("heap: input exceeds limit")
	ErrQueueFull         = errors.New("heap: queue is full")
	ErrUninitialized     = errors.New("heap: checked queue is uninitialized")
)

// CheckedPriorityQueue is the error-returning interface implemented by
// BoundedHeap. A failed Push leaves both contents and length unchanged.
type CheckedPriorityQueue[T any] interface {
	Push(value T) error
	Peek() (T, bool)
	Pop() (T, bool)
	Len() int
	Clear()
}

// BoundedHeap is a binary min-heap whose number of admitted elements never
// exceeds limit. The comparator must define a consistent strict weak ordering;
// equal-priority values are not guaranteed to retain insertion order.
//
// Its zero value is safe to query and clear, but cannot accept pushes until
// constructed with NewBoundedHeap or NewBoundedHeapFromSlice.
type BoundedHeap[T any] struct {
	items []T
	less  func(a, b T) bool
	limit int
}

var _ CheckedPriorityQueue[int] = (*BoundedHeap[int])(nil)

// NewBoundedHeap creates an empty bounded min-heap. A zero limit is valid and
// creates a queue that will reject every insertion with ErrQueueFull. The
// limit is not a preallocation request.
func NewBoundedHeap[T any](limit int, less func(a, b T) bool) (*BoundedHeap[T], error) {
	if limit < 0 {
		return nil, ErrNegativeLimit
	}
	if less == nil {
		return nil, ErrNilComparator
	}
	return &BoundedHeap[T]{items: make([]T, 0), less: less, limit: limit}, nil
}

// NewBoundedHeapFromSlice copies values and builds a bounded min-heap with
// bottom-up Floyd heapification. The caller's slice is never reordered and
// the returned heap owns its slice storage. T itself is copied shallowly.
func NewBoundedHeapFromSlice[T any](values []T, limit int, less func(a, b T) bool) (*BoundedHeap[T], error) {
	if limit < 0 {
		return nil, ErrNegativeLimit
	}
	if less == nil {
		return nil, ErrNilComparator
	}
	if len(values) > limit {
		return nil, ErrInputExceedsLimit
	}
	items := append([]T(nil), values...)
	h := &BoundedHeap[T]{items: items, less: less, limit: limit}
	h.heapify()
	return h, nil
}

// Limit reports the configured maximum element count. It does not report the
// slice's current capacity or cause storage to be preallocated.
func (h *BoundedHeap[T]) Limit() int {
	if h == nil {
		return 0
	}
	return h.limit
}

// Len reports the current number of elements. The zero value and nil receiver
// both report zero.
func (h *BoundedHeap[T]) Len() int {
	if h == nil {
		return 0
	}
	return len(h.items)
}

// Push inserts value unless the queue is nil/uninitialized or already full.
// The full check precedes append, so ErrQueueFull cannot mutate the queue.
func (h *BoundedHeap[T]) Push(value T) error {
	if h == nil || h.less == nil {
		return ErrUninitialized
	}
	if len(h.items) >= h.limit {
		return ErrQueueFull
	}
	h.items = append(h.items, value)
	h.up(len(h.items) - 1)
	return nil
}

// Peek returns the minimum value without removing it. Empty, zero-value, and
// nil queues return the type's zero value and false.
func (h *BoundedHeap[T]) Peek() (T, bool) {
	var zero T
	if h == nil || len(h.items) == 0 {
		return zero, false
	}
	return h.items[0], true
}

// Pop removes and returns the minimum value. The vacated backing-array slot is
// zeroed before shortening the slice so pointer-bearing T values are released.
func (h *BoundedHeap[T]) Pop() (T, bool) {
	var zero T
	if h == nil || len(h.items) == 0 {
		return zero, false
	}
	root := h.items[0]
	last := len(h.items) - 1
	if last == 0 {
		h.items[0] = zero
		h.items = h.items[:0]
		return root, true
	}
	h.items[0] = h.items[last]
	h.items[last] = zero
	h.items = h.items[:last]
	h.down(0)
	return root, true
}

// Clear drops every retained element reference while preserving the configured
// limit and comparator. Calling it on a nil receiver is a no-op.
func (h *BoundedHeap[T]) Clear() {
	if h == nil {
		return
	}
	clear(h.items)
	h.items = h.items[:0]
}

// heapify performs Floyd's O(n) bottom-up construction. Starting at the last
// parent avoids visiting leaves, and down checks the leaf boundary before
// calculating either child index.
func (h *BoundedHeap[T]) heapify() {
	for index := len(h.items) / 2; index > 0; index-- {
		h.down(index - 1)
	}
}

func (h *BoundedHeap[T]) up(index int) {
	for index > 0 {
		parent := (index - 1) / 2
		if !h.less(h.items[index], h.items[parent]) {
			return
		}
		h.items[index], h.items[parent] = h.items[parent], h.items[index]
		index = parent
	}
}

func (h *BoundedHeap[T]) down(index int) {
	for {
		// For a heap with n entries, only indexes below n/2 can have a left
		// child. This test keeps 2*index+1 within int range.
		if index < 0 || index >= len(h.items)/2 {
			return
		}
		left := 2*index + 1
		right := left + 1
		smallest := index
		if h.less(h.items[left], h.items[smallest]) {
			smallest = left
		}
		if right < len(h.items) && h.less(h.items[right], h.items[smallest]) {
			smallest = right
		}
		if smallest == index {
			return
		}
		h.items[index], h.items[smallest] = h.items[smallest], h.items[index]
		index = smallest
	}
}
