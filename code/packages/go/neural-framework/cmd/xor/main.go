// xor is a native training receipt, not a binding. Resume transports data on
// stdin, never code, paths or command arguments selected by a checkpoint.
package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"

	nn "github.com/adhithyan15/coding-adventures/code/packages/go/neural-framework"
)

type receipt struct {
	Checkpoint  nn.Checkpoint `json:"checkpoint"`
	Predictions nn.Grid       `json:"predictions"`
	InitialLoss float64       `json:"initial_loss"`
	FinalLoss   float64       `json:"final_loss"`
}

func run(args []string, input io.Reader, output io.Writer) error {
	if len(args) > 1 || (len(args) == 1 && args[0] != "--resume") {
		return errors.New("usage: xor [--resume]")
	}
	x, y := nn.Grid{{0, 0}, {0, 1}, {1, 0}, {1, 1}}, nn.Grid{{0}, {1}, {1}, {0}}
	var n *nn.Network
	var e error
	epochs := 3000
	if len(args) == 1 {
		data, err := io.ReadAll(io.LimitReader(input, nn.MaxCheckpointBytes+1))
		if err != nil {
			return err
		}
		n, e = nn.FromJSON(data)
		epochs = 1
	} else {
		n, e = nn.Seeded([]int{2, 4, 1}, []nn.Activation{nn.Tanh, nn.Sigmoid}, 7)
	}
	if e != nil {
		return e
	}
	history, e := n.Fit(x, y, epochs, 1)
	if e != nil {
		return e
	}
	predictions, e := n.Predict(x)
	if e != nil {
		return e
	}
	return json.NewEncoder(output).Encode(receipt{n.ToCheckpoint(), predictions, history[0], history[len(history)-1]})
}
func main() {
	if e := run(os.Args[1:], os.Stdin, os.Stdout); e != nil {
		fmt.Fprintln(os.Stderr, e)
		os.Exit(1)
	}
}
