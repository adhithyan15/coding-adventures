# VHDL Parser (Go)

Parses VHDL source code into ASTs using the grammar-driven parser engine. A thin wrapper that loads `vhdl.grammar` and delegates parsing to the generic `GrammarParser`.

## Usage

```go
import vhdlparser "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-parser"

ast, err := vhdlparser.ParseVhdl(`
    entity and_gate is
        port (a, b : in std_logic; y : out std_logic);
    end entity and_gate;
    architecture rtl of and_gate is
    begin
        y <= a and b;
    end architecture rtl;
`)
```

## API

- `CreateVhdlParser(source string) (*parser.GrammarParser, error)` -- tokenizes VHDL source and returns a configured `GrammarParser` ready to call `.Parse()`.
- `ParseVhdl(source string) (*parser.ASTNode, error)` -- convenience function that tokenizes, parses, and returns the AST in one step.

## How It Works

1. The source string is tokenized by `vhdl-lexer.TokenizeVhdl()`, which handles case normalization (VHDL is case-insensitive) and produces a token stream.
2. The `vhdl.grammar` file is loaded from `code/grammars/` (resolved relative to this source file using `runtime.Caller`).
3. The grammar is parsed by `grammar-tools.ParseParserGrammar()` into a rule set.
4. A `GrammarParser` is created with the token stream and grammar, then `.Parse()` drives the packrat parser to produce an `ASTNode` tree.

## VHDL vs Verilog

| VHDL | Verilog |
|------|---------|
| entity (interface) | module header |
| architecture (body) | module body |
| signal | wire/reg |
| variable (in process) | (in always block) |
| process | always block |
| port map | instance ports |
| generic | parameter |
| `<=` (signal assign) | `<=` (non-blocking) |
| `:=` (variable assign) | `=` (blocking) |

## Dependencies

- `grammar-tools` -- parses the `.grammar` file format
- `parser` -- the generic grammar-driven parser engine
- `vhdl-lexer` -- tokenizes VHDL source code
- `lexer` (indirect) -- token types and `GrammarLexer`
- `state-machine` (indirect) -- used by `lexer`
- `directed-graph` (indirect) -- used by `grammar-tools`
