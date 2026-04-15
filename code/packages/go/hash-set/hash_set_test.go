package hashset

import "testing"

func TestHashSetOperations(t *testing.T) {
	s := New()
	if !s.Add([]byte("a")) || !s.Add([]byte("b")) || s.Add([]byte("a")) {
		t.Fatal("unexpected add behavior")
	}
	if !s.Contains([]byte("a")) || s.Size() != 2 {
		t.Fatal("expected set membership")
	}
	other := New()
	other.Add([]byte("b"))
	other.Add([]byte("c"))
	if got := s.Intersection(other).ToSlice(); len(got) != 1 || string(got[0]) != "b" {
		t.Fatalf("unexpected intersection: %q", got)
	}
	if got := s.Union(other).ToSlice(); len(got) != 3 {
		t.Fatalf("unexpected union: %q", got)
	}
	if !s.Remove([]byte("a")) || s.Contains([]byte("a")) {
		t.Fatal("remove failed")
	}
}
