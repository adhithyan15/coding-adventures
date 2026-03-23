# Starlark Builtins

Shared Starlark modules that are loaded by BUILD rules across the monorepo.

## Modules

### `cmd.star` — Structured Command Builders

Provides functions for creating OS-aware structured command dicts.  Instead of writing raw shell commands that differ between Unix and Windows, BUILD rule authors use `cmd()` and its platform-specific variants.

**Functions:**

| Function | Description |
|----------|-------------|
| `cmd(program, args)` | Universal command — runs on all platforms |
| `cmd_windows(program, args)` | Windows only — returns `None` on other platforms |
| `cmd_linux(program, args)` | Linux only — returns `None` on other platforms |
| `cmd_macos(program, args)` | macOS only — returns `None` on other platforms |
| `cmd_unix(program, args)` | Any Unix (not Windows) — returns `None` on Windows |
| `filter_commands(cmds)` | Strips `None` entries from a command list |

**Usage in a rule file:**

```starlark
load("code/packages/starlark/builtins/cmd.star", "cmd", "cmd_linux", "filter_commands")

def rust_library(name, srcs=[], deps=[]):
    cmds = filter_commands([
        cmd("cargo", ["build"]),
        cmd("cargo", ["test"]),
        cmd_linux("cargo", ["tarpaulin"]),  # Linux-only coverage
    ])
    _targets.append({
        "rule": "rust_library",
        "name": name,
        "commands": cmds,
    })
```

**Requires:** The build tool must inject `_ctx` (with at least `_ctx["os"]`) into every Starlark scope via `WithGlobals()`.

## Adding New Platforms

To support a new OS (e.g., FreeBSD), add a function to `cmd.star`:

```starlark
def cmd_freebsd(program, args=[]):
    if _current_os != "freebsd":
        return None
    return cmd(program, args)
```

No build tool changes needed — `_ctx["os"]` already contains the correct value from `runtime.GOOS`.

## How It Fits in the Stack

```
BUILD file
  └── loads rule (e.g., python_library.star)
        └── loads builtins/cmd.star
              └── reads _ctx["os"] (injected by build tool)
              └── provides cmd(), cmd_windows(), etc.
```
