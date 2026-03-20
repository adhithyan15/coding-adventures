# 14 — Starlark: A Deterministic Configuration Language

## Overview

Starlark is a small, deterministic dialect of Python designed by Google for configuration files. It was created for Bazel (the build system), where BUILD files need to be declarative, reproducible, and safe to execute — even if authored by untrusted contributors. We are implementing Starlark using our own grammar-driven lexer and parser infrastructure.

This spec describes three things:

1. **What Starlark is** — its syntax, semantics, and deliberate limitations
2. **How to describe it** — the `starlark.tokens` and `starlark.grammar` files that feed our grammar-driven lexer/parser
3. **What our infrastructure needs to learn** — the extensions required to handle Starlark's complexity (indentation sensitivity, multi-line strings, comprehensions, etc.)

## Why Starlark?

Our monorepo currently uses shell-script BUILD files (see spec 12). This works but has problems:

- **No structure.** A BUILD file is just commands. There is no way to declare inputs, outputs, dependencies, or capabilities — you have to read the shell commands and guess.
- **No safety.** Shell scripts can do anything: `rm -rf /`, `curl | bash`, read environment secrets. There is no sandbox.
- **No composition.** You cannot define reusable rules like "a Python library test" and apply them uniformly across 140+ packages.

Starlark solves all three. It is:

- **Structured**: Function calls with named arguments naturally express build rules.
- **Sandboxed**: No file I/O, no network, no process spawning, no `import os`. By design.
- **Deterministic**: Same inputs → same outputs. No randomness, no ambient state.
- **Familiar**: It looks like Python. Anyone who can read Python can read Starlark.

## Why Build Our Own?

Google's reference implementation (`go.starlark.net`) exists and works. We could embed it. But:

