# algol-type-checker

Type checker for the first ALGOL 60 compiler subset.

This package consumes the generic AST from `algol-parser` and validates the
structured integer subset described by `code/specs/PL03-algol60-wasm-compiler.md`.
It currently supports integer scalar declarations, assignments, arithmetic,
comparisons, boolean conditions, nested blocks, `if` statements, and
`for ... step ... until ... do` loops.

Unsupported ALGOL 60 features, including arrays, switches, procedures,
call-by-name, labels, and `goto`, are reported as diagnostics instead of being
silently accepted by the compiled pipeline.

```python
from algol_parser import parse_algol
from algol_type_checker import check_algol

ast = parse_algol("begin integer result; result := 7 end")
checked = check_algol(ast)
assert checked.ok
```

## Dependencies

- algol-parser

## Development

```bash
# Run tests
bash BUILD
```
