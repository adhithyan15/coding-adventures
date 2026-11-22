module github.com/adhithyan15/coding-adventures/code/programs/go/kiln

go 1.26

// Kiln depends on in-repo modules only (D24, KILN00 section 17): never on a
// third-party module.  TestNoThirdPartyDependencies enforces it.
require (
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/strace-parser v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../../../packages/go/directed-graph

replace github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../../../packages/go/grammar-tools

replace github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../../../packages/go/lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../../../packages/go/parser

replace github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../../../packages/go/state-machine

replace github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer => ../../../packages/go/strace-lexer

replace github.com/adhithyan15/coding-adventures/code/packages/go/strace-parser => ../../../packages/go/strace-parser
