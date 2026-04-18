# Scaffold Generator — Package Scaffolding with a Dart Bootstrap Lane

A CLI tool, powered by CLI Builder, that generates correct-by-construction,
CI-ready package scaffolding across the monorepo's implementation languages.
The first published versions covered Python, Go, Ruby, Rust, TypeScript, and
Elixir; this spec now also defines a Dart bootstrap implementation focused on
generating Dart packages and programs correctly.

---

## 1. Overview

### 1.1 The Problem

The coding-adventures monorepo contains 400+ packages across many languages.
Every package follows a precise file structure, and getting any detail wrong
causes CI failures. The `lessons.md` file documents at least twelve recurring
categories of failure that all stem from the same root cause: **agents
hand-crafting package scaffolding from scratch**.

Here is a (non-exhaustive) catalogue of these failures:

| # | Failure Category | Languages Affected | Times Documented |
|---|------------------|--------------------|------------------|
| 1 | Missing BUILD files | All | 4 |
| 2 | BUILD files missing transitive dependency installs | Python, Ruby, Go, TS | 3 |
| 3 | TypeScript `main` field pointing to `dist/` instead of `src/` | TypeScript | 2 |
| 4 | Missing `@vitest/coverage-v8` in devDependencies | TypeScript | 2 |
| 5 | TypeScript BUILD not chain-installing `file:` deps | TypeScript | 3 |
| 6 | Ruby require ordering (deps before own modules) | Ruby | 2 |
| 7 | Rust workspace `Cargo.toml` not updated with new crate | Rust | 2 |
| 8 | Go `go mod tidy` not run in transitively dependent packages | Go | 1 |
| 9 | Elixir reserved words used as variable names | Elixir | 1 |
| 10 | Missing README.md or CHANGELOG.md | All | 1 |
| 11 | Ruby `include` inside method body instead of class level | Ruby | 1 |
| 12 | Python Enum constructed with invalid integer | Python | 1 |

Every one of these failures is preventable. A scaffold generator that produces
the correct structure — with the correct BUILD file, the correct metadata, the
correct test stubs, the correct dependency wiring — eliminates the entire
category.

### 1.2 The Solution

`scaffold-generator` is a CLI tool that takes a package name and a set of
options, and generates a complete, ready-to-build package directory. The
generated package:

- **Compiles** — all metadata files are syntactically valid
- **Lints** — generated code passes ruff (Python), go vet (Go), standardrb (Ruby)
- **Tests pass** — a minimal test suite verifies the package loads correctly
- **CI-ready** — the BUILD file installs all transitive dependencies in the
  correct order and runs the test suite
- **Documented** — README.md, CHANGELOG.md, and literate code comments are
  included from the start

### 1.3 Design Principles

1. **Correct by construction** — if the tool runs successfully, the output
   passes CI. No manual fixups required.
2. **Dependency-aware** — the tool reads existing packages' metadata to compute
   transitive dependency closures and topological install orders automatically.
3. **Convention over configuration** — the tool encodes every convention from
   this repository (naming, file layout, BUILD patterns) so that agents don't
   need to remember them.
4. **Built on CLI Builder** — the tool's own interface is defined in a JSON
   spec file, consistent with how all CLI tools in this repo are built.
5. **One tool, multiple implementation lanes** — the same conceptual tool is
   implemented in multiple languages, following the same pattern as `pwd`.

### 1.3.1 Dart Bootstrap Phase

The Dart implementation is intentionally narrower than the older cross-language
generators. Its near-term job is to unblock future Dart package work so agents
do not hand-craft `pubspec.yaml`, `BUILD`, `README.md`, `CHANGELOG.md`,
`.gitignore`, `lib/`, `bin/`, and `test/` layouts from memory.

In this bootstrap phase:

- the Dart program supports scaffolding **Dart libraries** under
  `code/packages/dart/`
- it also supports scaffolding **Dart programs** under `code/programs/dart/`
- its local CLI spec accepts `dart` explicitly and may treat `all` as the
  Dart bootstrap lane for compatibility with the broader scaffold-generator
  interface
- it reads existing Dart `pubspec.yaml` files to validate direct dependencies
  and compute transitive Dart dependency closures

### 1.4 How Agents Should Use This Tool

Instead of creating package files by hand, agents should:

```
# Scaffold a library that depends on arithmetic and logic-gates:
scaffold-generator my-package \
  --language python \
  --depends-on arithmetic,logic-gates \
  --description "A new package that does something useful"

# Then fill in the actual implementation
```

The agent's job shifts from "create all the boilerplate correctly" to "fill in
the business logic and tests." This is the same separation of concerns that
CLI Builder provides for argument parsing.

