# PL04 - Full ALGOL 60 WASM Runtime and Compiler Roadmap

## Overview

`PL03-algol60-wasm-compiler.md` proves the first useful ALGOL 60 compiler
lane:

```text
ALGOL 60 source
  -> algol-lexer
  -> algol-parser
  -> algol-type-checker
  -> algol-ir-compiler
  -> algol-wasm-compiler
  -> .wasm bytes
```

PL03 intentionally supports a small structured integer subset. This spec
describes the full roadmap for making that lane compile practical ALGOL 60
programs to WebAssembly while preserving ALGOL's historically important
semantics.

The central design goal is a reusable compiler spine:

```text
source AST
  -> typed semantic model
  -> lowered control/runtime model
  -> compiler IR
  -> WASM module plus ALGOL runtime helpers
```

If this is done well, later Pascal-like, BASIC-like, C-like, and block-scoped
languages can reuse the same runtime and lowering ideas instead of rebuilding
their compiler backends from scratch.

## Why Full ALGOL 60 Is Not Just More Syntax

The grammar already describes much of ALGOL 60:

- scalar declarations
- arrays with runtime bounds
- nested blocks
- procedures
- value parameters and by-name parameters
- labels
- `goto`
- switches
- designational expressions
- arithmetic, boolean, and conditional expressions

The hard work is semantic. Full ALGOL 60 needs:

- nested lexical environments
- activation records
- static links or displays
- procedures as callable runtime objects
- call-by-name thunks
- assignable by-name locations
- dynamic arrays
- labels as block-relative continuations
- nonlocal `goto`
- switch vectors
- `own` storage
- richer scalar types
- a small I/O and runtime support surface

WASM can support all of this, but not as direct one-instruction mappings. The
compiler needs a clear runtime model and a lowering pass before code generation.

## Scope

This spec covers the full intended implementation shape for ALGOL 60 on top of
the repository's existing compiler and WASM packages.

It does not require implementing every phase in one PR. Each phase should be
small enough to review, test, and merge independently.

The first language bucket remains Python because PL03 established Python as the
reference lane. Later phases may port stabilized pieces to other language
buckets.

## Design Principles

### Preserve ALGOL Semantics First

The compiler should reject unsupported ALGOL constructs explicitly until the
runtime model can implement them correctly. It must not silently compile an
ALGOL program with Pascal-like or C-like behavior when ALGOL requires different
behavior.

Examples:

- by-name parameters are not by-reference parameters
- array bounds are evaluated at block entry, not at parse time
- nested procedures need lexical access to outer variables
- a nonlocal `goto` exits intervening blocks
- switches are designational-expression tables, not integer arrays

### Lower Through Explicit Runtime Concepts

The typed AST should not lower directly to WASM once full ALGOL features appear.
It should first lower to an explicit semantic model:

- lexical scopes
- frame layouts
- symbols
- storage classes
- parameter modes
- procedure descriptors
- label descriptors
- array descriptors
- thunk descriptors

This model becomes the contract between the frontend and the backend.

### Keep WASM Backend Reusable

Where possible, add general compiler/runtime conventions above WASM instead of
teaching every frontend a one-off WASM trick. ALGOL should improve the
repository's shared language compilation story.

### Make Each Phase Executable

Every phase should include at least one end-to-end program that parses,
type-checks, lowers, compiles to WASM, and executes in the local WASM runtime.

## Package Shape

PL03 introduced:

- `code/packages/python/algol-type-checker`
- `code/packages/python/algol-ir-compiler`
- `code/packages/python/algol-wasm-compiler`

Full ALGOL should grow around those packages before adding new packages.

Expected additions inside or near the existing packages:

- semantic model classes in `algol-type-checker`
- frame and storage layout helpers in `algol-ir-compiler`
- runtime helper selection in `algol-wasm-compiler`
- fixtures for full ALGOL programs

Only create a new package when the concept is reusable across languages or
large enough to deserve its own boundary. Possible future packages:

- `algol-semantic-model`
- `algol-runtime-abi`
- `structured-control-lowering`

Do not create those packages until a concrete phase needs them.

## Semantic Model

The semantic model is the typed, resolved representation of an ALGOL program.
It should be independent of WASM opcodes.

### Program

A semantic program contains:

- the parsed AST root
- a root block
- a symbol table
- a list of procedures
- a list of labels
- diagnostics
- feature flags used by later lowering phases

### Block

Each block contains:

- a unique block id
- a lexical parent block id
- declarations
- statements
- labels declared in the block
- frame layout requirements
- array allocation requirements
- `own` storage requirements

Blocks are lexical scopes. Every nested block may access declarations from its
ancestors.

### Symbols

Each declared name resolves to exactly one semantic symbol in the relevant
scope.

Symbol fields:

- source name
- unique symbol id
- kind: scalar, array, procedure, parameter, label, switch, own
- source type
- storage class
- declaring block
- mutability and assignability
- source span

### Types

The full source type set is:

- `integer`
- `real`
- `boolean`
- `string`
- `label`
- `switch`
- procedure types
- array types

Internal runtime types should include:

- `i32`
- `f64`
- `bool`
- `string_ref`
- `frame_ref`
- `array_ref`
- `procedure_ref`
- `label_ref`
- `thunk_ref`

