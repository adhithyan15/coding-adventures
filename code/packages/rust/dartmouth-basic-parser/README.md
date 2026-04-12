# coding-adventures-dartmouth-basic-parser

Parser for the 1964 Dartmouth BASIC language. Takes a stream of BASIC tokens
and produces an Abstract Syntax Tree (AST) by applying the `dartmouth_basic.grammar`
rules through the grammar-driven `GrammarParser` engine.

## What This Is

This crate is the second stage of the Dartmouth BASIC front-end pipeline:

```
Source text
    │
    ▼
dartmouth-basic-lexer    → Vec<Token>
    │
    ▼
dartmouth_basic.grammar  → ParserGrammar (rules)
    │
    ▼
GrammarParser            → GrammarASTNode (AST)
    │
    ▼
compiler or interpreter
```

The parser is **grammar-driven**: no hand-written recursive descent code.
The `dartmouth_basic.grammar` file defines all 17 statement types and the
expression precedence hierarchy. The `GrammarParser` engine interprets these
rules at runtime using recursive descent with packrat memoization.

## Historical Context

Dartmouth BASIC was created by John G. Kemeny and Thomas E. Kurtz at Dartmouth
College in 1964. Running on a GE-225 mainframe accessed via uppercase-only
teletypes, it was the first programming language designed for non-science
students. Its numbered-line format and simple syntax made programming accessible
to an entirely new audience.

The 17 statement types in the 1964 specification:

| Statement | Purpose |
|-----------|---------|
| LET       | Variable assignment: `10 LET X = 5` |
| PRINT     | Output to terminal: `20 PRINT X, Y` |
| INPUT     | Read from user: `30 INPUT A, B` |
| IF-THEN   | Conditional branch: `40 IF X > 0 THEN 100` |
| GOTO      | Unconditional jump: `50 GOTO 200` |
| GOSUB     | Subroutine call: `60 GOSUB 300` |
| RETURN    | Return from subroutine: `300 RETURN` |
| FOR       | Start counted loop: `70 FOR I = 1 TO 10` |
| NEXT      | End counted loop: `80 NEXT I` |
| END       | Normal program termination |
| STOP      | Halt with message (resumable in DTSS) |
| REM       | Comment / remark |
| READ      | Read from DATA pool: `90 READ X, Y` |
| DATA      | Define data pool: `100 DATA 1, 2, 3` |
| RESTORE   | Reset DATA pool pointer |
| DIM       | Declare array size: `110 DIM A(100)` |
| DEF       | Define user function: `120 DEF FNA(X) = X*X` |

## Usage

```rust
use coding_adventures_dartmouth_basic_parser::parse_dartmouth_basic;

// Parse a complete BASIC program
let ast = parse_dartmouth_basic("10 LET X = 5\n20 PRINT X\n30 END\n");
assert_eq!(ast.rule_name, "program");
```

```rust
use coding_adventures_dartmouth_basic_parser::create_dartmouth_basic_parser;

// Get a parser object for step-by-step control
let mut parser = create_dartmouth_basic_parser("10 LET X = 5\n");
let result = parser.parse();
match result {
    Ok(ast) => println!("Parsed: {:?}", ast.rule_name),
    Err(e)  => eprintln!("Parse error: {}", e),
}
```

## AST Structure

The root node has `rule_name = "program"`. Its children are `line` nodes,
each representing one numbered BASIC line:

```
program
  └── line
        ├── LINE_NUM("10")
        ├── statement
        │     └── let_stmt
        │           ├── KEYWORD("LET")
        │           ├── variable → NAME("X")
        │           ├── EQ("=")
        │           └── expr → term → power → unary → primary → NUMBER("5")
        └── NEWLINE
```

## How It Fits in the Stack

- **Depends on**: `dartmouth-basic-lexer`, `grammar-tools`, `parser`, `lexer`
- **Used by**: `dartmouth-basic-compiler`, `dartmouth-basic-vm` (future crates)
- **Grammar file**: `code/grammars/dartmouth_basic.grammar` (shared across all
  language implementations)

## Running Tests

```
cargo test -p coding-adventures-dartmouth-basic-parser -- --nocapture
```
