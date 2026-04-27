module github.com/adhithyan15/coding-adventures/code/packages/go/aes-modes

go 1.26

require github.com/adhithyan15/coding-adventures/code/packages/go/aes v0.0.0

require github.com/adhithyan15/coding-adventures/code/packages/go/gf256 v0.0.0 // indirect

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/aes => ../aes
	github.com/adhithyan15/coding-adventures/code/packages/go/gf256 => ../gf256
)
