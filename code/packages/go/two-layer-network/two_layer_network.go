package twolayernetwork

import (
	"errors"
	"fmt"
	"math"
)

const Version = "0.1.0"

type Matrix [][]float64

type ActivationName string

const (
	Linear  ActivationName = "linear"
	Sigmoid ActivationName = "sigmoid"
)

type Parameters struct {
	InputToHiddenWeights  Matrix
	HiddenBiases          []float64
	HiddenToOutputWeights Matrix
	OutputBiases          []float64
}

type ForwardPass struct {
	HiddenRaw         Matrix
	HiddenActivations Matrix
	OutputRaw         Matrix
	Predictions       Matrix
}

type TrainingStep struct {
	Predictions                   Matrix
	Errors                        Matrix
	OutputDeltas                  Matrix
	HiddenDeltas                  Matrix
	HiddenToOutputWeightGradients Matrix
	OutputBiasGradients           []float64
	InputToHiddenWeightGradients  Matrix
	HiddenBiasGradients           []float64
	NextParameters                Parameters
	Loss                          float64
}

type Network struct {
	Parameters       Parameters
	LearningRate     float64
	HiddenActivation ActivationName
	OutputActivation ActivationName
}

func validateMatrix(name string, matrix Matrix) (int, int, error) {
	if len(matrix) == 0 {
		return 0, 0, fmt.Errorf("%s must contain at least one row", name)
	}
	width := len(matrix[0])
	if width == 0 {
		return 0, 0, fmt.Errorf("%s must contain at least one column", name)
	}
	for _, row := range matrix {
		if len(row) != width {
			return 0, 0, fmt.Errorf("%s must be rectangular", name)
		}
	}
	return len(matrix), width, nil
}

func sigmoid(value float64) float64 {
	if value >= 0 {
		z := math.Exp(-value)
		return 1.0 / (1.0 + z)
	}
	z := math.Exp(value)
	return z / (1.0 + z)
}

func activate(value float64, activation ActivationName) (float64, error) {
	switch activation {
	case Linear:
		return value, nil
	case Sigmoid:
		return sigmoid(value), nil
	default:
		return 0, fmt.Errorf("unsupported activation: %s", activation)
	}
}

func derivative(raw float64, activated float64, activation ActivationName) (float64, error) {
	switch activation {
	case Linear:
		return 1.0, nil
	case Sigmoid:
		return activated * (1.0 - activated), nil
	default:
		return 0, fmt.Errorf("unsupported activation: %s", activation)
	}
}

func dot(left Matrix, right Matrix) (Matrix, error) {
	leftRows, leftCols, err := validateMatrix("left", left)
	if err != nil {
		return nil, err
	}
	rightRows, rightCols, err := validateMatrix("right", right)
	if err != nil {
		return nil, err
	}
	if leftCols != rightRows {
		return nil, errors.New("matrix shapes do not align")
	}
	result := make(Matrix, leftRows)
	for row := 0; row < leftRows; row++ {
		result[row] = make([]float64, rightCols)
		for col := 0; col < rightCols; col++ {
			for k := 0; k < leftCols; k++ {
				result[row][col] += left[row][k] * right[k][col]
			}
		}
	}
	return result, nil
}

func transpose(matrix Matrix) Matrix {
	rows, cols, _ := validateMatrix("matrix", matrix)
	result := make(Matrix, cols)
	for col := 0; col < cols; col++ {
		result[col] = make([]float64, rows)
		for row := 0; row < rows; row++ {
			result[col][row] = matrix[row][col]
		}
	}
	return result
}

func addBiases(matrix Matrix, biases []float64) Matrix {
	result := make(Matrix, len(matrix))
	for row := range matrix {
		result[row] = make([]float64, len(matrix[row]))
		for col, value := range matrix[row] {
			result[row][col] = value + biases[col]
		}
	}
	return result
}

func applyActivation(matrix Matrix, activation ActivationName) (Matrix, error) {
	result := make(Matrix, len(matrix))
	for row := range matrix {
		result[row] = make([]float64, len(matrix[row]))
		for col, value := range matrix[row] {
			activated, err := activate(value, activation)
			if err != nil {
				return nil, err
			}
			result[row][col] = activated
		}
	}
	return result, nil
}

