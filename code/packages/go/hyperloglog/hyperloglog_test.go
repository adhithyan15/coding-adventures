package hyperloglog

import "testing"

func TestHyperLogLog(t *testing.T) {
	h := New()
	for i := 0; i < 1000; i++ {
		h.Add([]byte{byte(i >> 8), byte(i)})
	}
	if got := h.Count(); got < 800 || got > 1200 {
		t.Fatalf("unexpected approximate count: %d", got)
	}
	clone := h.Clone()
	if !h.Equal(clone) {
		t.Fatal("expected clone equality")
	}
}
