# symbol-core

`symbol-core` introduces interned symbolic names as a shared primitive for the
logic-programming work in this repo and for future symbolic-math packages.

The package gives the rest of the stack a precise type for "this is a symbolic
name" instead of using raw strings everywhere. That matters because atoms,
functor names, algebraic indeterminates, and other symbolic identifiers are not
the same thing as arbitrary text.

## API

```python
from symbol_core import SymbolTable, sym

parent = sym("parent")
parent_again = sym("parent")
sin = sym("sin", namespace="math")

assert parent is parent_again
assert str(sin) == "math:sin"

table = SymbolTable()
user_x = table.intern("x", namespace="user")
assert str(user_x) == "user:x"
```

## Why This Package Exists

- Logic programming needs stable atom and functor names.
- Symbolic math needs first-class symbolic identifiers like `x` and `sin`.
- Interning avoids repeated allocation and makes equality semantics explicit.

## Development

```bash
bash BUILD
```
