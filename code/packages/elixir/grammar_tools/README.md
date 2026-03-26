# Grammar Tools (Elixir)

Parser and validator for `.tokens` and `.grammar` files — the declarative
grammar definitions that drive the grammar-driven lexer and parser.

This is a port of the Python `grammar-tools` package to Elixir.

## Usage

```elixir
alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar}

# Parse a .tokens file
{:ok, token_grammar} = TokenGrammar.parse(File.read!("json.tokens"))

# Parse a .grammar file
{:ok, parser_grammar} = ParserGrammar.parse(File.read!("json.grammar"))
```
