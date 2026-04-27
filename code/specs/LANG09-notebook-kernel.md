# LANG09 — Notebook Kernel: Jupyter-Compatible Kernels for Every Language

## Overview

A **notebook kernel** is a long-running process that executes code cells on
behalf of a notebook frontend (Jupyter, JupyterLab, VS Code Notebooks, Marimo,
Quarto, etc.).  Kernels speak the **Jupyter Messaging Protocol** (JMP) over
ZeroMQ sockets.

Any language built on the LANG pipeline can be exposed as a notebook kernel
with minimal effort.  The kernel wraps the same `LanguagePlugin` used by the
REPL (LANG08) and adds:

1. **Cell execution** — run a code cell, return rich output (text, HTML, images)
2. **Kernel info** — language name, version, file extension, MIME type
3. **Interrupt handling** — cancel a running cell without killing the kernel
4. **Inspection** — hover-equivalent for `?obj` syntax
5. **Completions** — tab-completion inside a cell
6. **State persistence** — variable state accumulates across cells (same as REPL)

---

## What the language must provide

Exactly the same `LanguagePlugin` as LANG08.  The notebook kernel is a thin
adapter over the REPL plugin:

```python
from lang09_kernel import NotebookKernel
from my_lang_repl import MyLangREPLPlugin

kernel = NotebookKernel(
    language=MyLangREPLPlugin(),
    kernel_name="mylang",
    display_name="My Language",
    language_version="1.0.0",
    file_extension=".ml",
    mimetype="text/x-mylang",
    pygments_lexer="text",      # for syntax highlighting in output
)
kernel.start()    # blocks; listens on ZeroMQ sockets
```

That is the complete kernel implementation.  No Jupyter internals need to be
understood by the language author.

---

## Architecture

```
Notebook frontend (Jupyter, VS Code, Marimo)
    │
    │  Jupyter Messaging Protocol (JMP)
    │  ZeroMQ: shell socket, iopub socket, stdin socket, control socket
    │
    ▼
NotebookKernel (LANG09)
    │
    ├── Shell handler: execute_request, inspect_request, complete_request
    ├── Control handler: interrupt_request, shutdown_request
    ├── IOPub publisher: stream, execute_result, display_data, error
    │
    ▼
LanguagePlugin (LANG08)
    │
    ├── eval(source) → result
    ├── completions(partial, cursor) → list[str]
    └── inspect(source, cursor) → InspectResult
    │
    ▼
vm-core + jit-core
    (same runtime as REPL and debug sessions)
```

---

## Jupyter Messaging Protocol integration

JMP defines message types over four ZeroMQ sockets:

| Socket | Direction | Purpose |
|--------|-----------|---------|
| `shell` | frontend → kernel | execute, complete, inspect requests |
| `iopub` | kernel → all frontends | stream output, results, status |
| `stdin` | kernel → frontend | input() requests |
| `control` | frontend → kernel | interrupt, shutdown |

The `NotebookKernel` class handles all four sockets.  Message routing:

```
execute_request → LanguagePlugin.eval() → execute_result or error on iopub
complete_request → LanguagePlugin.completions() → complete_reply on shell
inspect_request  → LanguagePlugin.inspect() → inspect_reply on shell
interrupt_request → vm.interrupt() → kernel pauses current eval
shutdown_request → kernel.stop() → clean shutdown
```

### execute_request

```python
def handle_execute_request(self, msg: dict) -> None:
    source = msg["content"]["code"]
    self._publish_status("busy")
    try:
        result = self._plugin.eval(source)
        if result:
            self._publish_execute_result(result, execution_count=self._count)
    except LanguagePlugin.NeedsMoreInput:
        self._publish_stream("stderr", "Incomplete input — submit complete statements.")
    except Exception as e:
        self._publish_error(ename=type(e).__name__, evalue=str(e), traceback=[])
    finally:
        self._count += 1
        self._publish_status("idle")
```

### Rich output

Results are not restricted to plain text.  The `LanguagePlugin` can return a
`RichOutput` object with multiple MIME representations:

```python
@dataclass
class RichOutput:
    plain: str                          # text/plain — always required
    html: str | None = None            # text/html — optional rich rendering
    image_png: bytes | None = None     # image/png — for plot output
    json: dict | None = None           # application/json — for structured data
    latex: str | None = None           # text/latex — for math rendering
```

```python
# Example: a language that can produce SVG plots
def eval(self, source: str) -> str | RichOutput:
    ...
    if result_is_a_plot:
        return RichOutput(
            plain="<Plot: 320x240>",
            image_png=render_png(plot),
        )
    return repr(result)
```

The notebook frontend displays the richest representation it supports.
VS Code Notebooks renders `image_png`; classic Jupyter renders `text/html`.

---

## State persistence across cells

The kernel uses the same incremental state model as the REPL (LANG08):

- A single `VMCore` instance persists for the kernel's lifetime
- A single `IIRModule` accumulates all function definitions
- Cell `N+1` sees all definitions from cells `1` through `N`

This is identical to how IPython/Jupyter kernels work: every cell shares the
same Python namespace.

### Kernel restart

