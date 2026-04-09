# Dartmouth BASIC Parser

## Purpose

This spec defines `dartmouth_basic_parser` — the parser for the 1964 Dartmouth
BASIC language. It wraps the generic `parser` package, providing a
`basic.grammar` grammar file and a module that loads, caches, and delegates to
it.

The parser sits at Layer 2 of the Dartmouth BASIC pipeline:

```
[Token stream from dartmouth_basic_lexer]
      │
      ▼
┌─────────────────────────────────┐
│   dartmouth_basic_parser        │  ← this package
│   basic.grammar file            │
└─────────────────────────────────┘
      │
      ▼  ASTNode tree
┌─────────────────────────────────┐
│   dartmouth_basic_compiler      │
└─────────────────────────────────┘
```

The parser knows about BASIC's statement structure, expression precedence, and
syntactic rules. It knows nothing about runtime semantics (what GOTO does,
whether a line number exists, what variables are in scope). Those are the
compiler's and VM's concerns.

---

## Depends On

| Package | Role |
|---------|------|
| `grammar_tools` | Parses `basic.grammar` into a `ParserGrammar` struct |
| `parser` | Runs the grammar against a token list, produces an `ASTNode` tree |
| `dartmouth_basic_lexer` | Provides `tokenize/1` used in the combined `parse/1` entry point |

---

## Grammar File: `code/grammars/basic.grammar`

The grammar file drives the entire parser. The generic `parser` package
implements a recursive-descent parser with packrat memoisation — no ambiguity,
no backtracking cost, error messages pointing at the furthest-reached token.

### Newline Significance

BASIC is line-oriented. Each physical line is one statement. Newlines are
**significant** — they terminate statements. The `parser` package detects this
automatically: if the grammar references the `NEWLINE` token type anywhere, the
parser enters "newlines significant" mode and does not skip NEWLINE tokens
between elements.

