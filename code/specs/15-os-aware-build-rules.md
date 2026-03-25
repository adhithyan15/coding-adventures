# OS-Aware Starlark BUILD Rules
## Specification

### Problem

The monorepo has **79 BUILD_windows files** that exist as stopgaps because:

1. `GenerateCommands()` in the build tool hardcodes Unix shell syntax (e.g., `".[dev]"` quoting, `2>/dev/null` redirects).
2. Shell BUILD files contain platform-specific commands.
3. BUILD file authors must create per-platform variants manually.

This doesn't scale. Adding a new OS means adding more BUILD variants to every package.

### Solution

Make Starlark BUILD rules inherently OS-aware:

1. The build tool **injects a `_ctx` dict** into every Starlark scope via a proper VM-level global injection mechanism.
2. A `cmd.star` library provides `cmd()`, `cmd_windows()`, `cmd_linux()`, `cmd_macos()`, `cmd_unix()` functions that read `_ctx["os"]`.
3. Rules compose platform-specific and universal commands. Platform-irrelevant commands return `None` and are filtered out.
4. The build tool reads the final `commands` list from `_targets` and renders each structured command to a shell string.

BUILD file authors write a single Starlark BUILD file. No BUILD_windows, no BUILD_mac, no BUILD_linux.

### Non-goals

- Changing how shell BUILD files work (full backward compatibility).
- Adding OS detection to the Starlark VM itself (the VM is language-agnostic).
- Rewriting the executor's shell dispatch (it already handles `sh -c` vs `cmd /C`).

---

## `_ctx` Build Context Schema (v1)

The build tool constructs this dict and injects it as a global variable named `_ctx` into every Starlark execution scope.

### Schema

```starlark
_ctx = {
    # --- Version ---
    "version":   1,          # Schema version (integer, incrementing)

    # --- Platform ---
    "os":        "darwin",   # "darwin", "linux", "windows", "freebsd", "openbsd", ...
    "arch":      "arm64",    # "amd64", "arm64", "386", "riscv64", ...
    "cpu_count": 10,         # Number of logical CPUs

    # --- Environment ---
    "ci":        False,      # True when running in CI (GitHub Actions, etc.)

    # --- Paths ---
    "repo_root": "/path/to/repo",  # Absolute path to repository root
}
```

### Field Reference

| Field | Type | Source (Go) | Description |
|-------|------|-------------|-------------|
| `version` | int | `1` (constant) | Schema version for backward compatibility |
| `os` | string | `runtime.GOOS` | Operating system identifier |
| `arch` | string | `runtime.GOARCH` | CPU architecture identifier |
| `cpu_count` | int | `runtime.NumCPU()` | Logical CPU count for parallelism decisions |
| `ci` | bool | `os.Getenv("CI") != ""` | Whether running in a CI environment |
| `repo_root` | string | CLI `-root` flag | Absolute path to the monorepo root |

### OS Identifiers

All build tools normalize their platform detection to Go's `runtime.GOOS` values:

| Value | Platform | Go | Python | TypeScript | Rust |
|-------|----------|-----|--------|-----------|------|
| `"darwin"` | macOS | `runtime.GOOS` | `platform.system()=="Darwin"` | `os.platform()=="darwin"` | `consts::OS=="macos"` |
| `"linux"` | Linux | `runtime.GOOS` | `platform.system()=="Linux"` | `os.platform()=="linux"` | `consts::OS=="linux"` |
| `"windows"` | Windows | `runtime.GOOS` | `platform.system()=="Windows"` | `os.platform()=="win32"` | `consts::OS=="windows"` |

### Versioning Rules

- **`version`** is required. Consumers SHOULD check it if they depend on specific fields.
- **Additive changes** (new fields) do NOT require a version bump.
- **Breaking changes** (removing, renaming, or changing field semantics) require `version` → v2.
- The build tool always sets `version` to its current constant.

