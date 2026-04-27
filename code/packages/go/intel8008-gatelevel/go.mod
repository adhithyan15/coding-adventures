module github.com/adhithyan15/coding-adventures/code/packages/go/intel8008-gatelevel

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0
)

require github.com/adhithyan15/coding-adventures/code/packages/go/transistors v0.0.0 // indirect

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic => ../arithmetic
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
	github.com/adhithyan15/coding-adventures/code/packages/go/transistors => ../transistors
)