```
# @version 1

# =======================================================================
# program — the top-level rule
#
# A BASIC program is a sequence of one or more numbered lines.
# The program ends at EOF.
# =======================================================================

program = { line } ;

# =======================================================================
# line — one numbered statement
#
# Every line begins with a LINE_NUM token, contains exactly one
# statement, and ends with a NEWLINE. This structure is what makes line
# numbers the program's addressing scheme: LINE_NUM is both label and
# sort key.
#
# A bare NEWLINE line (empty line) is valid and produces nothing.
# We handle this by making statement optional when preceded by LINE_NUM:
#   "10\n" is a line with no statement — legal in BASIC (it deletes line 10
#   if the program is already stored, but in source form it's a no-op).
# =======================================================================

line = LINE_NUM [ statement ] NEWLINE ;

# =======================================================================
# statement — one of the 17 statement types
#
# Alternation: the parser tries each alternative in order and takes the
# first that matches. Because every statement begins with a distinct
# keyword (except for the bare line case above), there is no ambiguity.
# =======================================================================

statement = let_stmt
          | print_stmt
          | input_stmt
          | if_stmt
          | goto_stmt
          | gosub_stmt
          | return_stmt
          | for_stmt
          | next_stmt
          | end_stmt
          | stop_stmt
          | rem_stmt
          | read_stmt
          | data_stmt
          | restore_stmt
          | dim_stmt
          | def_stmt
          ;

# =======================================================================
# LET — variable assignment
#
#   10 LET X = 5
#   20 LET A(3) = X + 1
#
# The `=` in LET is always assignment. It is never a comparison.
# =======================================================================

let_stmt = "LET" variable EQ expr ;

# =======================================================================
# PRINT — output to terminal
#
#   10 PRINT                   — blank line
#   20 PRINT X                 — print X, then newline
#   30 PRINT X, Y              — print X, space to next zone, print Y
#   40 PRINT X; Y              — print X immediately followed by Y
#   50 PRINT "HELLO", X        — mix strings and expressions
#   60 PRINT X,                — trailing comma: no newline after
#
# print_list is a sequence of items separated by , or ;.
# A trailing separator suppresses the final newline.
# =======================================================================

print_stmt = "PRINT" [ print_list ] ;

print_list = print_item { print_sep print_item } [ print_sep ] ;

print_item = STRING | expr ;

print_sep = COMMA | SEMICOLON ;

# =======================================================================
# INPUT — read values from user at runtime
#
#   10 INPUT X
#   20 INPUT A, B, C
#
# INPUT with multiple variables reads one value per variable in order.
# The VM reads from :input_queue (for testing) or from stdin.
# =======================================================================

input_stmt = "INPUT" variable { COMMA variable } ;

# =======================================================================
# IF ... THEN — conditional branch
#
#   10 IF X > 0 THEN 100
#   20 IF A = B THEN 50
#
# The 1964 spec supports ONLY `IF expr relop expr THEN LINE_NUM`.
# There is no ELSE. There is no `IF ... THEN statement` (that came later).
# The target must be a literal line number, not an expression.
#
# Note: `relop` is a separate rule because the grammar needs to match
# multi-character operators (<=, >=, <>) as well as single-char ones.
# =======================================================================

if_stmt = "IF" expr relop expr "THEN" LINE_NUM ;

relop = EQ | LT | GT | LE | GE | NE ;

# =======================================================================
# GOTO — unconditional jump
#
#   10 GOTO 50
#
# The target is a LINE_NUM token (an integer literal that names a line).
# The compiler emits a JUMP instruction; the VM resolves it via the
# line table built from LINE_MARKER opcodes.
# =======================================================================

goto_stmt = "GOTO" LINE_NUM ;

# =======================================================================
# GOSUB / RETURN — subroutine call and return
#
#   10 GOSUB 200
#   ...
#   200 PRINT "IN SUBROUTINE"
#   210 RETURN
#
# GOSUB pushes the return address (PC after the GOSUB instruction) onto
# the call stack and jumps to the target line. RETURN pops and jumps back.
# The 1964 spec does not define a maximum nesting depth (unlike the 4004).
# =======================================================================

gosub_stmt = "GOSUB" LINE_NUM ;

return_stmt = "RETURN" ;

# =======================================================================
# FOR / NEXT — counted loop
#
#   10 FOR I = 1 TO 10
#   20   PRINT I
#   30 NEXT I
#
#   10 FOR I = 10 TO 1 STEP -1
#   20   PRINT I
#   30 NEXT I
#
# STEP is optional; it defaults to 1 if absent.
# The loop variable must be a plain NAME (not an array element).
# NEXT must name the same variable as the matching FOR.
#
# Edge cases:
#   FOR I = 5 TO 1         → zero iterations (step defaults to +1, 5 > 1)
#   FOR I = 5 TO 1 STEP -1 → five iterations (5, 4, 3, 2, 1)
#   FOR I = 1 TO 1         → exactly one iteration
# =======================================================================

for_stmt = "FOR" NAME EQ expr "TO" expr [ "STEP" expr ] ;

next_stmt = "NEXT" NAME ;

# =======================================================================
# END / STOP — program termination
#
# END is the normal termination. STOP terminates with a message
# ("STOP IN LINE n") and can be resumed with CONT in the original
# DTSS system. For our purposes both terminate execution.
# =======================================================================

end_stmt  = "END" ;
stop_stmt = "STOP" ;

# =======================================================================
# REM — remark / comment
#
#   10 REM THIS IS A COMMENT
#
# The lexer suppresses all tokens after REM until NEWLINE (see lexer spec).
# The parser sees only: LINE_NUM KEYWORD("REM") NEWLINE.
# The rem_stmt rule matches an empty body because the comment content
# has already been removed from the token stream.
# =======================================================================

rem_stmt = "REM" ;

# =======================================================================
# READ / DATA / RESTORE — sequential data pool
#
#   10 READ X
#   20 READ A, B, C
#   30 DATA 1, 2, 3, 4, 5
#   40 RESTORE
#
# DATA lines define a pool of values in line-number order.
# READ pops values from the pool in sequence.
# RESTORE resets the pool pointer to the beginning.
#
# DATA values are literals only (not expressions). The 1964 spec allows
# numeric literals and quoted strings. We support numeric literals only
# (strings in DATA require string variables, which 1964 BASIC lacks).
# =======================================================================

read_stmt    = "READ" variable { COMMA variable } ;

data_stmt    = "DATA" NUMBER { COMMA NUMBER } ;

restore_stmt = "RESTORE" ;

# =======================================================================
# DIM — array dimensioning
#
#   10 DIM A(10)
#   20 DIM A(10), B(20)
#
# DIM declares an array with a given size. Without DIM, arrays default
# to size 10 (indices 0 through 10). DIM allows larger arrays.
# The size must be a literal integer (not an expression).
#
# 1964 BASIC supports only 1-dimensional arrays. Indices are 0-based
# (though the original Dartmouth spec used 1-based — we use 0-based
# for consistency with our VM; see compiler spec for adjustment).
# =======================================================================

dim_stmt = "DIM" dim_decl { COMMA dim_decl } ;

dim_decl = NAME LPAREN NUMBER RPAREN ;

# =======================================================================
# DEF — user-defined function
#
#   10 DEF FNA(X) = X * X
#   20 DEF FNB(T) = SIN(T) / COS(T)
#
# DEF defines a single-argument mathematical function named FNA through
# FNZ. The function body is an expression that may reference the formal
# parameter (X, T, etc.) and any globally defined variables.
#
# USER_FN is the token type for FNA..FNZ (emitted by the lexer).
# The formal parameter NAME is local to the function body.
# =======================================================================

def_stmt = "DEF" USER_FN LPAREN NAME RPAREN EQ expr ;

# =======================================================================
# variable — an lvalue or rvalue location
#
#   X         → scalar variable (NAME token)
#   A(I)      → array element (NAME + index expression)
#
# Note: in the grammar, `variable` is used both for assignment targets
# (LET, INPUT, READ) and for reading values in expressions. The same
# syntax covers both uses.
# =======================================================================

variable = NAME LPAREN expr RPAREN
         | NAME
         ;

# =======================================================================
# Expressions — arithmetic with standard precedence
#
# Precedence (lowest to highest):
#   1. Addition and subtraction         +  -
#   2. Multiplication and division      *  /
#   3. Exponentiation                   ^  (right-associative)
#   4. Unary negation                   -
#   5. Primary: literal, variable, call, parenthesised expression
#
# This is encoded as a grammar cascade: each level's rule references
# the next-higher-precedence level as its operand. This automatically
# enforces precedence without any explicit priority annotations.
#
# Example: `2 + 3 * 4`
#   expr → term(2) + term(3 * 4)
#        → term(2) + power(3) * power(4)
#   Result: Add(2, Mul(3, 4))   ← correct: multiplication binds tighter
#
# Example: `2 ^ 3 ^ 2`
#   power → unary(2) ^ power(3 ^ 2)
#          → unary(2) ^ (unary(3) ^ power(2))
#   Result: Pow(2, Pow(3, 2))   ← correct: right-associative
# =======================================================================

expr  = term { ( PLUS | MINUS ) term } ;

term  = power { ( STAR | SLASH ) power } ;

# Exponentiation is right-associative: `power` recurses on itself on the
# right side. This means `2^3^2` parses as `2^(3^2) = 512`, not `(2^3)^2 = 64`.
# The original Dartmouth BASIC spec specifies right-associativity for `^`.

power = unary [ CARET power ] ;

# Unary minus: `-X`, `-3.14`, `-(X + 1)`. Only minus is unary in BASIC.
# Unary plus is not supported (not in the 1964 spec).

unary = MINUS primary
      | primary
      ;

# Primary: the atomic units of expressions.
#   NUMBER       — numeric literal
#   variable     — scalar or array access (variable rule above)
#   function call — built-in or user-defined
#   ( expr )     — parenthesised sub-expression

primary = NUMBER
        | BUILTIN_FN LPAREN expr RPAREN
        | USER_FN LPAREN expr RPAREN
        | variable
        | LPAREN expr RPAREN
        ;
```

