# Adding Perl to the Monorepo Build System

## 1. Overview

The build system discovers, resolves, and builds packages across a
multi-language monorepo. It currently supports eight languages: Python, Ruby,
Go, TypeScript, Rust, Elixir, Lua, and Starlark. This spec adds Perl as the
ninth.

Three implementations of the build tool exist:

| Implementation | Location | Role |
|----------------|----------|------|
| **Go** | `code/programs/go/build-tool/` | Primary — used in CI |
| **Python** | `code/programs/python/build-tool/` | Educational port |
| **Ruby** | `code/programs/ruby/build-tool/` | Educational port |

All three must be updated. The Go implementation is authoritative; Python and
Ruby mirror its logic.

### Why add Perl?

The monorepo is gaining Perl packages — pure-Perl ports of the computing
stack (logic-gates through virtual-machine) and later FFI-accelerated data
structures wrapping Rust. Without build tool support, Perl packages cannot
participate in:

- **Change detection** — the build system won't know which file extensions
  matter for Perl, so changes to `.pm` files would be invisible.
- **Dependency resolution** — the system won't know how to parse `cpanfile`
  for internal dependencies, so Perl packages would always build in
  arbitrary order.
- **Parallel execution** — without dependency edges, the executor can't
  determine which Perl packages are independent.

Adding Perl is a horizontal change: it touches five subsystems but requires
no new architecture. We are adding one more row to every language table.

---

## 2. Where It Fits

```
          BUILD file found?
                |
                v
        +--------------+
        |  Discovery    |  <-- Add "perl" to inferLanguage()
        +--------------+
                |
                v
        +--------------+
        |  Hasher       |  <-- Add .pm, .pl, .t, .xs extensions
        +--------------+
                |
                v
        +--------------+
        |  Resolver     |  <-- Add parsePerlDeps() for cpanfile
        +--------------+
                |
                v
        +--------------+
        |  Executor     |  <-- No changes (shell execution is generic)
        +--------------+
                |
                v
        +--------------+
        |  Starlark     |  <-- Add perl_library() / perl_binary() rules
        +--------------+
```

The executor needs no changes because it runs shell commands from BUILD files
regardless of language. The other four subsystems each need Perl-specific
additions.

---

## 3. Perl Package Ecosystem Primer

Before diving into the changes, a brief tour of Perl's packaging world for
readers unfamiliar with it.

### 3.1 cpanfile — The Dependency Declaration

A `cpanfile` is Perl's equivalent of `Gemfile` (Ruby), `pyproject.toml`
(Python), or `package.json` (Node). It uses a Perl DSL:

```perl
requires 'Some::Module', '>= 1.0';
requires 'Another::Module';

on 'test' => sub {
    requires 'Test2::V0';
};
```

We chose cpanfile over `Makefile.PL` for dependency parsing because:
- It is **declarative** — one `requires` per line, easy to grep.
- It separates **declaration** from **execution** — `Makefile.PL` is
  executable Perl that can do arbitrary things.
- It matches the pattern of `Gemfile`, `package.json`, etc. — a dedicated
  dependency file alongside the build config.

### 3.2 cpanm — The Installer

`cpanm` (App::cpanminus) is the fast, modern CPAN installer. It replaces the
older `cpan` shell. In BUILD files we use:

```
cpanm --installdeps --quiet .
```

This reads `cpanfile` (or `Makefile.PL` / `Build.PL`) and installs all
declared dependencies.

### 3.3 prove — The Test Runner

`prove` runs Perl test files (`.t` files) and produces TAP (Test Anything
Protocol) output:

```
prove -l -v t/
```

