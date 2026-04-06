# PL01 — Dartmouth BASIC (1964)

## Overview

BASIC — **Beginner's All-purpose Symbolic Instruction Code** — was created by
John G. Kemeny and Thomas E. Kurtz at Dartmouth College and ran for the first
time on May 1, 1964. Their goal was radical for its era: make programming
accessible to every student at a liberal arts college, not just mathematics and
science majors.

Dartmouth's General Electric 225 mainframe ran a time-sharing operating system
that let dozens of students use the computer simultaneously from teletype
terminals. Each user got a few seconds of CPU time per turn. BASIC was designed
to fit this environment: programs were small, feedback was immediate, and error
messages were comprehensible to someone who had never programmed before.

Within two decades, BASIC had shipped on virtually every personal computer ever
sold. The 1964 Dartmouth specification is the root of all of them.

This spec covers **only the 1964 original**. No strings variables. No `ELSE`
branches. No `WHILE` loops. No graphics. This is the minimal, mathematically
pure, historically faithful version — 17 statements, 11 built-in functions, and
a philosophy that computing should feel like conversation.

## Historical Context

### Why Line Numbers?

Kemeny and Kurtz did not invent line numbers for aesthetics. They were a
practical solution to an engineering constraint: the teletype terminals of 1964
had no cursor keys, no backspace that erased, and no screen. A "terminal" was a
printer. You could not go back and fix a line — you could only type a new one.

Line numbers gave every statement an address. To replace line 30, you simply
typed a new line 30. The system kept lines sorted by number. To delete line 30,
you typed `30` with nothing after it. This was version control for a world
without editors.

### The Time-Sharing Context

The GE-225 would interleave dozens of BASIC sessions. Each user's program and
variables lived in memory. When you typed `RUN`, the system queued your program
for execution. When it was your turn, execution ran until either it finished or
your time slice expired. The interactive feel was an illusion of simultaneity.

This context explains why BASIC's REPL is dual-mode. You interact with the
system even when not running a program — storing lines, listing them, erasing
them. The terminal session itself is a persistent workspace.

## Layer Position

```
                   User types at terminal
                          ↓
          ┌───────────────────────────────┐
          │     REPL (PL00)               │
          │  dual-mode: store vs execute  │
          └───────────────────────────────┘
                          ↓ immediate commands or RUN
          ┌───────────────────────────────┐
          │     BASIC Source Code         │
          │  10 LET X = 5                 │
          │  20 PRINT X * 2               │
          │  30 END                       │
          └───────────────────────────────┘
                          ↓
          ┌───────────────────────────────┐
          │     Lexer  (basic.tokens)     │
          │  tokens: LINE_NUM, KEYWORD,   │
          │  NUMBER, STRING, OP, etc.     │
          └───────────────────────────────┘
                          ↓
          ┌───────────────────────────────┐
          │     Parser  (basic.grammar)   │
          │  AST: program → line+         │
          │       line → stmt             │
          └───────────────────────────────┘
                          ↓
          ┌───────────────────────────────┐
          │  Bytecode Compiler            │
          │  register_rule handlers       │
          │  per BASIC statement type     │
          └───────────────────────────────┘
                          ↓  CodeObject
          ┌───────────────────────────────┐
          │  Virtual Machine              │
          │  register_opcode handlers     │
          │  per BASIC opcode             │
          └───────────────────────────────┘
                          ↓
                     output / VMTrace
```

**Depends on:** `lexer`, `parser`, `bytecode_compiler`, `virtual_machine`, `repl`
**Elixir package:** `code/packages/elixir/basic/`
**Module namespace:** `CodingAdventures.Basic`

## The BASIC REPL (Dual-Mode)

The BASIC REPL plugin implements `CodingAdventures.Repl.Language` with dual-mode
`eval/2`:

```
input starts with a number?
    YES → strip the line from the program buffer if it already exists,
          add this line to the program buffer (sorted by line number),
          return {:ok, nil, new_state}
    NO  → is it a REPL command (RUN, LIST, NEW, SAVE, OLD)?
              YES → handle the command, return result
              NO  → compile and execute as a single immediate statement,
                    return {:ok, output, new_state}
```

