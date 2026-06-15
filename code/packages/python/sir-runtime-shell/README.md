# coding-adventures-sir-runtime-shell

Shell runtime for **Semantic-IR-emitted Python**.

The Ruby→SIR frontend lowers a backtick literal `` `cmd` `` to
`BuiltinCall("backtick", [StrLit cmd])`. Emitted Python needs a runtime landing
point for that builtin — one that runs the command through the system shell and
returns its stdout, exactly as Ruby's backtick expression does. That runtime is
this package.

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-python ─▶ .py
                                                                             │ imports
                                                                             ▼
                                                       coding-adventures-sir-runtime-shell
```

The Python backend emits an import of this package only when a module uses a
backtick; pure modules never gain the dependency.

## What it does — Ruby backtick semantics

In Ruby, `` `cmd` `` (and `%x{cmd}`) hands the whole command line to the system
shell, waits for it, and evaluates to the command's **standard output** as a
string. The child's exit status is recorded in `$?` but does *not* change the
value — even a non-zero exit returns whatever was printed to stdout. Standard
error is not captured by the expression. This package mirrors all of that:

| Ruby backtick behaviour | Python implementation |
|---|---|
| runs via the system shell | `subprocess.run(..., shell=True)` |
| returns captured stdout as a `str` | `capture_output=True, text=True` → `.stdout` |
| ignores the child's exit status | `check=False` (never raises on non-zero exit) |
| stderr goes to the parent | captured by the call but not returned |

## Security — `shell=True` is intentional and author-supplied input only

The internal `subprocess.run` uses `shell=True`. This is **load-bearing**: Ruby
backtick is *defined* as "run via `/bin/sh -c`", so shell features (pipes,
redirections, globbing, `$VAR`) are part of the builtin. Running without a shell
would silently change the meaning of every compiled backtick.

There is no new untrusted-input path. `command` is the string literal the
programmer wrote inside the backticks of their *own* Ruby source, threaded
verbatim through the compiler into the emitted Python. It carries exactly the
trust level Ruby itself grants it — the author's own code — and this package
interpolates **no** external or runtime-derived data into the command.

## API

| Export | Purpose |
|---|---|
| `backtick(command) -> str` | Run `command` via the system shell and return its captured stdout (Ruby `` `cmd` ``). Non-zero exits still return stdout. |
| `Val` | The universal SIR value type alias (`Any`) at this boundary. |

## Usage

```python
from coding_adventures_sir_runtime_shell import backtick

backtick('python -c "print(123)"')   # "123\n"   (captured stdout)
backtick('python -c "exit(1)"')      # ""        (non-zero exit → stdout still returned)
```

## Development

```bash
uv venv && uv pip install -e .[dev]
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
```

## License

MIT
