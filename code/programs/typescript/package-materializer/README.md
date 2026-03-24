# Package Materializer (TypeScript)

Materializes standalone publish artifacts for TypeScript packages that use the
shared-source plus shell-package architecture.

## What it does

Given a package name, this program:

- reads package metadata from `code/packages/typescript/<pkg>/`
- reads canonical source from `code/src/typescript/<pkg>/`
- computes the transitive shared-source closure for internal dependencies
- copies that closure into a standalone output tree
- generates top-level wrapper files for the target package
- copies shared token grammars from `code/src/tokens/`
- removes internal runtime dependencies from the emitted package manifest

## Usage

```bash
node --experimental-strip-types src/index.ts typescript-lexer
```

Optional output root:

```bash
node --experimental-strip-types src/index.ts typescript-lexer /tmp/materialized
```
