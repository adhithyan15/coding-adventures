module github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-encoder

go 1.26

toolchain go1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 => ../wasm-leb128
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser => ../wasm-module-parser
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes => ../wasm-opcodes
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types => ../wasm-types
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator => ../wasm-validator
)