The first full-runtime phases may continue lowering `integer` to `i32` and
`real` to `f64`. If the existing compiler IR cannot express `f64`, the real
number phase must first extend or wrap the IR rather than pretending reals are
integers.

### Diagnostics

Every unsupported construct must produce a targeted diagnostic. Examples:

- `call-by-name parameter 'x' requires thunk lowering, not implemented`
- `nonlocal goto to 'done' requires frame unwinding, not implemented`
- `real arithmetic requires f64 backend support, not implemented`

Generic "unsupported node" errors are acceptable only during internal
development, not at a phase's completion point.

## Runtime ABI

Full ALGOL needs a small runtime ABI on top of WASM linear memory.

The ABI should be documented as data layouts plus helper calls. The compiler
may inline simple operations once the ABI is stable, but tests should validate
the conceptual layout.

### WASM Values

Recommended low-level representations:

| Semantic value | WASM representation |
|----------------|---------------------|
| integer | `i32` |
| boolean | `i32`, where 0 is false and 1 is true |
| real | `f64` when backend support exists |
| string | pointer to runtime string descriptor |
| frame reference | `i32` linear-memory pointer |
| array reference | `i32` linear-memory pointer |
| procedure reference | `i32` descriptor pointer or function table index |
| label reference | `i32` label token |
| thunk reference | `i32` descriptor pointer |

### Linear Memory

The module should own a single linear memory.

Reserved globals:

- `__algol_sp`: top of runtime stack
- `__algol_fp`: current frame pointer
- `__algol_heap`: bump pointer for heap or long-lived descriptors
- `__algol_status`: runtime error code

Initial phases may implement a simple bump allocator. Later phases can add
free lists or region allocators if arrays and thunks need it.

### Frame Layout

Every activation record should contain a fixed header followed by compiler-laid
out slots.

```text
frame + 0   previous dynamic frame pointer
frame + 4   static link or display parent pointer
frame + 8   return continuation token
frame + 12  frame size in bytes
frame + 16  block id or procedure id
frame + 20  first local slot
```

Slot classes:

- scalar value slots
- pointer slots for arrays, strings, thunks, and procedure descriptors
- saved temporaries required across calls
- optional display entries

The exact byte offsets may change during implementation, but the fields must
remain explicit and tested.

### Static Links Versus Displays

The first implementation should use static links because they are simple and
teach the lexical-chain model directly.

For a reference from block depth `d` to an ancestor at depth `a`, generated code
walks `d - a` static links and then loads the slot offset from that frame.

A later optimization phase may introduce displays:

- display entry `N` points to the active frame at lexical depth `N`
- procedure calls save and restore affected display entries
- nonlocal control flow must restore display state

Displays are an optimization, not a semantic requirement.

## Storage Model

### Scalar Locals

Scalar locals live in frame slots. Their lifetime is the lifetime of the block
activation.

Nested blocks that do not outlive their parent can allocate locals in a nested
frame or in an extended region of the same activation. The initial full-runtime
model should use explicit frames for clarity.

### Parameters

Parameters can be:

- value parameters
- by-name parameters

Value parameters receive evaluated values in frame slots.

By-name parameters receive thunk descriptors. A by-name parameter use evaluates
the thunk. A by-name parameter assignment stores through the thunk if the
argument is assignable.

### `own` Declarations

`own` declarations have static lifetime but lexical visibility. They should be
stored outside ordinary activation frames.

Recommended representation:

- allocate one static slot per `own` symbol in linear memory
- initialize on first program entry or module instantiation
- reference through the symbol's static storage address

`own` array support may wait until dynamic arrays are stable.

### Temporaries

Compiler temporaries should remain virtual until lowering chooses where they
live.

Temporaries that do not cross calls can stay in virtual registers or WASM
locals. Temporaries that cross calls, thunk invocations, or nonlocal jumps must
spill to frame slots.

## Procedures

ALGOL procedures may be nested and may access outer lexical variables.

### Procedure Descriptors

Each procedure needs a descriptor:

```text
procedure_descriptor:
  code id or function index
  lexical parent block id
  static environment frame pointer
  arity
  parameter descriptor pointer
```

If the procedure is never used as a value, the compiler may call its generated
function directly while still passing the correct static link.

### Captured Environment Lifetimes

Procedure descriptors and by-name thunks must never retain raw frame pointers
that can outlive the activation that owns those frames.

Before procedure values, by-name thunks, or nested procedure descriptors can be
stored beyond the immediate call, the compiler must choose one of these
strategies:

- reject escaping descriptors with a clear diagnostic
- prove through escape analysis that the descriptor cannot outlive its frame
- heap-lift the captured environment into a closure record with explicit
  lifetime management

The first implementation should prefer rejection unless a phase explicitly
implements heap-lifted closure environments. Direct calls to lexically nested
procedures may still pass ordinary static links because the callee does not
outlive the caller's activation.

### Calling Convention

Every generated procedure should receive:

- static link
- dynamic caller frame
- arguments

Conceptual call:

```text
call procedure_code(static_link, caller_frame, arg0, arg1, ...)
```

The callee:

1. allocates a frame
2. stores dynamic link
3. stores static link
4. stores parameters
5. initializes local declarations
6. executes body
7. returns result or void
8. restores `__algol_fp` and `__algol_sp`

