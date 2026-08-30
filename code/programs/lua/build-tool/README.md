# Build Tool (Lua)

An educational implementation of the monorepo build tool in Lua 5.4.

## Purpose

This is one of several parallel implementations of the build tool, alongside
Go (primary), Python, Ruby, TypeScript, Rust, and Elixir. Each implementation
follows the same architecture and produces the same output, serving as a
teaching tool for how build systems work across different languages.

## Architecture

The build tool follows a pipeline:

1. **Discovery** (`lib/build_tool/discovery.lua`): Walk the directory tree,
   find packages with BUILD files, infer their language from the path.

2. **Resolution** (`lib/build_tool/resolver.lua`): Parse each package's
   metadata file (pyproject.toml, .gemspec, go.mod, .rockspec, etc.) to
   extract internal dependencies. Build a directed graph.

3. **Topological Sort** (`lib/build_tool/directed_graph.lua`): Use Kahn's
   algorithm to partition packages into parallel execution levels.

4. **Execution** (`lib/build_tool/executor.lua`): Run BUILD commands for
   each package, level by level.

5. **Reporting** (`lib/build_tool/reporter.lua`): Print a summary of
   pass/fail results.

6. **Validation** (`lib/build_tool/validator.lua`): Apply process-free policy
   to caller-supplied snapshots. Tracked dependency artifacts consume all five
   language-neutral fixtures and reject unsafe or Unicode-compatible
   `node_modules` paths without reading a checkout. Cargo/BUILD/ledger
   snapshots consume all four orphan-crate fixtures with the same pure
   boundary.

7. **Toolchain Detection** (`lib/build_tool/toolchain_detection.lua`): Evaluate
   caller-supplied BUILD snapshots for canonical package languages and exact
   extra-CI declarations. The pure adapter owns platform-front precedence,
   byte-exact CRLF grammar, scheduled and forced toolchain unions, and the
   shared byte, line, and aggregate resource ceilings.

## Usage

```bash
lua build.lua                          # Build all packages
lua build.lua --root /path/to/repo     # Specify root
lua build.lua --dry-run                # Show what would build
lua build.lua --language python        # Only build Python packages
lua build.lua --force                  # Rebuild everything
```

## Lua-Specific Design Notes

- **Tables as everything**: The directed graph uses tables for adjacency
  lists, sets, and node storage. No separate Set or Map types needed.
- **No threading**: Standard Lua has no built-in threading, so builds run
  sequentially. LuaJIT or Lua lanes could add parallelism, but the
  educational value is in the algorithm, not the concurrency.
- **Optional LuaFileSystem**: Uses `lfs` if available for directory listing,
  falls back to `ls`/`dir` shell commands otherwise.
- **Metatables for OOP**: DirectedGraph uses the standard `__index`
  metatable pattern for method dispatch.

## Metadata Safety

Lua `.rockspec` package metadata is decoded as strict UTF-8 before dependency
resolution. Invalid bytes fail closed with `METADATA_INVALID_UTF8`, identify
the package and repository-relative manifest, and make the CLI exit with code
2. Diagnostics never expose the checkout path or silently replace invalid
input bytes; a well-formed literal Unicode replacement character remains valid.

## Tracked-Artifact Validation

`Validator.validate_tracked_artifact_snapshot(entries, unicode_version)` is a
pure adapter over inert path and entry-kind records. It performs lexical slash
normalization, rejects non-portable paths with redacted diagnostics, detects
exact, case, nested, and Unicode compatibility aliases of `node_modules`, and
sorts diagnostics by Unicode scalar value. Regular, symlink, and reparse kinds
are policy metadata only; the adapter never opens or follows them.

The adapter uses generated, source-embedded Unicode 17.0.0 NFC, NFKC, full
default case-fold, and full-uppercase tables. It therefore does not inherit
normalization or casing behavior from the installed Lua runtime. Regenerate
the module and its Unicode License v3 notice with:

```bash
python code/scripts/generate_tracked_artifact_unicode17.py \
  --lua-executable .lua/bin/lua
```

## Orphan-Crate Validation

`Validator.validate_orphan_crate_snapshot(snapshot)` accepts only caller-owned
tables describing Cargo manifest directories, recognized BUILD records, and
bounded exemption-ledger entries. It ignores exact artifact components,
selects the closest runnable ancestor BUILD with the contract's fixed filename
rank, reports closer empty BUILDs without letting them mask runnable ancestors,
and validates `EXCLUDED` and `PENDING` entries with stable redacted failures.

Portable paths use the same source-embedded Unicode 17 NFC, full-fold, and
full-uppercase data as tracked-artifact validation. Duplicate exemption
identities, Python-exact blank reasons, stale entries, active pending counts,
and diagnostics are therefore deterministic across Lua hosts. The adapter does
not enumerate the filesystem, inspect Git, follow links, launch processes,
read the environment, or access the network.

## Extra CI Toolchain Declarations

`ToolchainDetection.evaluate_snapshot(platform, force_full, packages,
scheduled_packages, forced_toolchains)` is the language-native conformance
boundary for toolchain detection. It returns every canonical toolchain flag on
success and the stable `TOOLCHAIN_UNSUPPORTED` diagnostic for an unsupported
selected language or forced toolchain. The package-local Busted suite discovers
and consumes all 11 language-neutral `toolchain-detection-*.json` fixtures.

The adapter accepts only caller-owned strings. It does not enumerate a
checkout, read the environment, launch processes, or access the network.
Production CLI integration is intentionally a separate tranche because the
current Lua front door does not yet expose affected-package or emitted-plan
semantics.

## Dependencies

- Lua 5.4 (for native integers and bitwise operators)
- LuaFileSystem (optional for faster discovery and used by tests to enumerate
  the shared fixture corpus)
- DKJSON (tests only, for consuming the shared JSON fixture corpus)
