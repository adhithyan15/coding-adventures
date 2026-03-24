# Verilog Parser (Go)

Parses Verilog HDL source code into ASTs using the grammar-driven parser engine. A thin wrapper that loads `verilog.grammar` and delegates parsing to the generic `GrammarParser`.

## Usage

```go
import verilogparser "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-parser"

ast, err := verilogparser.ParseVerilog("module m; endmodule")
```

## API

- `CreateVerilogParser(source string) (*parser.GrammarParser, error)` — tokenizes Verilog source and returns a configured `GrammarParser` ready to call `.Parse()`.
- `ParseVerilog(source string) (*parser.ASTNode, error)` — convenience function that tokenizes, parses, and returns the AST in one step.

## How It Works

1. The source string is tokenized by `verilog-lexer.TokenizeVerilog()`, which handles the preprocessor and produces a token stream.
2. The `verilog.grammar` file is loaded from `code/grammars/` (resolved relative to this source file using `runtime.Caller`).
3. The grammar is parsed by `grammar-tools.ParseParserGrammar()` into a rule set.
4. A `GrammarParser` is created with the token stream and grammar, then `.Parse()` drives the packrat parser to produce an `ASTNode` tree.

## Dependencies

- `grammar-tools` — parses the `.grammar` file format
- `parser` — the generic grammar-driven parser engine
- `verilog-lexer` — tokenizes Verilog source code
- `lexer` (indirect) — token types and `GrammarLexer`
- `state-machine` (indirect) — used by `lexer`
- `directed-graph` (indirect) — used by `grammar-tools`