---

## 2. CLI Interface

### 2.1 Synopsis

```
scaffold-generator <PACKAGE_NAME> [options]

Options:
  -t, --type <TYPE>           Package type: "library" or "program" (default: library)
  -l, --language <LANG>       Target language(s), comma-separated or "all" (default: all)
  -d, --depends-on <DEPS>     Comma-separated sibling package names
      --layer <N>             Layer number for README context
      --description <TEXT>    One-line description of the package
      --dry-run               Print what would be generated without writing files
      --help                  Show help text
      --version               Show version
```

### 2.2 Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `PACKAGE_NAME` | Yes | The package name in **kebab-case** (e.g., `my-package`). Lowercase letters, digits, and hyphens only. No leading/trailing/consecutive hyphens. |

### 2.3 Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--type` / `-t` | enum | `library` | `library` → `code/packages/<lang>/`, `program` → `code/programs/<lang>/` |
| `--language` / `-l` | string | implementation-defined | The accepted language set is implementation-specific. The Dart bootstrap implementation accepts `dart` and `all`, where `all` resolves to the Dart lane. |
| `--depends-on` / `-d` | string | (none) | Comma-separated list of sibling package names (in kebab-case). These are direct dependencies. The tool computes transitive dependencies automatically. |
| `--layer` | integer | (none) | Layer number in the computing stack (e.g., 1 for logic-gates, 2 for arithmetic). Used in README context. |
| `--description` | string | (none) | One-line description. Used in metadata files and README. |
| `--dry-run` | boolean | false | Print the file tree and file contents that *would* be generated, without actually writing anything to disk. |

### 2.4 The JSON Spec

