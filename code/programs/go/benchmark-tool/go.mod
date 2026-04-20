module github.com/adhithyan15/coding-adventures/code/programs/go/benchmark-tool

go 1.23

require github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder v0.0.0

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder => ../../../packages/go/cli-builder
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../../../packages/go/directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../../../packages/go/state-machine
)
