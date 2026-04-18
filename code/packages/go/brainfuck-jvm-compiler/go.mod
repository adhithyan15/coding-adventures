module github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-jvm-compiler

go 1.26

toolchain go1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-jvm-class-file v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine v0.0.0 // indirect
	golang.org/x/sys v0.43.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck => ../brainfuck
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler => ../brainfuck-ir-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir => ../compiler-ir
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map => ../compiler-source-map
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer => ../ir-optimizer
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-jvm-class-file => ../ir-to-jvm-class-file
	github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file => ../jvm-class-file
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine => ../virtual-machine
)
