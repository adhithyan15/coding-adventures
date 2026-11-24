package logicgates

import (
	"fmt"
	"math"
	"testing"
)

// Security regressions: exact-type rejection identity, nil safety, bounded
// widths (no 1<<n overflow or OOM), and validated state.

func TestIsInvalidBitExactTypeOnly(t *testing.T) {
	inner := &InvalidBitError{Param: "a", Value: 2}
	if _, ok := IsInvalidBit(&InternalError{Gate: "AND", Err: inner}); ok {
		t.Error("InternalError wrapping InvalidBitError must not be a rejection")
	}
	if _, ok := IsInvalidBit(fmt.Errorf("x: %w", inner)); ok {
		t.Error("a foreign wrapper must not be a rejection (exact panic value only)")
	}
	if e, ok := IsInvalidBit(inner); !ok || e != inner {
		t.Error("exact value must be a rejection")
	}
}

func TestTypedNilIsNotRejectionAndErrorIsSafe(t *testing.T) {
	var nilErr *InvalidBitError
	if _, ok := IsInvalidBit(nilErr); ok {
		t.Error("typed nil must not be a rejection")
	}
	if nilErr.Error() == "" {
		t.Error("nil Error() should return a fixed string")
	}
}

func shapePanics(t *testing.T, name string, fn func()) {
	t.Helper()
	defer func() {
		r := recover()
		if r == nil {
			t.Errorf("%s: expected panic", name)
			return
		}
		if _, ok := r.(string); !ok {
			t.Errorf("%s: want string shape panic, got %T", name, r)
		}
	}()
	fn()
}

func TestWidthBoundaries(t *testing.T) {
	zeros := func(n int) []int { return make([]int, n) }
	if len(Decoder(zeros(MaxSelectBits))) != MaxWidth {
		t.Error("Decoder at cap should work")
	}
	shapePanics(t, "Decoder 17", func() { Decoder(zeros(MaxSelectBits + 1)) })
	shapePanics(t, "Decoder 64", func() { Decoder(zeros(64)) })

	if MuxN(zeros(MaxWidth), zeros(MaxSelectBits)) != 0 {
		t.Error("MuxN at cap should work")
	}
	shapePanics(t, "MuxN over", func() { MuxN(zeros(2*MaxWidth), zeros(MaxSelectBits+1)) })

	if len(Demux(1, zeros(MaxSelectBits), MaxWidth)) != MaxWidth {
		t.Error("Demux at cap should work")
	}
	shapePanics(t, "Demux over", func() { Demux(1, zeros(MaxSelectBits+1), 2*MaxWidth) })
	shapePanics(t, "Demux huge", func() { Demux(1, nil, math.MaxInt) })
	shapePanics(t, "Demux 1<<62", func() { Demux(1, nil, 1<<62) })

	onehot := zeros(MaxWidth)
	onehot[5] = 1
	if len(Encoder(onehot)) != MaxSelectBits {
		t.Error("Encoder at cap should work")
	}
	shapePanics(t, "Encoder over", func() { Encoder(zeros(2 * MaxWidth)) })
	if _, v := PriorityEncoder(onehot); v != 1 {
		t.Error("PriorityEncoder at cap should work")
	}
	shapePanics(t, "PriorityEncoder over", func() { PriorityEncoder(zeros(2 * MaxWidth)) })
}

func TestCounterBounds(t *testing.T) {
	if out, _ := Counter(1, 0, &CounterState{Width: MaxCounterWidth}); len(out) != MaxCounterWidth {
		t.Error("Counter at cap should work")
	}
	shapePanics(t, "Counter 65", func() { Counter(1, 0, &CounterState{Width: MaxCounterWidth + 1}) })
	shapePanics(t, "Counter mismatch", func() { Counter(1, 0, &CounterState{Width: 4, Bits: []int{0, 0}}) })
	shapePanics(t, "Counter mismatch long", func() { Counter(1, 0, &CounterState{Width: 2, Bits: []int{0, 0, 0}}) })
}

func TestCounterBadStateBitEveryPath(t *testing.T) {
	for _, c := range []struct{ clock, reset int }{{0, 0}, {0, 1}, {1, 0}, {1, 1}} {
		assertInvalidBit(t, "Counter", "state.Bits[1]", 7, func() {
			Counter(c.clock, c.reset, &CounterState{Width: 3, Bits: []int{0, 7, 0}})
		})
	}
}

func TestFlipFlopStateFieldsValidated(t *testing.T) {
	fields := []string{"MasterQ", "MasterQBar", "SlaveQ", "SlaveQBar"}
	for i, f := range fields {
		mk := func() FlipFlopState {
			s := FlipFlopState{0, 1, 0, 1}
			p := []*int{&s.MasterQ, &s.MasterQBar, &s.SlaveQ, &s.SlaveQBar}[i]
			*p = 9
			return s
		}
		assertInvalidBit(t, "DFlipFlop", "state."+f, 9, func() { s := mk(); DFlipFlop(0, 0, &s) })
		assertInvalidBit(t, "Register", "state[1]."+f, 9, func() {
			good := FlipFlopState{0, 1, 0, 1}
			Register([]int{0, 0}, 0, []FlipFlopState{good, mk()})
		})
		assertInvalidBit(t, "ShiftRegister", "state[0]."+f, 9, func() {
			ShiftRegister(0, 0, []FlipFlopState{mk()}, "left")
		})
	}
}
