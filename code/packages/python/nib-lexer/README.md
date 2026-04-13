# coding-adventures-nib-lexer

A grammar-driven lexer for **Nib** — a safe, statically-typed toy language
designed to compile down to Intel 4004 machine code. Nib is named after
*nibble*, the 4-bit unit that is the native word size of the 4004.

## What Is Nib?

The Intel 4004 was the world's first commercial microprocessor, introduced by
Intel in November 1971. Federico Faggin led its design for the Japanese
calculator company Busicom. It runs at 740 kHz and contains 2,300 transistors
etched onto a chip the size of a fingernail.

Its hardware constraints are extreme:

| Resource | Limit |
|---|---|
| Word size | 4 bits (one nibble) |
| RAM | 160 bytes (shared with call stack) |
| ROM | 4 KB |
| Hardware call stack | 3 levels |
| Multiply instruction | None |
| Divide instruction | None |
| Floating-point | None |

Writing 4004 assembly by hand is error-prone and hard to verify. Nib provides a
safer, higher-level notation. The compiler can statically verify call depth,
catch overflow conditions, and prove loop bounds before generating machine code.

### The Safety Model

Nib enforces a strict subset of operations suited to the 4004:

- **Types fit the hardware**: `u4` (4-bit), `u8` (8-bit), `bcd` (BCD digit),
  `bool`. No 16-bit or 32-bit types — the 4004 cannot hold them in a register.
- **Bounded call depth**: The compiler verifies the static call graph is acyclic
  and no path exceeds depth 2. This prevents silent stack overflow on the 3-level
  hardware stack.
- **No recursion**: A recursive call graph would require a heap-allocated stack.
  Nib bans it at compile time.
- **No heap**: All data is stack-local, static RAM, or ROM constants.
- **Bounded loops**: Loop bounds must be compile-time constants. No unbounded
  `while` loops — `for` with `const` bounds is the only loop construct.

### A Taste of Nib

```nib
// Clamp a nibble to a maximum value using saturating addition.
fn clamp(x: u4, max: u4) -> u4 {
    return x +? max;
}

// Count leading zeros in a nibble.
fn clz(x: u4) -> u4 {
    let count: u4 = 0;
    for i: u4 in 0..4 {
        if x & 0x8 == 0 {
            count = count +% 1;
        }
        x = x +% x;  // shift left by 1
    }
    return count;
}
```

## Token Types

Every source character in a Nib program is classified into one of these tokens.

### Multi-Character Operators

These must appear before their single-character prefixes in the grammar.
The lexer uses *first-match-wins* semantics: the first pattern that matches
at the current position wins.

| Token | Text | Meaning |
|---|---|---|
| `WRAP_ADD` | `+%` | Wrapping addition: `15 +% 1 = 0` on u4 |
| `SAT_ADD` | `+?` | Saturating addition: `15 +? 1 = 15` on u4 |
| `RANGE` | `..` | For-loop range separator: `0..8` |
| `ARROW` | `->` | Return type annotation: `fn f() -> u4` |
| `EQ_EQ` | `==` | Equality comparison |
| `NEQ` | `!=` | Not-equal comparison |
| `LEQ` | `<=` | Less-or-equal comparison |
| `GEQ` | `>=` | Greater-or-equal comparison |
| `LAND` | `&&` | Short-circuit logical AND |
| `LOR` | `\|\|` | Short-circuit logical OR |

### Why `+%` and `+?` Are Separate Tokens

Both start with `+`, but they express fundamentally different arithmetic
operations:

- **`+%` (WRAP_ADD)**: Modular arithmetic. `15 +% 1 = 0`. The result wraps
  around at the type boundary, discarding the carry. Mirrors Rust's
  `wrapping_add()` and Zig's `+%` operator. The `%` sigil signals "modular
  wrap". Used when you *want* wrap-around behavior, such as a nibble counter
  cycling through 0–15 repeatedly.

- **`+?` (SAT_ADD)**: Saturating arithmetic. `15 +? 1 = 15`. The result clamps
  at the maximum value. The `?` sigil asks "did we overflow?" and saturates at
  the limit. Mirrors ARM's `UQADD` instruction and Rust's `saturating_add()`.
  Used when you want to stay at the maximum rather than wrap — for example,
  accumulating BCD digits where 9 + 1 should stay 9.

Making these separate tokens forces the programmer to choose explicitly between
wrapping and saturating semantics. A plain `+` (PLUS) is also available but may
trigger a compile-time overflow error if the compiler cannot prove the result
fits.

### Why HEX_LIT Must Come Before INT_LIT

The grammar lists `HEX_LIT` before `INT_LIT`. Consider the input `0xFF`:

- If `INT_LIT` (`/[0-9]+/`) fired first, it would match the leading `0` as a
  decimal integer literal, then the `x` would fail to start a valid token.
- With `HEX_LIT` (`/0x[0-9A-Fa-f]+/`) listed first, the entire `0xFF` is
  consumed as one hex literal.

Hex literals are essential for 4004 programming. Nibble masks, port addresses,
ROM addresses, and hardware register values are all naturally expressed in hex:

```nib
let mask: u4   = 0xF;   // nibble mask — isolate lower 4 bits
let port: u4   = 0xA;   // port address for SRC instruction
const ROM_BASE = 0x000; // ROM base address
```

### Why `//` Comments

