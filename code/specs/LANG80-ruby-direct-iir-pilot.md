# LANG80 — Ruby integer expressions on the native LANG VM

**Status:** Draft, 2026-10-07. First bounded Ruby-to-IIR pilot.

## Execution path

Ruby 3.0 source is tokenized and parsed by the existing Rust `ruby-lexer` and
`ruby-parser`. A new `ruby-iir-compiler` lowers the parser tree directly to
`interpreter-ir`; Rust `vm-core` executes it. Semantic IR and a host Ruby
runtime are not stages in the execution path. A Ruby installation may only
serve as an independent conformance oracle.

## Accepted source subset

The pilot accepts one or more `puts(<expression>)` statements. Each call has
exactly one positional argument. Expressions contain plain decimal integer
literals without separators,
parentheses, unary `-`, and binary `+`, `-`, `*`, `/` with Ruby's precedence and
left associativity. Variables, assignments, strings, interpolation, method
definitions, multiple arguments, and every other construct are rejected with
an unsupported-syntax diagnostic. The parser's original recursion guard
remains in force.

The decimal-only subset rejects multi-digit literals beginning with `0`:
Ruby interprets legacy forms such as `010` as octal. Until the pilot supports
their Ruby values, it must reject them rather than lower them as decimal.
The compiler must also check the lexer's numeric token type before reading
digits, because Ruby string token values can contain the same digits after
the lexer removes their quotes.

Ruby integers have arbitrary precision. This pilot accepts only programs for
which each literal and every intermediate expression value is provably within
`i64` range. There are no variables in the subset, so the compiler can check
that property with widened integer arithmetic before emitting IIR. Inputs
outside the range are rejected. The VM's `i64` arithmetic then agrees exactly
with Ruby for accepted `+`, `-`, and `*` expressions, with no runtime overflow.

Ruby integer `/` rounds toward negative infinity, while the VM's generic
integer division rounds toward zero. Emit a Ruby-specific VM builtin for `/`
that implements floor division and reports `ZeroDivisionError` for a zero
divisor. Reject any statically known division whose result would exceed `i64`
range. `puts` uses a separate Ruby-specific builtin that writes the integer's
decimal form and one newline. Output is bounded by the runner.
The pilot limits source to 64 KiB before parsing, a directly supplied AST to
16,384 nodes or tokens and depth 256 before recursive lowering, execution to
100,000 instructions, and captured output to one million bytes. Direct AST
input also has a one MiB aggregate text limit and 64 KiB per rule name or token
field, including correlation IDs; the module name has the same field limit.
The item count is enforced before adding children to the traversal queue.

## Acceptance

- `puts(1 + 2)`, `puts(7 / 2)`, and `puts(-7 / 2)` execute through source →
  Ruby parser → IIR → Rust `vm-core`, producing `3`, `3`, and `-4` respectively.
- The emitted IIR contains arithmetic instructions and the Ruby-specific
  builtins, rather than a precomputed output string or a host Ruby call.
- Integer range overflow, division by zero, unsupported syntax, and malformed
  source fail without panics. In particular, the compiler must use a fallible
  Ruby parser entry point so a lexer error is returned rather than unwrapped.
  A separate Ruby oracle cross-checks representative accepted programs.
- Oversized directly supplied AST item counts, nesting, text fields, and module
  names fail before lowering or copying token text.

This is an interpreter pilot. Bindings, objects, blocks, exceptions, the full
integer tower, float literals, and JIT execution remain later work.

## Follow-on bounded call form: `puts expression`

Ruby also permits a one-argument `puts` call without parentheses. The native
frontend may accept `puts 1 + 2` and `puts -7 / 2` through the existing
`method_call_no_paren` parser rule, using the same expression lowering and
integer bounds as the parenthesized form. Each accepted statement must have
exactly one positional expression and a literal `puts` callee. Multiple
arguments, keyword arguments, splats, blocks, other methods, and unsupported
expression forms still fail before VM execution. The resulting IIR must call
the existing `rb_puts_int` builtin; Ruby is only a conformance oracle.
Directly supplied ASTs must also carry a name-like callee token with a matching
effective grammar type; a string or number token spelling `puts` is not a call.

## Follow-on bounded empty call: `puts()`

An exact parenthesized zero-argument `puts()` call lowers through the Ruby
parser's three-child `method_call` AST to a zero-argument Ruby-specific IIR
builtin. It emits one newline on Rust `vm-core`, in source order with the
existing one-argument calls. The host Ruby runtime is only a conformance
oracle. The bare zero-argument spelling `puts` and all other new call forms
remain outside this stage. Directly supplied ASTs must have a literal
name-like `puts` callee and actual left/right parenthesis token kinds with
matching effective grammar types; matching delimiter text alone is
insufficient. The existing source, AST, VM instruction, and output bounds
still apply, including a checked output-length increment before appending the
newline.

## Follow-on bounded two-argument call: `puts(a, b)`

An exact parenthesized `puts` call with two positional integer expressions
lowers through the Ruby grammar AST directly to IIR. The Rust VM evaluates
both expressions in source order, then a Ruby-specific builtin appends their
decimal forms with a newline after each value. Host Ruby is a conformance
oracle only. Zero- and one-argument behavior remains as specified above;
bare multi-argument calls, three or more arguments, splats, keyword arguments,
blocks, and unsupported expressions remain outside this stage.

The compiler validates the callee, both parentheses, the comma token, and
both `call_arg` wrappers by token kind, effective grammar type, and rule shape
before lowering. Each expression must satisfy the existing `i64` proof and
resource bounds. The output builtin checks the combined size before appending
either value, so an output-limit failure does not partially append this call.
Expression errors reject the program before execution or stop VM execution
before this call's output builtin runs. The runner returns an error without
exposing buffered output on any failure.
