# ca-capability-analyzer

Static analyzer for OS capability detection in Python source code. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) capability security system.

## What It Does

Walks the Python AST to detect OS-level capability usage — filesystem access, network connections, process execution, environment variable reads, FFI calls — and compares them against a package's declared capabilities in `required_capabilities.json`.

This is **Layer 4 (CI Gate)** of the capability security friction stack. It answers: "Does this package use only the capabilities it declared?"

## How It Works

The analyzer uses Python's `ast` module to parse source files into abstract syntax trees, then visits each node looking for patterns that indicate capability usage:

| AST Pattern | Detected Capability |
|-------------|-------------------|
| `import os` | `fs:*:*` |
| `open("file.txt")` | `fs:read:file.txt` |
| `open("file.txt", "w")` | `fs:write:file.txt` |
| `import socket` | `net:*:*` |
| `import subprocess` | `proc:exec:*` |
| `os.environ["KEY"]` | `env:read:KEY` |
| `import ctypes` | `ffi:*:*` |

It also detects **banned constructs** — dynamic execution patterns like `eval()`, `exec()`, `__import__()`, `pickle.loads()` — that are forbidden regardless of declared capabilities because they enable static analysis evasion.

## Installation

```bash
pip install ca-capability-analyzer
```

## CLI Usage

### Detect capabilities

Scan source files and output detected capabilities as JSON:

```bash
ca-capability-analyzer detect src/
ca-capability-analyzer detect src/my_module.py
```

### Check against manifest

Compare detected capabilities against `required_capabilities.json` and exit with code 0 (pass) or 1 (fail):

```bash
ca-capability-analyzer check src/
ca-capability-analyzer check src/ --manifest path/to/required_capabilities.json
ca-capability-analyzer check src/ --json  # Also output JSON
```

The manifest is auto-discovered by walking up from the source directory.

### Scan for banned constructs

Find dynamic execution constructs that are banned outright:

```bash
ca-capability-analyzer banned src/
```

## Usage in BUILD Files / CI

```bash
# In a BUILD file or CI step:
.venv/bin/python -m ca_capability_analyzer check src/

# Exit code 0 = all capabilities declared
# Exit code 1 = undeclared capabilities found
```

## Python API

```python
from ca_capability_analyzer import (
    analyze_file,
    analyze_directory,
    load_manifest,
    compare_capabilities,
    detect_banned_constructs,
)

# Detect capabilities in a file
caps = analyze_file("src/my_module.py")
for cap in caps:
    print(f"{cap.category}:{cap.action}:{cap.target} at line {cap.line}")

# Compare against manifest
manifest = load_manifest("required_capabilities.json")
result = compare_capabilities(caps, manifest)
print(result.summary())

# Scan for banned constructs
violations = detect_banned_constructs("src/my_module.py")
for v in violations:
    print(f"BANNED: {v.construct} at {v.file}:{v.line}")
```

## Capability Taxonomy

Format: `category:action:target`

| Category | Actions | Example |
|----------|---------|---------|
| `fs` | read, write, create, delete, list | `fs:read:../../grammars/*.tokens` |
| `net` | connect, listen, dns | `net:connect:*` |
| `proc` | exec, fork, signal | `proc:exec:git` |
| `env` | read, write | `env:read:HOME` |
| `ffi` | call, load | `ffi:*:*` |

## Banned Constructs

These are banned regardless of declared capabilities:

| Construct | Why |
|-----------|-----|
| `eval()` | Executes arbitrary Python from a string |
| `exec()` | Executes arbitrary Python statements |
| `compile()` | Creates code objects from strings |
| `__import__()` | Imports modules by name, evading static analysis |
| `importlib.import_module()` | Same, official API |
| `getattr()` with dynamic arg | Dynamic attribute access on modules |
| `globals()` / `locals()` | Dict access to scope, enabling injection |
| `pickle.loads()` / `pickle.load()` | Deserializes arbitrary objects |
| `marshal.loads()` / `marshal.load()` | Deserializes Python bytecode |
| `import ctypes` / `import cffi` | FFI — calls arbitrary C functions |

## Design Philosophy

This is **friction engineering**, not perfect security. The analyzer is bypassable — an attacker could modify the linter config or CI workflow. But each bypass requires a separate visible action that leaves an audit trail. The goal is to make attacks slow, loud, and reviewable.

## License

MIT
