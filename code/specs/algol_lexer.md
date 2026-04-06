# ALGOL 60 Lexer

## Overview

This spec defines the lexer for ALGOL 60 — the 1960 language that gave computing its formal grammar
notation (BNF), block-structured scoping, and the call stack model that every language runtime uses
today. The lexer is the first transformation stage: it takes a raw string of ALGOL source and
produces a flat sequence of typed tokens.

ALGOL 60 is a historically important starting point for language implementation because:

- It was the **first language designed with a formal grammar** (the ALGOL 60 report used BNF to
  specify the entire language, written by John Backus and Peter Naur in 1960)
- It has a **simple, free-format source structure** — no column restrictions, no fixed-format
  punched-card layout unlike Fortran and COBOL of the same era
- Its token set is **clean and unambiguous** — no context-sensitive lexer modes, no special
  fixed-width strings, no embedded mini-languages (compare to COBOL's PICTURE clause)
- It is **statically typed** with only five primitive types, making it a good foundation before
  tackling dynamically typed or richly typed languages

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → [YOU ARE HERE] → Parser → Compiler → VM
```

**Input from:** Raw ALGOL 60 source text (a string).
**Output to:** ALGOL 60 parser (consumes the token stream to build an AST).

## Source Format

ALGOL 60 is **free-format**. Whitespace (spaces, tabs, newlines) is insignificant between tokens —
you may write a program entirely on one line or spread across hundreds. This is a deliberate
departure from Fortran 77 and COBOL, which inherited punched-card column restrictions.

```algol
begin integer x; x := 1 + 2 end   (* one line — valid *)

begin
  integer x;
  x := 1 + 2
end                                  (* spread across lines — identical meaning *)
```

The original ALGOL 60 report used special typographic conventions (bold keywords, mathematical
symbols like `≤`, `≠`, `↑` for exponentiation). Real hardware of the era could not print these,
so every implementation mapped them to ASCII sequences. This spec uses the ASCII conventions that
became standard.

## Concepts

### Keywords vs. Identifiers

ALGOL 60 keywords are **reserved words** — identifiers that the lexer reclassifies after an
initial NAME match. The lexer first matches any sequence of letters and digits starting with a
letter as a NAME token, then checks if its lowercase value appears in the keyword table. If it
does, the token kind is replaced with the keyword kind.

This means `begin` is the keyword BEGIN, but `beginning` is the identifier NAME("beginning") — the
keyword match requires the entire token to match, not just a prefix.

```
"begin"     → BEGIN      (keyword)
"beginning" → NAME       (identifier — keyword is a prefix but not a full match)
"Begin"     → BEGIN      (case-insensitive — normalized to lowercase first)
"INTEGER"   → INTEGER    (keyword — case-insensitive)
"integer1"  → NAME       (digit suffix makes it a non-keyword identifier)
```

### Assignment vs. Equality

ALGOL 60 makes a sharp distinction that C and its descendants later blurred:

- `:=` is the **assignment operator** — it stores a value into a variable
- `=` is the **equality test** — it compares two values, producing a boolean

```algol
x := 5       (* assign 5 to x *)
x = 5        (* test whether x equals 5, produces true or false *)
```

C chose `=` for assignment and `==` for equality. This caused decades of bugs:
`if (x = 5)` in C assigns 5 to x and tests the result (always true for nonzero), when the
programmer almost certainly meant `if (x == 5)`. ALGOL's `:=` convention, later adopted by
Pascal and Ada, eliminates this ambiguity entirely. The lexer must recognise `:=` as a single
token (ASSIGN) before it sees `:` alone (COLON).

### Exponentiation Operator

The ALGOL 60 report used the mathematical uparrow symbol `↑` for exponentiation. ASCII
implementations used either `^` (caret) or `**` (double star, borrowed from Fortran). This lexer
accepts both:

```algol
2 ^ 3     (* 8 — using caret *)
2 ** 3    (* 8 — using double star, same meaning *)
```

`**` must be lexed before `*` in the token rules so that `2 ** 3` produces
[INTEGER_LIT(2), POWER, INTEGER_LIT(3)] and not [INTEGER_LIT(2), STAR, STAR, INTEGER_LIT(3)].

### Real Number Literals

ALGOL 60 real literals use `E` notation for the exponent (all ASCII implementations). The original
report used a superscript `10` with an exponent, i.e. `1.5 × 10³`, but that is unrepresentable
in ASCII:

```
3.14          integer part + decimal part
1.5E3         1500.0  — E followed by positive exponent
1.5E-3        0.0015  — E followed by negative exponent
100E2         10000.0 — no decimal point, just integer with exponent
.5            0.5     — leading decimal point (some implementations)
```

The lexer distinguishes REAL_LIT from INTEGER_LIT by the presence of a decimal point or `E`.

### Comments

ALGOL 60 uses a keyword-based comment syntax — not a special character like `#` or `//`:

```algol
comment this is a comment and everything up to the semicolon is ignored;
```

The `comment` keyword signals that everything up to and including the next `;` is a comment.
Comments are silently consumed by the lexer and not emitted as tokens.

There is a second comment form: within a `begin...end` block, the text between `end` and the
next `;`, `end`, or `else` keyword is treated as a comment. This second form is a quirk of the
block structure and is handled at the parser level, not the lexer level. This spec covers only
the `comment ... ;` form.

### Boolean Keywords

ALGOL 60 uses English words for boolean operators rather than symbols:

| ALGOL 60 | C equivalent | Meaning |
|----------|-------------|---------|
| `not`    | `!`         | logical negation |
| `and`    | `&&`        | logical conjunction |
| `or`     | `\|\|`      | logical disjunction |
| `impl`   | (none)      | logical implication: `a impl b` = `not a or b` |
| `eqv`    | (none)      | logical equivalence: `a eqv b` = `(a and b) or (not a and not b)` |

`impl` and `eqv` have no direct equivalent in C or Java. They come from formal logic and were
included because ALGOL was designed by mathematicians who expected programs to express algorithms
in mathematical notation.

## Token Types

### Value Tokens

```
IDENT        letter followed by zero or more letters or digits.
             No underscore — original ALGOL did not allow it.
             Examples: x, sum, customerName, A1

INTEGER_LIT  One or more decimal digits, no decimal point, no exponent.
             Examples: 0, 42, 1000

REAL_LIT     A decimal number with a fractional part, an exponent, or both.
             Pattern: digits.digits? (E[+-]?digits)? | digits E[+-]?digits
             Examples: 3.14, 1.5E3, 1.5E-3, 100E2

STRING_LIT   Single-quoted sequence of characters. Single quotes cannot appear
             inside a string (no escape sequences in ALGOL 60).
             Examples: 'hello', 'x = 5', ''  (empty string)

TRUE         The literal boolean value true.
FALSE        The literal boolean value false.
```

### Operator Tokens (multi-character first)

```
ASSIGN       :=     assignment
POWER        **     exponentiation (ASCII alternative to ^)
LEQ          <=     less than or equal  (ASCII for ≤)
GEQ          >=     greater than or equal (ASCII for ≥)
NEQ          !=     not equal (ASCII for ≠)
```

### Operator Tokens (single character)

```
PLUS         +
MINUS        -
STAR         *
SLASH        /
CARET        ^      exponentiation (alternative to **)
EQ           =      equality test
LT           <
GT           >
```

### Delimiter Tokens

```
LPAREN       (
RPAREN       )
LBRACKET     [
RBRACKET     ]
SEMICOLON    ;
COMMA        ,
COLON        :
```

### Keywords

```
BEGIN        begin     block open
END          end       block close
IF           if        conditional
THEN         then      conditional branch
ELSE         else      conditional branch
FOR          for       loop
DO           do        loop body
STEP         step      loop step specification
UNTIL        until     loop limit
WHILE        while     loop while condition
GOTO         goto      unconditional jump
SWITCH       switch    computed goto table declaration
PROCEDURE    procedure subroutine/function declaration
INTEGER      integer   type declaration
REAL         real      type declaration
BOOLEAN      boolean   type declaration
STRING       string    type declaration
ARRAY        array     array declaration
OWN          own       static (retained between calls) storage
LABEL        label     parameter type specifier
VALUE        value     call-by-value parameter specifier
NOT          not       boolean negation
AND          and       boolean conjunction
OR           or        boolean disjunction
IMPL         impl      boolean implication
EQV          eqv       boolean equivalence
DIV          div       integer division
MOD          mod       integer modulo
COMMENT      comment   (triggers comment-skip mode; not emitted)
```

### Skip Patterns

```
WHITESPACE   spaces, tabs, carriage returns, newlines — consumed silently
COMMENT      comment...;  — consumed silently (see Comments section)
```

## Token Grammar File

The `.tokens` grammar for the grammar-driven lexer engine:

```
# Token definitions for ALGOL 60
# @version 1
#
# ALGOL 60 is the ancestor of nearly every modern language. It was the first
# language specified with a formal grammar (BNF). Unlike Fortran and COBOL of
# the same era, ALGOL source is free-format — no column restrictions.
#
# Key disambiguation rules:
#   := must come before : (assignment before colon)
#   ** must come before * (power before star)
#   <= must come before < (leq before lt)
#   >= must come before > (geq before gt)
#   != must come before ! (if ! were a token, which it isn't in ALGOL)
#   REAL_LIT must come before INTEGER_LIT (decimal/exponent before plain digits)

# --- Value tokens ---

REAL_LIT     = /[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+/
INTEGER_LIT  = /[0-9]+/
STRING_LIT   = /'[^']*'/
IDENT        = /[a-zA-Z][a-zA-Z0-9]*/

# --- Multi-character operators (must precede single-char versions) ---

ASSIGN       = ":="
POWER        = "**"
LEQ          = "<="
GEQ          = ">="
NEQ          = "!="

# --- Single-character operators ---

PLUS         = "+"
MINUS        = "-"
STAR         = "*"
SLASH        = "/"
CARET        = "^"
EQ           = "="
LT           = "<"
GT           = ">"

# --- Delimiters ---

LPAREN       = "("
RPAREN       = ")"
LBRACKET     = "["
RBRACKET     = "]"
SEMICOLON    = ";"
COMMA        = ","
COLON        = ":"

# --- Keywords (reclassified from IDENT) ---

keywords:
  begin
  end
  if
  then
  else
  for
  do
  step
  until
  while
  goto
  switch
  procedure
  integer
  real
  boolean
  string
  array
  own
  label
  value
  true
  false
  not
  and
  or
  impl
  eqv
  div
  mod
  comment

# --- Skip patterns ---

skip:
  WHITESPACE = /[ \t\r\n]+/
  COMMENT    = /comment[^;]*;/
```

## Public API

```elixir
# Token kinds
@type token_kind ::
  :ident | :integer_lit | :real_lit | :string_lit |
  :assign | :power | :leq | :geq | :neq |
  :plus | :minus | :star | :slash | :caret | :eq | :lt | :gt |
  :lparen | :rparen | :lbracket | :rbracket | :semicolon | :comma | :colon |
  :begin | :end | :if | :then | :else | :for | :do | :step | :until | :while |
  :goto | :switch | :procedure | :integer | :real | :boolean | :string |
  :array | :own | :label | :value | :true | :false |
  :not | :and | :or | :impl | :eqv | :div | :mod |
  :eof

@type token :: %{
  kind:   token_kind,
  value:  String.t(),      # raw source text of the token
  line:   pos_integer(),
  column: pos_integer()
}

# Main API
@spec tokenize(String.t()) :: {:ok, [token]} | {:error, String.t()}
```

## Data Flow

```
Input:  ALGOL 60 source string
           ↓
        [Column stripping]   ← none needed, ALGOL is free-format
           ↓
        [Grammar-driven lexer engine]
           reads algol.tokens grammar
           applies regex/literal rules in order
           reclassifies IDENT → keyword when value in keyword table
           skips WHITESPACE and COMMENT patterns
           ↓
Output: List of token maps with kind, value, line, column
        Final token is always EOF
```

## Test Strategy

### Individual tokens
- Each keyword: `begin`, `end`, `if`, `procedure`, `integer`, `real`, `boolean`, etc.
- Identifier: `x`, `sum`, `customerName`, `A1`
- Integer literals: `0`, `42`, `1000`
- Real literals: `3.14`, `1.5E3`, `1.5E-3`, `100E2`, `0.5`
- String literals: `'hello'`, `'x = 5'`, `''` (empty)
- All operators: `:=`, `**`, `<=`, `>=`, `!=`, `+`, `-`, `*`, `/`, `^`, `=`, `<`, `>`
- All delimiters: `(`, `)`, `[`, `]`, `;`, `,`, `:`

### Disambiguation
- `:=` produces ASSIGN, not COLON then EQ
- `**` produces POWER, not STAR then STAR
- `<=` produces LEQ, not LT then EQ
- `>=` produces GEQ, not GT then EQ
- Real before integer: `3.14` produces REAL_LIT, not INTEGER_LIT then DOT then INTEGER_LIT

### Keyword boundary
- `begin` → BEGIN
- `beginning` → NAME (not a keyword — full token must match)
- `INTEGER` → INTEGER (case-insensitive)
- `integer1` → NAME (digit suffix disqualifies keyword match)
- `realvalue` → NAME (not `real` + `value`, it's one token)

### Comments
- `comment this is ignored; x := 1` → only [IDENT(x), ASSIGN, INTEGER_LIT(1)]
- Comment containing semicolons-in-strings: not a concern (ALGOL comments end at first `;`)
- Comment at end of file with terminating `;`

### Whitespace
- Newlines, tabs, multiple spaces between tokens are all consumed
- `x:=1` (no spaces) produces same tokens as `x := 1`

### Multi-token expressions
- `x := 1 + 2 * 3` → IDENT, ASSIGN, INTEGER_LIT, PLUS, INTEGER_LIT, STAR, INTEGER_LIT
- `if x <= 0 then x := -x` → IF, IDENT, LEQ, INTEGER_LIT, THEN, IDENT, ASSIGN, MINUS, IDENT
- `2 ** 3 ^ 4` → INTEGER_LIT, POWER, INTEGER_LIT, CARET, INTEGER_LIT

## Future Extensions

- **Hollerith-style strings**: Some implementations used `n"text"` counted strings. Not in ALGOL 60 proper.
- **Nested comment forms**: The `end ... end` comment convention at the parser level.
- **Unicode operators**: Accept `≤`, `≥`, `≠`, `↑`, `÷` as alternatives to their ASCII equivalents.
- **Source positions as byte offsets**: Replace line/column with byte spans for use with the
  preprocessor_core span model (enables macro expansion provenance tracking).
