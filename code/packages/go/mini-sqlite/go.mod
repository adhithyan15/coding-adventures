module github.com/adhithyan15/coding-adventures/code/packages/go/mini-sqlite

go 1.23

require github.com/adhithyan15/coding-adventures/code/packages/go/sql-execution-engine v0.0.0

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/capability-cage v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/sql-lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/sql-parser v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/capability-cage => ../capability-cage
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser
	github.com/adhithyan15/coding-adventures/code/packages/go/sql-execution-engine => ../sql-execution-engine
	github.com/adhithyan15/coding-adventures/code/packages/go/sql-lexer => ../sql-lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/sql-parser => ../sql-parser
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
)
