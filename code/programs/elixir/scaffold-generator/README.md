# scaffold-generator (Elixir)

Generates CI-ready package scaffolding for the coding-adventures monorepo.

This is the Elixir implementation of the scaffold-generator program, ported from the Go reference implementation. It supports all six languages (Python, Go, Ruby, TypeScript, Rust, Elixir) and handles:

- Name normalization (kebab-case to snake_case, CamelCase, joinedlower)
- Dependency reading from existing packages' metadata files
- Transitive closure via BFS
- Topological sort via Kahn's algorithm (leaf-first install order)
- File generation with correct BUILD files, README, CHANGELOG

## Usage

```bash
# Generate a package for all languages
./scaffold-generator my-package --description "My new package"

# Generate for specific languages only
./scaffold-generator my-package -l python,go --description "My new package"

# With dependencies
./scaffold-generator cpu-core -d logic-gates,registers --layer 5

# Dry run (see what would be created)
./scaffold-generator my-package --dry-run

# Show help
./scaffold-generator --help
```

## Development

```bash
# Run tests
bash BUILD

# Or manually
mix deps.get
mix test --cover
```

## Architecture

The program has three modules:

- `CodingAdventures.ScaffoldGenerator` — Core algorithms (name normalization, dependency resolution, file generation)
- `CodingAdventures.ScaffoldGenerator.CLI` — Command-line argument parsing using OptionParser
- `CodingAdventures.ScaffoldGenerator.Config` — Configuration struct
