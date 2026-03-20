module github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic => ../fp-arithmetic
	github.com/adhithyan15/coding-adventures/code/packages/go/clock => ../clock
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
)
