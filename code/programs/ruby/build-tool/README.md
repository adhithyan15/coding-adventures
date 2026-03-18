# Build Tool (Ruby)

A Ruby port of the Python monorepo build tool. Discovers packages via DIRS/BUILD files, resolves dependencies from package metadata, hashes source files for change detection, caches build state, and executes builds in parallel respecting dependency order.

## How It Fits in the Stack

This is a standalone program (not a publishable gem) that orchestrates building all packages in the coding-adventures monorepo. It is a direct port of the Python build tool, sharing the same DIRS/BUILD file conventions and cache format.

## Architecture

| Module       | Responsibility                                      |
|-------------|-----------------------------------------------------|
| `discovery`  | Walk DIRS files, find packages with BUILD files     |
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

## Testing

```bash
bundle install
bundle exec rake test
```

## Dependencies

Zero runtime gem dependencies -- uses only Ruby standard library modules:
- `json` for cache serialization
- `digest/sha2` for file hashing
- `open3` for subprocess execution
- `pathname` for path manipulation
- `optparse` for CLI argument parsing
- `set` for efficient set operations

Dev dependencies: minitest, rake, simplecov.
