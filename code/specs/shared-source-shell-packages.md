# Shared Source + Shell Packages

## 1. Overview

This spec defines a monorepo packaging model for languages whose packages are
published separately but must not depend on unpublished internal packages at
runtime.

The core idea is:

- **Real implementation code lives in one shared source tree**
- **Package directories are thin shells**
- **A publish tool materializes standalone package artifacts**

This lets us:

- avoid internal package-manager dependencies such as `@coding-adventures/*`
- develop against source directly inside the monorepo
- publish each package independently
- keep published artifacts self-contained

This is motivated by supply-chain hardening. If package `A` relies on package
`B` via the package manager, then `A` inherits `B`'s publication and integrity
risks. In this model, published package `A` contains the internal code it needs
and does not trust a separately installed internal package at runtime.

The initial target is **TypeScript**, but the architecture is intended to be
portable to Ruby, Python, Go, Rust, Elixir, and future languages.

---

## 2. Problem Statement

Today, many packages in the repo are structured as if sibling internal packages
were already published:

- `typescript-parser` depends on `@coding-adventures/parser`
- `typescript-lexer` depends on `@coding-adventures/lexer`
- similar patterns exist across other languages

This creates two problems:

1. **Local development assumes publish-time package boundaries**
   - internal code is split into packages even though the repo is the real
     source of truth

2. **Published packages inherit internal package trust**
   - if `A` depends on internal package `B`, then `A` is vulnerable to any bad
     release of `B`

We want a model where:

- packages may still be published separately
- package code may directly use shared source in the monorepo
- published packages are self-contained
- the language runtime is the only mandatory runtime dependency

---

## 3. Goals

- **Separate publication**: every package can still be released independently
- **No internal runtime package dependencies**: published packages do not
  require separate installation of internal sibling packages
- **Shared source of truth**: implementation is authored in one canonical
  source tree, not duplicated across package directories
- **Thin shell packages**: `code/packages/<lang>/<pkg>/` becomes metadata plus
  minimal re-export surface
- **Deterministic publishing**: a tool can always generate the same standalone
  output from the same commit
- **No third-party runtime dependencies**: only the language runtime may be
  assumed at runtime

---

## 4. Non-Goals

- Solving every possible dynamic import pattern
- Supporting arbitrary package layouts across languages
- Preserving today's package folder structure forever
- Avoiding a publish/build step entirely
- Minimizing published file count at all costs

This spec intentionally favors **simple, deterministic rules** over maximum
flexibility.

---

## 5. High-Level Architecture

### 5.1 Canonical Source Tree

Real implementation code lives under:

```text
code/src/<language>/<package-name>/
```

Example:

```text
code/src/typescript/lexer/
code/src/typescript/parser/
code/src/typescript/typescript-lexer/
code/src/typescript/typescript-parser/
```

Each package subtree owns:

- its implementation files
- its local assets
- its public entrypoint

### 5.2 Shell Package Tree

Publishable package directories live under:

```text
code/packages/<language>/<package-name>/
```

Each shell package contains:

- `package.json`, `.gemspec`, `pyproject.toml`, etc.
- minimal wrapper files that re-export the package's public API
- package-local tests that verify shell wiring if needed

Example:

```text
code/packages/typescript/lexer/package.json
code/packages/typescript/lexer/src/index.ts
```

Wrapper example:

```ts
export * from "../../../../src/typescript/lexer/index.js";
```

### 5.3 Publish Output Tree

Packages are **not** published directly from the shell package directory.

Instead, a publish tool generates a standalone output tree:

```text
.out/publish/<language>/<package-name>/
```

This generated directory contains:

- package manifest
- copied or rewritten wrapper files
- all internal source required by that package
- copied assets needed at runtime
- no internal package-manager dependencies

---

## 6. Design Principles

### 6.1 Package Boundaries Are API Boundaries, Not Storage Boundaries

Packages remain meaningful as public APIs and release units, but their source
does not have to live inside the package folder.

### 6.2 Published Artifacts Must Be Self-Contained

Anything referenced by the published package must ship inside the generated
artifact.

### 6.3 Shared Source Is Canonical

The shared `code/src/<language>/...` tree is the only place where real logic
should be hand-maintained.

### 6.4 Shell Packages Stay Thin

Shell packages may define metadata and public exports, but they must not become
a second implementation tree.

### 6.5 Determinism Over Cleverness

The publish tool should follow simple structural rules, not heuristics.

---

## 7. Hard Constraints

These constraints are required to keep the publish tool scoped and reliable.

### 7.1 One Public Entrypoint Per Package

Each package must have exactly one canonical public entrypoint:

