module github.com/adhithyan15/coding-adventures/code/packages/go/pdf417

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d => ../barcode-2d
	github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions => ../paint-instructions
)
