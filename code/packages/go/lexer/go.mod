module github.com/adhithyan15/coding-adventures/code/packages/go/lexer

go 1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0
)

require github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
