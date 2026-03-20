module github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/cache v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/clock v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic => ../arithmetic
	github.com/adhithyan15/coding-adventures/code/packages/go/cache => ../cache
	github.com/adhithyan15/coding-adventures/code/packages/go/clock => ../clock
	github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit => ../compute-unit
	github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic => ../fp-arithmetic
	github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core => ../gpu-core
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
	github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine => ../parallel-execution-engine
)