---

## AST Node Shapes

The parser produces an `ASTNode` tree. Each node has a `rule_name` and a list
of `children` (mix of `ASTNode` and `Token`). Here are the shapes produced by
each rule, written as S-expressions for clarity.

### `program`
```
(program
  (line ...)
  (line ...)
  ...)
```

### `line`
```
(line
  Token(LINE_NUM, "10")
  (statement ...)      ← optional; absent for empty lines
  Token(NEWLINE))
```

### `let_stmt`
```
(let_stmt
  Token(KEYWORD, "LET")
  (variable ...)
  Token(EQ, "=")
  (expr ...))
```

### `print_stmt`
```
(print_stmt
  Token(KEYWORD, "PRINT")
  (print_list ...))   ← optional; absent for bare PRINT
```

### `print_list`
```
(print_list
  (print_item ...)
  Token(COMMA)
  (print_item ...)
  Token(SEMICOLON)
  ...)
```

### `if_stmt`
```
(if_stmt
  Token(KEYWORD, "IF")
  (expr ...)
  (relop Token(GT))
  (expr ...)
  Token(KEYWORD, "THEN")
  Token(LINE_NUM, "100"))
```

### `for_stmt`
```
(for_stmt
  Token(KEYWORD, "FOR")
  Token(NAME, "I")
  Token(EQ, "=")
  (expr ...)           ← start
  Token(KEYWORD, "TO")
  (expr ...)           ← limit
  Token(KEYWORD, "STEP")  ← optional
  (expr ...))             ← step (optional)
```

