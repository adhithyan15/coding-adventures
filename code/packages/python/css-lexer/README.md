# CSS Lexer (Python)

Thin wrapper around the grammar-driven lexer engine for CSS tokenization.

## Usage

```python
from css_lexer import tokenize_css

tokens = tokenize_css("h1 { color: red; font-size: 16px; }")
for token in tokens:
    print(f"{token.type}: {token.value}")
# IDENT: h1
# LBRACE: {
# IDENT: color
# COLON: :
# IDENT: red
# SEMICOLON: ;
# IDENT: font-size
# COLON: :
# DIMENSION: 16px
# SEMICOLON: ;
# RBRACE: }
# EOF:
```

## How It Works

Loads `css.tokens` from the shared grammars directory and feeds it to
`GrammarLexer.tokenize()`. The token grammar defines ~39 token types with
careful first-match-wins priority ordering to handle CSS's compound tokens
(DIMENSION before NUMBER, FUNCTION before IDENT, etc.).

## Key Token Types

| Token | Example | Notes |
|-------|---------|-------|
| DIMENSION | `10px`, `2em` | Number + unit (single token) |
| PERCENTAGE | `50%` | Number + percent sign |
| FUNCTION | `rgb(`, `calc(` | Identifier + opening paren |
| HASH | `#fff`, `#header` | Colors and ID selectors |
| AT_KEYWORD | `@media` | At-rule keywords |
| CUSTOM_PROPERTY | `--main-color` | CSS variables |
| URL_TOKEN | `url(path.png)` | Unquoted URL function |
| BAD_STRING | `"unclosed` | Error recovery token |

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine
