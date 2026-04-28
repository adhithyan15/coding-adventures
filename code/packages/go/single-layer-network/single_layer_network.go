package singlelayernetwork

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

type TrainingStep struct {
	Predictions     Matrix
	Errors          Matrix
	WeightGradients Matrix
	BiasGradients   []float64
	NextWeights     Matrix
	NextBiases      []float64
	Loss            float64
}

type SingleLayerNetwork struct {
	Weights    Matrix
	Biases     []float64
	Activation ActivationName
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

func activate(value float64, activation ActivationName) (float64, error) {
	switch activation {
	case Linear:
		return value, nil
	case Sigmoid:
		if value >= 0 {
			z := math.Exp(-value)
			return 1.0 / (1.0 + z), nil
		}
		z := math.Exp(value)
		return z / (1.0 + z), nil
	default:
		return 0, fmt.Errorf("unsupported activation: %s", activation)
	}
}

func derivativeFromOutput(output float64, activation ActivationName) (float64, error) {
	switch activation {
	case Linear:
		return 1.0, nil
	case Sigmoid:
		return output * (1.0 - output), nil
	default:
		return 0, fmt.Errorf("unsupported activation: %s", activation)
	}
}

func PredictWithParameters(inputs Matrix, weights Matrix, biases []float64, activation ActivationName) (Matrix, error) {
	samples, inputCount, err := validateMatrix("inputs", inputs)
	if err != nil {
		return nil, err
	}
	weightRows, outputCount, err := validateMatrix("weights", weights)
	if err != nil {
		return nil, err
	}
	if inputCount != weightRows {
		return nil, errors.New("input column count must match weight row count")
	}
	if len(biases) != outputCount {
		return nil, errors.New("bias count must match output count")
	}

	predictions := make(Matrix, samples)
	for row := 0; row < samples; row++ {
		predictions[row] = make([]float64, outputCount)
		for output := 0; output < outputCount; output++ {
			total := biases[output]
			for input := 0; input < inputCount; input++ {
				total += inputs[row][input] * weights[input][output]
			}
			predictions[row][output], err = activate(total, activation)
			if err != nil {
				return nil, err
			}
		}
	}
	return predictions, nil
}

func TrainOneEpochWithMatrices(inputs Matrix, targets Matrix, weights Matrix, biases []float64, learningRate float64, activation ActivationName) (TrainingStep, error) {
	samples, inputCount, err := validateMatrix("inputs", inputs)
	if err != nil {
		return TrainingStep{}, err
	}
	targetRows, outputCount, err := validateMatrix("targets", targets)
	if err != nil {
		return TrainingStep{}, err
	}
	weightRows, weightCols, err := validateMatrix("weights", weights)
	if err != nil {
		return TrainingStep{}, err
	}
	if targetRows != samples {
		return TrainingStep{}, errors.New("inputs and targets must have the same row count")
	}
	if weightRows != inputCount || weightCols != outputCount {
		return TrainingStep{}, errors.New("weights must be shaped input_count x output_count")
	}
	if len(biases) != outputCount {
		return TrainingStep{}, errors.New("bias count must match output count")
	}

	predictions, err := PredictWithParameters(inputs, weights, biases, activation)
	if err != nil {
		return TrainingStep{}, err
	}
	scale := 2.0 / float64(samples*outputCount)
	errorsMatrix := make(Matrix, samples)
	deltas := make(Matrix, samples)
	lossTotal := 0.0
	for row := 0; row < samples; row++ {
		errorsMatrix[row] = make([]float64, outputCount)
		deltas[row] = make([]float64, outputCount)
		for output := 0; output < outputCount; output++ {
			predictionError := predictions[row][output] - targets[row][output]
			derivative, err := derivativeFromOutput(predictions[row][output], activation)
			if err != nil {
				return TrainingStep{}, err
			}
			errorsMatrix[row][output] = predictionError
			deltas[row][output] = scale * predictionError * derivative
			lossTotal += predictionError * predictionError
		}
	}

	weightGradients := make(Matrix, inputCount)
	nextWeights := make(Matrix, inputCount)
	for input := 0; input < inputCount; input++ {
		weightGradients[input] = make([]float64, outputCount)
		nextWeights[input] = make([]float64, outputCount)
		for output := 0; output < outputCount; output++ {
			for row := 0; row < samples; row++ {
				weightGradients[input][output] += inputs[row][input] * deltas[row][output]
			}
			nextWeights[input][output] = weights[input][output] - learningRate*weightGradients[input][output]
		}
	}

	biasGradients := make([]float64, outputCount)
	nextBiases := make([]float64, outputCount)
	for output := 0; output < outputCount; output++ {
		for row := 0; row < samples; row++ {
			biasGradients[output] += deltas[row][output]
		}
		nextBiases[output] = biases[output] - learningRate*biasGradients[output]
	}

	return TrainingStep{
		Predictions:     predictions,
		Errors:          errorsMatrix,
		WeightGradients: weightGradients,
		BiasGradients:   biasGradients,
		NextWeights:     nextWeights,
		NextBiases:      nextBiases,
		Loss:            lossTotal / float64(samples*outputCount),
	}, nil
}

func New(inputCount int, outputCount int, activation ActivationName) SingleLayerNetwork {
	weights := make(Matrix, inputCount)
	for input := 0; input < inputCount; input++ {
		weights[input] = make([]float64, outputCount)
	}
	return SingleLayerNetwork{Weights: weights, Biases: make([]float64, outputCount), Activation: activation}
}

func (network *SingleLayerNetwork) Predict(inputs Matrix) (Matrix, error) {
	return PredictWithParameters(inputs, network.Weights, network.Biases, network.Activation)
}

func (network *SingleLayerNetwork) Fit(inputs Matrix, targets Matrix, learningRate float64, epochs int) ([]TrainingStep, error) {
	history := make([]TrainingStep, 0, epochs)
	for epoch := 0; epoch < epochs; epoch++ {
		step, err := TrainOneEpochWithMatrices(inputs, targets, network.Weights, network.Biases, learningRate, network.Activation)
		if err != nil {
			return nil, err
		}
		network.Weights = step.NextWeights
		network.Biases = step.NextBiases
		history = append(history, step)
	}
	return history, nil
}

func FitSingleLayerNetwork(inputs Matrix, targets Matrix, learningRate float64, epochs int, activation ActivationName) (SingleLayerNetwork, []TrainingStep, error) {
	_, inputCount, err := validateMatrix("inputs", inputs)
	if err != nil {
		return SingleLayerNetwork{}, nil, err
	}
	_, outputCount, err := validateMatrix("targets", targets)
	if err != nil {
		return SingleLayerNetwork{}, nil, err
	}
	network := New(inputCount, outputCount, activation)
	history, err := network.Fit(inputs, targets, learningRate, epochs)
	return network, history, err
}
