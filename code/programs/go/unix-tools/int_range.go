package main

import (
	"fmt"
	"math"
)

func intFromInt64(value int64) (int, error) {
	const maxIntValue = int(^uint(0) >> 1)
	const minIntValue = -maxIntValue - 1
	if value < int64(minIntValue) || value > int64(maxIntValue) {
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

	const maxIntValue = int(^uint(0) >> 1)
	const minIntValue = -maxIntValue - 1
	if truncated < float64(minIntValue) || truncated > float64(maxIntValue) {
		return 0, fmt.Errorf("%v is outside the supported integer range", value)
	}

	return int(truncated), nil
}
