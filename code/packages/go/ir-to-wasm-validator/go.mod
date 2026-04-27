module github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-validator

go 1.26

toolchain go1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-compiler v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck => ../brainfuck
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler => ../brainfuck-ir-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir => ../compiler-ir
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map => ../compiler-source-map
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-compiler => ../ir-to-wasm-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine => ../virtual-machine
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 => ../wasm-leb128
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes => ../wasm-opcodes
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types => ../wasm-types
)
