// Package microbenchmark collects paired batch timings and analyzes their ratios.
package microbenchmark

import (
	"errors"
	"fmt"
	"math"
	"math/rand"

	rankstatistics "github.com/adhithyan15/coding-adventures/code/packages/go/rank-statistics"
)

// Config fixes the experiment before collection begins.
type Config struct {
	Pairs             int     `json:"pairs"`
	ConfidencePercent int     `json:"confidence_percent"`
	MinimumFactor     float64 `json:"minimum_factor"`
	Seed              int64   `json:"seed"`
	TargetNS          int64   `json:"target_ns"`
	MaxBatch          int     `json:"max_batch"`
}

// Defaults returns the VER07d collection defaults.
func Defaults() Config { return Config{40, 95, 1.05, 0, 100_000, 65_536} }

// Pair is one adjacent observation pair.
type Pair struct {
	BaselineNS  int64  `json:"baseline_ns"`
	CandidateNS int64  `json:"candidate_ns"`
	Order       string `json:"order"`
}

// Interval is a closed confidence interval.
type Interval [2]float64

// Analysis summarizes candidate/baseline ratios.
type Analysis struct {
	Ratio    float64   `json:"ratio"`
	Interval *Interval `json:"interval"`
	Verdict  string    `json:"verdict"`
}

// Report preserves raw observations so the analysis can be replayed.
type Report struct {
	Config    Config   `json:"config"`
	BatchSize int      `json:"batch_size"`
	Pairs     []Pair   `json:"pairs"`
	Analysis  Analysis `json:"analysis"`
}

// Batch executes n logical operations and returns the checksum of the batch.
type Batch func(n int) (int64, error)

// Clock returns a monotonic timestamp in nanoseconds. MonotonicClock uses
// an elapsed-time source rather than wall-clock Unix time.
type Clock func() int64

// CollectOptions holds dependencies for a deterministic collection run.
type CollectOptions struct {
	Config              Config
	ExpectedChecksum    int64
	Baseline, Candidate Batch
	Now                 Clock
}

func validateConfig(c Config) error {
	if c.Pairs < 1 || c.Pairs > 10_000 {
		return errors.New("pairs must be from 1 to 10000")
	}
	if c.ConfidencePercent < 1 || c.ConfidencePercent > 99 {
		return errors.New("confidence must be from 1 to 99")
	}
	if math.IsNaN(c.MinimumFactor) || math.IsInf(c.MinimumFactor, 0) || c.MinimumFactor <= 1 {
		return errors.New("factor must be finite and greater than 1")
	}
	if c.TargetNS <= 0 || c.MaxBatch <= 0 {
		return errors.New("target and max batch must be positive")
	}
	return nil
}

// Analyze computes the sample median ratio, exact median interval, and VER07b verdict.
func Analyze(pairs []Pair, confidence int, factor float64) (Analysis, error) {
	if len(pairs) == 0 {
		return Analysis{}, errors.New("at least one pair is required")
	}
	if len(pairs) > 10_000 {
		return Analysis{}, errors.New("at most 10000 pairs are supported")
	}
	if confidence < 1 || confidence > 99 {
		return Analysis{}, errors.New("confidence must be from 1 to 99")
	}
	if math.IsNaN(factor) || math.IsInf(factor, 0) || factor <= 1 {
		return Analysis{}, errors.New("factor must be finite and greater than 1")
	}
	ratios := make([]float64, len(pairs))
	for i, p := range pairs {
		if p.BaselineNS <= 0 || p.CandidateNS <= 0 {
			return Analysis{}, fmt.Errorf("pair %d durations must be positive", i)
		}
		if p.Order != "baseline-first" && p.Order != "candidate-first" {
			return Analysis{}, fmt.Errorf("pair %d has invalid order", i)
		}
		ratios[i] = float64(p.CandidateNS) / float64(p.BaselineNS)
	}
	median, err := rankstatistics.Median(ratios)
	if err != nil {
		return Analysis{}, err
	}
	ranks, err := rankstatistics.MedianInterval(ratios, confidence)
	if err != nil {
		return Analysis{}, err
	}
	result := Analysis{Ratio: median, Verdict: "inconclusive"}
	if ranks == nil {
		return result, nil
	}
	result.Interval = &Interval{ranks.Low, ranks.High}
	fast, slow := 1/factor, factor
	switch {
	case ranks.High < fast:
		result.Verdict = "faster"
	case ranks.Low > slow:
		result.Verdict = "slower"
	case ranks.Low >= fast && ranks.High <= slow:
		result.Verdict = "equivalent"
	}
	return result, nil
}

