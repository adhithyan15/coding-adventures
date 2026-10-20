# LANG78 — Native dynamic-language frontends for InterpreterIR

**Status:** Draft, 2026-10-07. This spec tracks four bounded frontend lines.

## Execution contract

JavaScript, Python, Ruby, and Perl programs should eventually run in the
repository's Rust LANG VM. Each language owns its syntax and semantics:

```text
source → language lexer/parser → language AST/CST lowering → IIRModule
       → vm-core interpreter → later JIT tiers
```

Semantic IR is **not** a stage in this execution path. Its existing
source-to-source frontends remain separate; any future integration must not
change the direct language-to-IIR contract. A host runtime may serve as a
test oracle, but must not execute the compiled program in production.

LANG84 (`LANG84-grammar-driven-versioned-frontends.md`) is the frontend
contract for these four lines and for every other IIR frontend. Lexing and
parsing come only from the generic grammar-driven lexer and parser over a
per-release `<lang><release>.tokens` / `.grammar` pair. The IIR compiler walks
that parse tree directly; it does not lift it into a typed AST. Every tagged
release is inventoried in `releases.csv` and selectable without silent
fallback, and the emitted operations come from LANG83. Where this spec's
earlier slices describe a typed-AST bridge (JavaScript), a hand-written Ruby
lexer, or a hard-coded version, LANG84's migration plan supersedes them.

Use one active implementation PR at a time. Interleave this queue with the
existing LANG VM backlog and PREP01 preprocessor queue, selecting the next
bounded item after each verified merge.

## Current foundations

| Language | Rust lexer/parser | Lowering input | First missing piece |
| --- | --- | --- | --- |
| JavaScript | Yes, including a typed `javascript_ast::Program` bridge (not on the IIR path) | Generic grammar tree (CST) since `javascript-iir-compiler` 0.3.0; typed AST before | Broader subset; LANG84 phases 6a/6b/6d/6e |
| Python | Yes, versioned through 3.12 | Generic grammar tree | Direct tree-to-IIR compiler |
| Ruby | Yes | Generic grammar tree | Direct tree-to-IIR compiler |
| Perl | No Perl-language grammar, lexer, or parser in Rust | None | Define a bounded Perl grammar and parser first |

The existing `interpreter-ir` and `vm-core` crates already support `f64`
constants and arithmetic, `call_builtin`, and execution through `VMCore`.
That is sufficient for first numeric programs. It is not a promise that
dynamic objects, coercions, closures, exceptions, and built-in libraries
already work unchanged. Emit a clear unsupported-syntax or unsupported-
semantic diagnostic whenever the selected slice cannot preserve behavior.

## Slice A — JavaScript interpreter pilot

Create `code/packages/rust/javascript-iir-compiler` with `compile_source`
and `compile_ast` APIs. Read the typed AST from `javascript-parser`; do not
round-trip through Semantic IR. (Superseded by LANG84 phase 6c: since
`javascript-iir-compiler` 0.3.0 the compiler reads the generic
`GrammarASTNode` from `parse_javascript_cst_division_only` and `compile_ast`
takes that tree; the emitted IIR and the errors are unchanged.) Compile JavaScript numeric literals as
`Operand::Float` even when their spelling has no decimal point. The first
accepted source subset is expression statements containing numeric literals,
parentheses, unary minus, and binary `+`, `-`, `*`, `/`, plus
`console.log(<accepted expression>)`. Preserve evaluation order. Compile to
an `IIRModule` with a `main` function. `console.log` uses a narrowly named
VM builtin registered by the JavaScript runner; its output matches JavaScript
number formatting for the accepted values. The pilot accepts ordinary finite
display values with magnitude from `1e-6` (inclusive) to `1e21` (exclusive),
plus zero, NaN, and infinities. It rejects other display magnitudes until the
full ECMAScript number-to-string algorithm is available. Reject all other
syntax. Bound source to 64 KiB, direct typed ASTs to 16,384 visited nodes and
expression depth 64, VM execution to 100,000 instructions, and captured output
to one million bytes. The file runner must enforce the source bound while
reading rather than after loading the whole file.

Acceptance requires actual source → parser → IIR → `VMCore` tests for
`console.log(1 + 2)` and `console.log(1 / 2)`, plus a negative test for an
unsupported construct. A separate Node oracle may cross-check results.
The native runtime test may not invoke Node to execute the compiled module.

