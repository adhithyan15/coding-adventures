module github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-intel-4004-compiler

go 1.23

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/intel-4004-ir-validator v0.0.0
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir => ../compiler-ir

replace github.com/adhithyan15/coding-adventures/code/packages/go/intel-4004-ir-validator => ../intel-4004-ir-validator
