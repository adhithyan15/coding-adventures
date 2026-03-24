# Scaffold Generator Perl Support — Adding Perl Templates

## 1. Overview

### 1.1 The Problem

The scaffold generator produces correct-by-construction package scaffolding
for six languages: Python, Go, Ruby, TypeScript, Rust, and Elixir. Adding
Perl to the monorepo means the scaffold generator must also produce Perl
packages. Without this, agents would hand-craft Perl package boilerplate —
the exact failure mode the scaffold generator was designed to eliminate.

The `lessons.md` file documents twelve recurring CI failure categories, all
caused by hand-crafted scaffolding. The same categories apply to Perl:

| Risk | How scaffold generator prevents it |
|------|-----------------------------------|
| Missing BUILD file | Always generates BUILD with correct commands |
| BUILD missing transitive deps | Computes full closure, topologically sorted |
| Missing cpanfile | Always generates cpanfile with correct requires |
| Wrong module namespace | Name normalization converts kebab to CamelCase |
| Missing `1;` at end of `.pm` | Template always includes it |
| Test file missing `use strict` | Template always includes it |
| Missing README.md or CHANGELOG.md | Always generated |

### 1.2 The Solution

Add a `generatePerl()` function (or equivalent) to all seven scaffold
generator implementations:

| Implementation | Location | Language |
|----------------|----------|----------|
| Go (primary) | `code/programs/go/scaffold-generator/main.go` | Go |
| Python | `code/programs/python/scaffold-generator/` | Python |
| Ruby | `code/programs/ruby/scaffold-generator/` | Ruby |
| TypeScript | `code/programs/typescript/scaffold-generator/` | TypeScript |
| Rust | `code/programs/rust/scaffold-generator/` | Rust |
| Elixir | `code/programs/elixir/scaffold-generator/` | Elixir |
| Lua | `code/programs/lua/scaffold-generator/` | Lua |

Additionally, `"perl"` must be added to the valid language list in the CLI
spec (`code/programs/scaffold-generator.json`) and in each implementation's
validation logic.

---

## 2. Where It Fits

```
scaffold-generator my-package --language perl --depends-on logic-gates
    |
    v
code/packages/perl/my-package/
    ├── BUILD
    ├── CHANGELOG.md
    ├── README.md
    ├── Makefile.PL
    ├── cpanfile
    ├── lib/CodingAdventures/MyPackage.pm
    └── t/
        ├── 00-load.t
        └── 01-basic.t
```

The scaffold generator reads existing packages to discover transitive
dependencies, then writes 8 files with correct content. This spec defines
what those 8 files look like for Perl.

---

## 3. Name Normalization

### 3.1 Conversion Rules

The input `PACKAGE_NAME` is always kebab-case (e.g., `my-package`). Perl
needs three derived forms:

| Form | Function | Example for `my-package` |
|------|----------|--------------------------|
| Snake case | `to_snake_case()` | `my_package` |
| CamelCase | `to_camel_case()` | `MyPackage` |
| Kebab (original) | Identity | `my-package` |

### 3.2 Perl-Specific Names

Given the input `my-package`:

| Context | Value |
|---------|-------|
| **Directory name** | `my-package` (kebab-case) |
| **CPAN distribution name** | `coding-adventures-my-package` |
| **Module namespace** | `CodingAdventures::MyPackage` |
| **Source file** | `lib/CodingAdventures/MyPackage.pm` |
| **Test file (load)** | `t/00-load.t` |
| **Test file (basic)** | `t/01-basic.t` |
| **Import statement** | `use CodingAdventures::MyPackage;` |

### 3.3 Comparison with Other Languages

| Context | Python | Ruby | Go | Perl |
|---------|--------|------|----|------|
| Dir name | `my-package` | `my_package` | `my-package` | `my-package` |
| Package/dist name | `coding-adventures-my-package` | `coding_adventures_my_package` | (module path) | `coding-adventures-my-package` |
| Module name | `my_package` | `CodingAdventures::MyPackage` | `mypackage` | `CodingAdventures::MyPackage` |
| Source dir | `src/my_package/` | `lib/coding_adventures/my_package/` | (flat) | `lib/CodingAdventures/` |
| Import | `from my_package import ...` | `require "coding_adventures_my_package"` | `import mypackage` | `use CodingAdventures::MyPackage;` |

Note that Perl's directory naming uses **kebab-case** (matching Python, Go,
TypeScript, and Rust), not snake_case (which Ruby and Elixir use).