```text
code/src/<language>/<package>/index.*
```

For TypeScript:

```text
code/src/typescript/lexer/index.ts
```

The shell package re-exports from this entrypoint.

### 7.2 No Deep Cross-Package Imports

Internal source may import another package only through that package's public
entrypoint.

Allowed:

```ts
import { GrammarParser } from "../parser/index.js";
```

Not allowed:

```ts
import { GrammarParser } from "../parser/grammar-parser.js";
```

This avoids exposing file-layout details across packages.

### 7.3 No Cyclic Package Dependencies

The package dependency graph must be acyclic.

File-level cycles inside a package may be allowed if the language tolerates
them, but package-level cycles are forbidden because they complicate copying,
rewriting, and release reasoning.

### 7.4 No Dynamic Internal Imports

Internal package usage must be statically discoverable.

Forbidden patterns include:

- computed import strings
- runtime `require()` path construction
- string concatenation used to resolve internal modules

### 7.5 Assets Stay Inside the Package Subtree

Any non-code asset required by package `X` must live under:

```text
code/src/<language>/<X>/
```

Examples:

- grammar files
- JSON schemas
- templates
- CSS
- fixtures needed at runtime

Package `A` may not reach into package `B`'s asset directory directly.

### 7.6 Shell Packages Contain No Real Logic

The shell package directory may contain:

- metadata
- wrapper exports
- tiny package-specific glue if absolutely necessary

It may not contain the canonical implementation.

### 7.7 No Internal Package-Manager Dependencies

Shell package manifests must not declare internal sibling packages as runtime
dependencies.

Examples of forbidden runtime dependencies:

- `@coding-adventures/parser`
- `coding_adventures_parser`
- `coding-adventures-parser`

### 7.8 Generated Publish Directories Are Disposable

Publish output is generated from scratch and must never be edited manually.

---

## 8. TypeScript Layout

The initial TypeScript structure should look like:

```text
code/src/typescript/
  lexer/
    index.ts
    grammar-lexer.ts
    tokenizer.ts
  parser/
    index.ts
    parser.ts
    grammar-parser.ts
  typescript-lexer/
    index.ts
    tokenizer.ts
  typescript-parser/
    index.ts
    parser.ts

code/packages/typescript/
  lexer/
    package.json
    src/index.ts
  parser/
    package.json
    src/index.ts
  typescript-lexer/
    package.json
    src/index.ts
  typescript-parser/
    package.json
    src/index.ts
```

TypeScript shell wrapper example:

```ts
export * from "../../../../src/typescript/typescript-parser/index.js";
```

TypeScript source packages may import sibling shared-source packages directly
via relative paths inside `code/src/typescript`.

---

## 9. Publish Tool Responsibilities

We introduce a new tool, conceptually:

```text
materialize-package <language> <package-name>
```

Its job is to convert a shell package plus shared source into a standalone
publishable artifact.

### 9.1 Inputs

- shared source tree under `code/src/<language>/`
- shell package under `code/packages/<language>/<package>/`
- package metadata
- current commit contents

### 9.2 Output

A self-contained directory:

```text
.out/publish/<language>/<package>/
```

### 9.3 Required Behavior

The tool must:

1. resolve the shell package's public wrapper entrypoint
2. resolve the corresponding shared-source package entrypoint
3. walk the transitive internal package graph
4. copy required source subtrees into the output package
5. copy required assets
6. rewrite imports so all references stay within the output tree
7. emit package metadata with no internal runtime dependencies
8. fail clearly if constraints are violated

---

## 10. Minimal First-Version Algorithm

To keep the first version small, the tool should work at the **package subtree**
level, not the individual-file reachability level.

### 10.1 Step 1: Start at a Package

Given:

```text
materialize-package typescript typescript-parser
```

Locate:

- shell package: `code/packages/typescript/typescript-parser/`
- canonical source: `code/src/typescript/typescript-parser/`

### 10.2 Step 2: Read Shared-Source Entrypoint

Start from:

```text
code/src/typescript/typescript-parser/index.ts
```

Parse imports and identify any references to sibling shared-source packages.

### 10.3 Step 3: Build Package Dependency Closure

If `typescript-parser` imports:

- `parser`
- `grammar-tools`
- `typescript-lexer`

and `typescript-lexer` imports:

- `lexer`
- `grammar-tools`

then the closure is:

- `typescript-parser`
- `typescript-lexer`
- `parser`
- `lexer`
- `grammar-tools`

### 10.4 Step 4: Copy Whole Package Subtrees

For the first implementation, copy each package's whole source subtree:

