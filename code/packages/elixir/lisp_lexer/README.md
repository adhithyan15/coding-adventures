# Lisp Lexer (Elixir)

Grammar-driven Lisp lexer for Elixir.

This package loads `code/grammars/lisp.tokens` and delegates tokenization to `CodingAdventures.Lexer.GrammarLexer`.

```elixir
{:ok, tokens} = CodingAdventures.LispLexer.tokenize("(define x 42)")
```