### Functions

In ALGOL 60, typed procedures are functions. The function result is assigned by
assigning to the procedure's own name inside the procedure body.

The semantic model should represent that as a hidden result slot:

- symbol kind: procedure result
- visible inside the procedure body under the procedure's name
- returned when the procedure exits normally

Using the procedure name in a call context means "call the procedure." Using it
inside its own body as an assignment target means "write the result slot."

### Recursion

Recursive calls allocate fresh frames. Static links must still point to the
lexically enclosing environment, not to the dynamic caller unless those happen
to be the same.

Tests must include:

- direct recursion
- nested procedure referencing an outer variable
- recursive procedure with a local variable shadowing an outer variable

## Call-By-Name

Call-by-name is the most ALGOL-specific part of this roadmap.

### Semantics

A by-name argument is not evaluated at call entry. Instead, the callee receives
a delayed computation that can be evaluated each time the formal parameter is
read.

If the actual argument is assignable, assigning to the formal parameter writes
back to the actual location.

ALGOL's default parameter mode is by-name. A formal parameter appears in a
`value` part only when it should be evaluated once at call entry. Therefore the
semantic model must record each formal parameter's mode explicitly:

- `value`: current Phase 3 behavior
- `name`: delayed by-name behavior

The first implementation should support `integer` by-name parameters only. It
should continue to reject real, Boolean, string, array, procedure-valued, label,
and switch formals until their value representation and runtime helpers exist.
Array elements such as `A[i]` are valid integer by-name actuals because the
element designator computes an integer location on every read or write.

By-name parameters are not by-reference parameters. The actual expression is
re-evaluated for every formal read, and an assignable actual is re-located for
every formal write. If a variable used by the actual expression changes inside
the callee, later reads/writes see the new computation.

Examples:

```algol
procedure twice(x);
  integer x;
begin
  x := x + 1;
  x := x + 1
end
```

Calling `twice(a[i])` must update the current element selected by the expression
each time the thunk is used. If `i` changes between uses, the referenced element
can change.

Calling `twice(1 + y)` is syntactically valid when `twice` only reads `x`, but
it must fail with a targeted diagnostic if the procedure body assigns to `x`.

### Semantic Model

Each procedure descriptor should carry `ProcedureParameter.mode`:

```text
procedure_parameter:
  name
  type id
  mode: value | name
  symbol id
  frame slot offset
```

The type checker must also compute whether a by-name formal may be assigned
during the procedure call. This is an interprocedural property, not just a scan
of the immediate procedure body:

```text
by_name_parameter_use:
  formal symbol id
  read count
  written: true | false
  write reason: local assignment | transitive call | unknown callee
```

The analysis may be conservative. A by-name formal is considered written when:

- the procedure body assigns directly to the formal
- the formal is passed as an actual to another by-name formal that may be
  written
- the formal is passed to a procedure whose body or descriptor is not available
  for analysis
- a recursive cycle contains any path that writes the corresponding formal

This prevents a caller from passing a read-only expression such as `1 + y` to
`outer(x)` when `outer` itself only reads `x` but passes `x` to `inner(z)`, and
`inner` assigns to `z`.

At each call site, the checker validates every by-name actual against the
callee's formal use:

- read-only formals accept any integer expression
- written formals require an assignable integer designator
- scalar variables and array elements are assignable designators
- literals, arithmetic expressions, comparisons, function calls, and conditional
  expressions are not assignable in the first implementation

When validation fails, use diagnostics like:

- `by-name parameter 'x' is assigned, but actual expression is not assignable`
- `by-name parameter 'x' expects integer, got boolean`
- `by-name parameter 'x' requires thunk lowering, not implemented for procedure values`

### Thunk Descriptor

A by-name thunk descriptor should contain:

```text
thunk_descriptor:
  eval code id or function index
  store code id or function index, or 0 if not assignable
  static link for the caller expression
  caller frame pointer
  type id
  flags
```

The compiler generates one eval helper per argument expression shape as needed.
For assignable arguments, it also generates a store helper.

Recommended flags:

| Flag | Meaning |
|------|---------|
| `1` | actual is assignable |
| `2` | actual reads caller frame slots |
| `4` | actual reads array descriptors |

The first implementation may store thunk descriptors in the caller's active
frame or in a bounded call-scoped heap region. In either case, a descriptor must
not escape the dynamic extent of the call that receives it. Until procedure
values and heap-lifted environments exist, reject any construct that would store
a by-name thunk in a longer-lived location.

The descriptor's environment pointers are valid only while the caller
activation is active. This is acceptable for direct procedure calls because
the callee returns before the caller frame is torn down. It is not acceptable
for procedure values, stored thunk descriptors, or returning thunks from a
procedure.

### Calling Convention

By-name parameters are passed as descriptor pointers. A generated procedure with
mixed value and by-name parameters should still receive:

```text
call procedure_code(static_link, caller_frame, arg0, arg1, ...)
```

but each `argN` is interpreted according to the formal mode:

- `value integer`: immediate `i32` value
- `name integer`: `i32` pointer to a `thunk_descriptor`

The callee stores the descriptor pointer in the formal parameter's frame slot.
Reading a by-name formal invokes the descriptor's eval helper. Assigning to a
by-name formal invokes the descriptor's store helper with the new value.

