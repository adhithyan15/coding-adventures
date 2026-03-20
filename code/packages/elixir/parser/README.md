# Parser (Elixir)

Grammar-driven parser engine for the coding-adventures computing stack.

## What It Does

Takes a list of tokens and a `ParserGrammar` (parsed from a `.grammar` file) and produces a generic AST tree of `ASTNode` structs. Uses backtracking with packrat memoization for efficient parsing.

## Usage

```elixir
alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar}
alias CodingAdventures.Lexer.GrammarLexer
alias CodingAdventures.Parser.GrammarParser

{:ok, tg} = TokenGrammar.parse(File.read!("json.tokens"))
{:ok, pg} = ParserGrammar.parse(File.read!("json.grammar"))
{:ok, tokens} = GrammarLexer.tokenize(source, tg)
{:ok, ast} = GrammarParser.parse(tokens, pg)
```

## How It Fits

```
.grammar file → ParserGrammar.parse/1 → ParserGrammar struct
                                              ↓
tokens list  → GrammarParser.parse/2 → ASTNode tree
```

## Dependencies

- `grammar_tools` — provides `ParserGrammar` struct and parser
- `lexer` — provides `Token` struct