The CLI interface is declared in `scaffold-generator.json`. Mature
implementations may share one root JSON spec; bootstrap implementations may
carry a local variant as long as they preserve the same flag names and core
semantics (same pattern as `pwd.json`):

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "scaffold-generator",
  "display_name": "Scaffold Generator",
  "description": "Generate CI-ready package scaffolding for the coding-adventures monorepo",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "type",
      "short": "t",
      "long": "type",
      "description": "Package type: 'library' goes in code/packages/, 'program' goes in code/programs/",
      "type": "enum",
      "enum_values": ["library", "program"],
      "default": "library"
    },
    {
      "id": "language",
      "short": "l",
      "long": "language",
      "description": "Target language(s). The accepted set is implementation-specific; the Dart bootstrap lane accepts 'dart' and 'all'",
      "type": "string",
      "default": "all"
    },
    {
      "id": "depends-on",
      "short": "d",
      "long": "depends-on",
      "description": "Comma-separated list of sibling package names this package depends on (kebab-case)",
      "type": "string",
      "default": ""
    },
    {
      "id": "layer",
      "long": "layer",
      "description": "Layer number for README context (e.g., 1 for logic-gates, 2 for arithmetic)",
      "type": "integer"
    },
    {
      "id": "description",
      "long": "description",
      "description": "One-line description of the package",
      "type": "string",
      "default": ""
    },
    {
      "id": "dry-run",
      "long": "dry-run",
      "description": "Print what would be generated without writing files",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "package-name",
      "name": "PACKAGE_NAME",
      "description": "Name of the package in kebab-case (e.g., 'my-package')",
      "type": "string",
      "required": true
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": []
}
```

### 2.5 Input Validation

Before generating any files, the tool validates:

1. **Package name format** — must match `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`.
   Reject names with uppercase, underscores, leading/trailing hyphens, or
   consecutive hyphens.

2. **Language values** — each comma-separated value must be recognized by the
   active implementation. The Dart bootstrap implementation accepts `dart` and
   `all`.

3. **Dependencies exist** — for each dependency in `--depends-on`, verify that
   the directory `code/packages/<lang>/<dep>` (or `code/programs/<lang>/<dep>`)
   exists for every target language. If a dependency is missing for a specific
   language, report which language and package are missing and abort.

4. **Target directory does not exist** — refuse to overwrite. If the directory
   already exists, print an error and exit with code 1.

### 2.6 Exit Status

| Code | Meaning |
|------|---------|
| 0 | All packages scaffolded successfully |
| 1 | Validation error (bad name, missing dep, target exists) |
| 2 | I/O error (cannot create directory, cannot write file) |

---

## 3. Name Normalization

The input `PACKAGE_NAME` is always kebab-case. Each language has different
conventions for directory names, code identifiers, and package registry names.
The scaffold generator must convert the input name correctly for each context.

### 3.1 Conversion Functions

```
to_snake_case("my-package")   → "my_package"    # replace hyphens with underscores
to_camel_case("my-package")   → "MyPackage"     # capitalize each segment
to_joined_lower("my-package") → "mypackage"     # remove hyphens, no underscores
```

### 3.2 Language-Specific Names

Given the input `my-package`:

| Context | Python | Ruby | Go | TypeScript | Rust | Elixir | Dart |
|---------|--------|------|----|-----------|------|--------|------|
| **Directory name** | `my-package` | `my_package` | `my-package` | `my-package` | `my-package` | `my_package` | `my-package` |
| **Package/crate/gem/app name** | `coding-adventures-my-package` | `coding_adventures_my_package` | (module path) | `@coding-adventures/my-package` | `my-package` | `:coding_adventures_my_package` | `coding_adventures_my_package` for libraries, `my_package` for programs |
| **Module/namespace** | `my_package` | `CodingAdventures::MyPackage` | `mypackage` | (ESM exports) | `my_package` | `CodingAdventures.MyPackage` | top-level library file `lib/my_package.dart` |
| **Import in code** | `from my_package import ...` | `require "coding_adventures_my_package"` | `import mypackage` | `import { ... } from "@coding-adventures/my-package"` | `use my_package::...` | `alias CodingAdventures.MyPackage` | `import 'package:coding_adventures_my_package/my_package.dart';` for libraries |
| **Source directory** | `src/my_package/` | `lib/coding_adventures/my_package/` | (flat) | `src/` | `src/` | `lib/coding_adventures/my_package/` | `lib/` and `lib/src/`, plus `bin/` for programs |
| **Test file** | `tests/test_my_package.py` | `test/test_my_package.rb` | `my_package_test.go` | `tests/my-package.test.ts` | `tests/my_package_test.rs` | `test/my_package_test.exs` | `test/my_package_test.dart` |

### 3.3 Dependency Name Normalization

When a dependency name (also kebab-case) appears in a language-specific
context, it must be normalized the same way. For example, `--depends-on logic-gates`:

| Language | How the dependency appears |
|----------|--------------------------|
| Python | `-e ../logic-gates` in BUILD, `from logic_gates import ...` in code |
| Ruby | `gem "coding_adventures_logic_gates", path: "../logic_gates"` in Gemfile |
| Go | `github.com/.../go/logic-gates` in go.mod, `import logicgates` in code |
| TypeScript | `"@coding-adventures/logic-gates": "file:../logic-gates"` in package.json |
| Rust | `logic-gates = { path = "../logic-gates" }` in Cargo.toml |
| Elixir | `{:coding_adventures_logic_gates, path: "../logic_gates"}` in mix.exs |

---

## 4. Dependency Resolution

This is the most critical feature of the scaffold generator. Missing or
misordered transitive dependencies in BUILD files are the single most
common cause of CI failures in this repository.

### 4.1 The Problem

Consider a dependency chain: `A → B → C → D` (A depends on B, B on C, C on D).

When CI runs the BUILD file for package A, it starts with a completely clean
environment — no cached installs, no pre-existing `node_modules`, no virtual
environments. The BUILD file must explicitly install D, then C, then B, then A,
in that order. Missing any link in the chain causes a build failure that only
manifests in CI, never locally (because local development has cached state).

### 4.2 Reading Existing Dependencies

The scaffold generator reads existing packages' metadata files to discover
their dependencies. Each language uses a different metadata format:

| Language | Metadata File | Dependency Extraction |
|----------|--------------|----------------------|
| Python | BUILD file | Parse `-e ../` entries from `uv pip install` lines |
| Ruby | Gemfile | Lines matching `gem "coding_adventures_*", path: "../*"` |
| Go | go.mod | `replace` directives with `=> ../` targets |
| TypeScript | package.json | `dependencies` entries with `"file:../"` values |
| Rust | Cargo.toml | `[dependencies]` entries with `path = "../"` values |
| Elixir | mix.exs | Entries in `deps/0` with `path: "../"` |

**Why read BUILD files for Python?** Python's `pyproject.toml` lists PyPI
dependencies, not local sibling packages. The sibling packages are installed
via `-e ../sibling` flags in the BUILD file's `uv pip install` command. This
is the authoritative source of local dependencies for Python packages.

### 4.3 Computing the Transitive Closure

Given the direct dependencies from `--depends-on`, the tool performs a
breadth-first traversal to find all transitive dependencies:

```
function transitive_closure(direct_deps, language):
    visited = set()
    queue = list(direct_deps)

    while queue is not empty:
        dep = queue.pop_front()
        if dep in visited:
            continue
        visited.add(dep)
        dep_dir = resolve_package_dir(dep, language)
        dep_deps = read_dependencies(dep_dir, language)
        for d in dep_deps:
            if d not in visited:
                queue.append(d)

    return visited
