module github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core

go 1.26

require github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic v0.0.0

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/clock v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/clock => ../clock
	github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic => ../fp-arithmetic
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
)
