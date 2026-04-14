# coding-adventures-nib-parser

A grammar-driven parser for **Nib** — a safe, statically-typed toy language
that compiles to Intel 4004 machine code. Nib is named for "nibble" (4 bits),
the native word size of the Intel 4004, the world's first commercial
microprocessor (1971).

## What Is Nib?

The Intel 4004 is extraordinarily constrained by modern standards:

- **4-bit words**: The accumulator holds one nibble (0–15). All arithmetic
  is on 4-bit or 8-bit (register-pair) values.
- **160 bytes of usable RAM**: Not kilobytes — 160 bytes. Every ``static``
  variable in a Nib program must fit in this budget.
- **4 KB of ROM**: The compiled program must fit in 4,096 bytes.
- **3-level hardware call stack**: The CPU has exactly three PC registers.
  Nib statically enforces a maximum call depth of 2.
- **No multiply, no divide, no floating point**: The 4004 instruction set
  has ADD, SUB, AND, OR, XOR — that's it for arithmetic.

Writing 4004 assembly by hand is tedious and error-prone. Nib provides a
higher-level notation with static safety guarantees, while compiling to
efficient 4004 instruction sequences.

Nib's key safety features:

- **Typed nibbles**: ``u4``, ``u8``, ``bcd``, ``bool`` — types match the
  hardware word sizes.
- **Explicit overflow**: ``+%`` (wrapping add) and ``+?`` (saturating add)
  force the programmer to state their overflow intention. No silent overflow.
- **Static call depth**: The compiler verifies at build time that no call
  chain exceeds depth 2 (matching the 4004's 3-level stack).
- **No recursion**: The call graph must be acyclic.
- **Const loop bounds**: For-loop bounds must be compile-time constants so
  the compiler can generate DJNZ (Decrement and Jump if Not Zero) patterns.
- **No heap**: No dynamic allocation — all data is static or stack.

## How It Fits the Stack

```
nib.grammar (grammar rules)         nib.tokens (token defs)
      │                                    │
      ▼                                    ▼
grammar_tools.parse_parser_grammar()  nib-lexer
      │                                    │ list[Token]
      ▼                                    ▼
GrammarParser ←── nib-parser (this package, thin wrapper)
      │
      ▼
ASTNode (generic parse tree)
```

This package depends on:
- **`coding-adventures-nib-lexer`**: Tokenizes Nib source into a ``list[Token]``.
- **`coding-adventures-grammar-tools`**: Parses ``nib.grammar`` into a
  ``ParserGrammar`` data structure.
- **`coding-adventures-lang-parser`**: The ``GrammarParser`` engine that runs
  the grammar against the token stream and produces an ``ASTNode`` tree.

## Usage

```python
from nib_parser import parse_nib, create_nib_parser

# Simple one-shot parse
ast = parse_nib("fn main() { let x: u4 = 5; }")
print(ast.rule_name)  # "program"

# Factory function (inspect the parser before running)
parser = create_nib_parser("fn add(a: u4, b: u4) -> u4 { return a; }")
ast = parser.parse()
```

## AST Structure

Parsing ``fn main() { let x: u4 = 5; }`` produces:

```
ASTNode(rule_name="program")
└── ASTNode(rule_name="top_decl")
    └── ASTNode(rule_name="fn_decl")
        ├── Token(FN, 'fn')
        ├── Token(NAME, 'main')
        ├── Token(LPAREN, '(')
        ├── Token(RPAREN, ')')
        └── ASTNode(rule_name="block")
            ├── Token(LBRACE, '{')
            ├── ASTNode(rule_name="stmt")
            │   └── ASTNode(rule_name="let_stmt")
            │       ├── Token(LET, 'let')
            │       ├── Token(NAME, 'x')
            │       ├── Token(COLON, ':')
            │       ├── ASTNode(rule_name="type")
            │       │   └── Token(NAME, 'u4')
            │       ├── Token(EQ, '=')
            │       ├── ASTNode(rule_name="expr")
            │       │   └── ...
            │       └── Token(SEMICOLON, ';')
            └── Token(RBRACE, '}')
```

## Grammar Rules

The parser covers the complete Nib v1 grammar:

| Category | Rules |
|----------|-------|
| Top level | ``program``, ``top_decl``, ``const_decl``, ``static_decl``, ``fn_decl`` |
| Parameters | ``param_list``, ``param`` |
| Statements | ``block``, ``stmt``, ``let_stmt``, ``assign_stmt``, ``return_stmt``, ``for_stmt``, ``if_stmt``, ``expr_stmt`` |
| Types | ``type`` (``u4``, ``u8``, ``bcd``, ``bool``) |
| Expressions | ``expr``, ``or_expr``, ``and_expr``, ``eq_expr``, ``cmp_expr``, ``add_expr``, ``bitwise_expr``, ``unary_expr``, ``primary`` |
| Calls | ``call_expr``, ``arg_list`` |

### Expression Precedence (lowest → highest)

| Level | Rule | Operators |
|-------|------|-----------|
| 1 | ``or_expr`` | ``\|\|`` |
| 2 | ``and_expr`` | ``&&`` |
| 3 | ``eq_expr`` | ``==``, ``!=`` |
| 4 | ``cmp_expr`` | ``<``, ``>``, ``<=``, ``>=`` |
| 5 | ``add_expr`` | ``+``, ``-``, ``+%``, ``+?`` |
| 6 | ``bitwise_expr`` | ``&``, ``\|``, ``^`` |
| 7 | ``unary_expr`` | ``!``, ``~`` (prefix) |
| 8 | ``primary`` | literals, names, calls, ``(expr)`` |

The bitwise-above-additive ordering follows Java/Rust (not C). This avoids
the classic C bug where ``x & MASK == 0`` parses as ``x & (MASK == 0)``
instead of ``(x & MASK) == 0``. In Nib, bitwise binds tighter than additive,
so you need explicit parentheses only for the unusual case of ``(a + b) & c``.

## Complete Program Example

```nib
// Count from 0 to MAX using wrapping arithmetic, then check
const MAX: u8 = 10;
static counter: u8 = 0;
static done: bool = false;

fn inc() {
    counter = counter +% 1;
}

fn main() {
    for i: u8 in 0..MAX {
        inc();
    }
    done = counter == MAX;
}
```

Parsing this produces:

- One ``const_decl`` node for ``MAX``
- Two ``static_decl`` nodes for ``counter`` and ``done``
- Two ``fn_decl`` nodes for ``inc`` and ``main``
- Inside ``main``: a ``for_stmt`` node containing an ``expr_stmt`` (the
  call to ``inc()``)

## Development

```bash
pip install -e ".[dev]"
pytest
ruff check src/
```
