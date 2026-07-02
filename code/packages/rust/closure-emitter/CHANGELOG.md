# Changelog

All notable changes to the `coding-adventures-closure-emitter` crate will be documented in this file.

## [0.21.0] - 2026-07-02

### Added — CLOC12.154: emit `TemplateLiteral` (backtick template strings)

`emit_template_literal` prints the new `Expression::TemplateLiteral` node —
interleaving the `quasis` (each from its verbatim `raw` text) with the `${…}`
expressions: `` `q0${e0}q1${e1}…qN` ``. The `${` / `}` delimiters make each
inserted expression a full-expression context, so it emits at the loosest
precedence with no wrapping. A template tags at `PREC_PRIMARY` (a primary
expression / member-call base, `` `x`.length ``), so no parent ever wraps it and
it needs no statement-start guard. 5 new unit tests. Part of the atomic
`TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0).

## [0.20.0] - 2026-07-02

### Added — CLOC12.151: emit `ArrowFunctionExpression` (the `=>` form)

`emit_arrow_function_expression` prints the new `Expression::ArrowFunctionExpression`
node. Shape rules:

- **Param parens dropped for a single plain identifier** — `x=>x`; zero
  (`()=>`) and two-or-more (`(a,b)=>`) keep them.
- **Dual body** — a block body emits like a function body (`x=>{return x}`,
  with the last statement's trailing `;` dropped in compact mode); a concise
  body emits the bare expression at `PREC_ASSIGNMENT` (`x=>x+1`).
- **Object-literal concise bodies are wrapped** — `()=>({a:1})`, so the leading
  `{` isn't read as a block. (The deeper leftmost-`{` case, e.g. `()=>({}).x`,
  is not yet wrapped — tracked in CLOC12-gaps.)
- **Precedence** — an arrow is tagged `PREC_ASSIGNMENT`, so a call callee /
  member object parent wraps it (`(()=>{})()`, `(()=>{}).x`), while an
  assignment RHS or conditional branch leaves it bare. Unlike a function
  expression it needs no statement-start wrap (`x=>x;` is a valid statement).
- **`async`** — prints an `async` prefix, with a separating space only before
  an unparenthesised identifier param (`async x=>x`, but `async()=>{}`).

12 new unit tests. Part of the atomic `ArrowFunctionExpression` enum-variant
rollout (javascript-ast 0.15.0).

### Changed — `EMIT_STACK_SIZE` 64 MiB → 128 MiB

The arrow arm widens `emit_expression_inner`'s per-frame footprint, and aarch64
(Apple-silicon CI) lays out larger frames than x86-64. Doubling the emit-worker
stack keeps a 2× cushion above the ~20 000-deep adversarial input the deep-emit
DoS regression exercises, so a modest future frame increase can't re-break it
(lazy pages → no cost for real code). Mirrors the same bump in
`closure-pass-constant-fold`'s `FOLD_STACK_SIZE`.

## [0.19.1] - 2026-07-01

### Test — CodePrinter function-expression port (#88, CLOC12.150)

New upstream conformance port `tests/upstream/code_printer_function_test.rs`
(from `CodePrinterTest.java`'s `testFunctionExpression*` / IIFE /
`function`-at-statement-start cases), now unblocked because
`Expression::FunctionExpression` is emitted end-to-end (CLOC12.149 + gap-153).
**12 active `#[test]`s, no `#[ignore]`** — pins that `emit_function_expression`
conforms to every covered shape: anonymous/named (`(function(){})` /
`(function f(){})`), params, a return body (no trailing `;` after `}`), the
three parenthesised contexts (statement-start, IIFE callee `(function(){})()`,
member object `(function(){}).x`), the un-parenthesised call-argument context
(`g(function(){})`), and the `function*` / `async function` prefixes.
Test-only; no `src/` change.

## [0.19.0] - 2026-07-01

### Added — CLOC12.149: emit `FunctionExpression`

Adds `emit_function_expression` (the expression sibling of
`emit_function_declaration`: optional name, and no trailing `;` since a
function *value* is not a declaration). Tags `FunctionExpression` below
`PREC_PRIMARY` in `expr_prec` so a call/member parent wraps it
(`(function(){})()`, `(function(){}).x`), and wraps a leading one in
`emit_expression_statement` (a statement starting with `function` would
otherwise parse as a declaration). +5 tests.

## [0.18.16] - 2026-07-01

### Test — CodePrinter object-literal port (#88, CLOC12.147)

The fifth CodePrinter port into `closure-emitter`. New file
`tests/upstream/code_printer_object_literal_test.rs` (registered as the
`upstream_code_printer_object_literal` test target) reshapes upstream
`CodePrinterTest`'s object-literal cases (`testObjectLit*` and the key-quoting
behaviors) onto our AST surface, pinning the `emit_object` /
`emit_property` / `emit_property_key` output in the default (minified) mode.

- **13 active `#[test]`s pass on the first run** (no new emitter bug): the
  empty object, single/multiple identifier-keyed properties (comma-separated,
  no interior whitespace), string-key quote-stripping (`{"abc":1}` → `{abc:1}`,
  reserved words bared, non-identifier and numeric-looking keys kept quoted,
  the `"__proto__"` exception kept quoted), numeric-literal keys, computed keys
  (`{[a]:1}`), shorthand (`{a}`), a nested object, and a string-valued
  property. Every case shows the object parenthesized at statement start
  (`({…});`).
- **No `#[ignore]` placeholders.** Getters/setters (`{get a(){}}`) and method
  shorthand (`{m(){}}`) are intentionally out of scope — their values are
  `FunctionExpression`s, which the Phase-1 emitter does not yet print and the
  AST cannot fully represent; they join when function-expression emission
  lands.

This is a **test-only** change: no `src/` file is touched, so there is no
ripple into downstream consumers. Bumps the crate 0.18.15 → 0.18.16.

## [0.18.15] - 2026-07-01

### Test — CodePrinter ASCII-only escape port (#88, CLOC12.144)

New upstream port `tests/upstream/code_printer_ascii_escape_test.rs` (registered
as the `upstream_code_printer_ascii_escape` test target), covering the
`--output_charset=US-ASCII` mode from `CodePrinterTest.testUnicode` — our
`EmitOptions { ascii_only: true }` path (`escape_ascii_only`). The sibling
`code_printer_string_escape_test.rs` pins the DEFAULT (non-ASCII passthrough)
mode; this pins the distinct ASCII-only branch of `emit_string`.

- **7 active `#[test]`s** (all pass on the first run — no new emitter bug):
  printable ASCII verbatim; a Latin-1 accent (`é` → `é`); a BMP CJK
  ideograph (`中` → `中`); an astral code point using the braced form
  (`💩` → `{1F4A9}`); a control char (`U+0007` → ``); the named
  `\n`/`\t` escapes still applying; and the `ascii_only`-always-double-quotes
  rule (a value of two `"` prints `"\"\""`, not single-quoted).

No library code changed — this release is test coverage plus docs.

## [0.18.14] - 2026-07-01

### Test — CodePrinter string-escape port (#88, CLOC12.143)

New upstream port `tests/upstream/code_printer_string_escape_test.rs` (registered
as the `upstream_code_printer_string_escape` test target), extending the
CodePrinter coverage beyond the quote-choice case already active in
`code_printer_test.rs` (gap-026). It pins the **escape sequences** `emit_string`
emits, in both the default double-quote path and the single-quote path that
quote-choice selects:

- **7 active `#[test]`s** (all pass on the first run — no new emitter bug):
  backslash doubling (`a\b` → `"a\\b"`), the `\n`/`\r`/`\t` short escapes, an
  "other" control char as upper-case `\uXXXX` (`U+0007` → ``), the
  U+2028/U+2029 line-terminator escapes, printable non-ASCII left verbatim
  (`café` stays `café` in the default non-`ascii_only` mode), and two
  single-quote-path cases (backslash still doubles; the active `'` is escaped
  while `"` stays bare).

No library code changed — this release is test coverage plus docs.

## [0.18.13] - 2026-07-01

### Test — CodePrinter number-formatting port (#88, CLOC12.142)

New upstream port `tests/upstream/code_printer_numbers_test.rs` (registered as
the `upstream_code_printer_numbers` test target), extending the CodePrinter
coverage beyond the core shortest-form cases already active in
`code_printer_test.rs`. It pins the **exponential-vs-decimal cut-over** that
`format_js_number` performs — it keeps whichever spelling is strictly shorter,
breaking ties toward decimal:

- **6 active `#[test]`s** covering the cut-over both ways: `1e18` → `1E18`,
  `1e100` → `1E100`, `2.5e10` → `2.5E10`, `123456789` stays decimal (its
  `1.23456789E8` is longer), `1e-7` → `1E-7`, `1234.5` stays decimal.
- **2 `#[ignore = "blocked on gap-133"]` placeholders** for the leading-zero
  drop upstream applies to bare fractions (`0.25` → `.25`, `0.125` → `.125`).
  Our emitter's `format_js_number` currently keeps the `0`; the placeholders
  document the intended upstream byte output. This is the *emitter* (AST →
  string) path — the separate source-preserving byte-identity path already
  elides the leading zero (gap-107 / gap-113).

No library code changed — this release is test coverage plus docs. Every active
assertion matched the emitter's current output on the first run (no new bug).

## [0.18.12] - 2026-06-30

### Test — activate stale CodePrinter conformance placeholders (#88, CLOC12.138)

Three `#[ignore]`d placeholders in `tests/upstream/code_printer_test.rs` were
left as documentation stubs when their underlying emitter features shipped
(gap-025 number shortest-form in CLOC12.12, gap-026 quote-choice in CLOC12.11,
gap-027 precedence-parens in CLOC12.10). This turns them into **active**
byte-equal conformance tests against the emitter's actual output — no
production code change:

- `test_number_formatting_shortest_form` — `1E9`/`1E6`/`1E21` exponential
  collapse, `100`→decimal tie, `0.5`, and `-0` sign preservation.
- `test_string_quote_choice_minimises_escapes` — single-quote when the value has
  more `"` than `'` (`she said "hi"` → `'she said "hi"'`), double otherwise.
- `test_operator_precedence_inserts_inner_parens` — `a*(b+c)`, `(a+b)*c`, and
  `a+b*c` (no parens where `*` already binds tighter).

Also opened **gap-133** (CLOC12-gaps.md): the number formatter keeps `0.5`
where upstream drops the leading zero to `.5` — a conservative miss (not a
miscompile), tracked for a small `format_js_number` follow-up.

Test-only + docs; crate version 0.18.11 → 0.18.12. Full emitter suite green
(96 lib + 9 code_printer port, 3 unrelated placeholders still ignored).

## [0.18.11] - 2026-06-30

### Fixed — deep operator chains no longer overflow the native stack (DoS)

`emit_binary`/`emit_logical` recurse on the left operand once per operator, so
a deeply left-nested chain — the shape the bridge builds for flat source like
`1+1+…+1` (tens of thousands of terms) — recursed once per operator and, past a
few thousand levels, overflowed the caller's ordinary ~2 MiB stack. That is an
*uncatchable* abort, and closurec feeds this emitter *untrusted* JS, so it must
not be crashable by pathological input.

`emit` now runs the recursive emission on a 64 MiB worker thread
(`EMIT_STACK_SIZE`) via `std::thread::scope` (borrows the program without
`'static`; the shallow source-map serialisation finishes on the caller thread).
Emission is **byte-identical** to before — only the stack size differs — so
every existing fixture is unchanged. Regression test builds a 20 000-deep `+`
chain and asserts it emits `1+1+…+1;` without crashing.

(Very deep ASTs can still stress *other* recursive stages — notably the AST's
own recursive `Drop` — which closurec absorbs on its 8 MiB main thread for the
inputs seen here; full pipeline-level hardening is tracked separately.)

## [0.18.10] - 2026-06-30

### Changed — drop needless parens around assignments in conditional branches

`emit_conditional` parenthesised an assignment in the consequent or alternate
of a `? :`:

```
a ? b = 1 : c = 2   →  a?(b=1):(c=2)     (now: a?b=1:c=2)
cond ? x += 1 : y   →  cond?(x+=1):y     (now: cond?x+=1:y)
```

The ECMAScript grammar is
`ConditionalExpression : ShortCircuitExpression ? AssignmentExpression : AssignmentExpression`,
so both branches are full `AssignmentExpression`s and need no parentheses —
the `?`/`:` punctuation delimits them. The consequent was being emitted at
`PREC_CONDITIONAL + 1` and the alternate at `PREC_CONDITIONAL`, both higher
than assignment precedence, so the wrapper added parens. Both branches now
emit at `PREC_ASSIGNMENT` (the precedence floor of the expression set — there
is no `SequenceExpression` in the AST), so an assignment or nested conditional
branch is never wrapped (`a?b?c:d:e` instead of `a?(b?c:d):e`).

**Soundness.** The TEST is unchanged — it is a `ShortCircuitExpression`, which
does NOT include assignment or conditional, so a test that is itself an
assignment keeps its required parens: `(a=1)?b:c` (without them, `a=1?b:c`
parses as `a=(1?b:c)`).

Tests: `conditional_branches_do_not_parenthesize_assignments`,
`conditional_test_assignment_stays_parenthesized`.

## [0.18.9] - 2026-06-30

### Added — Closure-style boolean shorthand: `true` → `!0`, `false` → `!1`

Boolean literals now minify to their negation-of-digit form, matching the
Closure Compiler: `true` (4 chars) → `!0` (2), `false` (5) → `!1` (2).
`!0 === true` and `!1 === false` in every context (`!` coerces to boolean and
negates), so the rewrite is value-exact.

**Precedence (soundness).** `!0` / `!1` are `UnaryExpression`s, which bind
*looser* than member access, call, `new`, and tagged templates — so a naive
`true.x` → `!0.x` would reparse as `!(0.x)`, a miscompile. To prevent that,
`expr_prec` now tags `BooleanLiteral` at `PREC_UNARY` (exactly as the `void 0`
`UndefinedLiteral` is already handled), so the existing precedence wrapper in
`emit_expression_inner` inserts parentheses automatically where required:

```
true.toString()  →  (!0).toString()
true()           →  (!0)()
x = true         →  x=!0           (no parens needed)
[true, false]    →  [!0,!1]
a && true        →  a&&!0
true == 1        →  !0==1          (unary binds tighter than ==)
```

Tests: `boolean_literals_minify_to_bang_zero_and_bang_one`,
`boolean_as_member_object_is_parenthesized`, `boolean_in_binary_needs_no_parens`
(and `unary_not_no_space` updated: `!true` → `!!0`).

## [0.18.8] - 2026-06-30

### Fixed — large integer literals saturated to `i64::MAX`/`MIN` (miscompile)

An integral numeric literal whose magnitude was at or above `2^63` but below
`1e21` was emitted as the wrong number:

```
console.log(12345678901234567890)  →  console.log(9223372036854775807)
a = 18446744073709551615           →  a = 9223372036854775807
(negative counterparts)            →  -9223372036854775808
```

`format_js_number` chose the integer spelling with `format!("{}", n as i64)`
guarded only by `n.abs() < 1e21`. But `n as i64` is a **saturating** cast in
Rust: any `f64` ≥ `i64::MAX` clamps to `9223372036854775807` and any ≤
`i64::MIN` clamps to `-9223372036854775808`. So every integral value in
`[2^63, 1e21)` collapsed to the same i64 bound — a different number than the
source (a real miscompile, not just a formatting nit).

**Fix.** The i64 path is now guarded by `n.abs() < 2^63`
(`9223372036854775808.0`), the exact range where `n as i64` is lossless.
Integral values at or above `2^63` fall through to `n.to_string()`, which
prints the shortest decimal that round-trips to the same `f64`; the
exponential candidate is still compared so the shorter spelling wins
(`1e20` → `1E20`). Values that fit in i64 are unaffected.

Regression tests: `number_large_integral_does_not_saturate_to_i64_bound`
(round-trip + not-saturated, both signs), `number_values_within_i64_range_keep_exact_integer_spelling`.

## [0.18.7] - 2026-06-30

### Fixed — `**` exponentiation operand precedence (invalid-JS miscompile)

`emit_binary` emitted every operator's left child at `my_prec` and right child
at `my_prec + 1` — correct for the left-associative operators, but wrong for
`**`, which is right-associative AND whose grammar base is an `UpdateExpression`
(it binds tighter than unary). Two bugs resulted:

- **Invalid output.** A unary base printed without parentheses: `(-a)**2` became
  `-a**2`, which is a `SyntaxError` in JavaScript (a unary operator may not be
  the base of `**` without parens). Same for `(~a)**2`, `(!a)**2`.
- **Over-parenthesisation.** Right-associativity was not modelled, so
  `a**b**c` (which natively *is* `a**(b**c)`) printed as `a**(b**c)`.

`**` now emits its left at `PREC_UNARY + 1` (parenthesising a unary/lower base —
`(-a)**2`, `(a**b)**c`) and its right at `my_prec` (a same-precedence right needs
no parens — `a**b**c`; a unary right is still legal bare — `a**-b`). All other
operators are unchanged. New test `exponentiation_base_and_right_precedence`.

## [0.18.6] - 2026-06-30

### Fixed — member object / call callee lost its parentheses (miscompile)

`emit_member` emitted its object, and `emit_call` its callee, via
`emit_expression` (parent precedence 0), which never parenthesises. A
lower-precedence object/callee therefore dropped the parentheses that make it a
unit, changing the parse:

| input        | was         | now (correct) |
|--------------|-------------|---------------|
| `(a\|\|b).c`   | `a\|\|b.c` (= `a\|\|(b.c)`) | `(a\|\|b).c` |
| `(a=b).c`    | `a=b.c`     | `(a=b).c`   |
| `(a?b:c).d`  | `a?b:c.d`   | `(a?b:c).d` |
| `(a+b).c`    | `a+b.c`     | `(a+b).c`   |
| `(-a).b`     | `-a.b`      | `(-a).b`    |
| `(a\|\|b)()`   | `a\|\|b()`    | `(a\|\|b)()`  |
| `(a=b)(c)`   | `a=b(c)`    | `(a=b)(c)`  |

Both now emit the object/callee at `PREC_PRIMARY`. Member and call expressions
are themselves `PREC_PRIMARY`, so `a.b.c`, `f().x`, and `a.b()` stay
parenthesis-free, while anything lower (binary, logical, unary, conditional,
assignment, sequence) is wrapped. Computed member objects (`(a\|\|b)[c]`) are
covered too. New emitter test
`member_and_call_object_below_member_precedence_is_parenthesised`.

## [0.18.5] - 2026-06-30

### Changed — tighten binary/logical operator spacing in compact mode

`emit_binary` and `emit_logical` previously wrote a space on **both** sides of
every operator unconditionally, so compact output carried needless bytes —
`a + b`, `a && b`, `a << b`, `a === b` — diverging from upstream Closure's
`a+b`. Symbolic operators are now emitted tight in compact mode (a space appears
only in `pretty` mode), matching upstream's `assertPrint("2 + 3", "2+3")`.

Two carve-outs keep this sound:

- **Word operators** `in` / `instanceof` still get a mandatory space on both
  sides — `1 in obj`, not `1inobj`.
- **Additive `+` / `-`** keep one minimal space at a seam where the operand
  would otherwise fuse the pair into the increment/decrement token: `a + (+b)`
  emits `a+ +b`, never `a++b` (which reparses as `a++ b`); likewise `a - (-b)` →
  `a- -b`. No other symbolic operator can fuse (no operand begins or ends with
  `<`,`>`,`&`,`|`,`*`,`/`,`%`,`^`,`=`, and `++`/`--` are not representable unary
  operators), so they de-space unconditionally. The right-seam check reuses the
  existing `arg_starts_with_sign` helper (which is parenthesisation-aware: a
  wrapped operand leads with `(` and so needs no guard).

New tests cover tight symbolic output, retained word-operator spaces, and the
`a+ +b` / `a- -b` / negative-literal / mixed-sign (`a+-b`) hazard matrix.

## [0.18.4] - 2026-06-30

### Fixed — trailing array hole lost its comma (miscompile)

`emit_array` wrote one separating comma *between* elements, so an array whose
last element is a hole printed one comma short: `[Some(1), None]` became `[1,]`
(length 1) instead of `[1,,]` (length 2). A trailing hole is semantically real
(`[1,,].length === 2`), so the shortened output is a miscompile. The emitter now
appends one extra comma when the last element is a hole, fixing `[1,,]`, `[,,]`
(from `[None, None]`), `[,]` (from `[None]`), etc.; arrays ending in a real
element are unchanged. Pairs with the bridge elision fix in `javascript-parser`
0.19.4. New test `array_trailing_and_leading_holes_round_trip`.

## [0.18.3] - 2026-06-29

### Fixed — U+2028 / U+2029 emitted unescaped in string literals (invalid JS)

The default (non-`ascii_only`) string escapers `escape_str_dq` / `escape_str_sq`
only escaped control characters below `0x20`, so they passed U+2028 (LINE
SEPARATOR) and U+2029 (PARAGRAPH SEPARATOR) through verbatim. Those two
codepoints are ECMAScript **line terminators**, and before ES2019 an unescaped
one inside a string literal is a `SyntaxError` — so a source string containing
either character could be minified into invalid output. (The `ascii_only`
escaper already escaped them as non-ASCII; only the default path was affected.)

Both escapers now emit ` ` / ` ` explicitly. New test
`line_and_paragraph_separators_are_escaped` covers the double-quoted,
single-quoted, and `ascii_only` paths. Flagged during the security review of the
property-key fix (0.18.2) as a pre-existing latent miscompile.

## [0.18.2] - 2026-06-29

### Added — sound quote-stripping for string object-property keys

`emit_property_key` now drops the quotes on a `PropertyKey::StringLiteral` —
`{"abc":1}` → `{abc:1}`, matching Closure's CodePrinter — but **only** when the
decoded `value` is a valid ASCII identifier name (new private `is_identifier_name`
helper) and is not `__proto__`. The two carve-outs are what make it sound:

- Non-identifier values stay quoted: `"a-b"`, `"a b"`, and `"x\ty"` would be
  syntax errors bare, and `"1"` would become a numeric key.
- `"__proto__"` stays quoted: the bare form `{__proto__: v}` is the §B.3.1
  prototype setter, a *different* object from the own property `{"__proto__": v}`.

Previously the emitter relied on the bridge to pre-decide the key node kind; the
bridge bug (see `javascript-parser` 0.19.3) meant every quoted key arrived as an
`Identifier` and was emitted bare regardless of validity. With the bridge now
producing faithful `StringLiteral` keys, this is the single place that decides
quote-vs-bare. New tests cover identifier, hyphen, space, leading-digit, tab,
reserved-word, and `__proto__` keys.

## [0.18.1] - 2026-06-29

### Fixed — negative zero (`-0`) lost its sign on emit (miscompile)

`format_js_number` short-circuited every value equal to `0.0` to the string
`"0"`. Because Rust's `-0.0 == 0.0` is `true`, negative zero took that path and
emitted `0` — but `-0` is observably distinct from `0` in JavaScript
(`1 / -0 === -Infinity` vs `1 / 0 === Infinity`; `Object.is(-0, 0) === false`),
so dropping the sign is a **miscompile**. It surfaced through the constant-fold
pass: `f(-0)`, `f(-(5-5))`, and `f(0 * -1)` all fold the argument to the
negative-zero numeric literal `-0.0`, which the emitter then printed as `0`.

The zero fast path now checks `is_sign_negative()` and emits `-0` for negative
zero, `0` otherwise. `-0` is also the minimal correct representation. Positive
zero and every other value are byte-identical to before — the blast radius is
exactly negative zero. End-to-end: `f(-0);` → `f(-0);` (was `f(0);`), and
`g(1/-0);` → `g(-Infinity);`.

## [0.18.0] - 2026-06-21

### Fixed — prefix-unary precedence and `--`/`++` token fusion

`emit_unary` emitted its argument with `emit_expression` (parent precedence 0),
so a lower-precedence operand was never parenthesised: `!(a == b)` printed as
`!a == b`, which JS reparses as `(!a) == b` — a **different program**. The
argument is now emitted at `PREC_UNARY`, so binary / logical / conditional /
assignment operands are wrapped:

| AST            | before    | after       |
|----------------|-----------|-------------|
| `!(a == b)`    | `!a == b` | `!(a == b)` |
| `-(a + b)`     | `-a + b`  | `-(a + b)`  |
| `~(a \| b)`    | `~a \| b` | `~(a \| b)` |
| `!(a ? b : c)` | `!a?b:c`  | `!(a?b:c)`  |

A second fusion hazard is now handled too: a sign operator (`-`/`+`) directly
followed by a same-sign operand fused into the decrement/increment token —
`-(-a)` printed `--a` (pre-decrement of `a`), `+(+a)` printed `++a`. `emit_unary`
now inserts a separating space in exactly those cases (`- -a`, `+ +a`, `- -5`),
via the new `sign_op_char` / `arg_starts_with_sign` helpers. `!`/`~` never fuse,
and equal-precedence nests like `!!a` are left paren-free.

These miscompiles were latent until the `javascript-parser` bridge stopped
dropping prefix operators (it had been emitting the bare operand); this change
ships alongside that fix. Added 10 emitter unit tests.

## [0.17.0] - 2026-06-20

### Added — CLOC23: emit `for (… of …)`

New `emit_for_of` writes `for ( <left> of <right> ) <body>` — identical to
`emit_for_in` but with the `of` keyword, spaced on both sides for the same
token-separation reason (the left ends in an identifier and the right starts
with one). Added emitter unit tests for the `var` declaration-left and
expression-left forms.

## [0.16.0] - 2026-06-20

### Added — CLOC22: emit `for (… in …)`

New `emit_for_in` writes `for ( <left> in <right> ) <body>`. The `in` keyword is
separated on both sides with `required_ws` (a single space): the left ends in an
identifier (`var k` / `k` / `o.p`) and the right starts with one, so `kin` /
`inobj` would otherwise mis-lex. In the rare `a[b] in` / `in (x)` cases the space
is one redundant byte but never wrong, matching upstream Closure's spacing around
`in`. Added emitter unit tests for the `var`/`const` declaration left and the
expression-left (bare-body) forms.

## [0.15.0] - 2026-06-20

### Added — CLOC21: emit `debugger;`

New `emit_debugger` writes `debugger;`. The keyword is followed only by its
terminator `;` (or, after the semi is popped, a `}`/EOF), so no token-separation
handling is needed. `DebuggerStatement` is added to
`last_stmt_uses_terminator_semi` — its `;` is popped before a closing `}` (e.g.
`{debugger}`), like `return`/`throw`/expression statements. Added emitter unit
tests for the keyword+semi, terminator-pop, and not-last (semi kept) cases.

## [0.14.0] - 2026-06-20

### Added — CLOC20: emit `do`/`while`

New `emit_do_while` writes `do <body> while ( <test> ) ;`. Token-separation: a
required space is inserted after `do` only when the body is NOT a block (so
`do{…}` stays tight but `do foo();` does not glue into `dofoo()`). The trailing
`;` is a real statement terminator, so `DoWhileStatement` is added to
`last_stmt_uses_terminator_semi` — its `;` is popped before a closing `}` (e.g.
`{do{a}while(b)}`) just like `return`/`throw`/expression statements, but unlike
plain `while` (whose trailing `;` is a body slot). Added emitter unit tests for
the tight, bare-body, pretty-mode, and terminator-pop cases.

## [0.13.0] - 2026-06-20

### Added — CLOC19: emit `try`/`catch`/`finally`

New `emit_try` writes `try <block> [ catch [(param)] <block> ] [ finally <block> ]`.
No `required_ws` is needed anywhere: every boundary is keyword↔`{`/`}` or
`}`↔keyword (`try{…}catch{…}`, `}finally{…}`), which lex cleanly with no
separator. Pretty mode inserts readability spaces (`try {`, `catch (e) {`,
`finally {`); minified mode emits the tight form. The optional-catch-binding
form (`catch { … }`) emits with no parens. Added emitter unit tests pinning the
minified token boundaries, the optional-binding and no-catch forms, and the
pretty-mode spacing.

## [0.12.0] - 2026-06-08

### Changed
- **gap-030 part 1 (AST emitter side):** Function-declaration
  ASI policy now matches upstream Closure v20240317 in compact
  (non-pretty) mode.
  - `emit_block_statement` drops a single trailing `;` before
    `}` via a new `pop_trailing_semi_if_compact()` helper —
    BUT only when the block's last child is a leaf
    statement-terminator type (gated by
    `last_stmt_uses_terminator_semi`). Per ECMAScript §11.9
    (Automatic Semicolon Insertion), the `}` terminates the
    in-progress statement, so a true terminator `;` is
    redundant noise that upstream Closure doesn't emit.
  - **Critical correctness gate:** when the block's last child
    is a compound statement (If/While/For/Labeled) whose body
    is an `EmptyStatement`, the trailing `;` is structurally
    part of the body grammar — NOT a terminator. Popping it
    would produce SyntaxError-emitting output like
    `function f(){if(x)}` or `function f(){for(;;)}`. The
    `last_stmt_uses_terminator_semi` helper enumerates the
    safe-to-pop set (ExpressionStatement / ReturnStatement /
    BreakStatement / ContinueStatement / ThrowStatement); all
    other types fall through to the conservative "preserve
    `;`" path. Caught by pre-push security review.
  - `emit_function_declaration` emits a `;` after the body's
    closing `}` in compact mode. Even at EOF this is a no-op
    `EmptyStatement`, but in concatenation contexts it keeps
    the function-declaration's output shape predictable.
- **Pretty mode is unchanged.** Visual delimiter clarity
  outranks byte minimization in the human-facing pretty mode;
  both gap-030 changes are compact-only.
- The pre-existing `function_declaration_minified` test
  assertion was updated from `function f(x){return x;}` to
  `function f(x){return x};` to track the new compact-mode
  shape.

### Added
- Five new inline tests:
  - `gap030_block_multi_stmts_drops_only_last_semi`: multi-
    statement blocks drop only the final `;` (intermediate
    `;`s between statements are preserved).
  - `gap030_pretty_mode_unchanged`: pretty mode still emits
    inner `;` and no trailing `;` after `}`.
  - `gap030_empty_function_body_compact`: empty body stays
    `{}` with trailing `;` (`function noop(){};`).
  - `gap030_does_not_pop_empty_body_of_if`: regression for the
    security-review-caught bug — `function f(){if(x);}` must
    not collapse to `function f(){if(x)}` (SyntaxError).
  - `gap030_does_not_pop_empty_body_of_while`: same defense
    applied to `while(x);` body shape.

### Known limitation
- **closurec's CLI WHITESPACE_ONLY path uses a separate
  token-level re-stitcher** (`whitespace_only.rs`), not this
  emitter. So gap-030's `minify_function_decl` byte-identity
  fixture stays IGNORED for now; flipping it to PASS requires
  porting the same two rules to the token re-stitcher in a
  follow-up PR (gap-030 spec entry tracks both parts).

## [0.11.0] - 2026-06-04

### Added — CLOC12.33: `SwitchStatement` emitter rule (gap-014)

Adds `emit_switch` + `emit_switch_case` covering the new
`SwitchStatement` / `SwitchCase` variants shipped in
`javascript-ast` 0.7.0.

Output shape (compact mode):

```
switch(<discriminant>){case <test>:<consequent>...case <test>:<consequent>...default:<consequent>...}
```

- `case <test>:` is emitted with a single space between `case`
  and the test expression (required to avoid `case1:` ambiguity).
- `default:` has no space (no expression follows the colon).
- Empty consequent emits just `case <test>:` / `default:` with
  nothing after the colon.
- Multiple cases concatenate in source order. The trailing `}`
  closes the switch directly — no separating semicolons.

Pretty mode (`opts.pretty = true`) lays each case on its own
indented line and each consequent statement on its own
double-indented line under the case header. Same trailing-comma
discipline as block: no separator before the closing brace.

Six new inline tests pin: empty switch, single `case 1:` with
expression-statement body, `default:` with body, `case 1:` with
empty body, full `case/case/default` triple in order, and
`case 1: break;` (break-in-consequent invariant).

## [Unreleased] — CLOC12.32: trailing-comma ports (gap-022)

Test-only. Closes `gap-022`.

* New port file `tests/upstream/code_printer_trailing_comma_test.rs` — 16 hand-built tests pinning that the emitter never emits a trailing comma before `]` or `}`, covering empty / single / multi / nested arrays and objects in **both** compact and pretty modes, plus the elision-isn't-a-trailing-comma edge case.
* Re-annotated the old gap-022 placeholder in `tests/upstream/code_printer_test.rs::test_trailing_comma_in_array_and_object_with_pretty_print` to route to the new home.
* Added the `[[test]]` entry in `Cargo.toml` (Cargo only auto-discovers `tests/*.rs` one level deep).

The original gap entry asked for a `trailing_comma: bool` AST flag and an emitter rule. Reviewing the upstream test family showed the flag is unnecessary: upstream's `assertPrettyPrint("var x = [1,];", "var x = [1];\n")` relies on parse-side trailing-comma stripping (it's purely syntactic in ES2017, not an elision) and emitter-side never-write. Our AST already collapses `[1,]` to `[Some(1)]` (identical to `[1]`) at parse time, and the emitter loop only writes `,` between elements. Spec gap-022 → RESOLVED with that note.

