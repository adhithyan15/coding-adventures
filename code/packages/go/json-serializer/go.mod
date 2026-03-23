module github.com/coding-adventures/json-serializer

go 1.23

require github.com/coding-adventures/json-value v0.0.0

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/json-lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/json-parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
)

replace github.com/coding-adventures/json-value => ../json-value

replace github.com/adhithyan15/coding-adventures/code/packages/go/json-parser => ../json-parser

replace github.com/adhithyan15/coding-adventures/code/packages/go/json-lexer => ../json-lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser

replace github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools

replace github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine

replace github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
