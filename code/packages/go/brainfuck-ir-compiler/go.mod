module github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler

go 1.23

toolchain go1.24.2

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck => ../brainfuck

replace github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir => ../compiler-ir

replace github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map => ../compiler-source-map

replace github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser

replace github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools

replace github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph

replace github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine

replace github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine => ../virtual-machine