| Change | Version Bump? | Reason |
|--------|---------------|--------|
| Add `"user"` field | No | Additive |
| Add `"go_version"` | No | Additive |
| Rename `"os"` → `"platform"` | Yes → v2 | Breaking |
| Remove `"repo_root"` | Yes → v2 | Breaking |
| Change `"cpu_count"` int → string | Yes → v2 | Breaking |

---

## Injection Mechanism: VM-Level Global State

### Layer 1: GenericVM

Add `InjectGlobals(globals map[string]interface{})` to the GenericVM. This pre-seeds named variables into the VM's global scope before execution begins.

- Injected globals are merged into `Variables` (they don't replace the map).
- If a key already exists, the injected value overwrites it.
- This is a general-purpose mechanism — not specific to `_ctx`.

### Layer 2: Starlark Interpreter

Add `WithGlobals(globals map[string]interface{})` as an `InterpreterOption`. The interpreter calls `vm.InjectGlobals(interp.Globals)` after creating each VM instance — including VMs created for `load()` calls. This means `_ctx` is automatically available in every loaded `.star` file.

### Layer 3: Build Tool

The build tool constructs the `_ctx` dict from runtime information and passes it via `WithGlobals({"_ctx": ctxDict})` when creating the interpreter.

---

## Structured Commands

### `cmd()` Function

The `cmd()` function (in `cmd.star`) creates a structured command dict:

```starlark
def cmd(program, args=[]):
    return {"type": "cmd", "program": program, "args": args}
```

### Platform-Specific Variants

Each variant checks `_ctx["os"]` and returns `None` if the current OS doesn't match:

```starlark
def cmd_windows(program, args=[]):
    if _ctx["os"] != "windows":
        return None
    return cmd(program, args)

def cmd_linux(program, args=[]):
    if _ctx["os"] != "linux":
        return None
    return cmd(program, args)

def cmd_macos(program, args=[]):
    if _ctx["os"] != "darwin":
        return None
    return cmd(program, args)

def cmd_unix(program, args=[]):
    if _ctx["os"] == "windows":
        return None
    return cmd(program, args)
```

Adding new platforms (e.g., `cmd_freebsd()`) requires only a new function in `cmd.star`. No build tool changes.

### `filter_commands()`

Removes `None` entries from a command list:

```starlark
def filter_commands(cmds):
    result = []
    for c in cmds:
        if c != None:
            result.append(c)
    return result
```

### Command Rendering

The build tool converts structured command dicts to shell strings by joining `program` and `args` with proper quoting. Since platform-specific filtering already happened in Starlark, the renderer is platform-agnostic — it just joins strings.

---

## Rule Updates

Each rule file adds a `"commands"` field to `_targets`:

```starlark
load("code/packages/starlark/builtins/cmd.star", "cmd", "filter_commands")

_targets = []

def py_library(name, srcs=[], deps=[], test_runner="pytest"):
    cmds = [cmd("uv", ["pip", "install", "--system", "-e", ".[dev]"])]
    if test_runner == "pytest":
        cmds.append(cmd("python", ["-m", "pytest", "--cov", "--cov-report=term-missing"]))
    else:
        cmds.append(cmd("python", ["-m", "unittest", "discover", "tests/"]))
    _targets.append({
        "rule": "py_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "test_runner": test_runner,
        "commands": filter_commands(cmds),
    })
```

### Backward Compatibility

- Targets WITH `"commands"` → build tool uses the command renderer.
- Targets WITHOUT `"commands"` → build tool falls back to `GenerateCommands()`.
- Shell BUILD files → completely unchanged.

---

## Migration Path

1. **Pilot**: Convert 3 packages (1 Python, 1 Rust, 1 Elixir) to Starlark BUILD files.
2. **Batch**: Convert remaining ~76 packages in groups by language.
3. **Delete**: Remove all BUILD_windows files.

After migration, a package that previously needed two files (BUILD + BUILD_windows) has one Starlark BUILD file that works everywhere.
