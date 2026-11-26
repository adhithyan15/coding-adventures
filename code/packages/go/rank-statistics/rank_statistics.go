// Package rankstatistics provides reusable sample medians and exact
// distribution-free confidence intervals for a population median.
package rankstatistics

import (
	"errors"
	"math"
	"math/big"
	"slices"
)

// Interval is a closed order-statistic confidence interval.
type Interval struct {
	Low  float64
	High float64
}

var errEmptySample = errors.New("sample must not be empty")
var errNonFinite = errors.New("sample values must be finite")

func sortedFinite(values []float64) ([]float64, error) {
	if len(values) == 0 {
		return nil, errEmptySample
	}
	ordered := slices.Clone(values)
	for _, value := range ordered {
		if math.IsNaN(value) || math.IsInf(value, 0) {
			return nil, errNonFinite
		}
	}
	slices.Sort(ordered)
	return ordered, nil
}

// midpoint computes the arithmetic mean of ordered finite values without
// overflowing when both values have the same sign or opposite extremes.
func midpoint(low, high float64) float64 {
	sum := low + high
	if math.IsInf(sum, 0) {
		return low/2 + high/2
	}
	return sum / 2
}

// Median returns the sample median without modifying values. An even-sized
// sample uses the arithmetic midpoint of its two central order statistics.
func Median(values []float64) (float64, error) {
	ordered, err := sortedFinite(values)
	if err != nil {
		return 0, err
	}
	middle := len(ordered) / 2
	if len(ordered)%2 == 1 {
		return ordered[middle], nil
	}
	return midpoint(ordered[middle-1], ordered[middle]), nil
}

// MedianInterval returns an exact, two-sided order-statistic interval at the
// requested confidence percentage. A nil interval means that no rank meets
// the confidence cutoff. The input slice is not modified.
func MedianInterval(values []float64, confidencePercent int) (*Interval, error) {
	if confidencePercent < 1 || confidencePercent > 99 {
		return nil, errors.New("confidence percentage must be from 1 to 99")
	}
	ordered, err := sortedFinite(values)
	if err != nil {
		return nil, err
	}
	rank := exactMedianRank(len(ordered), confidencePercent)
	if rank == 0 {
		return nil, nil
	}
	return &Interval{Low: ordered[rank-1], High: ordered[len(ordered)-rank]}, nil
}

// exactMedianRank returns the largest one-based rank satisfying
// 2*P(Binomial(n, 1/2) <= rank-1) <= 1-confidence. All cutoff arithmetic is
// exact, even for sample sizes whose binomial counts exceed machine integers.
func exactMedianRank(n, confidencePercent int) int {
	maxTail := new(big.Int).Lsh(big.NewInt(int64(100-confidencePercent)), uint(n))
	coefficient := big.NewInt(1)
	tail := new(big.Int)
	scaledTail := new(big.Int)
	twoHundred := big.NewInt(200)
	rank := 0
	for j := 0; j <= n/2; j++ {
		tail.Add(tail, coefficient)
		if scaledTail.Mul(tail, twoHundred).Cmp(maxTail) > 0 {
			break
		}
		rank = j + 1
		coefficient.Mul(coefficient, big.NewInt(int64(n-j)))
		coefficient.Quo(coefficient, big.NewInt(int64(j+1)))
	}
	return rank
}
