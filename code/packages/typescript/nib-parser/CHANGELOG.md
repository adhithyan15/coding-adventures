# Changelog

## 0.1.0

- add a TypeScript Nib parser wrapper over the shared grammar-driven parser
- add opt-in `preserveSourceInfo` support to propagate trivia-rich source data
  onto grammar AST nodes
- add `parseNibDocument()` for formatter-style callers that need both the AST
  and the original token stream, including EOF trivia
