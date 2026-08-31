# W31 — GC wildcard expectations + `module definition`/`module instance` generative instantiation

## Purpose and how this slice was chosen

A prior investigation session's `fetch_testsuite.py` comment named 9
remaining pinned-corpus files (`WebAssembly/testsuite`,
`28864811cf03bdbf880733786148feaba339582d`) as "genuinely blocked":
`array.wast`, `array_new_data.wast`, `array_new_elem.wast`, `struct.wast`,
`ref_null.wast`, `type-rec.wast`, `type-subtyping.wast`, `extern.wast`,
`instance.wast`. This slice re-investigated all 9 fresh against the real
fetched files rather than trusting that categorization (a recurring lesson
in this campaign — see `lessons.md`: prior "blocked" claims have
repeatedly turned out stale or incomplete). Six turned out tractable;
three stay genuinely blocked, for real, re-confirmed reasons documented in
`wasm-conformance/CHANGELOG.md`.

This spec covers the two genuinely NEW pieces of grammar/behavior added.
The per-file pass/fail numbers and the "still blocked" analysis live in
`wasm-conformance/CHANGELOG.md`, not duplicated here.

## Piece 1 — three new `assert_return` wildcard expectations

`array.wast`/`array_new_data.wast`/`array_new_elem.wast`/`struct.wast`
were assumed to fail because this crate has no `(type $t (array ...))`/
`(type $t (struct ...))` type-declaration grammar. That gap is real (see
"Deliberately out of scope" below), but it was NOT what was aborting these
four files' parse — actually running `wasm_wast_parser::parse_script`
against each one (not assumed) isolated the true cause to a single
`assert_return` shape:

```wat
(assert_return (invoke "new") (ref.array))
(assert_return (invoke "new") (ref.struct))
(assert_return (invoke "new") (ref.eq))
```

Three bare wildcard expectations — the array/struct/eqref counterparts of
the already-supported `(ref.func)`/`(ref.i31)` (W20) — that
`script::parse_expected` had never seen. Hitting one raised a hard
`WastParseError` that propagated all the way out of `parse_script`'s
per-directive loop, aborting the ENTIRE script. This is a materially
different failure mode from an unsupported MODULE body (`Directive::
Module`'s own `Result` capture already isolates those, see W14) — nothing
downstream of a `parse_directive` call for `assert_return`/`assert_trap`/
etc. catches its `?`-propagated errors before they reach `parse_script`'s
top-level `.collect()`.

Fixed by adding `Expected::RefArrayAny`/`RefStructAny`/`RefEqAny`, parsed
identically to `RefFuncAny`/`RefI31Any` (`("ref.array"|"ref.struct"|
"ref.eq", None) => Ok(...)`), and graded in `wasm-conformance::
value_matches_expected` the same conservative way: `RefArrayAny`/
`RefStructAny` accept any non-null ref handle (`WasmValue::Ref(Some(_))`)
since this crate's value representation carries no per-kind runtime type
tag distinguishing "some array ref" from "some struct ref" from "some
funcref" — the same limitation `RefFuncAny`'s own doc comment already
names. `RefEqAny` additionally accepts an i31 (`WasmValue::I32(_)`), since
`eqref`'s real members are `i31ref` plus every struct/array ref (not
`funcref`/`externref`).

This alone was enough for all four files to go from `FAILED TO PARSE
SCRIPT` to parsing cleanly. Their actual array/struct INSTRUCTION
directives (`array.new`/`struct.get`/etc. bodies, and the non-null
`(ref $t)` field/param types the real GC grammar uses pervasively even in
plain type declarations) still correctly grade `not_yet_supported` — see
"Deliberately out of scope" below.

## Piece 2 — `(module definition $M ...)` / `(module instance $I $M)`

`instance.wast`'s three sections ("Instantiation is generative", "Import
is not generative", "Export is not generative") need to declare ONE module
template and instantiate it possibly more than once, each instantiation
getting independent mutable global/table/memory state. A plain `(module
$id ...)` directive can't express this — `Directive::Module` builds AND
instantiates eagerly, exactly once, in the same step.

