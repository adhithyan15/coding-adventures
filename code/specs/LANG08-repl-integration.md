# LANG08 — REPL Integration: Interactive Sessions for Every Language

## Overview

Any language built on the LANG pipeline can be wired into the `PL00` generic
REPL framework in under 30 lines.  The result is a fully featured interactive
session with history, multi-line input, syntax error recovery, and (optionally)
a rich async waiting experience.

This spec defines the three integration points between the LANG pipeline and
PL00:

1. The **language plugin** — wraps the LANG compiler + vm-core into a single
   `eval(input) → result` callable
2. The **incremental state model** — how the REPL accumulates definitions across
   successive inputs without restarting the VM
3. The **multi-line input protocol** — how the parser signals that a statement
   is incomplete so the REPL can read more lines before evaluating

---

## What the language must provide

```python
from pl00 import REPL, LanguagePlugin, PromptPlugin
from my_lang_lexer import tokenise
from my_lang_parser import parse, ParseIncomplete
from my_lang_compiler import compile_module
from interpreter_ir import IIRModule
from vm_core import VMCore

class MyLangREPLPlugin(LanguagePlugin):
    def __init__(self):
        self._vm = VMCore(register_count=8, max_frames=64,
                          opcodes=MY_LANG_OPCODES, u8_wrap=False)
        self._module = IIRModule(name="repl", functions=[], entry_point=None)

    def eval(self, source: str) -> str:
        try:
            tokens = tokenise(source)
            ast = parse(tokens)
        except ParseIncomplete:
            # Signal to the REPL to read another line before evaluating.
            raise LanguagePlugin.NeedsMoreInput
        except SyntaxError as e:
            return f"SyntaxError: {e}"

        new_fns, result_expr = compile_module(ast, existing=self._module)
        self._module.functions.extend(new_fns)

        if result_expr is not None:
            result = self._vm.execute_expr(result_expr, self._module)
            return repr(result)
        return ""   # definition-only input (e.g. "fn add(a, b) = a + b")

repl = REPL(
    language=MyLangREPLPlugin(),
    prompt=PromptPlugin(global_prompt=">>> ", continuation_prompt="... "),
)
repl.run()
```

That is the complete language-specific REPL implementation.

---

## The incremental state model

A REPL session accumulates state: functions defined in earlier inputs must be
available in later ones.  The LANG pipeline handles this naturally because
`IIRModule` is a mutable container of `IIRFunction` objects.

The pattern is:

1. Start with an empty `IIRModule` (no functions)
2. On each `eval` call, compile the new input into `new_fns` (new function
   definitions) and `result_expr` (the expression to evaluate, if any)
3. Append `new_fns` to the persistent `IIRModule`
4. Evaluate `result_expr` in the context of the full accumulated `IIRModule`

```
Session:
  Input 1: "fn double(x) = x + x"
    → new_fns = [IIRFunction("double", ...)]
    → module.functions = [double]
    → result = ""   (no expression to print)

  Input 2: "double(21)"
    → new_fns = []
    → result_expr = IIRInstr sequence for "call double [21]"
    → vm evaluates in context of module (double is defined)
    → result = "42"
    → prints: 42
```

### Re-defining functions

If the user re-defines a function that already exists, the new definition
replaces the old one in `module.functions`:

```python
def _update_module(self, new_fns: list[IIRFunction]) -> None:
    existing = {fn.name: i for i, fn in enumerate(self._module.functions)}
    for fn in new_fns:
        if fn.name in existing:
            self._module.functions[existing[fn.name]] = fn  # replace
        else:
            self._module.functions.append(fn)
```

The vm-core JIT cache entry for the old version is invalidated automatically
when `jit-core` is in use (via `jit.invalidate(fn.name)`).

---

## Multi-line input protocol

Some language constructs span multiple lines:

```
>>> fn factorial(n) =
...   if n == 0 then 1
...   else n * factorial(n - 1)
```

The `PL00` framework handles multi-line input via the `NeedsMoreInput`
exception.  When the parser encounters an incomplete token stream (e.g., an
unclosed `if` or an unterminated function body), it raises `ParseIncomplete`.
The REPL plugin converts this to `NeedsMoreInput`, and the framework reads
another line, appends it, and retries `eval`.

The framework switches from `global_prompt` (`>>>`) to `continuation_prompt`
(`...`) automatically when `NeedsMoreInput` is raised.

