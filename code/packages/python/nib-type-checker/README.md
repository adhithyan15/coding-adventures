# nib-type-checker

Stage 3 of the Nib compiler pipeline. Takes the untyped `ASTNode` tree
produced by `nib-parser` and verifies that the program is type-safe,
returning an annotated AST and a list of type errors.

## What Is Type Checking?

A compiler typically processes source code through several stages:

```
Source text
    → Lexer            (characters → tokens)
    → Parser           (tokens → AST)
    → Type Checker     (untyped AST → typed AST)   ← this package
    → IR Compiler      (typed AST → IR)
    → Backend Validator (IR → validated IR)
    → Code Generator   (validated IR → machine code)
```

The type checker's job is to:

1. **Verify** that every expression is type-safe — no adding a `bool` to a
   `u4`, no calling a function with the wrong argument types.
2. **Annotate** the AST — attach a `._nib_type` attribute to every expression
   node so later stages don't need to re-infer types.
3. **Collect all errors** in a single pass — so the programmer sees all
   mistakes at once instead of fixing one error to reveal the next.

## The Four Nib Types

Nib is designed for the Intel 4004 microprocessor (1971), which has extreme
hardware constraints: 4-bit registers, 8-bit register pairs, 160 bytes of
RAM, and no floating-point. The type system matches this hardware exactly.

| Type   | Bits | Range  | Hardware          | Description                          |
|--------|------|--------|-------------------|--------------------------------------|
| `u4`   | 4    | 0–15   | One register      | 4-bit unsigned nibble                |
| `u8`   | 8    | 0–255  | Register pair     | 8-bit unsigned byte                  |
| `bcd`  | 4    | 0–9    | One register+DAA  | Binary-Coded Decimal digit           |
| `bool` | 4    | 0 or 1 | One register      | Boolean (true/false)                 |

There is **no implicit widening**: you cannot assign a `u4` to a `u8`
variable without an explicit cast. This prevents the subtle bugs that arise
when a narrow value silently widens into a different numeric range.

## Language Invariants Enforced

This checker enforces *language-level* rules — rules that apply regardless
of what hardware you're targeting:

### 1. All names declared before use

```nib
fn main() {
    x = 5;  // ERROR: 'x' is not defined
}
```

### 2. Expression types correct bottom-up

```nib
fn main() {
    let b: bool = 1 +% 2;  // ERROR: '+%' returns u4, not bool
}
```

### 3. Assignment LHS type == RHS type (no implicit widening)

```nib
fn main() {
    let x: u4 = 0;
    let y: u8 = 0;
    x = y;  // ERROR: cannot assign u8 to u4 variable
}
```

### 4. Function call argument types must match

```nib
fn f(x: u4) { }
fn main() {
    f(true);  // ERROR: argument 1 expected u4 but got bool
}
```

### 5. BCD operator restriction