### REPL Commands (handled inside eval/2)

| Command | Effect |
|---------|--------|
| `RUN` | Compile the full program buffer and execute it |
| `LIST` | Print all stored lines in ascending line-number order |
| `LIST 10-50` | Print only lines 10 through 50 |
| `NEW` | Erase all stored lines and reset variable state |
| `SAVE "name"` | Write the program buffer to a named file |
| `OLD "name"` | Load a program from a named file into the buffer |

Typing just a line number (e.g. `30`) with no statement deletes line 30.

### Session State

```elixir
%CodingAdventures.Basic.SessionState{
  program: %{pos_integer() => String.t()},  # line_number => source_line
  variables: %{String.t() => number()},     # "X", "A1", etc.
  arrays: %{String.t() => :array.array()},  # "A", "B", etc.
  data_values: [number()],                  # from DATA statements (flattened)
  data_pointer: non_neg_integer()           # current READ position
}
```

## The Language

### Data Types

The 1964 BASIC has exactly **one data type: floating-point numbers**. There are
no strings, no booleans, no integers as a separate type. String *literals* can
appear in `PRINT` statements and `DATA` statements, but they cannot be stored in
variables.

All arithmetic uses IEEE 754 double-precision floating point. Results are printed
with up to 6 significant digits.

```
1/3    prints   .333333
22/7   prints   3.14286
```

### Variables

Variables are either:

- A single uppercase letter: `A` through `Z` (26 variables)
- A letter followed by a single digit: `A0` through `Z9` (260 variables)

All variables are global. All are initialised to 0 at program start. Variable
names are case-insensitive; `a` and `A` refer to the same variable.

```
10 LET A = 5
20 LET B2 = A * 3
30 PRINT B2
```

### Arrays

Arrays are declared with `DIM` and are single-dimensional. Subscripts are
1-based. Undeclared arrays default to size 10 (subscripts 1–10).

```
10 DIM A(100)
20 FOR I = 1 TO 100
30   LET A(I) = I * I
40 NEXT I
```

Array variables share the same name space as scalar variables. `A` and `A(1)`
refer to different things: `A` is a scalar, `A(1)` is the first element of the
array `A`.

### Expressions

Expressions follow standard algebraic precedence:

```
Highest: - (unary negation)
         ^ (exponentiation, right-associative)
         * /
         + -
Lowest:  = < > <= >= <> (relational — only in IF conditions)
```

Parentheses override precedence as expected.

**Operators:**

| Operator | Meaning | Example |
|----------|---------|---------|
| `+` | Addition | `X + 1` |
| `-` | Subtraction or unary negation | `X - 1`, `-X` |
| `*` | Multiplication | `X * Y` |
| `/` | Division | `X / Y` |
| `^` | Exponentiation | `X ^ 2` |
| `=` | Equal (in IF only) | `IF X = 0 THEN ...` |
| `<` | Less than | `IF X < 10 THEN ...` |
| `>` | Greater than | `IF X > 0 THEN ...` |
| `<=` | Less than or equal | `IF X <= 5 THEN ...` |
| `>=` | Greater than or equal | `IF X >= 0 THEN ...` |
| `<>` | Not equal | `IF X <> 0 THEN ...` |

There are no logical operators (`AND`, `OR`, `NOT`) in the 1964 spec. Complex
conditions must be expressed with nested `IF` statements or `GOTO`.

### Built-in Functions

| Function | Description | Domain |
|----------|-------------|--------|
| `SIN(x)` | Sine, x in radians | all reals |
| `COS(x)` | Cosine, x in radians | all reals |
| `TAN(x)` | Tangent, x in radians | x ≠ π/2 + nπ |
| `ATN(x)` | Arctangent, result in radians | all reals |
| `EXP(x)` | eˣ | all reals |
| `LOG(x)` | Natural logarithm | x > 0 |
| `SQR(x)` | Square root | x ≥ 0 |
| `INT(x)` | Floor (largest integer ≤ x) | all reals |
| `ABS(x)` | Absolute value | all reals |
| `RND(x)` | Random number in [0, 1) | x ignored |
| `SGN(x)` | Sign: -1, 0, or 1 | all reals |

