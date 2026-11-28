package microbenchmark

import (
	"bytes"
	"encoding/json"
	"errors"
	"math"
	"os"
	"path/filepath"
	"testing"
)

func TestSharedFixture(t *testing.T) {
	var fixture struct {
		Confidence int     `json:"confidence_percent"`
		Factor     float64 `json:"minimum_factor"`
		Cases      []struct {
			Name      string      `json:"name"`
			Baseline  int64       `json:"baseline_ns"`
			Candidate []int64     `json:"candidate_ns"`
			Ratio     float64     `json:"ratio"`
			Interval  *[2]float64 `json:"interval"`
			Verdict   string      `json:"verdict"`
		} `json:"cases"`
	}
	b, err := os.ReadFile("testdata/microbenchmark-pairs-v1.json")
	if err != nil {
		t.Fatal(err)
	}
	fixtureDir := filepath.Join("..", "..", "..", "specs", "fixtures")
	canonicalPath := filepath.Join(fixtureDir, "microbenchmark-pairs-v1.json")
	if _, statErr := os.Stat(fixtureDir); statErr == nil {
		canonical, readErr := os.ReadFile(canonicalPath)
		if readErr != nil {
			t.Fatalf("canonical fixture is missing: %v", readErr)
		}
		if !bytes.Equal(b, canonical) {
			t.Fatal("module-local fixture copy differs from canonical repository fixture")
		}
	} else if !errors.Is(statErr, os.ErrNotExist) {
		t.Fatal(statErr)
	}
	if err = json.Unmarshal(b, &fixture); err != nil {
		t.Fatal(err)
	}
	for _, tc := range fixture.Cases {
		t.Run(tc.Name, func(t *testing.T) {
			pairs := make([]Pair, len(tc.Candidate))
			for i, v := range tc.Candidate {
				pairs[i] = Pair{BaselineNS: tc.Baseline, CandidateNS: v, Order: "baseline-first"}
			}
			got, e := Analyze(pairs, fixture.Confidence, fixture.Factor)
			if e != nil {
				t.Fatal(e)
			}
			if got.Verdict != tc.Verdict || math.Abs(got.Ratio-tc.Ratio) > 1e-12 {
				t.Fatalf("got %+v", got)
			}
			if tc.Interval == nil {
				if got.Interval != nil {
					t.Fatalf("unexpected interval %+v", got.Interval)
				}
			} else if got.Interval == nil || math.Abs(got.Interval[0]-tc.Interval[0]) > 1e-12 || math.Abs(got.Interval[1]-tc.Interval[1]) > 1e-12 {
				t.Fatalf("interval got %+v want %v", got.Interval, tc.Interval)
			}
		})
	}
}

func TestCollectDeterministic(t *testing.T) {
	c := Config{Pairs: 5, ConfidencePercent: 95, MinimumFactor: 1.05, Seed: 42, TargetNS: 2, MaxBatch: 16}
	clock := int64(0)
	now := func() int64 { clock += 3; return clock }
	batch := func(n int) (int64, error) { return int64(7 * n), nil }
	got, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 7, Baseline: batch, Candidate: batch, Now: now})
	if err != nil {
		t.Fatal(err)
	}
	if got.BatchSize != 1 || len(got.Pairs) != 5 {
		t.Fatalf("bad report: %+v", got)
	}
	for _, p := range got.Pairs {
		if p.BaselineNS != 3 || p.CandidateNS != 3 {
			t.Fatalf("bad duration: %+v", p)
		}
	}
	clock = 0
	again, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 7, Baseline: batch, Candidate: batch, Now: func() int64 { clock += 3; return clock }})
	if err != nil {
		t.Fatal(err)
	}
	for i := range got.Pairs {
		if got.Pairs[i].Order != again.Pairs[i].Order {
			t.Fatal("seed did not reproduce schedule")
		}
	}
}

func TestCollectFailureIsAtomic(t *testing.T) {
	c := Config{Pairs: 2, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 1, MaxBatch: 1}
	n := 0
	_, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 1, Baseline: func(int) (int64, error) {
		n++
		if n > 1 {
			return 2, nil
		}
		return 1, nil
	}, Candidate: func(int) (int64, error) { return 1, nil }, Now: MonotonicClock()})
	if err == nil {
		t.Fatal("expected checksum failure")
	}
}