```

### 4.4 Topological Sort for Install Order

After computing the transitive closure, the tool must determine the correct
install order. Dependencies that have no dependencies of their own (leaves)
must be installed first, working up to the direct dependencies last.

```
function install_order(all_deps, language):
    graph = {}
    for dep in all_deps:
        dep_deps = read_dependencies(dep, language)
        graph[dep] = [d for d in dep_deps if d in all_deps]

    return topological_sort(graph)  // Kahn's algorithm, leaves first
```

**Example:** If the dependency graph is:

```
my-package → arithmetic → logic-gates
my-package → cpu-simulator → arithmetic → logic-gates
                           → clock
```

The transitive closure is: `{arithmetic, logic-gates, cpu-simulator, clock}`

The topological install order is: `logic-gates, clock, arithmetic, cpu-simulator`

(logic-gates and clock are leaves — they have no dependencies within the set —
so they are installed first.)

### 4.5 Cycle Detection

If the dependency graph contains a cycle, the tool must detect it and report
an error rather than looping forever. Kahn's algorithm naturally detects
cycles: if the algorithm terminates with unvisited nodes, those nodes form
one or more cycles.

---

## 5. Generated File Templates

### 5.1 Python

**Target directory:** `code/packages/python/{package-name}/` (library) or
`code/programs/python/{package-name}/` (program)

**Files:**

#### `pyproject.toml`

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "coding-adventures-{package-name}"
version = "0.1.0"
description = "{description}"
requires-python = ">=3.12"
license = "MIT"
authors = [{ name = "Adhithya Rajasekaran" }]
readme = "README.md"

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-cov>=5.0", "ruff>=0.4", "mypy>=1.10"]

[tool.hatch.build.targets.wheel]
packages = ["src/{snake_name}"]

[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = ["E", "W", "F", "I", "UP", "B", "SIM", "ANN"]

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--cov={snake_name} --cov-report=term-missing --cov-fail-under=80"

[tool.coverage.run]
source = ["src/{snake_name}"]

[tool.coverage.report]
fail_under = 80
show_missing = true
```

#### `src/{snake_name}/__init__.py`

```python
"""{package-name} — {description}

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
{layer_context}
"""

__version__ = "0.1.0"
```

#### `tests/__init__.py`

Empty file.

#### `tests/test_{snake_name}.py`

```python
"""Tests for {package-name}."""

from {snake_name} import __version__


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"
```

#### `BUILD`

```bash
uv venv --quiet --clear
{for each dep in topological_install_order:}
uv pip install -e ../{dep} --quiet
{end for}
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

If there are no dependencies, the BUILD file simplifies to:

```bash
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

---

### 5.2 Ruby

**Target directory:** `code/packages/ruby/{snake_name}/`

Note: Ruby directories use snake_case, not kebab-case.

**Files:**

#### `coding_adventures_{snake_name}.gemspec`

```ruby
# frozen_string_literal: true

require_relative "lib/coding_adventures/{snake_name}/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_{snake_name}"
  spec.version       = CodingAdventures::{CamelName}::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "{description}"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }

  {for each direct_dep:}
  spec.add_dependency "coding_adventures_{dep_snake}", "~> 0.1"
  {end for}
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
```

#### `Gemfile`

```ruby
# frozen_string_literal: true

source "https://rubygems.org"
gemspec

# All transitive path dependencies must be listed here.
# Bundler needs to know where to find each gem locally.
{for each transitive_dep in all_transitive_deps:}
gem "coding_adventures_{dep_snake}", path: "../{dep_snake}"
{end for}
```

**Critical note:** The Gemfile must include ALL transitive dependencies, not
just direct ones. If `arithmetic` depends on `logic_gates`, and we depend on
`arithmetic`, the Gemfile must list both `arithmetic` and `logic_gates`.

#### `Rakefile`

```ruby
# frozen_string_literal: true

require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/test_*.rb"]
end

task default: :test
```

#### `lib/coding_adventures_{snake_name}.rb`

```ruby
# frozen_string_literal: true

# =========================================================================
# IMPORTANT: Require dependencies FIRST, before own modules.
# =========================================================================
#
# Ruby loads files in require order. If our modules reference constants
# from dependencies (e.g., CodingAdventures::Arithmetic::HalfAdder),
# those gems must be loaded before our own modules try to use them.
#
# This ordering rule is documented in lessons.md (2026-03-21) after
# repeated CI failures caused by `require_relative` loading modules
# that referenced not-yet-loaded dependency constants.
{for each direct_dep:}
require "coding_adventures_{dep_snake}"
{end for}

require_relative "coding_adventures/{snake_name}/version"

module CodingAdventures
  # {description}
  module {CamelName}
  end
end
```

