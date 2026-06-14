package main

import (
	"fmt"
	"log"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/neural-graph-vm"
	neuralnetwork "github.com/adhithyan15/coding-adventures/code/packages/go/neural-network"
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

	runGraphVMInference(model, houseData)
}

func runGraphVMInference(model *perceptron.Perceptron, houseData [][]float64) {
	if model.Weights == nil {
		log.Fatal("expected trained perceptron weights before graph VM inference")
	}

	network := neuralnetwork.CreateNeuralNetwork("mansion-classifier").
		Input("bedrooms").
		Input("bathrooms").
		Constant("bias", 1.0, neuralnetwork.PropertyBag{"nn.role": "bias"}).
		WeightedSum("mansion_logit", []neuralnetwork.WeightedInput{
			{From: "bedrooms", Weight: model.Weights.Data[0][0], EdgeID: "bedrooms_weight"},
			{From: "bathrooms", Weight: model.Weights.Data[1][0], EdgeID: "bathrooms_weight"},
			{From: "bias", Weight: model.Bias, EdgeID: "bias_weight"},
		}, neuralnetwork.PropertyBag{"nn.layer": "output", "nn.role": "weighted_sum"}).
		Activation("mansion_probability", "mansion_logit", neuralnetwork.Sigmoid, neuralnetwork.PropertyBag{"nn.layer": "output", "nn.role": "activation"}, "logit_to_sigmoid").
		Output("mansion_output", "mansion_probability", "mansion_probability", neuralnetwork.PropertyBag{"nn.layer": "output"}, "probability_to_output")

	bytecode, err := vm.CompileNeuralNetworkToBytecode(network)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println("\n--- Graph VM Inference ---")
	for index, house := range houseData {
		outputs, err := vm.RunNeuralBytecodeForward(bytecode, map[string]float64{
			"bedrooms":  house[0],
			"bathrooms": house[1],
		})
		if err != nil {
			log.Fatal(err)
		}
		probability := outputs["mansion_probability"]
		guess := "Normal"
		if probability > 0.5 {
			guess = "Mansion"
		}
		fmt.Printf("House %d -> VM: %s (%.2f%%)\n", index+1, guess, probability*100)
	}
}
