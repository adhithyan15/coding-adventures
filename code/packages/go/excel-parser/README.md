# JavaScript Parser (Go)

Parses JavaScript source code into ASTs using the grammar-driven parser engine. A thin wrapper that loads `excel.grammar` and delegates parsing to the generic `GrammarParser`.

## Usage

```go
import excelparser "github.com/adhithyan15/coding-adventures/code/packages/go/excel-parser"

ast, err := excelparser.ParseExcel("let x = 1 + 2;")
```