#### `lib/coding_adventures/{snake_name}/version.rb`

```ruby
# frozen_string_literal: true

module CodingAdventures
  module {CamelName}
    VERSION = "0.1.0"
  end
end
```

#### `test/test_{snake_name}.rb`

```ruby
# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_{snake_name}"

class Test{CamelName} < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::{CamelName}::VERSION
  end
end
```

#### `BUILD`

```bash
bundle install --quiet
bundle exec rake test
```

Ruby's BUILD file is simpler than Python/TypeScript because Bundler reads the
Gemfile and resolves all path dependencies itself. The Gemfile is where the
transitive dependency work happens.

---

### 5.3 Go

**Target directory:** `code/packages/go/{package-name}/`

**Files:**

#### `go.mod`

```
module github.com/adhithyan15/coding-adventures/code/packages/go/{package-name}

go 1.26
{if has_deps:}

require (
{for each direct_dep:}
	github.com/adhithyan15/coding-adventures/code/packages/go/{dep} v0.0.0
{end for}
)

replace (
{for each transitive_dep:}
	github.com/adhithyan15/coding-adventures/code/packages/go/{dep} => ../{dep}
{end for}
)
{end if}
```

**Critical note:** The `replace` block must include ALL transitive
dependencies, not just direct ones. Go's module system requires every module
in the dependency graph to have a `replace` directive pointing to its local
path.

#### `{snake_name}.go`

```go
// Package {go_pkg_name} provides {description}.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
// {layer_context}
package {go_pkg_name}
```

Where `{go_pkg_name}` is the Go package name — typically the last segment
of the module path with hyphens removed (e.g., `my-package` → `mypackage`).

#### `{snake_name}_test.go`

```go
package {go_pkg_name}

import "testing"

func TestPackageLoads(t *testing.T) {
	t.Log("{package-name} package loaded successfully")
}
```

#### `BUILD`

```bash
go test ./... -v -cover
```

Go's BUILD file is simple because `go test` resolves local `replace`
directives automatically. However, **post-generation**, the tool must remind
the user to run `go mod tidy` in all packages that will transitively depend on
this new package.

---

### 5.4 TypeScript

**Target directory:** `code/packages/typescript/{package-name}/`

This is the most failure-prone language. Pay careful attention to every field.

**Files:**

#### `package.json`

```json
{
  "name": "@coding-adventures/{package-name}",
  "version": "0.1.0",
  "description": "{description}",
  "type": "module",
  "main": "src/index.ts",
  "scripts": {
    "build": "tsc",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage"
  },
  "author": "Adhithya Rajasekaran",
  "license": "MIT",
  "dependencies": {
    {for each direct_dep:}
    "@coding-adventures/{dep}": "file:../{dep}"
    {end for}
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "vitest": "^3.0.0",
    "@vitest/coverage-v8": "^3.0.0"
  }
}
```

**Three critical fields that have caused CI failures:**

1. **`"main": "src/index.ts"`** — MUST point to the TypeScript source, NOT
   `dist/index.js`. Vitest resolves `file:` dependencies using the `main`
   field and transforms TypeScript on the fly. If `main` points to `dist/`,
   resolution fails because we don't compile before testing.

2. **`"type": "module"`** — required for ESM imports/exports.

3. **`"@vitest/coverage-v8": "^3.0.0"`** — MUST be in devDependencies. This
   was missed on 5+ packages and caused coverage reporting failures in CI.

#### `tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true
  },
  "include": ["src"]
}
```

#### `vitest.config.ts`

```typescript
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
```

#### `src/index.ts`

```typescript
/**
 * @coding-adventures/{package-name}
 *
 * {description}
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 * {layer_context}
 */

export const VERSION = "0.1.0";
```

#### `tests/{package-name}.test.ts`

```typescript
import { describe, it, expect } from "vitest";
import { VERSION } from "../src/index.js";

