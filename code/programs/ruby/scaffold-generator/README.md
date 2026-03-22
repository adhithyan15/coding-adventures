# scaffold-generator (Ruby)

Generate CI-ready package scaffolding for the coding-adventures monorepo.

## What This Tool Does

This CLI tool generates correctly-structured, CI-ready package directories for all six languages: Python, Go, Ruby, TypeScript, Rust, and Elixir. It eliminates the recurring CI failures documented in lessons.md by producing packages that compile, lint, and pass tests out of the box.

## How CLI Builder Powers This

The entire CLI interface is defined in `scaffold-generator.json`. This program never parses a single argument by hand. CLI Builder handles all parsing, validation, and help generation.

## Architecture

```
scaffold-generator.json (spec)     generator.rb (this tool)
+-----------------------------+    +----------------------------------+
| flags: -t, -l, -d, etc.    |    | 1. Parse argv via cli-builder    |
| argument: PACKAGE_NAME     |--->| 2. Resolve dependencies          |
| help, version, validation  |    | 3. Generate files per language   |
+-----------------------------+    +----------------------------------+
    CLI Builder handles this            Your code handles this
```

## Usage

```bash
# Generate a library for all 6 languages
ruby scaffold_generator_tool.rb my-package --description "A cool package"

# Generate for a specific language
ruby scaffold_generator_tool.rb my-package -l ruby --description "A cool package"

# With dependencies and layer info
ruby scaffold_generator_tool.rb my-package -l python -d logic-gates,arithmetic --layer 5

# Dry run (preview what would be generated)
ruby scaffold_generator_tool.rb my-package --dry-run
```

## Dependencies

- coding_adventures_cli_builder (CLI argument parsing)
- coding_adventures_state_machine (transitive dep of cli_builder)
- coding_adventures_directed_graph (transitive dep of state_machine)

## Development

```bash
# Run tests
bundle install --quiet && bundle exec rake test
```