### Eval Helper

An eval helper:

1. restores or receives the caller lexical environment
2. evaluates the original expression
3. returns the value

Conceptual helper signature:

```text
eval(thunk_descriptor_ptr) -> i32
```

The helper loads the captured static link and caller frame pointer from the
descriptor, then evaluates the actual expression using the same reference and
array-access lowering rules as ordinary caller code.

For `sum(i, 1, 3, A[i])`, the `term` eval helper must load the current `i`
through the caller frame and then compute `A[i]` through the current array
descriptor bounds on every call.

### Store Helper

A store helper:

1. restores or receives the caller lexical environment
2. evaluates the original designator to a location
3. stores the incoming value

Non-assignable actuals, such as `x + 1`, should be valid for read-only by-name
parameters but must fail if the callee assigns to the formal.

Conceptual helper signature:

```text
store(thunk_descriptor_ptr, value: i32) -> void
```

If the descriptor has no store helper and the callee assigns to the formal,
code generation should not proceed. The type checker should normally catch this
before IR lowering. The IR compiler may keep a defensive check and raise
`CompileError` if a written by-name formal has no store helper.

Store helpers for scalar variables write the resolved frame slot. Store helpers
for array elements re-evaluate the current subscripts, run the normal runtime
bounds checks, compute the current element address, and store to that address.

### Lowering Rules

When lowering a procedure body:

- reads of value parameters keep the current frame-slot load path
- writes to value parameters keep the current frame-slot store path
- reads of by-name parameters lower to eval-helper calls
- writes to by-name parameters lower to store-helper calls
- by-name formals can be used in integer expressions wherever scalar integers
  are currently allowed

When lowering a call site:

1. value actuals lower exactly as they do today
2. by-name actuals emit a thunk descriptor before the call
3. each thunk descriptor captures the caller static link and caller frame pointer
4. read-only actuals emit only an eval helper
5. assignable actuals emit both eval and store helpers
6. the descriptor pointer is passed in the normal argument position

The descriptor allocation must be bounded. The compiler should enforce:

- maximum by-name parameter count per procedure
- maximum generated thunk helpers per program
- maximum call-site thunk descriptor bytes
- no thunk descriptor escapes the receiving call

### Phase 5 Integer Subset

The completed integer-focused Phase 5 implementation supports:

- `integer` by-name scalar formals
- by-name integer expression actuals for read-only formals
- assignable scalar variable actuals for written formals
- assignable integer array element actuals for written formals
- by-name actual expressions that reference variables in the caller's lexical
  ancestors
- by-name actual expressions that reference Phase 4 integer arrays
- by-name actual expressions that call integer procedures
- Jensen's device over integer scalar and array expressions

The integer-focused implementation still intentionally rejects:

- non-integer by-name formals
- by-name array descriptors as whole-array values
- procedure-valued by-name actuals
- label, switch, real, Boolean, and string by-name formals
- escaping thunk descriptors
- by-name actuals that require nonlocal `goto`, switches, or procedure values

Current Phase 5a lowering may use a storage-pointer fast path for scalar
variable actuals before general descriptor helpers exist. In that slice, the
callee frame stores the caller scalar slot address in the by-name formal slot,
and formal reads/writes dereference that address. This is executable for scalar
variable actuals and useful for validating the WASM calling path, but it must
continue to reject read-only expression actuals and array-element actuals until
the full eval/store descriptor helpers can re-evaluate or re-locate the actual
on every formal access.

Current Phase 5b lowering may add read-only integer expression eval descriptors
without store helpers. Descriptor pointers should be distinguishable from the
Phase 5a scalar storage pointers, for example by tagging aligned descriptor
addresses before passing them through the by-name formal slot. A by-name formal
read checks the tag: untagged values are scalar storage pointers, while tagged
values dispatch to a bounded eval helper that receives the descriptor, restores
the caller frame for the captured expression, and returns the freshly evaluated
integer. Assigning to a tagged descriptor must fail until store helpers exist.
This slice should still reject expression thunks that read array elements or
invoke procedure calls unless their helper can perform the correct runtime
unwind and lifetime handling.

Current Phase 5c lowering may add integer array-element descriptors for by-name
actuals. Array-element descriptors reuse the tagged descriptor path, carry a
store-capable flag only when the formal may be assigned, and dispatch reads
through the eval helper. Writable formals dispatch assignments through a store
helper that re-evaluates the original designator, including current subscript
values, before storing. Helper-side bounds failures must be reported back to the
calling procedure so the normal frame and thunk-region unwinding still runs.

Current Phase 5d lowering may allow read-only expression descriptors to read
integer arrays. The eval helper should mark its synthetic caller-frame scope as
a helper context, so checked array loads inside expression thunks report bounds
failures through the thunk failure flag and let the by-name formal read perform
the caller's normal unwind.

Current Phase 5e lowering may allow procedure calls inside read-only expression
descriptors. Eval and store helpers should maintain a runtime helper-depth
counter while thunk code executes. Runtime failures in procedures called from a
helper set the thunk failure flag, and procedure-call lowering in helper scopes
returns immediately to the helper caller when that flag is set. This keeps
nested procedure frames and call-scoped thunk descriptors bounded while letting
normal, non-helper procedure failures keep their existing return-`0` behavior.

