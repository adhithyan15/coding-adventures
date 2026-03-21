package main

import (
	"fmt"
	"github.com/adhithyan15/coding-adventures/code/packages/go/perceptron"
)

func main() {
	fmt.Println("\n--- Booting Go Space Launch Predictor (OOP V2) ---")

	shuttleData := [][]float64{
		{12.0, 15.0}, {35.0, 85.0}, {5.0, 5.0},
		{40.0, 95.0}, {15.0, 30.0}, {28.0, 60.0},
	}
	targetData := [][]float64{
		{1.0}, {0.0}, {1.0}, {0.0}, {1.0}, {0.0},
	}

	model := perceptron.New(0.01, 3000)
	model.Fit(shuttleData, targetData, 500)

	fmt.Println("\n--- Final Inference ---")
	predictions := model.Predict(shuttleData)
	for i, prob := range predictions {
		truth := "Abort"
		if targetData[i][0] == 1.0 {
			truth = "Safe"
		}
		guess := "Abort"
		if prob > 0.5 {
			guess = "Safe"
		}
		fmt.Printf("Scenario %d (Truth: %s) -> System: %s (%.2f%%)\n", i+1, truth, guess, prob*100)
	}
}
