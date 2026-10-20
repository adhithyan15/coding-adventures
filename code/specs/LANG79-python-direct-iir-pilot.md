# LANG79 — Python float expressions on the native LANG VM

**Status:** Draft, 2026-10-07. This is the first bounded Python-to-IIR slice
under LANG78's four-language frontend program.

## Pipeline

Python 3.12 source is tokenized and parsed by the existing Rust `python-lexer`
and `python-parser` crates. The new `python-iir-compiler` lowers that parser's
grammar tree directly to `interpreter-ir`; `vm-core` executes the result in
Rust. Semantic IR is not an execution stage. The Python host interpreter may
be used only to confirm expected results.

## Accepted source

The first slice accepts a file of expression statements formed only from
Python float literals, parentheses, unary `+` and `-`, binary `+`, `-`, `*`,
and `/`, and `print(<one accepted expression>)`. Statements execute in source
order. Every literal and intermediate arithmetic result has Python `float`
semantics. A numeric spelling without a decimal point or exponent is an
arbitrary-precision Python `int`, so this slice rejects it. It also rejects
assignments, names other than the exact `print` callee, keyword arguments,
multiple print arguments, semicolons, and every other statement or operator.
Errors must identify unsupported syntax rather than silently omit it.
Python float division by zero raises an error; the VM lowering must check that
case rather than inheriting an IEEE infinity result from a generic `f64` divide.

The pilot rejects source larger than 64 KiB before tokenization; the file
runner enforces that bound while reading. Its AST-input
entry point also rejects trees exceeding 16,384 nodes or tokens, or 64 nested
grammar nodes, before recursive lowering. Supplied ASTs additionally have a
1 MiB aggregate text budget and a 64 KiB limit per rule name or token field;
the IIR module name has the same field limit. These limits bound frontend and
generated-IIR work even when an AST is supplied directly by another caller.

`print` is a Python-specific VM builtin registered by the Rust runner. It
prints one float and a newline. Its accepted display range is zero and finite
values with absolute magnitude from `1e-4` through `1e15`; values outside
that range are rejected until Python's full float formatting is implemented.
Within this range, integral floats include a `.0` suffix and negative zero
prints `-0.0`. NaN and infinities are initially rejected at the print
boundary. This display restriction does not change arithmetic semantics.
If a later statement fails at runtime, an earlier successful `print` remains
visible on stdout; the runner's error value retains both that output and the
error message.

## Acceptance

- Source `print(1.0 + 2.0)` executes on `VMCore` and prints `3.0`.
- Source `print(1.0 / 2.0)` executes on `VMCore` and prints `0.5`.
- Source `print(-0.0)` prints `-0.0`, preserving the float sign bit.
- `print(1 + 2)`, `print(1.0 // 2.0)`, and assignment fail explicitly.
- `print(1.0 / 0.0)` raises a division-by-zero error on the native runner.
- `print(1.0)` followed by a failing division still writes `1.0` before the
  error, matching source-order Python output.
- The emitted `IIRModule` validates before execution; tests confirm that the
  Rust runner does not invoke a host Python process.
- Oversized source and directly supplied deep or wide ASTs fail with explicit
  pilot-limit errors before lowering; oversized AST text and module names do too.

The slice is an interpreter pilot, not a Python engine completion claim.
Bindings, integers, strings, truth, control flow, functions, exceptions,
objects, library modules, and JIT tiers remain separate work.

## Bounded follow-up: zero-argument `print()`

Accept the exact Python 3.12 grammar call shape `print()` as a statement. Lower
it directly from the grammar tree to an IIR call of a zero-argument Rust
`vm-core` builtin that appends one newline to captured stdout. It consumes
the same output budget and preserves the same source-order error behavior as
the existing one-float `print` path. Host Python is a conformance oracle only.

This stage does not accept multiple arguments, keyword arguments, other
callees, or new expression forms. Both source parsing and direct AST lowering
must reject malformed call shapes rather than treating them as an empty call.
The callee must be a name token in an `atom` node and both delimiters must be
actual parenthesis tokens; matching token text alone does not validate a
caller-supplied AST. Apply the same check to the one-argument path.
`print()\nprint(1.0)\n` must produce `\n1.0\n`; a completed empty print before
a later float-division failure must remain in the typed run error's output.

## Bounded follow-up: two positional float arguments

Accept exactly `print(<float expression>, <float expression>)` in the Python
3.12 grammar tree. Lower the two expressions from left to right to IIR, then
call a Rust `vm-core` builtin once to append their displayed values separated
by one space and followed by one newline. Use the existing float display
range, output byte limit, source and AST limits, and typed partial-output error
behavior. Host Python is a conformance oracle only; Semantic IR is not an
execution stage.

