module github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 => ../wasm-leb128
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types => ../wasm-types
)
