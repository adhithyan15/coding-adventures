module github.com/adhithyan15/coding-adventures/code/packages/go/bytecode-compiler

go 1.22

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine v0.0.0
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser

replace github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine => ../virtual-machine
