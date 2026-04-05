package main

import (
	"fmt"
	"math"
)

func intFromInt64(value int64) (int, error) {
	converted := int(value)
	if int64(converted) != value {
		return 0, fmt.Errorf("%d is outside the supported integer range", value)
	}
	return converted, nil
}

func intFromFloat64(value float64) (int, error) {
	if math.IsNaN(value) || math.IsInf(value, 0) {
		return 0, fmt.Errorf("%v is not a finite integer", value)
	}

	truncated := math.Trunc(value)
	if truncated != value {
		return 0, fmt.Errorf("%v is not an integer", value)
	}

	asInt64 := int64(truncated)
	if float64(asInt64) != truncated {
		return 0, fmt.Errorf("%v is outside the supported integer range", value)
	}

	converted := int(asInt64)
	if int64(converted) != asInt64 {
		return 0, fmt.Errorf("%v is outside the supported integer range", value)
	}

	return converted, nil
}
