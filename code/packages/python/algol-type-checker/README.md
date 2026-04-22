# algol-type-checker

Type checker for the first ALGOL 60 compiler subset.

This package consumes the generic AST from `algol-parser` and validates the
structured integer subset described by `code/specs/PL03-algol60-wasm-compiler.md`.
It currently supports integer scalar declarations, assignments, arithmetic,
comparisons, boolean conditions, nested blocks, `if` statements,
`for ... step ... until ... do` loops, integer value/by-name procedures, and
descriptor metadata for integer arrays with integer bounds.

The checker also builds the first ALGOL 60 full-runtime semantic model. Each
source block receives a stable block id, lexical depth, static-parent id, and a
planned frame layout. Scalar declarations are assigned explicit frame slots,
and variable references record the resolved symbol, slot offset, and number of
static links a later WASM lowering pass must walk.
Procedure declarations receive semantic descriptors with generated function
labels, parameter slots, value-vs-name parameter modes, conservative by-name
write metadata, result slots for typed procedures, and resolved call sites
carrying the static-link delta needed by code generation.
Integer array declarations receive descriptor slots in their declaring frame,
dimension metadata for lower/upper bound expressions, and resolved read/write
accesses that preserve the static-link delta and subscript count needed by the
IR and WASM lowering stages.

Unsupported ALGOL 60 features, including real/Boolean/string arrays, array
parameters, switches, procedure-valued parameters, labels, and `goto`, are
reported as diagnostics instead of being silently accepted by the compiled
pipeline. By-name parameters are accepted in the semantic model, while later
lowering packages now implement the integer call-by-name subset. The checker
keeps guarding the remaining full-ALGOL gaps, including non-assignable actuals
passed to written by-name formals, non-integer by-name types, whole-array
parameters, procedure-valued parameters, labels, switches, and `goto`.

```python
from algol_parser import parse_algol
from algol_type_checker import check_algol

ast = parse_algol("begin integer result; result := 7 end")
checked = check_algol(ast)
assert checked.ok
assert checked.semantic is not None
assert checked.semantic.root_block is not None
assert checked.semantic.root_block.frame_layout.slots[0].offset == 20
```

## Dependencies

- algol-parser

## Development

```bash
# Run tests
bash BUILD
```
