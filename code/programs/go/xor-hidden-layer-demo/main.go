package main

import (
	"fmt"

	single "github.com/adhithyan15/coding-adventures/code/packages/go/single-layer-network"
	two "github.com/adhithyan15/coding-adventures/code/packages/go/two-layer-network"
)

var xorInputs = single.Matrix{{0, 0}, {0, 1}, {1, 0}, {1, 1}}
var xorTargets = single.Matrix{{0}, {1}, {1}, {0}}

func rounded(value float64) int {
	if value >= 0.5 {
		return 1
	}
	return 0
}

func runLinearFailure() error {
	network := single.New(2, 1, single.Sigmoid)
	history, err := network.Fit(xorInputs, xorTargets, 1.0, 50000)
	if err != nil {
		return err
	}
	predictions, err := network.Predict(xorInputs)
	if err != nil {
		return err
	}
	fmt.Printf("No hidden layer after many runs: loss %.4f\n", history[len(history)-1].Loss)
	for index, row := range predictions {
		fmt.Printf("%v target=%.0f prediction=%.4f rounded=%d\n", xorInputs[index], xorTargets[index][0], row[0], rounded(row[0]))
	}
	return nil
}

func runHiddenSuccess() error {
	inputs := two.Matrix{{0, 0}, {0, 1}, {1, 0}, {1, 1}}
	pass, err := two.Forward(inputs, two.XorWarmStartParameters(), two.Sigmoid, two.Sigmoid)
	if err != nil {
		return err
	}
	fmt.Println("\nWith one hidden layer:")
	for index, row := range pass.Predictions {
		fmt.Printf("%v target=%.0f prediction=%.4f rounded=%d hidden=[%.4f %.4f]\n",
			inputs[index], xorTargets[index][0], row[0], rounded(row[0]),
			pass.HiddenActivations[index][0], pass.HiddenActivations[index][1])
	}
	return nil
}

func main() {
	if err := runLinearFailure(); err != nil {
		panic(err)
	}
	if err := runHiddenSuccess(); err != nil {
		panic(err)
	}
}