User-defined functions are declared with `DEF FN` (see Statements below).

## Statement Reference

### LET — Assignment

```
LET var = expr
LET var(expr) = expr
```

Assigns the value of `expr` to a variable or array element. The keyword `LET`
is required in the 1964 spec (later dialects made it optional).

```
10 LET X = 3.14159
20 LET A(5) = X * 2
```

---

### PRINT — Output

```
PRINT [item [, item ...]]
PRINT [item [; item ...]]
```

Prints values to the terminal. A line with nothing after `PRINT` prints a blank
line.

**Separators control spacing:**

- **Comma (`,`)** — advance to the next print zone. Zones are 15 characters wide.
  Columns 1, 16, 31, 46, 61. If already past the last zone, move to the next line.
- **Semicolon (`;`)** — print the next item immediately adjacent, no spacing.
- **No trailing separator** — print a newline after the last item.

```
10 PRINT "A", "B", "C"     →   A              B              C
20 PRINT "X"; "Y"; "Z"     →   XYZ
30 PRINT 1, 2; 3            →   1               23
```

Numbers are printed with a leading space if positive, a minus sign if negative,
and a trailing space. This is the 1964 convention for column alignment.

---

### INPUT — Read from User

```
INPUT var [, var ...]
```

Suspends execution and displays a `?` prompt. The user types one or more
comma-separated values. They are assigned to the listed variables in order.

```
10 INPUT X
20 INPUT A, B, C
```

If the user types fewer values than requested, the system prompts again with
`??`. If more values are typed than needed, the extras are ignored.

In our VM implementation, INPUT reads from the `:input_queue` in `vm.extra`,
exactly as the Brainfuck VM reads from `:input_buffer`. No actual I/O occurs
inside the VM itself.

---

### IF — Conditional Branch

```
IF expr relop expr THEN line-number
```

Evaluates the condition. If true, jumps to the given line number. If false,
continues to the next line. There is no `ELSE` in the 1964 spec.

The condition must be a **relational expression** — two numeric expressions
separated by one of `=`, `<`, `>`, `<=`, `>=`, `<>`. Logical combinations
(`AND`, `OR`) do not exist in this version.

```
10 IF X > 0 THEN 50
20 PRINT "X is zero or negative"
30 END
50 PRINT "X is positive"
60 END
```

---

### GOTO — Unconditional Jump

```
GOTO line-number
```

Transfers control to the specified line number. If the line number does not
exist in the program, an error is raised at runtime.

```
10 PRINT "LOOP"
20 GOTO 10
```

---

### GOSUB and RETURN — Subroutines

```
GOSUB line-number
RETURN
```

`GOSUB` saves the current line number onto the call stack and transfers to the
specified line. `RETURN` pops the saved address and transfers back to the line
*after* the `GOSUB`.

Subroutines are not functions in the modern sense — they share all variables
with the calling code. There is no local scope.

```
10 GOSUB 100
20 PRINT "BACK IN MAIN"
30 END

100 PRINT "IN SUBROUTINE"
110 RETURN
```

---

### FOR and NEXT — Counted Loops

```
FOR var = expr TO expr [STEP expr]
  ...
NEXT var
```

`FOR` initialises `var` to the start expression, then on each iteration checks
whether `var` has exceeded the limit. If the limit is exceeded, execution
continues after the matching `NEXT`. Otherwise, the body executes and `var` is
incremented by `STEP` (default 1) before the next check.

```
10 FOR I = 1 TO 5
20   PRINT I
30 NEXT I
```

Prints 1, 2, 3, 4, 5 on separate lines.

STEP can be negative for countdown loops:

```
10 FOR I = 10 TO 1 STEP -1
20   PRINT I
30 NEXT I
```

