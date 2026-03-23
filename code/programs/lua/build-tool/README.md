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

## Dependencies

- Lua 5.4 (for native integers and bitwise operators)
- LuaFileSystem (optional, for faster directory traversal)
