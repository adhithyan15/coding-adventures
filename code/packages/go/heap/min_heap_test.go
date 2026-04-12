package heap

import "testing"

func TestMinHeap(t *testing.T) {
	h := NewMinHeap(func(a, b int) bool { return a < b })
	h.Push(3)
	h.Push(1)
	h.Push(2)

	if h.Len() != 3 {
		t.Fatalf("expected 3, got %d", h.Len())
	}
	if top, ok := h.Peek(); !ok || top != 1 {
		t.Fatalf("expected min 1, got %v %v", top, ok)
	}
	if val, ok := h.Pop(); !ok || val != 1 {
		t.Fatalf("expected pop 1, got %v %v", val, ok)
	}
	if val, ok := h.Pop(); !ok || val != 2 {
		t.Fatalf("expected pop 2, got %v %v", val, ok)
	}
}