**Loop entry semantics:** If the start value already exceeds the limit (with the
given STEP direction), the loop body is skipped entirely — execution jumps to the
line after `NEXT`. This is consistent with the original Dartmouth spec.

FOR/NEXT loops may be nested. Each `NEXT var` matches the innermost `FOR var`.

---

### END and STOP — Termination

```
END
STOP
```

`END` terminates the program normally. The last statement of every program must
be `END`. `STOP` is equivalent but historically printed `STOP at line N` on the
terminal to distinguish normal completion from `END`.

---

### REM — Remark

```
REM anything
```

A comment. The entire rest of the line is ignored. REM is a statement, not
inline syntax, so it occupies its own line number.

```
10 REM THIS PROGRAM COMPUTES SQUARES
20 FOR I = 1 TO 10
```

---

### READ and DATA — Inline Data

```
READ var [, var ...]
DATA literal [, literal ...]
RESTORE
```

`DATA` statements define a pool of literal values embedded in the program.
`READ` consumes values from that pool sequentially, assigning them to variables.
`RESTORE` resets the pointer back to the first DATA value.

DATA statements can appear anywhere in the program. At compile time (or load
time), all DATA values are collected into a single ordered list. The order is
the order of the line numbers, not the order the program executes.

```
10 READ A, B, C
20 PRINT A + B + C
30 END
40 DATA 10, 20, 30
```

Prints `60`.

---

### DIM — Array Declaration

```
DIM var(size) [, var(size) ...]
```

Declares an array with the given number of elements. Subscripts run from 1 to
`size`. Multiple arrays can be declared on one `DIM` line.

```
10 DIM A(50), B(20)
```

Undeclared arrays are automatically dimensioned to 10 (subscripts 1–10) on
first use. `DIM` must appear before the array is used if a size larger than 10
is needed.

---

### DEF FN — User-Defined Functions

```
DEF FNa(x) = expr
```

Defines a single-line function named `FNa` (where `a` is one letter). The
parameter `x` can be any variable name — it is local to the function body.
Functions can only be single expressions; they cannot span multiple lines or
call `GOSUB`.

```
10 DEF FNS(X) = X * X
20 PRINT FNS(5)
```

Prints `25`.

Functions are called as `FNa(arg)` in any expression. The function name is
always `FN` followed by a single letter, making up to 26 user functions possible.

## Complete Example Program

The following program demonstrates most of 1964 BASIC:

```
10  REM DARTMOUTH BASIC DEMO
20  DEF FNS(X) = X * X
30  DIM A(10)
40  FOR I = 1 TO 10
50    READ A(I)
60  NEXT I
70  LET S = 0
80  FOR I = 1 TO 10
90    LET S = S + FNS(A(I))
100 NEXT I
110 PRINT "SUM OF SQUARES =", S
120 INPUT X
130 IF X = 0 THEN 160
140 GOSUB 200
150 GOTO 120
160 PRINT "DONE"
170 END
200 PRINT "SQUARE OF"; X; "IS"; FNS(X)
210 RETURN
900 DATA 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
```

## Bytecode Design

### Opcodes

The BASIC compiler emits the following opcodes. All are registered as handlers
on the `GenericVM`.

