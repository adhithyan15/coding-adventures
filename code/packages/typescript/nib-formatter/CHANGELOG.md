# Changelog

## 0.1.0

- add the initial `@coding-adventures/nib-formatter` TypeScript package
- lower Nib parser ASTs into `format-doc` documents using shared formatter templates
- expose both `Doc`-level and end-to-end ASCII formatting entry points
- cover ugly-input normalization, wrapping, and idempotence with unit tests
- avoid CI failures from an unrelated upstream TypeScript `lexer` error by
  relying on the package test suite in `BUILD` until the shared typecheck is fixed
