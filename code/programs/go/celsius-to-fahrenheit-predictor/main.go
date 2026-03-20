package main

import (
	"fmt"
	"log"

	opt "github.com/adhithyan15/coding-adventures/code/packages/go/gradient-descent"
	loss "github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions"
)

func train(lossName string, lossFn func([]float64, []float64) (float64, error), lossDerivFn func([]float64, []float64) ([]float64, error), learningRate float64, maxEpochs int) {
	celsius := []float64{-40.0, -10.0, 0.0, 8.0, 15.0, 22.0, 38.0}
	fahrenheit := []float64{-40.0, 14.0, 32.0, 46.4, 59.0, 71.6, 100.4}

	w := 0.5
	b := 0.5

	fmt.Printf("\n--- Celsius to Fahrenheit Predictor: Training with %s ---\n", lossName)

	for epoch := 0; epoch < maxEpochs; epoch++ {
		yPred := make([]float64, len(celsius))
		for i, c := range celsius {
			yPred[i] = w*c + b
		}

		errVal, err := lossFn(fahrenheit, yPred)
		if err != nil {
			log.Fatal(err)
		}

		if errVal < 0.5 {
			fmt.Printf("Converged beautifully in %d epochs! (Loss: %.6f)\n", epoch+1, errVal)
			fmt.Printf("Final Formula: F = C * %.6f + %.6f\n", w, b)
			break
		}

		gradients, err := lossDerivFn(fahrenheit, yPred)
		if err != nil {
			log.Fatal(err)
		}

		gradW := 0.0
		gradB := 0.0
		for i, g := range gradients {
			gradW += g * celsius[i]
			gradB += g
		}

		newParams, err := opt.SGD([]float64{w, b}, []float64{gradW, gradB}, learningRate)
		if err != nil {
			log.Fatal(err)
		}
		
		w = newParams[0]
		b = newParams[1]

		if (epoch+1)%1000 == 0 {
			fmt.Printf("Epoch %04d -> Loss: %.6f | w: %.4f | b: %.4f\n", epoch+1, errVal, w, b)
		}
	}

	testC := 100.0
	predF := w*testC + b
	fmt.Printf("Prediction for 100.0 C -> %.2f F (Expected ~212.00 F)\n", predF)
}

func main() {
	train("Mean Squared Error (MSE)", loss.MSE, loss.MSED, 0.0005, 10000)
	train("Mean Absolute Error (MAE)", loss.MAE, loss.MAED, 0.01, 10000)
}