### Why silently falling through was actively dangerous

Before this change, `items[1]` being the bare atom `definition`/`instance`
(never `$`-prefixed) fell through to the ordinary `(module ...)` handling.
`extract_module_id` returns `None` for a non-`$`-prefixed `items[1]`, and
`parse_module_expr`'s own field loop only matches specific `SExpr::List`
keyword forms (`f.is_keyword_list("global")`, etc.) — a bare atom is
silently skipped, never an error. Concretely, `(module definition $M
(global ...) (table ...) ...)` would build as an ANONYMOUS module
containing `$M`'s own fields (losing the `$M` identity entirely), and
`(module instance $I1 $M)` (three bare atoms, zero field lists) would
build as a trivially EMPTY anonymous module. Both "succeed" while doing
nothing like what the script asked for — confirmed by actually running the
old code against `instance.wast`, not assumed: `(register "I1" $I1)`
referencing the never-really-registered `$I1` then hit a genuine, hard-to-
diagnose `Fail` ("no module registered as $I1") instead of the file's
actual generative-instantiation semantics ever running.

### The fix

Two new `wasm_wast_parser::script::Directive` variants:

- `ModuleDefinition { id: Option<String>, result: Box<Result<WasmModule,
  String>> }` — `id` is `None` for the rarer anonymous form (see below).
- `ModuleInstance { id: Option<String>, definition_id: String }`.

Both are recognized in `parse_directive` BEFORE the ordinary `"module" =>`
fallback, guarded on `items.get(1).and_then(|i| i.as_atom())` being
exactly `"definition"`/`"instance"`. `module definition`'s `$name` is
OPTIONAL: an anonymous `(module definition <fields...>)` is exactly how
the ALREADY-VENDORED `memory.wast`/`table.wast` spell "validate this
boundary-case module (e.g. a memory at exactly the max page count) but
don't actually instantiate/allocate it" — see the "Fixed" entry below.
`module instance`'s definition name is always required; its OWN instance
name is optional (`(module instance $M)`, an anonymous instance).

`wasm-conformance::Executor` gained:

- `definitions: HashMap<String, WasmModule>` — a `ModuleDefinition`'s
  built module, stored as a plain template (structurally validated, but
  NOT instantiated). Storing the raw template rather than a
  `ValidatedModule`/live instance is what makes "instantiate the same
  definition twice, independently" possible with no extra bookkeeping:
- `instantiate_and_register(&mut self, module: &WasmModule, id:
  Option<String>, set_current: bool) -> DirectiveOutcome` — the shared
  tail of `Directive::Module`'s own success path (validate → instantiate →
  register), extracted so `Directive::ModuleInstance` can reuse it with
  `set_current: false` (a named instance never becomes "the current
  module" — only reachable by its own `$id`, matching `instance.wast`'s
  own usage, which always addresses `$I1`/`$I2`/`$I` by name). Each
  `instantiate()` call already builds fresh global/table/memory state from
  the `WasmModule` template it's given, so instantiating `$M` twice
  naturally gives two independent live instances with zero additional
  code — this is the one place where an existing design decision (state
  lives on the instantiated `WasmInstance`, never mutates the template)
  paid for a feature it wasn't originally built for.

### Fixed as a direct consequence — `Register`'s capability-gap tracking generalized

