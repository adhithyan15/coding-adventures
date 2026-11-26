package microbenchmark

import (
	"runtime"
	"syscall"
	"unsafe"
)

// Kernel32 is a Windows KnownDLL. Using its fixed system-library name avoids
// accepting a caller-supplied DLL or a performance-counter implementation.
var performanceKernel = syscall.NewLazyDLL("kernel32.dll")
var performanceCounter = performanceKernel.NewProc("QueryPerformanceCounter")
var performanceFrequency = performanceKernel.NewProc("QueryPerformanceFrequency")

func queryPerformanceValue(procedure *syscall.LazyProc) (int64, bool) {
	if err := procedure.Find(); err != nil {
		return 0, false
	}
	var value int64
	// Both APIs synchronously write one LARGE_INTEGER (a signed 64-bit value).
	// The pointer is valid for the call and is never retained by Windows.
	ok, _, _ := procedure.Call(uintptr(unsafe.Pointer(&value)))
	runtime.KeepAlive(&value)
	return value, ok != 0
}

// MonotonicClock uses Windows' high-resolution performance counter. Some Go
// runtime monotonic clocks on Windows have millisecond-scale resolution, so
// expressing their readings in nanoseconds does not make them suitable for
// sub-millisecond benchmark batches. Frequency is cached once per clock.
func MonotonicClock() Clock {
	frequency, ok := queryPerformanceValue(performanceFrequency)
	if !ok {
		return func() int64 { return -1 }
	}
	return performanceClock(frequency, func() (int64, bool) {
		return queryPerformanceValue(performanceCounter)
	})
}
