package fparithmetic

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// TestPipelinedFPAdder tests the 5-stage pipelined adder.
func TestPipelinedFPAdder(t *testing.T) {
	t.Run("single addition", func(t *testing.T) {
		clk := clock.New(1000000)
		adder := NewPipelinedFPAdder(clk, FP32)

		a := FloatToBits(1.5, FP32)
		b := FloatToBits(2.5, FP32)
		adder.Submit(a, b)

		// Run 5 full cycles for the result to emerge
		for i := 0; i < 5; i++ {
			clk.FullCycle()
		}

		if len(adder.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(adder.Results))
		}
		got := BitsToFloat(*adder.Results[0])
		if float32(got) != 4.0 {
			t.Errorf("pipelined add(1.5, 2.5) = %v, want 4.0", got)
		}
	})

	t.Run("multiple additions", func(t *testing.T) {
		clk := clock.New(1000000)
		adder := NewPipelinedFPAdder(clk, FP32)

		// Submit multiple additions
		adder.Submit(FloatToBits(1.0, FP32), FloatToBits(2.0, FP32)) // 3.0
		adder.Submit(FloatToBits(3.0, FP32), FloatToBits(4.0, FP32)) // 7.0
		adder.Submit(FloatToBits(0.5, FP32), FloatToBits(0.5, FP32)) // 1.0

		// Run enough cycles: 5 (first result) + 2 (remaining pipeline)
		for i := 0; i < 8; i++ {
			clk.FullCycle()
		}

		if len(adder.Results) != 3 {
			t.Fatalf("expected 3 results, got %d", len(adder.Results))
		}

		expected := []float64{3.0, 7.0, 1.0}
		for i, want := range expected {
			got := BitsToFloat(*adder.Results[i])
			if float32(got) != float32(want) {
				t.Errorf("result[%d] = %v, want %v", i, got, want)
			}
		}
	})

	t.Run("NaN propagation", func(t *testing.T) {
		clk := clock.New(1000000)
		adder := NewPipelinedFPAdder(clk, FP32)

		adder.Submit(FloatToBits(math.NaN(), FP32), FloatToBits(1.0, FP32))

		for i := 0; i < 5; i++ {
			clk.FullCycle()
		}

		if len(adder.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(adder.Results))
		}
		if !IsNaN(*adder.Results[0]) {
			t.Error("expected NaN result")
		}
	})

	t.Run("infinity handling", func(t *testing.T) {
		clk := clock.New(1000000)
		adder := NewPipelinedFPAdder(clk, FP32)

		adder.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(1.0, FP32))

		for i := 0; i < 5; i++ {
			clk.FullCycle()
		}

		if len(adder.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(adder.Results))
		}
		if !IsInf(*adder.Results[0]) {
			t.Error("expected Inf result")
		}
	})

	t.Run("zero handling", func(t *testing.T) {
		clk := clock.New(1000000)
		adder := NewPipelinedFPAdder(clk, FP32)

		adder.Submit(FloatToBits(0.0, FP32), FloatToBits(5.0, FP32))

		for i := 0; i < 5; i++ {
			clk.FullCycle()
		}

		if len(adder.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(adder.Results))
		}
		got := BitsToFloat(*adder.Results[0])
		if float32(got) != 5.0 {
			t.Errorf("0 + 5 = %v, want 5.0", got)
		}
	})

	t.Run("cycle count", func(t *testing.T) {
		clk := clock.New(1000000)
		adder := NewPipelinedFPAdder(clk, FP32)

		clk.FullCycle()
		clk.FullCycle()

		if adder.CycleCount != 2 {
			t.Errorf("CycleCount = %d, want 2", adder.CycleCount)
		}
	})
}

