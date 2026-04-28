package featurenormalization

import (
	"errors"
	"math"
)

type StandardScaler struct {
	Means              []float64
	StandardDeviations []float64
}

type MinMaxScaler struct {
	Minimums []float64
	Maximums []float64
}

func validateMatrix(rows [][]float64) (int, error) {
	if len(rows) == 0 || len(rows[0]) == 0 {
		return 0, errors.New("matrix must have at least one row and one column")
	}
	width := len(rows[0])
	for _, row := range rows {
		if len(row) != width {
			return 0, errors.New("all rows must have the same number of columns")
		}
	}
	return width, nil
}

func FitStandardScaler(rows [][]float64) (StandardScaler, error) {
	width, err := validateMatrix(rows)
	if err != nil {
		return StandardScaler{}, err
	}

	means := make([]float64, width)
	for _, row := range rows {
		for col, value := range row {
			means[col] += value
		}
	}
	for col := range means {
		means[col] /= float64(len(rows))
	}

	stds := make([]float64, width)
	for _, row := range rows {
		for col, value := range row {
			diff := value - means[col]
			stds[col] += diff * diff
		}
	}
	for col := range stds {
		stds[col] = math.Sqrt(stds[col] / float64(len(rows)))
	}

	return StandardScaler{Means: means, StandardDeviations: stds}, nil
}

func TransformStandard(rows [][]float64, scaler StandardScaler) ([][]float64, error) {
	width, err := validateMatrix(rows)
	if err != nil {
		return nil, err
	}
	if width != len(scaler.Means) || width != len(scaler.StandardDeviations) {
		return nil, errors.New("matrix width must match scaler width")
	}

	out := make([][]float64, len(rows))
	for rowIndex, row := range rows {
		out[rowIndex] = make([]float64, width)
		for col, value := range row {
			if scaler.StandardDeviations[col] == 0 {
				out[rowIndex][col] = 0
			} else {
				out[rowIndex][col] = (value - scaler.Means[col]) / scaler.StandardDeviations[col]
			}
		}
	}
	return out, nil
}

func FitMinMaxScaler(rows [][]float64) (MinMaxScaler, error) {
	width, err := validateMatrix(rows)
	if err != nil {
		return MinMaxScaler{}, err
	}

	minimums := append([]float64(nil), rows[0]...)
	maximums := append([]float64(nil), rows[0]...)
	for _, row := range rows[1:] {
		for col, value := range row {
			minimums[col] = math.Min(minimums[col], value)
			maximums[col] = math.Max(maximums[col], value)
		}
	}

	if len(minimums) != width {
		return MinMaxScaler{}, errors.New("matrix width must match scaler width")
	}
	return MinMaxScaler{Minimums: minimums, Maximums: maximums}, nil
}

func TransformMinMax(rows [][]float64, scaler MinMaxScaler) ([][]float64, error) {
	width, err := validateMatrix(rows)
	if err != nil {
		return nil, err
	}
	if width != len(scaler.Minimums) || width != len(scaler.Maximums) {
		return nil, errors.New("matrix width must match scaler width")
	}

	out := make([][]float64, len(rows))
	for rowIndex, row := range rows {
		out[rowIndex] = make([]float64, width)
		for col, value := range row {
			span := scaler.Maximums[col] - scaler.Minimums[col]
			if span == 0 {
				out[rowIndex][col] = 0
			} else {
				out[rowIndex][col] = (value - scaler.Minimums[col]) / span
			}
		}
	}
	return out, nil
}