### Required Diagnostics

The phase is not complete unless unsupported forms fail before WASM lowering
with diagnostics targeted to the offending source construct.

Required cases:

- value part omitted means by-name, not value
- assigned by-name formal with literal actual
- assigned by-name formal with arithmetic expression actual
- assigned by-name formal with procedure-call actual
- wrong actual type for by-name formal
- unsupported by-name formal type
- thunk helper count or descriptor byte cap exceeded

### Acceptance Programs

Repeated reads must re-evaluate the actual expression:

```algol
begin
  integer result, i, y;

  integer procedure pick(x);
    integer x;
  begin
    i := 2;
    pick := x
  end;

  i := 1;
  y := 10;
  result := pick(i + y)
end
```

Expected result: `12`.

Assigning to a by-name scalar writes through to the actual variable:

```algol
begin
  integer result, a;

  procedure twice(x);
    integer x;
  begin
    x := x + 1;
    x := x + 1
  end;

  a := 3;
  twice(a);
  result := a
end
```

Expected result: `5`.

Assigning to a by-name array element re-evaluates the designator:

```algol
begin
  integer result, i;
  integer array A[1:2];

  procedure put(x);
    integer x;
  begin
    x := 7;
    i := 2;
    x := 9
  end;

  i := 1;
  put(A[i]);
  result := A[1] * 10 + A[2]
end
```

Expected result: `79`.

### Jensen's Device Acceptance Test

The call-by-name phase is not complete until it can express a Jensen's-device
style program:

```algol
begin
  integer i, result;
  integer array A[1:3];

  integer procedure sum(k, lo, hi, term);
    value lo, hi;
    integer k, lo, hi, term;
  begin
    integer s;
    s := 0;
    for k := lo step 1 until hi do
      s := s + term;
    sum := s
  end;

  A[1] := 2;
  A[2] := 3;
  A[3] := 5;
  result := sum(i, 1, 3, A[i])
end
```

Expected result: `10`.

## Arrays

ALGOL arrays have bounds evaluated at runtime when the declaration is entered.

### Array Descriptor

Every array value should be represented by a descriptor:

```text
array_descriptor:
  element type id
  dimension count
  total element count
  element byte width
  data pointer
  bounds pointer
```

Bounds table:

```text
dimension 0 lower
dimension 0 upper
dimension 0 stride
dimension 1 lower
dimension 1 upper
dimension 1 stride
...
```

### Allocation

At block entry:

1. evaluate every lower and upper bound once
2. validate `upper >= lower`
3. compute dimension lengths
4. check multiplication for overflow
5. allocate descriptor and data
6. initialize elements to the language-defined default for the element type

The compiler must cap maximum dimensions and aggregate bytes so hostile source
cannot force unbounded allocation.

### Subscript Lowering

For `A[i, j]`, generated code:

1. loads descriptor
2. checks dimension count
3. evaluates each subscript
4. checks each subscript against bounds
5. computes linear offset using strides
6. loads or stores the element

Bounds failures should set `__algol_status` and trap or return a runtime error
according to the WASM runtime convention chosen for the package.

### Array Parameters

Array parameters should pass descriptors. By-value array copying is not part of
the first array phase unless a specific ALGOL form requires it.

By-name array element arguments use thunks that compute element locations on
each access.

## Labels, Goto, and Switches

Structured WASM blocks are not enough for full ALGOL control flow. Labels and
`goto` require a second lowering strategy.

### Label Descriptors

Each label should resolve to:

- label id
- declaring block id
- target statement id
- target frame depth
- whether the jump is local or nonlocal from each use site

### Local Goto

Within one lowered function and one active frame, a local `goto` lowers to:

- an IR jump if the backend supports arbitrary labels
- or a dispatch loop state update if the backend only supports structured WASM

The completed Phase 6 implementation uses the repository's unstructured IR
control-flow path. It accepts direct labels and direct `goto` targets within the
same active ALGOL frame, including forward jumps, backward jumps, and terminal
labels on empty statements. Phase 7a extends that local path to conditional
designational expressions and switch selections.

### Nonlocal Goto

A nonlocal `goto` exits one or more active frames.

Lowering must:

1. identify the target frame
2. run any required frame cleanup
3. restore stack and frame pointers
4. restore display entries if displays are used
5. transfer control to the target label

The completed Phase 7b-1 implementation accepts direct nonlocal block gotos
that stay inside one lowered function. It unwinds each exited block by
restoring the block's heap mark, dynamic current frame, and stack pointer, then
jumps to the target label. Procedure-crossing gotos, nonlocal conditional
designational branches, and nonlocal switch selections remain guarded until
the runtime has procedure escape propagation.

### Dispatch Loop Lowering

The most portable strategy for arbitrary ALGOL control flow is a state-machine
lowering:

```text
state := entry_label
loop:
  switch state:
    case label_0: ...; state := label_7; continue
    case label_1: ...; return
```

In WASM 1.0, this can be represented with a loop plus nested structured
branches, a table dispatch if supported by the local backend, or a helper
lowering pass that maps states to reducible control regions.

This should be introduced only after structured procedures and frames are
stable.

