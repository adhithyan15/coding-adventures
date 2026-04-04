# Mosaic Lexer (Elixir)

Thin wrapper around the grammar-driven lexer engine for Mosaic tokenization.

Mosaic is a Component Description Language (CDL) for declaring UI component
structure with named typed slots. This package handles the lexical layer —
converting raw `.mosaic` source text into a flat list of typed tokens.

## Usage

```elixir
{:ok, tokens} = CodingAdventures.MosaicLexer.tokenize(~S(component Foo { Box { } }))
# => [%Token{type: "KEYWORD", value: "component"},
#     %Token{type: "NAME",    value: "Foo"},
#     %Token{type: "LBRACE", value: "{"},
#     %Token{type: "NAME",   value: "Box"},
#     %Token{type: "LBRACE", value: "{"},
#     %Token{type: "RBRACE", value: "}"},
#     %Token{type: "RBRACE", value: "}"},
#     %Token{type: "EOF",    value: ""}]
```

## Token Types

| Type        | Examples                                    |
|-------------|---------------------------------------------|
| `KEYWORD`   | `component`, `slot`, `when`, `each`, `text` |
| `NAME`      | `Foo`, `padding-left`, `corner_radius`      |
| `DIMENSION` | `16dp`, `50%`, `1.5rem`                     |
| `NUMBER`    | `42`, `-3.14`, `0.5`                        |
| `COLOR_HEX` | `#fff`, `#2563eb`, `#ff000080`              |
| `STRING`    | `"hello"`, `"path/to/image.png"`            |
| `LBRACE`    | `{`                                         |
| `RBRACE`    | `}`                                         |
| `COLON`     | `:`                                         |
| `SEMICOLON` | `;`                                         |
| `AT`        | `@`                                         |
| `EOF`       | end of input                                |

Whitespace and comments (both `//` line and `/* */` block) are automatically
skipped and never appear in the token list.

## How It Works

Reads `mosaic.tokens` from the shared grammars directory and delegates to
`GrammarLexer.tokenize/2`. The grammar is cached via `:persistent_term` for
fast repeated access across calls.

## Dependencies

- `grammar_tools` — parses `.tokens` files into `TokenGrammar` structs
- `lexer` — grammar-driven tokenization engine (`GrammarLexer`)
- `directed_graph` — used internally by `grammar_tools`