The `-l` flag adds `lib/` to `@INC` (Perl's module search path), and `-v`
enables verbose output.

### 3.4 Naming Conventions

| Context | Convention | Example |
|---------|-----------|---------|
| Directory | kebab-case | `code/packages/perl/logic-gates/` |
| CPAN distribution name | `coding-adventures-<kebab>` | `coding-adventures-logic-gates` |
| Module namespace | `CodingAdventures::<CamelCase>` | `CodingAdventures::LogicGates` |
| Source file path | `lib/CodingAdventures/<CamelCase>.pm` | `lib/CodingAdventures/LogicGates.pm` |
| Test files | `t/<number>-<name>.t` | `t/00-load.t` |

The CPAN distribution name `coding-adventures-<kebab>` follows the same
pattern as Python (`coding-adventures-<kebab>`) and Lua
(`coding-adventures-<kebab>`). This consistency simplifies the resolver.

---

## 4. Changes Per Subsystem

### 4.1 Language Detection (Discovery)

**File:** `code/programs/go/build-tool/internal/discovery/discovery.go`
**Function:** `inferLanguage()` at line 119

The `inferLanguage` function splits a package path on `/` and searches for
known language names. Adding Perl is a one-line addition to the slice:

```
Current:  "python", "ruby", "go", "rust", "typescript", "elixir", "lua", "starlark"
New:      "python", "ruby", "go", "rust", "typescript", "elixir", "lua", "starlark", "perl"
```

When the build system encounters `code/packages/perl/logic-gates/BUILD`, it
splits the path, finds `"perl"` as a component, and infers the language.

The package name is then built by `inferPackageName()` as `"perl/logic-gates"`.

### 4.2 Source Hashing (Hasher)

**File:** `code/programs/go/build-tool/internal/hasher/hasher.go`

Two maps need Perl entries:

**`sourceExtensions` (line 52):**

```go
"perl": {".pl": true, ".pm": true, ".t": true, ".xs": true},
```

| Extension | What it is | Why include it |
|-----------|-----------|----------------|
| `.pm` | Perl module | The primary source file type |
| `.pl` | Perl script | Executable scripts, sometimes used in packages |
| `.t` | Perl test file | Test changes should trigger rebuilds |
| `.xs` | XS C bindings | For packages with native extensions |

**`specialFilenames` (line 64):**

```go
"perl": {
    "Makefile.PL": true,
    "Build.PL":    true,
    "cpanfile":    true,
    "MANIFEST":    true,
    "META.json":   true,
    "META.yml":    true,
},
```

These are Perl packaging files that don't have recognized extensions but
affect the build. Changes to `cpanfile` (dependencies) or `Makefile.PL`
(build configuration) must trigger a rebuild.

### 4.3 Dependency Resolution (Resolver)

**File:** `code/programs/go/build-tool/internal/resolver/resolver.go`

Three additions:

#### 4.3.1 Known Names Mapping

In `buildKnownNames()` (line 494), add a `case "perl":` that maps CPAN
distribution names to internal package names:

```
Directory basename: "logic-gates"
CPAN dist name:     "coding-adventures-logic-gates"
Internal name:      "perl/logic-gates"
```

The mapping is: `"coding-adventures-" + basename → pkg.Name`.

This matches the Python convention exactly:

| Language | External name | Internal name |
|----------|--------------|---------------|
| Python | `coding-adventures-logic-gates` | `python/logic-gates` |
| **Perl** | `coding-adventures-logic-gates` | `perl/logic-gates` |
| Ruby | `coding_adventures_logic_gates` | `ruby/logic_gates` |
| TypeScript | `@coding-adventures/logic-gates` | `typescript/logic-gates` |

#### 4.3.2 cpanfile Parsing

A new `parsePerlDeps()` function reads `cpanfile` from the package directory
and extracts internal dependencies.

**Algorithm:**

```
function parsePerlDeps(pkg, knownNames):
    cpanfile = read(pkg.path + "/cpanfile")
    if cpanfile does not exist:
        return []

    deps = []
    for each line in cpanfile:
        # Match: requires 'coding-adventures-logic-gates';
        # Or:    requires 'coding-adventures-logic-gates', '>= 0.01';
        if line matches /requires\s+['"]coding-adventures-([^'"]+)['"]/
            depName = "coding-adventures-" + capture_group_1
            if depName in knownNames:
                deps.append(knownNames[depName])

    return deps
```

**Why regex is sufficient:**

The `cpanfile` format is a Perl DSL, so in theory it could contain arbitrary
Perl code. However, our `cpanfile` files are generated by the scaffold
generator and follow a strict format — one `requires` per line with a quoted
string argument. Regex parsing handles this reliably.

This is the same approach used for Python (regex on `pyproject.toml`), Ruby
(regex on `Gemfile`), and TypeScript (regex on `package.json`). We parse just
enough to extract dependency names without importing a full parser for each
language's config format.

**Edge cases:**

| Input | Behavior |
|-------|----------|
| `requires 'Moo';` | Skipped — doesn't start with `coding-adventures-` |
| `requires 'coding-adventures-bitset', '>= 0.01';` | Extracted as `coding-adventures-bitset` |
| `requires "coding-adventures-bitset";` | Extracted — regex accepts both single and double quotes |
| `on 'test' => sub { requires 'Test2::V0'; };` | Skipped — no `coding-adventures-` prefix |
| Empty cpanfile | Returns empty list |
| Missing cpanfile | Returns empty list |
| `# requires 'coding-adventures-foo';` | Skipped — commented out |

The comment case deserves attention. Our regex matches `requires` anywhere
on the line. To handle comments, the parsing should skip lines whose first
non-whitespace character is `#`. This matches how the Python and Ruby parsers
work.

#### 4.3.3 ResolveDependencies Switch

In `ResolveDependencies()` (line 574), add:

```go
case "perl":
    deps = parsePerlDeps(pkg, knownNames)
```

### 4.4 Starlark Rules

**File:** `code/programs/go/build-tool/internal/starlark/evaluator.go`

Two changes:

#### 4.4.1 IsStarlarkBuild Detection (line 74)

Add `"perl_library("` and `"perl_binary("` to the `knownRules` slice:

```go
"perl_library(", "perl_binary(",
```

#### 4.4.2 GenerateCommands (line 241)

Add a new case:

```go
case "perl_library", "perl_binary":
    return []string{
        "cpanm --installdeps --quiet .",
        "prove -l -v t/",
    }
```

### 4.5 Language List (main.go)

**File:** `code/programs/go/build-tool/main.go`

Add `"perl"` to `allLanguages` (line 382):

```go
var allLanguages = []string{"python", "ruby", "go", "typescript", "rust", "elixir", "lua", "perl"}
```

This enables:
- `--language perl` to filter builds to only Perl packages.
- `--detect-languages` to include Perl in its output.

---

## 5. BUILD File Pattern

A typical Perl package BUILD file:

```bash
# Install dependencies from cpanfile
cpanm --installdeps --quiet .
# Run tests with lib/ in @INC
prove -l -v t/
```

For packages with dependencies on other Perl packages in the monorepo, the
BUILD file must chain-install transitive dependencies leaf-first (same pattern
as Python, Ruby, TypeScript, and Go):

```bash
# Install transitive deps leaf-first
cd ../logic-gates && cpanm --installdeps --quiet .
cd ../arithmetic && cpanm --installdeps --quiet .
# Install this package's deps and run tests
cpanm --installdeps --quiet .
prove -l -v t/
```

This ensures a clean CI environment can build from scratch. The scaffold
generator (Spec 3) will produce these BUILD files automatically with the
correct transitive ordering.

---

## 6. Mirror Changes in Python and Ruby Build Tools

The Python and Ruby build tool implementations must receive the same logical
changes.

### 6.1 Python Build Tool

**Location:** `code/programs/python/build-tool/`

| Module | Change |
|--------|--------|
| `discovery.py` | Add `"perl"` to `SUPPORTED_LANGUAGES` |
| `hasher.py` | Add Perl to `SOURCE_EXTENSIONS` and `SPECIAL_FILES` dicts |
| `resolver.py` | Add `parse_perl_deps()` function, add `"perl"` case to `build_known_names()` and `resolve_dependencies()` |

### 6.2 Ruby Build Tool

**Location:** `code/programs/ruby/build-tool/`

| Module | Change |
|--------|--------|
| `discovery.rb` | Add `"perl"` to `SUPPORTED_LANGUAGES` |
| `hasher.rb` | Add Perl to `SOURCE_EXTENSIONS` and `SPECIAL_FILES` hashes |
| `resolver.rb` | Add `parse_perl_deps()` method, add `"perl"` case to `build_known_names()` and `resolve_dependencies()` |

---

## 7. Test Strategy

### 7.1 Discovery Tests (~5 cases)

| Test | Input | Expected |
|------|-------|----------|
| Infer Perl language | Path `code/packages/perl/logic-gates` | Language: `"perl"` |
| Infer Perl package name | Path `code/packages/perl/logic-gates` | Name: `"perl/logic-gates"` |
| Discover Perl package | Directory with BUILD file under `code/packages/perl/` | Package discovered |
| Skip non-Perl | Path `code/packages/python/logic-gates` | Language: `"python"`, not `"perl"` |
| Unknown remains unknown | Path `code/packages/unknown/foo` | Language: `"unknown"` |

### 7.2 Hasher Tests (~6 cases)

| Test | Input | Expected |
|------|-------|----------|
| Include .pm files | `lib/CodingAdventures/Foo.pm` | Included in hash |
| Include .pl files | `bin/script.pl` | Included |
| Include .t files | `t/00-load.t` | Included |
| Include .xs files | `Foo.xs` | Included |
| Include cpanfile | `cpanfile` | Included |
| Include Makefile.PL | `Makefile.PL` | Included |
| Exclude .bak files | `Foo.pm.bak` | Excluded |
| Exclude non-source | `README.md` | Excluded (not in extensions or specials) |

### 7.3 Resolver Tests (~10 cases)

| Test | Input (cpanfile contents) | Expected |
|------|--------------------------|----------|
| Single dep | `requires 'coding-adventures-logic-gates';` | `["perl/logic-gates"]` |
| Multiple deps | Two `requires` lines | Both resolved |
| External dep skipped | `requires 'Moo';` | `[]` |
| Versioned dep | `requires 'coding-adventures-bitset', '>= 0.01';` | `["perl/bitset"]` |
| Double-quoted dep | `requires "coding-adventures-bitset";` | `["perl/bitset"]` |
| Commented dep | `# requires 'coding-adventures-foo';` | `[]` |
| Test-phase dep | `on 'test' => sub { requires 'Test2::V0'; };` | `[]` |
| Empty cpanfile | (empty file) | `[]` |
| Missing cpanfile | (no file) | `[]` |
| Known names mapping | Dir `bitset` | Known name: `"coding-adventures-bitset"` |

### 7.4 Starlark Tests (~3 cases)

| Test | Input | Expected |
|------|-------|----------|
| Detect perl_library | `perl_library(name = "foo")` | `IsStarlarkBuild() == true` |
| Generate commands | Target with Rule `"perl_library"` | `["cpanm --installdeps --quiet .", "prove -l -v t/"]` |
| Detect perl_binary | `perl_binary(name = "bar")` | `IsStarlarkBuild() == true` |

### 7.5 Integration Test (~1 case)

| Test | Setup | Expected |
|------|-------|----------|
| Perl dep chain | Package A depends on B, B depends on C | Build order: C, B, A |

**Total: ~25 test cases.**

---

## 8. Exhaustive File List

Every file that must be modified, across all three implementations:

### Go (Primary)

| File | What changes |
|------|-------------|
| `code/programs/go/build-tool/internal/discovery/discovery.go` | Add `"perl"` to language list |
| `code/programs/go/build-tool/internal/hasher/hasher.go` | Add Perl to `sourceExtensions` and `specialFilenames` |
| `code/programs/go/build-tool/internal/resolver/resolver.go` | Add `parsePerlDeps()`, extend `buildKnownNames()`, extend `ResolveDependencies()` |
| `code/programs/go/build-tool/internal/starlark/evaluator.go` | Add `perl_library/perl_binary` to `knownRules` and `GenerateCommands()` |
| `code/programs/go/build-tool/main.go` | Add `"perl"` to `allLanguages` |
| `code/programs/go/build-tool/internal/discovery/discovery_test.go` | Add Perl discovery tests |
| `code/programs/go/build-tool/internal/hasher/hasher_test.go` | Add Perl hashing tests |
| `code/programs/go/build-tool/internal/resolver/resolver_test.go` | Add Perl resolver tests |
| `code/programs/go/build-tool/internal/starlark/evaluator_test.go` | Add Perl starlark tests |

### Python (Educational)

| File | What changes |
|------|-------------|
| `code/programs/python/build-tool/build_tool/discovery.py` | Add `"perl"` |
| `code/programs/python/build-tool/build_tool/hasher.py` | Add Perl extensions/specials |
| `code/programs/python/build-tool/build_tool/resolver.py` | Add `parse_perl_deps()` |

### Ruby (Educational)

| File | What changes |
|------|-------------|
| `code/programs/ruby/build-tool/lib/build_tool/discovery.rb` | Add `"perl"` |
| `code/programs/ruby/build-tool/lib/build_tool/hasher.rb` | Add Perl extensions/specials |
| `code/programs/ruby/build-tool/lib/build_tool/resolver.rb` | Add `parse_perl_deps()` |

---

## 9. Trade-Offs

### 9.1 cpanfile vs Makefile.PL for Dependency Parsing

| | cpanfile | Makefile.PL |
|-|----------|-------------|
| Format | Declarative DSL | Executable Perl |
| Parsing difficulty | Simple regex | Requires Perl evaluation |
| Generated by scaffold | Yes | Yes (but contains less dep info) |
| CPAN standard | Yes | Yes |
| **Decision** | **cpanfile** | — |

We parse `cpanfile` because it is declarative and grep-friendly, matching the
approach used for every other language in the build system.

### 9.2 Distribution Names vs Module Names

| | Dist name (`coding-adventures-bitset`) | Module name (`CodingAdventures::Bitset`) |
|-|---------------------------------------|------------------------------------------|
| Format consistency | Matches Python prefix pattern | Unique to Perl |
| Regex simplicity | Hyphen-separated, flat | Double-colon separated, CamelCase |
| CPAN convention | Standard for dist names | Standard for module names |
| **Decision** | **Distribution names** | — |

Using CPAN distribution names in `cpanfile` keeps the resolver consistent
with Python and Lua, which also use `coding-adventures-<kebab>` prefixes.

---

## 10. Future Extensions

- **Devel::Cover integration:** Add coverage checking to BUILD commands
  (e.g., `cover -test -report text`).
- **Perl::Critic linting:** Add `perlcritic lib/` to the Starlark
  `GenerateCommands()` output for `perl_library`.
- **XS compilation detection:** If a package contains `.xs` files, the
  hasher should also track `ppport.h` and `typemap` files.
- **Carton lockfile support:** Parse `cpanfile.snapshot` for exact version
  pinning in CI builds.
