# lattice-transpiler

End-to-end Lattice source to CSS text pipeline

## Notes

Lattice mixins support both zero-argument definition styles:

```lattice
@mixin panel() {
  padding: 16px;
}

@mixin panel {
  padding: 16px;
}
```

Both can be included with either `@include panel();` or `@include panel;`.

Undefined mixin errors also try to be more helpful now by including nearby
defined mixin names and a reminder about the zero-argument forms.

## Dependencies

- lattice-ast-to-css
- lattice-parser
- lattice-lexer
- grammar-tools
- parser
- lexer

## Development

```bash
# Run tests
bash BUILD
```
