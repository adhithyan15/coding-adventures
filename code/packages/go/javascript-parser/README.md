# JavaScript Parser (Go)

Parses JavaScript / ECMAScript source code into ASTs using the grammar-driven
parser engine. A thin wrapper that loads the appropriate `.grammar` file and
delegates parsing to the generic `GrammarParser`.

## Usage

```go
import javascriptparser "github.com/adhithyan15/coding-adventures/code/packages/go/javascript-parser"

// Generic grammar — best when you don't know the exact ECMAScript version.
ast, err := javascriptparser.ParseJavascript("let x = 1 + 2;", "")

// Versioned grammar — pin to a specific ECMAScript edition.
ast, err := javascriptparser.ParseJavascript("const x = 1;", "es2022")
```

## Supported versions

| Version  | Standard          | Year |
|----------|-------------------|------|
| `""`     | Generic (default) | —    |
| `"es1"`  | ECMAScript 1      | 1997 |
| `"es3"`  | ECMAScript 3      | 1999 |
| `"es5"`  | ECMAScript 5      | 2009 |
| `"es2015"` | ECMAScript 2015 | 2015 |
| `"es2016"` | ECMAScript 2016 | 2016 |
| `"es2017"` | ECMAScript 2017 | 2017 |
| `"es2018"` | ECMAScript 2018 | 2018 |
| `"es2019"` | ECMAScript 2019 | 2019 |
| `"es2020"` | ECMAScript 2020 | 2020 |
| `"es2021"` | ECMAScript 2021 | 2021 |
| `"es2022"` | ECMAScript 2022 | 2022 |
| `"es2023"` | ECMAScript 2023 | 2023 |
| `"es2024"` | ECMAScript 2024 | 2024 |
| `"es2025"` | ECMAScript 2025 | 2025 |

Passing an unrecognised version string returns a descriptive error immediately,
preventing silent fallback to the wrong grammar.