### 3.4 Dependency Name Normalization

When `--depends-on logic-gates` is specified:

| Context | How the dependency appears |
|---------|--------------------------|
| cpanfile | `requires 'coding-adventures-logic-gates';` |
| Makefile.PL PREREQ_PM | `'CodingAdventures::LogicGates' => 0` |
| BUILD install line | `cd ../logic-gates && cpanm --installdeps --quiet .` |
| Source code import | `use CodingAdventures::LogicGates;` |

---

## 4. Generated Files

### 4.1 Makefile.PL

```perl
use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::<CamelCase>',
    VERSION_FROM     => 'lib/CodingAdventures/<CamelCase>.pm',
    ABSTRACT         => '<description>',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        <for each direct dep:>
        'CodingAdventures::<DepCamelCase>' => 0,
    },
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
    META_MERGE       => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan15/coding-adventures.git',
                web  => 'https://github.com/adhithyan15/coding-adventures',
            },
        },
    },
);
```

**Template variables:**
- `<CamelCase>`: `to_camel_case(package_name)`
- `<description>`: from `--description` flag, or `"A coding-adventures package"`
- Direct dependencies populate `PREREQ_PM`.

### 4.2 cpanfile

```perl
# Runtime dependencies
<for each direct dep:>
requires 'coding-adventures-<dep-kebab>';

# Test dependencies
on 'test' => sub {
    requires 'Test2::V0';
};
```

The cpanfile declares runtime dependencies using CPAN distribution names
(kebab-case with `coding-adventures-` prefix). Test dependencies go in the
`on 'test'` phase.

### 4.3 Source Module (`lib/CodingAdventures/<CamelCase>.pm`)

```perl
package CodingAdventures::<CamelCase>;

# ============================================================================
# CodingAdventures::<CamelCase> — <description>
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#
<if layer:>
# Layer <layer> in the computing stack.
<end if>
#
# Usage:
#
#   use CodingAdventures::<CamelCase>;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

<for each direct dep:>
use CodingAdventures::<DepCamelCase>;

# TODO: Implement <CamelCase>

1;

__END__

=head1 NAME

CodingAdventures::<CamelCase> - <description>

=head1 SYNOPSIS

    use CodingAdventures::<CamelCase>;

=head1 DESCRIPTION

<description>

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
```

**Key points:**
- `use strict; use warnings;` at the top of every file — non-negotiable.
- `our $VERSION = '0.01';` — `VERSION_FROM` in `Makefile.PL` reads this.
- `1;` at the end — Perl requires modules to return a true value.
- Pod documentation after `__END__` — separates code from docs.
- Import statements for direct dependencies.

### 4.4 Test: Load (`t/00-load.t`)

```perl
use strict;
use warnings;
use Test2::V0;

use_ok('CodingAdventures::<CamelCase>');

# Verify the module exports a version number.
ok(CodingAdventures::<CamelCase>->VERSION, 'has a VERSION');

done_testing;
```

This test verifies the module loads without errors and has a version. It
catches compilation errors, missing dependencies, and syntax problems.

### 4.5 Test: Basic (`t/01-basic.t`)

```perl
use strict;
use warnings;
use Test2::V0;

use CodingAdventures::<CamelCase>;

# TODO: Replace this placeholder with real tests.
ok(1, '<CamelCase> module loaded successfully');

done_testing;
```

A placeholder that the implementer replaces with real tests. It passes
immediately, ensuring the BUILD file succeeds out of the box.

### 4.6 BUILD

**Without dependencies:**

```bash
cpanm --installdeps --quiet .
prove -l -v t/
```

**With transitive dependencies (leaf-first order):**

```bash
cd ../<transitive-dep-1> && cpanm --installdeps --quiet .
cd ../<transitive-dep-2> && cpanm --installdeps --quiet .
cd ../<direct-dep-1> && cpanm --installdeps --quiet .
cd ../<this-package> && cpanm --installdeps --quiet .
prove -l -v t/
```

The scaffold generator computes the transitive closure of dependencies, then
topologically sorts them (leaves first). This ensures that when CI runs the
BUILD file in a clean environment, every dependency is installed before
anything that uses it.

**Example:** If `my-package` depends on `arithmetic`, which depends on
`logic-gates`:

```bash
cd ../logic-gates && cpanm --installdeps --quiet .
cd ../arithmetic && cpanm --installdeps --quiet .
cd ../my-package && cpanm --installdeps --quiet .
prove -l -v t/
```

