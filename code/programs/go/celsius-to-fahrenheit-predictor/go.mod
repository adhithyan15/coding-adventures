module github.com/adhithyan15/coding-adventures/code/programs/go/celsius-to-fahrenheit-predictor

go 1.21

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/gradient-descent v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-graph-vm v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-network v0.0.0
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions => ../../../packages/go/loss-functions

replace github.com/adhithyan15/coding-adventures/code/packages/go/gradient-descent => ../../../packages/go/gradient-descent

replace github.com/adhithyan15/coding-adventures/code/packages/go/neural-network => ../../../packages/go/neural-network

replace github.com/adhithyan15/coding-adventures/code/packages/go/neural-graph-vm => ../../../packages/go/neural-graph-vm
