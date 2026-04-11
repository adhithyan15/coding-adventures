# coding-adventures-dartmouth-basic-parser

A Python parser for 1964 Dartmouth BASIC, built on the grammar-driven parser
infrastructure. This package parses BASIC source code into generic Abstract
Syntax Trees using a plain-text grammar specification.

## What Is 1964 Dartmouth BASIC?

BASIC (Beginner's All-purpose Symbolic Instruction Code) was created by John
Kemeny and Thomas Kurtz at Dartmouth College in 1964. Their mission: make
time-shared computing accessible to every student, not just computer science
majors. The result was a language with 20 keywords, 11 built-in math functions,
and an elegantly simple line-numbered structure.

```basic
10 REM CLASSIC EXAMPLE
20 FOR I = 1 TO 10
30   PRINT I * I
40 NEXT I
50 END
```

## Where This Package Fits

```
dartmouth_basic.grammar  (code/grammars/)
         │
         ▼
coding-adventures-dartmouth-basic-lexer
  (tokenizes BASIC text → token stream)
         │
         ▼
coding-adventures-dartmouth-basic-parser   ← this package
  (token stream + grammar → AST)
         │
         ▼
  ASTNode tree
  (ready for evaluation, compilation, or analysis)
```

## Installation

```bash
pip install coding-adventures-dartmouth-basic-parser
```

Or in development (from the repo root):

```bash
cd code/packages/python/dartmouth-basic-parser
uv pip install -e ../grammar-tools -e ../graph -e ../directed-graph \
               -e ../state-machine -e ../lexer -e ../parser \
               -e ../dartmouth-basic-lexer -e ".[dev]"
```

## Usage

```python
from dartmouth_basic_parser import parse_dartmouth_basic

source = """\
10 LET X = 5
20 FOR I = 1 TO X
30   PRINT I * I
40 NEXT I
50 END
"""

ast = parse_dartmouth_basic(source)
print(ast.rule_name)  # "program"
print(len(ast.children))  # 5 (one line node per numbered line)
```

For lower-level access:

```python
from dartmouth_basic_parser import create_dartmouth_basic_parser

parser = create_dartmouth_basic_parser(source)
ast = parser.parse()
```

## The AST Structure

The root is always a `program` node. Each child is a `line` node containing
the `LINE_NUM` token, an optional `statement` node, and a `NEWLINE` token:

```
ASTNode(rule_name="program")
  └── ASTNode(rule_name="line")
        ├── Token(LINE_NUM, "10")
        ├── ASTNode(rule_name="statement")
        │     └── ASTNode(rule_name="let_stmt")
        │           ├── Token(KEYWORD, "LET")
        │           ├── ASTNode(rule_name="variable") → Token(NAME, "X")
        │           ├── Token(EQ, "=")
        │           └── ASTNode(rule_name="expr") → ... → Token(NUMBER, "5")
        └── Token(NEWLINE, "\n")
```

## Supported Statements (all 17 from the 1964 spec)

| Keyword   | Syntax                             | Purpose                    |
|-----------|------------------------------------|----------------------------|
| LET       | `LET var = expr`                   | Assignment                 |
| PRINT     | `PRINT [expr/str {,/; expr/str}]`  | Output                     |
| INPUT     | `INPUT var {, var}`                | Read from user             |
| IF...THEN | `IF expr relop expr THEN line`     | Conditional branch         |
| GOTO      | `GOTO line`                        | Unconditional jump         |
| GOSUB     | `GOSUB line`                       | Call subroutine            |
| RETURN    | `RETURN`                           | Return from subroutine     |
| FOR       | `FOR var = expr TO expr [STEP e]`  | Counted loop start         |
| NEXT      | `NEXT var`                         | Counted loop end           |
| END       | `END`                              | Normal program termination |
| STOP      | `STOP`                             | Halt (resumable)           |
| REM       | `REM ...`                          | Comment                    |
| READ      | `READ var {, var}`                 | Read from DATA pool        |
| DATA      | `DATA num {, num}`                 | Define data pool           |
| RESTORE   | `RESTORE`                          | Reset DATA read pointer    |
| DIM       | `DIM name(n) {, name(n)}`          | Declare array size         |
| DEF       | `DEF FNx(var) = expr`              | Define function            |

## Built-in Functions

SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN

## How the Grammar-Driven Approach Works

Instead of handwriting a BASIC-specific parser, this package:

1. Loads `dartmouth_basic.grammar` — a plain EBNF grammar file
2. Passes it to the generic `GrammarParser` engine
3. The engine recursively matches the token stream against the grammar rules
4. The result is a generic `ASTNode` tree

The same grammar file drives parsers in Python, Ruby, Go, Rust, TypeScript,
and every other language in this codebase. One grammar file; many implementations.

## Running Tests

```bash
cd code/packages/python/dartmouth-basic-parser
./BUILD_windows   # on Windows
# or
./BUILD           # on Unix
```

## License

MIT
