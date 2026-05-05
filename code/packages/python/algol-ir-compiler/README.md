# algol-ir-compiler

Lower the Python ALGOL 60 semantic lane to compiler IR.

The compiler accepts a checked ALGOL AST and emits `compiler_ir.IrProgram`
instructions that the existing structured IR-to-WASM lowerer can consume.
The outermost ALGOL block becomes `_start`. Integer variables live in planned
activation-frame slots, and scalar reads/writes lower to `LOAD_WORD` and
`STORE_WORD` through the semantic model's static-link metadata. If the root
block declares an integer scalar named `result`, it is loaded into `v1` before
`HALT` so the generated WASM function returns it; otherwise `_start` returns
`0`.

Value-only integer procedures lower to generated `_fn_algol_...` functions.
Calls pass an explicit static link followed by value arguments, procedure frames
are allocated from the module frame stack, and typed procedures return through
their procedure-name result slot. Integer and boolean procedure results flow
through `v1`; real procedure results flow through the WASM backend's dedicated
f64 result register, `v31`. Because the type checker registers all procedure
signatures in a block before checking procedure bodies, direct calls can target
procedures declared later in the same block and typed procedures can be
mutually recursive.
Parameterless procedures may be declared and called with explicit empty
parentheses, and typed procedures can still be used by bare name in expression
positions, following ALGOL's
omitted-parentheses call syntax; read-only by-name actuals of that form lower
through eval thunks so each formal read re-runs the procedure.

Integer arrays lower to frame-stored descriptors backed by a separate bounded
ALGOL heap segment. The compiler evaluates integer lower/upper bounds once at
block entry, stores row-major stride metadata beside the descriptor, and emits
checked element loads/stores through the descriptor. Block and procedure exits
restore the heap pointer to the activation's entry mark, so dynamic arrays keep
ALGOL block lifetime instead of leaking through loops or recursive calls.
Because the checker defers bound expression analysis until declaration
registration completes, those lower/upper expressions can call later sibling
typed procedures in the same block. Bound expressions may read earlier array
descriptors, but same-block arrays declared later remain rejected before IR
lowering because their descriptors do not exist at allocation time.

Expression lowering includes mixed integer/real arithmetic, boolean operators,
comparisons, chained assignment targets, branch-selected conditional
expressions, and ALGOL-left-associative exponentiation. Integer exponents use
the existing loop lowering, real bases with negative integer exponents use a
reciprocal path, and real exponents lower through the imported real `pow`
runtime with a domain-failure guard. Boolean `and`, `or`, and `impl` lower
through short-circuiting control flow; `eqv` remains strict and evaluates both
operands.
Standard numeric functions `abs`, `sign`, `entier`, `sqrt`, `sin`, `cos`,
`arctan`, `ln`, and `exp` lower to existing integer/f64 comparison,
arithmetic, conversion, native square-root IR, and imported standard real-math
IR instructions.

Scalar by-name parameters lower through a one-word cell in the callee frame.
Passing a scalar variable as a by-name actual gives the callee a storage pointer,
so assignments to the formal write back to the caller slot while value
parameters still remain isolated copies. Read-only scalar expression actuals
lower to tagged, bounded eval thunk descriptors. Formal reads dispatch through a
generated helper that re-evaluates the expression against the caller frame each
time. Array-element actuals lower to tagged descriptors as well; formal
reads re-compute the element address through the eval helper, and writable
formals call a generated store helper that re-locates the element before storing
the new value. Read-only expression thunks can also read array elements; helper
bounds failures are reported back to the calling procedure before the normal
frame and thunk-region unwind. Procedure calls inside read-only expression
thunks are supported, including nested by-name descriptor allocation and runtime
failure propagation from callees back to the by-name formal read. Assignable
scalar and array-element actuals have storage/store-helper paths; non-assignable
expression actuals remain read-only and are rejected if by-name flow analysis
finds a write. The supported scalar by-name surface is covered by the WASM
acceptance suite, including typed whole-array formals passed as descriptor
pointers. `value` whole-array formals allocate a callee-local descriptor, bounds
table, and element storage copy at procedure entry, so writes to the formal do
not alias the caller's array. Label formals pass pending-goto targets, and
switch formals pass descriptor closures that re-evaluate in the caller's
declaring scope; these label/switch descriptor paths also cover `value`
formals. Procedure formals pass descriptor closures containing the callee
procedure id and static link; formal calls dispatch through generated helpers
for statement calls and typed expression calls with scalar, whole-array, label,
switch, or procedure arguments. The dispatch helpers pass scalar actual
arguments as lazy storage pointers or thunk descriptors, evaluating them once
for target `value` parameters and forwarding them directly for target by-name
parameters. Whole-array, switch, and procedure actuals pass descriptor pointers,
and label actuals pass label ids, so forwarded procedure formals keep the
original environment in value, by-name, array, label, switch, or procedure mode.
Real-valued formal procedure calls can accept integer-returning actual
procedures and promote the dispatched result. Concrete procedure actuals
forwarded through a formal procedure call retain enough type-checker metadata to
validate nested procedure-parameter contracts.

Direct `goto`/`go to` statements lower to ordinary IR `JUMP` instructions targeting
generated ALGOL labels. Local jumps emit the jump directly. Direct nonlocal
block jumps unwind each exited block with the same heap-pointer, current-frame,
and stack-pointer restoration used by normal block exits before transferring
control to the outer label. The downstream WASM backend's unstructured
control-flow lowering handles forward, backward, and nonlocal block jumps.
Local conditional designational expressions now lower as condition-controlled
branch points that only evaluate the selected target. Local switch selections
evaluate their integer index once, compare against one-based switch entries,
and lower the chosen designational entry into the same jump path. Switch entries
may target labels in lexical parent blocks or later sibling switch declarations;
those entries unwind exited frames or propagate pending procedure-crossing gotos
just like direct designational gotos. An out-of-range switch index follows the
existing runtime-failure path and returns `0`. Recursive switch self-selection
lowers through the switch evaluation helper so finite recursive dispatch
executes at runtime instead of expanding the descriptor at compile time.

This phase keeps ALGOL frame memory and its 32-byte runtime state bounded to
one 64 KiB WASM page, and keeps array descriptors plus element storage inside a
separate 64 KiB heap segment. Larger semantic frame plans raise `CompileError`
before the WASM data encoder can materialize the memory image, dynamic
procedure recursion stops at the bounded frame stack, and invalid array bounds,
out-of-bounds subscripts, integer `div`/`mod` by zero or signed divide
overflow, real division by zero, zero-real-base negative exponentiation,
real-to-integer conversions outside the WASM `i32.trunc_f64_s` domain,
non-finite real output, oversized arrays, negative `sqrt` arguments, or heap
exhaustion return `0`.
The lowering pass also has configurable generated-state limits for eval thunks,
conditional label sets, loop label sets, switch dispatch states, and output
helper label sets. Callers can pass `IrCompilerLimits` to `compile_algol` or
`AlgolIrCompiler` to tighten those defaults for sandboxed compilation.

```python
from algol_ir_compiler import compile_algol
from algol_parser import parse_algol

ir = compile_algol(parse_algol("begin integer result; result := 1 + 2 end"))
assert ir.program.entry_label == "_start"
assert ir.variable_slots["result@block0"] == 20
```

## Dependencies

- algol-type-checker
- compiler-ir

## Development

```bash
# Run tests
bash BUILD
```
