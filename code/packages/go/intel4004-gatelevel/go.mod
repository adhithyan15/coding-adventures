module github.com/adhithyan15/coding-adventures/code/packages/go/intel4004-gatelevel

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
	github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic => ../arithmetic
)
