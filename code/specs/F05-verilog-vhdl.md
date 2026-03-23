# F05 — Verilog & VHDL Lexer/Parser

## Overview

Verilog and VHDL are the two dominant **Hardware Description Languages** (HDLs).
Where our existing packages build circuits from logic gates in code, HDLs let
you *describe* circuits in a text file and then synthesize, simulate, or
visualize them. This spec adds lexer and parser support for both languages,
reusing the grammar-driven infrastructure already in place.

The Verilog preprocessor (`` `define ``, `` `ifdef ``, `` `include ``) is a
simplified C preprocessor. Building it here creates a stepping stone toward
full C preprocessor support later.

### Why Both Languages?

| | Verilog | VHDL |
|---|---------|------|
| **Origin** | Gateway Design Automation, 1984 | US Dept. of Defense, 1987 |
| **Style** | C-like, concise | Ada-like, verbose |
| **Typing** | Weak / implicit | Strong / explicit |
| **Case** | Sensitive | Insensitive |
| **Preprocessor** | Yes (`` `define ``, `` `ifdef ``) | No |
| **Signal assignment** | `=` (blocking), `<=` (non-blocking) | `<=` (signal), `:=` (variable) |

Both are IEEE standards (Verilog: IEEE 1364-2005, VHDL: IEEE 1076-2008), both
target the same domain (digital circuit description), and comparing them
side-by-side is one of the most effective ways to understand HDL design.

### Relationship to Existing Specs

- **02-lexer.md**: Base lexer — we use `GrammarLexer` with `.tokens` files
- **03-parser.md**: Base parser — we use `GrammarParser` with `.grammar` files
- **lexer-parser-hooks.md**: The hook API is specified but not yet implemented
  in the base lexer package. Hooks are applied directly in the wrapper:
  Verilog preprocessor runs on source text before creating `GrammarLexer`;
  VHDL case normalization runs on the token list after `GrammarLexer.tokenize()`
- **F04-lexer-pattern-groups.md**: Not needed — both HDLs are context-free at
  the lexical level
- **F01-fpga.md**: The FPGA package consumes HDL descriptions; this spec
  produces the parsed representation that feeds into synthesis

## Layer Position

```
HDL Source Text (Verilog / VHDL)
    │
    ▼
┌─────────────────────────────┐
│  Lexer (this spec)          │
│  verilog.tokens / vhdl.tokens│
│  + preprocessor hook        │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│  Parser (this spec)         │
│  verilog.grammar / vhdl.grammar│
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│  AST (output)               │
│  Module/Entity declarations │
│  Port lists, assignments    │
│  Always/Process blocks      │
└─────────────────────────────┘
    │
    ▼ (future)
  Synthesis → Bitstream → FPGA (F01)
```

## Grammar Scope

We implement a **synthesizable subset** of each language — the constructs that
map to real hardware. This deliberately excludes simulation-only constructs
(testbench scaffolding, file I/O, delays for simulation) to keep the grammar
tractable and focused on the FPGA pipeline.

### Included (both languages)

- Module / entity declarations with ports and parameters / generics
- Wire, reg, signal, variable declarations with bit widths / types
- Continuous / concurrent assignments
- Procedural blocks (always / process) with sensitivity lists
- If/else, case/when statements
- Begin/end blocks
- Module / component instantiation with port connections
- Generate blocks (for, if)
- Expressions with full operator precedence
- Concatenation and replication (Verilog) / aggregates (VHDL)

### Excluded (deferred to future specs)

- Testbench constructs (`initial`, `$display`, `$monitor`, `wait`)
- File I/O (`$fopen`, `$fwrite`, `textio`)
- Delay specifications (`#10`, `after 10 ns`)
- SystemVerilog extensions (interfaces, classes, assertions)
- VHDL-2008 extensions (external names, force/release)
- Configuration declarations (VHDL)
- Specify blocks and timing checks (Verilog)

---

## Part 1: Verilog

### 1.1 Token Inventory

#### Comments (skip patterns)

```
// single-line comment
/* block comment
   spanning multiple lines */
```

Both are consumed by skip patterns and produce no tokens.

#### String Literals

```
"Hello, world!\n"
```

Double-quoted strings with C-style escape sequences (`\n`, `\t`, `\\`, `\"`).

#### Number Literals

Verilog numbers are the most distinctive token type. The format is:

```
[size]'[signed][base]digits
```

Where:
- **size** (optional): decimal integer specifying bit width
- **signed** (optional): `s` or `S` for signed interpretation
- **base**: `b`/`B` (binary), `o`/`O` (octal), `d`/`D` (decimal), `h`/`H` (hex)
- **digits**: `0-9`, `a-f`/`A-F` (hex), `x`/`X` (unknown), `z`/`Z` (high-impedance), `_` (separator)

Examples and their meanings:

| Literal | Size | Base | Value | Binary |
|---------|------|------|-------|--------|
| `4'b1010` | 4 bits | binary | 10 | `1010` |
| `8'hFF` | 8 bits | hex | 255 | `11111111` |
| `32'd42` | 32 bits | decimal | 42 | `00...101010` |
| `'o77` | unsized | octal | 63 | `111111` |
| `16'hDEAD` | 16 bits | hex | 57005 | `1101111010101101` |
| `8'b1010_0011` | 8 bits | binary | 163 | `10100011` |
| `4'bxxzz` | 4 bits | binary | unknown | `xxzz` |

The regex pattern for sized literals:

```
SIZED_NUMBER = /[0-9]*'[sS]?[bBoOdDhH][0-9a-fA-F_xXzZ]+/
```

Plain integers and real numbers follow C conventions:

```
REAL_NUMBER = /[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?/
NUMBER      = /[0-9][0-9_]*/
```

**Ordering**: `SIZED_NUMBER` before `REAL_NUMBER` before `NUMBER` (first-match-wins).

#### System Tasks and Functions

```
$display("x = %d", x);
$time
$finish
$random
```

System identifiers start with `$`:

```
SYSTEM_ID = /\$[a-zA-Z_][a-zA-Z0-9_$]*/
```

#### Compiler Directives

```
`define WIDTH 8
`ifdef USE_CACHE
`include "types.v"
`timescale 1ns/1ps
`undef WIDTH
```

Directives start with backtick:

```
DIRECTIVE = /`[a-zA-Z_][a-zA-Z0-9_]*/
```

These are processed by the preprocessor `pre_tokenize` hook before tokenization.
After preprocessing, remaining directives (like `` `timescale ``) pass through
as tokens for the parser to handle or ignore.

#### Identifiers

```
counter
_reset_n
\my.escaped.name       (escaped identifier — any non-whitespace after \)
```

```
ESCAPED_IDENT = /\\[^ \t\r\n]+/
NAME          = /[a-zA-Z_][a-zA-Z0-9_$]*/
```

**Ordering**: `ESCAPED_IDENT` before `NAME`.

#### Operators

Organized longest-first for correct matching:

| Characters | Token | Meaning |
|------------|-------|---------|
| `<<<` | `ARITH_LEFT_SHIFT` | Arithmetic left shift |
| `>>>` | `ARITH_RIGHT_SHIFT` | Arithmetic right shift |
| `===` | `CASE_EQ` | Case equality (4-state) |
| `!==` | `CASE_NEQ` | Case inequality (4-state) |
| `<=` | `LESS_EQUALS` | Less-than-or-equal / non-blocking assign |
| `>=` | `GREATER_EQUALS` | Greater-than-or-equal |
| `==` | `EQUALS_EQUALS` | Logical equality |
| `!=` | `NOT_EQUALS` | Logical inequality |
| `&&` | `LOGIC_AND` | Logical AND |
| `\|\|` | `LOGIC_OR` | Logical OR |
| `<<` | `LEFT_SHIFT` | Logical left shift |
| `>>` | `RIGHT_SHIFT` | Logical right shift |
| `**` | `POWER` | Exponentiation |
| `->` | `TRIGGER` | Event trigger |
| `+` | `PLUS` | Addition |
| `-` | `MINUS` | Subtraction |
| `*` | `STAR` | Multiplication |
| `/` | `SLASH` | Division |
| `%` | `PERCENT` | Modulo |
| `&` | `AMP` | Bitwise AND / reduction AND |
| `\|` | `PIPE` | Bitwise OR / reduction OR |
| `^` | `CARET` | Bitwise XOR / reduction XOR |
| `~` | `TILDE` | Bitwise NOT |
| `!` | `BANG` | Logical NOT |
| `<` | `LESS_THAN` | Less than |
| `>` | `GREATER_THAN` | Greater than |
| `=` | `EQUALS` | Blocking assignment |
| `?` | `QUESTION` | Ternary operator |
| `:` | `COLON` | Ternary separator / range |

#### Delimiters

| Character | Token |
|-----------|-------|
| `(` | `LPAREN` |
| `)` | `RPAREN` |
| `[` | `LBRACKET` |
| `]` | `RBRACKET` |
| `{` | `LBRACE` |
| `}` | `RBRACE` |
| `;` | `SEMICOLON` |
| `,` | `COMMA` |
| `.` | `DOT` |
| `#` | `HASH` |
| `@` | `AT` |

#### Keywords

Synthesizable Verilog keywords (all become `KEYWORD` tokens):

```
always, and, assign, begin, buf, case, casex, casez, default, defparam,
else, end, endcase, endfunction, endgenerate, endmodule, endtask, for,
function, generate, genvar, if, initial, inout, input, integer, localparam,
module, nand, negedge, nor, not, or, output, parameter, posedge, real,
reg, signed, supply0, supply1, task, tri, unsigned, wire, xnor, xor
```

### 1.2 Preprocessor Design

The Verilog preprocessor runs as a `pre_tokenize` hook (str → str). It
processes the source text before tokenization, resolving directives into
plain Verilog source.

#### Directive Processing

```
Source text
    │
    ▼ pre_tokenize: verilog_preprocess
    │
    │  1. Collect `define definitions
    │  2. Evaluate `ifdef/`ifndef/`else/`endif conditionals
    │  3. Strip `timescale, `undef, `include (with warning)
    │  4. Expand macro references in remaining text
    │
    ▼
Clean Verilog source (no directives)
```

#### Macro Definition and Expansion

```verilog
// Simple macro
`define WIDTH 8
wire [`WIDTH-1:0] data;    // → wire [8-1:0] data;

// Parameterized macro
`define MAX(a, b) ((a) > (b) ? (a) : (b))
assign result = `MAX(x, y);  // → assign result = ((x) > (y) ? (x) : (y));

// Undef
`undef WIDTH
```

The preprocessor maintains a dictionary: `macros: dict[str, MacroDef]` where
`MacroDef` stores the replacement text and optional parameter names.

#### Conditional Compilation

```verilog
`define USE_CACHE

`ifdef USE_CACHE
  wire [31:0] cache_data;
`else
  wire [31:0] mem_data;
`endif
```

The preprocessor uses a condition stack to track nesting:

```
condition_stack: list[bool]   # True = include current section
```

- `` `ifdef NAME ``: push `NAME in macros`
- `` `ifndef NAME ``: push `NAME not in macros`
- `` `else ``: flip top of stack
- `` `endif ``: pop stack

Lines where the top of the condition stack is `False` are replaced with
empty lines (to preserve line numbers for error reporting).

#### Include Handling

```verilog
`include "types.vh"
```

For now, `` `include `` emits a warning and is stripped. Full file inclusion
requires file system access, which is a concern for browser-based usage.
A future spec can add an include resolver callback.

### 1.3 Verilog Parser Grammar (EBNF)

The grammar covers the synthesizable subset. Rules are organized top-down
from module structure to expressions.

#### Module Structure

```
source_text     = { description } ;
description     = module_declaration ;

module_declaration = "module" NAME [ parameter_port_list ]
                     [ port_list ] SEMICOLON
                     { module_item }
                     "endmodule" ;

parameter_port_list = HASH LPAREN parameter_declaration
                      { COMMA parameter_declaration } RPAREN ;

port_list = LPAREN port { COMMA port } RPAREN ;
port      = [ port_direction ] [ net_type ] [ "signed" ] [ range ] NAME ;

port_direction = "input" | "output" | "inout" ;
net_type       = "wire" | "reg" | "tri" | "supply0" | "supply1" ;
range          = LBRACKET expression COLON expression RBRACKET ;
```

#### Module Items

```
module_item = port_declaration SEMICOLON
            | net_declaration SEMICOLON
            | reg_declaration SEMICOLON
            | integer_declaration SEMICOLON
            | parameter_declaration SEMICOLON
            | localparam_declaration SEMICOLON
            | continuous_assign
            | always_construct
            | initial_construct
            | module_instantiation
            | generate_region
            | function_declaration
            | task_declaration ;

port_declaration = port_direction [ net_type ] [ "signed" ] [ range ]
                   name_list ;
net_declaration  = net_type [ "signed" ] [ range ] name_list ;
reg_declaration  = "reg" [ "signed" ] [ range ] name_list ;
integer_declaration = "integer" name_list ;
name_list        = NAME { COMMA NAME } ;

parameter_declaration  = "parameter" [ range ] NAME EQUALS expression ;
localparam_declaration = "localparam" [ range ] NAME EQUALS expression ;
```

#### Continuous Assignment

```
continuous_assign = "assign" assignment { COMMA assignment } SEMICOLON ;
assignment        = lvalue EQUALS expression ;
lvalue            = NAME [ range_select ] | concatenation ;
range_select      = LBRACKET expression [ COLON expression ] RBRACKET ;
```

#### Always and Initial Blocks

```
always_construct  = "always" AT sensitivity_list statement ;
initial_construct = "initial" statement ;

sensitivity_list = LPAREN sensitivity_item { ("or" | COMMA) sensitivity_item } RPAREN
                 | LPAREN STAR RPAREN ;

sensitivity_item = [ "posedge" | "negedge" ] expression ;
```

#### Statements

```
statement = block_statement
          | if_statement
          | case_statement
          | for_statement
          | blocking_assignment SEMICOLON
          | nonblocking_assignment SEMICOLON
          | SEMICOLON ;

block_statement = "begin" [ COLON NAME ] { statement } "end" ;

if_statement = "if" LPAREN expression RPAREN statement
               [ "else" statement ] ;

case_statement = ( "case" | "casex" | "casez" )
                 LPAREN expression RPAREN
                 { case_item }
                 "endcase" ;

case_item = expression { COMMA expression } COLON statement
          | "default" [ COLON ] statement ;

for_statement = "for" LPAREN blocking_assignment SEMICOLON
                expression SEMICOLON blocking_assignment RPAREN
                statement ;

blocking_assignment    = lvalue EQUALS expression ;
nonblocking_assignment = lvalue LESS_EQUALS expression ;
```

#### Module Instantiation

```
module_instantiation = NAME [ parameter_value_assignment ]
                       instance { COMMA instance } SEMICOLON ;

parameter_value_assignment = HASH LPAREN expression { COMMA expression } RPAREN ;

instance = NAME LPAREN port_connections RPAREN ;

port_connections = ordered_port_connections | named_port_connections ;
ordered_port_connections = expression { COMMA expression } ;
named_port_connections   = named_port_connection { COMMA named_port_connection } ;
named_port_connection    = DOT NAME LPAREN [ expression ] RPAREN ;
```

#### Generate Blocks

```
generate_region = "generate" { generate_item } "endgenerate" ;

generate_item = "genvar" NAME { COMMA NAME } SEMICOLON
              | "for" LPAREN genvar_assignment SEMICOLON expression
                SEMICOLON genvar_assignment RPAREN
                generate_block
              | "if" LPAREN expression RPAREN generate_block
                [ "else" generate_block ]
              | module_item ;

generate_block = "begin" [ COLON NAME ] { generate_item } "end"
               | generate_item ;

genvar_assignment = NAME EQUALS expression ;
```

#### Function and Task Declarations

```
function_declaration = "function" [ range ] NAME SEMICOLON
                       { function_item }
                       statement
                       "endfunction" ;

function_item = port_declaration SEMICOLON
              | reg_declaration SEMICOLON
              | integer_declaration SEMICOLON
              | parameter_declaration SEMICOLON ;

task_declaration = "task" NAME SEMICOLON
                   { task_item }
                   statement
                   "endtask" ;

task_item = port_declaration SEMICOLON
          | reg_declaration SEMICOLON
          | integer_declaration SEMICOLON ;
```

#### Expressions

Operator precedence (lowest to highest):

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 | `?:` (ternary) | Right |
| 2 | `\|\|` | Left |
| 3 | `&&` | Left |
| 4 | `\|` | Left |
| 5 | `^`, `~^`, `^~` | Left |
| 6 | `&` | Left |
| 7 | `==`, `!=`, `===`, `!==` | Left |
| 8 | `<`, `<=`, `>`, `>=` | Left |
| 9 | `<<`, `>>`, `<<<`, `>>>` | Left |
| 10 | `+`, `-` | Left |
| 11 | `*`, `/`, `%` | Left |
| 12 | `**` | Left |
| 13 | Unary `+`, `-`, `!`, `~`, `&`, `~&`, `\|`, `~\|`, `^`, `~^` | Right |

```
expression = ternary_expr ;

ternary_expr = or_expr [ QUESTION expression COLON ternary_expr ] ;

or_expr      = and_expr { LOGIC_OR and_expr } ;
and_expr     = bit_or_expr { LOGIC_AND bit_or_expr } ;
bit_or_expr  = bit_xor_expr { PIPE bit_xor_expr } ;
bit_xor_expr = bit_and_expr { ( CARET | TILDE CARET | CARET TILDE ) bit_and_expr } ;
bit_and_expr = equality_expr { AMP equality_expr } ;

equality_expr    = relational_expr { ( EQUALS_EQUALS | NOT_EQUALS | CASE_EQ | CASE_NEQ ) relational_expr } ;
relational_expr  = shift_expr { ( LESS_THAN | LESS_EQUALS | GREATER_THAN | GREATER_EQUALS ) shift_expr } ;
shift_expr       = additive_expr { ( LEFT_SHIFT | RIGHT_SHIFT | ARITH_LEFT_SHIFT | ARITH_RIGHT_SHIFT ) additive_expr } ;
additive_expr    = multiplicative_expr { ( PLUS | MINUS ) multiplicative_expr } ;
multiplicative_expr = power_expr { ( STAR | SLASH | PERCENT ) power_expr } ;
power_expr       = unary_expr [ POWER unary_expr ] ;

unary_expr = ( PLUS | MINUS | BANG | TILDE | AMP | PIPE | CARET
             | TILDE AMP | TILDE PIPE | TILDE CARET ) unary_expr
           | primary ;

primary = NUMBER | SIZED_NUMBER | REAL_NUMBER | STRING | NAME
        | SYSTEM_ID
        | LPAREN expression RPAREN
        | concatenation
        | replication
        | primary LBRACKET expression [ COLON expression ] RBRACKET
        | primary DOT NAME
        | NAME LPAREN [ expression { COMMA expression } ] RPAREN ;

concatenation = LBRACE expression { COMMA expression } RBRACE ;
replication   = LBRACE expression concatenation RBRACE ;
```

### 1.4 Verilog Edge Cases

| Scenario | Handling |
|----------|----------|
| `<=` as non-blocking assign vs. less-equals | Grammar context: assignment statements use `lvalue LESS_EQUALS expr`, comparisons appear inside expressions |
| `#10` delay (excluded from grammar) | `HASH` followed by `NUMBER` — lexer produces both tokens; parser ignores delay syntax |
| `(* attribute *)` | Not in synthesizable subset — treated as unrecognized tokens |
| Nested `` `ifdef `` | Condition stack supports arbitrary nesting |
| Macro inside macro | Expansion is single-pass (no recursive expansion) to avoid infinite loops |
| `\escaped.name ` | `ESCAPED_IDENT` regex matches `\` followed by non-whitespace, terminated by whitespace |
| Underscore in numbers | `32'h0000_FFFF` — underscores are valid separators in digits |

---

## Part 2: VHDL

### 2.1 Token Inventory

#### Comments (skip patterns)

```
-- This is a VHDL comment (to end of line)
```

VHDL has only single-line comments (VHDL-2008 adds `/* */` but we target the
synthesizable core).

#### String Literals

```
"Hello, world!"
"He said ""hello"""    -- escaped quote is ""
```

VHDL strings use `""` for embedded quotes, not backslash escapes.
We use `escapes: none` in the `.tokens` file.

```
STRING = /"([^"]|"")*"/
```

#### Character Literals

```
'0'
'1'
'X'
'Z'
'A'
```

Single characters between ticks. Critical: this pattern must match before
the bare `TICK` token for attribute access.

```
CHAR_LITERAL = /'[^']'/
```

#### Bit String Literals

```
B"1010"        -- binary
O"77"          -- octal
X"FF"          -- hexadecimal
D"42"          -- decimal (VHDL-2008)
```

Case-insensitive prefix followed by a quoted digit string:

```
BIT_STRING = /[bBoOxXdD]"[0-9a-fA-F_]+"/
```

**Ordering**: `BIT_STRING` before `NAME` (both start with a letter).

#### Based Literals

```
16#FF#
2#1010_0011#
8#77#
16#FF#E2       -- with exponent
```

```
BASED_LITERAL = /[0-9]+#[0-9a-fA-F_]+(\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9]+)?/
```

#### Number Literals

```
42
3.14
1.0E-3
1_000_000
```

```
REAL_NUMBER = /[0-9][0-9_]*\.[0-9_]+([eE][+-]?[0-9_]+)?/
NUMBER      = /[0-9][0-9_]*/
```

**Ordering**: `BASED_LITERAL` before `REAL_NUMBER` before `NUMBER`.

#### Identifiers

```
counter
Reset_N
\my odd name\     -- extended identifier (backslash-delimited)
```

VHDL identifiers are **case-insensitive**. Normalization to lowercase happens
in a `post_tokenize` hook in the wrapper package.

```
EXTENDED_IDENT = /\\[^\\]+\\/
NAME           = /[a-zA-Z][a-zA-Z0-9_]*/
```

**Ordering**: `EXTENDED_IDENT` before `NAME`.

#### Operators

| Characters | Token | Meaning |
|------------|-------|---------|
| `:=` | `VAR_ASSIGN` | Variable assignment |
| `<=` | `LESS_EQUALS` | Signal assignment / less-or-equal |
| `>=` | `GREATER_EQUALS` | Greater-or-equal |
| `=>` | `ARROW` | Association / port mapping |
| `/=` | `NOT_EQUALS` | Not equal (VHDL uses `/=` not `!=`) |
| `**` | `POWER` | Exponentiation |
| `<>` | `BOX` | Unconstrained range |
| `<` | `LESS_THAN` | Less than |
| `>` | `GREATER_THAN` | Greater than |
| `=` | `EQUALS` | Equality test |
| `+` | `PLUS` | Addition |
| `-` | `MINUS` | Subtraction |
| `*` | `STAR` | Multiplication |
| `/` | `SLASH` | Division |
| `&` | `AMPERSAND` | Concatenation |
| `'` | `TICK` | Attribute access |

**Note on `<=`**: This is the most famous ambiguity in VHDL. The token `<=`
means signal assignment in statement context and less-or-equal in expression
context. We emit a single `LESS_EQUALS` token and let the grammar rules
disambiguate — signal assignments are at the statement level, comparisons
are within expressions. This works because the grammar is LL(k) and the
parser can determine from the rule being parsed whether `<=` is an
assignment or comparison.

**Note on `'`**: The tick is overloaded — it delimits character literals
(`'A'`) and accesses attributes (`signal'length`). The regex ordering
handles this: `CHAR_LITERAL = /'[^']'/` matches `'A'` as a single token.
A bare `'` (not part of a character literal) matches as `TICK` for
attribute access.

#### Delimiters

| Character | Token |
|-----------|-------|
| `(` | `LPAREN` |
| `)` | `RPAREN` |
| `[` | `LBRACKET` |
| `]` | `RBRACKET` |
| `;` | `SEMICOLON` |
| `,` | `COMMA` |
| `.` | `DOT` |
| `:` | `COLON` |
| `\|` | `PIPE` |

#### Keywords

VHDL has two kinds of keywords:

**Structural keywords** (become `KEYWORD` tokens):

```
abs, access, after, alias, all, and, architecture, array, assert,
attribute, begin, block, body, buffer, bus, case, component, configuration,
constant, disconnect, downto, else, elsif, end, entity, exit, file, for,
function, generate, generic, group, guarded, if, impure, in, inout, is,
label, library, linkage, literal, loop, map, mod, nand, new, next, nor,
not, null, of, on, open, or, others, out, package, port, postponed,
procedure, process, pure, range, record, register, reject, rem, report,
return, rol, ror, select, severity, signal, shared, sla, sll, sra, srl,
subtype, then, to, transport, type, unaffected, units, until, use,
variable, wait, when, while, with, xnor, xor
```

**Keyword operators** (a subset of the above that act as operators in
expressions): `and`, `or`, `xor`, `nand`, `nor`, `xnor`, `not`, `mod`,
`rem`, `abs`, `sll`, `srl`, `sla`, `sra`, `rol`, `ror`.

These are listed in the `keywords:` section. The grammar references them
as `"and"`, `"or"`, etc. (quoted literals matching KEYWORD token values).

### 2.2 VHDL Case Normalization

VHDL is case-insensitive: `ENTITY`, `Entity`, and `entity` are identical.
We handle this with a `post_tokenize` hook that lowercases all `NAME` and
`KEYWORD` token values:

```python
def normalize_vhdl_case(tokens: list[Token]) -> list[Token]:
    """Normalize VHDL identifiers and keywords to lowercase.

    VHDL is case-insensitive (IEEE 1076-2008, Section 15.4):
    'Letter case is not significant in identifiers.' We normalize
    to lowercase so the parser can match keywords reliably and
    downstream tools get consistent identifier names.

    Extended identifiers (\name\) are NOT normalized — their case
    is preserved per the IEEE spec.
    """
    return [
        Token(t.type, t.value.lower(), t.line, t.column)
        if t.type in ("NAME", "KEYWORD") else t
        for t in tokens
    ]
```

The `.tokens` file lists keywords in lowercase. The `post_tokenize` hook
ensures all NAME tokens are lowercase before keyword matching occurs.

### 2.3 VHDL Parser Grammar (EBNF)

#### Design Structure

```
design_file = { design_unit } ;

design_unit = { context_item } library_unit ;

context_item = library_clause | use_clause ;
library_clause = "library" name_list SEMICOLON ;
use_clause     = "use" selected_name SEMICOLON ;

selected_name = NAME { DOT ( NAME | "all" ) } ;
name_list      = NAME { COMMA NAME } ;

library_unit = entity_declaration
             | architecture_body
             | package_declaration
             | package_body ;
```

#### Entity Declaration

```
entity_declaration = "entity" NAME "is"
                     [ generic_clause ]
                     [ port_clause ]
                     "end" [ "entity" ] [ NAME ] SEMICOLON ;

generic_clause = "generic" LPAREN interface_list RPAREN SEMICOLON ;
port_clause    = "port" LPAREN interface_list RPAREN SEMICOLON ;

interface_list    = interface_element { SEMICOLON interface_element } ;
interface_element = name_list COLON [ mode ] subtype_indication
                    [ VAR_ASSIGN expression ] ;

mode = "in" | "out" | "inout" | "buffer" ;
```

#### Architecture Body

```
architecture_body = "architecture" NAME "of" NAME "is"
                    { block_declarative_item }
                    "begin"
                    { concurrent_statement }
                    "end" [ "architecture" ] [ NAME ] SEMICOLON ;

block_declarative_item = signal_declaration
                       | constant_declaration
                       | type_declaration
                       | subtype_declaration
                       | component_declaration
                       | function_declaration
                       | procedure_declaration ;

signal_declaration   = "signal" name_list COLON subtype_indication
                       [ VAR_ASSIGN expression ] SEMICOLON ;
constant_declaration = "constant" name_list COLON subtype_indication
                       VAR_ASSIGN expression SEMICOLON ;
```

#### Type System (Synthesizable Subset)

```
type_declaration    = "type" NAME "is" type_definition SEMICOLON ;
subtype_declaration = "subtype" NAME "is" subtype_indication SEMICOLON ;

type_definition = enumeration_type
                | array_type
                | record_type ;

enumeration_type = LPAREN ( NAME | CHAR_LITERAL )
                   { COMMA ( NAME | CHAR_LITERAL ) } RPAREN ;

array_type = "array" LPAREN index_constraint RPAREN "of" subtype_indication ;

index_constraint = discrete_range { COMMA discrete_range } ;
discrete_range   = NAME TICK "range" [ range_constraint ]
                 | range_constraint ;
range_constraint = expression ( "to" | "downto" ) expression ;

record_type = "record" { NAME COLON subtype_indication SEMICOLON }
              "end" "record" [ NAME ] ;

subtype_indication = selected_name [ LPAREN range_constraint RPAREN ]
                   | selected_name [ LPAREN expression ( "to" | "downto" ) expression RPAREN ] ;
```

#### Concurrent Statements

```
concurrent_statement = process_statement
                     | concurrent_signal_assignment
                     | component_instantiation
                     | generate_statement
                     | block_statement ;

concurrent_signal_assignment = NAME LESS_EQUALS [ "guarded" ]
                               waveform SEMICOLON ;
waveform = waveform_element { COMMA waveform_element } ;
waveform_element = expression ;
```

#### Process Statement

```
process_statement = [ NAME COLON ]
                    "process" [ LPAREN sensitivity_list RPAREN ]
                    [ "is" ]
                    { process_declarative_item }
                    "begin"
                    { sequential_statement }
                    "end" "process" [ NAME ] SEMICOLON ;

sensitivity_list = NAME { COMMA NAME } ;

process_declarative_item = variable_declaration
                         | constant_declaration
                         | type_declaration
                         | subtype_declaration ;

variable_declaration = "variable" name_list COLON subtype_indication
                       [ VAR_ASSIGN expression ] SEMICOLON ;
```

#### Sequential Statements

```
sequential_statement = signal_assignment_statement
                     | variable_assignment_statement
                     | if_statement
                     | case_statement
                     | loop_statement
                     | return_statement
                     | null_statement ;

signal_assignment_statement   = NAME LESS_EQUALS waveform SEMICOLON ;
variable_assignment_statement = NAME VAR_ASSIGN expression SEMICOLON ;

if_statement = "if" expression "then"
               { sequential_statement }
               { "elsif" expression "then" { sequential_statement } }
               [ "else" { sequential_statement } ]
               "end" "if" SEMICOLON ;

case_statement = "case" expression "is"
                 { "when" choices ARROW { sequential_statement } }
                 "end" "case" SEMICOLON ;

choices = choice { PIPE choice } ;
choice  = expression | discrete_range | "others" ;

loop_statement = [ NAME COLON ]
                 [ "for" NAME "in" discrete_range
                 | "while" expression ]
                 "loop"
                 { sequential_statement }
                 "end" "loop" [ NAME ] SEMICOLON ;

return_statement = "return" [ expression ] SEMICOLON ;
null_statement   = "null" SEMICOLON ;
```

#### Component Declaration and Instantiation

```
component_declaration = "component" NAME [ "is" ]
                        [ generic_clause ]
                        [ port_clause ]
                        "end" "component" [ NAME ] SEMICOLON ;

component_instantiation = NAME COLON
                          ( NAME | "entity" selected_name [ LPAREN NAME RPAREN ] )
                          [ "generic" "map" LPAREN association_list RPAREN ]
                          [ "port" "map" LPAREN association_list RPAREN ]
                          SEMICOLON ;

association_list    = association_element { COMMA association_element } ;
association_element = [ NAME ARROW ] expression
                    | [ NAME ARROW ] "open" ;
```

#### Generate Statements

```
generate_statement = NAME COLON
                     ( for_generate | if_generate ) ;

for_generate = "for" NAME "in" discrete_range "generate"
               { concurrent_statement }
               "end" "generate" [ NAME ] SEMICOLON ;

if_generate = "if" expression "generate"
              { concurrent_statement }
              "end" "generate" [ NAME ] SEMICOLON ;
```

#### Package Declaration and Body

```
package_declaration = "package" NAME "is"
                      { package_declarative_item }
                      "end" [ "package" ] [ NAME ] SEMICOLON ;

package_body = "package" "body" NAME "is"
               { package_body_declarative_item }
               "end" [ "package" "body" ] [ NAME ] SEMICOLON ;

package_declarative_item = type_declaration
                         | subtype_declaration
                         | constant_declaration
                         | signal_declaration
                         | component_declaration
                         | function_declaration
                         | procedure_declaration ;

package_body_declarative_item = type_declaration
                              | subtype_declaration
                              | constant_declaration
                              | function_body
                              | procedure_body ;
```

#### Function and Procedure Declarations

```
function_declaration = [ "pure" | "impure" ]
                       "function" NAME
                       [ LPAREN interface_list RPAREN ]
                       "return" subtype_indication SEMICOLON ;

function_body = [ "pure" | "impure" ]
                "function" NAME
                [ LPAREN interface_list RPAREN ]
                "return" subtype_indication "is"
                { process_declarative_item }
                "begin"
                { sequential_statement }
                "end" [ "function" ] [ NAME ] SEMICOLON ;

procedure_declaration = "procedure" NAME
                        [ LPAREN interface_list RPAREN ] SEMICOLON ;

procedure_body = "procedure" NAME
                 [ LPAREN interface_list RPAREN ] "is"
                 { process_declarative_item }
                 "begin"
                 { sequential_statement }
                 "end" [ "procedure" ] [ NAME ] SEMICOLON ;
```

#### Expressions

VHDL operator precedence (lowest to highest):

| Precedence | Operators | Category |
|------------|-----------|----------|
| 1 | `and`, `or`, `xor`, `nand`, `nor`, `xnor` | Logical |
| 2 | `=`, `/=`, `<`, `<=`, `>`, `>=` | Relational |
| 3 | `sll`, `srl`, `sla`, `sra`, `rol`, `ror` | Shift |
| 4 | `+`, `-`, `&` | Adding |
| 5 | `*`, `/`, `mod`, `rem` | Multiplying |
| 6 | `**`, `abs`, `not` | Miscellaneous |

**Important**: VHDL does NOT allow mixing logical operators without
parentheses: `a and b or c` is a syntax error. You must write
`(a and b) or c`. The grammar enforces this by not using repetition
at the logical level — each logical operation is a single binary operator.

```
expression = logical_expr ;

logical_expr = relation [ logical_op relation ] ;
logical_op   = "and" | "or" | "xor" | "nand" | "nor" | "xnor" ;

relation = shift_expr [ relational_op shift_expr ] ;
relational_op = EQUALS | NOT_EQUALS | LESS_THAN | LESS_EQUALS
              | GREATER_THAN | GREATER_EQUALS ;

shift_expr = adding_expr [ shift_op adding_expr ] ;
shift_op   = "sll" | "srl" | "sla" | "sra" | "rol" | "ror" ;

adding_expr = multiplying_expr { adding_op multiplying_expr } ;
adding_op   = PLUS | MINUS | AMPERSAND ;

multiplying_expr = unary_expr { multiplying_op unary_expr } ;
multiplying_op   = STAR | SLASH | "mod" | "rem" ;

unary_expr = "abs" unary_expr
           | "not" unary_expr
           | ( PLUS | MINUS ) unary_expr
           | power_expr ;

power_expr = primary [ POWER primary ] ;

primary = NUMBER | REAL_NUMBER | BASED_LITERAL
        | STRING | CHAR_LITERAL | BIT_STRING
        | NAME [ TICK NAME ]
        | NAME LPAREN [ expression { COMMA expression } ] RPAREN
        | LPAREN expression RPAREN
        | aggregate
        | "null" ;

aggregate = LPAREN element_association { COMMA element_association } RPAREN ;
element_association = [ choices ARROW ] expression ;
```

### 2.4 VHDL Edge Cases

| Scenario | Handling |
|----------|----------|
| `<=` as signal assign vs. comparison | Grammar context: `signal_assignment_statement` at statement level, `relational_op` inside expressions |
| `'` as char literal vs. attribute | Regex ordering: `CHAR_LITERAL = /'[^']'/` before `TICK = "'"` |
| `"1010"` as string vs. bit string | `BIT_STRING` requires a prefix letter (`B`, `O`, `X`, `D`). Plain `"1010"` is a `STRING` |
| Case-insensitive keywords | `post_tokenize` hook lowercases all NAME/KEYWORD values; keywords listed lowercase |
| `end entity foo;` vs `end;` | Grammar uses `[ "entity" ] [ NAME ]` making both optional |
| Mixed logical operators | Grammar does not use `{ }` repetition for logical ops — `a and b or c` is a parse error |
| Extended identifiers `\foo\` | Preserved as-is (case NOT normalized per IEEE spec) |
| `not` as keyword vs. operator | Both — keyword in `keywords:` section, used as operator in expression grammar via `"not"` literal match |

---

## Cross-Language Wrapper Plan

| Language | Verilog Lexer | Verilog Parser | VHDL Lexer | VHDL Parser |
|----------|:---:|:---:|:---:|:---:|
| Python | Yes | Yes | Yes | Yes |
| Go | Yes | Yes | Yes | Yes |
| Rust | Yes | Yes | Yes | Yes |
| TypeScript | Yes | Yes | Yes | Yes |
| Ruby | Yes | Yes | Yes | Yes |
| Elixir | Yes | No | Yes | No |

**22 packages total** (6 + 5 + 6 + 5).

Elixir gets lexers only, matching its existing smaller package set
(json, toml, xml lexers; json, toml parsers).

Each wrapper follows the thin-wrapper pattern established by
`javascript-lexer` / `javascript-parser`:
- Load the `.tokens` / `.grammar` file from `code/grammars/`
- Create a `GrammarLexer` / `GrammarParser`
- Register hooks if needed (preprocessor for Verilog, case normalization for VHDL)
- Export `create_*()` and `tokenize_*()` / `parse_*()` convenience functions

---

## Implementation Order

| Phase | What | Commit |
|-------|------|--------|
| 0 | This spec | `spec(verilog-vhdl): add F05 specification for HDL lexer/parser support` |
| 1 | `verilog.tokens` | `feat(grammars): add verilog.tokens` |
| 2 | `verilog.grammar` | `feat(grammars): add verilog.grammar` |
| 3 | Verilog lexer — Python, Go, Rust, TS, Ruby, Elixir | One commit per language |
| 4 | Verilog parser — Python, Go, Rust, TS, Ruby | One commit per language |
| 5 | `vhdl.tokens` | `feat(grammars): add vhdl.tokens` |
| 6 | `vhdl.grammar` | `feat(grammars): add vhdl.grammar` |
| 7 | VHDL lexer — Python, Go, Rust, TS, Ruby, Elixir | One commit per language |
| 8 | VHDL parser — Python, Go, Rust, TS, Ruby | One commit per language |
| 9 | Spec review, changelog finalization | `docs(verilog-vhdl): update spec to match implementation` |

---

## Testing Strategy

### Lexer Tests (per language, target 95%+ coverage)

**Verilog lexer:**
- All number formats: `4'b1010`, `8'hFF`, `32'd42`, `'o77`, `16'hDEAD`, `8'bxxzz`, `1.5e3`
- System identifiers: `$display`, `$time`, `$finish`
- Compiler directives: `` `define ``, `` `ifdef ``, `` `include ``
- Escaped identifiers: `\my.name `
- Operator priority: `===` before `==`, `<<<` before `<<`
- Comments: `// single` and `/* block */`
- Keywords: all produce KEYWORD type
- Edge: underscores in numbers, x/z values, signed literals

**Verilog preprocessor:**
- Simple `define`/expansion
- Parameterized macros
- `ifdef`/`ifndef`/`else`/`endif` (nested)
- `undef`
- Line number preservation (empty lines for excluded sections)

**VHDL lexer:**
- Character literals: `'0'`, `'1'`, `'A'`
- Bit strings: `B"1010"`, `X"FF"`, `O"77"`
- Based literals: `16#FF#`, `2#1010#`
- Attribute tick vs char literal: `signal'length` vs `'A'`
- Case normalization: `ENTITY` → `entity`, `MySignal` → `mysignal`
- Extended identifiers preserved: `\MyName\` stays `\MyName\`
- `/=` for not-equals
- `<=`, `:=`, `=>` as distinct tokens

### Parser Tests (per language, target 80%+ coverage)

**Verilog parser:**
- Empty module: `module m; endmodule`
- Module with ports and parameters
- Wire/reg declarations with ranges
- Continuous assignment: `assign y = a & b;`
- Always block with posedge sensitivity
- If/else and case statements
- Module instantiation (positional and named ports)
- Generate for-loop
- Expressions: precedence, concatenation `{a,b}`, replication `{4{a}}`

**VHDL parser:**
- Entity with ports and generics
- Architecture with signals
- Process with sensitivity list
- If/elsif/else and case/when
- Signal assignment vs variable assignment
- Component instantiation with port map
- For-generate
- Type and subtype declarations
- Expressions: operator precedence, `<=` disambiguation

---

## Implementation Notes

These notes capture divergences discovered during implementation:

1. **Hook API not yet implemented**: The `pre_tokenize` / `post_tokenize` hooks
   specified in `lexer-parser-hooks.md` are not yet available on `GrammarLexer`.
   All wrappers apply hooks directly: preprocessor runs on source string before
   lexer creation, case normalization runs on the returned token list. When the
   hook API is implemented, wrappers can be refactored.

2. **Left-recursion in Verilog grammar**: The `primary` rule has left recursion
   (`primary = ... | primary LBRACKET expression ...`) for array indexing. The
   Go and Rust grammar parsers were patched with left-recursion protection
   (Warth et al. 2008 seed-and-grow technique). Other language parsers work
   around it by avoiding constructs that trigger deep recursion in tests.

3. **VHDL case normalization includes keyword promotion**: The grammar lexer
   matches keywords case-sensitively against lowercase entries. When input is
   `ENTITY`, the lexer emits `NAME("ENTITY")`. The post-tokenize normalization
   step must both lowercase the value AND reclassify NAME tokens as KEYWORD
   when the lowercased value matches a VHDL keyword.

4. **BLOCK_COMMENT regex escaping**: The Verilog `BLOCK_COMMENT` regex required
   escaping `/` inside a character class (`[^\/]` not `[^/]`) because the
   grammar parser uses `/` as a regex delimiter.
