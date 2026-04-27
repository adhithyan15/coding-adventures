# css-parser

Grammar-driven CSS parser for Go.

This package tokenizes CSS with `css-lexer`, loads the shared
`code/grammars/css.grammar` parser grammar, and returns the generic AST
produced by the Go `parser` package.

```go
ast, err := cssparser.ParseCSS("h1 { color: red; }")
if err != nil {
    panic(err)
}
fmt.Println(ast.RuleName) // "stylesheet"
```
