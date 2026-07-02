# coding-adventures-sir-runtime-core

Core runtime imported by **Semantic-IR-emitted Python**.

## What it is

Semantic-IR (SIR) backends translate most constructs to **native** Python — a
sequence is a `list`, a map is a `dict`, a loop is a `for`, a class is a `class`.
A handful of SIR semantics have **no faithful native equivalent**, and those live
here so the emitted code can `import` and call them instead of inlining a runtime
prelude into every file:

| Provided | Why it's not native |
|---|---|
| `truthy(v)` | SIR truthiness is **false/nil-only** — `0`, `""`, `[]`, `{}` are *truthy*, unlike Python's `bool()`. |
| `Symbol` / `intern` | Interned identity objects; Python has no symbol type. |
| `Pair` / `cons` / `car` / `cdr` | Lisp cons cells; no native type. |
| `eq`, `to_display`, `print` | Symbol-aware equality and Lisp/Ruby display (`nil`, `#t`/`#f`). |
| `sir_puts` (`puts`) | Ruby `puts`: per-arg line, arrays flattened element-per-line, no double trailing newline, no-arg → one newline. |
| `add`/`sub`/`mul`/`div`, `lt`/`gt` | Variadic folds + truncating-integer `/`. |
| `Closure`/`apply`/`make_closure`, global store, builtin dispatch | Uniform closure handles + SIR `Globals`. |

It implements **SIR** semantics, not any one source language's — so a Ruby
frontend today and a JavaScript or Python frontend tomorrow all reuse it.

## How emitted code uses it

```python
import coding_adventures_sir_runtime_core as _sir

def add(a, b):
    return a + b

xs = [1, 2, 3]
i = 0
while _sir.truthy(i < len(xs)):
    _sir.print(xs[i])
    i = i + 1
```

## Where it fits

Frontend (Ruby / JS / Python …) → `semantic-ir` → `semantic-ir-to-python` →
emitted `.py` that imports this package. See
[`code/specs/sir-runtime.md`](../../../specs/sir-runtime.md).

## Development

```sh
uv venv && uv pip install -e .[dev]
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest
```