`Directive::Register`'s "target not found" fallback only checked
capability-gap tracking (`current_module_status`) for the `None` ("current
module") registry key, never an explicit `$id` key. `(register "I1" $I1)`
naming an `$id` that never built for a genuine capability-gap reason
(`instance.wast`'s own case, and `type-rec.wast`'s `(register "M" $M)`
where `$M` uses a `(rec ...)` type group this crate can't build yet)
graded a hard `Fail` — indistinguishable from a real script bug — instead
of the honest `NotYetSupported` a capability gap deserves.
`current_module_status: Option<String>` generalized into
`unavailable_reasons: HashMap<Option<String>, String>`, keyed by ANY
registry key (`None` still covers "the current module", unchanged
behavior). This is a real, general harness correctness fix, not scoped to
GC files — it also turned ALREADY-VENDORED `linking.wast`'s pre-existing
`register` 2 `fail` into 0 `fail`/3 `not_yet_supported`, confirmed via a
full before/after baseline diff.

### Fixed as a direct consequence — anonymous `module definition` no longer silently instantiated

`memory64.wast`/`table64.wast` (already vendored) each contain an
anonymous `(module definition (memory/table i64 ...))` at a boundary-case
size specifically so an implementation can validate it WITHOUT actually
allocating it. Before this change, the `definition` keyword was silently
ignored (see "Why silently falling through was actively dangerous" above)
and the module was built AND INSTANTIATED as an ordinary anonymous module
— which is why each file had one `module` `trap` (a real runtime trap from
instantiating/executing against a huge i64 memory/table). Recognizing
`module definition` explicitly means these are now correctly validated
without being instantiated: both files' one `trap` is now a `pass`,
confirmed via the same full before/after baseline diff. `memory.wast`/
`table.wast` (same shape, smaller boundary size that never actually
trapped) are unaffected either way.

## Deliberately out of scope

- **GC `(type $t (array ...))`/`(type $t (struct ...))` type-declaration
  grammar itself**, and the array/struct INSTRUCTION opcodes that need it.
  `wasm_types::WasmModule` already has `struct_types: Vec<StructType>` and
  binary-format `struct.new`/`struct.get`/`struct.set` support (from an
  earlier slice), but there is still no text-format parsing path that
  populates it, and no `ArrayType`/array-instruction support anywhere in
  this crate stack at all. Every array/struct.wast directive that
  exercises this still correctly grades `not_yet_supported` after this
  slice.
- **Non-null concrete reference types (`(ref $t)`, no `null` keyword).**
  Confirmed still genuinely absent — `wasm_types::ValueType` has no
  non-nullable variant, only `ConcreteFuncRef`/`StructRef` (both
  nullable). This is a large, separate, multi-crate effort (`wasm-types`,
  `wasm-validator`'s type lattice, `wasm-module-parser`/`wasm-module-
  encoder`'s binary round-trip) — see this slice's own PR description for
  a scoping note toward a dedicated follow-up epic. `struct.wast`'s own
  function signatures (`(param (ref $vec)) ...)`) and even some of its
  OWN struct field lists (`(field i8 ... (ref 0) (ref null 1))`, non-null
  and nullable mixed in one field list) depend on this pervasively — this
  is why most of `struct.wast`'s real content still grades
  `not_yet_supported` even after this slice's grammar-adjacent fixes.
- **Bottom reference types** (`nullfuncref`/`nullexternref`/`nullexnref`/
  `nullref` as their own `ValueType` variants). `ref_null.wast` needs
  these to be genuine subtypes of every compatible ref type — this
  crate's current aliasing (`nullfuncref` → plain `ValueType::Funcref`)
  is unsound to also accept wherever a concrete `ConcreteFuncRef` is
  declared, and widening `is_assignable` to allow it would silently break
  `return_call.wast`'s/`return_call_indirect.wast`'s own mirror-image
  `assert_invalid` case. Deliberately NOT attempted here — see this
  slice's own PR description for the fuller scoping note.
- **Real recursive-type-group (`(rec (type $a ...) (type $b ...))`)
  semantics and explicit structural subtype checking.** `type-rec.wast`/
  `type-subtyping.wast` both need genuine work here; `type-rec.wast`
  parses and grades mostly-honestly without it (`assert_invalid` 9/10),
  but `type-subtyping.wast` has real, non-gradeable `fail`s (this crate's
  `call_indirect`/`ref.cast` are simply too permissive without real
  subtype checking) and is deliberately NOT vendored in this slice.

## Real corpus evidence

See `wasm-conformance/CHANGELOG.md`'s own entry for this slice for the
full per-file pass/`not_yet_supported`/fail breakdown and the complete
"still blocked" analysis for `ref_null.wast`/`type-subtyping.wast`/
`extern.wast`.
