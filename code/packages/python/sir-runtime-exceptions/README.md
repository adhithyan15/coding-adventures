# coding-adventures-sir-runtime-exceptions

Exception runtime for **Semantic-IR-emitted Python**.

The SIR (Semantic IR) backends translate most Ruby-surface constructs to
*native* Python: a sequence becomes a `list`, a loop becomes a `for`, a
`begin/rescue/ensure` becomes a native `try / except / finally`. Two pieces of
exception handling have **no faithful native equivalent**, and this package
supplies them:

1. **A SIR exception object** — `SirError`, a real `Exception` (so tracebacks
   work) tagged with the Ruby/SIR class name in `sir_class`. Python's `raise`
   takes an exception *instance* and a plain `Exception` carries no class tag.
2. **Rescue-clause type matching** — a native `except` matches by Python class.
   Ruby's `rescue TypeError, ArgumentError => e` matches a *set* of Ruby class
   *names* (and their subclasses) and falls through otherwise. `rescue_matches`
   answers "does this caught value match this clause?" so the emitted `except`
   body can dispatch to the right clause or re-`raise`.

It is **keyed to SIR, not Ruby**: a future JavaScript→SIR→Python path reuses it
unchanged. See [`code/specs/sir-runtime.md`](../../../specs/sir-runtime.md).

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-python ─▶ .py
                                                                             │ imports
                                                                             ▼
                                                       coding-adventures-sir-runtime-exceptions
```

The Python backend emits a `from coding_adventures_sir_runtime_exceptions import …`
header (aliased `_sir_exc_*`) **only** when a module uses the `Exceptions`
feature (a `try`/`rescue` or a `raise`); pure modules never gain the dependency.

## API

| Export | Purpose |
|---|---|
| `class SirError(Exception)` | The raised object; `sir_class` holds the Ruby class name, the message defaults to the class name. |
| `raise_error(class_name="RuntimeError", message=None) -> NoReturn` | Raise a `SirError`. Bare `raise_error()` re-raises as a generic `RuntimeError`. |
| `class_of_thrown(exc) -> str` | The SIR class name of a caught value (native errors → `StandardError`). |
| `rescue_matches(exc, class_names) -> bool` | Does a caught value match a `rescue` clause naming `class_names`? Empty list = bare `rescue` (catch-all). |
| `register_ancestry(mapping) -> None` | Merge user `{childClassName: superclassName}` edges (from `class Child < Parent`) into the ancestry the matcher walks, so `rescue StandardError` catches a raised user `MyErr < StandardError`. Additive over the built-in table; explicit string map (no reflection). |

### Emitted shape

```python
from coding_adventures_sir_runtime_exceptions import (
    raise_error as _sir_exc_raise_error,
    rescue_matches as _sir_exc_rescue_matches,
    SirError as _SirError,
)

try:
    _sir_exc_raise_error("ArgumentError", "bad")
except _SirError as __exc:
    if _sir_exc_rescue_matches(__exc, ["StandardError"]):
        e = __exc          # rescue StandardError => e
        ...
    else:
        raise              # no clause matched → propagate
finally:
    ...                    # ensure body
```

## Usage

```python
from coding_adventures_sir_runtime_exceptions import SirError, raise_error, rescue_matches

try:
    raise_error("KeyError", "missing")
except SirError as e:
    rescue_matches(e, ["IndexError"])  # True — KeyError < IndexError
    e.sir_class                        # "KeyError"
```

## Built-in exception hierarchy

SIR has no exception-class symbol table, so this package bakes in a curated
slice of Ruby's built-in tree so `rescue StandardError` catches the everyday
subclasses:

```
Exception
└─ StandardError
   ├─ RuntimeError  ├─ ArgumentError      ├─ TypeError
   ├─ NameError ─ NoMethodError           ├─ RangeError
   ├─ IndexError ─ KeyError               ├─ ZeroDivisionError
   ├─ IOError     ├─ StopIteration        └─ NotImplementedError
```

`Exception` (and a bare `rescue`) matches anything; **user-defined** exception
classes match by exact name only.

## v0 limitation (honest)

Because SIR threads no exception-class definitions, the ancestry of
*user-defined* classes is unknown here — `rescue StandardError` will **not**
catch a user `class MyError < StandardError` (it matches `MyError` by exact name
only). A bare `raise` with no in-flight exception re-raises as a generic
`RuntimeError`. Both await a frontend that threads the exception class model and
in-flight exception into SIR.

## Development

```bash
uv venv && uv pip install -e .[dev]
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
```

## License

MIT
