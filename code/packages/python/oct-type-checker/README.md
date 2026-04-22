# coding-adventures-oct-type-checker

Type-checks Oct ASTs — enforces language-level invariants and annotates expression nodes with resolved OctTypes.

## What Is This?

This package is the **third stage** of the Oct compiler pipeline:

```
Source text
    → oct-lexer       (characters → tokens)
    → oct-parser      (tokens → untyped ASTNode tree)
    → oct-type-checker (untyped AST → typed AST)   ← this package
    → oct-ir-compiler  (typed AST → IR)
    → …
```

Oct is a safe, statically-typed toy language designed to compile to Intel 8008
machine code. The name comes from *octet* — the networking term for exactly 8
bits, the native word size of the Intel 8008 ALU (1972).

## Oct's Type System

Oct has exactly **two value types**:

| Type   | Description                          | Range    |
|--------|--------------------------------------|----------|
| `u8`   | Unsigned 8-bit integer               | 0–255    |
| `bool` | Boolean (true = 1, false = 0)        | true/false|

**Compatibility rule**: `bool` may be used wherever `u8` is expected (implicit
coercion), but a `u8` may NOT be used where `bool` is expected without an
explicit comparison. This prevents the classic C bug `if (x)` when you meant
`if (x != 0)`.

```oct
let x: u8 = true;       // OK — bool coerces to u8
let y: bool = carry();  // OK — carry() returns bool
let z: bool = x;        // ERROR — u8 cannot coerce to bool
```

## Language Invariants Enforced

The checker enforces *language-level* invariants only — hardware constraints
(max 4 locals, max 7 call depth, port ranges) are backend concerns:

1. **All names declared before use** — variables must be declared with `let`
   or `static` before appearing in expressions.
2. **Expression types correct bottom-up** — arithmetic, bitwise, logical, and
   comparison operators all have enforced operand types.
3. **Assignment compatibility** — RHS type must be compatible with LHS type.
4. **Function call argument types** — argument count and types must match.
5. **Intrinsic argument types** — hardware intrinsics have fixed signatures.
6. **Port arguments are compile-time literals** — `in(PORT)` and `out(PORT,
   val)` require literal port numbers since the Intel 8008 encodes them in
   the instruction opcode.
7. **`if`/`while` conditions must be `bool`** — not just truthy.
8. **Return statements match declared return type**.
9. **Integer literals in range 0–255**.
10. **`main` function exists** with no parameters and no return type.

## Usage

```python
from oct_parser import parse_oct
from oct_type_checker import check_oct

# Parse source text into an untyped AST
ast = parse_oct("""
    static THRESHOLD: u8 = 128;

    fn process(val: u8) -> bool {
        return val > THRESHOLD;
    }

    fn main() {
        let data: u8 = in(0);
        let high: bool = process(data);
        if high {
            out(1, data);
        }
    }
""")

# Type-check the AST
result = check_oct(ast)

if result.ok:
    # result.typed_ast is the same ASTNode with ._oct_type set on each expression
    print("Type checking passed!")
else:
    for err in result.errors:
        print(f"  {err.line}:{err.column}: {err.message}")
```

### Using the class directly

```python
from oct_type_checker import OctTypeChecker

checker = OctTypeChecker()
result = checker.check(ast)
```

### Accessing type annotations

After a successful type check, every expression node in the AST has an
`._oct_type` attribute set to `"u8"` or `"bool"`:

```python
result = check_oct(ast)
if result.ok:
    # Walk the AST and read ._oct_type on expression nodes
    for node in walk(result.typed_ast):
        if hasattr(node, "_oct_type"):
            print(f"  {node.rule_name}: {node._oct_type}")
```

## Intrinsic Signatures

Oct exposes the Intel 8008's special instructions as built-in functions:

| Intrinsic     | Return Type | Description                              |
|---------------|-------------|------------------------------------------|
| `in(PORT)`    | `u8`        | Read from I/O port (PORT = literal)      |
| `out(PORT, v)`| void        | Write to I/O port (PORT = literal)       |
| `adc(a, b)`   | `u8`        | Add with carry                           |
| `sbb(a, b)`   | `u8`        | Subtract with borrow                     |
| `rlc(a)`      | `u8`        | Rotate left through carry                |
| `rrc(a)`      | `u8`        | Rotate right through carry               |
| `ral(a)`      | `u8`        | Rotate accumulator left                  |
| `rar(a)`      | `u8`        | Rotate accumulator right                 |
| `carry()`     | `bool`      | Read the carry flag                      |
| `parity(a)`   | `bool`      | Read the parity flag for a value         |

## Operator Types

| Operators           | Operands           | Result |
|---------------------|--------------------|--------|
| `+`, `-`            | `u8`-compatible    | `u8`   |
| `&`, `\|`, `^`      | `u8`-compatible    | `u8`   |
| `~` (unary NOT)     | `u8`-compatible    | `u8`   |
| `==`, `!=`, `<`, `>`, `<=`, `>=` | `u8`-compatible | `bool` |
| `&&`, `\|\|`        | `bool`             | `bool` |
| `!` (logical NOT)   | `bool`             | `bool` |

## Project Structure

```
oct-type-checker/
├── src/
│   └── oct_type_checker/
│       ├── __init__.py      # Public API: OctTypeChecker, check_oct
│       └── checker.py       # Full implementation (~700 lines, literate)
├── tests/
│   └── test_oct_type_checker.py   # ~250 test cases
├── BUILD                    # Linux/macOS build script
├── BUILD_windows            # Windows build script
└── pyproject.toml
```

## How It Fits in the Stack

```
Layer 0: graph, directed-graph        — graph primitives
Layer 1: grammar-tools, lexer, parser — generic parsing infrastructure
Layer 2: state-machine                — automata
Layer 3: oct-lexer                    — Oct tokenizer
Layer 4: oct-parser                   — Oct grammar → ASTNode
Layer 5: type-checker-protocol        — TypeChecker protocol, GenericTypeChecker
Layer 6: oct-type-checker  ← HERE     — Oct type checker
Layer 7: oct-ir-compiler (next)       — typed AST → IR
```

## Building and Testing

```bash
./BUILD          # on Linux/macOS
BUILD_windows    # on Windows
```

This creates a `.venv`, installs all dependencies from local source, and runs
`pytest` with coverage.