Nib uses C++/Java/Rust-style `//` line comments. The `//` prefix was chosen
over `#` (Python/Ruby) because 4004 assembly conventionally uses `;` for
comments — keeping `//` as the Nib comment marker avoids confusion when reading
Nib source alongside generated assembly output.

### Single-Character Arithmetic Operators

| Token | Text | Notes |
|---|---|---|
| `PLUS` | `+` | Ordinary addition (may trigger overflow error) |
| `MINUS` | `-` | Subtraction |
| `STAR` | `*` | Reserved — 4004 has no multiply instruction (v2) |
| `SLASH` | `/` | Reserved — 4004 has no divide instruction (v2) |

### Single-Character Bitwise Operators

| Token | Text | 4004 Instruction |
|---|---|---|
| `AMP` | `&` | ANL (AND logical) |
| `PIPE` | `\|` | ORL (OR logical) |
| `CARET` | `^` | XRL (XOR logical) |
| `TILDE` | `~` | CMA (complement accumulator) |

### Comparison and Logical Operators

| Token | Text | Meaning |
|---|---|---|
| `BANG` | `!` | Logical NOT (boolean negation) |
| `LT` | `<` | Less-than |
| `GT` | `>` | Greater-than |
| `EQ` | `=` | Assignment (declarations only) |

### Delimiters

| Token | Text | Use |
|---|---|---|
| `LBRACE` | `{` | Begin block |
| `RBRACE` | `}` | End block |
| `LPAREN` | `(` | Begin parameter list or grouped expression |
| `RPAREN` | `)` | End parameter list or grouped expression |
| `COLON` | `:` | Type annotation separator: `x: u4` |
| `SEMICOLON` | `;` | Statement terminator |
| `COMMA` | `,` | Argument/parameter separator |

### Literals

| Token | Pattern | Examples |
|---|---|---|
| `HEX_LIT` | `/0x[0-9A-Fa-f]+/` | `0x0`, `0xA`, `0xFF`, `0x1F` |
| `INT_LIT` | `/[0-9]+/` | `0`, `1`, `15`, `42`, `255` |

### Identifiers

| Token | Pattern | Examples |
|---|---|---|
| `NAME` | `/[a-zA-Z_][a-zA-Z0-9_]*/` | `counter`, `my_var`, `_hidden`, `u4` |

**Important**: `u4`, `u8`, `bcd`, and `bool` are **not keywords** — they lex
as `NAME` tokens. The parser promotes them to type productions in
type-annotation context. This keeps the keyword set minimal.

### Keywords

Keywords are reclassified from `NAME` after a full-token match. Nib keywords
are **case-sensitive** and **lowercase only** (unlike ALGOL 60).

| Token | Text | Meaning |
|---|---|---|
| `fn` | `fn` | Function declaration |
| `let` | `let` | Local variable declaration |
| `static` | `static` | Static RAM variable |
| `const` | `const` | Compile-time constant |
| `return` | `return` | Return from function |
| `for` | `for` | For loop |
| `in` | `in` | Range separator in for loop |
| `if` | `if` | Conditional |
| `else` | `else` | Else branch |
| `true` | `true` | Boolean true literal |
| `false` | `false` | Boolean false literal |

### Skipped Tokens

| Pattern | Meaning |
|---|---|
| `/[ \t\r\n]+/` | Whitespace — ignored |
| `/\/\/[^\n]*/` | Line comment `//` — ignored |

## How It Fits the Stack

```
nib.tokens (grammar definition in code/grammars/)
      │
      ▼
grammar_tools.parse_token_grammar()
      │
      ▼
GrammarLexer  ←── nib-lexer (this package, thin wrapper)
      │
      ▼
list[Token]   ──► nib-parser (PR 4)
                  ──► nib-type-checker (PR 5)
                  ──► nib-ir-compiler (PR 6)
                  ──► nib-codegen-4004 (PR 7)
```

This is PR 3 of 10 in the Nib compiler pipeline. The real work is done by:

- **`coding-adventures-grammar-tools`**: Parses `nib.tokens` into a
  `TokenGrammar` data structure describing each token pattern.
- **`coding-adventures-lexer`**: The `GrammarLexer` engine that runs the
  token grammar against source text using a state machine.
- **`coding-adventures-state-machine`**: The underlying state machine powering
  the lexer's pattern matching.
- **`coding-adventures-directed-graph`**: Used internally by the state machine
  for NFA/DFA construction.

## Usage

```python
from nib_lexer import tokenize_nib, create_nib_lexer

# All-in-one: tokenize and get a list
tokens = tokenize_nib('let x: u4 = 0xF;')
for token in tokens:
    print(token)

# Factory: create a GrammarLexer for more control
lexer = create_nib_lexer('fn add(a: u4, b: u4) -> u4 { return a +% b; }')
tokens = lexer.tokenize()
```

### Example Output

```
Token(type='let', value='let')
Token(type='NAME', value='x')
Token(type='COLON', value=':')
Token(type='NAME', value='u4')
Token(type='EQ', value='=')
Token(type='HEX_LIT', value='0xF')
Token(type='SEMICOLON', value=';')
Token(type='EOF', value='')
```

## Installation

```bash
pip install coding-adventures-nib-lexer
```

Or from source in the monorepo (the dependencies are installed as local
`file:` packages by the build tool):

```bash
cd code/packages/python/nib-lexer
pip install -e ".[dev]"
```

## Running Tests

```bash
pytest
```
