# Build Plan Specification — v1

## Overview

The **build plan** is a versioned JSON manifest that serializes the build tool's
discovery, dependency resolution, and change detection results. It enables
cross-job communication in CI: the `detect` job computes the plan once, and all
`build` jobs on different platforms consume it — eliminating redundant computation.

## Motivation

The CI pipeline has two jobs:

1. **detect** — compiles the build tool, discovers packages, resolves
   dependencies, runs git diff, and determines which languages need toolchains.
2. **build** — installs toolchains, compiles the build tool again, and repeats
   all of the above before actually running builds.

Steps 1–5 of the 11-step build flow are pure computation with no
platform-specific behavior. By serializing these results as a JSON artifact,
the build job skips straight to step 6 (hashing), saving time on each of the
3 CI platforms.

## JSON Schema

```json
{
  "$schema": "https://json-schema.org/draft-07/schema#",
  "title": "Build Plan v1",
  "description": "Serialized build plan for cross-job/cross-platform build orchestration.",
  "type": "object",
  "required": ["schema_version", "packages", "dependency_edges", "languages_needed"],
  "additionalProperties": true,
  "properties": {
    "schema_version": {
      "type": "integer",
      "const": 1,
      "description": "Schema version number. Readers MUST reject plans with a schema_version higher than what they support, falling back to the normal discovery flow."
    },
    "diff_base": {
      "type": "string",
      "description": "Git ref used for change detection (e.g., 'origin/main', 'HEAD~1'). Informational — the build job does not re-run git diff."
    },
    "force": {
      "type": "boolean",
      "default": false,
      "description": "When true, all packages should be rebuilt regardless of change detection or cache."
    },
    "affected_packages": {
      "type": ["array", "null"],
      "items": { "type": "string" },
      "description": "Qualified names of packages that need building. Semantics: null means 'all packages' (force mode or git diff unavailable). An empty array [] means 'nothing changed — build nothing'. A non-empty array lists specific affected packages."
    },
    "packages": {
      "type": "array",
      "description": "ALL discovered packages, not just affected ones. The executor needs the full list to compute 'dep-skipped' status and the complete dependency graph.",
      "items": {
        "$ref": "#/$defs/package_entry"
      }
    },
    "dependency_edges": {
      "type": "array",
      "description": "Directed edges representing the dependency graph. Each edge is [from, to] where from→to means 'to depends on from' (i.e., from must be built before to).",
      "items": {
        "type": "array",
        "items": { "type": "string" },
        "minItems": 2,
        "maxItems": 2
      }
    },
    "languages_needed": {
      "type": "object",
      "description": "Map of language name to boolean indicating whether that language's toolchain is needed for this build.",
      "additionalProperties": { "type": "boolean" }
    },
    "shards": {
      "type": "array",
      "description": "Optional prerequisite-closed build shards for multi-runner CI execution. See build-plan-sharding.md.",
      "items": {
        "$ref": "#/$defs/shard_entry"
      }
    }
  },
  "$defs": {
    "package_entry": {
      "type": "object",
      "required": ["name", "rel_path", "language", "build_commands"],
      "additionalProperties": true,
      "properties": {
        "name": {
          "type": "string",
          "description": "Qualified package name: 'language/package-name' (e.g., 'python/starlark-vm')."
        },
        "rel_path": {
          "type": "string",
          "description": "Path relative to the repository root. MUST use forward slashes (/) as separator on all platforms. Consumers convert to platform-native separators on read."
        },
        "language": {
          "type": "string",
          "enum": ["python", "ruby", "go", "typescript", "rust", "elixir", "starlark", "unknown"],
          "description": "The package's programming language, inferred from its path."
        },
        "build_commands": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Shell commands to execute for building/testing. For Starlark BUILD files, these are generated from the declared rule. For shell BUILD files, these are the raw lines."
        },
        "is_starlark": {
          "type": "boolean",
          "default": false,
          "description": "Whether the BUILD file uses Starlark syntax (as opposed to raw shell commands)."
        },
        "declared_srcs": {
          "type": "array",
          "items": { "type": "string" },
          "default": [],
          "description": "Glob patterns declaring which source files are inputs to this package. From the Starlark srcs field. Used for strict input hashing and git diff filtering."
        },
        "declared_deps": {
          "type": "array",
          "items": { "type": "string" },
          "default": [],
          "description": "Qualified names of packages this package depends on, as declared in the Starlark deps field."
        }
      }
    },
    "shard_entry": {
      "type": "object",
      "required": ["index", "name", "assigned_packages", "package_names"],
      "additionalProperties": true,
      "properties": {
        "index": {
          "type": "integer",
          "description": "Stable shard index used by CI matrix jobs."
        },
        "name": {
          "type": "string",
          "description": "Human-readable shard label, e.g. shard-1-of-5."
        },
        "assigned_packages": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Packages directly assigned to this shard."
        },
        "package_names": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Assigned packages plus transitive prerequisites; this is the package set the shard builds."
        },
        "languages_needed": {
          "type": "object",
          "additionalProperties": { "type": "boolean" },
          "description": "Toolchains needed by this shard."
        },
        "estimated_cost": {
          "type": "integer",
          "description": "Heuristic cost used for balancing shards."
        }
      }
    }
  }
}
```