### `expr` (with operators)
```
(expr
  (term ...)
  Token(PLUS, "+")
  (term ...))
```

### `primary` (variable)
```
(primary
  (variable
    Token(NAME, "X")))
```

### `primary` (array)
```
(primary
  (variable
    Token(NAME, "A")
    Token(LPAREN)
    (expr ...)
    Token(RPAREN)))
```

### `primary` (built-in function call)
```
(primary
  Token(BUILTIN_FN, "SIN")
  Token(LPAREN)
  (expr ...)
  Token(RPAREN))
```

---

## Public API

```elixir
# Parse a token list (from the lexer) into an AST.
# The token list must include the final EOF token.
CodingAdventures.DartmouthBasicParser.parse(
  tokens :: [CodingAdventures.Lexer.Token.t()]
) :: {:ok, CodingAdventures.Parser.ASTNode.t()} | {:error, String.t()}

# Convenience: tokenise + parse in one call.
# Most callers use this.
CodingAdventures.DartmouthBasicParser.parse_source(
  source :: String.t()
) :: {:ok, CodingAdventures.Parser.ASTNode.t()} | {:error, String.t()}
```

---

## Package Structure

```
code/packages/elixir/dartmouth_basic_parser/
├── BUILD
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── mix.exs
├── lib/
│   └── coding_adventures/
│       └── dartmouth_basic_parser.ex
└── test/
    └── dartmouth_basic_parser_test.exs
```

Grammar file (shared with all language implementations):
```
code/grammars/
├── basic.tokens   ← defined in lexer spec
└── basic.grammar  ← defined in this spec
```

---

## `mix.exs` Dependencies

```elixir
defp deps do
  [
    {:coding_adventures_grammar_tools,      path: "../grammar_tools"},
    {:coding_adventures_lexer,              path: "../lexer"},
    {:coding_adventures_parser,             path: "../parser"},
    {:coding_adventures_dartmouth_basic_lexer, path: "../dartmouth_basic_lexer"},
  ]
end
```