func columnSums(matrix Matrix) []float64 {
	_, cols, _ := validateMatrix("matrix", matrix)
	sums := make([]float64, cols)
	for _, row := range matrix {
		for col := 0; col < cols; col++ {
			sums[col] += row[col]
		}
	}
	return sums
}

func meanSquaredError(errors Matrix) float64 {
	total := 0.0
	count := 0
	for _, row := range errors {
		for _, value := range row {
			total += value * value
			count++
		}
	}
	return total / float64(count)
}

func subtractScaled(matrix Matrix, gradients Matrix, learningRate float64) Matrix {
	result := make(Matrix, len(matrix))
	for row := range matrix {
		result[row] = make([]float64, len(matrix[row]))
		for col, value := range matrix[row] {
			result[row][col] = value - learningRate*gradients[row][col]
		}
	}
	return result
}

func XorWarmStartParameters() Parameters {
	return Parameters{
		InputToHiddenWeights:  Matrix{{4, -4}, {4, -4}},
		HiddenBiases:          []float64{-2, 6},
		HiddenToOutputWeights: Matrix{{4}, {4}},
		OutputBiases:          []float64{-6},
	}
}

func Forward(inputs Matrix, parameters Parameters, hiddenActivation ActivationName, outputActivation ActivationName) (ForwardPass, error) {
	_, inputCount, err := validateMatrix("inputs", inputs)
	if err != nil {
		return ForwardPass{}, err
	}
	weightRows, hiddenCount, err := validateMatrix("input-to-hidden weights", parameters.InputToHiddenWeights)
	if err != nil {
		return ForwardPass{}, err
	}
	hiddenRows, outputCount, err := validateMatrix("hidden-to-output weights", parameters.HiddenToOutputWeights)
	if err != nil {
		return ForwardPass{}, err
	}
	if inputCount != weightRows {
		return ForwardPass{}, errors.New("input width must match input-to-hidden weight row count")
	}
	if len(parameters.HiddenBiases) != hiddenCount {
		return ForwardPass{}, errors.New("hidden bias count must match hidden width")
	}
	if hiddenCount != hiddenRows {
		return ForwardPass{}, errors.New("hidden width must match hidden-to-output weight row count")
	}
	if len(parameters.OutputBiases) != outputCount {
		return ForwardPass{}, errors.New("output bias count must match output width")
	}
	hiddenDot, err := dot(inputs, parameters.InputToHiddenWeights)
	if err != nil {
		return ForwardPass{}, err
	}
	hiddenRaw := addBiases(hiddenDot, parameters.HiddenBiases)
	hiddenActivations, err := applyActivation(hiddenRaw, hiddenActivation)
	if err != nil {
		return ForwardPass{}, err
	}
	outputDot, err := dot(hiddenActivations, parameters.HiddenToOutputWeights)
	if err != nil {
		return ForwardPass{}, err
	}
	outputRaw := addBiases(outputDot, parameters.OutputBiases)
	predictions, err := applyActivation(outputRaw, outputActivation)
	if err != nil {
		return ForwardPass{}, err
	}
	return ForwardPass{hiddenRaw, hiddenActivations, outputRaw, predictions}, nil
}

