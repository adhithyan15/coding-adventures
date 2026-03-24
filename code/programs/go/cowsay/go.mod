module github.com/adhithyan15/coding-adventures/code/programs/go/cowsay

go 1.23

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder => ../../../packages/go/cli-builder
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../../../packages/go/directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../../../packages/go/state-machine
)
