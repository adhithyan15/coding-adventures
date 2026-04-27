# Coding Adventures

Coding Adventures is a learning-first monorepo for building the computing stack
ourselves: data structures, language tooling, runtimes, protocol layers,
architecture experiments, and the tooling needed to support them.

This root README is intentionally small. Package-level documentation lives with
each package, and package READMEs are the authoritative place for package
usage, API notes, and language-specific details.

## Repository Layout

```text
code/
├── learning/   Explanatory notes and teaching material
├── packages/   Publishable libraries grouped by language
├── programs/   Standalone tools, demos, and build infrastructure
└── specs/      Specifications and architecture documents
```

Useful entry points:

- `code/specs/`
- `code/packages/`
- `code/programs/`
- `code/learning/`

## How Work Usually Flows

1. Write or refine the spec.
2. Add or update the learning material.
3. Implement the package or program.
4. Add tests and verify behavior.
5. Update the package README and changelog.

## Copyright And Licensing

Copyright © Adhithya Rajasekaran. All rights reserved unless a package
explicitly provides different licensing terms.

If a package includes its own license file or declares a license in package
metadata, that package-specific license governs that package. Otherwise, treat
the code as all rights reserved.
