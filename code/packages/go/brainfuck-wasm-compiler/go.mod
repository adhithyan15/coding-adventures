module github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-wasm-compiler

go 1.26

toolchain go1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-compiler v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-validator v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-encoder v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-runtime v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck => ../brainfuck
	github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler => ../brainfuck-ir-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir => ../compiler-ir
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map => ../compiler-source-map
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer => ../ir-optimizer
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-compiler => ../ir-to-wasm-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-validator => ../ir-to-wasm-validator
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine => ../virtual-machine
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution => ../wasm-execution
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 => ../wasm-leb128
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-encoder => ../wasm-module-encoder
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser => ../wasm-module-parser
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes => ../wasm-opcodes
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-runtime => ../wasm-runtime
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types => ../wasm-types
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator => ../wasm-validator
)
