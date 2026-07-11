# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `code_printer_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
    - blob SHA at port time: `64944e8e615c95b0cf845aab86f77d634776a5b1`
    - tracked commit: see `UPSTREAM_SHA`

- `code_printer_function_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the `testFunctionExpression*` / IIFE / `function`-at-statement-start
      cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_function_expression` + the precedence wrap that landed
      with `Expression::FunctionExpression` (CLOC12.149) and became reachable
      once the bridge converted `function_expression` (gap-153). 12 active
      `#[test]`s, no `#[ignore]` — the emitter conforms to every covered
      shape (anonymous/named, params, body, IIFE, member-object,
      call-argument, generator/async prefixes).

- `code_printer_arrow_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the arrow-function `=>` printing cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_arrow_function_expression` + the `PREC_ASSIGNMENT`
      precedence wrap that landed with `Expression::ArrowFunctionExpression`
      (CLOC12.151). 12 active `#[test]`s, no `#[ignore]` — the emitter
      conforms to every covered shape (single/zero/multi param, concise vs
      block body, object-literal-body wrap, IIFE, member-object,
      call-argument, async prefix). Inputs are hand-constructed AST so the
      port can exercise block-bodied arrows the grammar can't yet parse
      (gap-156).

- `code_printer_template_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the template-literal `` `…` `` printing cases, including `${…}`
      substitutions and the multiline internal-whitespace case)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_template_literal` / `emit_template_element` + the
      `PREC_PRIMARY` classification that landed with
      `Expression::TemplateLiteral` (CLOC12.154). 19 active `#[test]`s and
      **0 `#[ignore]`** — the emitter conforms to every covered shape
      (no-substitution, escaped backtick / escaped `${`, member-object and
      binary operands without wrapping, single / adjacent / text-interleaved
      `${…}` substitutions, low-precedence / member-access substitution
      bodies, and multiline quasis with literal interior newlines). Inputs are
      hand-constructed AST so the port can exercise `${…}` substitution
      templates the grammar tokenises only as no-substitution today (gap-157).
      The multiline case (`raw_preserves_internal_newline`) was `#[ignore]`d
      under gap-158 until CLOC12.157 made `emit_template_element`
      newline-aware; it and `raw_preserves_leading_and_trailing_newline` are
      now active.

- `code_printer_update_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the update-operator `++` / `--` printing cases — prefix/postfix ×
      increment/decrement, precedence wraps, and the token-fusion seams)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_update` + the `PREC_UNARY` classification and the
      `+`/`-` token-fusion seams that landed with
      `Expression::UpdateExpression` (CLOC12.158). 14 active `#[test]`s and
      **0 `#[ignore]`** — the emitter conforms to every covered shape
      (prefix/postfix increment/decrement, member operand, bare under
      `!` / `typeof`, member-object and exponent-base precedence wraps
      `(x++).y` / `(++x)**2`, and the fusion seams `a- --b` / `a+ ++b` /
      `x++ +y` / `x-- -y` plus the non-fusing `x++*y`). Inputs are
      hand-constructed AST; the bridge conversion of `++`/`--`
      (CLOC12.158 PR2, gap-159) is exercised separately in
      `javascript-parser`.

- `code_printer_new_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the `new`-operator `new Ctor(args)` printing cases — argument lists,
      precedence, and the callee-with-call wrapping)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_new` + the `PREC_PRIMARY` classification, the
      `new`-keyword space, and the callee-with-call wrap that landed with
      `Expression::NewExpression` (CLOC12.159). 10 active `#[test]`s and
      **0 `#[ignore]`** — the emitter conforms to every covered shape
      (identifier / member-chain callee, argument lists, member argument,
      the callee-with-call wraps `new (f())()` / `new (a.b().c)()`, the
      argumented-`new`-as-member-object cases `new X(a).y` / `new X().y`,
      nested `new new X()()`, and a call on a `new` member `new X().m()`).
      Inputs are hand-constructed AST; the bridge conversion of `new`
      (CLOC12.159 PR2, gap-160) is exercised separately in
      `javascript-parser`.

- `code_printer_sequence_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the comma-operator `a, b, c` printing cases — bare positions and the
      assignment-position wraps)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_sequence` + the `PREC_SEQUENCE` (lowest) classification
      and the four assignment-position wrap sites that landed with
      `Expression::SequenceExpression` (CLOC12.160). 9 active `#[test]`s and
      **0 `#[ignore]`** — the emitter conforms to every covered shape: the two
      bare positions (statement `a,b,c`, computed-member key `a[b,c]`) and the
      wrapped positions (sole/multi call argument `f((a,b),c)`, array element
      `[(a,b),c]`, assignment RHS `x=(a,b)`, conditional branch `x?(a,b):c`,
      unary operand `!(a,b)`). Inputs are hand-constructed AST; the bridge
      conversion of the comma operator (CLOC12.160 PR2, gap-161) is exercised
      separately in `javascript-parser`.

