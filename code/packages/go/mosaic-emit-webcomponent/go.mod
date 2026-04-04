module github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-emit-webcomponent

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-vm v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-lexer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer => ../mosaic-analyzer
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-lexer => ../mosaic-lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-parser => ../mosaic-parser
	github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-vm => ../mosaic-vm
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
)