### Switch Declarations

An ALGOL switch is a named table of designational expressions.

For:

```algol
switch s := L1, L2, if b then L3 else L4;
goto s[i]
```

The runtime model should evaluate the selected designational expression and
then jump to the resulting label. It is not merely an array of integers.

Switch descriptor:

```text
switch_descriptor:
  count
  entry descriptor pointer
```

Each entry may be:

- direct label
- conditional designational expression
- nested switch selection

The first switch phase may support only direct-label entries.

The completed Phase 7a implementation keeps switch descriptors in semantic
metadata rather than runtime memory. Local switch declarations may contain
direct labels and conditional designational expressions over local labels. A
`goto s[i]` evaluates `i` once, uses ALGOL's one-based switch entry numbering,
and jumps through the selected designational entry. Indexes outside the
declared switch range follow the existing runtime-failure path and return `0`.
Switch selections may cross ALGOL frame boundaries by evaluating the chosen
entry in the switch's declaring scope while still using the active execution
scope for frame unwinding and pending-goto propagation. Switch entries may
select another switch when that expansion does not recurse back to the same
switch.

## Expressions

### Conditional Expressions

ALGOL supports conditional arithmetic, boolean, and designational expressions.

Lowering should:

- type-check both branches against the expected result type
- lower to value-producing control flow
- avoid evaluating the unchosen branch

For by-name thunk expressions, conditional expressions must be evaluated when
the thunk is invoked, not when it is created.

### Boolean Operators

The semantic model must decide whether boolean operators are strict or
short-circuiting according to the implemented ALGOL dialect. The grammar
already encodes precedence.

The spec recommendation is:

- `and`, `or`, `impl` should short-circuit in the compiler runtime model
- `eqv` evaluates both sides

If the historical report or a chosen dialect requires different behavior, note
that explicitly in the phase PR before implementation.

### Real Arithmetic

Real arithmetic requires backend support for:

- `f64` constants
- `f64` add/subtract/multiply/divide
- integer-to-real conversion
- real-to-integer conversion if supported by the language form
- real comparisons

If `compiler_ir` remains integer-only, add a typed IR extension or a runtime
helper layer before claiming real support.

### Strings

Initial string support may be limited to literals and output. General string
variables require a descriptor and allocation policy.

String descriptor:

```text
string_descriptor:
  byte length
  data pointer
```

The compiler should preserve source bytes exactly according to the lexer
contract.

## Runtime Errors

Runtime checks must be explicit and testable.

Required runtime errors:

- array lower bound greater than upper bound
- array allocation exceeds configured limit
- subscript out of bounds
- integer division by zero
- modulo by zero
- call through invalid procedure descriptor
- by-name store through non-assignable argument
- switch index out of bounds
- invalid label token
- unsupported runtime helper

The first implementation can signal errors by trapping, returning a status
code, or setting `__algol_status`. The chosen behavior must be stable in tests.

## I/O Surface

ALGOL 60 did not standardize modern I/O the way later languages did. This repo
should define a tiny implementation-supported I/O surface rather than pretending
there is a universal historical one.

Recommended initial helpers:

- `algol_print_int(i32)`
- `algol_print_real(f64)`
- `algol_print_bool(i32)`
- `algol_print_string(ptr)`
- `algol_newline()`

These can map to WASI `fd_write` when available or to the repository's WASM
runtime host interface in tests.

### I/O Host Boundary Validation

I/O helpers are a host boundary. Even when the compiler generated the WASM, the
runtime must treat guest pointers, descriptors, and lengths as untrusted at the
moment a host helper reads them.

Required checks:

- `algol_print_string(ptr)` validates the string descriptor address before
  reading it
- string byte lengths are capped before host allocation or copying
- string data pointers and ranges must fit inside the module's linear memory
- total output bytes per call and per program execution are capped
- imported host helpers are restricted to the documented ALGOL I/O surface
- WASI mappings may only write through approved descriptors such as stdout or a
  test capture sink

The I/O phase is not complete until tests cover malformed string descriptors,
oversized output requests, and attempts to resolve unsupported host imports.

I/O is not required for compiler correctness phases until runtime programs need
observable behavior beyond `_start` returning `result`.

## IR and Backend Requirements

Full ALGOL will put pressure on the current IR.

### Required IR Capabilities

The compiler needs either direct IR support or a lowering layer for:

- load/store from linear memory
- function calls
- typed values beyond `i32`
- labels and branches
- indirect calls or dispatch
- runtime helper imports
- table or state-machine dispatch

If the existing `compiler_ir` lacks any of these, the ALGOL compiler should add
a lowering pass that encodes the feature using available operations until the
shared IR grows a first-class instruction.

### Avoid Premature IR Expansion

Do not add large generic IR features solely because ALGOL will need them
eventually. Each IR extension should be motivated by a phase that uses it and
tests it.

## Phase Plan

### Phase 1: Semantic Model and Frame Planning

Goal: Make lexical scopes and storage explicit without changing observable
language support much.

Deliverables:

- semantic program/block/symbol model
- block depth and static-parent tracking
- frame layout planner for scalar locals
- resolved variable references carrying lexical depth and slot information
- tests for shadowing and outer-scope lookup

Acceptance:

- existing PL03 programs still compile
- nested blocks resolve through planned frames
- diagnostics name the resolved or missing symbol clearly

### Phase 2: Frame-Based Scalar Codegen

Goal: Move scalar locals from plain virtual registers into frame slots.

Deliverables:

- WASM linear-memory frame setup for `_start`
- scalar load/store helpers or generated memory ops
- frame pointer and stack pointer initialization
- frame teardown
- compiler-side cap for the phase's static frame image before WASM data
  allocation

Acceptance:

- existing integer programs execute through frame-backed storage
- nested block variables can shadow outer variables
- an inner block can read and write an outer variable
- oversized frame plans fail with a compiler diagnostic instead of allocating
  an oversized WASM data segment

### Phase 3: Value Procedures

Goal: Implement nested procedures with call-by-value parameters.

Scope:

- support untyped procedure statements and typed `integer procedure`
  expression calls
- require every formal parameter in this phase to appear in a `value` part
- require every formal parameter in this phase to have an `integer` specifier
- continue to reject call-by-name, arrays, procedure-valued parameters, labels,
  switches, and non-integer procedure results with explicit diagnostics
- lower calls directly to generated `_fn_...` functions; procedure values and
  escaping descriptors remain future work

Deliverables:

- procedure symbols and descriptors
- static-link computation at call sites
- procedure frame allocation
- parameter slots
- typed procedure result slot
- direct recursion
- explicit IR call argument registers so ALGOL argument passing does not
  overwrite frame-pointer temporaries
- bounded dynamic frame stack within the module's linear memory

Acceptance:

- simple procedure call mutates an outer variable when allowed
- call-by-value parameter assignment does not write back to the actual argument
- typed procedure returns a value through its procedure-name result slot
- recursive factorial or summation program runs
- runaway recursive calls stop at the bounded frame stack instead of allocating
  unbounded host memory

### Phase 4: Dynamic Arrays

Goal: Implement runtime-bounded arrays and array element access.

Initial scope:

- support `integer array` declarations only
- support one or more dimensions with integer lower/upper bound expressions
- evaluate bounds exactly once when the declaring block is entered
- store an array descriptor pointer in the declaring frame
- support direct reads and writes through `A[i]` and `A[i, j]`
- support dynamic bounds that reference visible scalar variables
- continue to reject real/boolean/string arrays, array parameters,
  procedure-valued arrays, by-name array element thunks, and array values used
  without subscripts

Runtime representation:

- array descriptors and element storage live in a bounded ALGOL heap region in
  WASM linear memory
- descriptor slots in activation frames hold `i32` descriptor pointers
- each descriptor records element type, dimension count, element count, element
  byte width, data pointer, and bounds pointer
- the bounds table stores lower, upper, and stride for each dimension
- the first Phase 4 implementation returns `0` through the existing runtime
  failure convention for invalid bounds, out-of-bounds subscripts, heap
  exhaustion, or configured array-cap violations

Required caps:

- maximum dimension count
- maximum element count per array
- maximum allocation bytes per array
- maximum aggregate ALGOL heap bytes
- stack/heap regions must remain separate and every allocation must check the
  configured heap limit before advancing the heap pointer

Deliverables:

- array type checking
- descriptor layout
- bounds evaluation at block entry
- subscript load/store lowering
- allocation caps and runtime errors

Acceptance:

- one-dimensional integer arrays work
- multidimensional integer arrays work
- dynamic bounds from variables work
- out-of-bounds access is rejected at runtime

### Phase 5: Call-By-Name

Goal: Implement ALGOL's default parameter mode.

Deliverables:

- explicit value-vs-name parameter mode in the semantic model
- conservative interprocedural analysis of which by-name formals may be assigned
- thunk descriptor layout
- eval and store helper generation
- by-name read and write lowering
- bounded descriptor/helper generation
- non-escaping thunk lifetime checks
- diagnostics for non-assignable by-name stores

Acceptance:

- repeated formal reads re-evaluate the actual expression
- assigning to a by-name formal writes through assignable actuals
- scalar variable actuals work for assigned by-name formals
- array element actuals re-compute their element address on each use
- read-only expression actuals work without store helpers
- assigned read-only expression actuals are rejected before IR lowering
- read-only expression actuals can read arrays and call integer procedures
- Jensen's device works

### Phase 6: Labels and Local Goto

Goal: Support labels and gotos inside one procedure/frame.

Status: complete for direct local label targets.

Deliverables:

- label symbol resolution
- local jump lowering
- unstructured IR/WASM lowering where structured WASM is insufficient
- diagnostics for missing labels, nonlocal targets, and Phase 7 designational
  forms

Acceptance:

- forward and backward local gotos work
- labels on empty statements work
- invalid label references are rejected
- unsupported nonlocal and conditional/switch designational forms are rejected
  with targeted diagnostics until their later phases land

### Phase 7: Nonlocal Goto and Switches

Goal: Support designational expressions across block boundaries.

Status: Phase 7a is complete for local switch declarations, local switch
selections, and local conditional designational `goto` forms. Phase 7b-1 is
complete for direct nonlocal block `goto` statements inside one lowered
function. Procedure-crossing `goto`, nonlocal conditional designational
branches, nonlocal switch selections, and nested non-recursive switch entries
now execute through the same pending goto machinery and designational
evaluation model.

