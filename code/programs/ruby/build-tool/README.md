# Build Tool (Ruby)

A Ruby port of the Python monorepo build tool. It discovers packages via recursive `BUILD` file walking, resolves dependencies from package metadata, hashes source files for change detection, caches build state, and executes builds in parallel respecting dependency order.

## How It Fits in the Stack

This is a standalone program (not a publishable gem) that orchestrates building all packages in the coding-adventures monorepo. It is a direct port of the Python build tool, sharing the same recursive `BUILD` discovery model and cache format.

## Architecture

| Module | Responsibility |
|---|---|
| `discovery` | Recursively walk directories, find packages with BUILD files |
| `resolver` | Parse metadata, build a dependency graph |
| `hasher` | SHA256 hashing for change detection |
| `cache` | JSON cache file for incremental builds |
| `executor` | Parallel execution via threads + Open3 |
| `reporter` | Human-readable build report formatting |
| `toolchain_detection` | Pure bounded extra-CI toolchain snapshot decisions |
| `validator` | Pure orphan-crate and tracked-artifact snapshot policy validation |

## Usage

```bash
ruby build.rb                        # Auto-detect root, build changed packages
ruby build.rb --root /path/to/repo   # Specify root explicitly
ruby build.rb --force                # Rebuild everything
ruby build.rb --dry-run              # Show what would build without building
ruby build.rb --jobs 4               # Limit parallel workers
ruby build.rb --language python      # Only build Python packages
ruby build.rb --cache-file FILE      # Custom cache file path
```

## Metadata Safety

Lua rockspecs are read as raw bytes and decoded as strict UTF-8 before any
dependency parsing. Malformed metadata fails closed with CLI exit code `2` and
a stable diagnostic that contains only portable identities:

```text
METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8
```

The resolver never inserts replacement characters and never includes the host
checkout root in this error. A literal U+FFFD character encoded correctly as
UTF-8 remains valid metadata.

## Canonical Discovery and Identities

Discovery uses the repository's canonical language registry and treats only
the exact component immediately below `packages` or `programs` as the language
bucket. Programs retain a `programs` identity segment, such as
`go/programs/build-tool`, so a library and program with the same basename stay
distinct. Specification fixture trees are excluded, and duplicate qualified
identities fail closed with exit code `2` and repository-relative paths.
Cabal's exact, case-sensitive `dist-newstyle` directory component and Dune's
exact, case-sensitive `_build` component are also excluded as generated build
output. Source directories such as `Dist-Newstyle`, `dist-newstyle-example`,
`_Build`, and `_build-example` remain discoverable.

The resolver preserves those identities in metadata edges and in qualified
dependencies declared by the selected legacy BUILD file's
`# build-tool: deps=` comment. The shared discovery, duplicate-identity,
package/program, and BUILD-comment fixtures exercise the same contracts used by
other build-tool implementations.

## Extra CI Toolchain Declarations

`BuildTool::ToolchainDetection.evaluate_snapshot` accepts only caller-owned
package records and inline BUILD-front strings. It selects the exact Windows,
Darwin, or Linux front, parses inert `# needs-toolchain: NAME` comments, and
returns the complete sorted 16-key toolchain map for the supplied affected,
forced, or full-rebuild decision. It never reads a checkout, probes PATH,
consults the environment, invokes Git, starts a process, or accesses a network.

Every supplied BUILD front is validated before selection: one front is limited
to 65,536 encoded UTF-8 bytes and 4,096 logical lines, and the complete snapshot
is limited to 1 MiB. Parsing uses literal LF boundaries, strips one CR only
when it forms a CRLF terminator, trims only ASCII space and tab, and stably
deduplicates exact lowercase canonical declarations. Unsupported selected
languages and explicitly forced values return the stable
`TOOLCHAIN_UNSUPPORTED` diagnostic.

## Canonical Starlark BUILD Files

Canonical Starlark files are evaluated with the normalized build-tool v1
context (`version`, `os`, `arch`, `cpu_count`, `ci`, and `repo_root`). Loaded
rule functions retain their defining module globals, and returned structured
commands are validated as `program` plus string `args` before being rendered
deterministically for the existing executor boundary.