describe("{package-name}", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });
});
```

#### `BUILD`

```bash
{for each dep in topological_install_order:}
cd ../{dep} && npm install --silent && \
{end for}
cd ../{package-name} && npm install --silent
npx vitest run --coverage
```

**Why chain-install?** TypeScript `file:` dependencies create symlinks to
sibling directories. When CI runs with a clean filesystem, the sibling's
`node_modules` is empty — its own `file:` dependencies are unresolved. We
must install from leaves to root so that each package's `node_modules` is
populated before any package that depends on it tries to resolve imports.

If there are no dependencies:

```bash
npm install --silent
npx vitest run --coverage
```

**Post-generation step:** Run `npm install` in the generated directory to
create `package-lock.json`. This file must be committed.

---

### 5.5 Rust

**Target directory:** `code/packages/rust/{package-name}/`

**Files:**

#### `Cargo.toml`

```toml
[package]
name = "{package-name}"
version = "0.1.0"
edition = "2021"
description = "{description}"

[dependencies]
{for each direct_dep:}
{dep} = { path = "../{dep}" }
{end for}
```

Rust's Cargo handles transitive dependency resolution automatically — only
direct dependencies need to be listed in `Cargo.toml`.

#### `src/lib.rs`

```rust
//! # {package-name}
//!
//! {description}
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.
//! {layer_context}

#[cfg(test)]
mod tests {
    #[test]
    fn it_loads() {
        assert!(true, "{package-name} crate loaded successfully");
    }
}
```

#### `BUILD`

```bash
cargo test -p {package-name} -- --nocapture
```

#### Post-Generation: Workspace `Cargo.toml` Update

**This is mandatory.** After generating the Rust package, the tool MUST either:

1. **Automatically** append `"{package-name}"` to the `members` list in
   `code/packages/rust/Cargo.toml`, OR
2. **Print a clear reminder** that the user must do this manually.

Option 1 is preferred. The tool should:
- Read `code/packages/rust/Cargo.toml`
- Find the `members = [...]` array
- Add `"{package-name}"` in alphabetical order
- Write the updated file

If the crate declares its own `[workspace]` (e.g., FFI bridge crates), it
must NOT be added to the parent workspace. The tool should check for this by
looking for `[workspace]` in the generated `Cargo.toml` — but since the
scaffold generator creates the file, it knows whether it added a workspace
declaration (it won't for standard packages).

**Why this matters:** Every Rust crate in the monorepo must be listed in the
workspace `Cargo.toml`. Missing entries cause "current package believes it's
in a workspace when it's not" errors that break ALL Rust packages in CI, not
just the new one.

---

### 5.6 Elixir

**Target directory:** `code/packages/elixir/{snake_name}/`

Note: Elixir directories use snake_case, not kebab-case.

**Files:**

#### `mix.exs`

```elixir
defmodule CodingAdventures.{CamelName}.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_{snake_name},
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {for each direct_dep:}
      {:coding_adventures_{dep_snake}, path: "../{dep_snake}"}
      {end for}
    ]
  end
end
```

#### `lib/coding_adventures/{snake_name}.ex`

```elixir
defmodule CodingAdventures.{CamelName} do
  @moduledoc """
  {description}

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  {layer_context}
  """
end
```

**Elixir reserved word warning:** The generated module and variable names must
not conflict with Elixir reserved words: `after`, `rescue`, `catch`, `else`,
`end`, `fn`, `do`, `when`, `cond`, `try`, `receive`. If the package name
contains any of these as a segment (e.g., `error-handler` contains no reserved
words, but a variable named `after` would), the tool should produce a warning.

#### `test/{snake_name}_test.exs`

```elixir
defmodule CodingAdventures.{CamelName}Test do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.{CamelName})
  end
end
```

#### `test/test_helper.exs`

```elixir
ExUnit.start()
```

#### `BUILD`

```bash
{for each dep in topological_install_order:}
cd ../{dep_snake} && mix deps.get --quiet && mix compile --quiet && \
{end for}
cd ../{snake_name} && mix deps.get --quiet && mix test --cover
```

If there are no dependencies:

```bash
mix deps.get --quiet && mix test --cover
```

---

### 5.7 Common Files (All Languages)

#### `README.md`

```markdown
# {package-name}

{description}

## Layer {layer}

This package is part of Layer {layer} of the coding-adventures computing stack.

## Where It Fits

```
[dependency diagram generated from --depends-on]
```

## Installation

{language-specific install instructions}

## Usage

```{language}
// TODO: Add usage examples after implementation
```

## Development

```bash
# Run tests
bash BUILD
```

## Spec

See [/code/specs/{spec-file}](/code/specs/{spec-file}) for the full
specification.
```

#### `CHANGELOG.md`

```markdown
# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - {YYYY-MM-DD}

### Added

- Initial package scaffolding generated by scaffold-generator
```

---

## 6. Post-Scaffold Output

After generating all files, the tool prints a summary and a language-specific
verification checklist:

```
✓ Scaffolded {package-name} for {language}

  Directory: code/packages/{language}/{dir-name}/
  Files created:
    {list of files, one per line}

  Post-generation steps:
    {language-specific items — see below}

  Verify with:
    cd code/packages/{language}/{dir-name} && bash BUILD
