module github.com/adhithyan15/coding-adventures/code/packages/go/micro-qr

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/gf256 v0.0.0
)

require github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions v0.0.0 // indirect

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d => ../barcode-2d
	github.com/adhithyan15/coding-adventures/code/packages/go/gf256 => ../gf256
	github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions => ../paint-instructions
)