## Versioning Strategy

### Rules

1. **`schema_version`** is a required integer field. It starts at `1` and
   increments monotonically. It is NOT semver — just an integer.

2. **Readers MUST** check `schema_version` before parsing. If the version is
   higher than what the reader supports, it MUST reject the plan gracefully
   (log a warning, fall back to the normal discovery flow). It MUST NOT crash.

3. **Writers MUST** set `schema_version` to the version they implement. The
   `Write()` function enforces this — callers do not set it manually.

4. **Forward compatibility**: Both the top-level object and `package_entry`
   have `additionalProperties: true`. New optional fields can be added in
   future versions without breaking v1 readers (they simply ignore unknown
   fields).

5. **Breaking changes** require incrementing `schema_version`. A change is
   breaking if a v1 reader would misinterpret the plan or fail silently:
   - Removing or renaming a required field
   - Changing the type or semantics of an existing field
   - Restructuring arrays/objects (e.g., changing edge format)

6. **Additive changes** do NOT require a version bump:
   - Adding new optional fields
   - Adding new enum values to `language`
   - Adding new fields to `package_entry`

### Evolution Examples

| Change | Version Bump? | Rationale |
|--------|:---:|-----------|
| Add optional `build_timeout` to package_entry | No | Additive — v1 readers ignore it |
| Add optional `platform_overrides` top-level field | No | Additive |
| Add optional `shards` top-level field | No | Additive |
| Add `"zig"` to the language enum | No | Additive |
| Rename `rel_path` → `path` | **Yes → v2** | Breaking — v1 readers expect `rel_path` |
| Change edges from `[[a,b]]` to `[{from:a, to:b}]` | **Yes → v2** | Breaking — structural change |
| Make `declared_srcs` required (currently optional) | **Yes → v2** | Breaking — v1 writers may omit it |
| Add optional `hashes` field | No | Additive |

## Path Conventions

- All paths in the plan use **forward slashes** (`/`) regardless of the
  platform that produced the plan.
- On write: `filepath.ToSlash(rel)` (Go), `path.replace("\\", "/")` (others).
- On read: `filepath.FromSlash(relPath)` (Go), `os.path.join()` (Python),
  `File.join()` (Ruby), `path.join()` (Node), etc.
- Paths are relative to the repository root. The consumer joins them with its
  own `repoRoot` to produce absolute paths.

## Semantics of `affected_packages`

| Value | Meaning |
|-------|---------|
| `null` | Rebuild all packages. Used when `force` is true or git diff is unavailable. |
| `[]` (empty array) | Nothing changed — build nothing. The build job skips execution entirely. |
| `["python/foo", "go/bar"]` | Only these packages (and their dependents, already computed) need building. |

The distinction between `null` and `[]` is critical. JSON serialization
preserves this: `null` serializes as `null`, `[]` serializes as `[]`.

## CLI Integration

### `--emit-plan <path>`

Runs steps 1–5 of the build flow (discover, evaluate Starlark, filter,
resolve deps, git diff), then serializes the results to `<path>` as JSON.
Can be combined with `--detect-languages` to also output language flags.
Exits after writing the plan.

### `--plan-file <path>`

Reads a build plan from `<path>` and reconstructs the in-memory state
(packages, dependency graph, affected set). Skips steps 1–5 and proceeds
directly to step 6 (hashing). If the file is missing, unparseable, or has
an unsupported `schema_version`, logs a warning and falls back to the
normal flow.

## CI Usage

```yaml
# Detect job: emit plan + language flags
- run: ./build-tool -root . -diff-base $BASE -detect-languages -emit-plan build-plan.json

- uses: actions/upload-artifact@v4
  with:
    name: build-plan
    path: build-plan.json
    retention-days: 1

# Build job: consume plan
- uses: actions/download-artifact@v4
  with:
    name: build-plan
  continue-on-error: true

- run: |
    PLAN=""
    if [ -f build-plan.json ]; then PLAN="-plan-file build-plan.json"; fi
    ./build-tool -root . $PLAN -language all
```

## Cross-Language Compatibility

All 6 build tool implementations (Go, Python, Ruby, TypeScript, Rust, Elixir)
read and write the same JSON schema. A plan emitted by the Go build tool can
be consumed by the Python build tool, and vice versa.
