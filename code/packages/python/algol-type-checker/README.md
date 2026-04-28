# algol-type-checker

Type checker for the first ALGOL 60 compiler subset.

This package consumes the generic AST from `algol-parser` and validates the
structured integer subset described by `code/specs/PL03-algol60-wasm-compiler.md`.
It currently supports integer scalar declarations, assignments, arithmetic,
comparisons, boolean conditions, nested blocks, `if` statements,
`for ... step ... until ... do` loops, integer value/by-name procedures, and
descriptor metadata for integer arrays with integer bounds. Direct labels and
direct local `goto` statements are accepted within one active ALGOL frame, as
are direct nonlocal block `goto` statements that stay inside the same lowered
function. Local switch declarations, switch selections, and conditional
designational `goto` forms are also supported.

The expression checker also accepts chained assignments, ALGOL conditional
expressions, tolerant trailing/repeated semicolons from the parser, and
left-associative exponentiation for numeric bases with integer exponents.
Mixed integer/real conditional branches resolve to `real`; incompatible branch
types are still rejected before IR lowering.

The checker also builds the first ALGOL 60 full-runtime semantic model. Each
source block receives a stable block id, lexical depth, static-parent id, and a
planned frame layout. Scalar declarations are assigned explicit frame slots,
and variable references record the resolved symbol, slot offset, and number of
static links a later WASM lowering pass must walk.
Procedure declarations receive semantic descriptors with generated function
labels, parameter slots, value-vs-name parameter modes, conservative by-name
write metadata, result slots for typed procedures, and resolved call sites
carrying the static-link delta needed by code generation.
Bare no-argument typed procedure names used in expressions are resolved as
procedure calls, matching ALGOL's omitted-parentheses call syntax, while
procedure result variables inside their own bodies still resolve as storage.
Integer array declarations receive descriptor slots in their declaring frame,
dimension metadata for lower/upper bound expressions, and resolved read/write
accesses that preserve the static-link delta and subscript count needed by the
IR and WASM lowering stages.
Labels receive stable label descriptors and direct local `goto` statements
resolve to those descriptors. Direct nonlocal block `goto` statements resolve
to outer active blocks in the same procedure/function; jumps that cross a
procedure boundary remain guarded. Switch declarations receive stable
descriptors whose entries point at checked local designational expressions, and
switch selection use sites resolve to their chosen switch symbol. Nonlocal
switch selections and nonlocal conditional/switch designational branches remain
guarded with targeted diagnostics. Switch entries that recursively select
another switch are also guarded until the later dispatch-lowering phase.

Unsupported ALGOL 60 features are reported as diagnostics instead of being
silently accepted by the compiled pipeline. By-name parameters are accepted in
the semantic model, while later lowering packages now implement scalar
call-by-name, typed whole-array formals, label formals, switch formals, and
no-argument statement procedure formals. The checker keeps guarding the
remaining full-ALGOL gaps, including non-assignable actuals passed to written
by-name formals, value array/label/switch/procedure parameters, and richer
procedure-valued parameters with arguments or return values.

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
