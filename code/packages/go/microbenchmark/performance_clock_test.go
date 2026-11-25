package microbenchmark

import (
	"math"
	"math/big"
	"math/rand"
	"testing"
)

func TestPerformanceTicksNSBoundaries(t *testing.T) {
	for _, tc := range []struct{ ticks, frequency, want int64 }{
		{0, 10_000_000, 0}, {1, 10_000_000, 100}, {4, 3, 1_333_333_333},
		{math.MaxInt64, 1_000_000_000, math.MaxInt64},
		{math.MaxInt64, math.MaxInt64, 1_000_000_000},
		{math.MaxInt64 - 1, math.MaxInt64, 999_999_999},
		{math.MaxInt64, 1, -1}, {-1, 1, -1}, {1, 0, -1}, {1, -1, -1},
	} {
		if got := performanceTicksNS(tc.ticks, tc.frequency); got != tc.want {
			t.Errorf("ticks=%d frequency=%d: got %d want %d", tc.ticks, tc.frequency, got, tc.want)
		}
	}
}

func ticksOracle(ticks, frequency int64) int64 {
	if ticks < 0 || frequency <= 0 {
		return -1
	}
	value := new(big.Int).Mul(big.NewInt(ticks), big.NewInt(1_000_000_000))
	value.Quo(value, big.NewInt(frequency))
	if !value.IsInt64() {
		return -1
	}
	return value.Int64()
}

func TestPerformanceTicksMatchesIndependentBigIntegerOracle(t *testing.T) {
	random := rand.New(rand.NewSource(71))
	for range 20_000 {
		ticks, frequency := random.Int63(), random.Int63()
		if got, want := performanceTicksNS(ticks, frequency), ticksOracle(ticks, frequency); got != want {
			t.Fatalf("ticks=%d frequency=%d: got %d want %d", ticks, frequency, got, want)
		}
	}
}

func TestPerformanceClockRejectsInvalidAndFailedReadings(t *testing.T) {
	for _, frequency := range []int64{0, -1} {
		if got := performanceClock(frequency, nil)(); got != -1 {
			t.Fatalf("invalid frequency accepted: %d", got)
		}
	}
	if got := performanceClock(1, nil)(); got != -1 {
		t.Fatalf("nil reader accepted: %d", got)
	}
	for _, start := range []int64{-1, 0} {
		if got := performanceClock(1, func() (int64, bool) { return start, false })(); got != -1 {
			t.Fatalf("failed initial query accepted: %d", got)
		}
	}
	if got := performanceClock(1, func() (int64, bool) { return -1, true })(); got != -1 {
		t.Fatalf("negative initial query accepted: %d", got)
	}
	for _, tc := range []struct {
		next int64
		ok   bool
	}{{99, true}, {101, false}, {math.MaxInt64, true}} {
		first := true
		clock := performanceClock(1, func() (int64, bool) {
			if first {
				first = false
				return 100, true
			}
			return tc.next, tc.ok
		})
		if got := clock(); got != -1 {
			t.Errorf("invalid next reading %+v accepted: %d", tc, got)
		}
	}
}

func TestPerformanceClockSubtractsStartBeforeConversion(t *testing.T) {
	readings := []int64{math.MaxInt64 - 10, math.MaxInt64 - 10, math.MaxInt64 - 9, math.MaxInt64}
	i := 0
	clock := performanceClock(10_000_000, func() (int64, bool) { value := readings[i]; i++; return value, true })
	for _, want := range []int64{0, 100, 1000} {
		if got := clock(); got != want {
			t.Fatalf("got %d want %d", got, want)
		}
	}
}

func FuzzPerformanceTicksNS(f *testing.F) {
	f.Add(int64(1), int64(10_000_000))
	f.Add(int64(math.MaxInt64), int64(math.MaxInt64))
	f.Add(int64(math.MaxInt64), int64(1))
	f.Fuzz(func(t *testing.T, ticks, frequency int64) {
		if got, want := performanceTicksNS(ticks, frequency), ticksOracle(ticks, frequency); got != want {
			t.Fatalf("ticks=%d frequency=%d: got %d want %d", ticks, frequency, got, want)
		}
	})
}