- `code_printer_spread_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the spread `...arg` printing cases — bare argument/element positions and
      the assignment-position wrap)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_spread` + the `PREC_ASSIGNMENT` classification that landed
      with `Expression::SpreadElement` (CLOC12.162). 10 active `#[test]`s and
      **0 `#[ignore]`** — the covered shapes: spread call arguments (sole
      `f(...a)`, interleaved `f(a,...b,c)`, two adjacent `f(...a,...b)`, member
      argument `f(...a.b)`), array elements (sole `[...a]`, interleaved
      `[1,...a,2]`), `new` arguments (`new F(...a)`, interleaved
      `new F(a,...b)`), and the precedence cases (sequence argument wraps
      `f(...(a,b))`, conditional argument stays bare `f(...a?b:c)`). Inputs are
      hand-constructed AST; the bridge conversion of the spread form
      (CLOC12.162 PR2, gap-163) is exercised separately in `javascript-parser`.

- `code_printer_yield_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the generator `yield` / `yield*` printing cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_yield` + the `PREC_ASSIGNMENT` classification that landed
      with `Expression::YieldExpression` (CLOC12.163). 9 active `#[test]`s and
      **0 `#[ignore]`** — the covered shapes: the three surface forms (bare
      `yield`, non-delegate `yield a` with its mandatory keyword↔operand space,
      delegate `yield*xs` with no space plus the member-operand `yield*a.b`),
      the operand-precedence cases (conditional `yield a?b:c` and assignment
      `yield a=b` stay bare, sequence `yield (a,b)` wraps), and the
      whole-node-precedence cases where a tighter parent wraps the yield
      (`(yield a)+1`, `(yield a).b`). Inputs are hand-constructed AST; the
      bridge conversion of yield (CLOC12.163 PR2, gap-164) is exercised
      separately in `javascript-parser` once generator bodies parse.

- `code_printer_await_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the async `await` printing cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_await` + the `PREC_UNARY` classification that landed with
      `Expression::AwaitExpression` (CLOC12.164). 9 active `#[test]`s and **0
      `#[ignore]`** — `await` printed like the word-unaries typeof/void/delete:
      the surface `await p` (mandatory keyword↔operand space), operands that
      bind tighter print bare (`await a.b`, `await f()`) while a looser binary
      operand wraps (`await (a+b)`), and the whole-node precedence cases where
      the unary-strength await is left bare under a binary parent (`await p+1`)
      but wrapped by member/call parents (`(await p).x`, `(await f)()`), by the
      exponentiation base (`(await p)**2` — a bare `await p**2` is a syntax
      error), and nested (`await await p`). Inputs are hand-constructed AST; the
      bridge conversion of await (gap-165) is deferred — the current grammar
      treats `await` inside an async body as a plain identifier, so it does not
      yet parse (see the spec).

- `code_printer_this_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the `this` keyword printing cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_this` + the `PREC_PRIMARY` classification that landed with
      `Expression::ThisExpression` (CLOC12.165). 7 active `#[test]`s and **0
      `#[ignore]`** — `this` printed as a bare reserved-word primary that never
      needs wrapping and never wraps an operand: the surface `this`, as a member
      object (`this.x`), as a call callee (`this()`), as a call argument
      (`f(this)`), composed in a member chain (`this.a.b`) and a method call
      (`this.m()`), and left bare under a binary parent (`this+1`). Inputs are
      hand-constructed AST; the bridge conversion of `this` (gap-166) is
      exercised separately in `javascript-parser` (CLOC12.165 PR2) — unlike
      `await`, `this` was already parseable, so that bridge slice needed no
      grammar work.

- `code_printer_super_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the `super` keyword printing cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_super` + the `PREC_PRIMARY` classification that landed with
      `Expression::Super` (CLOC12.166). 7 active `#[test]`s and **0 `#[ignore]`**
      — `super` printed as a bare reserved-word primary (the sibling of `this`)
      that never needs wrapping and never wraps an operand: the surface `super`,
      as a member object (`super.x`), as a call callee (`super()`), as a call
      argument (`f(super)`), composed in a member chain (`super.a.b`) and a
      method call (`super.m()`), and left bare under a binary parent
      (`super+1`). Inputs are hand-constructed AST; the bridge conversion of
      `super` (gap-167) is exercised separately in `javascript-parser`
      (CLOC12.166 PR2) — like `this` and unlike `await`, `super` was already
      parseable, so that bridge slice needed no grammar work. The bare `super;`
      / `f(super)` / `super+1` inputs isolate the printer's leaf handling and
      are not asserted to be valid JS (`super` is syntactically restricted to
      member/call position inside a method or derived constructor).