### Detecting incompleteness

The parser must be able to distinguish:

- **Complete but invalid** — `fn 123 = abc` → `SyntaxError` (do not request more lines)
- **Incomplete** — `fn factorial(n) =` → `ParseIncomplete` (read more)

Most LL/recursive-descent parsers can detect incompleteness naturally: if the
parser is waiting for a token that would complete a production rule and reaches
EOF, that is `ParseIncomplete`.

```python
class MyLangParser:
    def parse(self, tokens: list[Token]) -> AST:
        try:
            return self._parse_program(tokens)
        except UnexpectedEOF:
            raise ParseIncomplete("unexpected end of input")
        except SyntaxError:
            raise   # real error, not incompleteness
```

---

## JIT integration in REPL sessions

When `jit-core` is used, the REPL benefits from JIT compilation across
successive invocations of the same function:

```
>>> fn fib(n) = if n <= 1 then n else fib(n-1) + fib(n-2)
>>> fib(10)    # interpreted; call count = 1
55
>>> fib(20)    # call count = 2
6765
...
>>> fib(30)    # after 10 calls, jit-core compiles fib to native code
832040         # subsequent calls run at native speed
```

No REPL-specific code is needed — `jit-core`'s tier promotion runs
automatically because the `VMCore` instance persists across REPL inputs.

---

## Error recovery

A good REPL does not crash on errors.  The LANG plugin catches all exceptions
and formats them for display:

```python
def eval(self, source: str) -> str:
    try:
        ...
        result = self._vm.execute_expr(result_expr, self._module)
        return repr(result)
    except ParseIncomplete:
        raise LanguagePlugin.NeedsMoreInput
    except SyntaxError as e:
        return f"SyntaxError at line {e.lineno}: {e.msg}"
    except VMError as e:
        return f"RuntimeError: {e}"
    except Exception as e:
        return f"InternalError: {e}"
```

The VM's register state after an error is undefined.  To prevent stale state
from corrupting subsequent inputs, vm-core's `execute_expr` runs in a
**snapshot frame**: a shallow copy of the current register file is used for
evaluation; on success it is committed back; on error it is discarded.

```python
with vm.snapshot():
    result = vm.execute_expr(result_expr, module)
# On exception: snapshot is rolled back automatically.
# On success: snapshot is committed.
```

---

## Rich REPL features via prompt plugin

The `PL00` framework's `PromptPlugin` interface handles history and line
editing.  The LANG REPL inherits:

- **Command history** — up/down arrows navigate previous inputs
- **Reverse search** — Ctrl-R searches history
- **Line editing** — Ctrl-A (start of line), Ctrl-E (end), Ctrl-K (kill to end)
- **Tab completion** — pluggable; can be wired to the LSP completion provider

### Tab completion via LSP

When `lsp-integration` (LANG07) is active, the REPL can route tab-completion
requests to the LSP server:

```python
class MyLangREPLPlugin(LanguagePlugin):
    def completions(self, partial_input: str, cursor: int) -> list[str]:
        return self._lsp_client.complete(partial_input, cursor)
```

This gives the REPL the same completion intelligence as the editor — defined
functions, in-scope variables, and language keywords — without duplicating
the completion logic.

---

## Async waiting experience

For long-running computations, the `PL00` async mode shows a waiting animation.
The LANG plugin opts in by implementing `WaitingPlugin`:

```python
from pl00 import WaitingPlugin

class MyLangWaiting(WaitingPlugin):
    _frames = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"]

    def start(self) -> dict:
        return {"frame": 0, "start_time": time.monotonic()}

    def tick(self, state: dict) -> dict:
        state["frame"] = (state["frame"] + 1) % len(self._frames)
        sys.stdout.write(f"\r{self._frames[state['frame']]} thinking...")
        sys.stdout.flush()
        return state

    def tick_ms(self) -> int:
        return 80   # spinner speed

    def stop(self, state: dict) -> None:
        elapsed = time.monotonic() - state["start_time"]
        sys.stdout.write(f"\r  ({elapsed:.2f}s)\n")
```

---

## Package additions

| Package | Addition |
|---------|----------|
| `pl00` (PL00) | No changes — interface is already language-agnostic |
| `vm-core` | `execute_expr()` for single-expression evaluation; `snapshot()` context manager |
| Language packages | 20–30 lines: `LanguagePlugin` subclass connecting lexer/parser/compiler/vm |
