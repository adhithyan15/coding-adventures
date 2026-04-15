package heap

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
	h.items[0] = h.items[last]
	h.items = h.items[:last]
	if len(h.items) > 0 {
		h.down(0)
	}
	return root, true
}

func (h *MinHeap[T]) Clear() {
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
