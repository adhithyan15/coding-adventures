module github.com/adhithyan15/coding-adventures/code/programs/go/build-tool

go 1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/progress-bar v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-interpreter v0.0.0-00010101000000-000000000000
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-ast-to-bytecode-compiler v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-vm v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../../../packages/go/directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../../../packages/go/grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../../../packages/go/lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../../../packages/go/parser
	github.com/adhithyan15/coding-adventures/code/packages/go/progress-bar => ../../../packages/go/progress-bar
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-ast-to-bytecode-compiler => ../../../packages/go/starlark-ast-to-bytecode-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-interpreter => ../../../packages/go/starlark-interpreter
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-lexer => ../../../packages/go/starlark-lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-parser => ../../../packages/go/starlark-parser
	github.com/adhithyan15/coding-adventures/code/packages/go/starlark-vm => ../../../packages/go/starlark-vm
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../../../packages/go/state-machine
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine => ../../../packages/go/virtual-machine
)
