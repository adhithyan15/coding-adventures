package heap

// MinHeap stores a caller-ordered tree in a flat slice. The comparator must
// define a consistent strict weak ordering; elements with equal priority do
// not have a stability guarantee.
type MinHeap[T any] struct {
	items []T
	less  func(a, b T) bool
}

func NewMinHeap[T any](less func(a, b T) bool) *MinHeap[T] {
	return &MinHeap[T]{items: make([]T, 0), less: less}
}

func (h *MinHeap[T]) Clone() *MinHeap[T] {
	if h == nil {
		return nil
	}
	// Clone copies the slice storage but, for pointer-bearing T, deliberately
	// keeps the same referenced objects. It is not a deep copy.
	clone := &MinHeap[T]{
		items: append([]T(nil), h.items...),
		less:  h.less,
	}
	return clone
}

func (h *MinHeap[T]) Len() int {
	if h == nil {
		return 0
	}
	return len(h.items)
}

func (h *MinHeap[T]) Push(value T) {
	if h.less == nil {
		panic("heap: missing comparator")
	}
	h.items = append(h.items, value)
	h.up(len(h.items) - 1)
}

func (h *MinHeap[T]) Peek() (T, bool) {
	var zero T
	if h == nil || len(h.items) == 0 {
		return zero, false
	}
	return h.items[0], true
}

func (h *MinHeap[T]) Pop() (T, bool) {
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

func (h *MinHeap[T]) Clear() {
	if h == nil {
		return
	}
	clear(h.items)
	h.items = nil
}

func (h *MinHeap[T]) up(index int) {
	for index > 0 {
		parent := (index - 1) / 2
		if !h.less(h.items[index], h.items[parent]) {
			break
		}
		h.items[index], h.items[parent] = h.items[parent], h.items[index]
		index = parent
	}
}

func (h *MinHeap[T]) down(index int) {
	for {
		// A node at or beyond len/2 is a leaf. Check before multiplying so
		// child-index arithmetic is bounded by the slice length.
		if index < 0 || index >= len(h.items)/2 {
			return
		}
		left := 2*index + 1
		right := left + 1
		smallest := index

		if left < len(h.items) && h.less(h.items[left], h.items[smallest]) {
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