The call must contain two positional `argument` nodes separated by an actual
comma token. The exact `print` callee and parenthesis token-kind checks remain
mandatory for direct AST callers. Reject keyword or unpacking arguments,
three or more arguments, and other expression forms explicitly. A failure
while evaluating the second argument must not emit part of that call's line;
earlier completed print calls remain visible. For example,
`print(1.0 + 2.0, -0.0)` prints `3.0 -0.0\n`, and a later
`print(2.0, 1.0 / 0.0)` retains only the earlier output.

## Bounded follow-up: three positional float arguments

Accept exactly `print(<float expression>, <float expression>, <float expression>)`
from the Python 3.12 grammar tree. Lower all three expressions from left to
right to InterpreterIR, then invoke one Rust `vm-core` builtin that appends
their displayed values with one space between each and one final newline.
Host Python is a conformance oracle only; Semantic IR stays outside execution.
Keep the existing float display range, output-byte budget, source and direct-AST
limits, and typed prior-output error behavior. A failure while evaluating any
argument or displaying any value must append none of that call's line, while
earlier completed calls remain visible.

The call must have three positional `argument` nodes separated by actual comma
tokens. Keep exact `print` callee and parenthesis token-kind checks for direct
AST callers, and reject forged comma kinds, keyword or unpacking arguments,
four or more arguments, and unsupported expressions. `print(1.0, -0.0, 2.5)`
must yield `1.0 -0.0 2.5\n`. A later `print(2.0, 3.0, 1.0 / 0.0)` must retain
only the earlier completed line.

## Bounded follow-up: arithmetic token validation for direct AST callers

Before lowering a caller-supplied arithmetic AST, require unary `+`/`-` and
binary `+`/`-`/`*`/`/` tokens to have the canonical lexer token kind, no
conflicting effective-type override, and the exact supported spelling.
Arithmetic `paren_expr` nodes must contain actual opening and closing
parenthesis tokens around the grammar's existing `paren_body` wrapper,
which must contain exactly one `expression` node. Matching
text alone does not validate an operator or delimiter. Reject a node in
place of a token and invalid spellings even if the other fields look valid.

Preserve source arithmetic and precedence, source-order evaluation, float
and output bounds, zero/one/two/three-argument print, and prior-output errors.
Apply these checks in standalone expressions and every print argument.
The existing source and direct-AST item/depth/text budgets run before
lowering. Typed grammar AST still lowers directly to InterpreterIR and Rust
vm-core; host Python remains a separate conformance oracle. Do not extend
expression syntax or claim general validation of every possible forged AST.
Tests must first expose previously accepted forged operators/delimiters,
then cover token kind, effective type, spelling, token/node substitution,
all argument positions, and unchanged source/runtime behavior.

## Bounded follow-up: float literal validation for direct AST callers

Require a float atom token to have the canonical grammar-lexer representation:
`TokenType::Name` with the exact named type `FLOAT`. Validate its complete raw
ASCII spelling against the three FLOAT forms declared in
`code/grammars/python/python3.12.tokens` before removing underscores or
converting to f64. Decimal digit groups permit a single underscore only
between digits. Accepted forms are digit-group exponent, dot digit-group
with optional exponent, and digit-group dot digit-group with optional exponent.
An exponent contains e/E, an optional sign, and a nonempty digit group.

Reject forged enum kinds, missing or conflicting named types, integer-only
spellings, misplaced underscores, embedded signs/whitespace, Unicode digits,
NaN/infinity names, suffixes and trailing bytes. This preserves the existing
versioned lexer subset: trailing-dot floats such as `1.` are not newly added.
Use a bounded linear byte scan with no host runtime or new regex dependency.
Existing AST text/item/depth and module-name guards still run first.

Keep source parsing, arithmetic, signed zero, float overflow/underflow,
display limits, zero through three positional print arguments and prior-output
errors unchanged. In particular a syntactically valid `1e9999` still lowers
to IEEE infinity and fails only if displayed; do not reject valid literals
merely because f64 conversion overflows. Conversion uses raw spelling rather
than optional cooked metadata. This is no general forged-AST validation claim.

Tests must expose the prior acceptance of forged FLOAT tokens before the fix,
cover malformed raw spellings in standalone expressions and every print
argument position, and preserve real parser-produced token forms. Compare
native Rust VM output and errors to host Python solely as an external oracle.

