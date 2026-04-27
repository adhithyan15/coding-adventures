module github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-jvm-class-file

go 1.26

toolchain go1.26.1

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file v0.0.0
	golang.org/x/sys v0.43.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir => ../compiler-ir
	github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file => ../jvm-class-file
)