No source-incompatible change. No production code touched.

## [0.10.0] - 2026-06-01

### Added — CLOC12.16: emit `UndefinedLiteral` as `void 0` (gap-001)

Adds emitter support for the new `UndefinedLiteral` variant.
Renders as `void 0` — six characters of shadow-safe goodness.

**Why `void 0` and not the keyword `undefined`?** In ECMAScript
`undefined` is an *identifier*, not a reserved word. Code can
legally do `var undefined = 1;` in non-strict mode (or just declare
a local `undefined` parameter) and that binding shadows the global.
Reading the identifier `undefined` from inside such a scope would
yield the shadow value, not the genuine undefined.

`void <expression>` always evaluates `<expression>` and then
produces the **real** undefined value, regardless of any name in
scope. `void 0` is the shortest spelling and matches upstream
Closure Compiler's `CodePrinter` behaviour.

**Precedence wiring.** `UndefinedLiteral` is mapped to `PREC_UNARY`
(not `PREC_PRIMARY` like other literals) so contexts like
`(void 0).x` and `(void 0)()` automatically get the parens they
need. Without that, the emit would produce `void 0.x` — which
JS parses as `void (0.x)`, a semantically different expression.

Two inline tests cover the bare `void 0;` output for traced and
untraced cases.

## [0.9.0] - 2026-06-01

