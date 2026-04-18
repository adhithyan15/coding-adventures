# Changelog

## 0.1.0

- add the initial `@coding-adventures/nib-formatter` TypeScript package
- lower Nib parser ASTs into `format-doc` documents using shared formatter templates
- expose both `Doc`-level and end-to-end ASCII formatting entry points
- cover ugly-input normalization, wrapping, and idempotence with unit tests
- preserve line comments and blank lines through the source-based formatter path
- recover EOF comments by formatting from a trivia-rich parsed document rather
  than from the AST alone
- cover top-level, block, `else`, trailing, and EOF comment behavior with unit
  tests