// TestPipelinedFPMultiplier tests the 4-stage pipelined multiplier.
func TestPipelinedFPMultiplier(t *testing.T) {
	t.Run("single multiplication", func(t *testing.T) {
		clk := clock.New(1000000)
		mul := NewPipelinedFPMultiplier(clk, FP32)

		mul.Submit(FloatToBits(3.0, FP32), FloatToBits(4.0, FP32))

		for i := 0; i < 4; i++ {
			clk.FullCycle()
		}

		if len(mul.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(mul.Results))
		}
		got := BitsToFloat(*mul.Results[0])
		if float32(got) != 12.0 {
			t.Errorf("pipelined mul(3.0, 4.0) = %v, want 12.0", got)
		}
	})

	t.Run("multiple multiplications", func(t *testing.T) {
		clk := clock.New(1000000)
		mul := NewPipelinedFPMultiplier(clk, FP32)

		mul.Submit(FloatToBits(2.0, FP32), FloatToBits(3.0, FP32)) // 6.0
		mul.Submit(FloatToBits(5.0, FP32), FloatToBits(5.0, FP32)) // 25.0

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(mul.Results) != 2 {
			t.Fatalf("expected 2 results, got %d", len(mul.Results))
		}

		expected := []float64{6.0, 25.0}
		for i, want := range expected {
			got := BitsToFloat(*mul.Results[i])
			if float32(got) != float32(want) {
				t.Errorf("result[%d] = %v, want %v", i, got, want)
			}
		}
	})

	t.Run("special values", func(t *testing.T) {
		clk := clock.New(1000000)
		mul := NewPipelinedFPMultiplier(clk, FP32)

		// Inf * 0 = NaN
		mul.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(0.0, FP32))
		// NaN * 1.0 = NaN
		mul.Submit(FloatToBits(math.NaN(), FP32), FloatToBits(1.0, FP32))
		// Inf * 2.0 = Inf
		mul.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(2.0, FP32))
		// 0 * 5 = 0
		mul.Submit(FloatToBits(0.0, FP32), FloatToBits(5.0, FP32))

		for i := 0; i < 8; i++ {
			clk.FullCycle()
		}

		if len(mul.Results) < 4 {
			t.Fatalf("expected 4 results, got %d", len(mul.Results))
		}

		if !IsNaN(*mul.Results[0]) {
			t.Error("result[0] should be NaN (Inf * 0)")
		}
		if !IsNaN(*mul.Results[1]) {
			t.Error("result[1] should be NaN (NaN * 1)")
		}
		if !IsInf(*mul.Results[2]) {
			t.Error("result[2] should be Inf")
		}
		if !IsZero(*mul.Results[3]) {
			t.Error("result[3] should be 0")
		}
	})
}