For the next bounded JavaScript runner stage, preserve output from every
completed `console.log` when a later accepted statement fails during VM
execution. Return that prior output alongside the runtime diagnostic, and
have the `jsvm` command write and flush it to stdout before reporting the
error on stderr. Compilation failures have no completed output. The supported
source subset and number-display range do not expand in this stage. A later
out-of-range display supplies a reachable runtime-error regression; the host
JavaScript runtime serves only as an oracle for the ordering of completed
console effects, never as the executor of the IIR module.

The next bounded JavaScript stage also accepts `console.log()` with no
arguments through the existing typed JavaScript AST. Lower it directly to an
IIR builtin call with no value operand; the Rust VM appends exactly one newline
while preserving the order and output cap of completed console effects. The
one-argument numeric form is unchanged, and two or more arguments and other
callees still reject. A later accepted VM error retains the preceding empty
log's newline; a compile error returns no output. Test source-to-AST-to-IIR
execution and direct typed-AST lowering, with Node used only as an output
oracle. The broader JavaScript coercion and string-display rules stay open.

The next bounded stage accepts exactly two positional numeric expressions in
`console.log` through the typed JavaScript AST. Lower both expressions in
source order to the existing Rust VM builtin. Format each with the pilot's
bounded JavaScript Number display rule, join them with one space, and append
one newline. Keep zero- and one-argument behavior, the one-million-byte output
cap, and prior completed console output when a later accepted call fails at
runtime. A display failure in either argument must append none of that call's
text. Reject three or more arguments and unsupported expressions during
compilation, with no completed output. Verify actual AST-to-IIR-to-VM execution
against a Node output oracle; do not use Node to run the compiled module.