```

### 6.1 Language-Specific Post-Generation Steps

**Python:**
```
  (no additional steps required — BUILD file is self-contained)
```

**Ruby:**
```
  (no additional steps required — Gemfile lists all transitive deps)
```

**Go:**
```
  → Run: go mod tidy (in this package directory)
  → After other packages add this as a dependency, run go mod tidy in those too
```

**TypeScript:**
```
  → Run: npm install (to generate package-lock.json)
  → Commit package-lock.json alongside the package
```

**Rust:**
```
  → Added "{package-name}" to code/packages/rust/Cargo.toml workspace members
  → Run: cargo build --workspace (to verify the workspace compiles)
```

**Elixir:**
```
  (no additional steps required — BUILD file chain-installs deps)
```

---

## 7. Program Directory Structure

The scaffold generator itself is a program in the monorepo, implemented in all
six languages. It follows the same pattern as `unix-tools` (specifically `pwd`).

```
code/programs/
  scaffold-generator.json            ← Shared CLI spec (lives alongside pwd.json)

  python/scaffold-generator/
    BUILD
    CHANGELOG.md
    README.md
    pyproject.toml
    scaffold_generator.py            ← Entry point
    tests/
      test_scaffold_generator.py

  go/scaffold-generator/
    BUILD
    CHANGELOG.md
    README.md
    go.mod
    main.go                          ← Entry point
    main_test.go

  ruby/scaffold-generator/
    BUILD
    CHANGELOG.md
    README.md
    Gemfile
    Rakefile
    coding_adventures_scaffold_generator.gemspec
    lib/coding_adventures_scaffold_generator.rb
    test/test_scaffold_generator.rb

  typescript/scaffold-generator/
    BUILD
    CHANGELOG.md
    README.md
    package.json
    tsconfig.json
    vitest.config.ts
    src/index.ts                     ← Entry point
    tests/scaffold-generator.test.ts

  rust/scaffold-generator/
    BUILD
    CHANGELOG.md
    README.md
    Cargo.toml
    src/main.rs                      ← Entry point (binary crate)

  elixir/scaffold-generator/
    BUILD
    CHANGELOG.md
    README.md
    mix.exs
    lib/coding_adventures/scaffold_generator.ex
    test/scaffold_generator_test.exs
    test/test_helper.exs
```

Each implementation depends on its language's `cli-builder` package for argument
parsing. The `scaffold-generator.json` spec file is shared — each
implementation reads it relative to its own location (same pattern as `pwd.json`).

---

## 8. Core Algorithm

All six implementations follow the same algorithm:

```
function scaffold(argv):
    // Step 1: Parse arguments via CLI Builder
    result = Parser("scaffold-generator.json", argv).parse()
    if result is HelpResult:   print(result.text); exit(0)
    if result is VersionResult: print(result.version); exit(0)
    if result is ParseErrors:  print(errors); exit(1)

    // Step 2: Extract and validate inputs
    package_name = result.arguments["package-name"]
    validate_kebab_case(package_name)
    languages = parse_language_list(result.flags["language"])
    pkg_type = result.flags["type"]  // "library" or "program"
    direct_deps = parse_dep_list(result.flags["depends-on"])
    layer = result.flags["layer"]
    description = result.flags["description"]
    dry_run = result.flags["dry-run"]

    // Step 3: For each target language, generate the package
    for lang in languages:
        // 3a: Compute paths
        base_dir = repo_root / "code" / (pkg_type == "library" ? "packages" : "programs") / lang
        dir_name = normalize_directory_name(package_name, lang)
        target_dir = base_dir / dir_name

        // 3b: Validate
        if target_dir exists:
            error("Directory already exists: {target_dir}")
        for dep in direct_deps:
            dep_dir = base_dir / normalize_directory_name(dep, lang)
            if not dep_dir exists:
                error("Dependency {dep} not found for {lang} at {dep_dir}")

        // 3c: Resolve dependencies
        all_deps = transitive_closure(direct_deps, lang)
        ordered_deps = topological_sort(all_deps, lang)

        // 3d: Generate files
        if dry_run:
            print_file_tree(target_dir, lang, ...)
        else:
            create_directory(target_dir)
            write_files(target_dir, lang, package_name, description,
                        layer, direct_deps, all_deps, ordered_deps)

            // 3e: Language-specific post-generation
            if lang == "rust":
                update_workspace_cargo_toml(package_name)
            if lang == "typescript":
                run("npm install", cwd=target_dir)

        // 3f: Print summary
        print_summary(target_dir, lang)
