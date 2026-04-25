# Lisp Parser (Go)

Grammar-driven Lisp parser for Go.

This package tokenizes source with `go/lisp-lexer`, loads `code/grammars/lisp.grammar`, and delegates parsing to the shared Go `parser` package.

```go
ast, err := lispparser.ParseLisp("(+ 1 2)")
```