The Intel 4004 has a special `DAA` (Decimal Adjust Accumulator) instruction
for BCD addition. Other arithmetic operations don't have BCD equivalents.
So `bcd` operands only permit `+%` (wrapping add, which the compiler emits
as ADD + DAA) and `-` (subtraction via ten's complement):

```nib
fn main() {
    let d: bcd = 3 + 4;   // ERROR: BCD only allows +% and -
    let e: bcd = 3 +% 4;  // OK
    let f: bcd = 9 - 3;   // OK
}
```

This restriction lives in the *type checker* (not the code generator) because
it is a *language-level rule* — the programmer must explicitly opt into
BCD-aware arithmetic. Making it visible in the source and checkable without
any target knowledge is the right design.

### 6. For-loop bounds must be numeric

The Nib type checker verifies that `for i: T in start..end` uses numeric
start/end expressions. Whether a particular backend can lower those bounds
efficiently is a later-stage concern:

```nib
fn main() {
    let n: u8 = 10;
    for i: u8 in 0..n { }        // OK — numeric runtime bound

    let done: bool = false;
    for i: u8 in 0..done { }     // ERROR — bool is not numeric
}
```

### 7. If/for conditions must be bool

Unlike C (where any non-zero integer is "truthy"), Nib requires explicit
boolean conditions. This prevents the classic bug of `if (x = 0)` vs
`if (x == 0)`:

```nib
fn main() {
    let x: u4 = 5;
    if x { }          // ERROR: condition must be bool, got u4
    if x == 5 { }     // OK — comparison produces bool
}
```

### 8. Return type must match declaration

```nib
fn f() -> u4 {
    return true;  // ERROR: function declares -> u4 but returns bool
}
```

## What Is NOT Checked Here

Hardware constraints belong in the backend validator, not the type checker:

- **Call depth ≤ 2** — Intel 4004 hardware limit (3-level stack, one in use)
- **Recursive call graphs** — target limitation enforced before 4004 assembly
- **Total static RAM ≤ 160 bytes** — Intel 4004 hardware limit
- **Physical register count** — CPU architecture detail

Keeping hardware constraints out of the type checker makes the design
composable: the same `NibTypeChecker` can target the Intel 4004, an ARM
Cortex-M0, a WASM module, or any future ISA without any modification.

## Usage

```python
from nib_parser import parse_nib
from nib_type_checker import check

source = """
    const MAX: u8 = 100;

    fn add(a: u4, b: u4) -> u4 {
        return a +% b;
    }

    fn main() {
        let result: u4 = add(3, 4);
        for i: u8 in 0..MAX {
            let x: u4 = i +% result;
        }
    }
"""

ast = parse_nib(source)
result = check(ast)

if result.ok:
    print("Type check passed!")
    # result.typed_ast has ._nib_type annotations on every expression node
else:
    for err in result.errors:
        print(f"Line {err.line}, Col {err.column}: {err.message}")
```

## Architecture: Two-Pass Checking

The checker uses a two-pass approach over the AST:

**Pass 1 — Signature Collection**

Walk the top-level declarations and collect:
- `const` and `static` names with their types into the global scope.
- Function signatures (name, parameter types, return type) into the global scope.
- A call graph: `{fn_name → set of functions it calls}`.

This means functions can be called in any order — you don't have to declare
a function before calling it (unlike C without header files).

**Pass 2 — Body Type-Checking**

Walk each function body with the complete global scope available. Every
statement and expression is recursively checked. Errors are collected
without stopping — all errors are reported in one pass.

## Scope Chain

Lexical scoping is modelled as a stack of dictionaries:

```
Global scope:  { "MAX": Symbol(u8, is_const=True),
                 "add": Symbol(fn, params=[u4,u4], ret=u4),
                 "main": Symbol(fn) }
    ↓ push on function entry
Function scope: { "a": Symbol(u4), "b": Symbol(u4) }
    ↓ push on block entry
Block scope:    { "result": Symbol(u4) }
```

Name lookup searches from the innermost scope outward. A name not found in
any scope is an "undeclared variable" error.

## Cycle Detection

No-recursion enforcement uses iterative DFS with three-colour marking
(the standard textbook algorithm):

- **WHITE**: not yet visited
- **GREY**: currently on the DFS stack (potential back-edge target)
- **BLACK**: fully explored

A back-edge (reaching a GREY node) means a cycle exists. The algorithm
runs in O(V + E) time where V is the number of functions and E is the
number of call-site relationships.

## In the Pipeline

```
PR 1: nib-lexer           — tokenizes Nib source text
PR 2: nib-parser          — parses tokens into an ASTNode tree
PR 3: type-checker-protocol — defines TypeChecker[In, Out] protocol
PR 4: nib-grammar-spec    — formal grammar specification
PR 5: nib-type-checker    — ← THIS PACKAGE
PR 6: nib-ir-compiler     — compiles typed AST to architecture-independent IR
PR 7: intel-4004-ir-validator — validates IR for Intel 4004 hardware limits
PR 8: ir-to-intel-4004-compiler — generates Intel 4004 assembly text
```

## Development

```bash
# Install in editable mode with dev dependencies
uv pip install -e ".[dev]" \
    -e ../type-checker-protocol \
    -e ../parser \
    -e ../lexer \
    -e ../grammar-tools \
    -e ../state-machine \
    -e ../directed-graph \
    -e ../nib-lexer \
    -e ../nib-parser

# Run tests with coverage
pytest

# Lint
ruff check src/ tests/
```
