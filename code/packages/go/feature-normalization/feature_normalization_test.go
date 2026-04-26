package featurenormalization

import (
	"math"
	"testing"
)

var rows = [][]float64{
	{1000, 3, 1},
	{1500, 4, 0},
	{2000, 5, 1},
}

func assertClose(t *testing.T, expected, actual float64) {
	t.Helper()
	if math.Abs(expected-actual) > 1e-9 {
		t.Fatalf("expected %.12f, got %.12f", expected, actual)
	}
}

func TestStandardScaler(t *testing.T) {
	scaler, err := FitStandardScaler(rows)
	if err != nil {
		t.Fatal(err)
	}
	assertClose(t, 1500, scaler.Means[0])
	assertClose(t, 4, scaler.Means[1])
	assertClose(t, 2.0/3.0, scaler.Means[2])

	transformed, err := TransformStandard(rows, scaler)
	if err != nil {
		t.Fatal(err)
	}
	assertClose(t, -1.224744871391589, transformed[0][0])
	assertClose(t, 0, transformed[1][0])
	assertClose(t, 1.224744871391589, transformed[2][0])
}

func TestMinMaxScaler(t *testing.T) {
	scaler, err := FitMinMaxScaler(rows)
	if err != nil {
		t.Fatal(err)
	}
	transformed, err := TransformMinMax(rows, scaler)
	if err != nil {
		t.Fatal(err)
	}

	assertClose(t, 0, transformed[0][0])
	assertClose(t, 0.5, transformed[1][0])
	assertClose(t, 1, transformed[2][0])
	assertClose(t, 0, transformed[1][2])
}

func TestConstantColumnsMapToZero(t *testing.T) {
	constantRows := [][]float64{{1, 7}, {2, 7}}

	standardScaler, _ := FitStandardScaler(constantRows)
	standard, _ := TransformStandard(constantRows, standardScaler)
	assertClose(t, 0, standard[0][1])
	assertClose(t, 0, standard[1][1])

	minMaxScaler, _ := FitMinMaxScaler(constantRows)
	minMax, _ := TransformMinMax(constantRows, minMaxScaler)
	assertClose(t, 0, minMax[0][1])
	assertClose(t, 0, minMax[1][1])
}
