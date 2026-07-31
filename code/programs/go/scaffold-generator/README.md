# scaffold-generator

A CLI tool that generates CI-ready package scaffolding for the coding-adventures
monorepo across 15 languages: Python, Go, Ruby, TypeScript, Rust, Elixir, Perl,
Lua, Swift, Haskell, OCaml, Java, Kotlin, C, and C++.

Haskell scaffolds include a schema-v1 `required_capabilities.json` whose package
identity matches the generated directory. The manifest starts with an explicit
empty capability profile for the pure library template.

OCaml library and program scaffolds match the shared OCAML02 golden trees.
They include exact direct opam/Dune tool constraints, Alcotest, formatting and
coverage commands, local dependency pins, and schema-v1 capability profiles.
Transitive opam-repository/switch locking remains a separate CI-toolchain gate.

## Why

The lessons.md documents 12+ recurring CI failure categories caused by agents
hand-crafting packages inconsistently. This tool aims to eliminate those
failures as each language template completes the scaffold contract; remaining
cross-template exceptions are tracked in the parity backlog.

## Usage

```bash
# Scaffold a Python library with dependencies
scaffold-generator my-package --language python --depends-on arithmetic,logic-gates --description "My new package"

# Scaffold across all supported languages
scaffold-generator my-package --language all --description "My new package"

# Preview without creating files
scaffold-generator my-package --dry-run

# Scaffold a program (goes in code/programs/ instead of code/packages/)
scaffold-generator my-tool --type program --language go

# Scaffold an OCaml library
scaffold-generator my-package --language ocaml --description "Pure OCaml logic"
```

## Build

```bash
go build -o scaffold-generator .
```

## Development

```bash
bash BUILD
```
