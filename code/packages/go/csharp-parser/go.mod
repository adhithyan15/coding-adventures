module github.com/adhithyan15/coding-adventures/code/packages/go/csharp-parser

go 1.23

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/csharp-lexer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/csharp-lexer => ../csharp-lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser

replace github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools

replace github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine

replace github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
