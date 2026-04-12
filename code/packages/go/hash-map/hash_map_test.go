package hashmap

import "testing"

func TestHashMapBasicOperations(t *testing.T) {
	m := New[int]()
	m.Set([]byte("alpha"), 1)
	m.Set([]byte("beta"), 2)
	m.Set([]byte("beta"), 3)

	if got, ok := m.Get([]byte("alpha")); !ok || got != 1 {
		t.Fatalf("expected alpha=1, got %v %v", got, ok)
	}
	if got, ok := m.Get([]byte("beta")); !ok || got != 3 {
		t.Fatalf("expected beta=3, got %v %v", got, ok)
	}
	if !m.Has([]byte("alpha")) || m.Size() != 2 {
		t.Fatalf("unexpected map state: has alpha=%v size=%d", m.Has([]byte("alpha")), m.Size())
	}
	if !m.Delete([]byte("alpha")) || m.Has([]byte("alpha")) {
		t.Fatal("delete failed")
	}
	keys := m.Keys()
	if len(keys) != 1 || string(keys[0]) != "beta" {
		t.Fatalf("unexpected keys: %q", keys)
	}
}

func TestHashMapClone(t *testing.T) {
	m := New[string]()
	m.Set([]byte("x"), "1")
	clone := m.Clone()
	clone.Set([]byte("y"), "2")
	if m.Has([]byte("y")) {
		t.Fatal("clone should not mutate original")
	}
}
