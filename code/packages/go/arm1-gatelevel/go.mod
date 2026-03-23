module github.com/adhithyan15/coding-adventures/code/packages/go/arm1-gatelevel

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/arm1-simulator v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic => ../arithmetic
	github.com/adhithyan15/coding-adventures/code/packages/go/arm1-simulator => ../arm1-simulator
	github.com/adhithyan15/coding-adventures/code/packages/go/clock => ../clock
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
)
