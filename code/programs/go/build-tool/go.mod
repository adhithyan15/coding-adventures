module github.com/adhithyan15/coding-adventures/code/programs/go/build-tool

go 1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/progress-bar v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../../../packages/go/directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/progress-bar => ../../../packages/go/progress-bar
)