// Collect verifies, calibrates, and measures paired batches. It returns no
// partial report on any clock, checksum, calibration, or workload failure.
func Collect(o CollectOptions) (Report, error) {
	c := o.Config
	if err := validateConfig(c); err != nil {
		return Report{}, err
	}
	if o.Baseline == nil || o.Candidate == nil || o.Now == nil {
		return Report{}, errors.New("baseline, candidate, and clock are required")
	}
	want := func(n int) (int64, error) {
		x := o.ExpectedChecksum
		y := int64(n)
		if y != 0 && (x > math.MaxInt64/y || x < math.MinInt64/y) {
			return 0, errors.New("checksum arithmetic overflow")
		}
		return x * y, nil
	}
	verify := func(fn Batch, n int) error {
		expected, err := want(n)
		if err != nil {
			return err
		}
		got, err := fn(n)
		if err != nil {
			return err
		}
		if got != expected {
			return fmt.Errorf("checksum mismatch: got %d, want %d", got, expected)
		}
		return nil
	}
	if err := verify(o.Baseline, 1); err != nil {
		return Report{}, err
	}
	if err := verify(o.Candidate, 1); err != nil {
		return Report{}, err
	}
	last := int64(-1)
	read := func() (int64, error) {
		t := o.Now()
		if t < 0 || last >= 0 && t < last {
			return 0, errors.New("clock is negative or regressed")
		}
		last = t
		return t, nil
	}
	measure := func(fn Batch, n int, calibration bool) (int64, error) {
		expected, err := want(n)
		if err != nil {
			return 0, err
		}
		a, e := read()
		if e != nil {
			return 0, e
		}
		sum, e := fn(n)
		b, ce := read()
		if e != nil {
			return 0, e
		}
		if ce != nil {
			return 0, ce
		}
		if sum != expected {
			return 0, errors.New("checksum mismatch")
		}
		if b < a || (!calibration && b == a) {
			return 0, errors.New("measured duration must be positive and clock must not regress")
		}
		return b - a, nil
	}
	size := 1
	for {
		a, e := measure(o.Baseline, size, true)
		if e != nil {
			return Report{}, e
		}
		b, e := measure(o.Candidate, size, true)
		if e != nil {
			return Report{}, e
		}
		if a >= c.TargetNS && b >= c.TargetNS {
			break
		}
		if size >= c.MaxBatch {
			return Report{}, errors.New("calibration limit exhausted")
		}
		if size > c.MaxBatch/2 {
			size = c.MaxBatch
		} else {
			size *= 2
		}
	}
	orders := make([]bool, c.Pairs)
	for i := range orders {
		orders[i] = i%2 == 0
	}
	r := rand.New(rand.NewSource(c.Seed))
	r.Shuffle(len(orders), func(i, j int) { orders[i], orders[j] = orders[j], orders[i] })
	pairs := make([]Pair, 0, c.Pairs)
	for _, baseFirst := range orders {
		var b, ca int64
		var err error
		order := "candidate-first"
		if baseFirst {
			order = "baseline-first"
			b, err = measure(o.Baseline, size, false)
			if err == nil {
				ca, err = measure(o.Candidate, size, false)
			}
		} else {
			ca, err = measure(o.Candidate, size, false)
			if err == nil {
				b, err = measure(o.Baseline, size, false)
			}
		}
		if err != nil {
			return Report{}, err
		}
		pairs = append(pairs, Pair{b, ca, order})
	}
	a, err := Analyze(pairs, c.ConfidencePercent, c.MinimumFactor)
	if err != nil {
		return Report{}, err
	}
	return Report{c, size, pairs, a}, nil
}
