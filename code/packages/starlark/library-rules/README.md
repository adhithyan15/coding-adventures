# Starlark Library Rules

Build rule definitions for library packages in the coding-adventures monorepo.

## What Are These Files?

Each `.star` file defines a **build rule** — a function that BUILD files call to declare what kind of package they are, what source files they contain, and what they depend on. The build tool's Starlark interpreter loads and executes these rules.

Think of it like a form you fill out: "I'm a Python library named logic-gates, my source is in src/, and I depend on transistors." The build tool reads the form and figures out how to build, test, and manage your package.

## Available Rules

| File | Rule Function | Language | Test Runner Options |
|------|--------------|----------|-------------------|
| `python_library.star` | `py_library()` | Python | pytest (default), unittest |
| `go_library.star` | `go_library()` | Go | go test (built-in) |
| `ruby_library.star` | `ruby_library()` | Ruby | minitest (default), rspec |
| `typescript_library.star` | `ts_library()` | TypeScript | vitest (default), jest |
| `rust_library.star` | `rust_library()` | Rust | cargo test (built-in) |
| `elixir_library.star` | `elixir_library()` | Elixir | ExUnit (built-in) |

## Usage

In a package's BUILD file:

```python
load("//rules:python_library.star", "py_library")

py_library(
    name = "logic-gates",
    srcs = ["src/**/*.py"],
    deps = ["python/transistors"],
    test_runner = "pytest",
)
```

## Common Parameters

All library rules share these parameters:

- **name** — Package name matching the directory name (e.g., "logic-gates")
- **srcs** — Glob patterns for change detection (e.g., `["src/**/*.py"]`)
- **deps** — Dependencies as `"language/package-name"` strings (e.g., `["python/transistors"]`)

Some rules have additional parameters:
- **test_runner** — Python: pytest/unittest. Ruby: minitest/rspec. TypeScript: vitest/jest.

## How the Build Tool Uses These

1. The build tool discovers all BUILD files in the repo
2. For each BUILD file, it executes the Starlark code (which calls rules like `py_library()`)
3. Each rule call registers a target with its metadata (name, srcs, deps)
4. The build tool constructs a dependency graph from all targets
5. It determines which targets need rebuilding (via git diff change detection)
6. It builds targets in topological order, parallelizing independent ones

## How This Fits in the Stack

```
BUILD files (per-package)          — "I am a Python library called X"
    |
    v
Library/Program rules (.star)      — "Here's what py_library means"  <-- YOU ARE HERE
    |
    v
Build tool (Go program)            — "I'll figure out what to build and in what order"
    |
    v
Language toolchains                — "I'll actually compile/test the code"
(pytest, go test, cargo, etc.)
```
