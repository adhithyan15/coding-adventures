# algol-lexer

A grammar-driven lexer for **ALGOL 60** (ALGOrithmic Language, 1960), implemented in Go.

## What Is ALGOL 60?

ALGOL 60 was designed by an international committee including John Backus, Peter Naur, John McCarthy, and Edsger Dijkstra. It was the first programming language formally specified using **BNF (Backus-Naur Form)** — the notation used to describe virtually every programming language since.

ALGOL 60 introduced concepts now considered fundamental to programming:

- **Block structure**: `begin...end` blocks with lexical scoping
- **Recursive procedures**: functions that can call themselves
- **The call stack**: runtime activation records for nested calls
- **Free-format source**: whitespace is for humans, not the compiler
- **Formal grammar**: a language described by its own syntax rules

Every mainstream language today — C, Java, Python, Go, Rust — is an ALGOL descendant. C derives from BCPL via CPL via ALGOL. Pascal was a direct descendant. Simula (the first OOP language) extended ALGOL. Java copied Pascal.

## How This Package Works

This package is a thin wrapper around the generic grammar-driven lexer. It:

1. Reads `code/grammars/algol.tokens` (the token grammar)
2. Passes it to `GrammarLexer`, which compiles the regex patterns into a DFA
3. The `GrammarLexer` handles skip patterns (whitespace, comments) automatically

The grammar path is resolved at runtime using `runtime.Caller(0)` so the package works from any working directory.

## Usage

```go
import algollexer "github.com/adhithyan15/coding-adventures/code/packages/go/algol-lexer"

// One-shot tokenization
tokens, err := algollexer.TokenizeAlgol("begin integer x; x := 42 end")
if err != nil {
    log.Fatal(err)
}
for _, tok := range tokens {
    fmt.Printf("%s(%q) at %d:%d\n", tok.TypeName, tok.Value, tok.Line, tok.Column)
}

// Or create a reusable lexer
lex, err := algollexer.CreateAlgolLexer("begin real pi; pi := 3.14159 end")
if err != nil {
    log.Fatal(err)
}
tokens = lex.Tokenize()
```

## Token Types

| Token | Example | Notes |
|-------|---------|-------|
| `BEGIN` / `END` | `begin` / `end` | Block delimiters |
| `IF` `THEN` `ELSE` | `if x > 0 then` | Conditional |
| `FOR` `DO` `STEP` `UNTIL` `WHILE` | `for i := 1 step 1 until 10 do` | Loop |
| `INTEGER` `REAL` `BOOLEAN` `STRING` | `integer x` | Type names |
| `TRUE` `FALSE` | `true` | Boolean literals |
| `AND` `OR` `NOT` `IMPL` `EQV` | `x and y` | Boolean operators (keywords!) |
| `DIV` `MOD` | `n div 2` | Integer arithmetic keywords |
| `ASSIGN` | `:=` | Assignment (not `=`) |
| `EQ` | `=` | Equality comparison |
| `POWER` / `CARET` | `**` / `^` | Exponentiation |
| `LEQ` `GEQ` `NEQ` | `<=` `>=` `!=` | Relational operators |
| `INTEGER_LIT` | `42` | Integer literals |
| `REAL_LIT` | `3.14`, `1.5E3` | Floating-point literals |
| `STRING_LIT` | `'hello'` | Single-quoted string literals |
| `IDENT` | `x`, `result` | Identifiers |

## Comment Syntax

ALGOL 60 uses a unique comment form: the keyword `comment` followed by text up to the next `;`. Comments are silently consumed by the lexer.

```
comment this is ignored; x := 1
```

## Design Notes

**`:=` vs `=`**: ALGOL requires `:=` for assignment and `=` for equality. This prevents the classic C bug of writing `=` when you mean `==`.

**Keywords are case-insensitive**: `BEGIN`, `Begin`, and `begin` all produce the same token.

**No underscores in identifiers**: Original ALGOL 60 did not allow `_` in names. Identifiers are letters followed by letters and digits only.

**Boolean operators are words**: `and`, `or`, `not`, `impl`, `eqv` are keywords, not symbols. This follows mathematical notation more closely than C's `&&`, `||`, `!`.

## Stack

This package depends on:
- `go/lexer` — generic GrammarLexer engine
- `go/grammar-tools` — token grammar parser

The algol-parser package depends on this package.
