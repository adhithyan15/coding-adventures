# Scaffold Generator

Generate CI-ready Dart package and program scaffolding for the
coding-adventures monorepo.

## What it does

This is the Dart bootstrap implementation of `scaffold-generator`. It focuses
on the Dart lane first so future Dart ports can start from a correct package
layout instead of hand-crafting `pubspec.yaml`, `BUILD`, `README.md`,
`CHANGELOG.md`, `.gitignore`, `lib/`, `bin/`, and `test/` files.

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

## Development

```bash
bash BUILD
```