### 4.7 README.md

```markdown
# <CamelCase>

<description>

<if layer:>
**Layer <layer>** in the coding-adventures computing stack.
<end if>

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::<CamelCase>;
```

## Testing

```bash
prove -l -v t/
```

## Dependencies

<if deps:>
<for each direct dep:>
- [`CodingAdventures::<DepCamelCase>`](../<dep-kebab>/)
<end for>
<else>
None.
<end if>

## License

MIT
```

### 4.8 CHANGELOG.md

```markdown
# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - <today's date>

### Added

- Initial package scaffolding.
- Module `CodingAdventures::<CamelCase>` with version 0.01.
- Load test (`t/00-load.t`) and basic test (`t/01-basic.t`).
```

---

## 5. Dependency Resolution

### 5.1 How It Works

The scaffold generator must compute the **transitive closure** of
dependencies and produce a **topologically sorted** install order for the
BUILD file. This is the most critical feature — missing transitive
dependencies in BUILD files is the #1 cause of CI failures.

**Algorithm:**

```
function compute_install_order(direct_deps, language):
    # Step 1: Read existing packages to discover their deps
    all_deps = {}
    for each dep in direct_deps:
        pkg_path = find_package(dep, language)
        all_deps[dep] = read_deps_from_metadata(pkg_path, language)

    # Step 2: Compute transitive closure via BFS
    transitive = set(direct_deps)
    queue = list(direct_deps)
    while queue is not empty:
        current = queue.pop()
        for subdep in all_deps.get(current, []):
            if subdep not in transitive:
                transitive.add(subdep)
                queue.append(subdep)
                all_deps[subdep] = read_deps_from_metadata(
                    find_package(subdep, language), language)

    # Step 3: Topological sort (Kahn's algorithm)
    return topological_sort(transitive, all_deps)
```

### 5.2 Reading Perl Dependencies

To compute transitive deps, the scaffold generator must read existing Perl
packages' `cpanfile` files:

```
function read_perl_deps(package_path):
    cpanfile = read(package_path + "/cpanfile")
    deps = []
    for each line in cpanfile:
        if line matches /requires\s+['"]coding-adventures-([^'"]+)['"]/
            deps.append(capture_group_1)
    return deps
