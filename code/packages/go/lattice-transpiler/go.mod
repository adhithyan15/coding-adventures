module github.com/adhithyan15/coding-adventures/code/packages/go/lattice-transpiler

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/lattice-ast-to-css v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/lattice-parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/lattice-lexer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/lattice-ast-to-css => ../lattice-ast-to-css
	github.com/adhithyan15/coding-adventures/code/packages/go/lattice-lexer => ../lattice-lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/lattice-parser => ../lattice-parser
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
)