func TestAnalyzeRejectsInvalidInput(t *testing.T) {
	valid := Pair{BaselineNS: 1, CandidateNS: 1, Order: "baseline-first"}
	cases := []struct {
		name       string
		pairs      []Pair
		confidence int
		factor     float64
	}{
		{"empty", nil, 95, 1.05}, {"confidence", []Pair{valid}, 0, 1.05}, {"nan factor", []Pair{valid}, 95, math.NaN()},
		{"zero duration", []Pair{{BaselineNS: 0, CandidateNS: 1, Order: "baseline-first"}}, 95, 1.05}, {"bad order", []Pair{{BaselineNS: 1, CandidateNS: 1, Order: "bad"}}, 95, 1.05},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if _, err := Analyze(tc.pairs, tc.confidence, tc.factor); err == nil {
				t.Fatal("expected error")
			}
		})
	}
}

func TestAnalyzePairLimitBeforeRatioWork(t *testing.T) {
	pairs := make([]Pair, 10_001)
	if _, err := Analyze(pairs, 95, 1.05); err == nil {
		t.Fatal("expected oversized analysis input to be rejected")
	}
	pairs = make([]Pair, 10_000)
	for i := range pairs {
		pairs[i] = Pair{BaselineNS: 1, CandidateNS: 1, Order: "baseline-first"}
	}
	if _, err := Analyze(pairs, 95, 1.05); err != nil {
		t.Fatalf("maximum supported input rejected: %v", err)
	}
}

func TestCollectRejectsConfigurationAndDependencies(t *testing.T) {
	c := Config{Pairs: 1, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 1, MaxBatch: 1}
	if _, err := Collect(CollectOptions{Config: c}); err == nil {
		t.Fatal("expected missing dependency error")
	}
	c.Pairs = 0
	if _, err := Collect(CollectOptions{Config: c}); err == nil {
		t.Fatal("expected invalid config error")
	}
}

func TestValidateConfigRejectsEveryInvalidField(t *testing.T) {
	base := Defaults()
	cases := []struct {
		name   string
		config Config
	}{
		{"pairs below range", func() Config { c := base; c.Pairs = 0; return c }()},
		{"pairs above range", func() Config { c := base; c.Pairs = 10_001; return c }()},
		{"confidence below range", func() Config { c := base; c.ConfidencePercent = 0; return c }()},
		{"confidence above range", func() Config { c := base; c.ConfidencePercent = 100; return c }()},
		{"nan factor", func() Config { c := base; c.MinimumFactor = math.NaN(); return c }()},
		{"infinite factor", func() Config { c := base; c.MinimumFactor = math.Inf(1); return c }()},
		{"factor at one", func() Config { c := base; c.MinimumFactor = 1; return c }()},
		{"zero target", func() Config { c := base; c.TargetNS = 0; return c }()},
		{"zero max batch", func() Config { c := base; c.MaxBatch = 0; return c }()},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if err := validateConfig(tc.config); err == nil {
				t.Fatal("expected invalid configuration error")
			}
		})
	}
}

func TestAnalyzeRejectsUpperConfidenceAndNonPositiveFactor(t *testing.T) {
	p := Pair{BaselineNS: 1, CandidateNS: 1, Order: "baseline-first"}
	for _, tc := range []struct {
		name       string
		confidence int
		factor     float64
	}{{"high confidence", 100, 1.05}, {"factor below one", 95, .5}, {"infinite factor", 95, math.Inf(1)}} {
		t.Run(tc.name, func(t *testing.T) {
			if _, err := Analyze([]Pair{p}, tc.confidence, tc.factor); err == nil {
				t.Fatal("expected invalid parameter error")
			}
		})
	}
}

func TestCollectWorkloadClockCalibrationAndOverflowFailures(t *testing.T) {
	config := Config{Pairs: 1, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 2, MaxBatch: 2}
	good := func(n int) (int64, error) { return int64(n), nil }
	newOptions := func(c Config) CollectOptions {
		return CollectOptions{Config: c, ExpectedChecksum: 1, Baseline: good, Candidate: good, Now: MonotonicClock()}
	}
	workload := newOptions(config)
	workload.Baseline = func(int) (int64, error) { return 0, os.ErrInvalid }
	if _, err := Collect(workload); err == nil {
		t.Fatal("expected workload error")
	}
	clock := newOptions(config)
	tick := int64(10)
	clock.Now = func() int64 { tick--; return tick }
	if _, err := Collect(clock); err == nil {
		t.Fatal("expected regressing clock error")
	}
	zero := newOptions(config)
	zero.Now = func() int64 { return 0 }
	if _, err := Collect(zero); err == nil {
		t.Fatal("expected exhausted zero-duration calibration")
	}
	overflow := newOptions(config)
	overflow.ExpectedChecksum = math.MaxInt64
	invokedTooLarge := false
	overflow.Baseline = func(n int) (int64, error) {
		if n > 1 {
			invokedTooLarge = true
		}
		return math.MaxInt64, nil
	}
	overflow.Now = func() int64 { return 0 }
	if _, err := Collect(overflow); err == nil || invokedTooLarge {
		t.Fatal("expected overflow before workload")
	}
}