---

## Implementation Notes

### Grammar Loading and Caching

```elixir
defp get_grammar do
  case :persistent_term.get({__MODULE__, :grammar}, nil) do
    nil ->
      grammar = load_grammar()
      :persistent_term.put({__MODULE__, :grammar}, grammar)
      grammar
    grammar ->
      grammar
  end
end

defp load_grammar do
  path = Path.join([@grammars_dir, "basic.grammar"])
  {:ok, grammar} = ParserGrammar.parse(File.read!(path))
  grammar
end
```

### Newline Significance

Because the grammar references `NEWLINE` tokens, the `parser` package
automatically operates in "newlines significant" mode. NEWLINE tokens are not
skipped between rule elements — they act as statement terminators.

### Error Messages

The `parser` package reports errors in terms of the token that was furthest
into the stream when parsing failed. For example:

```
Parse error at line 10, column 5:
  Expected: EQ ("=") after variable in LET statement
  Got: PLUS ("+")
```

---

## Test Strategy

Tests operate at the `parse_source/1` level — full source string in, AST out.
This exercises both the lexer and parser together and gives the most realistic
signal.

### Happy Path — All 17 Statement Types

Every statement type must have at least one test that:
1. Parses without error
2. Produces an AST with the correct `rule_name` at each node
3. Has the correct number and type of children

```elixir
# LET
"10 LET X = 5\n"
# → program → line → let_stmt with variable(X) and expr(NUMBER 5)

# PRINT bare
"10 PRINT\n"
# → print_stmt with no print_list

# PRINT with expression
"10 PRINT X + 1\n"
# → print_stmt → print_list → print_item → expr

# PRINT with string
"10 PRINT \"HELLO\"\n"
# → print_item with STRING token

# PRINT with comma separator
"10 PRINT X, Y\n"
# → print_list with two print_items separated by COMMA

# PRINT with semicolon separator
"10 PRINT X; Y\n"
# → print_list with SEMICOLON

# PRINT with trailing comma
"10 PRINT X,\n"
# → print_list ending with trailing COMMA (no final newline at runtime)

# INPUT
"10 INPUT X\n"
"10 INPUT A, B, C\n"

# IF ... THEN
"10 IF X > 0 THEN 100\n"
"10 IF A = B THEN 50\n"
"10 IF X <= Y THEN 30\n"
"10 IF X >= Y THEN 30\n"
"10 IF X <> Y THEN 30\n"

# GOTO
"10 GOTO 50\n"

# GOSUB / RETURN
"10 GOSUB 200\n"
"200 RETURN\n"

# FOR / NEXT (no STEP)
"10 FOR I = 1 TO 10\n20 NEXT I\n"

# FOR / NEXT (with STEP)
"10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n"

# END / STOP
"10 END\n"
"10 STOP\n"

# REM
"10 REM THIS IS A COMMENT\n"

# READ / DATA
"10 READ X\n20 DATA 3.14\n"
"10 READ A, B\n20 DATA 1, 2\n"

# RESTORE
"10 RESTORE\n"

# DIM
"10 DIM A(10)\n"
"10 DIM A(10), B(20)\n"

# DEF
"10 DEF FNA(X) = X * X\n"
"10 DEF FNB(T) = SIN(T) / T\n"
```

### Expression Precedence Tests

These tests verify that the AST structure correctly reflects precedence.
The compiler spec will walk this tree, so the shape must be right.

