module github.com/adhithyan15/coding-adventures/code/packages/go/paint-codec-png

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-png v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/lzss v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/zip v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-png => ../image-codec-png

replace github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container => ../pixel-container

replace github.com/adhithyan15/coding-adventures/code/packages/go/zip => ../zip

replace github.com/adhithyan15/coding-adventures/code/packages/go/lzss => ../lzss
