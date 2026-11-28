package rankstatistics

import (
	_ "embed"
	"encoding/binary"
	"encoding/json"
	"math"
	"math/bits"
	"math/rand"
	"slices"
	"testing"
)

// The shared fixture is embedded so this pure package's test binary does not
// read files at runtime. BUILD compares this copy with the canonical fixture.
//go:embed testdata/microbenchmark-pairs-v1.json
var sharedFixture []byte

func TestSharedFixtureConformance(t *testing.T) {
	var fixture struct {
		Confidence int `json:"confidence_percent"`
		Cases      []struct {
			Name       string    `json:"name"`
			Baseline   int64     `json:"baseline_ns"`
			Candidates []int64   `json:"candidate_ns"`
			Ratio      float64   `json:"ratio"`
			Interval   []float64 `json:"interval"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(sharedFixture, &fixture); err != nil {
		t.Fatal(err)
	}
	for _, testCase := range fixture.Cases {
		t.Run(testCase.Name, func(t *testing.T) {
			ratios := make([]float64, len(testCase.Candidates))
			for i, value := range testCase.Candidates {
				ratios[i] = float64(value) / float64(testCase.Baseline)
			}
			gotMedian, err := Median(ratios)
			if err != nil {
				t.Fatal(err)
			}
			if !closeEnough(gotMedian, testCase.Ratio) {
				t.Fatalf("median %g, want %g", gotMedian, testCase.Ratio)
			}
			got, err := MedianInterval(ratios, fixture.Confidence)
			if err != nil {
				t.Fatal(err)
			}
			if len(testCase.Interval) == 0 {
				if got != nil {
					t.Fatalf("interval %v, want none", got)
				}
			} else if got == nil || !closeEnough(got.Low, testCase.Interval[0]) || !closeEnough(got.High, testCase.Interval[1]) {
				t.Fatalf("interval %v, want %v", got, testCase.Interval)
			}
		})
	}
}

func TestMedianOddEvenExtremesAndInputPreservation(t *testing.T) {
	tiny := math.SmallestNonzeroFloat64
	cases := []struct {
		values []float64
		want   float64
	}{
		{[]float64{9, 1, 5}, 5},
		{[]float64{8, 1, 7, 2}, 4.5},
		{[]float64{math.MaxFloat64, math.MaxFloat64}, math.MaxFloat64},
		{[]float64{-math.MaxFloat64, math.MaxFloat64}, 0},
		{[]float64{-math.MaxFloat64, -math.MaxFloat64}, -math.MaxFloat64},
		{[]float64{tiny, 2 * tiny}, 2 * tiny}, // exact midpoint tie rounds to even
	}
	for _, testCase := range cases {
		before := slices.Clone(testCase.values)
		got, err := Median(testCase.values)
		if err != nil {
			t.Fatal(err)
		}
		if got != testCase.want || math.IsNaN(got) || math.IsInf(got, 0) {
			t.Fatalf("Median(%v)=%g, want finite %g", testCase.values, got, testCase.want)
		}
		if !slices.Equal(testCase.values, before) {
			t.Fatalf("input mutated: %v -> %v", before, testCase.values)
		}
	}
}

func TestIntervalCutoffAndTies(t *testing.T) {
	got, err := MedianInterval([]float64{3, 1, 2}, 75)
	if err != nil || got == nil || *got != (Interval{Low: 1, High: 3}) {
		t.Fatalf("n=3 at 75%%: %v, %v", got, err)
	}
	got, err = MedianInterval([]float64{3, 1, 2}, 76)
	if err != nil || got != nil {
		t.Fatalf("n=3 at 76%%: %v, %v; want no interval", got, err)
	}
	got, err = MedianInterval([]float64{3, 1, 3, 2, 1, 3, 2, 1}, 50)
	if err != nil || got == nil || *got != (Interval{Low: 1, High: 3}) {
		t.Fatalf("tied ranks: %v, %v", got, err)
	}
}

// bruteForceRank enumerates sign assignments instead of using binomial
// coefficients, keeping the bounded exact oracle structurally independent.
func bruteForceRank(n, confidence int) int {
	assignments, best := 1<<n, 0
	for rank := 1; rank <= n/2+1; rank++ {
		below := 0
		for mask := 0; mask < assignments; mask++ {
			if bits.OnesCount(uint(mask)) < rank {
				below++
			}
		}
		if 200*below <= (100-confidence)*assignments {
			best = rank
		} else {
			break
		}
	}
	return best
}

func TestExactCutoffAgainstExhaustiveSignOracle(t *testing.T) {
	for n := 1; n <= 10; n++ {
		values := make([]float64, n)
		for i := range values {
			values[i] = float64(i + 1)
		}
		for confidence := 1; confidence <= 99; confidence++ {
			wantRank := bruteForceRank(n, confidence)
			got, err := MedianInterval(values, confidence)
			if err != nil {
				t.Fatalf("n=%d confidence=%d: %v", n, confidence, err)
			}
			if wantRank == 0 {
				if got != nil {
					t.Fatalf("n=%d confidence=%d: got %v, want none", n, confidence, got)
				}
			} else {
				want := Interval{Low: float64(wantRank), High: float64(n - wantRank + 1)}
				if got == nil || *got != want {
					t.Fatalf("n=%d confidence=%d: got %v, want %v", n, confidence, got, want)
				}
			}
		}
	}
}

func TestSeededGeneratedPermutationInvariance(t *testing.T) {
	rng := rand.New(rand.NewSource(0x5eed))
	for trial := 0; trial < 250; trial++ {
		n := 1 + rng.Intn(20)
		values := make([]float64, n)
		for i := range values {
			values[i] = float64(rng.Intn(9) - 4)
		}
		confidence := 1 + rng.Intn(99)
		median, err := Median(values)
		if err != nil {
			t.Fatal(err)
		}
		interval, err := MedianInterval(values, confidence)
		if err != nil {
			t.Fatal(err)
		}
		permuted := slices.Clone(values)
		rng.Shuffle(len(permuted), func(i, j int) { permuted[i], permuted[j] = permuted[j], permuted[i] })
		otherMedian, err := Median(permuted)
		if err != nil || otherMedian != median {
			t.Fatalf("trial %d: median changed %v to %v (%v)", trial, median, otherMedian, err)
		}
		otherInterval, err := MedianInterval(permuted, confidence)
		if err != nil || !sameInterval(interval, otherInterval) {
			t.Fatalf("trial %d: interval changed %v to %v (%v)", trial, interval, otherInterval, err)
		}
	}
}

func FuzzMedianInterval(f *testing.F) {
	f.Add([]byte{1, 2, 3}, uint8(75))
	f.Add([]byte{1, 2, 3}, uint8(74))
	f.Add([]byte{0, 0, 255, 128, 128}, uint8(95))
	f.Add([]byte{}, uint8(1))
	f.Fuzz(func(t *testing.T, raw []byte, confidenceByte uint8) {
		if len(raw) > 10 {
			raw = raw[:10]
		}
		values := make([]float64, len(raw))
		for i, value := range raw {
			values[i] = float64(int(value%17) - 8)
		}
		if len(values) == 0 {
			values = []float64{0}
		}
		confidence := int(confidenceByte%99) + 1
		median, err := Median(values)
		if err != nil || math.IsNaN(median) || math.IsInf(median, 0) {
			t.Fatalf("median=%v error=%v", median, err)
		}
		got, err := MedianInterval(values, confidence)
		if err != nil {
			t.Fatal(err)
		}
		ordered, _ := sortedFinite(values)
		rank := bruteForceRank(len(values), confidence)
		var want *Interval
		if rank > 0 {
			want = &Interval{Low: ordered[rank-1], High: ordered[len(ordered)-rank]}
		}
		if !sameInterval(got, want) {
			t.Fatalf("interval=%v want=%v values=%v confidence=%d", got, want, values, confidence)
		}
		slices.Reverse(ordered)
		reversed, err := MedianInterval(ordered, confidence)
		if err != nil || !sameInterval(got, reversed) {
			t.Fatalf("reverse permutation changed interval %v to %v (%v)", got, reversed, err)
		}
	})
}

func FuzzMedianFiniteValues(f *testing.F) {
	f.Add(packFloatBits(math.MaxFloat64, math.MaxFloat64))
	f.Add(packFloatBits(-math.MaxFloat64, math.MaxFloat64))
	f.Add(packFloatBits(math.SmallestNonzeroFloat64, 2*math.SmallestNonzeroFloat64))
	f.Add(packFloatBits(math.NaN(), 1))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 80 {
			raw = raw[:80]
		}
		values := make([]float64, len(raw)/8)
		if len(values) == 0 {
			values = []float64{0}
		}
		allFinite := true
		for i := range values {
			values[i] = math.Float64frombits(binary.LittleEndian.Uint64(raw[i*8 : i*8+8]))
			allFinite = allFinite && !math.IsNaN(values[i]) && !math.IsInf(values[i], 0)
		}
		median, err := Median(values)
		if !allFinite {
			if err == nil {
				t.Fatalf("accepted non-finite sample %v", values)
			}
			return
		}
		if err != nil || math.IsNaN(median) || math.IsInf(median, 0) {
			t.Fatalf("finite sample median=%v error=%v", median, err)
		}
		interval, err := MedianInterval(values, 95)
		if err != nil {
			t.Fatal(err)
		}
		if interval != nil && (math.IsNaN(interval.Low) || math.IsNaN(interval.High) || math.IsInf(interval.Low, 0) || math.IsInf(interval.High, 0)) {
			t.Fatalf("non-finite interval for finite sample %v: %v", values, interval)
		}
	})
}

func packFloatBits(values ...float64) []byte {
	packed := make([]byte, 8*len(values))
	for i, value := range values {
		binary.LittleEndian.PutUint64(packed[i*8:], math.Float64bits(value))
	}
	return packed
}

func TestRejectInvalidInput(t *testing.T) {
	for _, values := range [][]float64{nil, {}, {math.NaN()}, {math.Inf(1)}, {math.Inf(-1)}} {
		if _, err := Median(values); err == nil {
			t.Errorf("Median(%v) accepted invalid sample", values)
		}
		if _, err := MedianInterval(values, 95); err == nil {
			t.Errorf("MedianInterval(%v) accepted invalid sample", values)
		}
	}
	for _, confidence := range []int{-1, 0, 100, 101} {
		if _, err := MedianInterval([]float64{1}, confidence); err == nil {
			t.Errorf("confidence %d accepted", confidence)
		}
	}
}

func sameInterval(left, right *Interval) bool {
	return left == nil && right == nil || left != nil && right != nil && *left == *right
}

func closeEnough(left, right float64) bool { return math.Abs(left-right) <= 1e-12 }
