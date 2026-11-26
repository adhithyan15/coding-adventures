module github.com/adhithyan15/coding-adventures/code/programs/go/graph-shortest-path-sat

go 1.26

toolchain go1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/graph v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/graph => ../../../packages/go/graph
	github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver => ../../../packages/go/sat-solver
)