func TestCollectClockRegressionAcrossBatches(t *testing.T) {
	c := Config{Pairs: 1, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 2, MaxBatch: 1}
	values := []int64{100, 110, 105}
	index := 0
	clock := func() int64 { v := values[index]; index++; return v }
	batch := func(n int) (int64, error) { return int64(n), nil }
	_, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 1, Baseline: batch, Candidate: batch, Now: clock})
	if err == nil || index != 3 {
		t.Fatalf("expected cross-batch regression at read three; reads=%d err=%v", index, err)
	}
}

func TestCollectCalibrationDoublesPastHalfMaximum(t *testing.T) {
	c := Config{Pairs: 1, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 2, MaxBatch: 3}
	spans := []int64{1, 1, 1, 1, 2, 2}
	read := 0
	var current int64
	clock := func() int64 {
		if read%2 == 0 {
			current += 1
		} else if read/2 < len(spans) {
			current += spans[read/2]
		} else {
			current += 2
		}
		read++
		return current
	}
	batch := func(n int) (int64, error) { return int64(n), nil }
	report, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 1, Baseline: batch, Candidate: batch, Now: clock})
	if err != nil {
		t.Fatal(err)
	}
	if report.BatchSize != 3 {
		t.Fatalf("expected capped calibration size 3, got %d", report.BatchSize)
	}
}

func TestCollectRejectsNonPositiveTimedDuration(t *testing.T) {
	c := Config{Pairs: 1, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 1, MaxBatch: 1}
	values := []int64{0, 1, 2, 3, 4, 4}
	index := 0
	clock := func() int64 { v := values[index]; index++; return v }
	batch := func(int) (int64, error) { return 1, nil }
	_, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 1, Baseline: batch, Candidate: batch, Now: clock})
	if err == nil || index != len(values) {
		t.Fatalf("expected equal timed timestamps to fail; reads=%d err=%v", index, err)
	}
}

func TestCollectRejectsChecksumOverflowBeforeLargeBatch(t *testing.T) {
	c := Config{Pairs: 1, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 1, MaxBatch: 2}
	calledAtTwo := false
	batch := func(n int) (int64, error) {
		if n == 2 {
			calledAtTwo = true
		}
		return math.MaxInt64, nil
	}
	_, err := Collect(CollectOptions{Config: c, ExpectedChecksum: math.MaxInt64, Baseline: batch, Candidate: batch, Now: func() int64 { return 0 }})
	if err == nil || calledAtTwo {
		t.Fatal("expected checksum multiplication overflow before batch execution")
	}
}

func TestCollectTimedWorkloadErrorsInBothOrders(t *testing.T) {
	base := Config{Pairs: 2, ConfidencePercent: 95, MinimumFactor: 1.05, TargetNS: 1, MaxBatch: 1}
	good := func(int) (int64, error) { return 1, nil }
	firstOrder := func(seed int64) string {
		c := base
		c.Seed = seed
		tick := int64(0)
		r, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 1, Baseline: good, Candidate: good, Now: func() int64 { tick++; return tick }})
		if err != nil {
			t.Fatal(err)
		}
		return r.Pairs[0].Order
	}
	seen := map[string]int64{}
	for seed := int64(0); seed < 100 && len(seen) < 2; seed++ {
		seen[firstOrder(seed)] = seed
	}
	if len(seen) != 2 {
		t.Fatal("could not produce both pair orders")
	}
	for order, seed := range seen {
		for _, variant := range []string{"baseline", "candidate"} {
			c := base
			c.Seed = seed
			calls := map[string]int{}
			failAt := func(target string) Batch {
				return func(int) (int64, error) {
					calls[target]++
					if target == variant && calls[target] == 3 {
						return 0, errors.New("timed batch failed")
					}
					return 1, nil
				}
			}
			tick := int64(0)
			r, err := Collect(CollectOptions{Config: c, ExpectedChecksum: 1, Baseline: failAt("baseline"), Candidate: failAt("candidate"), Now: func() int64 { tick++; return tick }})
			if err == nil || r.Pairs != nil {
				t.Fatalf("order=%s target=%s: expected atomic timed failure", order, variant)
			}
		}
	}
}

