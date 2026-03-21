/*
Multi-Variable Linear Regression: House Price Predictor
-------------------------------------------------------
This standalone program practically applies the custom-built 'matrix' package 
to run a comprehensive multi-variable Gradient Descent loop natively using explicit
literate programming variables that explain exactly what the calculations represent physically.
*/
package main

import (
	"fmt"
	loss "github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions"
	"github.com/adhithyan15/coding-adventures/code/packages/go/matrix"
)

func main() {
	fmt.Println("\n--- Booting Multi-Variable Predictor: House Prices ---\n")

	// 1. The Dataset (Inputs)
	houseFeatures := matrix.New2D([][]float64{
		{2.0, 3.0}, // House 1: 2000 SqFt, 3 Beds
		{1.5, 2.0}, // House 2: 1500 SqFt, 2 Beds
		{2.5, 4.0}, // House 3: 2500 SqFt, 4 Beds
		{1.0, 1.0}, // House 4: 1000 SqFt, 1 Bed
	})

	// 2. The Target Labels (Y)
	truePrices := matrix.New2D([][]float64{
		{400.0},
		{300.0},
		{500.0},
		{200.0},
	})
	
	// 3. Model Parameters mapping Features to Prices.
	featureWeights := matrix.New2D([][]float64{{0.5}, {0.5}})
	basePriceBias := 0.5 
	learningRate := 0.01

	fmt.Println("Beginning Training Epochs...")
	for epoch := 0; epoch <= 1500; epoch++ {
		
		// --- THE FORWARD PASS ---
		// houseFeatures (4x2 Matrix) DOT featureWeights (2x1 Vector) => Generates a 4x1 Prediction block!
		rawPredictions, _ := houseFeatures.Dot(featureWeights)
		finalPredictions := rawPredictions.AddScalar(basePriceBias)

		// --- LOSS EVALUATION ---
		linearTruePrices := make([]float64, truePrices.Rows)
		linearPredictions := make([]float64, finalPredictions.Rows)
		for i := 0; i < truePrices.Rows; i++ {
			linearTruePrices[i] = truePrices.Data[i][0]
			linearPredictions[i] = finalPredictions.Data[i][0]
		}
		meanSquaredError, _ := loss.MSE(linearTruePrices, linearPredictions)

		// --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
		// How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
		// 1. We take our original (N BY 2) Data Grid and physically flip it on its side to become (2 BY N). 
		//    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
		// 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
		//    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
		predictionErrors, _ := finalPredictions.Subtract(truePrices)
		
		transposedFeatures := houseFeatures.Transpose()
		featuresDotErrors, _ := transposedFeatures.Dot(predictionErrors)
		
		// We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
		weightGradients := featuresDotErrors.Scale(2.0 / float64(truePrices.Rows))

		// For the Bias, because it shifts the prediction unconditionally for every house,
		// its "share" of the blame is simply the average of all the mistakes combined!
		biasGradientTotal := 0.0
		for i := 0; i < predictionErrors.Rows; i++ {
			biasGradientTotal += predictionErrors.Data[i][0]
		}
		biasGradient := biasGradientTotal * (2.0 / float64(truePrices.Rows))

		// --- OPTIMIZATION STEP ---
		// Finally, we take our original Weights and Bias and nudge them against the slope.
		// We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't explode to infinity.
		scaledWeightGradients := weightGradients.Scale(learningRate)
		featureWeights, _ = featureWeights.Subtract(scaledWeightGradients)
		basePriceBias = basePriceBias - (biasGradient * learningRate)

		if epoch%150 == 0 {
			fmt.Printf("Epoch %4d | Global Loss: %10.4f | Weights [SqFt: %6.2f, Bed: %6.2f] | Bias: %6.2f\n", epoch, meanSquaredError, featureWeights.Data[0][0], featureWeights.Data[1][0], basePriceBias)
		}
	}
	
	fmt.Println("\nFinal Optimal Mapping Achieved!")
	finalPred, _ := houseFeatures.Dot(featureWeights)
	fmt.Printf("Prediction for House 1 (Target $400k): $%.2fk\n", finalPred.AddScalar(basePriceBias).Data[0][0])
}
