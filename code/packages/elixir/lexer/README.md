# Lexer (Elixir)

Grammar-driven lexer engine for the coding-adventures computing stack.

## What It Does

Takes source code and a `TokenGrammar` (parsed from a `.tokens` file) and produces a list of `Token` structs. This is the Elixir port of the Python `GrammarLexer` — it reads token definitions at runtime instead of hardcoding character-matching logic.

## Usage

```elixir
alias CodingAdventures.GrammarTools.TokenGrammar
alias CodingAdventures.Lexer.GrammarLexer

{:ok, grammar} = TokenGrammar.parse(File.read!("json.tokens"))
{:ok, tokens} = GrammarLexer.tokenize(~s({"key": 42}), grammar)
# => [%Token{type: "LBRACE", ...}, %Token{type: "STRING", value: "key", ...}, ...]
```

## How It Fits

```
.tokens file → TokenGrammar.parse/1 → TokenGrammar struct
                                            ↓
source code  → GrammarLexer.tokenize/2 → [Token, Token, ..., EOF]
                                            ↓
                              GrammarParser.parse/2 → AST
```

## Dependencies

- `grammar_tools` — provides `TokenGrammar` struct and parser