When the user restarts the kernel (Kernel → Restart), the `NotebookKernel`:

1. Calls `vm.reset()` to clear register state
2. Replaces `IIRModule` with a fresh empty module
3. Clears the JIT cache
4. Publishes a `kernel_info_reply` to confirm readiness

---

## Cell interrupt handling

Long-running cells (e.g., a recursive Fibonacci call that takes 30 seconds)
must be interruptible without killing the kernel process.

```python
def handle_interrupt_request(self) -> None:
    self._vm.interrupt()   # new vm-core API
```

`vm.interrupt()` sets an `_interrupted` flag that the dispatch loop checks
between instructions:

```python
# In dispatch_loop:
if self._interrupted:
    self._interrupted = False
    raise VMInterrupt("execution interrupted by kernel")
```

`VMInterrupt` is caught by `handle_execute_request`, which publishes an
`error` message with `ename="KeyboardInterrupt"` — the same presentation
Jupyter uses for Python's `KeyboardInterrupt`.

---

## Inspection (`?`)

Jupyter notebooks support `obj?` and `obj??` syntax for quick documentation.
The `inspect_request` message is routed to `LanguagePlugin.inspect()`:

```python
class MyLangREPLPlugin(LanguagePlugin):
    def inspect(self, source: str, cursor_pos: int) -> InspectResult:
        word = extract_word_at(source, cursor_pos)
        fn = self._module.get_function(word)
        if fn is None:
            return InspectResult(found=False)
        return InspectResult(
            found=True,
            plain=f"{fn.name}({', '.join(p for p, _ in fn.params)}) -> {fn.return_type}",
            html=f"<b>{fn.name}</b>({', '.join(f'<i>{p}</i>: {t}' for p, t in fn.params)}) → {fn.return_type}",
        )
```

If the LSP server (LANG07) is running, `inspect()` can delegate to the LSP
`textDocument/hover` handler, getting documentation from doc-comments.

---

## Kernel spec file

Jupyter discovers kernels via a `kernel.json` file installed in a well-known
directory.  The `NotebookKernel.install()` class method writes this file:

```python
NotebookKernel.install(
    kernel_name="mylang",
    display_name="My Language",
    language="mylang",
    argv=["python", "-m", "my_lang_kernel", "--connection-file", "{connection_file}"],
)
```

This writes to `~/.local/share/jupyter/kernels/mylang/kernel.json`:

```json
{
  "display_name": "My Language",
  "language": "mylang",
  "argv": ["python", "-m", "my_lang_kernel", "--connection-file", "{connection_file}"]
}
```

After installation, the kernel appears in Jupyter's kernel selector.

---

## VS Code Notebook integration

VS Code Notebooks speak JMP via the `jupyter` extension.  The generic kernel
works out of the box — no VS Code extension needed beyond the standard
Jupyter extension.

For a richer experience (syntax highlighting, cell-level diagnostics from the
LSP), the `coding-adventures-lsp` extension (LANG07) adds VS Code Notebook
cell language support:

```json
// package.json contributes
"notebookRenderer": [
  { "id": "mylang", "displayName": "My Language", "mimeTypes": ["text/x-mylang"] }
]
```

---

## Pipeline hooks summary

The notebook kernel uses every layer of the LANG pipeline:

| Pipeline layer | Role in notebook |
|----------------|-----------------|
| Frontend (lexer + parser) | Parses each cell on submission |
| Type checker | Provides hover info and completion |
| Bytecode compiler | Compiles cell to `IIRModule` additions |
| `vm-core` | Executes cells; persists state across cells |
| `jit-core` | Automatically compiles hot functions after repeated cell execution |
| `debug-integration` (LANG06) | Breakpoints inside cells via the VS Code DAP extension |
| `lsp-integration` (LANG07) | Tab-completion and hover inside cells |
| `repl-integration` (LANG08) | `LanguagePlugin` re-used directly; state model shared |

---

## Package structure

```
lang09-kernel/
  pyproject.toml
  BUILD
  README.md
  CHANGELOG.md
  src/lang09_kernel/
    __init__.py       # exports NotebookKernel, RichOutput, InspectResult
    kernel.py         # NotebookKernel class — main entry point
    messaging.py      # JMP message encoding / decoding
    sockets.py        # ZeroMQ socket setup (shell, iopub, stdin, control)
    handlers.py       # per-message-type handlers
    install.py        # NotebookKernel.install() — kernel spec writer
    rich_output.py    # RichOutput dataclass + MIME bundle encoding
  tests/
    test_execute.py       # cell execution round-trips (mocked sockets)
    test_interrupt.py     # interrupt during long-running cell
    test_completions.py
    test_inspect.py
    test_rich_output.py
    test_install.py
```

---

## Non-goals

- **Distributed / multi-kernel execution** — out of scope; each kernel is
  a single process
- **Kernel gateway** — routing multiple clients to the same kernel is handled
  by Jupyter Server, not by this package
- **Custom frontend** — this spec targets standard Jupyter/VS Code frontends;
  a custom notebook UI is a future concern
- **Persistent kernel state across Python process restarts** — restoring state
  from a saved `.ipynb` file is the notebook frontend's responsibility
