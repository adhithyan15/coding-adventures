/*
Multi-Variable Linear Regression: House Price Predictor
-------------------------------------------------------
This standalone program practically applies the custom-built 'matrix' package 
to run a comprehensive multi-variable Gradient Descent loop natively. It showcases
structuring inputs natively, performing matrix dot products gracefully, and scaling mathematical errors correctly.
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
	// X represents our data grid. Each row is a discrete real-estate property cleanly structured.
	// Feature 1: SqFt (in 1000s) | Feature 2: Bedrooms
	X := matrix.New2D([][]float64{
		{2.0, 3.0}, // House 1
		{1.5, 2.0}, // House 2
		{2.5, 4.0}, // House 3
		{1.0, 1.0}, // House 4
	})

	// 2. The Target Labels (Y)
	// Price boundaries defined dynamically in $1000s natively.
	Y := matrix.New2D([][]float64{
		{400.0},
		{300.0},
		{500.0},
		{200.0},
	})
	
	// 3. Model Parameters
	// W encapsulates the dimensional mapping vector mathematically linking Features -> Prices.
	W := matrix.New2D([][]float64{{0.5}, {0.5}})
	b := 0.5 // Scalar Bias (base-price mapping)
	
	lr := 0.01 // Learning Rate controlling mathematical slope descents iteratively

	fmt.Println("Beginning Training Epochs...")
	for epoch := 0; epoch <= 1500; epoch++ {
		
		// --- THE FORWARD PASS ---
		// We execute massive native runtime Matrix Multiplication mathematically!
		// Y_pred = (4x2 Matrix) DOT (2x1 Vector) => Generates a 4x1 Prediction block globally!
		pred, _ := X.Dot(W)
		YPred := pred.AddScalar(b)

		// --- LOSS EVALUATION ---
		// Extract raw float mappings aligning cleanly into the core loss-functions nested structure natively.
		yTrueList := make([]float64, Y.Rows)
		yPredList := make([]float64, YPred.Rows)
		for i := 0; i < Y.Rows; i++ {
			yTrueList[i] = Y.Data[i][0]
			yPredList[i] = YPred.Data[i][0]
		}
		totalLoss, _ := loss.MSE(yTrueList, yPredList)

		// --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
		// How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
		// 1. We take our original (N BY 2) Data Grid (X) and physically flip it on its side to become (2 BY N). 
		//    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
		// 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
		//    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
		errMat, _ := YPred.Subtract(Y)
		
		xT := X.Transpose()
		dotErr, _ := xT.Dot(errMat)
		
		// We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
		dW := dotErr.Scale(2.0 / float64(Y.Rows))

		// For the Bias (b), because it shifts the prediction unconditionally for every house,
		// its "share" of the blame is simply the average of all the mistakes combined!
		// We take the raw (N BY 1) Error array, sum up the N values, and scale it by 2/N.
		dbTotal := 0.0
		for i := 0; i < errMat.Rows; i++ {
			dbTotal += errMat.Data[i][0]
		}
		db := dbTotal * (2.0 / float64(Y.Rows))

		// --- OPTIMIZATION STEP ---
		// Finally, we take our original Weights and Bias and nudge them against the slope.
		// We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't 
		// overshoot the target and cause the math to explode into infinity!
		scaledDW := dW.Scale(lr)
		W, _ = W.Subtract(scaledDW)
		b = b - (db * lr)

		if epoch%150 == 0 {
			fmt.Printf("Epoch %4d | Global Loss: %10.4f | Weights [SqFt: %6.2f, Bed: %6.2f] | Bias: %6.2f\n", epoch, totalLoss, W.Data[0][0], W.Data[1][0], b)
		}
	}
	
	fmt.Println("\nFinal Optimal Mapping Achieved!")
	finalPred, _ := X.Dot(W)
	fmt.Printf("Prediction for House 1 (Target $400k): $%.2fk\n", finalPred.AddScalar(b).Data[0][0])
}