// TestPipelinedFMA tests the 6-stage pipelined FMA unit.
func TestPipelinedFMA(t *testing.T) {
	t.Run("single FMA", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		// 2.0 * 3.0 + 1.0 = 7.0
		fma.Submit(FloatToBits(2.0, FP32), FloatToBits(3.0, FP32), FloatToBits(1.0, FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		got := BitsToFloat(*fma.Results[0])
		if float32(got) != 7.0 {
			t.Errorf("pipelined FMA(2, 3, 1) = %v, want 7.0", got)
		}
	})

	t.Run("multiple FMAs", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		// 1.0 * 2.0 + 3.0 = 5.0
		fma.Submit(FloatToBits(1.0, FP32), FloatToBits(2.0, FP32), FloatToBits(3.0, FP32))
		// 4.0 * 5.0 + 0.0 = 20.0
		fma.Submit(FloatToBits(4.0, FP32), FloatToBits(5.0, FP32), FloatToBits(0.0, FP32))

		for i := 0; i < 8; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 2 {
			t.Fatalf("expected 2 results, got %d", len(fma.Results))
		}

		got0 := BitsToFloat(*fma.Results[0])
		if float32(got0) != 5.0 {
			t.Errorf("FMA[0] = %v, want 5.0", got0)
		}
		got1 := BitsToFloat(*fma.Results[1])
		if float32(got1) != 20.0 {
			t.Errorf("FMA[1] = %v, want 20.0", got1)
		}
	})

	t.Run("NaN propagation", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		fma.Submit(FloatToBits(math.NaN(), FP32), FloatToBits(1.0, FP32), FloatToBits(1.0, FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		if !IsNaN(*fma.Results[0]) {
			t.Error("expected NaN result")
		}
	})

	t.Run("Inf * 0 = NaN", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		fma.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(0.0, FP32), FloatToBits(1.0, FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		if !IsNaN(*fma.Results[0]) {
			t.Error("expected NaN result from Inf * 0")
		}
	})

	t.Run("0 * finite + c = c", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		fma.Submit(FloatToBits(0.0, FP32), FloatToBits(5.0, FP32), FloatToBits(3.0, FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		got := BitsToFloat(*fma.Results[0])
		if float32(got) != 3.0 {
			t.Errorf("FMA(0, 5, 3) = %v, want 3.0", got)
		}
	})

	t.Run("Inf product + Inf (same sign)", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		fma.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(1.0, FP32), FloatToBits(math.Inf(1), FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		if !IsInf(*fma.Results[0]) {
			t.Error("expected Inf result")
		}
	})

	t.Run("Inf * 1 + (-Inf) = NaN", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		fma.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(1.0, FP32), FloatToBits(math.Inf(-1), FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		if !IsNaN(*fma.Results[0]) {
			t.Error("expected NaN result from Inf + (-Inf)")
		}
	})

	t.Run("finite + Inf = Inf", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		fma.Submit(FloatToBits(2.0, FP32), FloatToBits(3.0, FP32), FloatToBits(math.Inf(1), FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		if !IsInf(*fma.Results[0]) {
			t.Error("expected Inf result")
		}
	})

	t.Run("0 * 0 + 0 = 0", func(t *testing.T) {
		clk := clock.New(1000000)
		fma := NewPipelinedFMA(clk, FP32)

		fma.Submit(FloatToBits(0.0, FP32), FloatToBits(0.0, FP32), FloatToBits(0.0, FP32))

		for i := 0; i < 6; i++ {
			clk.FullCycle()
		}

		if len(fma.Results) != 1 {
			t.Fatalf("expected 1 result, got %d", len(fma.Results))
		}
		if !IsZero(*fma.Results[0]) {
			t.Error("expected zero result")
		}
	})
}

// TestFPUnit tests the complete FP unit with all three pipelines.
func TestFPUnit(t *testing.T) {
	clk := clock.New(1000000)
	unit := NewFPUnit(clk, FP32)

	// Submit to all three pipelines simultaneously
	unit.Adder.Submit(FloatToBits(1.0, FP32), FloatToBits(2.0, FP32))       // 3.0
	unit.Multiplier.Submit(FloatToBits(3.0, FP32), FloatToBits(4.0, FP32))  // 12.0
	unit.Fma.Submit(FloatToBits(2.0, FP32), FloatToBits(3.0, FP32), FloatToBits(1.0, FP32)) // 7.0

	// Run enough cycles for all results (FMA needs 6, the longest)
	unit.Tick(10)

	// Check adder result
	if len(unit.Adder.Results) != 1 {
		t.Fatalf("adder: expected 1 result, got %d", len(unit.Adder.Results))
	}
	addResult := BitsToFloat(*unit.Adder.Results[0])
	if float32(addResult) != 3.0 {
		t.Errorf("adder result = %v, want 3.0", addResult)
	}

	// Check multiplier result
	if len(unit.Multiplier.Results) != 1 {
		t.Fatalf("multiplier: expected 1 result, got %d", len(unit.Multiplier.Results))
	}
	mulResult := BitsToFloat(*unit.Multiplier.Results[0])
	if float32(mulResult) != 12.0 {
		t.Errorf("multiplier result = %v, want 12.0", mulResult)
	}

	// Check FMA result
	if len(unit.Fma.Results) != 1 {
		t.Fatalf("FMA: expected 1 result, got %d", len(unit.Fma.Results))
	}
	fmaResult := BitsToFloat(*unit.Fma.Results[0])
	if float32(fmaResult) != 7.0 {
		t.Errorf("FMA result = %v, want 7.0", fmaResult)
	}
}

// TestPipelineThroughput tests that the pipeline achieves its rated throughput.
// After the initial fill-up latency, one result should emerge per clock cycle.
func TestPipelineThroughput(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	// Submit 10 additions
	for i := 0; i < 10; i++ {
		adder.Submit(FloatToBits(float64(i), FP32), FloatToBits(1.0, FP32))
	}

	// Run 14 cycles (5 for latency + 9 for remaining 9 results)
	for i := 0; i < 14; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 10 {
		t.Errorf("expected 10 results after 14 cycles, got %d", len(adder.Results))
	}

	// Verify results
	for i := 0; i < len(adder.Results); i++ {
		got := BitsToFloat(*adder.Results[i])
		want := float64(i) + 1.0
		if float32(got) != float32(want) {
			t.Errorf("result[%d] = %v, want %v", i, got, want)
		}
	}
}

// TestFPUnitTick tests the Tick convenience method.
func TestFPUnitTick(t *testing.T) {
	clk := clock.New(1000000)
	unit := NewFPUnit(clk, FP32)

	unit.Adder.Submit(FloatToBits(10.0, FP32), FloatToBits(20.0, FP32))
	unit.Tick(5)

	if len(unit.Adder.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(unit.Adder.Results))
	}
	got := BitsToFloat(*unit.Adder.Results[0])
	if float32(got) != 30.0 {
		t.Errorf("result = %v, want 30.0", got)
	}
}

// TestPipelinedAdderSubtraction tests subtraction via the pipeline (negative b).
func TestPipelinedAdderSubtraction(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	// 5.0 + (-3.0) = 2.0
	adder.Submit(FloatToBits(5.0, FP32), FloatToBits(-3.0, FP32))

	for i := 0; i < 5; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(adder.Results))
	}
	got := BitsToFloat(*adder.Results[0])
	if float32(got) != 2.0 {
		t.Errorf("5.0 + (-3.0) = %v, want 2.0", got)
	}
}

// TestEmptyPipeline tests that an empty pipeline produces no results.
func TestEmptyPipeline(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	// Run without submitting anything
	for i := 0; i < 10; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 0 {
		t.Errorf("empty pipeline should have 0 results, got %d", len(adder.Results))
	}
}