After discovery classifies a file as Starlark, parsing, loading, evaluation,
target selection, and structured-command extraction are fail-closed. The tool
does not reinterpret a failed Starlark file as legacy shell commands. Legacy
fallback remains available only when a valid selected target intentionally
provides no structured command list. Failure diagnostics redact the checkout
root while retaining the package identity and stable evaluation detail.

## Tracked Artifact Validation

`BuildTool::Validator.validate_tracked_artifact_snapshot` validates an
in-memory snapshot of tracked repository entries without reading the
filesystem or following links. It applies the shared build-tool v1 portable
path policy in the required precedence order, normalizes `\` to `/`, uses
Unicode scalar lengths and ordering, treats entry kinds as inert metadata, and
returns deterministic diagnostics. Invalid hostile paths are redacted to
`repository`; forbidden-component diagnostics retain their normalized safe
repository-relative path.

The validator uses the generated `TrackedArtifactUnicode17` module for pinned
Unicode 17 NFC, NFKC, full default case folding, and uppercase behavior instead
of inheriting the host Ruby runtime's Unicode tables. Regenerate and verify all
language targets with:

```bash
(cd code/programs/typescript/build-tool && npm ci)
python code/scripts/generate_tracked_artifact_unicode17.py
python code/scripts/generate_tracked_artifact_unicode17.py --check
```

The generated Unicode data is redistributed under the Unicode License v3;
the complete notice is shipped as `UNICODE-LICENSE.txt`.

## Orphan Crate Validation

`BuildTool::Validator.validate_orphan_crate_snapshot` accepts only a closed
Hash of Cargo-manifest directories, recognized BUILD records, and exemption
records. It does not enumerate a checkout, inspect Git, read files, launch a
process, consult environment state, or access the network. This keeps native
discovery authority outside the language-neutral policy adapter.

The validator derives direct and component-wise ancestor coverage, prefers the
closest runnable BUILD in the fixed platform-name order, and keeps a nearer
empty BUILD from masking a runnable ancestor. Exact case-sensitive artifact
components are excluded. Uncovered and empty crates, malformed exemptions,
stale ledger entries, and the active PENDING count are returned as stable,
sorted diagnostics.

Portable exemption paths are NFC repository-relative directories beneath
`code/`. Invalid paths are always redacted to `code/BUILD-EXEMPTIONS`; raw host
or hostile values never enter diagnostics. Duplicate identities use pinned
Unicode 17 NFC plus full case folding, and detail ordering uses Python-compatible
ASCII JSON so Ruby produces the same result as every other engine.

## Testing

```bash
bundle install
bundle exec rake test
```

The `test/test_toolchain_detection.rb` suite independently consumes all 11
language-neutral declaration fixtures and covers UTF-8 representation,
resource ceilings, CRLF grammar, empty-front precedence, selection, forcing,
freshness, and caller-input preservation. The `test/test_identity_registry.rb`
and `test/test_resolution_utf8.rb` coverage
exercise shared language-neutral discovery, dependency, valid-text, and
invalid-text fixtures plus real CLI subprocesses. `test/test_validator.rb`
consumes every shared orphan-crate and tracked-artifact fixture and covers
hostile-path redaction, Unicode 17 sentinels, Python-compatible blank reasons,
component-wise ancestry, fixed BUILD ranking, separator normalization,
deterministic ordering, and inert entry kinds. The full Rake suite enforces the whole-program
coverage threshold. Starlark evaluator tests are mandatory: the suite removes
ambient `RUBYLIB` and `RUBYOPT` injection in a subprocess and proves that the
repository-local interpreter loads through the build tool's declared bundle
rather than skipping when it is unavailable.

## Dependencies

The build tool has no third-party runtime gems. Its source-tree runtime closure
is repository-owned:

- `coding_adventures_progress_bar` provides progress reporting.
- `coding_adventures_starlark_interpreter` and its transitive repository gems
  are imported from the interpreter's authoritative Gemfile.

After `bundle install`, `bundle exec ruby build.rb ...` loads that closure with
no manual Ruby search path and performs no network resolution during build
execution. The remaining runtime modules come from the Ruby standard library:

- `json` for cache serialization
- `digest/sha2` for file hashing
- `open3` for subprocess execution
- `pathname` for path manipulation
- `optparse` for CLI argument parsing
- `set` for efficient set operations

Development dependencies: minitest, rake, simplecov.
