module github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128 => ../wasm-leb128
)