func TestReportJSONRoundTrip(t *testing.T) {
	config := Defaults()
	config.Pairs = 1
	report := Report{Config: config, BatchSize: 2, Pairs: []Pair{{10, 9, "baseline-first"}}, Analysis: Analysis{Ratio: .9, Verdict: "inconclusive"}}
	b, err := json.Marshal(report)
	if err != nil {
		t.Fatal(err)
	}
	var replay Report
	if err = json.Unmarshal(b, &replay); err != nil {
		t.Fatal(err)
	}
	if replay.Pairs[0] != report.Pairs[0] {
		t.Fatalf("round trip changed observations: %+v", replay.Pairs)
	}
	got, err := Analyze(replay.Pairs, replay.Config.ConfidencePercent, replay.Config.MinimumFactor)
	if err != nil {
		t.Fatal(err)
	}
	if got.Ratio != .9 || got.Verdict != "inconclusive" {
		t.Fatalf("bad replay: %+v", got)
	}
}

func FuzzAnalyzeSinglePair(F *testing.F) {
	F.Add(uint64(3), uint64(4))
	F.Add(uint64(100), uint64(95))
	F.Fuzz(func(t *testing.T, a, b uint64) {
		if a == 0 || b == 0 || a > math.MaxInt64 || b > math.MaxInt64 {
			return
		}
		result, err := Analyze([]Pair{{BaselineNS: int64(a), CandidateNS: int64(b), Order: "baseline-first"}}, 95, 1.05)
		if err != nil {
			t.Fatal(err)
		}
		if math.IsNaN(result.Ratio) || math.IsInf(result.Ratio, 0) {
			t.Fatal("non-finite ratio")
		}
	})
}

func FuzzAnalyzePairSetInvariants(F *testing.F) {
	F.Add([]byte{1, 4, 9, 16, 25, 36, 49, 64, 81, 2, 3, 5, 7, 11, 13, 17, 19, 23})
	F.Add([]byte{255, 0, 128, 64, 32, 16, 8, 4, 2, 1, 3, 5, 7, 9, 11, 13, 15, 17})
	F.Fuzz(func(t *testing.T, data []byte) {
		if len(data) == 0 {
			return
		}
		pairs := make([]Pair, 9)
		for i := range pairs {
			b := int64(1 + data[(2*i)%len(data)]%200)
			c := int64(1 + data[(2*i+1)%len(data)]%200)
			order := "baseline-first"
			if i%2 != 0 {
				order = "candidate-first"
			}
			pairs[i] = Pair{BaselineNS: b, CandidateNS: c, Order: order}
		}
		got, err := Analyze(pairs, 95, 1.05)
		if err != nil {
			t.Fatal(err)
		}
		if got.Interval == nil {
			t.Fatal("nine observations should yield a 95% interval")
		}
		reversed := append([]Pair(nil), pairs...)
		for i, j := 0, len(reversed)-1; i < j; i, j = i+1, j-1 {
			reversed[i], reversed[j] = reversed[j], reversed[i]
		}
		permuted, err := Analyze(reversed, 95, 1.05)
		if err != nil {
			t.Fatal(err)
		}
		if permuted.Ratio != got.Ratio || permuted.Interval == nil || *permuted.Interval != *got.Interval || permuted.Verdict != got.Verdict {
			t.Fatal("pair permutation changed analysis")
		}
		scaled := make([]Pair, len(pairs))
		for i, p := range pairs {
			scaled[i] = Pair{BaselineNS: p.BaselineNS * 3, CandidateNS: p.CandidateNS * 3, Order: p.Order}
		}
		scaledAnalysis, err := Analyze(scaled, 95, 1.05)
		if err != nil {
			t.Fatal(err)
		}
		if math.Abs(scaledAnalysis.Ratio-got.Ratio) > 1e-12 || scaledAnalysis.Interval == nil || math.Abs(scaledAnalysis.Interval[0]-got.Interval[0]) > 1e-12 || math.Abs(scaledAnalysis.Interval[1]-got.Interval[1]) > 1e-12 || scaledAnalysis.Verdict != got.Verdict {
			t.Fatal("common scaling changed analysis")
		}
	})
}
