package microbenchmark

import "testing"

func TestWindowsPerformanceCounterAndFrequency(t *testing.T) {
	frequency, ok := queryPerformanceValue(performanceFrequency)
	if !ok || frequency <= 0 {
		t.Fatalf("performance frequency unavailable: %d, %t", frequency, ok)
	}
	first, ok := queryPerformanceValue(performanceCounter)
	second, secondOK := queryPerformanceValue(performanceCounter)
	if !ok || !secondOK || first < 0 || second < first {
		t.Fatalf("performance counter invalid: %d %d %t %t", first, second, ok, secondOK)
	}
	clock := MonotonicClock()
	previous := clock()
	for range 100 {
		next := clock()
		if previous < 0 || next < previous {
			t.Fatalf("native clock invalid: %d then %d", previous, next)
		}
		previous = next
	}
}

func TestWindowsMissingPerformanceProcedureFailsClosed(t *testing.T) {
	if _, ok := queryPerformanceValue(performanceKernel.NewProc("MissingMicrobenchmarkClockProcedure")); ok {
		t.Fatal("a missing performance procedure was accepted")
	}
}
