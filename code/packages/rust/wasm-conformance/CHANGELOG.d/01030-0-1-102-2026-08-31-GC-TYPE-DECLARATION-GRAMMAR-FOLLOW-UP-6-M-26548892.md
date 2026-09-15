## 0.1.102 — 2026-08-31 — GC type-declaration grammar follow-up: 6 more files vendored, honestly

Re-investigates the 9 files a prior session's `fetch_testsuite.py` comment
named as "genuinely blocked" (`array.wast`, `array_new_data.wast`,
`array_new_elem.wast`, `struct.wast`, `ref_null.wast`, `type-rec.wast`,
`type-subtyping.wast`, `extern.wast`, `instance.wast`) fresh against the
real fetched files (pinned SHA `28864811cf03bdbf880733786148feaba339582d`)
rather than trusting that categorization. Six turn out tractable; three
stay genuinely blocked for real, documented reasons.

### Vendored (6 files, zero real `fail`)

- `array.wast`, `array_new_data.wast`, `array_new_elem.wast`, `struct.wast`
  — the real blocker was never GC type-declaration grammar (that gap is
  still real — see "Still blocked" below — but it wasn't what was
  stopping these four files from parsing at all). It was three missing
  `assert_return` wildcard expectations, `(ref.array)`/`(ref.struct)`/
  `(ref.eq)`, which raised a hard parse error that aborted the WHOLE
  script rather than just that one directive. See `wasm-wast-parser`'s own
  CHANGELOG (`Expected::RefArrayAny`/`RefStructAny`/`RefEqAny`) for the
  fix. Their actual array/struct INSTRUCTION directives (bodies using
  `array.new`/`struct.get`/etc., and the non-null `(ref $t)` field/param
  types the real GC grammar uses pervasively) still correctly grade
  `not_yet_supported` — that wall is real and out of scope here.
- `instance.wast` — needed the `(module definition ...)`/`(module
  instance ...)` generative-instantiation script-directive forms
  (`wasm-wast-parser`'s new `Directive::ModuleDefinition`/
  `ModuleInstance`, see that crate's own CHANGELOG). `Executor` gained a
  `definitions: HashMap<String, WasmModule>` template store (a
  "definition" is validated but deliberately NOT instantiated) and an
  `instantiate_and_register` helper shared with `Directive::Module`'s own
  success path — instantiating the SAME definition twice naturally gives
  two independent live instances, since each `instantiate()` call already
  builds fresh global/table/memory state from the `WasmModule` template.
  `register` 3/3 (100%); `assert_return` is 100% `not_yet_supported`,
  cascading from an unrelated, separate, honest gap this file also happens
  to exercise (`(elem declare func $f)` declarative element segments,
  which `wasm-wast-parser` doesn't parse at all yet).
- `type-rec.wast` — parses and grades honestly with the `Register` fix
  below; real recursive-type-group (`(rec (type $a ...) ...)`) semantics
  are still out of scope. `assert_invalid` 9/10, `assert_unlinkable` 2/2.

### Fixed — `Register`'s capability-gap tracking generalized beyond "the current module"

`Directive::Register`'s "target not found" fallback only checked
capability-gap tracking (`current_module_status`) for the `None` ("current
module") registry key, never an explicit `$id` key — so `(register "I1"
$I1)` naming an `$id` that never built for a genuine capability-gap reason
(`instance.wast`'s own case above, and `type-rec.wast`'s `(register "M"
$M)` where `$M` used a `(rec ...)` type group this crate can't build yet)
graded a hard `Fail` that looked like a real script/harness bug instead of
the honest `NotYetSupported` capability gap it actually is. Generalized
`current_module_status` into `unavailable_reasons: HashMap<Option<String>,
String>`, keyed by ANY registry key. This fix alone turned
ALREADY-VENDORED `linking.wast`'s pre-existing `register` 2 `fail` into 0
`fail`/3 `not_yet_supported` — a genuine correctness improvement to a file
already in the corpus, confirmed via a full before/after baseline diff (no
other already-vendored file's tally changed from this fix).

### Fixed — anonymous `module definition` no longer silently mis-parsed as a plain module

Before `(module definition ...)`/`(module instance ...)` were recognized
as their own `wasm-wast-parser` directive shapes, the bare `definition`/
`instance`/`$name` atoms fell through to the ordinary `(module ...)` path
and were silently treated as harmless unrecognized module fields (every
module-field loop only matches specific `SExpr::List` keyword forms),
quietly building a WRONG anonymous-or-empty module instead of erroring.
This was invisible until now because nothing previously vendored used
`module definition`/`module instance` — but two ALREADY-VENDORED files,
`memory64.wast` and `table64.wast`, each contain an anonymous `(module
definition (memory/table ...))` at a boundary-case size (e.g. exactly the
max page count), specifically so an implementation can validate it
WITHOUT actually allocating it. Before this fix, that `definition` keyword
was silently ignored and the module was built AND INSTANTIATED as an
ordinary anonymous module — which is why each file had one `module`
`trap` (a real runtime trap from instantiating/executing against a huge
i64 memory/table). Recognizing `module definition` explicitly fixes this:
both files' one `trap` is now a `pass`, confirmed via the same full
before/after baseline diff (`memory.wast`/`table.wast`, which have the
same anonymous-`module definition` shape but at a size that never actually
trapped, are unaffected either way — no file's `module` PASS count went
down anywhere).

### Still blocked (investigated fully, not just re-asserted)

- `ref_null.wast` — its second module fails REAL structural validation
  (`TypeMismatch: expected ConcreteFuncRef(0), found Funcref`):
  `nullfuncref` is represented as the plain abstract `ValueType::Funcref`
  (a deliberate, documented simplification elsewhere in this codebase,
  since `WasmValue::Ref` carries no runtime type tag), which is unsound to
  also accept wherever a concrete `ConcreteFuncRef` ref type is declared —
  `wasm_validator::type_check::is_assignable`'s own doc comment names the
  OPPOSITE direction (concrete-to-abstract) as the one deliberate
  exception, and widening it the other way would silently make
  `return_call.wast`'s/`return_call_indirect.wast`'s own mirror-image
  `assert_invalid` "Result subtyping" case wrongly valid. Needs genuine
  BOTTOM reference types (`nullfuncref`/`nullexternref`/`nullexnref`/
  `nullref` as their own `ValueType` variants, each a subtype of every
  compatible ref type) — see this crate's own PR description for a fuller
  scoping note toward a dedicated follow-up epic.
- `type-subtyping.wast` — needs real structural func-type subtype checking
  for `call_indirect`/`ref.cast`: 2 `assert_trap` + 2 `assert_unlinkable`
  cases where this crate's interpreter is simply too PERMISSIVE (it lets a
  call/cast through that a real subtype checker would reject), confirmed
  by direct debugging of the actual outcome, not assumed.
- `extern.wast` — unchanged from the prior investigation: fails to parse
  (needs `anyref`, `any.convert_extern`/`extern.convert_any`,
  `struct.new_default`, `array.new_default`, declarative elem segments,
  and a `ref.host N` constant-expression literal this crate's parser
  doesn't recognize at all).

### Baseline diff

Full before/after `testsuite-status.json` diff confirms the only
already-vendored files whose tally changed are `linking.wast` and
`memory64.wast`/`table64.wast` (both explained above, both strict
improvements, zero new `fail` anywhere) — every other already-vendored
file's tally is byte-for-byte identical.

