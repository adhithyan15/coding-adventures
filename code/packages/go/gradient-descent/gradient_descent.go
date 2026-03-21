package gradientdescent

import (
	"errors"
)

// SGD implements Stochastic Gradient Descent.
// It subtracts (learning_rate * gradient) from each weight.
func SGD(weights []float64, gradients []float64, learningRate float64) ([]float64, error) {
	if len(weights) != len(gradients) || len(weights) == 0 {
		return nil, errors.New("arrays must have the same non-zero length")
	}

	newWeights := make([]float64, len(weights))
	for i := range weights {
		newWeights[i] = weights[i] - (learningRate * gradients[i])
	}

	return newWeights, nil
}