| Opcode | Hex | Name | Stack effect | Description |
|--------|-----|------|--------------|-------------|
| 0x01 | `LOAD_CONST` | `(idx)` | `→ value` | Push constants[idx] |
| 0x02 | `LOAD_VAR` | `(idx)` | `→ value` | Push variables[names[idx]] |
| 0x03 | `STORE_VAR` | `(idx)` | `value →` | Pop and store in variables[names[idx]] |
| 0x04 | `LOAD_ARRAY` | `(idx)` | `subscript → value` | Pop subscript, push arrays[names[idx]][sub] |
| 0x05 | `STORE_ARRAY` | `(idx)` | `value subscript →` | Pop both, store in array element |
| 0x10 | `ADD` | | `a b → a+b` | |
| 0x11 | `SUB` | | `a b → a-b` | |
| 0x12 | `MUL` | | `a b → a*b` | |
| 0x13 | `DIV` | | `a b → a/b` | Division by zero raises error |
| 0x14 | `POW` | | `a b → a^b` | |
| 0x15 | `NEG` | | `a → -a` | Unary negation |
| 0x20 | `CMP_EQ` | | `a b → 1.0 or 0.0` | Equal |
| 0x21 | `CMP_LT` | | `a b → 1.0 or 0.0` | Less than |
| 0x22 | `CMP_GT` | | `a b → 1.0 or 0.0` | Greater than |
| 0x23 | `CMP_LE` | | `a b → 1.0 or 0.0` | Less than or equal |
| 0x24 | `CMP_GE` | | `a b → 1.0 or 0.0` | Greater than or equal |
| 0x25 | `CMP_NE` | | `a b → 1.0 or 0.0` | Not equal |
| 0x30 | `JUMP` | `(target)` | | Unconditional jump to PC=target |
| 0x31 | `JUMP_IF_FALSE` | `(target)` | `cond →` | Pop condition; if 0.0, jump |
| 0x40 | `PRINT_VALUE` | | `value →` | Pop and print value |
| 0x41 | `PRINT_STRING` | `(idx)` | | Print constants[idx] (string literal) |
| 0x42 | `PRINT_NEWLINE` | | | Print newline |
| 0x43 | `PRINT_TAB` | | | Advance to next print zone |
| 0x50 | `INPUT` | `(idx)` | `→ value` | Read one value from input queue into names[idx] |
| 0x60 | `CALL_BUILTIN` | `(idx)` | `arg → result` | Call builtins[names[idx]] with arg |
| 0x61 | `CALL_FN` | `(idx)` | `arg → result` | Call user DEF FN function |
| 0x70 | `FOR_INIT` | `(idx)` | `start limit step →` | Set up loop; stores loop frame in extra |
| 0x71 | `FOR_NEXT` | `(idx)` | | Increment var; jump back or exit loop |
| 0x80 | `GOSUB` | `(target)` | | Push CallFrame, jump to target |
| 0x81 | `RETURN` | | | Pop CallFrame, restore PC |
| 0x90 | `READ` | `(idx)` | | Read next DATA value into names[idx] |
| 0x91 | `RESTORE` | | | Reset data_pointer to 0 |
| 0xF0 | `LINE_MARKER` | `(line_no)` | | No-op; marks start of source line for GOTO resolution |
| 0xFF | `HALT` | | | Stop execution |

### LINE_MARKER and GOTO Resolution

Every BASIC statement compiles to a `LINE_MARKER` pseudo-instruction first:

```
source:        10 PRINT "HELLO"
               20 END

bytecode:
  pc=0   LINE_MARKER  10
  pc=1   LOAD_CONST   0        (index of "HELLO" in constants pool)
  pc=2   PRINT_STRING
  pc=3   PRINT_NEWLINE
  pc=4   LINE_MARKER  20
  pc=5   HALT
```

When the `CodeObject` is loaded into the VM, a single pass builds the line
number table and stores it in `vm.extra`:

```
:line_table => %{10 => 0, 20 => 4}
```

`GOTO 20` compiles to `JUMP` with operand `20` (the *line number*, not the PC).
The `JUMP` handler looks up `20` in `:line_table` and calls `jump_to(vm, 4)`.
This means no second compiler pass and no backpatching.

`LINE_MARKER` at runtime is a true no-op: it emits `{nil, vm}` from its handler
and the framework calls `advance_pc`. It exists only to populate the line table
on load.

### FOR/NEXT Loop State

Loop frames are stored in `vm.extra` under `:for_stack` (a list, head = innermost):

```elixir
%{
  var: "I",           # variable name
  limit: 10.0,        # the TO value
  step: 1.0,          # the STEP value (default 1)
  next_pc: 2          # PC of the FOR_INIT instruction (to jump back to body)
}
```

`FOR_INIT` pops start, limit, step from the stack, sets the variable, checks
whether the loop body should be skipped entirely (start already past limit),
and pushes a loop frame.