Deliverables:

- switch descriptors (complete for local switches)
- direct-label switch entries (complete for local switches)
- conditional designational expressions (complete for local jumps)
- nonlocal target analysis (complete for direct block gotos)
- frame unwinding (complete for direct block gotos)
- procedure escape propagation

Acceptance:

- direct local switch selection works
- conditional designational expression jumps to the chosen local label
- direct nonlocal block goto exits nested frames correctly
- procedure-crossing goto exits called procedures correctly

### Phase 8: Rich Scalar Types

Goal: Add real, boolean, and string variables.

Deliverables:

- boolean storage and assignment
- real storage and arithmetic
- string literal descriptors
- initial string output helper

Acceptance:

- boolean variables participate in conditions
- real arithmetic executes with `f64` semantics
- string literals can be passed to output helpers

### Phase 9: `own` Storage

Goal: Implement static-lifetime ALGOL declarations.

Deliverables:

- `own` symbol kind
- static storage layout
- initialization rules
- access from nested procedures

Acceptance:

- `own integer` preserves value across procedure calls
- ordinary locals remain fresh per activation

### Phase 10: I/O and Program Harness

Goal: Give compiled ALGOL programs a stable observable runtime surface.

Deliverables:

- host I/O helpers
- stdout capture in tests
- optional WASI mapping
- package README examples

Acceptance:

- compiled programs can print integers, booleans, reals, and strings
- tests assert captured output without relying only on `_start` return values

### Phase 11: Cross-Language Rollout

Goal: Port stabilized concepts beyond Python.

Recommended order:

1. TypeScript
2. Go
3. Rust
4. Ruby, Elixir, Lua
5. JVM and .NET buckets if their nearby WASM lanes are ready

Each port should follow the same semantic model tests before adding
language-specific flourishes.

## Testing Strategy

### Golden Programs

Create a shared set of ALGOL source fixtures:

- scalar arithmetic
- lexical shadowing
- outer variable access
- value procedure call
- recursive function
- one-dimensional array
- multidimensional array
- call-by-name read
- call-by-name write
- Jensen's device
- local goto
- nonlocal goto
- switch declaration
- `own` variable
- real arithmetic
- string output

Each fixture should state:

- required compiler phase
- expected return value
- expected output
- expected diagnostics, if invalid

### Type Checker Tests

Every phase must add negative tests for invalid programs:

- duplicate declarations
- undeclared names
- invalid parameter modes
- wrong argument count
- wrong argument type
- invalid array bounds type
- invalid switch entry
- invalid goto target
- assignment to non-assignable expression

### Runtime Tests

Runtime tests should include both normal execution and failure paths.

Failure-path tests are required for:

- bounds errors
- division by zero
- invalid switch index
- non-assignable by-name store
- allocation cap exceeded

### Security Tests

Because source programs are untrusted input, phases that allocate or recurse
must add limits and tests.

Required limits:

- maximum source length accepted by convenience APIs
- maximum AST recursion depth where recursive visitors are used
- maximum block nesting depth
- maximum procedure nesting depth
- maximum dynamic call depth
- maximum runtime instruction fuel or host execution time
- maximum frame stack bytes
- maximum array dimension count
- maximum array allocation bytes
- maximum aggregate heap bytes
- maximum generated thunk count
- maximum generated label/state count
- stack/heap collision checks before every frame or heap allocation

The exact defaults may be conservative and configurable.

Current implementation note: the Python ALGOL lane enforces the first
front-end limits through the WASM package's source-length guard and the type
checker's configurable AST-depth, block-depth, and procedure-depth checks. The
runtime lowering already keeps frames, dynamic arrays, output, eval-thunk
descriptors, and generated control/helper label state bounded; future
convergence work should continue closing the remaining host-execution budget
items against this list.

## Documentation Requirements

Every implementation phase must update:

- package README files for supported language surface
- package CHANGELOG files
- this spec if implementation choices diverge
- PL03 only if the original first-lane contract changes

Each phase PR description should say:

- which phase it implements
- which ALGOL constructs are newly supported
- which constructs remain explicitly unsupported
- which tests prove the phase

## Non-Goals

This roadmap does not require:

- a browser UI for ALGOL programs
- optimizing generated WASM
- garbage collection
- full historical I/O compatibility with every ALGOL system
- simultaneous implementation in every repository language
- replacing the existing grammar
- hiding unsupported features behind partial miscompilation

## Completion Definition

Full ALGOL 60 to WASM is complete when:

- the existing `algol60` grammar is the only grammar used by the lane
- every grammar-supported ALGOL declaration form is either implemented or has a
  documented dialect exclusion
- nested procedures use correct lexical access
- value and by-name parameters behave correctly
- arrays with runtime bounds work
- labels, `goto`, switches, and designational expressions work
- `integer`, `real`, `boolean`, and string literals have runtime support
- `own` declarations preserve static lifetime
- runtime errors are bounded, explicit, and tested
- golden ALGOL fixtures execute through the local WASM runtime
- the Python packages document the full language surface
- any reusable IR/runtime extensions are covered by their own package tests

The final result should make ALGOL 60 a reference implementation for compiling
block-scoped, procedure-heavy historical languages onto the repository's WASM
stack.