## Bounded follow-up: module-level float bindings

Accept a module-level statement of exactly one name target, one `=` token and
one accepted float expression: `NAME = <float expression>`. The Python 3.12
grammar tree for this is an `assign_stmt` with three children: a target
`expression_list` holding one `expression`, an `EQUALS` token, and a value
`expression_list` holding one `expression`. The target `expression` must
descend through single-child wrappers to an `atom` whose only child is one
`NAME` token. Reassignment of an already bound name is accepted. NAME atoms
are accepted inside every float expression: standalone expression statements,
the right-hand side of a binding and each of the zero-to-three `print`
argument positions. Every binding value is a Python `float`.

Python evaluates the right-hand side completely before binding the target, so
`x = x + 1.0` reads the previous value. The lowering evaluates the value into
compiler temporaries and then emits one IIR `mov` into the name's binding
register. A literal value therefore lowers to `const` followed by `mov`.
A use reads the binding register directly; expressions in this subset cannot
rebind a name, so no snapshot copy is needed.

User names are mangled into the `py:` register namespace (`x` becomes
`py:x`). Compiler temporaries remain `py` followed by decimal digits, and
builtin names are carried only as the literal first operand of
`call_builtin`, so neither can collide with a Python identifier.

A use before any binding raises `NameError: name 'x' is not defined` at run
time. The subset has no branches, loops, functions or `del`, so whether a name
is bound is known statically at every program point: a name is bound after
the first earlier statement that binds it and stays bound. The compiler emits
a `call_builtin py_name_error "x"` at exactly that use. Earlier statements'
output is retained in the typed run error and later statements do not run,
matching CPython. Evaluation stays left to right, so `print(1.0 / 0.0, y)`
still raises `ZeroDivisionError` and `y + 1.0 / 0.0` raises `NameError`.

Some unbound reads do not raise `NameError` in CPython because the name
resolves to a builtin or to a module attribute that this pilot cannot
represent. Reading `print` as a value, any other Python 3.12 builtin name
(for example `float`, `type` or `len`) before the module binds it, or any
dunder name (`__name__`, `__debug__` and so on) before it is bound, fails at
compile time as unsupported rather than producing a wrong `NameError`.
Binding a non-`print` builtin name such as `float` or soft keywords such as
`match`, `type` and `_` is ordinary module-global shadowing and is accepted.

The following fail explicitly at compile time, with distinct messages:

- binding `print`, which the pilot treats as its only callable builtin;
- binding `__debug__`, which CPython rejects as a syntax error;
- chained assignment such as `a = b = 1.0`;
- tuple, list, parenthesized or starred targets, and tuple values such as
  `x = 1.0,` (starred targets are already parse errors in this grammar;
  caller-supplied ASTs are rejected by the single-NAME target rule);
- annotated assignment `x: float = 1.0`;
- augmented assignment such as `x += 1.0`;
- attribute and subscript targets such as `a.b = 1.0` and `a[0] = 1.0`;
- keyword targets such as `True = 1.0`;
- binding a `print(...)` call result, which would be `None`;
- integer right-hand sides such as `x = 1`, which still require
  arbitrary-precision runtime support;
- multiple statements joined by semicolons, as before.

Direct-AST callers bypass the lexer, so the binding path validates syntax
tokens the same way as the earlier token slices. The `=` token must be
`TokenType::Equals` with no effective-type override and the exact spelling
`=`. Every target and every read NAME token must be `TokenType::Name` with no
override and an ASCII spelling matching the versioned lexer's NAME pattern
`[a-zA-Z_][a-zA-Z0-9_]*` that is not a hard Python keyword. A node in place of
either token, a forged kind, a named override or another spelling fails.
The existing source size and direct-AST item, depth and text budgets run
before lowering and cover the new nodes; the VM instruction budget and output
limits are unchanged.

Acceptance:

- `x = 1.5\nprint(x * 2.0)\n` prints `3.0\n`.
- `x = 1.0\nx = x + 1.0\nprint(x)\n` prints `2.0\n`.
- `print(1.0)\nprint(y)\n` retains `1.0\n` and then raises `NameError`.
- `print(1.0)\nprint(z)\nz = 2.0\n` raises `NameError` after `1.0\n`.
- Each rejection above fails with a compile error.
- Forged NAME and EQUALS tokens fail in every accepted position.
- A literal binding lowers to `const` then `mov` into `py:x`.

This slice does not add integers, strings, `del`, `global`, functions,
control flow or any non-module scope. Host Python remains a conformance
oracle only; Semantic IR is not an execution stage.