### Added — CLOC12.15: emit `BigIntLiteral` (gap-021)

Adds emitter support for the new `BigIntLiteral` variant added to
`javascript-ast` in CLOC12.15. The emitter writes the `raw` field
verbatim — we deliberately do NOT reformat from `value` because:

1. Hex/octal/binary radixes (`0x1fn`, `0o17n`, `0b11111n`) are part
   of the literal's source identity and shorter than their decimal
   equivalents.
2. There is no exponential bigint syntax (`1e9n` is not valid JS),
   so the number-formatter's shortest-form trick from CLOC12.12
   doesn't apply.

Therefore: `raw` in, same string out. The PREC_PRIMARY table also
lists `BigIntLiteral` so emit_expression_inner never inserts
unnecessary parens around bigint literals in nested contexts.

Three new inline tests cover decimal (`123n`), zero (`0n`), and
hex (`0x1fn`) cases.

## [0.8.0] - 2026-06-01

### Added — CLOC12.14: emit `ThrowStatement` (gap-020 AST partial close)

Adds emitter support for the new `ThrowStatement` variant added to
`javascript-ast` in CLOC12.14. Compact form is `throw <expr>;` with
a mandatory single space between the keyword and the expression
(without it `throw1` parses as an identifier in lenient lexers and
is ambiguous in strict ones); the trailing `;` is always emitted.

