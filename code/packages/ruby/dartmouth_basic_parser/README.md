# coding_adventures_dartmouth_basic_parser

A Ruby parser for 1964 Dartmouth BASIC, built on the grammar-driven parser
infrastructure. This gem parses BASIC source code into generic Abstract
Syntax Trees using a plain-text grammar specification.

## What Is 1964 Dartmouth BASIC?

BASIC (Beginner's All-purpose Symbolic Instruction Code) was created by John
Kemeny and Thomas Kurtz at Dartmouth College in 1964. The design goal: make
time-shared computing available to every student, not just computer scientists.
The language had 20 keywords and 11 built-in math functions.

```basic
10 REM CLASSIC EXAMPLE
20 FOR I = 1 TO 10
30   PRINT I * I
40 NEXT I
50 END
```

## Where This Gem Fits

```
dartmouth_basic.grammar  (code/grammars/)
         │
         ▼
coding_adventures_dartmouth_basic_lexer
  (tokenizes BASIC text → token stream)
         │
         ▼
coding_adventures_dartmouth_basic_parser   ← this gem
  (token stream + grammar → AST)
         │
         ▼
  ASTNode tree
  (ready for evaluation, compilation, or analysis)
```

## Installation

```bash
gem install coding_adventures_dartmouth_basic_parser
```

Or in development (from the repo root):

```bash
cd code/packages/ruby/dartmouth_basic_parser
bundle install
```

## Usage

```ruby
require "coding_adventures_dartmouth_basic_parser"

source = <<~BASIC
  10 LET X = 5
  20 FOR I = 1 TO X
  30   PRINT I * I
  40 NEXT I
  50 END
BASIC

ast = CodingAdventures::DartmouthBasicParser.parse(source)
puts ast.rule_name  # "program"
puts ast.children.length  # 5 (one line node per numbered line)
```

## The AST Structure

The root is always a `program` node. Each child is a `line` node containing
the `LINE_NUM` token, an optional `statement` node, and a `NEWLINE` token:

```
ASTNode(rule_name: "program")
  ASTNode(rule_name: "line")
    Token(LINE_NUM, "10")
    ASTNode(rule_name: "statement")
      ASTNode(rule_name: "let_stmt")
        Token(KEYWORD, "LET")
        ASTNode(rule_name: "variable") => Token(NAME, "X")
        Token(EQ, "=")
        ASTNode(rule_name: "expr") => ... => Token(NUMBER, "5")
    Token(NEWLINE, "\n")
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

## Running Tests

```bash
cd code/packages/ruby/dartmouth_basic_parser
bundle exec rake test
```

## License

MIT
