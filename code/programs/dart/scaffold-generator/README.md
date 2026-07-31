# Scaffold Generator

Generate CI-ready Dart package and program scaffolding for the
coding-adventures monorepo.

## What it does

This is the Dart bootstrap implementation of `scaffold-generator`. It focuses
on the Dart lane first so future Dart ports can start from a correct package
layout instead of hand-crafting `pubspec.yaml`, `BUILD`, `README.md`,
`CHANGELOG.md`, `required_capabilities.json`, `.gitignore`, `lib/`, `bin/`,
and `test/` files.

## Usage

```bash
dart run bin/scaffold_generator.dart my-package --description "My new package"
dart run bin/scaffold_generator.dart my-tool --type program
dart run bin/scaffold_generator.dart nib-lexer --depends-on lexer,grammar-tools
dart run bin/scaffold_generator.dart parser --dry-run
```

## Current scope

- Scaffolds Dart libraries under `code/packages/dart/`
- Scaffolds Dart programs under `code/programs/dart/`
- Validates direct dependencies against the existing Dart tree
- Computes the transitive Dart dependency closure from `pubspec.yaml`
- Uses the shared Dart `cli-builder` package for argument parsing
- Emits byte-stable Spec 13 schema-v1 capability metadata: an empty profile for
  libraries and the generated program's truthful `stdout:write:*` profile

## Capability contract

The generator itself declares the filesystem read/create/write, clock-read,
and standard-output authority exercised by repository discovery, dependency
planning, scaffold creation, dated changelogs, previews, and diagnostics. It
derives a fixed repository root from its own checked-in package location, and
exposes no callable public library API: the checked-in `bin/` entrypoint imports
the internal implementation directly. It does not request network, subprocess,
environment, FFI, or stdin access.

Generated profiles are checked byte-for-byte against language-neutral golden
fixtures and separately validated against the shared Draft 2020-12 schema.
Existing nonempty Dart package profiles are outside this generator contract and
remain subject to their dedicated migration and Layer 5 review.

## Development

```bash
bash BUILD
dart analyze
```