Tests cover `throw 1;` (NumericLiteral), `throw e;` (Identifier),
and `throw "oops";` (StringLiteral — confirms the quote-choice path
runs the same way it does for any other string expression context).

## [0.7.0] - 2026-06-01

### Added — CLOC12.13: emit `LabeledStatement` (gap-009 AST partial close)

Adds emitter support for the new `LabeledStatement` variant added to
`javascript-ast` in CLOC12.13. Compact form is `label:body` with
no whitespace; pretty form is `label: body` (single space between
the colon and the body). The body's own emitter writes its trailing
`;`, so we never double-print.

Tests cover `a: foo();`, `break;`, `break a;`, and the literal
upstream-test input `a: break a;`. The `BreakStatement` emit path
was already in place from the original 0.1.0 scaffold — these tests
pin its current behaviour now that there's a label node to combine
with.

## [0.6.0] - 2026-06-01

### Added — CLOC12.12: number formatting shortest-form (closes gap-025)

`format_js_number` now computes both decimal and exponential
representations for finite non-zero numbers and returns whichever is
shorter. Ties pick decimal (canonical). Matches upstream
`CodePrinter`'s behaviour.

| Value | Old emit | New emit |
|-------|----------|----------|
| `1` | `1` | `1` |
| `100` | `100` | `100` (tie 3=3 → decimal) |
| `1_000_000_000` | `1000000000` | `1E9` |
| `5_000_000` | `5000000` | `5E6` |
| `0.5` | `0.5` | `0.5` |
| `1.5e-10` | `0.00000000015` | `1.5E-10` |
| `NaN` | `NaN` | `NaN` |
| `Infinity` | `Infinity` | `Infinity` |

