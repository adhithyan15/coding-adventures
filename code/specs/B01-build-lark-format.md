# B01 — BUILD.lark: The Starlark Build Configuration Format

## 1. Overview

This specification defines **BUILD.lark**, a declarative build configuration
format that replaces the shell-script BUILD files used throughout the
coding-adventures monorepo. BUILD.lark files are valid
[Starlark](https://github.com/google/starlark-go) programs — the same
configuration language used by Bazel, Buck2, and Pants.

The `.lark` extension serves a practical purpose during migration: it lets the
build tool distinguish new Starlark configs from legacy shell BUILD files in the
same directory. Once every package has migrated, all `.lark` files will be
renamed back to `BUILD` and the legacy shell path will be removed.

BUILD.lark files are **evaluated** by the Starlark VM defined in spec PY03. They
are not executed directly — the build tool loads each file, evaluates it to
collect a list of targets, then executes the resulting commands in a hermetic
sandbox (spec B02).

```
 BUILD.lark file              Starlark VM              Build Tool
 ┌──────────────┐     eval    ┌──────────┐   targets   ┌──────────┐
 │ load(...)    │ ──────────> │ Execute  │ ──────────> │ Schedule │
 │ python_lib(  │             │ Starlark │             │ & Run    │
 │   name=...   │             │ code     │             │ commands │
 │ )            │             └──────────┘             └──────────┘
 └──────────────┘
```

## 2. Why BUILD.lark?

The current shell-script BUILD files have served the project well for simple
cases, but they break down as the monorepo grows. Let's examine each problem
and how BUILD.lark addresses it.

### Problem 1: Platform-specific variants

Many packages carry two files: `BUILD` (Unix) and `BUILD_windows` (Windows).
The differences are usually trivial — path separators, shell quoting, and which
Python binary to invoke:

```bash
# BUILD (Unix)
.venv/bin/python -m pytest tests/ -v

# BUILD_windows
.venv\Scripts\python.exe -m pytest tests/ -v
```

Maintaining two files that are 95% identical is error-prone. When someone adds
a new build step to `BUILD`, they often forget to update `BUILD_windows`.

**BUILD.lark solution**: The `select()` function handles platform conditionals
inline, in a single file:

```starlark
cmd(select({"windows": ".venv\\Scripts\\python.exe",
            "default": ".venv/bin/python"}),
    ["-m", "pytest", "tests/", "-v"])
```

### Problem 2: Manual dependency materialization

Shell BUILD files manually `cd` into sibling directories and run install
commands. This is fragile, order-dependent, and duplicated across packages:

```bash
# Current: manual, imperative, fragile
cd ../state-machine && uv pip install -e . --quiet
cd ../directed-graph && uv pip install -e . --quiet
```

**BUILD.lark solution**: Declare dependencies as labels. The build tool resolves
the dependency graph and materializes them automatically:

```starlark
python_library(
    name = "topological-sort",
    deps = ["//python/state-machine", "//python/directed-graph"],
)
```

### Problem 3: No declared inputs or outputs

Shell BUILD files can read and write anything on the filesystem. There is no
way to know, without reading the script, what files a build step touches. This
makes caching impossible — you cannot skip a build step if you don't know what
it depends on.

**BUILD.lark solution**: Rules declare their source files via `srcs` and
`test_srcs` attributes. The build tool knows exactly what each target reads.

### Problem 4: No composition or reuse

Every BUILD file reinvents the same patterns: create a virtualenv, install
dependencies, run tests. There is no way to extract a reusable "python_library"
pattern that all packages share.

**BUILD.lark solution**: Rules are defined as Starlark functions in `.star`
files. The `load()` statement imports them. One definition of `python_library`
serves every Python package in the repo.

### Problem 5: Shell syntax varies across platforms

Shell quoting, variable expansion, and command chaining all differ between
bash, zsh, PowerShell, and cmd.exe. BUILD files written for one shell may not
work on another.

**BUILD.lark solution**: The `cmd()` function constructs commands as structured
data. The build tool renders them to the correct shell syntax at execution time.

## 3. File Format

A BUILD.lark file is a valid Starlark program. It consists of two kinds of
statements:

1. **`load()` statements** — import rule functions from `.star` files
2. **Rule invocations** — declare build targets

That's it. No `if` statements at the top level, no `for` loops, no variable
assignments (though Starlark supports all of these inside rule implementations).
BUILD.lark files are intentionally **declarative**: they say *what* to build,
not *how* to build it.

Here is a complete, minimal BUILD.lark file:

```starlark
load("//prelude:python_rules.star", "python_library")

python_library(
    name = "directed-graph",
    srcs = ["src/**/*.py"],
    test_srcs = ["tests/**/*.py"],
    deps = ["//python/state-machine"],
    external_deps = [
        "pytest>=7.0",
        "pytest-cov>=4.0",
    ],
    test_runner = "pytest",
)
```

Let's break this down line by line:

- `load("//prelude:python_rules.star", "python_library")` — Import the
  `python_library` function from the prelude. The `//` prefix means "repo root".
  The prelude is a collection of `.star` files that define the standard rules.

- `python_library(...)` — Invoke the rule, passing keyword arguments that
  describe this particular package. Each argument is called an **attribute**.

A single BUILD.lark file can declare multiple targets:

```starlark
load("//prelude:python_rules.star", "python_library")
load("//prelude:python_rules.star", "python_binary")

python_library(
    name = "arithmetic",
    srcs = ["src/**/*.py"],
    deps = [],
)

python_binary(
    name = "arithmetic-cli",
    main = "src/cli.py",
    deps = [":arithmetic"],  # relative dep — same directory
)
```

## 4. The Prelude (Standard Rule Library)

The **prelude** is the standard library of build rules. It lives at
`code/packages/starlark/prelude/` and contains:

```
prelude/
  defs.star              # Core primitives (injected by build tool)
  python_rules.star      # python_library, python_binary
  go_rules.star          # go_library, go_binary
  typescript_rules.star  # typescript_library, typescript_binary
  rust_rules.star        # rust_library, rust_binary
  ruby_rules.star        # ruby_library, ruby_binary
  elixir_rules.star      # elixir_library, elixir_binary
  java_rules.star        # java_library, java_binary
  kotlin_rules.star      # kotlin_library, kotlin_binary
  swift_rules.star       # swift_library, swift_binary
```

### 4.1. defs.star — Core Primitives

`defs.star` is special. The build tool **injects** its symbols into every
Starlark evaluation as built-in globals. You never need to `load()` it
explicitly (though doing so is harmless). It provides the foundational
building blocks that all rules are made from.

Think of `defs.star` as the "kernel" of the build system. Everything else —
every language rule, every custom rule — is built on top of these primitives:

| Primitive            | Purpose                                        |
|----------------------|------------------------------------------------|
| `rule(impl, attrs)`  | Define a new build rule                        |
| `provider(fields)`   | Define a structured data type for cross-rule communication |
| `struct(**kwargs)`    | Create an immutable record                     |
| `attr` module         | Attribute type constructors for rule schemas   |
| `select(conditions)`  | Platform-conditional values                    |
| `cmd(program, args)`  | Construct a shell command as structured data   |
| `glob(patterns)`      | Match files by pattern                         |
| `DefaultInfo`         | Built-in provider carrying command lists       |
| `package_name()`      | Returns the current package's label path       |
| `repository_name()`   | Returns the repository name (always `""` here) |

#### The `attr` module

The `attr` module provides type constructors for rule attributes. Each
constructor validates its input at evaluation time, producing clear error
messages when a BUILD.lark file passes the wrong type:

```starlark
attr.string(mandatory = True)           # A required string
attr.string(default = "pytest")         # Optional string with default
attr.string_list(default = [])          # List of strings
attr.label_list(default = [])           # List of dependency labels
attr.bool(default = False)              # Boolean flag
attr.int(default = 0)                   # Integer
attr.string_dict(default = {})          # Dict[str, str]
```

When a user writes `test_runner = 42` in a BUILD.lark file but the rule
declares `attr.string()`, the Starlark VM produces:

```
BUILD.lark:5:17: error in python_library: attribute "test_runner"
  expected string, got int (42)
```

This is a key advantage over shell BUILD files, which silently accept any
value and fail at runtime with cryptic messages.

## 5. Defining Rules (Buck2-Style)

This is where BUILD.lark gets interesting. The build tool itself has **zero
knowledge** of any programming language. It does not know what Python is, what
`uv` does, or how Go modules work. All of that knowledge lives in `.star`
files that anyone can read, modify, and extend.

This is the Buck2 model: the build tool is a generic engine that evaluates
Starlark, resolves dependencies, and executes commands. The rules tell it
what commands to run.

### 5.1. Anatomy of a rule definition

A rule definition has two parts:

1. An **implementation function** that receives a context object and returns
   a list of providers (structured data including commands to execute).
2. A **rule declaration** that binds the implementation to an attribute schema.

Here is the complete definition of `python_library` from
`prelude/python_rules.star`:

```starlark
load("//prelude:defs.star", "rule", "provider", "select", "cmd", "attr")

# --- Providers -----------------------------------------------------------
#
# A "provider" is a named bundle of data that one rule passes to another.
# Think of it like a typed struct: it has a fixed set of fields, and the
# build tool can inspect it to wire up dependencies.

PythonInfo = provider(fields = ["interpreter", "site_packages"])

# --- Implementation ------------------------------------------------------
#
# The implementation function is called once per target. It receives a
# "context" object (ctx) whose .attrs field holds the evaluated attributes
# from the BUILD.lark invocation.
#
# Its job: produce a list of commands (the "build actions") and return
# one or more providers to communicate results to dependent targets.

def _python_library_impl(ctx):
    commands = []

    # Step 1: Create an isolated virtual environment.
    # The --clear flag ensures we start fresh every time.
    commands.append(cmd("uv", ["venv", "--quiet", "--clear"]))

    # Step 2: Install internal (monorepo) dependencies.
    # Each dep is a label like "//python/state-machine" that resolves
    # to a path. We install them as editable packages so that changes
    # propagate without re-installing.
    for dep in ctx.attrs.deps:
        commands.append(cmd("uv", [
            "pip", "install", "-e", dep.path, "--quiet",
        ]))

    # Step 3: Install the package itself (with dev extras).
    commands.append(cmd("uv", ["pip", "install", "-e", ".[dev]", "--quiet"]))

    # Step 4: Run tests.
    # Here is where select() shines — one rule, all platforms.
    if ctx.attrs.test_runner == "pytest":
        commands.append(select({
            "windows": cmd("uv", [
                "run", "--no-project", "python", "-m", "pytest",
                "tests/", "-v",
            ]),
            "default": cmd(".venv/bin/python", [
                "-m", "pytest", "tests/", "-v",
            ]),
        }))

    # Return providers. Every rule MUST return DefaultInfo.
    # Additional providers (like PythonInfo) are optional and allow
    # dependent rules to access structured data about this target.
    return [
        DefaultInfo(commands = commands),
        PythonInfo(
            interpreter = ".venv/bin/python",
            site_packages = ".venv/lib/python3.12/site-packages",
        ),
    ]

# --- Rule declaration -----------------------------------------------------
#
# The rule() call binds our implementation function to a named schema.
# When someone writes python_library(...) in BUILD.lark, the build tool:
#   1. Validates kwargs against this attr schema
#   2. Resolves label_list attrs to actual package paths
#   3. Calls _python_library_impl(ctx) with the validated attributes
#   4. Collects the returned providers

python_library = rule(
    implementation = _python_library_impl,
    attrs = {
        "name":          attr.string(mandatory = True),
        "srcs":          attr.string_list(default = ["src/**/*.py"]),
        "test_srcs":     attr.string_list(default = ["tests/**/*.py"]),
        "deps":          attr.label_list(default = []),
        "external_deps": attr.string_list(default = []),
        "test_runner":   attr.string(default = "pytest"),
    },
)
```

### 5.2. The ctx object

The context object passed to every implementation function has this shape:

```
ctx
 +-- attrs           # Evaluated attributes from BUILD.lark
 |    +-- name       # str: target name
 |    +-- srcs       # list[str]: source file patterns
 |    +-- deps       # list[Label]: resolved dependency labels
 |    +-- ...        # (other attrs as declared)
 +-- label           # Label: this target's fully qualified label
 +-- package_dir     # str: absolute path to this package's directory
```

`ctx.attrs.deps` deserves special attention. In the BUILD.lark file, the user
writes strings like `"//python/state-machine"`. By the time the implementation
function sees them, they have been resolved to `Label` objects with a `.path`
attribute pointing to the dependency's directory on disk.

## 6. User-Defined Rules

The prelude covers common cases, but a monorepo inevitably has domain-specific
build needs: protocol buffers, code generation, grammar compilation, benchmark
harnesses. Users define custom rules in their own `.star` files using the same
`rule()` primitive.

Here is a custom rule for compiling `.proto` files to Python:

```starlark
# code/packages/starlark/custom/protobuf_rules.star

load("//prelude:defs.star", "rule", "cmd", "attr")

def _protobuf_library_impl(ctx):
    return [DefaultInfo(commands = [
        cmd("protoc", [
            "--python_out=" + ctx.attrs.out_dir,
            ctx.attrs.proto_file,
        ]),
    ])]

protobuf_library = rule(
    implementation = _protobuf_library_impl,
    attrs = {
        "name":       attr.string(mandatory = True),
        "proto_file": attr.string(mandatory = True),
        "out_dir":    attr.string(default = "src/generated"),
    },
)
```

And here is how you would use it in a BUILD.lark file:

```starlark
load("//custom:protobuf_rules.star", "protobuf_library")

protobuf_library(
    name = "messages",
    proto_file = "messages.proto",
)
```

Another example — a rule for compiling `.grammar` files using the project's
own grammar_tools package:

```starlark
load("//prelude:defs.star", "rule", "cmd", "attr")

def _grammar_compile_impl(ctx):
    return [DefaultInfo(commands = [
        cmd(".venv/bin/python", [
            "-m", "grammar_tools", "compile",
            ctx.attrs.grammar_file,
            "--output", ctx.attrs.output,
        ]),
    ])]

grammar_compile = rule(
    implementation = _grammar_compile_impl,
    attrs = {
        "name":         attr.string(mandatory = True),
        "grammar_file": attr.string(mandatory = True),
        "output":       attr.string(mandatory = True),
        "deps":         attr.label_list(default = []),
    },
)
```

The key insight: **the build tool never needs to learn about new languages or
tools**. Users teach it by writing `.star` files. This is the same extensibility
model that makes Buck2 and Bazel successful at scale.

## 7. select() -- Platform Conditionals

`select()` is the mechanism that eliminates `BUILD_windows` files entirely. It
takes a dictionary mapping platform keys to values and returns a **lazy Select
object** that is resolved at execution time.

### 7.1. Syntax

```starlark
select({
    "windows": value_for_windows,
    "macos":   value_for_macos,
    "linux":   value_for_linux,
    "unix":    value_for_macos_and_linux,
    "default": fallback_value,
})
```

### 7.2. Resolution

During Starlark evaluation, `select()` does NOT pick a value. It returns a
`Select` object that remembers all the options. Later, when the build tool
processes the collected commands, it resolves each `Select` against the current
platform (`runtime.GOOS` in Go):

```
runtime.GOOS     Matches keys
────────────     ─────────────
"windows"    →   "windows"
"darwin"     →   "macos", "unix"
"linux"      →   "linux", "unix"
(anything)   →   "default"
```

If no key matches the current platform and there is no `"default"` key, the
build tool produces an error:

```
BUILD.lark:12: error: select() has no matching condition for platform "freebsd"
  and no "default" branch. Available conditions: windows, macos, linux
```

### 7.3. Concatenation

Select objects support concatenation with `+`, allowing you to combine
platform-specific and common values:

```starlark
# Platform-specific flags + common flags shared by all platforms
test_flags = select({
    "windows": ["--no-color"],
    "default": ["--color=yes"],
}) + ["-v", "--tb=short"]

# Result on Linux: ["--color=yes", "-v", "--tb=short"]
# Result on Windows: ["--no-color", "-v", "--tb=short"]
```

### 7.4. Where select() can appear

`select()` can appear anywhere a value is expected:

- As an attribute value: `test_runner = select({...})`
- Inside `cmd()`: `cmd(select({...}), args)`
- In a list: `[select({...}), "other"]`
- As a command argument: `cmd("prog", [select({...})])`

## 8. cmd() -- Command Construction

`cmd()` constructs a shell command as structured data rather than a raw string.
This is critical for cross-platform correctness.

### 8.1. Syntax

```starlark
cmd("program", ["arg1", "arg2", "arg with spaces"])
```

### 8.2. Return value

`cmd()` returns an immutable struct:

```python
{"type": "cmd", "program": "program", "args": ["arg1", "arg2", "arg with spaces"]}
```

### 8.3. Why structured commands matter

When the build tool renders this to a shell command, it handles quoting
correctly for each platform:

```
Unix:      program arg1 arg2 'arg with spaces'
Windows:   program arg1 arg2 "arg with spaces"
```

If you wrote raw shell strings in your BUILD.lark, you would need `select()`
everywhere just for quoting differences. With `cmd()`, the quoting is automatic.

### 8.4. Environment variables

`cmd()` accepts an optional `env` parameter for setting environment variables:

```starlark
cmd("cargo", ["build", "--release"], env = {
    "RUSTFLAGS": "-C target-cpu=native",
})
```

### 8.5. Working directory

By default, commands run in the sandbox copy of the package directory. To run
in a subdirectory:

```starlark
cmd("make", ["all"], cwd = "native/gf256_native")
```

## 9. glob() -- File Matching

`glob()` matches files against patterns **at evaluation time** (when the
Starlark VM processes the BUILD.lark file). It returns a list of file paths
relative to the package directory.

### 9.1. Syntax

```starlark
glob(["src/**/*.py", "!src/**/__pycache__/**"])
```

### 9.2. Pattern syntax

| Pattern     | Meaning                                  |
|-------------|------------------------------------------|
| `*`         | Any sequence of characters (not `/`)     |
| `**`        | Any number of directories (recursive)    |
| `?`         | Any single character                     |
| `!pattern`  | Exclude files matching pattern           |

### 9.3. Examples

```starlark
glob(["src/**/*.py"])                          # All Python files under src/
glob(["**/*.ex", "!**/test/**"])               # Elixir files, excluding tests
glob(["native/**/*.rs", "native/**/Cargo.toml"]) # Rust sources + manifests
```

`glob()` is typically used in `srcs` attributes, but since it returns a plain
list, it can be used anywhere a string list is expected.

## 10. Labels -- Dependency References

Labels are the universal naming scheme for targets. They identify a package
(and optionally a specific target within it) using a path-like syntax.

### 10.1. Syntax

```
//language/package-name         Absolute label (from repo root)
//language/package-name:target  Absolute label with explicit target
:target                         Relative label (same package)
```

### 10.2. Resolution

The build tool resolves labels to filesystem paths:

```
//python/state-machine  →  code/packages/python/state_machine/
//go/build-tool         →  code/programs/go/build-tool/
//elixir/arithmetic     →  code/packages/elixir/arithmetic/
```

The resolution algorithm:
1. Strip the `//` prefix
2. Split on `/` to get `[language, package-name]`
3. Search `code/packages/<language>/<package>/` and
   `code/programs/<language>/<package>/` for a BUILD.lark file

### 10.3. Usage in deps

```starlark
python_library(
    name = "topological-sort",
    deps = [
        "//python/state-machine",     # absolute: another package
        "//python/directed-graph",    # absolute: another package
        ":utils",                      # relative: target in same dir
    ],
)
```

## 11. Providers -- Cross-Rule Data Flow

Providers are the mechanism for passing structured data between rules. When
rule A depends on rule B, rule A's implementation can access the providers
that rule B's implementation returned.

### 11.1. Defining a provider

```starlark
PythonInfo = provider(fields = ["interpreter", "site_packages"])
GoInfo = provider(fields = ["go_binary", "go_path"])
RustInfo = provider(fields = ["cargo_target_dir", "edition"])
```

### 11.2. Returning providers

Every rule implementation MUST return a list containing at least `DefaultInfo`:

```starlark
def _my_rule_impl(ctx):
    return [
        DefaultInfo(commands = [...]),           # Required
        PythonInfo(interpreter = ".venv/bin/python"),  # Optional
    ]
```

### 11.3. Reading providers from dependencies

In a rule that depends on a Python library, you can access the `PythonInfo`
provider to discover where the interpreter lives:

```starlark
def _python_binary_impl(ctx):
    # Get PythonInfo from the first dependency that provides it
    for dep in ctx.attrs.deps:
        if PythonInfo in dep:
            interp = dep[PythonInfo].interpreter
            break

    return [DefaultInfo(commands = [
        cmd(interp, [ctx.attrs.main]),
    ])]
```

This is how rules compose without the build tool knowing anything about
Python, Go, or any other language. The rules communicate through providers.

## 12. File Priority

During migration, both legacy and new BUILD formats will coexist. The build
tool checks for build files in this order:

```
Priority    File             Evaluation Method
────────    ────             ─────────────────
1 (best)    BUILD.lark       Starlark VM evaluation
2           BUILD_<platform> Shell execution (legacy)
3           BUILD            Shell execution (legacy)
```

Rules for migration:

1. When a package gets a BUILD.lark file, the legacy BUILD and BUILD_windows
   files are deleted **in the same commit**. No ambiguity about which file
   the build tool should use.

2. The build tool logs a deprecation warning when it falls back to legacy
   shell execution:

   ```
   WARN: code/packages/python/foo/ using legacy BUILD (shell).
         Consider migrating to BUILD.lark.
   ```

3. Once all packages have migrated, BUILD.lark files are renamed to BUILD
   and the legacy shell execution path is removed from the build tool.

## 13. Examples

This section provides complete BUILD.lark examples for every language in the
monorepo. Each example is a realistic package, not a toy.

### 13.1. Python

```starlark
load("//prelude:python_rules.star", "python_library")

python_library(
    name = "directed-graph",
    srcs = ["src/**/*.py"],
    test_srcs = ["tests/**/*.py"],
    deps = [
        "//python/state-machine",
    ],
    external_deps = [
        "pytest>=7.0",
        "pytest-cov>=4.0",
    ],
    test_runner = "pytest",
)
```

### 13.2. Go

```starlark
load("//prelude:go_rules.star", "go_library", "go_binary")

go_library(
    name = "directed-graph",
    srcs = ["**/*.go"],
    test_srcs = ["**/*_test.go"],
    deps = [
        "//go/state-machine",
    ],
)

go_binary(
    name = "build-tool",
    srcs = ["**/*.go"],
    main = "main.go",
    deps = [
        "//go/directed-graph",
    ],
)
```

### 13.3. TypeScript

```starlark
load("//prelude:typescript_rules.star", "typescript_library")

typescript_library(
    name = "directed-graph",
    srcs = ["src/**/*.ts"],
    test_srcs = ["tests/**/*.test.ts"],
    deps = [
        "//typescript/state-machine",
    ],
    external_deps = {
        "vitest": "^1.0.0",
    },
    test_runner = "vitest",
)
```

### 13.4. Rust

```starlark
load("//prelude:rust_rules.star", "rust_library")

rust_library(
    name = "gf256",
    srcs = ["src/**/*.rs"],
    test_srcs = ["tests/**/*.rs"],
    edition = "2021",
    deps = [],
    features = ["default"],
)
```

### 13.5. Ruby

```starlark
load("//prelude:ruby_rules.star", "ruby_library")

ruby_library(
    name = "directed-graph",
    srcs = ["lib/**/*.rb"],
    test_srcs = ["test/**/*_test.rb"],
    deps = [
        "//ruby/state-machine",
    ],
    external_deps = [
        "minitest>=5.0",
    ],
    test_runner = "minitest",
)
```

### 13.6. Elixir

```starlark
load("//prelude:elixir_rules.star", "elixir_library")

elixir_library(
    name = "arithmetic",
    srcs = ["lib/**/*.ex"],
    test_srcs = ["test/**/*_test.exs"],
    deps = [],
    mix_deps = {},
)
```

### 13.7. Java

```starlark
load("//prelude:java_rules.star", "java_library")

java_library(
    name = "directed-graph",
    srcs = ["src/**/*.java"],
    test_srcs = ["test/**/*.java"],
    deps = [
        "//java/state-machine",
    ],
    java_version = "21",
    test_runner = "junit5",
)
```

### 13.8. Kotlin

```starlark
load("//prelude:kotlin_rules.star", "kotlin_library")

kotlin_library(
    name = "directed-graph",
    srcs = ["src/**/*.kt"],
    test_srcs = ["test/**/*.kt"],
    deps = [
        "//kotlin/state-machine",
    ],
    kotlin_version = "2.0",
    test_runner = "junit5",
)
```

### 13.9. Swift

```starlark
load("//prelude:swift_rules.star", "swift_library")

swift_library(
    name = "directed-graph",
    srcs = ["Sources/**/*.swift"],
    test_srcs = ["Tests/**/*.swift"],
    deps = [
        "//swift/state-machine",
    ],
    swift_version = "5.10",
)
```

## 14. Summary of Changes from Shell BUILD Files

| Aspect                | Shell BUILD         | BUILD.lark                      |
|-----------------------|---------------------|---------------------------------|
| Language              | bash/zsh/cmd        | Starlark (deterministic, safe)  |
| Platform handling     | Separate files      | `select()` inline               |
| Dependencies          | Manual `cd && install` | Declared `deps = [...]`      |
| Inputs                | Implicit (any file) | Declared `srcs = [...]`         |
| Commands              | Raw shell strings   | `cmd()` structured data         |
| Reuse                 | Copy-paste          | `load()` shared rules           |
| Validation            | Runtime errors      | Attribute type checking         |
| Extension             | Not possible        | User-defined `.star` rules      |
| File patterns         | Shell globs         | `glob()` with `**` and `!`     |
| Cross-rule data       | None                | Providers                       |

## 15. Open Questions

These items are deferred to future specs or implementation:

1. **Remote caching** — Should the build tool support a shared cache (like
   Buck2's RE)? For now, local-only is fine.

2. **Test result providers** — Should rules return structured test results
   (pass/fail counts, timing) via providers? This would enable a dashboard.

3. **Lockfile generation** — Should `external_deps` produce lockfiles
   automatically, or is that the user's responsibility?

4. **Multi-target packages** — The current build tool assumes one target per
   package directory. BUILD.lark supports multiple targets per file. The
   dependency graph needs to be updated to handle this.

5. **Transition from mise** — Currently, language runtimes are managed by
   mise. Should BUILD.lark declare the required runtime version, and should
   the build tool invoke mise automatically?