```text
code/src/typescript/parser/      -> .out/publish/typescript/typescript-parser/vendor/parser/
code/src/typescript/lexer/       -> .out/publish/typescript/typescript-parser/vendor/lexer/
```

This avoids file-level graph complexity in v1.

### 10.5 Step 5: Rewrite Imports

Any internal import in the output tree must point to copied local files.

Example:

```ts
import { GrammarParser } from "../parser/index.js";
```

inside vendored output becomes a relative path into the copied `vendor/`
directory.

The exact rewritten path depends on output layout, but it must never point
outside the output package.

### 10.6 Step 6: Emit the Package Entrypoint

The output package entrypoint should export from the materialized local code,
not from the monorepo shared source tree.

### 10.7 Step 7: Emit Clean Metadata

The published manifest must:

- keep version/name/description/scripts as appropriate
- remove internal runtime dependencies
- keep allowed external metadata only

---

## 11. Output Layout

For v1, the recommended output layout is:

```text
.out/publish/typescript/typescript-parser/
  package.json
  src/
    index.ts
  vendor/
    typescript-parser/
      index.ts
      parser.ts
    typescript-lexer/
      index.ts
      tokenizer.ts
    parser/
      index.ts
      parser.ts
      grammar-parser.ts
    lexer/
      index.ts
      grammar-lexer.ts
      tokenizer.ts
    grammar-tools/
      index.ts
      ...
```

This layout is verbose but easy to inspect and debug.

---

## 12. Asset Rules

The tool must copy assets needed by vendored packages.

### 12.1 Allowed Asset Locations

Assets must live inside the package subtree:

```text
code/src/<language>/<package>/assets/
```

or another package-local directory declared by convention.

### 12.2 Asset Access Rule

Runtime code must resolve assets relative to the current module, not relative
to the repository root.

Forbidden:

- assumptions about `/code/grammars` existing outside the package artifact
- hardcoded repo-root traversals

Required:

- module-relative asset lookup
- copied assets inside the materialized output

This is necessary because the published package no longer lives in the repo.

---

## 13. Failure Modes

The tool must fail with a clear error if it encounters:

- a package-level cycle
- a deep cross-package import
- a dynamic internal import
- an asset path that escapes the package subtree
- a shell package that contains canonical logic
- a shared-source package without a canonical `index.*`

Fast failure is preferred over partial output.

---

## 14. Why a Build Step Is Unavoidable

If shell packages point directly at shared source like:

```ts
export * from "../../../../src/typescript/lexer/index.js";
```

that works only inside the repo. Once a package is published, those external
paths do not exist in the installed artifact.

Therefore, a build/materialization step is not an implementation detail; it is
the mechanism that converts a monorepo-only source graph into a publishable,
self-contained artifact.

---

## 15. Language Portability

This architecture generalizes across languages:

- **Ruby**: shell gem files may `require_relative` shared source in-repo, but
  the published gem still needs copied source
- **Python**: shell packages may import shared modules in-repo, but wheels and
  sdists must include copied code
- **Rust**: crates may use path-based local development, but published crates
  still need vendored source inside the crate
- **Go**: module packages must exist inside the published module tree

The exact syntax differs, but the packaging boundary problem is the same in
every ecosystem.

---

## 16. Phased Rollout

### Phase 1: TypeScript Prototype

- move canonical TypeScript implementations into `code/src/typescript/`
- reduce `code/packages/typescript/*` to thin wrappers
- implement `materialize-package` for TypeScript only
- prove the model on lexer/parser families first

### Phase 2: Shared Conventions

- codify package entrypoints
- codify asset layout
- codify no-cycle / no-deep-import rules

### Phase 3: Other Languages

- port the same architecture to Ruby, Python, Go, Rust, Elixir, etc.
- adjust only syntax-specific details

---

## 17. Open Questions

These are intentionally deferred from v1:

- Should output be copied source or bundled output for TypeScript?
- Should declarations be rewritten from source or emitted from compiled output?
- Should the publish tool operate per package only, or support batch release?
- Should package dependency closure be derived entirely from imports, or also
  from an explicit package manifest?

None of these block the initial architecture.

---

## 18. Recommendation

Adopt the **shared source + shell packages + materialized publish output**
model for TypeScript first.

Specifically:

- use `code/src/typescript/<package>/` as the canonical implementation tree
- keep `code/packages/typescript/<package>/` as thin shells
- prohibit internal runtime package dependencies
- implement a deterministic materialization tool that copies package subtrees,
  rewrites imports, and emits standalone publish artifacts

This gives us a scoped, enforceable path to separately published packages
without trusting unpublished internal packages at runtime.
