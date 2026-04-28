/*
Multi-Variable Linear Regression: House Price Predictor
-------------------------------------------------------
Demonstrates n input features -> one output with feature normalization and a
short learning-rate sweep before the full training run.
*/
package main

import (
	"fmt"
	"math"

	norm "github.com/adhithyan15/coding-adventures/code/packages/go/feature-normalization"
	loss "github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions"
	"github.com/adhithyan15/coding-adventures/code/packages/go/matrix"
)

type trainingResult struct {
	learningRate float64
	loss         float64
	diverged     bool
	weights      *matrix.Matrix
	bias         float64
}

var houseFeaturesData = [][]float64{
	{2000.0, 3.0},
	{1500.0, 2.0},
	{2500.0, 4.0},
	{1000.0, 1.0},
}

var truePricesData = [][]float64{
	{400.0},
	{300.0},
	{500.0},
	{200.0},
}

func runTraining(featuresData [][]float64, pricesData [][]float64, learningRate float64, epochs int, logEvery int) trainingResult {
	houseFeatures := matrix.New2D(featuresData)
	truePrices := matrix.New2D(pricesData)
	featureWeights := matrix.New2D([][]float64{{0.5}, {0.5}})
	basePriceBias := 0.0
	lastLoss := math.Inf(1)

	for epoch := 0; epoch <= epochs; epoch++ {
		rawPredictions, _ := houseFeatures.Dot(featureWeights)
		finalPredictions := rawPredictions.AddScalar(basePriceBias)

		linearTruePrices := make([]float64, truePrices.Rows)
		linearPredictions := make([]float64, finalPredictions.Rows)
		for i := 0; i < truePrices.Rows; i++ {
			linearTruePrices[i] = truePrices.Data[i][0]
			linearPredictions[i] = finalPredictions.Data[i][0]
		}
		lastLoss, _ = loss.MSE(linearTruePrices, linearPredictions)

		if math.IsInf(lastLoss, 0) || math.IsNaN(lastLoss) || lastLoss > 1.0e12 {
			return trainingResult{learningRate: learningRate, loss: math.Inf(1), diverged: true, weights: featureWeights, bias: basePriceBias}
		}

		if logEvery > 0 && epoch%logEvery == 0 {
			fmt.Printf("Epoch %4d | Loss: %10.4f | Weights [SqFt: %7.3f, Beds: %7.3f] | Bias: %7.3f\n",
				epoch, lastLoss, featureWeights.Data[0][0], featureWeights.Data[1][0], basePriceBias)
		}

		predictionErrors, _ := finalPredictions.Subtract(truePrices)
		weightGradients, _ := houseFeatures.Transpose().Dot(predictionErrors)
		weightGradients = weightGradients.Scale(2.0 / float64(truePrices.Rows))

		biasGradientTotal := 0.0
		for i := 0; i < predictionErrors.Rows; i++ {
			biasGradientTotal += predictionErrors.Data[i][0]
		}
		biasGradient := biasGradientTotal * (2.0 / float64(truePrices.Rows))

		featureWeights, _ = featureWeights.Subtract(weightGradients.Scale(learningRate))
		basePriceBias -= biasGradient * learningRate
	}

	return trainingResult{learningRate: learningRate, loss: lastLoss, diverged: false, weights: featureWeights, bias: basePriceBias}
}

func findLearningRate(featuresData [][]float64, pricesData [][]float64) trainingResult {
	candidates := []float64{0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.6}
	results := make([]trainingResult, 0, len(candidates))

	fmt.Println("\nShort learning-rate sweep over normalized features:")
	for _, learningRate := range candidates {
		result := runTraining(featuresData, pricesData, learningRate, 120, 0)
		results = append(results, result)
		if result.diverged {
			fmt.Printf("  lr=%-6g -> loss=diverged\n", learningRate)
		} else {
			fmt.Printf("  lr=%-6g -> loss=%.4f\n", learningRate, result.loss)
		}
	}

	best := results[0]
	for _, result := range results[1:] {
		if !result.diverged && (best.diverged || result.loss < best.loss) {
			best = result
		}
	}
	return best
}

func main() {
	fmt.Println("\n--- Booting Multi-Variable Predictor: House Prices ---")
	fmt.Println("Features: square footage and bedroom count. Target: price in $1000s.")

	scaler, err := norm.FitStandardScaler(houseFeaturesData)
	if err != nil {
		panic(err)
	}
	normalizedFeatures, err := norm.TransformStandard(houseFeaturesData, scaler)
	if err != nil {
		panic(err)
	}

	bestTrial := findLearningRate(normalizedFeatures, truePricesData)
	fmt.Printf("\nSelected learning rate: %g\n", bestTrial.learningRate)
	fmt.Println("Beginning full training run...")
	finalResult := runTraining(normalizedFeatures, truePricesData, bestTrial.learningRate, 1500, 150)

	fmt.Println("\nFinal Optimal Mapping Achieved!")
	normalizedTestHouse, err := norm.TransformStandard([][]float64{{2000.0, 3.0}}, scaler)
	if err != nil {
		panic(err)
	}
	prediction, _ := matrix.New2D(normalizedTestHouse).Dot(finalResult.weights)
	fmt.Printf("Prediction for House 1 (Target $400k): $%.2fk\n", prediction.AddScalar(finalResult.bias).Data[0][0])
}