`FOR_NEXT` increments the variable by step, checks the limit, and either jumps
back to `next_pc` (loop again) or pops the frame and falls through.

### INPUT Queue

The VM is initialised with input values pre-loaded in `vm.extra`:

```elixir
:input_queue => ["3.14", "42", "0"]
```

The `INPUT` handler pops the first value, parses it as a float, and stores it
in the named variable. If the queue is empty, a `VMError` is raised.

This follows the same pattern as the Brainfuck VM's `:input_buffer` — the VM is
pure and deterministic; I/O is injected by the host.

### DATA Pool

All `DATA` values are extracted at compile time and stored as a constant in the
`CodeObject`. On VM initialisation, they are loaded into `vm.extra`:

```elixir
:data_values => [1.0, 2.0, 3.0, 10.0, 20.0, 30.0]  # all DATA values in line-order
:data_pointer => 0
```

`READ` reads `data_values[data_pointer]` and increments the pointer.
`RESTORE` sets `data_pointer` back to 0.

## Lexer Grammar (basic.tokens)

The token grammar defines the following token types:

| Token Type | Pattern | Examples |
|------------|---------|---------|
| `LINE_NUM` | `[0-9]+` at start of line | `10`, `999` |
| `NUMBER` | floating point | `3.14`, `42`, `.5` |
| `STRING` | `"[^"]*"` | `"HELLO"` |
| `KEYWORD` | reserved words | `PRINT`, `LET`, `IF`, `THEN`, ... |
| `FUNCTION` | `FN[A-Z]` or built-in names | `FNA`, `SIN`, `COS`, ... |
| `IDENT` | `[A-Z][0-9]?` | `X`, `A1`, `B9` |
| `PLUS` | `+` | |
| `MINUS` | `-` | |
| `STAR` | `*` | |
| `SLASH` | `/` | |
| `CARET` | `^` | |
| `EQ` | `=` | |
| `LT` | `<` | |
| `GT` | `>` | |
| `LE` | `<=` | |
| `GE` | `>=` | |
| `NE` | `<>` | |
| `LPAREN` | `(` | |
| `RPAREN` | `)` | |
| `COMMA` | `,` | |
| `SEMICOLON` | `;` | |
| `NEWLINE` | line boundary | |

Keywords are recognised before `IDENT` in the grammar so that `PRINT` does not
lex as an identifier.

## Parser Grammar (basic.grammar)

```
program      := line+

line         := LINE_NUM statement NEWLINE

statement    := let_stmt
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

let_stmt     := "LET" variable "=" expr
print_stmt   := "PRINT" [print_list]
input_stmt   := "INPUT" variable ("," variable)*
if_stmt      := "IF" expr relop expr "THEN" LINE_NUM
goto_stmt    := "GOTO" LINE_NUM
gosub_stmt   := "GOSUB" LINE_NUM
return_stmt  := "RETURN"
for_stmt     := "FOR" IDENT "=" expr "TO" expr ["STEP" expr]
next_stmt    := "NEXT" IDENT
end_stmt     := "END"
stop_stmt    := "STOP"
rem_stmt     := "REM" (anything until NEWLINE)
read_stmt    := "READ" variable ("," variable)*
data_stmt    := "DATA" literal ("," literal)*
restore_stmt := "RESTORE"
dim_stmt     := "DIM" IDENT "(" NUMBER ")" ("," IDENT "(" NUMBER ")")*
def_stmt     := "DEF" FUNCTION "(" IDENT ")" "=" expr

print_list   := print_item (print_sep print_item)* [print_sep]
print_item   := expr | STRING
print_sep    := "," | ";"

variable     := IDENT | IDENT "(" expr ")"
relop        := "=" | "<" | ">" | "<=" | ">=" | "<>"

expr         := term (("+"|"-") term)*
term         := power (("*"|"/") power)*
power        := unary ["^" power]
unary        := "-" primary | primary
primary      := NUMBER
              | variable
              | FUNCTION "(" expr ")"
              | "(" expr ")"
```

