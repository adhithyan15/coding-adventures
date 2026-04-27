# Go CSS lexer

A CSS lexer that follows the shared `css.tokens` priority order.

```go
tokens, err := csslexer.Tokenize("h1 { color: #333; }")
```
