# scaffold-generator (TypeScript)

Generate CI-ready package scaffolding for the coding-adventures monorepo.

## What It Does

This CLI tool generates correctly-structured, CI-ready package directories
for the package ecosystems currently scaffolded in the repo:
Python, Go, Ruby, TypeScript, Rust, Elixir, Perl, Haskell, C#, and F#.

It is powered by [cli-builder](../../../packages/typescript/cli-builder/) --
the entire command-line interface is defined in a JSON spec file, and this
program contains only the business logic.

## Why It Exists

The lessons.md file documents 12+ recurring categories of CI failures caused
by agents hand-crafting packages inconsistently:

- Missing BUILD files
- TypeScript `main` pointing to `dist/` instead of `src/`
- Missing transitive dependency installs in BUILD files
- Ruby require ordering (deps before own modules)
- Rust workspace Cargo.toml not updated

This tool eliminates those failures. Run it, get a package that compiles,
lints, and passes tests. Then fill in the business logic.

## Usage

```bash
# Generate a library package for all supported scaffold languages
npx tsx src/index.ts my-package --description "My awesome package" --layer 5

# Generate for a specific language
npx tsx src/index.ts my-package -l typescript --description "TS only"

# Generate a program (goes in code/programs/ instead of code/packages/)
npx tsx src/index.ts my-tool -t program -l go

# Generate a .NET package
npx tsx src/index.ts graph -l csharp --description "Undirected graph"
npx tsx src/index.ts graph -l fsharp --description "Undirected graph"

# Specify dependencies
npx tsx src/index.ts my-package -d logic-gates,arithmetic

# Dry run (see what would be generated without writing files)
npx tsx src/index.ts my-package --dry-run
```

## Dependencies

- [@coding-adventures/cli-builder](../../../packages/typescript/cli-builder/) -- Declarative CLI argument parsing

## Development

```bash
# Run tests
bash BUILD
```

The generated TypeScript packages enforce a line coverage threshold of 80% in
their `vitest.config.ts`, and the C#/F# templates run `dotnet test` with an
80% line threshold plus Coverlet wired in by default.