func TrainOneEpoch(inputs Matrix, targets Matrix, parameters Parameters, learningRate float64, hiddenActivation ActivationName, outputActivation ActivationName) (TrainingStep, error) {
	sampleCount, _, err := validateMatrix("inputs", inputs)
	if err != nil {
		return TrainingStep{}, err
	}
	targetRows, outputCount, err := validateMatrix("targets", targets)
	if err != nil {
		return TrainingStep{}, err
	}
	if targetRows != sampleCount {
		return TrainingStep{}, errors.New("inputs and targets must have the same row count")
	}
	forward, err := Forward(inputs, parameters, hiddenActivation, outputActivation)
	if err != nil {
		return TrainingStep{}, err
	}
	scale := 2.0 / float64(sampleCount*outputCount)
	errorsMatrix := make(Matrix, sampleCount)
	outputDeltas := make(Matrix, sampleCount)
	for row := 0; row < sampleCount; row++ {
		errorsMatrix[row] = make([]float64, outputCount)
		outputDeltas[row] = make([]float64, outputCount)
		for output := 0; output < outputCount; output++ {
			predictionError := forward.Predictions[row][output] - targets[row][output]
			deriv, err := derivative(forward.OutputRaw[row][output], forward.Predictions[row][output], outputActivation)
			if err != nil {
				return TrainingStep{}, err
			}
			errorsMatrix[row][output] = predictionError
			outputDeltas[row][output] = scale * predictionError * deriv
		}
	}
	h2oGradients, err := dot(transpose(forward.HiddenActivations), outputDeltas)
	if err != nil {
		return TrainingStep{}, err
	}
	outputBiasGradients := columnSums(outputDeltas)
	hiddenErrors, err := dot(outputDeltas, transpose(parameters.HiddenToOutputWeights))
	if err != nil {
		return TrainingStep{}, err
	}
	hiddenWidth := len(parameters.HiddenBiases)
	hiddenDeltas := make(Matrix, sampleCount)
	for row := 0; row < sampleCount; row++ {
		hiddenDeltas[row] = make([]float64, hiddenWidth)
		for hidden := 0; hidden < hiddenWidth; hidden++ {
			deriv, err := derivative(forward.HiddenRaw[row][hidden], forward.HiddenActivations[row][hidden], hiddenActivation)
			if err != nil {
				return TrainingStep{}, err
			}
			hiddenDeltas[row][hidden] = hiddenErrors[row][hidden] * deriv
		}
	}
	i2hGradients, err := dot(transpose(inputs), hiddenDeltas)
	if err != nil {
		return TrainingStep{}, err
	}
	hiddenBiasGradients := columnSums(hiddenDeltas)
	nextHiddenBiases := make([]float64, len(parameters.HiddenBiases))
	for index, bias := range parameters.HiddenBiases {
		nextHiddenBiases[index] = bias - learningRate*hiddenBiasGradients[index]
	}
	nextOutputBiases := make([]float64, len(parameters.OutputBiases))
	for index, bias := range parameters.OutputBiases {
		nextOutputBiases[index] = bias - learningRate*outputBiasGradients[index]
	}
	nextParameters := Parameters{
		InputToHiddenWeights:  subtractScaled(parameters.InputToHiddenWeights, i2hGradients, learningRate),
		HiddenBiases:          nextHiddenBiases,
		HiddenToOutputWeights: subtractScaled(parameters.HiddenToOutputWeights, h2oGradients, learningRate),
		OutputBiases:          nextOutputBiases,
	}
	return TrainingStep{
		Predictions:                   forward.Predictions,
		Errors:                        errorsMatrix,
		OutputDeltas:                  outputDeltas,
		HiddenDeltas:                  hiddenDeltas,
		HiddenToOutputWeightGradients: h2oGradients,
		OutputBiasGradients:           outputBiasGradients,
		InputToHiddenWeightGradients:  i2hGradients,
		HiddenBiasGradients:           hiddenBiasGradients,
		NextParameters:                nextParameters,
		Loss:                          meanSquaredError(errorsMatrix),
	}, nil
}

func New(parameters Parameters, learningRate float64) Network {
	return Network{parameters, learningRate, Sigmoid, Sigmoid}
}

func (network *Network) Fit(inputs Matrix, targets Matrix, epochs int) ([]TrainingStep, error) {
	history := make([]TrainingStep, 0, epochs)
	for epoch := 0; epoch < epochs; epoch++ {
		step, err := TrainOneEpoch(inputs, targets, network.Parameters, network.LearningRate, network.HiddenActivation, network.OutputActivation)
		if err != nil {
			return nil, err
		}
		network.Parameters = step.NextParameters
		history = append(history, step)
	}
	return history, nil
}

func (network Network) Predict(inputs Matrix) (Matrix, error) {
	pass, err := Forward(inputs, network.Parameters, network.HiddenActivation, network.OutputActivation)
	if err != nil {
		return nil, err
	}
	return pass.Predictions, nil
}
