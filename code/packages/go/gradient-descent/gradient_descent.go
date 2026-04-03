package gradientdescent

import (
	"errors"
)

// SGD implements Stochastic Gradient Descent.
// It subtracts (learning_rate * gradient) from each weight.
func SGD(weights []float64, gradients []float64, learningRate float64) ([]float64, error) {
	return StartNew[[]float64]("gradient-descent.SGD", nil,
		func(op *Operation[[]float64], rf *ResultFactory[[]float64]) *OperationResult[[]float64] {
			op.AddProperty("learningRate", learningRate)
			op.AddProperty("numWeights", len(weights))
			if len(weights) != len(gradients) || len(weights) == 0 {
				return rf.Fail(nil, errors.New("arrays must have the same non-zero length"))
			}

			newWeights := make([]float64, len(weights))
			for i := range weights {
				newWeights[i] = weights[i] - (learningRate * gradients[i])
			}

			return rf.Generate(true, false, newWeights)
		}).GetResult()
}
