# Lisp Lexer (Go)

Grammar-driven Lisp lexer for Go.

This package loads `code/grammars/lisp.tokens` and delegates tokenization to the shared Go `lexer` package.

```go
tokens, err := lisplexer.TokenizeLisp("(define x 42)")
```
