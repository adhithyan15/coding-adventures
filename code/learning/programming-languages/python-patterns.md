# Python Patterns Used in This Project

This document explains the Python language features and patterns used
throughout the coding-adventures packages. Every pattern is drawn from
actual code in the repo.

## Dataclasses — Structured Data Without Boilerplate

Python's `@dataclass` decorator generates `__init__`, `__repr__`, `__eq__`,
and other dunder methods automatically. We use dataclasses extensively for
value objects like ALU results, cache configurations, and tokens.

```python
from dataclasses import dataclass

@dataclass
class ALUResult:
    """The output of an ALU operation."""
    value: list[int]   # result bits (LSB first)
    zero: bool         # is result all zeros?
    carry: bool        # unsigned overflow?
    negative: bool     # MSB is 1?
    overflow: bool     # signed overflow?
```

Without `@dataclass`, you'd write 20+ lines of boilerplate for the same thing.

### Frozen dataclasses for immutability

Adding `frozen=True` makes instances immutable — any attempt to modify a
field raises `FrozenInstanceError`. This prevents accidental mutations:

```python
@dataclass(frozen=True)
class Token:
    type: str
    value: str
    line: int
    column: int
```

**Where used:** `code/packages/python/lexer/`, `code/packages/python/parser/`

## Enums — Named Constants

Python's `Enum` gives named, type-safe constants instead of magic strings:

```python
from enum import Enum

class ALUOp(Enum):
    ADD = "add"
    SUB = "sub"
    AND = "and"
    OR  = "or"
    XOR = "xor"
    NOT = "not"
```

You can switch on enum values, iterate over them, and compare them safely.
Typos in string values become `AttributeError` instead of silent bugs.

**Where used:** `code/packages/python/arithmetic/`, `code/packages/python/bytecode-compiler/`

## Type Hints — Documentation That the Linter Enforces

Every function in this project has type annotations. These serve as
documentation and are checked by ruff (ANN rules) and mypy:

```python
def ripple_carry_adder(
    a: list[int],
    b: list[int],
    carry_in: int = 0,
) -> tuple[list[int], int]:
    """Add two N-bit numbers using chained full adders."""
    ...
```

### The `list[int]` vs `List[int]` distinction

Python 3.9+ allows lowercase `list`, `dict`, `tuple` in type hints.
Older code uses `from typing import List`. We use the modern lowercase form
because we require Python 3.12+.

### Union types with `|`

```python
def _get_build_file(directory: Path) -> Path | None:
    ...
```

`Path | None` is the modern syntax (Python 3.10+) for `Optional[Path]`.

**Where used:** Every package

## Protocol Classes — Structural Typing

Python's `Protocol` class enables duck typing with type checking. Instead
of requiring a specific base class, you define the interface and any class
that implements those methods is accepted:

```python
from typing import Protocol

class BranchPredictor(Protocol):
    def predict(self, address: int) -> bool: ...
    def update(self, address: int, taken: bool) -> None: ...
```

Any class with `predict` and `update` methods satisfies this protocol,
even without inheriting from it. This is structural typing (like Go
interfaces) rather than nominal typing (like Java interfaces).

**Where used:** `code/packages/python/branch-predictor/`

## The `__future__` Import

Every Python file starts with:

```python
from __future__ import annotations
```

This makes all type annotations lazy (evaluated as strings, not at import
time). Benefits:
- Forward references work without quotes
- Slightly faster import times
- Required for some `dataclass` patterns

## The `src` Layout

All Python packages use the "src layout":

```
my-package/
├── pyproject.toml
├── src/
│   └── my_package/
│       ├── __init__.py
│       └── module.py
└── tests/
    └── test_module.py
```

Why not a flat layout? The src layout prevents accidentally importing the
local source directory instead of the installed package during testing.
It's the recommended layout for publishable packages.

**Where used:** Every Python package

## pytest Patterns

### Parametrize — Run the Same Test with Different Inputs

```python
@pytest.mark.parametrize("a, b, expected_sum, expected_carry", [
    (0, 0, 0, 0),
    (0, 1, 1, 0),
    (1, 0, 1, 0),
    (1, 1, 0, 1),
])
def test_half_adder(a, b, expected_sum, expected_carry):
    result = half_adder(a, b)
    assert result == (expected_sum, expected_carry)
```

This generates 4 test cases from one function — exhaustive truth table
testing in 8 lines.

### Fixtures with `tmp_path`

pytest provides a `tmp_path` fixture that creates a temporary directory
for each test:

```python
def test_cache_saves_and_loads(tmp_path):
    cache_file = tmp_path / "cache.json"
    cache = BuildCache()
    cache.update("pkg-a", "abc123", "def456")
    cache.save(cache_file)

    loaded = BuildCache.load(cache_file)
    assert loaded.get("pkg-a") is not None
```

**Where used:** `code/packages/python/directed-graph/`, `code/programs/python/build-tool/`

## `pathlib.Path` — Object-Oriented File Paths

We use `Path` objects everywhere instead of string paths:

```python
from pathlib import Path

directory = Path("/repo/code/packages/python")
build_file = directory / "BUILD"       # joins with /
if build_file.exists():
    content = build_file.read_text()
```

The `/` operator joins paths. Methods like `.exists()`, `.read_text()`,
`.iterdir()`, `.mkdir(parents=True)` replace the old `os.path` functions.

**Where used:** `code/programs/python/build-tool/`

## Context Managers and `with`

For resources that need cleanup (files, connections, temp directories):

```python
from contextlib import contextmanager

@contextmanager
def temp_cache():
    cache = BuildCache()
    try:
        yield cache
    finally:
        cache.cleanup()
```

**Where used:** Various test helpers

## List/Dict/Set Comprehensions

Compact data transformations:

```python
# Filter packages by language
python_pkgs = [p for p in packages if p.language == "python"]

# Build a lookup table
name_to_pkg = {pkg.name: pkg for pkg in packages}

# Collect unique languages
languages = {pkg.language for pkg in packages}
```

**Where used:** Everywhere — this is idiomatic Python
