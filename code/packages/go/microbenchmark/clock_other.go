//go:build !windows

package microbenchmark

import "time"

// MonotonicClock returns elapsed nanoseconds relative to its creation. On this
// platform it uses Go's monotonic time component, not the wall clock.
func MonotonicClock() Clock {
	start := time.Now()
	return func() int64 { return time.Since(start).Nanoseconds() }
}