```elixir
# Addition binds looser than multiplication
"10 LET X = 2 + 3 * 4\n"
# expr must be: Add(2, Mul(3, 4))  — NOT Mul(Add(2,3), 4)

# Left-associativity of addition
"10 LET X = 1 + 2 + 3\n"
# expr must be: Add(Add(1, 2), 3)

# Right-associativity of exponentiation
"10 LET X = 2 ^ 3 ^ 2\n"
# power must be: Pow(2, Pow(3, 2)) = 512 — NOT Pow(Pow(2,3), 2) = 64

# Parentheses override precedence
"10 LET X = (2 + 3) * 4\n"
# expr must be: Mul(Add(2,3), 4)

# Unary minus
"10 LET X = -Y\n"
# unary must be: Neg(variable(Y))

# Unary minus with expression
"10 LET X = -(Y + 1)\n"
# unary must be: Neg(Add(variable(Y), 1))
```

### Built-in Function Call Tests

```elixir
"10 LET X = SIN(Y)\n"
"10 LET X = COS(Y)\n"
"10 LET X = SQR(Y * Y)\n"
"10 LET X = ABS(X - Y)\n"
"10 LET X = INT(3.7)\n"
"10 LET X = RND(1)\n"
"10 LET X = LOG(X)\n"
"10 LET X = EXP(X)\n"
"10 LET X = SGN(X)\n"
"10 LET X = ATN(X)\n"
"10 LET X = TAN(X)\n"
```

### User-Defined Function Tests

```elixir
# Definition
"10 DEF FNA(X) = X * X\n"

# Usage in expression
"20 LET Y = FNA(3)\n"
# primary must have USER_FN("FNA") and expr child
```

### Array Tests

```elixir
# Array read in expression
"10 LET X = A(3)\n"
# variable must be: NAME("A") + LPAREN + expr(3) + RPAREN

# Array write (LET target)
"10 LET A(I) = X + 1\n"
# let_stmt variable child must be array form
```

### Multi-Line Program Tests

```elixir
# Hello world
"""
10 PRINT "HELLO, WORLD"
20 END
"""

# Counting loop
"""
10 FOR I = 1 TO 5
20 PRINT I
30 NEXT I
40 END
"""

# Conditional
"""
10 INPUT X
20 IF X > 0 THEN 50
30 PRINT "NOT POSITIVE"
40 GOTO 60
50 PRINT "POSITIVE"
60 END
"""

# Subroutine
"""
10 GOSUB 100
20 END
100 PRINT "IN SUBROUTINE"
110 RETURN
"""
```

### Error Cases

```elixir
# Missing THEN in IF
"10 IF X > 0 100\n"
# → {:error, "Expected THEN..."}

# Missing = in LET
"10 LET X 5\n"
# → {:error, "Expected EQ..."}

# Unknown token in expression
"10 LET X = @\n"
# → {:error, "Unexpected token UNKNOWN..."}

# FOR without TO
"10 FOR I = 1\n"
# → {:error, "Expected TO..."}

# Line with no statement (valid — empty body)
"10\n"
# → {:ok, program with line having no statement child}
```

### Coverage Target: ≥ 95%

Every grammar rule must be exercised in at least one test. Every alternative in
`statement`, `relop`, `print_sep`, and `primary` must appear. Every optional
element (`STEP`, `print_list`, `statement` in `line`) must be tested both
present and absent.

---

## What This Package Does NOT Do

- **Evaluate expressions** — `2 + 3` remains a tree node, not `5`. That is the
  VM's (or compiler's) job.
- **Validate line number order** — `20 GOTO 10\n10 LET X = 1` is syntactically
  valid even though line 10 appears after line 20 in the source.
- **Check for undefined variables** — `LET X = Y` is valid even if Y has never
  been assigned. BASIC initialises all variables to 0.
- **Verify NEXT matches FOR** — `10 NEXT I` with no preceding `FOR I` is
  syntactically valid. The compiler checks this.
- **Resolve line number targets** — `GOTO 999` is valid even if line 999 does
  not exist. The VM detects this at runtime.
- **Handle REPL commands** — `LIST`, `RUN`, `NEW` are not part of the compiled
  BASIC language. They are REPL-level commands handled by the REPL layer.
