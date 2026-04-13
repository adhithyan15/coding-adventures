module github.com/adhithyan15/coding-adventures/code/packages/go/deflate

go 1.23

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/lzss v0.0.0
)

require github.com/adhithyan15/coding-adventures/code/packages/go/heap v0.0.0 // indirect

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/heap => ../heap
	github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree => ../huffman-tree
	github.com/adhithyan15/coding-adventures/code/packages/go/lzss => ../lzss
)
