module github.com/adhithyan15/coding-adventures/code/packages/go/python-parser

go 1.22

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/python-lexer v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/python-lexer => ../python-lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser

replace github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools

replace github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
