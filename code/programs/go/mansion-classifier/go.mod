module github.com/adhithyan15/coding-adventures/code/programs/go/mansion-classifier

go 1.21

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-graph-vm v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-network v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/perceptron v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/activation-functions v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/matrix v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/matrix => ../../../packages/go/matrix

replace github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions => ../../../packages/go/loss-functions

replace github.com/adhithyan15/coding-adventures/code/packages/go/activation-functions => ../../../packages/go/activation-functions

replace github.com/adhithyan15/coding-adventures/code/packages/go/perceptron => ../../../packages/go/perceptron

replace github.com/adhithyan15/coding-adventures/code/packages/go/neural-network => ../../../packages/go/neural-network

replace github.com/adhithyan15/coding-adventures/code/packages/go/neural-graph-vm => ../../../packages/go/neural-graph-vm
