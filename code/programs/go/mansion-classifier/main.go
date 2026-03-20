package main

import (
	"fmt"
	"github.com/adhithyan15/coding-adventures/code/packages/go/perceptron"
)

func main() {
	fmt.Println("\n--- Booting Go Mansion Classifier (OOP V2) ---")

	houseData := [][]float64{
		{4.5, 6.0}, {3.8, 5.0}, {1.5, 2.0},
		{0.9, 1.0}, {5.5, 7.0}, {2.0, 3.0},
	}
	targetData := [][]float64{
		{1.0}, {1.0}, {0.0}, {0.0}, {1.0}, {0.0},
	}

	model := perceptron.New(0.1, 2000)
	model.Fit(houseData, targetData, 400)

	fmt.Println("\n--- Final Probability Inferences ---")
	predictions := model.Predict(houseData)
	for i, prob := range predictions {
		truth := "Normal"
		if targetData[i][0] == 1.0 {
			truth = "Mansion"
		}
		guess := "Normal"
		if prob > 0.5 {
			guess = "Mansion"
		}
		fmt.Printf("House %d (Truth: %s) -> System: %s (%.2f%%)\n", i+1, truth, guess, prob*100)
	}
}
