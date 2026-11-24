package logicgates

import (
	"fmt"
	"sync"
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

// A writer flips a shared slice between bits and an illegal value while the
// gates run. Each gate validates and computes from a single private copy, so
// a result is always a bit, or the call panics with *InvalidBitError; never a
// computed non-bit. The copy of the caller's slice is the caller's own race
// (this package cannot prevent concurrent mutation of memory it is reading),
// so the test serialises that one read with a mutex to keep -race meaningful
// for the gate internals.
func TestNaryConcurrentWriterNeverYieldsNonBit(t *testing.T) {
	var mu sync.Mutex
	in := make([]int, 64)
	stop := make(chan struct{})
	var wg sync.WaitGroup
	wg.Add(1)
	go func() {
		defer wg.Done()
		for i := 0; ; i++ {
			select {
			case <-stop:
				return
			default:
			}
			mu.Lock()
			for j := range in {
				in[j] = (i + j) % 3 // 0,1,2: includes a non-bit
			}
			mu.Unlock()
		}
	}()
	fns := map[string]func(...int) int{"ANDn": ANDn, "ORn": ORn, "XORn": XORn}
	for i := 0; i < 2000; i++ {
		for name, fn := range fns {
			func() {
				defer func() {
					if r := recover(); r != nil {
						if _, ok := IsInvalidBit(r); !ok {
							t.Errorf("%s: unexpected panic %v", name, r)
						}
					}
				}()
				got := func() int {
					mu.Lock()
					defer mu.Unlock() // the gate may panic; never leak the lock
					return fn(in...)
				}()
				if got != 0 && got != 1 {
					t.Errorf("%s returned non-bit %d", name, got)
				}
			}()
		}
	}
	close(stop)
	wg.Wait()
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
	for _, tc := range []struct{ clock, reset int }{{1, 0}, {0, 0}, {1, 1}} {
		st := &CounterState{Width: 4}
		out, ns := Counter(tc.clock, tc.reset, st)
		out[0] = 1
		if ns.Bits[0] == 1 && tc.clock == 0 || ns.Bits[0] == 1 && tc.reset == 1 {
			t.Errorf("clock=%d reset=%d: output write leaked into state", tc.clock, tc.reset)
		}
		ns.Bits[1] = 1
		if out[1] == 1 {
			t.Errorf("clock=%d reset=%d: state write leaked into output", tc.clock, tc.reset)
		}
	}
}
