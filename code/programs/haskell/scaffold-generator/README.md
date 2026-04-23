# scaffold-generator

Haskell scaffold generator for the monorepo's Haskell packages and programs.

## What it does

This program creates starter Haskell package or program directories with:

- a `cabal` file
- a `cabal.project` that links local sibling dependencies
- starter source and test modules
- repo-standard `BUILD` files
- `README.md` and `CHANGELOG.md`

## Usage

```bash
# Create a library under code/packages/haskell/
cabal run scaffold-generator -- logic-wizard

# Create a program under code/programs/haskell/
cabal run scaffold-generator -- --type program --depends-on logic-gates build-helper
```