Exponential form follows JS / upstream conventions: uppercase `E`,
no leading `+` for positive exponents, stripped trailing zeros in
the mantissa (`1E9`, not `1.0E+9`).

### New helper

`format_exponential_uppercase(n: f64) -> String` — wraps Rust's
`{:e}` formatter and uppercases the `E`.

### New inline tests (5)

- `number_shortest_form_small_integers_stay_decimal` — `0`, `1`, `42`, `100`, `-7`.
- `number_shortest_form_big_integers_switch_to_exponential` — `1E9`, `5E6`.
- `number_shortest_form_small_decimals_stay_decimal` — `0.5`, `3.14`.
- `number_shortest_form_tiny_floats_switch_to_exponential` — `1.5E-10`.
- `number_shortest_form_nan_and_infinity_unchanged` — sanity check.

Plus `emit_number_value(v: f64) -> String` helper.

### gap-025 → RESOLVED

### Reconciles missing version bump from CLOC12.11

CLOC12.11 (PR #4703) updated the CHANGELOG to `[0.5.0]` but the
`Cargo.toml` change was dropped, leaving the published crate at
`0.4.0`. This PR bumps directly `0.4.0` → `0.6.0`: the `0.5.0`
CHANGELOG entry below stays valid as the description of quote-choice
work; `0.6.0` is the first published version that actually includes
both quote-choice (CLOC12.11) AND shortest-form number rendering
(CLOC12.12).

### Version

`0.4.0` → `0.6.0` (skips `0.5.0` to absorb the missed CLOC12.11
Cargo.toml bump).

## [0.5.0] - 2026-06-01

### Added — CLOC12.11: string quote-choice optimisation (closes gap-026)

`emit_string` now picks the quote style that minimises required
escape characters. Upstream's CodePrinter does the same; matching it
brings us a step closer to byte-identical output.

**Algorithm** — count occurrences of `"` and `'` in the value. If
`"` strictly outnumbers `'`, emit with single quotes (each saved
`\"` is shorter); otherwise emit with double quotes (canonical form,
ties broken toward double).

```
value                            chosen quote   why
-----------------------------    ------------   -------------------
hello                            double         no quotes anywhere
o'malley                         double         no `"`; cheaper as `"o'malley"`
she said "hi"                    single         `"` saves one escape
"mixed 'both'"                   double         tie (2 each) → double
""x                              single         two `"`, zero `'`
```

`ascii_only` mode still always uses double quotes — switching mid-mode
would confuse downstream readers and upstream itself maintains that
invariant.

### New helpers in `lib.rs`

- `choose_quote_and_escape(value: &str) -> (&'static str, String)` —
  returns the chosen quote character plus the escaped body.
- `escape_str_sq(s: &str) -> String` — single-quoted variant of
  `escape_str_dq`. Identical control-char rules; differs only in
  which quote it escapes.

### New inline tests (6)

- `quote_choice_no_quotes_uses_double` — `"hello"`, `""`.
- `quote_choice_single_quotes_in_value_uses_double` — `"o'malley"`, `"it's"`.
- `quote_choice_double_quotes_in_value_switches_to_single` — `'she said "hi"'`.
- `quote_choice_tie_picks_double` — value `'"`, leading byte = `"`.
- `quote_choice_more_double_than_single_picks_single` — value `""x`, leading byte = `'`.
- (helper) `emit_string_value(value: &str) -> String` — emit a
  synthetic StringLiteral and return the code; used by the four
  parametric assertions.

### Side effect

The previous emit_string path used `s.raw` verbatim when present
(preserving the source-file's quote style). That's no longer used —
quote-choice now applies uniformly. The `raw` field is still
preserved in the AST for tooling but isn't consulted by emit.

### gap-026 → RESOLVED

The `test_string_quote_choice_minimises_escapes` placeholder in
`tests/upstream/code_printer_test.rs` stays `#[ignore]`-d pending a
follow-up that re-ports it with real upstream `assertPrint` cases
now that the underlying emitter behaviour is in place.

### Version bump

`0.4.0` → `0.5.0`.

## [0.4.0] - 2026-06-01

### Added — CLOC12.10: precedence-aware paren insertion (closes gap-024 + gap-027)

Replaces the previous "wrap every expression-statement body in parens"
policy with a precedence-aware emit. `emit_expression_inner(e, parent_prec)`
inspects the expression's own precedence and wraps in parens **only**
when the child binds more loosely than its parent context demands.

Precedence ladder (low → high, per ESTree §13):

```
 0   top level / statement / control-test position
 1   assignment
 2   conditional `? :`
 3   logical-or `||` / nullish `??`
 4   logical-and `&&`
 5-7 bitwise or/xor/and
 8   equality
 9   relational
10   shift
11   additive
12   multiplicative
13   exponent          right-assoc
14   prefix unary
17-18 call/member/primary       atomic — never wraps
```

Three new helper functions in `lib.rs`:

- `binary_prec(BinaryOperator) -> u8`
- `logical_prec(LogicalOperator) -> u8`
- `expr_prec(&Expression) -> u8`

`emit_binary` / `emit_logical` / `emit_conditional` no longer wrap
themselves in parens. They delegate to `emit_expression_inner` for
their children with appropriate `parent_prec` values
(`my_prec` for the left side, `my_prec + 1` for the right side of
left-associative operators).

### Truth table

| Source AST                     | Old emit               | New emit          |
|--------------------------------|------------------------|-------------------|
| `2 + 3`                        | `(2 + 3);`             | `2 + 3;`          |
| `"a" + "b"`                    | `("a" + "b");`         | `"a" + "b";`      |
| `(a + b) * c`                  | `((a + b) * c);`       | `(a + b) * c;`    |
| `a + b * c`                    | `(a + (b * c));`       | `a + b * c;`      |
| `!x`                           | `!x;` (unchanged)      | `!x;`             |
| `({a:1})`                      | `({a:1});` (unchanged) | `({a:1});`        |

### gap-024 → RESOLVED

The two `_is_current_behaviour` ports in
`tests/upstream/code_printer_test.rs` are renamed and their assertions
flipped to upstream's byte-equivalent forms:

| Old name (pinned divergence) | New name (matches upstream) | Was | Now |
|-------------------------------|------------------------------|-----|-----|
| `test_binary_addition_with_parens_is_current_behaviour` | `test_binary_addition_emits_without_outer_parens` | `(2 + 3);` | `2 + 3;` |
| `test_string_concat_with_parens_is_current_behaviour` | `test_string_concat_emits_without_outer_parens` | `("a" + "b");` | `"a" + "b";` |

The remaining whitespace difference between our `2 + 3;` (pretty-printed)
and upstream's `2+3;` (minified) is addressed by the pretty/minify
toggle work, not gap-024.

### gap-027 → RESOLVED (incidental)

The precedence ladder also closes gap-027 (precedence-aware paren
insertion) — they were two views of the same underlying problem. The
`test_operator_precedence_inserts_inner_parens` placeholder stays
`#[ignore]`-d pending a follow-up that adds the actual upstream
`a*(b+c)` test cases now that the emitter supports them.

### Updated inline tests

Three inline tests in `lib.rs`:

- `binary_addition_with_parens` renamed to `binary_addition_emits_without_outer_parens`, assertion flipped to `"2 + 3;"`.
- `string_concat_with_parens` renamed to `string_concat_emits_without_outer_parens`, assertion flipped to `"\"foo\" + \"bar\";"`.
- `untraced_program_still_emits` assertion flipped to `"2 + 3;"`.

### Version bump

`0.3.0` → `0.4.0`.

## [0.3.0] - 2026-05-31

### Added — CLOC12.07: port subset of upstream `CodePrinterTest`

Fourth port under CLOC12, first one targeting the emitter rather than
a transform pass.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/code_printer_test.rs` — 12 ported test methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.07 | **6** | **6** |

**Passing (6):** literal-position emits and bare unary that match our
current emitter output exactly:

- `test_binary_addition_with_parens_is_current_behaviour` — `2 + 3` emits as `(2 + 3);` (paren-wrapped; pins current behaviour).
- `test_string_concat_with_parens_is_current_behaviour` — `"a" + "b"` emits as `("a" + "b");`.
- `test_unary_not_emits_without_space` — `!x` emits as `!x;`.
- `test_boolean_literal_at_statement_position` — `true;` / `false;`.
- `test_integer_literals_at_statement_position` — `0;`, `42;`, `1;`.
- `test_string_literal_at_statement_position` — `"hello";`, `"a";`.

Two of those (`test_*_is_current_behaviour`) deliberately pin our
*current* paren-wrapping behaviour even though it diverges from
upstream — they serve as regression markers so when gap-024 is closed
and the wrapping comes off, the assertions can flip at the same time.

**Ignored (6):** record upstream's broader scope:

| Test | Gap | Blocker |
|------|-----|---------|
| `test_big_int` | gap-021 | `BigIntLiteral` not in Phase 1 AST |
| `test_trailing_comma_in_array_and_object_with_pretty_print` | gap-022 | array/object trailing-comma policy not modelled |
| `test_no_trailing_comma_in_empty_array_literal` | gap-023 | VariableDeclaration round-trip ports deferred |
| `test_number_formatting_shortest_form` | gap-025 | numeric exponential-form / shortest-form not implemented |
| `test_string_quote_choice_minimises_escapes` | gap-026 | quote-choice optimisation not implemented |
| `test_operator_precedence_inserts_inner_parens` | gap-027 | precedence-aware paren insertion not implemented |

Plus the meta-divergence:

- gap-024 — `ExpressionStatement` paren-wrapping is unconditional in our emitter, whereas upstream only wraps when ambiguity demands it. Not strictly "blocked" (we choose to wrap), but tracked so the eventual byte-identical match can flip the two `_is_current_behaviour` ports.

### Why the bulk of upstream is ignored

`CodePrinterTest` has 263 `@Test` methods. Most cover Phase 2+ AST
nodes (BigInt, optional chaining, template literals, classes,
spread, async/await, regex) or formatting policies (quote choice,
exponential-form numerics, precedence-aware parens) that aren't in
our emitter's v0.2.0 body. Each future emitter slice can re-port
the relevant subset and convert ignored markers into asserts.

### Version bump

`0.2.0` → `0.3.0`.

## [0.2.0] - 2026-05-24

### Added — real `emit` body (first real pipeline output)

Replaces v0.1.0's identity emit with a recursive printer that walks every Phase 1 AST node and produces JavaScript text. Step 3 of 4 in the autonomous-chain real-body rollout (after constant-fold + fold-control-flow; before DCE).

- Walks every Phase 1 variant: all expressions (Identifier, literals, Binary/Logical/Unary/Assignment/Conditional/Call/Member, Array with elisions, Object with shorthand/method), all statements (Expression, Block, If, While, For, Return, Break, Continue, Empty), and Declarations (Variable/Function).
- Honors all three `EmitOptions`:
  - `pretty: false` (default) → minified single-line.
  - `pretty: true` → 2-space-indented multi-line for block bodies.
  - `ascii_only: true` → escape non-ASCII as `\uXXXX` / `\u{XXXXXX}`.
  - `source_map: true` (default) → accumulate `(line, col, cv_id)` mappings via `SourceMapBuilder`, serialize as v3 JSON in `EmitOutput.source_map`.
- Tracks line/col cursor (UTF-16 code units per source-map v3 spec).

### Always-parenthesize policy in v1

v1 always parenthesizes `BinaryExpression`, `LogicalExpression`, `ConditionalExpression`, and `AssignmentExpression`. Precedence-aware elision is Phase 1.x. `ObjectExpression` at statement position is also wrapped (else `{}` parses as a block).

### CV tracing — both modes per CLOC09

- **Traced** (`cv: Some` on nodes) → `add_mapping` called per token.
- **Untraced** (`cv: None`) → no mappings recorded; output text identical; `source_map` field still contains a valid empty-mappings v3 blob when enabled.

### Headline test — end-to-end pipeline

```rust
let prog = AST(2 + 3);
let pipeline_out = PassPipeline::new()
    .add(ConstantFoldPass::new())
    .run(prog, &sidecar, &mut cv);
let emit_out = emit(&pipeline_out.program, ...);
assert_eq!(emit_out.code, "5;");
```

The full stack — AST → optimization → emit → text — works end-to-end for the first time.

### Tests

17 tests (up from 9 in v0.1.0): defaults + empty, basic expressions with always-paren, typeof spacing, const/function declarations (minified and pretty), `[1,,3]` array with elision, `({a:1,b:2});` ObjectExpression paren-wrap at statement start, `ascii_only` escapes Unicode (verified with "café"), `source_map` on/off, untraced still emits, **end-to-end pipeline produces `5;` from `2 + 3`**, `EmitError` `std::error::Error` compat.

### Dependencies
- Added `coding-adventures-closure-source-map` as a runtime dep.
- Added `coding-adventures-closure-pass-constant-fold` + `coding-adventures-closure-pass-pipeline` as dev-deps for the end-to-end test.

### Skipped (Phase 1.x / Phase 2+)
- Precedence-aware paren elision.
- Real source-map VLQ encoding (lives in `closure-source-map` v2; mappings accumulate now, final string is still empty).
- `FunctionExpression`, `ArrowFunctionExpression`, `ClassDeclaration` — Phase 2/3.
- JSDoc comment preservation.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC07 emit-and-source-map spec — the back end of the Closure Compiler clone. Takes a finalized `Program` + sidecar and produces output JavaScript text + companion source-map blob.
- `emit(program: &Program, sidecar: &Sidecar, cv: &mut CVLog, opts: &EmitOptions) -> Result<EmitOutput, EmitError>` — the canonical entry point. Signature pinned.
- `EmitOptions` struct with three knobs:
  - `ascii_only: bool` (default `false`) — when `true`, escape non-ASCII codepoints to `\uXXXX` / `\u{XXXXXX}`.
  - `pretty: bool` (default `false`) — production default is minified; switch on for human-reviewed output.
  - `source_map: bool` (default `true`) — production default is to emit a companion `.js.map`.
- `EmitOutput` struct:
  - `code: String` — JavaScript bytes (UTF-8 or ASCII-restricted).
  - `source_map: Option<String>` — source-map v3 blob; `None` when `source_map = false`.
  - `contributions: Vec<Contribution>` — per-token "emitted" CV trail per CLOC03.
- `EmitError` enum (`#[non_exhaustive]`) with `Display` + `std::error::Error` impls:
  - `UnknownCvId { id, site }` — AST referenced a CV id the log doesn't know.
  - `UnsupportedSidecarType { id, kind }` — sidecar held a type the emitter can't render.
- v1 body: emits empty `code`, an empty source-map placeholder when `source_map = true`, no contributions. `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to render. The real AST walk lands once the AST grows `Statement` / `Expression` / `Declaration` variants.
- 9 tests covering: `EmitOptions::default()` values, identity emit on empty program with default opts (code empty, source_map present-but-empty, contributions empty), `source_map = false` drops the source-map field entirely, `ascii_only` flag accepted (output trivially ASCII when empty), `pretty` flag accepted, `EmitOptions` `Clone` + `PartialEq`, `EmitError::Display` formats for both variants include the id/site/kind they carry, `EmitError` implements `std::error::Error`.

### Notes
- Dependencies: `coding-adventures-javascript-ast` (`Program`), `coding-adventures-type-sidecar` (`Sidecar` for future emit hints), `coding_adventures_correlation_vector` (`CVLog`, `Contribution`), `serde` + `serde_json` (for future source-map serialization and `Contribution.meta`). Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`.
- The emitter does **not** depend on `closure-pass-pipeline` or any pass crate. It runs after the pipeline and only consumes the final `Program` shape — keeping that decoupling means future passes can be added without touching the emit dependency graph.
- v1 is scaffolding. The function signature, options struct, output struct, and error enum are the deliverable that the future `closurec` CLI (CLOC08) and the source-map generator (`closure-source-map`, CLOC07 Phase 2) link against. The body fills in once the AST grows variants.