1. **This is an educational project.** The whole point of coding-adventures is building things from scratch to understand them deeply.
2. **Stress test for our infrastructure.** Our grammar-driven lexer/parser currently handles simple expression grammars. Starlark — with indentation-based blocks, multi-line strings, comprehensions, and 15 keywords — is the first real language. If the infrastructure survives this, it can handle anything.
3. **Self-hosting milestone.** Eventually, we want our own parser to replace external dependencies (Python's `ast`, Ruby's `prism`, Go's `go/ast`, Rust's `syn`, TypeScript's compiler API). Starlark is a stepping stone — complex enough to be real, small enough to be tractable.
4. **Zero external dependencies.** Our Starlark implementation will use only our own lexer, parser, and grammar-tools packages. No third-party code in the critical path.

## Layer Position

```
Grammar Files (.tokens, .grammar)
        ↓
Grammar Tools (parse token/parser grammars, validate, cross-check)
        ↓
Lexer (grammar-driven: .tokens → token stream)
        ↓
Parser (grammar-driven: .grammar → AST)
        ↓
[YOU ARE HERE: Starlark — first real language target]
        ↓
BUILD file evaluation (execute Starlark AST to produce build graph)
```

**Input from:** `.star` and `BUILD` files (raw text).
**Output to:** Build system (a structured build graph of targets, dependencies, and actions).

## The Starlark Language

### Design Philosophy: Python Minus Danger

Starlark starts with Python's syntax and removes everything that makes programs unpredictable or unsafe:

| Python Feature | Starlark | Why Removed |
|----------------|----------|-------------|
| `while` loops | Removed | Could loop forever; configurations must terminate |
| `class` | Removed | No need for OOP in config files |
| `try`/`except` | Removed | Errors should halt, not be silently swallowed |
| `import` | Removed | Replaced with `load()` which has restricted semantics |
| `yield`/generators | Removed | Too complex for configuration |
| `async`/`await` | Removed | No concurrency in config evaluation |
| `global`/`nonlocal` | Removed | Mutable shared state causes nondeterminism |
| `eval()`/`exec()` | Removed | Dynamic code execution defeats static analysis |
| `del` | Removed | No need for explicit deletion |
| `with` | Removed | No resources to manage (no files, no locks) |
| Recursion | Disabled | Prevents stack overflow; ensures termination |
| `*` import | Removed | `load()` requires explicit symbol names |
| Mutable default args | Frozen | Default values are frozen after evaluation |

What remains is a clean, functional-ish subset: functions, conditionals, for-loops over finite collections, list/dict comprehensions, and first-class functions. Enough to express any build configuration; not enough to write a virus.

### Keywords

Starlark has 15 active keywords and 18 reserved-for-future keywords:

**Active (15):**
```
and    break    continue    def      elif
else   for      if          in       lambda
load   not      or          pass     return
```

**Reserved (18):** These are syntax errors if used as identifiers, even though Starlark does not implement them. This prevents code that looks like Python but does something different.
```
as       assert    async     await     class
del      except    finally   from      global
import   is        nonlocal  raise     try
while    with      yield
```

### Data Types

**Immutable:**
- `None` — the null value
- `bool` — `True` or `False`
- `int` — arbitrary-precision integers (no overflow)
- `float` — IEEE 754 double-precision
- `string` — Unicode text (immutable, not iterable)
- `bytes` — byte sequences (immutable)
- `tuple` — immutable ordered sequence

**Mutable:**
- `list` — ordered, mutable sequence
- `dict` — ordered key-value mapping (insertion order preserved)

**Callable:**
- `function` — first-class functions (user-defined or built-in)
- `lambda` — anonymous single-expression functions

### Operator Precedence

From lowest to highest binding:

```
1.  lambda                              lambda x: x + 1
2.  if-else                             a if cond else b
3.  or                                  a or b
4.  and                                 a and b
5.  not                                 not a
6.  in, not in, ==, !=, <, >, <=, >=   a in b, a == b
7.  |                                   a | b
8.  ^                                   a ^ b
9.  &                                   a & b
10. <<, >>                              a << b
11. +, -                                a + b
12. *, /, //, %                         a * b
13. +x, -x, ~x                         -a, ~a
14. **                                  a ** b
15. x.attr, x[i], x[i:j], f(args)      a.b, a[0], f(x)
```

### Statements

Starlark has a small set of statement types. Each statement is either simple (fits on one line) or compound (contains a block).

**Simple statements:**
```python
x = 1                          # assignment
x += 1                         # augmented assignment (+=, -=, *=, /=, //=, %=, &=, |=, ^=, <<=, >>=)
return x                       # return from function
pass                           # do nothing
break                          # exit for loop
continue                       # next iteration
load("file.star", "symbol")    # load symbols from another module
expression                     # expression evaluated for side effects
```

**Compound statements:**
```python
if x > 0:                      # conditional
    print(x)
elif x == 0:
    print("zero")
else:
    print("negative")

for item in collection:        # iteration (finite only — no while)
    process(item)

def function_name(a, b=10):   # function definition
    return a + b
```

### Expressions

**Literals:**
```python
42                             # int (decimal)
0x2A                           # int (hex)
0o52                           # int (octal)
3.14                           # float
1e-5                           # float (scientific)
"hello"                        # string (double-quoted)
'hello'                        # string (single-quoted)
"""multi
line"""                        # triple-quoted string
r"raw\nstring"                 # raw string (backslashes literal)
b"bytes"                       # bytes literal
True, False, None              # boolean and null literals
```

**Compound expressions:**
```python
[1, 2, 3]                     # list literal
(1, 2, 3)                     # tuple literal
{"a": 1, "b": 2}              # dict literal
[x*2 for x in lst]            # list comprehension
[x for x in lst if x > 0]     # filtered list comprehension
{k: v for k, v in pairs}      # dict comprehension
x if cond else y               # conditional expression (ternary)
lambda x, y: x + y            # anonymous function
```

**Access and calls:**
```python
obj.attr                       # attribute access
lst[0]                         # indexing
lst[1:3]                       # slicing
lst[::2]                       # slicing with step
f(a, b, key=val)               # function call with positional and keyword args
f(*args, **kwargs)             # unpacking in calls
```

### The `load()` Statement

`load()` is Starlark's replacement for Python's `import`. It has restricted semantics:

```python
load("//rules/python.star", "py_library", "py_test")
load(":helpers.star", renamed = "original_name")
```

Rules:
- Must appear at module top level (not inside functions or conditionals)
- First argument is a string path to the module
- Remaining arguments are symbol names to import (strings) or `alias = "name"` bindings
- Loaded modules are evaluated once and cached
- Circular loads are an error

## Grammar File: `starlark.tokens`

This section defines every lexical token in Starlark. The grammar-driven lexer reads this file and produces a token stream.

### Token Ordering Strategy

The `.tokens` format uses first-match-wins. This means:

1. **Multi-character operators before single-character.** `**` before `*`, `//` before `/`, `==` before `=`, `!=` before `!`, `<=` before `<`, `>=` before `>`, `<<=` before `<<` before `<`, `>>=` before `>>` before `>`.
2. **Longer literals first.** `"""` before `"`, `'''` before `'`.
3. **Regex patterns last among their group.** `NAME` is regex-based and must come after all literal keywords are checked (the `keywords:` section handles this).

### New Infrastructure Requirements for Tokens

The current `.tokens` format needs extensions to handle Starlark:

#### Extension 1: Single-Quoted Strings

The current STRING token only matches `"double-quoted"`. Starlark needs `'single-quoted'` too.

**Solution:** Add a second string pattern. The lexer's first-match-wins handles priority:

```
STRING_TRIPLE_DQ = /"""([^"\\]|\\.|\n)*"""/
STRING_TRIPLE_SQ = /'''([^'\\]|\\.|\n)*'''/
STRING_DQ    = /"([^"\\]|\\.)*"/
STRING_SQ    = /'([^'\\]|\\.)*'/
```

All four produce a `STRING` token type (the lexer maps token names to types, and multiple names can map to one type).

#### Extension 2: Token Type Aliasing

Multiple token definition names should be able to map to a single `TokenType`. This lets us define `STRING_DQ`, `STRING_SQ`, `STRING_TRIPLE_DQ`, `STRING_TRIPLE_SQ` in the `.tokens` file but have the parser see them all as `STRING`.

**Syntax in `.tokens` file:**
```
STRING_DQ    = /"([^"\\]|\\.)*"/       -> STRING
STRING_SQ    = /'([^'\\]|\\.)*'/       -> STRING
```

The `-> TYPE` suffix is optional. Without it, the token name IS the type (current behavior). With it, the name is for ordering/documentation but the emitted token type is the alias.

#### Extension 3: Float Literals

```
FLOAT = /[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?|\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+/
```

This handles `3.14`, `.5`, `5.`, `1e10`, `1.5e-3`. Must come before `INT` so `3.14` isn't split into `INT(3)` `.` `INT(14)`.

#### Extension 4: Integer Literal Variants

```
INT_HEX = /0[xX][0-9a-fA-F]+/        -> INT
INT_OCT = /0[oO][0-7]+/              -> INT
INT     = /[0-9]+/
```

Hex and octal must come before decimal so `0x1F` is one token, not `INT(0)` `NAME(x1F)`.

#### Extension 5: Comments

The lexer must skip `#` comments. Currently, the lexer has no comment support.

**Option A:** Hard-code comment handling in the lexer engine (comment syntax rarely varies across languages).

**Option B:** Add a `skip:` section to the `.tokens` format:
```
skip:
  COMMENT = /#[^\n]*/
  WHITESPACE = /[ \t]+/
```

Skip patterns are matched and consumed but produce no token. We recommend Option B for consistency with the grammar-driven approach.

#### Extension 6: Indentation Tokens (The Big One)

Starlark uses Python-style indentation for blocks. This is the hardest extension because it requires **stateful lexing** — the lexer must track an indentation stack and emit synthetic `INDENT`/`DEDENT` tokens.

**The algorithm** (from the Python language reference):

```
Before tokenization begins:
  indent_stack = [0]    # stack of indentation levels, starting at column 0

At the start of each logical line:
  count leading spaces (tabs are not allowed in Starlark)
  current_indent = len(leading_spaces)

  if current_indent > indent_stack.top():
      indent_stack.push(current_indent)
      emit INDENT token

  while current_indent < indent_stack.top():
      indent_stack.pop()
      emit DEDENT token

  if current_indent != indent_stack.top():
      error: inconsistent indentation

At end of input:
  while indent_stack.top() > 0:
      indent_stack.pop()
      emit DEDENT token
```

**Implicit line joining:** Expressions inside `()`, `[]`, or `{}` can span multiple lines without backslash continuation. The lexer must track bracket depth and suppress `NEWLINE`/`INDENT`/`DEDENT` emission while inside brackets.

**Why this is hard for a grammar-driven lexer:** The current lexer is stateless — it matches patterns against the current position and advances. Indentation tracking requires state (the indent stack) and bracket depth counting. This cannot be expressed as a regex pattern.

**Solution:** Add a `mode:` directive to the `.tokens` format:

```
mode: indentation
```

When present, the lexer engine activates the indentation tracking algorithm above. This is a pragmatic concession: indentation sensitivity is a well-known special case that very few languages use (Python, Starlark, Haskell, YAML), and encoding it as state-machine configuration is cleaner than trying to express it as regex patterns.

The `mode: indentation` directive tells the lexer engine:
1. Track leading whitespace at each logical line start
2. Maintain an indentation stack
3. Emit `INDENT` and `DEDENT` tokens
4. Emit `NEWLINE` tokens at logical line ends
5. Track bracket depth and suppress indentation processing inside brackets
6. Reject tab characters (Starlark requires spaces)

### Complete `starlark.tokens` File

```
# Token definitions for Starlark
# A deterministic subset of Python designed for configuration files.
#
# Starlark specification: https://github.com/bazelbuild/starlark/blob/master/spec.md
#
# This file feeds the grammar-driven lexer. The lexer reads these definitions
# and produces a token stream that the grammar-driven parser consumes.

# Lexer mode: indentation-sensitive (Python-style INDENT/DEDENT)
mode: indentation

# Skip patterns — matched and consumed but produce no tokens
skip:
  COMMENT    = /#[^\n]*/
  WHITESPACE = /[ \t]+/

# String literals — longest match first (triple-quoted before single-quoted)
# Raw strings: r"..." or r'...' (backslashes are literal)
# Byte strings: b"..." or b'...' (byte values only)
# Raw byte strings: rb"..." or br"..."
STRING_RAW_TRIPLE_DQ = /[rR][bB]?"""([^"\\]|\\.|\n)*"""|[bB][rR]"""([^"\\]|\\.|\n)*"""/   -> STRING
STRING_RAW_TRIPLE_SQ = /[rR][bB]?'''([^'\\]|\\.|\n)*'''|[bB][rR]'''([^'\\]|\\.|\n)*'''/   -> STRING
STRING_TRIPLE_DQ     = /[bB]?"""([^"\\]|\\.|\n)*"""/                                       -> STRING
STRING_TRIPLE_SQ     = /[bB]?'''([^'\\]|\\.|\n)*'''/                                       -> STRING
STRING_RAW_DQ        = /[rR][bB]?"([^"\\]|\\.)*"|[bB][rR]"([^"\\]|\\.)*"/                  -> STRING
STRING_RAW_SQ        = /[rR][bB]?'([^'\\]|\\.)*'|[bB][rR]'([^'\\]|\\.)*'/                  -> STRING
STRING_DQ            = /[bB]?"([^"\\]|\\.)*"/                                               -> STRING
STRING_SQ            = /[bB]?'([^'\\]|\\.)*'/                                               -> STRING

# Numeric literals — float before int (3.14 must not split into 3 . 14)
FLOAT       = /[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?|\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+/
INT_HEX     = /0[xX][0-9a-fA-F]+/     -> INT
INT_OCT     = /0[oO][0-7]+/           -> INT
INT         = /[0-9]+/

# Identifiers
NAME        = /[a-zA-Z_][a-zA-Z0-9_]*/

# Three-character operators
DOUBLE_STAR_EQUALS  = "**="
LEFT_SHIFT_EQUALS   = "<<="
RIGHT_SHIFT_EQUALS  = ">>="
FLOOR_DIV_EQUALS    = "//="

# Two-character operators (must come before single-char versions)
DOUBLE_STAR     = "**"
FLOOR_DIV       = "//"
LEFT_SHIFT      = "<<"
RIGHT_SHIFT     = ">>"
EQUALS_EQUALS   = "=="
NOT_EQUALS      = "!="
LESS_EQUALS     = "<="
GREATER_EQUALS  = ">="
PLUS_EQUALS     = "+="
MINUS_EQUALS    = "-="
STAR_EQUALS     = "*="
SLASH_EQUALS    = "/="
PERCENT_EQUALS  = "%="
AMP_EQUALS      = "&="
PIPE_EQUALS     = "|="
CARET_EQUALS    = "^="

# Single-character operators
PLUS            = "+"
MINUS           = "-"
STAR            = "*"
SLASH           = "/"
PERCENT         = "%"
EQUALS          = "="
LESS_THAN       = "<"
GREATER_THAN    = ">"
AMP             = "&"
PIPE            = "|"
CARET           = "^"
TILDE           = "~"

# Delimiters
LPAREN          = "("
RPAREN          = ")"
LBRACKET        = "["
RBRACKET        = "]"
LBRACE          = "{"
RBRACE          = "}"
COMMA           = ","
COLON           = ":"
SEMICOLON       = ";"
DOT             = "."

# Keywords — active in Starlark
keywords:
  and
  break
  continue
  def
  elif
  else
  for
  if
  in
  lambda
  load
  not
  or
  pass
  return
  True
  False
  None

# Reserved keywords — syntax errors if used as identifiers
reserved:
  as
  assert
  async
  await
  class
  del
  except
  finally
  from
  global
  import
  is
  nonlocal
  raise
  try
  while
  with
  yield
```

### Token Infrastructure Changes Summary

| Change | Scope | Description |
|--------|-------|-------------|
| `mode: indentation` | New directive | Activates indent/dedent/newline tracking |
| `skip:` section | New section | Patterns matched and discarded (comments, whitespace) |
| `-> TYPE` alias | New syntax | Multiple patterns emit same token type |
| `reserved:` section | New section | Keywords that are errors (not just reclassified) |
| Multi-pattern strings | New capability | Four string patterns all emitting STRING |
| Float tokens | New pattern | Floating-point literal recognition |
| Hex/octal integers | New patterns | `0xFF`, `0o77` integer variants |

## Grammar File: `starlark.grammar`

The parser grammar describes the syntactic structure of Starlark programs. The grammar-driven parser reads this file and uses recursive descent with backtracking to produce an AST.

### New Infrastructure Requirements for Grammar

#### Extension 1: INDENT/DEDENT Block Support

The grammar needs to reference `INDENT` and `DEDENT` tokens. These are synthetic tokens produced by the lexer's indentation tracking, but from the grammar's perspective they are just tokens like `LPAREN` or `RPAREN`.

No grammar format change needed — the parser already matches tokens by type name. We just need the lexer to emit `INDENT`, `DEDENT`, and `NEWLINE` tokens, and the grammar references them.

#### Extension 2: NEWLINE as Statement Separator

In the current grammars, statements are separated by consuming any `NEWLINE` tokens between them. Starlark makes newlines significant: they terminate statements (unless inside brackets).

The grammar uses `NEWLINE` explicitly:

```
simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;
```

#### Extension 3: Negative Lookahead (for `not in`)

Starlark has `not in` as a two-keyword operator. The grammar handles this by treating `not in` as a comparison operator alternative:

```
comp_op = EQUALS_EQUALS | NOT_EQUALS | LESS_THAN | GREATER_THAN
        | LESS_EQUALS | GREATER_EQUALS | "in" | "not" "in" ;
```

The parser's backtracking handles this naturally: if it sees `not` followed by `in`, it matches the two-keyword operator. If `not` is followed by something else, backtracking tries the next alternative.

#### Extension 4: Left-Recursion Avoidance

The grammar must avoid left recursion because our parser uses recursive descent. Starlark's operator precedence is encoded via a chain of rules, each level calling the next:

```
or_expr   = and_expr { "or" and_expr } ;
and_expr  = not_expr { "and" not_expr } ;
not_expr  = "not" not_expr | comparison ;
comparison = or_bitwise { comp_op or_bitwise } ;
...
```

This is the standard technique for encoding precedence in LL grammars.

### Complete `starlark.grammar` File

```
# Parser grammar for Starlark
# A deterministic subset of Python for configuration files.
#
# This file feeds the grammar-driven parser. The parser reads these rules
# and uses recursive descent with backtracking to produce an AST.
#
# UPPERCASE names reference tokens from starlark.tokens
# lowercase names reference other grammar rules
#
# Notation:
#   |       alternation (or)
#   { x }   zero or more repetitions
#   [ x ]   optional
#   ( x )   grouping
#   "lit"   literal token match (keyword or exact value)

# ============================================================================
# Top-Level Structure
# ============================================================================
#
# A Starlark file is a sequence of statements. At the top level, statements
# are separated by newlines. The file ends with EOF.

file = { NEWLINE | statement } ;

# ============================================================================
# Statements
# ============================================================================
#
# Statements come in two flavors:
#
# 1. Simple statements — fit on one line, separated by semicolons
# 2. Compound statements — contain indented blocks (if, for, def)

statement = compound_stmt | simple_stmt ;

simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;

small_stmt = return_stmt
           | break_stmt
           | continue_stmt
           | pass_stmt
           | load_stmt
           | assign_stmt ;

# ============================================================================
# Simple Statements
# ============================================================================

# Return — exits a function with an optional value.
# Only valid inside a def body (semantic check, not grammar check).
return_stmt = "return" [ expression ] ;

# Break/Continue — loop control. Only valid inside for body.
break_stmt    = "break" ;
continue_stmt = "continue" ;

# Pass — does nothing. Useful as placeholder in empty blocks.
pass_stmt = "pass" ;

# Load — imports symbols from another Starlark module.
#   load("path.star", "sym1", "sym2", alias = "sym3")
# The first argument is always a string. Remaining arguments are either
# string names or keyword=string bindings.
load_stmt = "load" LPAREN STRING { COMMA load_arg } [ COMMA ] RPAREN ;
load_arg  = NAME EQUALS STRING | STRING ;

# Assignment and expression statements.
# This rule handles three cases:
#   1. Simple assignment:    x = expr
#   2. Augmented assignment: x += expr
#   3. Expression statement: f(x)  (no assignment, just evaluation)
#
# We also handle tuple unpacking: a, b = 1, 2
# And chained access assignment: a.b = expr, a[i] = expr
assign_stmt = expression_list ( assign_op expression_list
                               | augmented_assign_op expression_list
                               ) | expression_list ;

assign_op          = EQUALS ;
augmented_assign_op = PLUS_EQUALS | MINUS_EQUALS | STAR_EQUALS | SLASH_EQUALS
                    | FLOOR_DIV_EQUALS | PERCENT_EQUALS | AMP_EQUALS
                    | PIPE_EQUALS | CARET_EQUALS | LEFT_SHIFT_EQUALS
                    | RIGHT_SHIFT_EQUALS | DOUBLE_STAR_EQUALS ;

# ============================================================================
# Compound Statements
# ============================================================================

compound_stmt = if_stmt | for_stmt | def_stmt ;

# If/elif/else chain.
# The INDENT/DEDENT tokens delimit the block body.
if_stmt = "if" expression COLON suite
          { "elif" expression COLON suite }
          [ "else" COLON suite ] ;

# For loop — iterates over a finite collection.
# Starlark has no while loop (ensures termination).
for_stmt = "for" loop_vars "in" expression COLON suite ;

loop_vars = NAME { COMMA NAME } ;

# Function definition.
def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;

# A suite is either:
#   1. A single simple_stmt on the same line (rare but valid)
#   2. An indented block of statements
suite = simple_stmt | NEWLINE INDENT { statement } DEDENT ;

# ============================================================================
# Function Parameters
# ============================================================================
#
# Parameters follow Python's model:
#   def f(a, b=1, *args, **kwargs):
#
# Positional parameters come first, then default-value parameters,
# then *args (collects remaining positionals), then **kwargs (collects
# remaining keywords).

parameters = parameter { COMMA parameter } [ COMMA ] ;
parameter  = DOUBLE_STAR NAME          # **kwargs
           | STAR NAME                  # *args
           | NAME EQUALS expression     # name=default
           | NAME ;                     # positional

# ============================================================================
# Expressions
# ============================================================================
#
# Expressions are organized by precedence, from lowest to highest.
# Each level calls the next, forming a chain that encodes precedence
# without ambiguity.

# Expression list (for tuple creation and multi-assignment)
expression_list = expression { COMMA expression } [ COMMA ] ;

# Conditional expression (ternary): a if cond else b
# This is the lowest-precedence expression.
expression = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;

# Lambda: lambda x, y: x + y
lambda_expr = "lambda" [ lambda_params ] COLON expression ;
lambda_params = lambda_param { COMMA lambda_param } [ COMMA ] ;
lambda_param = NAME [ EQUALS expression ] | STAR NAME | DOUBLE_STAR NAME ;

# Boolean operators (short-circuit evaluation)
or_expr  = and_expr { "or" and_expr } ;
and_expr = not_expr { "and" not_expr } ;
not_expr = "not" not_expr | comparison ;

# Comparison operators — non-associative in Starlark (a < b < c is an error)
# But we allow chaining in the grammar and enforce single-comparison semantically.
comparison = bitwise_or { comp_op bitwise_or } ;
comp_op    = EQUALS_EQUALS | NOT_EQUALS | LESS_THAN | GREATER_THAN
           | LESS_EQUALS | GREATER_EQUALS | "in" | "not" "in" ;

# Bitwise operators
bitwise_or  = bitwise_xor { PIPE bitwise_xor } ;
bitwise_xor = bitwise_and { CARET bitwise_and } ;
bitwise_and = shift { AMP shift } ;
shift       = arith { ( LEFT_SHIFT | RIGHT_SHIFT ) arith } ;

# Arithmetic operators
arith  = term { ( PLUS | MINUS ) term } ;
term   = factor { ( STAR | SLASH | FLOOR_DIV | PERCENT ) factor } ;

# Unary operators
factor = ( PLUS | MINUS | TILDE ) factor | power ;

# Exponentiation (right-associative)
power = primary [ DOUBLE_STAR factor ] ;

# ============================================================================
# Primary Expressions
# ============================================================================
#
# A primary expression is an atom followed by zero or more suffixes:
# attribute access (.attr), subscription ([i]), slicing ([i:j:k]),
# or function call (f(args)).

primary = atom { suffix } ;

suffix = DOT NAME                                              # attribute access
       | LBRACKET expression RBRACKET                          # indexing
       | LBRACKET [ expression ] COLON [ expression ] [ COLON [ expression ] ] RBRACKET   # slicing
       | LPAREN [ arguments ] RPAREN ;                         # function call

# ============================================================================
# Atoms
# ============================================================================
#
# Atoms are the leaves of the expression tree: literals, names,
# and bracketed constructs (lists, dicts, tuples, comprehensions).

atom = INT
     | FLOAT
     | STRING { STRING }          # adjacent string concatenation
     | NAME
     | "True" | "False" | "None"
     | list_expr
     | dict_expr
     | paren_expr ;

# List literal or list comprehension
list_expr = LBRACKET [ list_body ] RBRACKET ;
list_body = expression comp_clause               # list comprehension
          | expression { COMMA expression } [ COMMA ] ;   # list literal

# Dict literal or dict comprehension
dict_expr = LBRACE [ dict_body ] RBRACE ;
dict_body = dict_entry comp_clause               # dict comprehension
          | dict_entry { COMMA dict_entry } [ COMMA ] ;   # dict literal
dict_entry = expression COLON expression ;

# Parenthesized expression or tuple
paren_expr = LPAREN [ paren_body ] RPAREN ;
paren_body = expression comp_clause              # generator (error in Starlark, but parse it)
           | expression COMMA [ expression { COMMA expression } [ COMMA ] ]   # tuple
           | expression ;                        # parenthesized expression

# ============================================================================
# Comprehensions
# ============================================================================
#
# Comprehension clauses are shared between list, dict, and set comprehensions.
# A comprehension has one or more for-clauses with optional if-filters.

comp_clause = comp_for { comp_for | comp_if } ;
comp_for    = "for" loop_vars "in" or_expr ;
comp_if     = "if" or_expr ;

# ============================================================================
# Function Call Arguments
# ============================================================================
#
# Arguments can be positional, keyword, *args unpacking, or **kwargs unpacking.
#   f(1, 2, key=val, *rest, **more)

arguments = argument { COMMA argument } [ COMMA ] ;
argument  = DOUBLE_STAR expression           # **kwargs unpacking
          | STAR expression                   # *args unpacking
          | NAME EQUALS expression            # keyword argument
          | expression ;                      # positional argument
```

### Grammar Infrastructure Changes Summary

| Change | Scope | Description |
|--------|-------|-------------|
| `INDENT`/`DEDENT`/`NEWLINE` tokens | Parser must recognize | Synthetic tokens from lexer's indentation mode |
| Larger grammar | Stress test | ~40 rules vs current 7; deeper nesting |
| Keyword matching | Parser feature | `"not"` in grammar matches a KEYWORD token with value "not" |
| Trailing comma | Grammar pattern | `[ COMMA ]` at end of lists/params allows trailing commas |

## Infrastructure Extensions: Detailed Design

This section specifies exactly what changes each component needs to support the Starlark grammar.

### Grammar-Tools Extensions

The grammar-tools package (all 5 languages) must be extended to:

1. **Parse `mode:` directives** in `.tokens` files. Store as a field on `TokenGrammar`.
2. **Parse `skip:` sections** in `.tokens` files. Store as `skip_definitions: list[TokenDefinition]`.
3. **Parse `-> TYPE` aliases** on token definitions. Store as `alias: str | None` on `TokenDefinition`.
4. **Parse `reserved:` sections** in `.tokens` files. Store as `reserved_keywords: list[str]`.
5. **Validate aliases** — the alias target must be a valid token name (either defined or a well-known synthetic name like `STRING`, `INT`).
6. **Cross-validate** — `INDENT`, `DEDENT`, `NEWLINE` are implicitly defined when `mode: indentation` is active; the cross-validator should not warn about them being missing from `.tokens`.

### Lexer Extensions

The lexer (all 5 languages) must be extended to:

1. **Process `skip:` patterns.** After trying all token patterns and before reporting an error, try skip patterns. If one matches, consume the text and restart tokenization at the new position.

   Actually, the better approach: try skip patterns FIRST at each position. Keep consuming skip patterns until none match, then try token patterns. This correctly handles `x  +  y` where multiple spaces separate tokens.

2. **Process `-> TYPE` aliases.** When a pattern with an alias matches, emit a token with the alias as its type (not the pattern name).

3. **Implement `mode: indentation`.** This is the largest change. The lexer must:

   a. **Track bracket depth.** Increment on `(`, `[`, `{`; decrement on `)`, `]`, `}`. When depth > 0, suppress `NEWLINE`/`INDENT`/`DEDENT` emission.

   b. **Track indentation at line start.** At each logical line start (position 0, or after a `NEWLINE` token when bracket depth is 0), count leading spaces.

   c. **Emit synthetic tokens.** `INDENT` when indentation increases, one or more `DEDENT` when it decreases, `NEWLINE` at logical line ends.

   d. **Maintain an indentation stack.** Stack of integers, starting with `[0]`.

   e. **Handle blank lines and comment-only lines.** These do NOT affect indentation. Skip them without emitting `NEWLINE`.

   f. **Handle end-of-file.** Emit `DEDENT` for each remaining indentation level, then `NEWLINE` (if not already emitted), then `EOF`.

   g. **Reject tabs.** If a tab character appears in leading whitespace, emit an error.

4. **Handle `reserved:` keywords.** When a `NAME` token matches a reserved keyword, emit an error (not a token). This catches `class Foo` immediately at lex time rather than producing a confusing parse error.

### Parser Extensions

The parser (all 5 languages) needs relatively few changes:

1. **Handle `NEWLINE` tokens explicitly.** The current parser skips newlines automatically. For Starlark, newlines are significant — they terminate statements. The parser must match `NEWLINE` where the grammar says so.

   **Implementation:** Add a grammar-level directive or parser configuration that controls whether newlines are significant or ignored. When parsing a grammar with `NEWLINE` in its rules, the parser should NOT auto-skip them.

2. **Handle larger grammars efficiently.** The current parser uses backtracking for alternations. With ~40 rules and deeper nesting, pathological backtracking could be slow.

   **Mitigation:** Add memoization (packrat parsing). Cache the result of each `(rule, position)` pair. This converts worst-case exponential backtracking to O(n × rules) at the cost of O(n × rules) memory. For configuration files (typically < 1000 lines), this is acceptable.

3. **Better error messages.** With a more complex grammar, "unexpected token" is insufficient. The parser should report which rule it was matching and what it expected.

   **Implementation:** Track the "furthest failure" position. When parsing fails, report the error at the position where the parser got furthest into the input before all alternatives failed. Include the expected token types at that position.

## Implementation Plan

### Phase 1: Grammar Files (This PR)

Write `starlark.tokens` and `starlark.grammar` and validate them against the grammar-tools cross-validator (which will fail on new features — that's expected and documented).

**Files created:**
- `code/grammars/starlark.tokens`
- `code/grammars/starlark.grammar`

### Phase 2: Grammar-Tools Extensions

Extend the grammar-tools package in all 5 languages to parse the new `.tokens` features:

| Language | Package | Key Changes |
|----------|---------|-------------|
| Python | `grammar-tools` | Parse `mode:`, `skip:`, `-> TYPE`, `reserved:` |
| Ruby | `grammar_tools` | Same |
| Go | `grammar-tools` | Same |
| TypeScript | `grammar-tools` | Same |
| Rust | `grammar-tools` | Same |

**Verification:** `parse_token_grammar(starlark_tokens)` succeeds in all 5 languages. Cross-validation with `starlark.grammar` produces no errors.

### Phase 3: Lexer Extensions

Extend the lexer in all 5 languages:

| Feature | Complexity | Notes |
|---------|-----------|-------|
| `skip:` patterns | Low | Try before/between token patterns |
| `-> TYPE` aliases | Low | Map name to alias at emit time |
| `reserved:` keywords | Low | Error on reserved keyword match |
| `mode: indentation` | High | Indent stack, bracket depth, synthetic tokens |
| Comment handling | Low | Handled by `skip:` section |

**Verification:** Lex a Starlark BUILD file and verify the token stream matches expected output. Test cases:

```python
# Should produce: KEYWORD(def) NAME(add) LPAREN NAME(x) COMMA NAME(y) RPAREN COLON NEWLINE INDENT KEYWORD(return) NAME(x) PLUS NAME(y) NEWLINE DEDENT
def add(x, y):
    return x + y
```

### Phase 4: Parser Extensions

Extend the parser in all 5 languages:

| Feature | Complexity | Notes |
|---------|-----------|-------|
| Significant newlines | Medium | Don't auto-skip NEWLINE when grammar uses it |
| Packrat memoization | Medium | Cache `(rule, position)` results |
| Better error messages | Medium | Track furthest failure position |

**Verification:** Parse a complete BUILD file and produce a correct AST. Round-trip test: parse → pretty-print → parse again → compare ASTs.

### Phase 5: Starlark Evaluator

This is beyond the current spec but worth noting: after parsing, we need an evaluator that executes the AST to produce a build graph. The evaluator:

- Walks the AST and evaluates expressions
- Maintains an environment (variable bindings)
- Provides built-in functions (`py_library`, `py_test`, etc.)
- Enforces Starlark restrictions (no recursion, no while loops, no mutation after freeze)
- Produces a structured build graph as output

This will be a separate spec (15-starlark-evaluator.md).

## Test Strategy

### Grammar Files

Cross-validate `starlark.tokens` and `starlark.grammar` using the grammar-tools package. This catches:
- Token names referenced in grammar but not defined in tokens
- Rules referenced but not defined
- Unused tokens (warnings)
- Duplicate definitions

### Lexer Tests

Test the indentation algorithm thoroughly — it is the most error-prone component:

```python
# Test: simple indent/dedent
"if True:\n    x = 1\n"
# Expected: KEYWORD(if) KEYWORD(True) COLON NEWLINE INDENT NAME(x) EQUALS INT(1) NEWLINE DEDENT

# Test: nested indent
"if True:\n    if True:\n        x = 1\n"
# Expected: ... INDENT ... INDENT ... DEDENT DEDENT

# Test: implicit line joining
"x = (\n    1 +\n    2\n)\n"
# Expected: NAME(x) EQUALS LPAREN INT(1) PLUS INT(2) RPAREN NEWLINE (no INDENT/DEDENT)

# Test: blank lines don't affect indentation
"if True:\n\n    x = 1\n"
# Expected: same as without blank line

# Test: multiple dedents at once
"if True:\n    if True:\n        x = 1\nx = 2\n"
# Expected: ... DEDENT DEDENT NAME(x) EQUALS INT(2) NEWLINE

# Test: tabs are rejected
"if True:\n\tx = 1\n"
# Expected: LexError (tabs not allowed)
```

### Parser Tests

Test each grammar rule with minimal valid inputs:

```python
# Assignments
"x = 1\n"
"x, y = 1, 2\n"
"x += 1\n"

# If/elif/else
"if x:\n    pass\n"
"if x:\n    pass\nelif y:\n    pass\nelse:\n    pass\n"

# For loops
"for x in lst:\n    pass\n"
"for x, y in pairs:\n    pass\n"

# Function definitions
"def f():\n    pass\n"
"def f(a, b=1, *args, **kwargs):\n    return a\n"

# Comprehensions
"[x for x in lst]\n"
"[x for x in lst if x > 0]\n"
"{k: v for k, v in d.items()}\n"

# Lambda
"f = lambda x: x + 1\n"

# Load
'load("file.star", "sym1", alias = "sym2")\n'

# Complex nested expression
"result = [f(x) for x in data if x.valid]\n"
```

### Integration Tests

Parse real-world Bazel BUILD files and verify they produce valid ASTs. We can use BUILD files from well-known open-source projects (TensorFlow, Kubernetes) as test inputs.

## Appendix A: Starlark Built-in Functions

These functions are available in all Starlark environments. They are NOT defined in the grammar — they are provided by the evaluator as pre-bound names.

**Type constructors:** `bool`, `bytes`, `dict`, `float`, `int`, `list`, `str`, `tuple`

**Functional:** `all`, `any`, `enumerate`, `filter`, `map`, `max`, `min`, `range`, `reversed`, `sorted`, `zip`

**Introspection:** `dir`, `getattr`, `hasattr`, `hash`, `type`, `repr`

**I/O:** `print`

**Error:** `fail`

**Other:** `len`

## Appendix B: Escape Sequences in Starlark Strings

| Escape | Value | Notes |
|--------|-------|-------|
| `\\` | Backslash | |
| `\'` | Single quote | |
| `\"` | Double quote | |
| `\n` | Newline (LF) | |
| `\r` | Carriage return (CR) | |
| `\t` | Horizontal tab | |
| `\a` | Bell (alert) | |
| `\b` | Backspace | |
| `\f` | Form feed | |
| `\v` | Vertical tab | |
| `\0` | Null byte | |
| `\ooo` | Octal (1-3 digits, max 377) | |
| `\xHH` | Hex (exactly 2 digits) | |
| `\uXXXX` | Unicode (exactly 4 hex digits) | Strings only, not bytes |
| `\UXXXXXXXX` | Unicode (exactly 8 hex digits) | Strings only, not bytes |

Raw strings (`r"..."`) treat backslashes as literal characters except for `\'` and `\"`.

## Appendix C: Differences from Python

A concise reference for Python programmers reading Starlark:

| Python | Starlark | Notes |
|--------|----------|-------|
| `while True:` | Not available | Use `for` over finite collections |
| `class Foo:` | Not available | Use structs/dicts |
| `try: ... except:` | Not available | Errors halt execution |
| `import os` | `load("os.star", ...)` | Explicit symbol names required |
| `yield x` | Not available | No generators |
| `async def f():` | Not available | No concurrency |
| `global x` | Not available | No mutable global state |
| `del x` | Not available | No explicit deletion |
| `x = []; f(x); # x mutated` | Frozen after scope | Top-level values are frozen |
| `for c in "abc":` | Error | Strings are not iterable |
| `def f(x=[]):` | Default frozen | Mutable defaults are frozen at definition time |
| `f(f(f(f(x))))` | May error | Recursion is restricted/disabled |

## Appendix D: What This Enables

Once Starlark parsing is complete, BUILD files can look like this instead of shell scripts:

```python
load("//rules/python.star", "py_library", "py_test")
load("//rules/capability.star", "capability_check")

py_library(
    name = "logic-gates",
    srcs = glob(["src/**/*.py"]),
    deps = [],
    capabilities = [],  # pure computation — no OS access
)

py_test(
    name = "logic-gates-test",
    srcs = glob(["tests/**/*.py"]),
    deps = [":logic-gates"],
)

capability_check(
    name = "logic-gates-cap-check",
    target = ":logic-gates",
    manifest = "required_capabilities.json",
)
```

Every package gets a `capability_check` target automatically via the `py_library` rule. You cannot forget it. If you try to publish without declaring capabilities, the build fails. That is the payoff of moving from shell-script BUILD files to a structured configuration language.
