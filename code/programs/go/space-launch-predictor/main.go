package main

import (
	"fmt"

	activation "github.com/adhithyan15/coding-adventures/code/packages/go/activation-functions"
	loss "github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions"
	"github.com/adhithyan15/coding-adventures/code/packages/go/matrix"
)

func main() {
	fmt.Println("\n--- Booting Go Space Launch Predictor ---")

	shuttleData := [][]float64{
		{12.0, 15.0}, {35.0, 85.0}, {5.0, 5.0},
		{40.0, 95.0}, {15.0, 30.0}, {28.0, 60.0},
	}
	targetData := [][]float64{
		{1.0}, {0.0}, {1.0}, {0.0}, {1.0}, {0.0},
	}

	features := matrix.New2D(shuttleData)
	trueLabels := matrix.New2D(targetData)

	weights := matrix.New2D([][]float64{{0.0}, {0.0}})
	bias := 0.0
	lr := 0.01 // Smaller LR for larger input values
	epochs := 3000

	for epoch := 0; epoch <= epochs; epoch++ {
		raw, _ := features.Dot(weights)
		raw.AddScalar(bias)

		linearProbs := make([]float64, features.Rows)
		linearTruth := make([]float64, features.Rows)
		gradData := make([][]float64, features.Rows)
		
		for i := 0; i < features.Rows; i++ {
			linearProbs[i] = activation.Sigmoid(raw.Data[i][0])
			linearTruth[i] = trueLabels.Data[i][0]
		}

		logLoss, _ := loss.BCE(linearTruth, linearProbs)
		lossGrad, _ := loss.BCED(linearTruth, linearProbs)

		var biasGrad float64
		for i := 0; i < features.Rows; i++ {
			actGrad := activation.SigmoidDerivative(raw.Data[i][0])
			combined := lossGrad[i] * actGrad
			gradData[i] = []float64{combined}
			biasGrad += combined
		}

		gradMatrix := matrix.New2D(gradData)
		transposed := features.Transpose()
		weightGrads, _ := transposed.Dot(gradMatrix)

		scaledWeights := weightGrads.Scale(lr)
		weights, _ = weights.Subtract(scaledWeights)
		bias -= biasGrad * lr

		if epoch%500 == 0 {
			fmt.Printf("Epoch %4d | BCE Loss: %.4f | Bias: %.2f\n", epoch, logLoss, bias)
		}
	}

	fmt.Println("\n--- Final Launch Probabilities ---")
    finalRaw, _ := features.Dot(weights)
    finalRaw.AddScalar(bias)
    for i := 0; i < trueLabels.Rows; i++ {
        prob := activation.Sigmoid(finalRaw.Data[i][0])
        fmt.Printf("Scenario %d Probability of Launch safely: %.2f%%\n", i+1, prob*100)
    }
}