```

This matches the pattern used for reading Ruby deps from Gemfile, Python
deps from BUILD files, etc.

---

## 6. CLI Spec Changes

### 6.1 scaffold-generator.json

The `language` flag's description should be updated to mention Perl:

```json
{
    "id": "language",
    "description": "Target language(s). Comma-separated or 'all' for all 7 languages",
    ...
}
```

The valid values become: `python`, `ruby`, `go`, `typescript`, `rust`,
`elixir`, `perl`, `all`.

Note: Lua is not currently in the scaffold generator's language list (it was
added separately). Perl joins the primary set.

### 6.2 Validation

When `--language perl` or `--language all` is used:
- The target directory `code/packages/perl/<name>/` must not exist.
- If `--depends-on` is specified, each dependency's Perl package must exist
  at `code/packages/perl/<dep-kebab>/`.

---

## 7. Files Modified

### 7.1 Go (Primary Implementation)

| File | Change |
|------|--------|
| `code/programs/go/scaffold-generator/main.go` | Add `"perl"` to `validLanguages`; add `case "perl":` in language dispatch; add `generatePerl()` function (~120 lines); add `readPerlDeps()` function |
| `code/programs/scaffold-generator.json` | Update description to mention Perl; update valid language count |

### 7.2 All Other Implementations

Each of the six port implementations needs the equivalent changes:

| Implementation | Key File | Function to Add |
|----------------|----------|-----------------|
| Python | `scaffold_generator.py` | `generate_perl()` |
| Ruby | scaffold generator source | `generate_perl()` |
| TypeScript | scaffold generator source | `generatePerl()` |
| Rust | scaffold generator source | `generate_perl()` |
| Elixir | scaffold generator source | `generate_perl()` |
| Lua | `scaffold.lua` | `generate_perl()` |

---

## 8. Test Strategy

Each implementation should test Perl scaffolding. The test cases below apply
to every implementation:

### 8.1 Basic Generation (~8 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 1 | Generate Perl library, no deps | `--language perl my-pkg` | 8 files created |
| 2 | Correct directory | Library mode | `code/packages/perl/my-pkg/` |
| 3 | Correct Makefile.PL | No deps | Empty PREREQ_PM |
| 4 | Correct cpanfile | No deps | Only test requires |
| 5 | Correct module file | `my-pkg` | `package CodingAdventures::MyPkg;` |
| 6 | Module ends with `1;` | Any | Last non-POD line is `1;` |
| 7 | Test loads module | Any | `use_ok('CodingAdventures::MyPkg')` |
| 8 | CHANGELOG has date | Any | Today's date in `[0.1.0]` entry |

### 8.2 Dependencies (~5 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 9 | Single dep in cpanfile | `--depends-on logic-gates` | `requires 'coding-adventures-logic-gates';` |
| 10 | Dep in Makefile.PL | `--depends-on logic-gates` | `'CodingAdventures::LogicGates' => 0` in PREREQ_PM |
| 11 | Dep import in module | `--depends-on logic-gates` | `use CodingAdventures::LogicGates;` |
| 12 | Transitive deps in BUILD | `--depends-on arithmetic` (which depends on logic-gates) | BUILD installs logic-gates before arithmetic |
| 13 | Multiple direct deps | `--depends-on bitset,matrix` | Both in cpanfile, BUILD, and Makefile.PL |

### 8.3 Name Normalization (~4 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 14 | Single word | `bitset` | Module: `CodingAdventures::Bitset` |
| 15 | Multi-word | `logic-gates` | Module: `CodingAdventures::LogicGates` |
| 16 | With numbers | `sha256` | Module: `CodingAdventures::Sha256` |
| 17 | Long name | `bytecode-compiler` | Module: `CodingAdventures::BytecodeCompiler` |

### 8.4 Edge Cases (~3 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 18 | Dry-run mode | `--dry-run` | No files written, output shows tree |
| 19 | Program type | `--type program` | Output in `code/programs/perl/` |
| 20 | `--language all` includes Perl | `--language all my-pkg` | Perl directory among outputs |

**Total: ~20 test cases per implementation, ~140 across all 7.**

---

## 9. Trade-Offs

### 9.1 ExtUtils::MakeMaker vs Module::Build

| | ExtUtils::MakeMaker | Module::Build |
|-|---------------------|---------------|
| Ships with Perl | Yes (core) | Removed from core in 5.22 |
| CPAN support | Universal | Less common |
| FFI compatibility | Proven | Possible but less tested |
| Complexity | Low for simple modules | Similar |
| **Decision** | **ExtUtils::MakeMaker** | — |

We use `Makefile.PL` with `ExtUtils::MakeMaker` because it is universally
supported, ships with every Perl installation, and works with FFI::Platypus
builds (relevant for Spec 5).

### 9.2 cpanfile vs PREREQ_PM-only

| | Both cpanfile + Makefile.PL | Makefile.PL only |
|-|---------------------------|-------------------|
| Build tool parsing | cpanfile is simple regex | Makefile.PL is executable Perl |
| Phase separation | cpanfile has `on 'test'` | PREREQ_PM + TEST_REQUIRES |
| CPAN compatibility | Modern standard | Works everywhere |
| **Decision** | **Both** | — |

We generate both because they serve different consumers: the build tool's
resolver reads `cpanfile` (declarative, easy to parse), while CPAN clients
read `Makefile.PL` (standard distribution format).

### 9.3 Pod Documentation Style

| | After `__END__` | Inline with code |
|-|-----------------|-----------------|
| Separation | Clean — code and docs don't interleave |  Docs next to code |
| Literate programming | Less literate | More literate |
| Perl convention | Common for module-level docs | Common for method-level docs |
| **Decision** | **After `__END__` for scaffold; inline later** | — |

The scaffold generates Pod after `__END__` for the initial template.
Implementers are encouraged to add inline Pod above each method as they
write the actual implementation, following the literate programming
principle.

---

## 10. Future Extensions

- **FFI scaffold template:** When `--ffi` flag is passed, also generate
  `lib/CodingAdventures/<CamelCase>/FFI.pm` with FFI::Platypus boilerplate
  and a reference to the Rust cdylib crate.
- **XS scaffold template:** When `--xs` flag is passed, generate `.xs` file,
  `typemap`, and `ppport.h` with a basic function stub.
- **Perl::Critic config:** Generate a `.perlcriticrc` file with the
  project's severity settings.
- **Devel::Cover config:** Add coverage threshold checking to BUILD files
  once Devel::Cover integration is standardized.
- **Perl scaffold generator implementation:** Create a Perl port of the
  scaffold generator itself at `code/programs/perl/scaffold-generator/`.
