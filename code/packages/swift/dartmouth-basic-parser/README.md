# DartmouthBasicParser (Swift)

A Dartmouth BASIC (1964) grammar-driven parser. Takes a stream of `Token` values
from `DartmouthBasicLexer` and produces a generic `ASTNode` tree using the rules
in `dartmouth_basic.grammar` and the `GrammarParser` engine from the `Parser` package.

## What does a parser do?

A lexer converts raw text into a flat list of tokens: `NUMBER("10")`,
`KEYWORD("LET")`, `NAME("x")`, `EQ("=")`, `NUMBER("5")`, `NEWLINE`. A parser
goes one step further — it applies grammar rules to the token stream and produces
a tree (the AST) that reflects the structure of the program.

The grammar for a LET statement is:

```
let_stmt = "LET" variable EQ expr ;
```

The parser matches those tokens in order, nests the sub-rules (`variable`, `expr`)
as child nodes, and returns an `ASTNode("let_stmt")` as a result.

## Usage

```swift
import DartmouthBasicParser

let ast = try DartmouthBasicParser.parse("""
    10 LET X = 5
    20 PRINT X
    30 END
    """)

print(ast.ruleName)  // "program"
```

Or from a pre-lexed token stream:

```swift
import DartmouthBasicLexer
import DartmouthBasicParser

let tokens = try DartmouthBasicLexer.tokenize(source)
let ast = try DartmouthBasicParser.parseTokens(tokens)
```

## AST Structure

The root node always has `ruleName == "program"`. Its children are `line` nodes,
each wrapping a `LINE_NUM` token, an optional `statement` node, and a `NEWLINE`.

```
program
├── line
│   ├── Token(LINE_NUM, "10")
│   ├── statement
│   │   └── let_stmt
│   │       ├── Token(KEYWORD, "LET")
│   │       ├── variable → Token(NAME, "x")
│   │       ├── Token(EQ, "=")
│   │       └── expr → term → power → unary → primary → Token(NUMBER, "5")
│   └── Token(NEWLINE, "\n")
└── line
    ├── Token(LINE_NUM, "20")
    ├── statement → end_stmt → Token(KEYWORD, "END")
    └── Token(NEWLINE, "\n")
```

## Statement Types

All 17 Dartmouth BASIC 1964 statements are supported:

| Rule          | Grammar                                              |
|---------------|------------------------------------------------------|
| `let_stmt`    | `"LET" variable EQ expr`                            |
| `print_stmt`  | `"PRINT" [ print_list ]`                            |
| `input_stmt`  | `"INPUT" variable { COMMA variable }`               |
| `if_stmt`     | `"IF" expr relop expr "THEN" LINE_NUM`              |
| `goto_stmt`   | `"GOTO" LINE_NUM`                                   |
| `gosub_stmt`  | `"GOSUB" LINE_NUM`                                  |
| `return_stmt` | `"RETURN"`                                          |
| `for_stmt`    | `"FOR" NAME EQ expr "TO" expr [ "STEP" expr ]`      |
| `next_stmt`   | `"NEXT" NAME`                                       |
| `end_stmt`    | `"END"`                                             |
| `stop_stmt`   | `"STOP"`                                            |
| `rem_stmt`    | `"REM"` (content already stripped by lexer)         |
| `read_stmt`   | `"READ" variable { COMMA variable }`                |
| `data_stmt`   | `"DATA" NUMBER { COMMA NUMBER }`                    |
| `restore_stmt`| `"RESTORE"`                                         |
| `dim_stmt`    | `"DIM" dim_decl { COMMA dim_decl }`                 |
| `def_stmt`    | `"DEF" USER_FN LPAREN NAME RPAREN EQ expr`          |

## LINE_NUM vs NUMBER for Jump Targets

The grammar uses `LINE_NUM` for GOTO/GOSUB/IF-THEN jump targets, but the lexer
only promotes line-start integers to `LINE_NUM`. Jump targets like `50` in
`GOTO 50` come out as `NUMBER` from the lexer.

A pre-parse hook `relabelJumpTargets` bridges this gap: it scans the token list
and promotes any `NUMBER` that immediately follows `GOTO`, `GOSUB`, or `THEN` to
`LINE_NUM`, so the grammar rules match correctly.

## Dependencies

- `GrammarTools` — parses `dartmouth_basic.grammar`
- `Lexer` — provides `Token` type
- `Parser` — provides `GrammarParser`, `ASTNode`, `ASTChild`
- `DartmouthBasicLexer` — tokenizes BASIC source

## Running tests

```bash
swift test --verbose
```

## Position in the stack

```
dartmouth_basic.tokens
        ↓
DartmouthBasicLexer
        ↓
dartmouth_basic.grammar
        ↓
DartmouthBasicParser  ← this package
        ↓
  compiler / VM
```
