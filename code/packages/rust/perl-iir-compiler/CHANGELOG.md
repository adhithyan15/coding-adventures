# Changelog

## Unreleased

- Reject legacy leading-zero octal tokens instead of lowering them as decimal.
- Bound every directly supplied AST text field and the module name before
  traversing or lowering the tree.

## 0.1.0

- Initial native Perl source-to-IIR-to-vm-core arithmetic and print pilot.
