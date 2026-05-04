module github.com/adhithyan15/coding-adventures/code/programs/go/xor-hidden-layer-demo

go 1.21

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-graph-vm v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-network v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/single-layer-network v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/two-layer-network v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-graph-vm => ../../../packages/go/neural-graph-vm
	github.com/adhithyan15/coding-adventures/code/packages/go/neural-network => ../../../packages/go/neural-network
	github.com/adhithyan15/coding-adventures/code/packages/go/single-layer-network => ../../../packages/go/single-layer-network
	github.com/adhithyan15/coding-adventures/code/packages/go/two-layer-network => ../../../packages/go/two-layer-network
)