Newlines are **significant** in BASIC — they terminate statements. The parser
must be configured with `newlines_significant: true`.

## Public API

```elixir
# Compile a complete BASIC program to a CodeObject
CodingAdventures.Basic.compile(source :: String.t()) ::
  {:ok, CodeObject.t()} | {:error, String.t()}

# Execute a compiled program with optional pre-loaded inputs
CodingAdventures.Basic.run(
  code :: CodeObject.t(),
  input :: [String.t()]
) :: {:ok, output :: [String.t()]} | {:error, String.t()}

# Compile and run in one step (most common for testing)
CodingAdventures.Basic.execute(
  source :: String.t(),
  input :: [String.t()]
) :: {:ok, output :: [String.t()]} | {:error, String.t()}

# Start an interactive REPL session (blocks)
CodingAdventures.Basic.repl() :: :ok

# Return the Language behaviour module for use with the REPL framework
CodingAdventures.Basic.language_module() :: module()
```

## Error Handling

Runtime errors that correspond to the original Dartmouth system's error messages:

| Error | Condition |
|-------|-----------|
| `UNDEFINED LINE NUMBER` | GOTO/GOSUB target line does not exist |
| `UNDEFINED VARIABLE` | Variable used before assignment (should return 0; only an error for arrays) |
| `SUBSCRIPT OUT OF RANGE` | Array subscript < 1 or > DIM size |
| `DIVISION BY ZERO` | Denominator is 0 |
| `ILLEGAL QUANTITY` | SQR of negative, LOG of non-positive |
| `OUT OF DATA` | READ with no more DATA values |
| `STACK OVERFLOW` | Too many nested GOSUB calls |
| `NEXT WITHOUT FOR` | NEXT with no matching FOR |
| `FOR WITHOUT NEXT` | Program ends with unclosed FOR |

## Test Strategy

Tests are organised in layers, testing each component independently before
integration:

### Lexer Tests
- All 17 statement keywords tokenise correctly
- Line numbers are separate from expression numbers
- String literals preserve content including spaces
- Operators `<=`, `>=`, `<>` tokenise as single tokens (not two)

### Parser Tests
- Every statement type produces the correct AST shape
- Operator precedence: `2 + 3 * 4` parses as `2 + (3 * 4)`
- `FOR I = 1 TO 10` with and without STEP
- Nested parentheses in expressions

### Compiler Tests
- `LINE_MARKER` emitted first for every line
- GOTO target is the line number (not a PC offset)
- DATA values collected in line-number order
- FOR/NEXT emits `FOR_INIT` and `FOR_NEXT` with matching variable name

### VM Tests
- LINE_MARKER is a no-op that populates `:line_table` on load
- GOTO resolves line number to PC via `:line_table`
- FOR/NEXT loop executes correct number of times including edge cases:
  - `FOR I = 1 TO 1` (exactly one iteration)
  - `FOR I = 5 TO 1` (zero iterations, no STEP)
  - `FOR I = 5 TO 1 STEP -1` (five iterations)
- GOSUB/RETURN correctly restores PC
- READ/DATA/RESTORE sequential and reset behaviour
- INPUT reads from `:input_queue` in order

### Integration Tests (sample programs)
- Hello world
- Sum 1 through N using a FOR loop
- Fibonacci sequence using GOTO
- Factorial using GOSUB
- Guessing game using IF/INPUT/GOTO
- Bubble sort using nested FOR loops and arrays
- Population model using DEF FN and READ/DATA

## Future Extensions (beyond 1964)

These are explicitly out of scope for this spec. They belong in a future `PL02`
spec if desired.

| Feature | Notes |
|---------|-------|
| String variables (`A$`) | Not in 1964 original; added in later BASIC dialects |
| `ELSE` clause | Not in 1964 original |
| `WHILE`/`WEND` | Not in 1964 original |
| `PRINT USING` | Formatted output; added in later dialects |
| Multi-statement lines (`10 LET X=1 : PRINT X`) | `:` separator; not in 1964 |
| Matrix operations (`MAT`) | Present in the 1964 manual as an optional extension |