- `code_printer_class_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the class-expression `class[ id][ extends S]{members}` printing cases)
    - tracked commit: see `UPSTREAM_SHA`
    - Isolates `emit_class` + `emit_class_member` + the `PREC_UNARY`
      classification that landed with `Expression::ClassExpression`
      (CLOC12.173). 22 active `#[test]`s and **0 `#[ignore]`** — the emitter
      conforms to every covered shape: the statement-start wrap (`(class{});`),
      anonymous/named surface, the four `extends`-operand precedence cases
      (identifier / member / call heritage print bare, a conditional heritage
      wraps: `extends (a?b:c)`), the member forms (empty / params+body /
      `static` / `get` / `set` / `constructor` / stacked `static get` /
      generator `*m` / `async m` / computed `[k]` / two members back-to-back),
      and the whole-node precedence cases where the class wraps as a member
      object (`(class{}).x`) and a call callee (`(class{})()`) but stays bare
      under a binary parent (`class{}+1`). Inputs are hand-constructed AST; the
      bridge conversion of `class_expression` (CLOC12.173 PR2, gap-167) is
      exercised separately in `javascript-parser`, and building the AST directly
      lets the port cover generator / async / computed-key methods and
      multi-member classes the grammar cannot yet parse.

- `code_printer_class_declaration_test.rs`
    - upstream: `test/com/google/javascript/jscomp/CodePrinterTest.java`
      (the class-**declaration** `class <id>[ extends S]{members}` printing
      cases — the `class` keyword in statement position)
    - tracked commit: see `UPSTREAM_SHA`
    - Companion to `code_printer_class_test.rs`; isolates
      `emit_class_declaration` + the shared `emit_class_tail` helper that landed
      with `Declaration::ClassDeclaration` (CLOC12.174 PR1). 20 active `#[test]`s
      and **0 `#[ignore]`** — the emitter conforms to every covered shape: the
      declaration is emitted **bare** (no wrapping paren — unlike the expression
      form's `(class …);` — and **no trailing `;`** — unlike a `function`
      declaration); the four `extends`-operand precedence cases (identifier /
      member / call heritage print bare, a conditional heritage wraps:
      `extends (a?b:c)`); the member forms (method / params+body / `static` /
      `get` / `set` / `constructor` / stacked `static get` / generator `*m` /
      `async m` / computed `[k]`, `[0]`, `[a+b]` / two members back-to-back); and
      the whole-node full shape (`class C extends B{m(){}}`). Inputs are
      hand-constructed AST; the bridge conversion of `class_declaration`
      (CLOC12.174 PR2) is exercised separately in `javascript-parser`, and
      building the AST directly lets the port cover the generator / async /
      computed-key / multi-member shapes the grammar cannot yet parse.

## Translation notes

Fourth port under CLOC12 (after `closure-pass-constant-fold` in
CLOC12.02, `closure-pass-dce` in CLOC12.04, and
`closure-pass-fold-control-flow` in CLOC12.05). First port that
targets the *emitter* rather than a transform pass — the shape of
assertions is "given AST, emit string equal to X" instead of "given
AST, fold to AST'".

- Upstream tests use `assertPrint(input_js, expected_js)` and
  `assertPrintSame(js)` which both lex/parse the input through their
  own compiler harness and pretty-print the result. Our `emit()`
  takes a typed `Program` directly — there's no parser bridge yet.
  Ports here hand-construct typed-AST inputs.
- **Coverage scope is narrow today.** Upstream `CodePrinterTest` is
  ~263 `@Test` methods covering BigInt, optional chaining,
  trailing-comma policies, spread, async/await, classes, generators,
  template literals, regex, and every operator precedence corner.
  Most of these reference Phase 2+ AST node variants we don't have
  yet (BigInt, OptionalCallExpression, TemplateLiteral, etc.).
- **Our emitter unconditionally wraps every ExpressionStatement in
  parens** — `(2 + 3);` instead of upstream's `2+3;`. That's a
  deliberate Phase 1 simplification documented in the emitter's
  crate-level docs. It means most upstream `assertPrintSame` tests
  fail today — the input form they expect to be unchanged isn't what
  our emitter produces. Each such case becomes a `#[ignore]` with
  a gap describing the divergence rather than a behaviour bug.
- Each ported test docstring records the upstream `assertPrint*`
  line being modelled so a future re-port can diff cleanly.

## Ignored tests

See `code/specs/CLOC12-gaps.md` for `gap-NNN` entries that gate
ignored ports.

## Skipped (intentionally not ported)

- Upstream `CodePrinterTest`'s **tagged template** cases
  (`` tag`${x} world` ``). We have no `TaggedTemplateExpression` AST node
  yet, so these inputs cannot be hand-constructed. They come in with a
  future tagged-template AST slice, not this port.
