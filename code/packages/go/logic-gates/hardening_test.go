package logicgates

import (
	"fmt"
	"testing"
)

func TestNaryInputCap(t *testing.T) {
	big := make([]int, MaxInputs+1)
	for name, fn := range map[string]func(...int) int{"ANDn": ANDn, "ORn": ORn, "XORn": XORn} {
		func() {
			defer func() {
				r := recover()
				if r == nil {
					t.Fatalf("%s accepted %d inputs", name, len(big))
				}
				if _, ok := IsInvalidBit(r); ok {
					t.Errorf("%s: cap violation must not be an InvalidBitError", name)
				}
			}()
			fn(big...)
		}()
		// Exactly MaxInputs is allowed.
		fn(big[:MaxInputs]...)
	}
}

// Deterministic snapshot invariants. These do NOT claim to prove freedom from
// a concurrent-writer TOCTOU: the snapshot's own read of a caller-shared slice
// is the caller's race by design (this package cannot stop someone mutating
// memory it is reading). What is checked is that a late bad value is found
// and named by index, and that the caller's slice is never modified, whether
// the call returns or panics.
func TestNaryLateBadValueNamedAndCallerUnchanged(t *testing.T) {
	for name, fn := range map[string]func(...int) int{"ANDn": ANDn, "ORn": ORn, "XORn": XORn} {
		bad := make([]int, 2000)
		bad[1000] = 2
		want := append([]int(nil), bad...)
		assertInvalidBit(t, name, "inputs[1000]", 2, func() { fn(bad...) })
		if fmt.Sprint(bad) != fmt.Sprint(want) {
			t.Errorf("%s mutated the caller slice while panicking", name)
		}
		good := make([]int, 2000)
		for i := range good {
			good[i] = i % 2
		}
		want = append([]int(nil), good...)
		if r := fn(good...); r != 0 && r != 1 {
			t.Errorf("%s returned non-bit %d", name, r)
		}
		if fmt.Sprint(good) != fmt.Sprint(want) {
			t.Errorf("%s mutated the caller slice", name)
		}
	}
}

func TestNaryDoesNotMutateCaller(t *testing.T) {
	in := []int{1, 0, 1, 1}
	ANDn(in...)
	ORn(in...)
	XORn(in...)
	if fmt.Sprint(in) != "[1 0 1 1]" {
		t.Errorf("caller slice mutated: %v", in)
	}
}

func TestCounterOutputsDoNotAliasState(t *testing.T) {
	paths := []struct {
		name         string
		clock, reset int
		start        []int
	}{
		{"increment", 1, 0, []int{1, 0, 1, 0}},
		{"hold", 0, 0, []int{1, 0, 1, 0}},
		{"reset", 0, 1, []int{1, 0, 1, 0}},
	}
	for _, p := range paths {
		// Mutating the returned outputs must not change the returned state.
		out, ns := Counter(p.clock, p.reset, &CounterState{Bits: append([]int(nil), p.start...), Width: 4})
		before := append([]int(nil), ns.Bits...)
		for i := range out {
			out[i] = 1 - out[i]
		}
		if fmt.Sprint(ns.Bits) != fmt.Sprint(before) {
			t.Errorf("%s: writing outputs changed state %v -> %v", p.name, before, ns.Bits)
		}
		// And vice versa: mutating the state must not change the outputs.
		out, ns = Counter(p.clock, p.reset, &CounterState{Bits: append([]int(nil), p.start...), Width: 4})
		beforeOut := append([]int(nil), out...)
		for i := range ns.Bits {
			ns.Bits[i] = 1 - ns.Bits[i]
		}
		if fmt.Sprint(out) != fmt.Sprint(beforeOut) {
			t.Errorf("%s: writing state changed outputs %v -> %v", p.name, beforeOut, out)
		}
	}
}
