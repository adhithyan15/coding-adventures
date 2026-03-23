# scaffold-generator

A CLI tool that generates CI-ready package scaffolding for the coding-adventures
monorepo across all six languages: Python, Go, Ruby, TypeScript, Rust, and Elixir.

## Why

The lessons.md documents 12+ recurring CI failure categories caused by agents
hand-crafting packages inconsistently. This tool eliminates those failures by
producing correct-by-construction packages.

## Usage

```bash
# Scaffold a Python library with dependencies
scaffold-generator my-package --language python --depends-on arithmetic,logic-gates --description "My new package"

# Scaffold across all 6 languages
scaffold-generator my-package --language all --description "My new package"

# Preview without creating files
scaffold-generator my-package --dry-run

# Scaffold a program (goes in code/programs/ instead of code/packages/)
scaffold-generator my-tool --type program --language go
```

## Build

```bash
go build -o scaffold-generator .
```

## Development

```bash
bash BUILD
```
