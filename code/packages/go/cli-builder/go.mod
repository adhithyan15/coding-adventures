module github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder

go 1.23

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
)
