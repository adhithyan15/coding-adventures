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

		// --- BACKPROPAGATION (CALCULUS) ---
		// Error derivatives structurally scale against dimensional inversions properly:
		// dW = X^T . (Y_pred - Y) * (2/N)
		errMat, _ := YPred.Subtract(Y)
		
		// Transpose shifts a (4x2) Matrix natively into a (2x4) framework to multiply the error vectors dimensionally.
		xT := X.Transpose()
		dotErr, _ := xT.Dot(errMat)
		
		// Scale handles dividing the aggregate matrix gradients flawlessly natively!
		dW := dotErr.Scale(2.0 / float64(Y.Rows))

		dbTotal := 0.0
		for i := 0; i < errMat.Rows; i++ {
			dbTotal += errMat.Data[i][0]
		}
		db := dbTotal * (2.0 / float64(Y.Rows))

		// --- OPTIMIZATION STEP ---
		// Descend functionally inverse strictly into the mathematical valley correctly efficiently.
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
