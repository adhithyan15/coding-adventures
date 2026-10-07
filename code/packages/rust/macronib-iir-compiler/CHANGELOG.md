# Changelog

## 0.1.0

- Compose MacroNib with the generic preprocessor and Nib's existing frontend.
- Check primary and included source against byte and token budgets before
  lexer allocation, including an aggregate cap across nested includes; bound
  all supplied in-memory includes before copying them.
- Add `.include`, object-like `.set`, and `.ifdef`/`.else`/`.endif` handling.
- Verify direct IIR identity against hand-expanded Nib source, including an
  eight-backend matrix row.
