package microbenchmark

import (
	"math"
	"math/bits"
)

// performanceClock keeps the Windows counter conversion testable without an OS
// clock. A negative reading deliberately uses the collector's existing failure
// path rather than silently substituting a clock with different resolution.
func performanceClock(frequency int64, read func() (int64, bool)) Clock {
	if frequency <= 0 || read == nil {
		return func() int64 { return -1 }
	}
	start, ok := read()
	if !ok || start < 0 {
		return func() int64 { return -1 }
	}
	return func() int64 {
		current, ok := read()
		if !ok || current < start {
			return -1
		}
		return performanceTicksNS(current-start, frequency)
	}
}

// performanceTicksNS computes floor(ticks*1e9/frequency) exactly. Split whole
// seconds first, then use a 128-bit intermediate for the fractional second.
// Because remainder < frequency, the division's quotient is always < 1e9 and
// its high limb is strictly less than the divisor, as bits.Div64 requires.
func performanceTicksNS(ticks, frequency int64) int64 {
	if ticks < 0 || frequency <= 0 {
		return -1
	}
	seconds, remainder := ticks/frequency, ticks%frequency
	if seconds > math.MaxInt64/1_000_000_000 {
		return -1
	}
	hi, lo := bits.Mul64(uint64(remainder), 1_000_000_000)
	fraction, _ := bits.Div64(hi, lo, uint64(frequency))
	whole := uint64(seconds) * 1_000_000_000
	if fraction > math.MaxInt64-whole {
		return -1
	}
	return int64(whole + fraction)
}
