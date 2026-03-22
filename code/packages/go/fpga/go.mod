module github.com/adhithyan15/coding-adventures/code/packages/go/fpga

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/block-ram v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/block-ram => ../block-ram
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
)
