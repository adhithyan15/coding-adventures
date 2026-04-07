package main

import (
	"fmt"
	"math"
)

func intFromInt64(value int64) (int, error) {
	if value < math.MinInt || value > math.MaxInt {
		return 0, fmt.Errorf("%d is outside the supported integer range", value)
	}
	return int(value), nil
}

func intFromFloat64(value float64) (int, error) {
	if math.IsNaN(value) || math.IsInf(value, 0) {
		return 0, fmt.Errorf("%v is not a finite integer", value)
	}

	truncated := math.Trunc(value)
	if truncated != value {
		return 0, fmt.Errorf("%v is not an integer", value)
	}

	if truncated < float64(math.MinInt) || truncated > float64(math.MaxInt) {
		return 0, fmt.Errorf("%v is outside the supported integer range", value)
	}

	return int(truncated), nil
}