```

---

## 9. Testing Strategy

### 9.1 Unit Tests

Each implementation must have unit tests covering:

| Test Category | Description | Target Coverage |
|---------------|-------------|-----------------|
| Name normalization | `to_snake_case`, `to_camel_case`, `to_joined_lower` with edge cases | 100% |
| Input validation | Reject bad names, unknown languages, check kebab-case regex | 100% |
| Dependency reading | Given mock metadata files, extract dependency lists correctly | 95% |
| Transitive closure | Given a graph, verify complete transitive set | 95% |
| Topological sort | Verify leaf-first ordering, cycle detection | 95% |
| Template rendering | For each language, verify generated file content matches expected output | 90% |
| BUILD file generation | Verify transitive deps appear in correct topological order | 100% |
| Dry-run mode | Verify no files are written, output shows what would be created | 90% |

### 9.2 Integration Tests

1. **Zero-dependency package** — scaffold, then run `bash BUILD`, verify exit 0
2. **Single dependency** — scaffold depending on an existing package, verify
   BUILD file includes the dependency
3. **Transitive dependencies** — scaffold depending on a package that itself
   has dependencies, verify BUILD file includes all transitive deps in correct
   order
4. **All-languages mode** — scaffold with `--language all`, verify all 6
   directories are created with correct conventions
5. **Directory exists** — attempt to scaffold over an existing package, verify
   error and exit code 1
6. **Missing dependency** — specify a dependency that doesn't exist, verify
   error message names the missing package and language
7. **Rust workspace update** — scaffold a Rust package, verify the workspace
   `Cargo.toml` was updated

### 9.3 Coverage Targets

- Core logic (name normalization, dep resolution, templates): **95%+**
- Overall including CLI integration: **80%+**

---

## 10. Edge Cases and Error Handling

### 10.1 Circular Dependencies

If the dependency graph contains a cycle (A → B → C → A), the topological sort
will fail. The tool must:
1. Detect the cycle
2. Report which packages form the cycle
3. Exit with code 1

### 10.2 Missing Dependencies for Specific Languages

When using `--language all` with `--depends-on`, a dependency might exist in
Python but not yet in Elixir. The tool must:
1. Report which (language, dependency) pairs are missing
2. Exit with code 1 (do not generate partial output)

### 10.3 Elixir Reserved Words

If the package name or a generated variable name would collide with an Elixir
reserved word (`after`, `rescue`, `catch`, `else`, `end`, `fn`, `do`, `when`,
`cond`, `try`, `receive`), the tool must print a warning. It should still
generate the package (the package *name* becoming a module name like
`CodingAdventures.Rescue` is fine — it's variable names that are problematic).

### 10.4 Rust Workspace Conflicts

If the package name is already in the workspace `Cargo.toml` members list
(perhaps from a previous failed scaffold that was partially cleaned up), the
tool should detect this and warn rather than duplicating the entry.

### 10.5 Go Package Name Conflicts

Go package names must not conflict with standard library packages. If the
package name normalizes to a standard library package name (e.g., `fmt`,
`os`, `net`), the tool should warn.

### 10.6 Mixed Library/Program Dependencies

A library depending on a program (or vice versa) is unusual. The tool should
print a warning but not prevent generation.

---

## 11. Implementation Priority

Not all language implementations need to ship simultaneously. The
recommended priority order:

1. **Go** — the primary build tool and most programs are in Go; fast compilation
   makes it ideal for a CLI tool that agents run frequently
2. **Python** — the most mature package ecosystem in the repo; highest package
   count alongside Rust
3. **TypeScript** — the most failure-prone language; getting scaffolding right
   here has the highest impact
4. **Ruby** — moderate complexity, well-understood patterns
5. **Rust** — important for workspace integration, but cargo handles deps well
6. **Dart** — bootstrap the Dart ecosystem so future lexer/parser and runtime
   ports start from correct templates instead of hand-written package layouts
7. **Elixir** — fewest packages, lowest priority but still needed for parity

---

## 12. Future Extensions

These are explicitly **out of scope** for v1.0 but noted for future work:

1. **`scaffold-generator update`** — update an existing package's BUILD file
   when its dependencies change (re-compute transitive closure)
2. **`scaffold-generator check`** — verify an existing package matches current
   conventions (useful for auditing hand-crafted packages)
3. **Spec file generation** — optionally generate a `code/specs/` document
   for the new package
4. **Interactive mode** — prompt for missing options instead of requiring flags
5. **Template customization** — allow per-project template overrides via a
   config file