The next bounded stage accepts exactly three positional numeric expressions
in `console.log`, retaining the zero-, one-, and two-argument forms. Compile
every argument in source order directly from the typed AST to IIR and execute
on Rust `vm-core`. Format all three numbers before appending any call output;
join them with one space and append one newline. A display failure in any
position retains only output from earlier completed calls. Four or more
arguments and unsupported expressions still reject at compile time with empty
output. Retain all source, AST node/depth, instruction, and output bounds,
including budget checks on the third argument of a caller-supplied AST.
Verify IIR argument order, native arithmetic and special-number output,
each display-failure position, and rejection boundaries. Use Node only as a
separate output oracle. Argument evaluation follows ECMAScript
[ArgumentListEvaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-argument-lists-runtime-semantics-argumentlistevaluation);
the numeric console fixtures use Node's
[console.log](https://nodejs.org/api/console.html#consolelogdata-args) behavior.

The next bounded stage accepts top-level numeric `let` and `const`
declarations with exactly one plain-identifier binding and an initializer in
the existing numeric expression subset, plus identifier references wherever a
numeric expression is accepted (including `console.log` arguments and other
initializers). Lower each binding directly from the typed AST to a named IIR
register written by `mov` after its initializer evaluates. Register names are
mangled into a namespace that cannot meet compiler temporaries (`js<n>`) or
runner builtins (`js_<name>`). There is no assignment or control flow yet, so
binding state is statically known at every program point.

Script semantics follow ECMAScript
[ScriptEvaluation](https://tc39.es/ecma262/multipage/ecmascript-language-scripts-and-modules.html#sec-runtime-semantics-scriptevaluation)
and [GlobalDeclarationInstantiation](https://tc39.es/ecma262/multipage/ecmascript-language-scripts-and-modules.html#sec-globaldeclarationinstantiation):

- A duplicate lexical name in the script is an early error. Compilation fails
  with `SyntaxError: Identifier '<name>' has already been declared` and no
  output, as V8 reports it.
- Reading a name declared later in the script (or inside its own initializer)
  is a temporal-dead-zone read. Reading a name declared nowhere is an
  unresolvable reference. Both lower to a narrow runner builtin call at that
  point, raising `ReferenceError: Cannot access '<name>' before initialization`
  or `ReferenceError: <name> is not defined` at runtime. Earlier completed
  console output is kept, as for other VM errors.
- The runner's global object is defined to hold the ECMA-262 standard global
  properties (plus Annex B `escape`/`unescape` and ECMA-402 `Intl`) and
  `console`. None is implemented as a value yet. Reading such a name, or binding one with
  `let`/`const`, is rejected at compile time; this keeps `NaN`, `undefined`
  and similar names from becoming false ReferenceErrors and avoids
  restricted-global declaration rules. Over-listing only rejects more input.
  Node-only host globals (`process`, `require`, ...) are not part of this
  runner's host, so they read as undeclared.

Reject with compile-time diagnostics and empty output: `var`, a declaration
without an initializer, more than one declarator per declaration, destructuring
patterns, non-numeric initializers, assignment expressions (so assignment to a
`const` cannot arise), and caller-supplied AST identifiers that are not plain
IdentifierName spellings. A direct AST charges one node for each declarator and
one for its binding, then applies the existing node and depth budget to the
initializer. Verify source → AST → IIR → `VMCore` execution, direct typed-AST
lowering, the IIR register namespace, each rejection, both ReferenceError forms
retaining prior output, the early duplicate error, and initializer budgets;
Node remains only an output oracle.

### Native numeric source division goal

The native numeric frontend must use an explicitly restricted division-only
tokenizer/parser entry. Remove `REGEX` and `BAD_REGEX` rules from the selected
compiled token grammar for this entry, while retaining slash/division-assignment
punctuation, comment skipping, strings, locations, newline flags, version choice
and the existing ASI/parser-depth path. Do not rewrite source text or split an
already emitted regex token: those approaches lose lexical boundaries.

The native subset admits no regular-expression literals. Slash at an operand
position must fail parsing; regex-shaped input and unsupported division
assignment must still reject compilation with empty output. Repeated division
in one expression and across console arguments must reach typed AST-to-IIR
lowering in source order, including `console.log(-0, 1 / 0, 0 / 0)`.
Retain source/AST/instruction/output limits and atomic display failures.

This restricted typed parser must reject exponentiation and exponentiation
assignment tokens (`STAR_STAR` / `STAR_STAR_EQUALS`) before ASI parsing. Neither
operator is in LANG78's native numeric subset; long right-recursive exponent
chains can overflow during parsing before any post-parse spine check. Do not
inspect raw source spelling: quoted text and comments must not trigger this
token check. Keep the general parser's exponentiation behavior unchanged.

Before the restricted parser constructs its typed AST, iteratively check a
conservative grammar-tree spine measure: each grammar node adds one, and flat
binary/member/call chains also add their number of child elements minus one.
Reject a measure above 512 before bridging. This safety bound is separate from
the native compiler's semantic expression-depth limit of 64: every supported
expression within that limit and the parser's rule-depth limit fits it. It
prevents a long flat operator chain from constructing recursively dropped boxes
that overflow the stack even when the later AST depth check rejects execution.
Verify hostile 5,000/20,000-term chains return errors without process abort.

This entry is for callers whose language subset excludes regex literals.
The general JavaScript lexer/parser entries retain their current behavior;
context-sensitive regex-versus-division selection for full JavaScript remains
open. ECMAScript distinguishes these lexical goals in
[Lexical Grammar](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-ecmascript-language-lexical-grammar).
Verify native execution against Node as a separate oracle, plus token locations,
comment boundaries, preserved full-entry regex tokens and rejection probes.

## Slice B — Python

Create `python-iir-compiler` against `python-parser`'s grammar tree. Its first
slice may accept only float literals and float arithmetic plus `print`, and
must reject integer programs until arbitrary-precision integer behavior is
provided. The Python `1 / 2`, `//`, truth, and print contracts need their own
lowering and tests; do not reuse JavaScript numeric rules by accident.

## Slice C — Ruby

Create `ruby-iir-compiler` against `ruby-parser`'s grammar tree. Its first
slice may accept only float literals and float arithmetic plus `puts`, and
must reject integer programs until Ruby integer and `/` behavior can be
preserved. Add native VM execution and Ruby-oracle conformance tests.

## Slice D — Perl

First add a bounded Perl-language token and grammar definition plus Rust
lexer/parser. Then create `perl-iir-compiler` against that parser. The first
executable slice may be numeric scalar expressions and `say`, but must define
Perl's scalar context, coercion, and version target before lowering them.

## Shared runtime follow-ups

The three-argument fixture audit exposed a source-lexer limitation:
`console.log(-0, 1 / 0, 0 / 0)` tokenizes `/ 0, 0 /` as a regex literal
and fails parsing before IIR lowering. The bounded native numeric division-only
entry above repairs this fixture for LANG78 while excluding regex execution.
General JavaScript still requires context-sensitive lexical-goal selection;
the native restriction must not be presented as that broader repair.

After the scalar pilots, expand each language separately through bindings,
assignments and scope, strings and coercions, control flow, functions,
closures, collections, objects, exceptions, and library calls. Add the
language binding/runtime hooks where semantics require them. Only then add
JIT execution against the same IIR with guards and deoptimization; retain
interpreter tests as the reference behavior.

Do not declare a language complete from its first scalar pilot.
