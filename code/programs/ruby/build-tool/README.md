# Build Tool (Ruby)

A Ruby port of the Python monorepo build tool. It discovers packages via recursive `BUILD` file walking, resolves dependencies from package metadata, hashes source files for change detection, caches build state, and executes builds in parallel respecting dependency order.

## How It Fits in the Stack

This is a standalone program (not a publishable gem) that orchestrates building all packages in the coding-adventures monorepo. It is a direct port of the Python build tool, sharing the same recursive `BUILD` discovery model and cache format.

## Architecture

| Module       | Responsibility                                      |
|-------------|-----------------------------------------------------|
| `discovery`  | Recursively walk directories, find packages with BUILD files |
| `resolver`   | Parse metadata, build a dependency graph            |
| `hasher`     | SHA256 hashing for change detection                 |
| `cache`      | JSON cache file for incremental builds              |
| `executor`   | Parallel execution via threads + Open3              |
| `reporter`   | Human-readable build report formatting              |

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

## Testing

```bash
bundle install
bundle exec rake test
```

The `test/test_resolution_utf8.rb` coverage exercises the shared
language-neutral valid and invalid fixtures, representative malformed sequence
classes, and the real CLI subprocess. The full Rake suite enforces the
whole-program coverage threshold. Starlark evaluator tests are mandatory: the
suite removes ambient `RUBYLIB` and `RUBYOPT` injection in a subprocess and
proves that the repository-local interpreter loads through the build tool's
declared bundle rather than skipping when it is unavailable.

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
