module github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/clock v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/clock => ../clock
	github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates
)
