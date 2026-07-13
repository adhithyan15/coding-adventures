# Changelog — iir-builtin-lowering

## 0.27.0 - 2026-07-13 (E6d-3b: `reverse` via a tail-recursive accumulator helper)

`lower_list_ops` gains a fourth list operation, `reverse`. It rewrites
`call_builtin "reverse" a` to a **nil-seeded** call to a synthesized
tail-recursive accumulator helper `__dyn_list_reverse` (injected once):

```
reverse(a)          = __dyn_list_reverse(a, nil)      # nil seed at the call site
__dyn_list_reverse(a, acc) = if null?(a) then acc
                             else __dyn_list_reverse(cdr(a), cons(car(a), acc))
```

Consing each element of `a` onto the *front* of the accumulator reverses the
order; the base case returns the accumulator. The call-site rewrite emits the
empty accumulator as a `const 0 : ref<LispyPair>` (the exact nil sentinel the
`list` desugar / `make_nil` emit), named `{dest}.rev_nil` (unique per SSA dest so
two `reverse` sites never collide). Like `append` there is no index → no
unbox/box; the recursion reuses `null?`/`car`/`cdr`/`cons` (E6d-1). Recursion is
in tail position but the backends do not yet TCO, so depth is bounded by the list
length. Five new unit tests (nil-seed + acc-call rewrite, single injection,
tail-recursive-accumulator shape, all-four-ops coexistence). `assoc` follows.

## 0.26.0 - 2026-07-12 (E6d-3b: `append` via a synthesized list-rebuild helper)

`lower_list_ops` gains a third list operation, `append`. It rewrites
`call_builtin "append" a b` to a call to a synthesized recursive helper
`__dyn_list_append` (injected once, idempotently) that *rebuilds* the first list
in front of the second:

```
__dyn_list_append(a: ref<any>, b: ref<any>) -> ref<any>:
    if !null?(a)  goto recurse       # → is_null (E6d-1)
    ret b                            # append(nil, b) = b
  recurse:
    ret cons(car(a), __dyn_list_append(cdr(a), b))
```

Unlike `list-ref` there is **no index**, so no unbox/box: `a`/`b`, `car(a)`, and
the recursive result are all lisp `ref<any>` references. Its only new op versus
`length`/`list-ref` is the `cons` in the recursive arm — the same E6d-1 heap
builtin, rewritten to `alloc`/`field_store` for the injected helper by the same
head-of-heap-lowering pass. Both arms return `ref<any>` (the second list, or a
fresh cons). Five new unit tests (rewrite, single injection, cons-rebuild shape,
all-three-ops coexistence). `reverse`/`assoc` follow the same pattern.

## 0.25.0 - 2026-07-12 (E6d-3b: `list-ref` via a synthesized index-walk helper)

`lower_list_ops` gains a second list operation, `list-ref`. Like `length` it
rewrites `call_builtin "list-ref" lst n` to a call to a synthesized recursive
helper `__dyn_list_ref` (injected once, idempotently) and reuses `car`/`cdr`
(E6d-1):

```
__dyn_list_ref(lst: ref<any>, n: ref<any>) -> ref<any>:
    ni = unbox n                       # boxed index → machine i64
    if !(ni == 0) goto recurse         # typed cmp_eq → raw bool
    ret car(lst)                       # base: the n-th element
  recurse:
    ret __dyn_list_ref(cdr(lst), box(ni - 1))   # typed sub, re-boxed
```

Design note — **the index is a boxed lisp value, not a raw `i64` param.** The
lisp boundary is uniform-anyref: `dyn_repr_structural` (managed) / `lower_dyn_repr`
(native) box *every* argument to a lisp function, so a raw-`i64` index param
faults at the call (`expected i64, got I32(2)`). The helper therefore takes
`n : ref<any>` and unboxes it once; the index test/decrement are then plain typed
`cmp_eq`/`sub` (the raw bool feeds `jmp_if_false` directly — hint `"bool"`, so it
is not treated as a lisp-truthiness condition), and the decremented index is
re-boxed before the recursive call (the same explicit-`box` shape the `length`
helper's base case uses). `length` and `list-ref` share the module entry and
each inject their helper independently. Five new unit tests (rewrite, single
injection, index-walk shape, coexistence with `length`). `append`/`reverse`/
`assoc` follow the same pattern.

## 0.24.0 - 2026-07-12 (E6d-3b: `length` via a synthesized cons-walk helper)

New `list_ops` module (`lower_list_ops`): a list *operation* like `length` walks
the cons chain, so it can't be a straight-line desugar (unlike E6d-3a's `list`
constructor). `lower_list_ops` rewrites `call_builtin "length" lst -> dest` into a
`call __dyn_list_length, lst -> dest : ref<any>` and injects (once per module) the
recursive helper

    __dyn_list_length(lst : ref<any>) -> ref<any>:
        if null?(lst) then (box 0) else (+ 1 (__dyn_list_length (cdr lst)))

The helper is a **proper lisp function** — both branches return a boxed
`ref<any>`, and the `+ 1 …` is the E6d-2 **dynamic** add (raw `i64` `1` + boxed
recursive result). This matters: a mixed i64/ref helper confused `dyn_repr`'s
lisp/machine partition (it classifies a function calling lisp builtins as lisp and
coerced the i64 return, giving `type mismatch: expected i64, got I32(0)`). As a
proper lisp function it rides `null?`/`cdr` (E6d-1) + dynamic arithmetic (E6d-2) —
nothing new lowers, so it reaches all five code-gen backends. Runs at the head of
both `lower_heap_builtins` and `lower_heap_builtins_runtime` (like the E6d-3a
desugar), so the helper's `null?`/`cdr` lower on both the managed and native
paths with no lang-aot pipeline change. 4 unit tests. (Depends on the WASM nil
`ref.null` fix, iir-to-wasm 0.38.0.) `list-ref`/`append`/`reverse` follow the same
helper pattern.

## 0.23.0 - 2026-07-12 (E6d-3a: `list` constructor desugars to a cons chain)

`list` is pure sugar over `cons` — `(list a b c)` = `(cons a (cons b (cons c
nil)))`. Rather than a new backend op, a new `desugar_list_in_function` pass
expands `call_builtin "list" …` into a nil `const` + a right-to-left `cons`
chain, and it runs at the **head of both** `lower_heap_builtins` (managed:
cons → `alloc`/`field_store`) **and** `lower_heap_builtins_runtime` (native/LLVM:
cons → `dyn_cons`), so `list` reaches all five code-gen backends via the E6d-1
cons path with **no lang-aot pipeline change and no `call_builtin` allowlist
entry** (the `list` builtin is gone before any backend sees it). `(list)` with no
args lowers directly to the nil sentinel. 5 unit tests (desugar shape, element
order + dest preservation, empty list, end-to-end alloc/field_store, and the
`dyn_cons` runtime path). List *operations* (`length`/`list-ref`/`append`/
`reverse`) are E6d-3b (they need a cons-walk helper, not a desugar).

## 0.22.0 - 2026-07-11 (E6d-2b: tagged-i64 box/unbox runtime calls + producer-agnostic ref<any>)

E6d-2b: new pass `lower_box_unbox_to_runtime_calls` rewrites the generic `box`/`unbox` ops (which `lower_dynamic_arith` emits) into `dyn_box_int`/`dyn_unbox_int` `call_builtin`s — the tagged-i64 (native/LLVM) representation of boxing, which the backends dispatch to `__dyn_box_int`/`__dyn_unbox_int`. The structural backends keep the generic ops. Also refines the DVAL01-3 `dyn_repr` seed: `ref<any>` (always a genuine tagged heap value) is now seeded **ungated**, so a **Twig** dynamic-arith result is exit-unboxed on the tagged-i64 backends, not just McCarthy Lisp; bare `any` stays gated on `is_lisp` (Twig placeholder). New tests: box/unbox -> runtime calls.

## 0.21.0 - 2026-07-11 (DVAL01-3: producer-agnostic DynValue classification)

DVAL01-3 (spec DVAL01 §3.3): `dyn_repr` now seeds its "boxed register" set from
**any op whose result type is a `DynValue`** (`any` / `ref<any>`) — not from a
hard-coded lisp-builtin allow-list. A register holds a tagged word because of
*what it is*, so a boxed **arithmetic** result (`dyn_box_int`, typed `ref<any>`)
is exit-unboxed exactly like a `dyn_car` result — the concrete generalisation
that unblocks dynamic arithmetic on native/LLVM (E6d-2b). The seed is gated on
`is_lisp` (Twig/Nib use `any` as a pre-resolution placeholder on ordinary
machine values, so seeding on the hint outside a dynamic module would mis-box
them). Strict superset of the old seeds, so existing lisp programs are
unaffected. New tests: a boxed non-cons DynValue is exit-unboxed; a Twig `any`
module is a no-op.



## 0.20.0 - 2026-07-11 (DVAL01-2: rename IIR builtin names lispy_* -> dyn_* + passes)

DVAL01-2 (spec DVAL01 sections 3.1-3.3): the IIR builtin *names* are de-lisped. The RUNTIME_RENAMES second column (`cons`->`dyn_cons`, `car`->`dyn_car`, `cdr`->`dyn_cdr`, `pair?`->`dyn_pair_p`, `equal?`->`dyn_equal`) and every `lispy_*` IIR name (`box_int`/`unbox_int`/`truthy`/`to_exit_code`/`nil`/`make_symbol`) become `dyn_*`. The boxing passes `lisp_repr`/`lisp_repr_structural` (files + `lower_lisp_repr`/`lower_lisp_repr_structural` fns) are renamed `dyn_repr`/`dyn_repr_structural`/`lower_dyn_repr`. Prefix-preserving (`dyn_pair_p`, not `dyn_is_pair`) so the IIR name maps cleanly to the already-shipped `__dyn_*` runtime symbol. Pure rename -- no lowering behaviour change; all backends stay in agreement.

## 0.19.0 - 2026-07-11 (DVAL01-1b: rename C runtime file lispy_runtime.c -> dynval_runtime.c)

DVAL01-1b: the shared C runtime file is renamed `lispy_runtime.c` -> `dynval_runtime.c` (and the golden test `lispy_runtime_golden.rs` -> `dynval_runtime_golden.rs`), continuing the de-lisp of the generic dynamic-value substrate (spec DVAL01). Pure file/path rename -- no symbol, ABI, or behaviour change; the link/build path strings that reference the runtime are updated to match. The `lispy-runtime` Rust crate rename follows in DVAL01-1c.

## 0.18.0 - 2026-07-11 (DVAL01-1a: dynamic-value runtime ABI __twig_lispy_* -> __dyn_*)

De-lisp the tagged dynamic-value runtime ABI: every `__twig_lispy_*` C symbol (box_int/unbox_int/cons/car/cdr/pair_p/equal/not/nil/make_symbol/truthy/to_exit_code/tag_*) is renamed to the language-neutral `__dyn_*` (per spec DVAL01). Pure rename -- the 3-bit tag layout, encodings, and runtime behaviour are byte-for-byte unchanged, so any dynamic frontend (not just lisp) can target the same primitives. The GC ABI (`__twig_gc_*`) is untouched.

## 0.17.0 — 2026-07-11 (LANG-FULL E6d-2a: dynamic integer arithmetic over `any`)

New `dynamic_arith` pass (`lower_dynamic_arith`): a dynamic (lisp) frontends `call_builtin "+"/"-"/"*"/"/"/quotient/remainder/modulo` and comparisons `=/<//>/<=/>=` over **boxed** `ref<any>` operands are expanded structurally to `unbox → typed op → box` — the same `unbox`/`add`/`box` ops the code-gen backends already run for `cons`. A raw (already-`i64`) operand is used directly. Integer contract (layer 2); i64 machine width. Proof: `(+ (car (cons 41 0)) 1)` → 42. 5 unit tests.

All notable changes to this crate are documented here.

---

## [0.16.0] — 2026-06-10 — lambda value-model: arg boxing + polymorphic result coercion (LANG77 / McCarthy W13b)

Closes the two tagged-word value-model gaps a `LAMBDA` exposes in `lower_lisp_repr`
(F7), reusing the managed pass's lisp-function partition (`lisp_functions`, now
`pub(crate)`):

- **Argument boxing** — `lisp_arg_regs` now includes the arguments of a `call` to a
  **lisp function** (not just lisp builtins), so an integer atom passed to a lambda
  is boxed (`n << 3`). Without it a raw `5` reads as tag `0b101` (`#t`) and `7` as
  `0b111` (a heap pair) inside the body.
- **Polymorphic result coercion** — a lambda result is a `call` typed `any` of
  unknown runtime tag, so the entry-exit coercion wraps it with the new
  `lispy_to_exit_code` (a RUNTIME tag switch) instead of the static
  `unbox_int`/`truthy`/verbatim that the int/bool/symbol cases use.
- **Language gate** — both behaviours fire only for a lisp module (`language ==
  "mccarthy-lisp"`). Twig shares this pass and also types untyped params `any`, so
  the gate keeps the pass a faithful no-op for Twig (regression-tested:
  `non_lisp_call_is_left_untouched`).

New unit tests `lambda_call_boxes_int_arg_and_coerces_result` +
`non_lisp_call_is_left_untouched`.

## [0.15.0] — 2026-06-10 — symbol program-result handling (LANG77 / McCarthy W13a)

Extends the type-directed program-exit coercion in `lower_lisp_repr` (the bool case
landed in 0.14.0) to **symbols**: a SYMBOL result is already a finished tagged
immediate from `intern_symbols` (`(id << shift) | TAG_SYMBOL`), so it must NOT be
`lispy_unbox_int`'d (`>> 3` would corrupt the id+tag). Such a `ret` is returned
verbatim (its tagged word). Reusable for every tagged-word backend; verified
end-to-end on the LLVM/clang path (`(EQ (QUOTE A) (QUOTE A))`→1). New unit test
`symbol_result_returned_verbatim`.

## [0.14.0] — 2026-06-10 — boolean program-result coercion (LANG77 / McCarthy W12b-2)

Closes the long-deferred "booleans land in L3b-2c-2" gap in `lower_lisp_repr`'s
program-exit handling. A value produced by a predicate (`pair?`/`equal?`/`not`) is
a tagged **boolean** (`LISPY_TRUE = 5` / `LISPY_FALSE = 3`), not a tagged integer —
so `insert_unbox_before_lisp_rets` is now **type-directed**:

- an INTEGER result is unboxed (`lispy_unbox_int`, `>> 3`) as before;
- a BOOLEAN result (its producing instruction carries the `bool` type hint) is run
  through `lispy_truthy` (→ raw `0`/`1`). Unboxing a true (`5 >> 3 = 0`) would have
  reported *false* — the bug this fixes.

Reusable for every tagged-word backend (LLVM/AOT/JIT) that links `lispy_runtime.c`.
Verified end-to-end in `lang-aot` on the LLVM/clang path: `(ATOM 7)`→1,
`(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(EQ 7 8)`→0. Updated the
`tagged_cond_is_wrapped_with_truthy` unit test (the bool-typed `ret` now also
truthy-coerces) and added `integer_result_unboxed_boolean_result_truthied`.

## [0.13.0] — 2026-06-09 — reference funnels + lisp-call result type (LANG77 / McCarthy W5b)

### Fixed

- `lower_lisp_repr_structural` now produces IIR a **strict** backend (JVM/CLR/
  BEAM) can type, where the loose wasm model previously got away with ambiguity:
  - **Lisp `call` results are retyped `ref<any>`.** The frontend hints a call
    `i64`; a lisp function returns the uniform-anyref value, so the call result is
    a reference. (The JVM stored an `Object` result into a `long` slot otherwise —
    a recursive `LABEL` returned garbage.)
  - **Reference funnels.** A `COND` `mov`s each clause's value into one result
    register. If any clause yields a reference (a cons, `nil`, or a lisp call
    result — e.g. a recursive `LABEL`), the funnel must be a reference in *every*
    clause. `ref`-ness is now propagated through `mov` chains to a fixpoint, and
    the rebuild **boxes each atom clause into the funnel** (`mov %fun, %atom` →
    `box %fun, %atom`) and retypes the reference clauses — instead of boxing the
    whole funnel once at `ret`, which mis-boxed a clause that already held a
    reference.

These make McCarthy `LAMBDA`/`LABEL`/recursion and mixed atom/cons `COND` run on
the JVM. wasm is unaffected (regression-tested). 1 new test.

## [0.12.0] — 2026-06-09 — uniform-anyref function boundary (LANG77 / McCarthy W2)

### Changed

- `lower_lisp_repr_structural` now makes the **function-call boundary**
  uniform-anyref, so a `LAMBDA`/`LABEL` can be applied and can recurse:
  - a new `lisp_functions` module analysis (functions using the heap/predicates
    or taking lisp params, closed under *calling*) replaces the per-function
    `function_uses_heap` gate — every lisp function is processed, even a trivial
    `(LAMBDA (X) X)`;
  - each **lisp parameter** (`any`/`symbol`) is retyped to `ref<any>` and treated
    as a reference;
  - each argument of a `call` to a lisp function is **boxed** (`i31ref`) before
    crossing the boundary, and the **call result** is a reference;
  - a **non-entry** function (a lambda) returns `ref<any>` — boxing a scalar /
    predicate-boolean / atom result — so every value crossing a boundary is an
    `anyref`; the entry function still unboxes its result to `i32`.
  - Recursion needs no special handling: a self-`call` is just a lisp call.
- `lang-aot::concretize_scalar_any_for_wasm` correspondingly skips functions with
  lisp params, keeping the two passes' partition exact.

1 new test (the `((LAMBDA (X) X) 5)` boundary: param→ref<any>, arg boxed, return
ref<any>, caller unboxes).

## [0.11.0] — 2026-06-09 — managed-backend symbol interning (LANG77 / McCarthy W1)

### Added

- **`intern_symbols_structural`** — the managed/uniform-reference twin of
  `intern_symbols`. Interns each symbol literal (`const Var(name) : symbol`) to a
  distinct integer in a reserved high range (`SYMBOL_ID_BASE = 2²⁹` + module-wide
  id) and retypes it to `i32`, so a symbol flows like any integer atom: the
  structural pass boxes it as an `i31ref` and `EQ` compares the payloads with
  `i32.eq`. Distinct symbols get distinct values (`(EQ 'A 'B)` → nil), same
  symbols share one (`(EQ 'A 'A)` → T), and the reserved range keeps symbols
  disjoint from integer atoms (`(EQ 'A 5)` → nil). No new value type, no
  polymorphic `EQ`. Reusable across WASM/JVM/CLR/BEAM (each adapts the encoding;
  the WASM path uses the integer ids directly). 4 new tests.

## [0.10.0] — 2026-06-09 — `COND` lisp-truthiness (LANG77 / McCarthy L3b-3a-4d)

### Changed

- `lower_lisp_repr_structural` now wraps **lisp-value `COND` guards** with a
  truthiness test, so `COND` branches with McCarthy semantics on wasm. A
  `jmp_if_false` whose condition is a predicate result (hint `"bool"`) is tested
  directly, but a condition that is a **lisp value** (an integer atom, `nil`, a
  cons, a variable) is rewritten:

  ```
  %n = is_null(%cond_boxed)   ;; 1 iff cond is nil
  %t = not(%n)                ;; 1 iff cond is truthy (non-nil)
  jmp_if_false %t, L          ;; branch iff cond is nil/false
  ```

  so a lisp integer atom — **even `0`** — is true, and only `nil` is false
  (integer atoms are boxed as `i31ref` first so `is_null` is well-typed). The
  result funnel is unchanged (it already returns the clause's value, or `nil` as
  `0`, through the loose value model — uniform funnel boxing is deferred because
  it would make a `nil` return unbox-trap).

1 new test (a lisp-value guard is wrapped with `is_null`/`not`; a machine-boolean
guard is tested directly).

## [0.9.0] — 2026-06-08 — predicate atoms box (LANG77 / McCarthy L3b-3a-4b)

### Changed

- `lower_lisp_repr_structural` now also handles functions that use the lisp
  **predicate** builtins (`pair?`/`not`/`equal?`), not just the cons heap ops —
  so a program like `(ATOM 5)` (which `cons`es nothing) is owned by this pass
  rather than slipping through untyped:
  - `function_uses_heap` additionally returns true for a `call_builtin` to a
    lisp builtin, matching `concretize_scalar_any_for_wasm`'s `LISP_BUILTINS`
    list so the two passes still partition the module cleanly.
  - the atom-boxing rule generalised from "the value stored into a cons field"
    to "any non-reference value flowing into a **lisp-value position**" — which
    now also covers the arguments of `pair?`/`equal?` (a lisp integer atom is
    boxed as an `i31ref` before the predicate). `not`'s argument is a machine
    boolean, so it is left alone.
  - a non-reference **scalar** result is concretised to its real width: a
    predicate result (hint `"bool"`) returns as `i32` (not widened to `i64`), so
    the wasm function's result type matches the value on the stack.

1 new test (predicate atom boxes, bool result stays i32, not unboxed).

## [0.8.0] — 2026-06-08 — structural lisp-value representation (LANG77 / McCarthy L3b-3a-3c)

### Added

- **`lower_lisp_repr_structural`** — the *managed-backend* (wasm/jvm/clr/beam)
  twin of `lower_lisp_repr`. Where the native pass tags integers with the
  NaN-box `n << 3` over the runtime-call form, this pass implements the
  **uniform-anyref** model over the *structural* heap form
  (`alloc`/`field_store`/`field_load`):
  - an integer atom stored into a cons field is **boxed** as an `i31ref` — a
    `box` op is inserted and the atom's `const` is narrowed to `i32` (the
    `ref.i31` payload width);
  - a value that is already a reference (an `alloc`/`field_load`/`box` result or
    a `ref<…>` const) is left alone;
  - in the **entry** function a `ret` of a reference is **unboxed** to `i32`
    (`unbox`), and the return type becomes `i32` (the machine exit code);
    non-entry functions return their lisp value as `ref<any>`.
  - Use-site directed and gate-free (no per-language switch): a function with no
    heap op is left entirely to `concretize_scalar_any_for_wasm`, so the two
    passes partition the module and every value ends up concretely typed.
  - Atoms outside the `i31` range (`±2³⁰`) are left unboxed (and rejected
    downstream) rather than silently truncated.

This is what lets a McCarthy **cons** program compile to a runnable WasmGC
module: `(CAR (CONS 7 9))` → `7`. 4 new unit tests.

## [0.7.0] — 2026-06-04

### Added (LANG77 — compile-time symbol interning, McCarthy L3b-2c-3)

- **`src/symbol_intern.rs`** + `intern_symbols` (re-exported at the crate
  root): rewrites each `const Var(name) : symbol` to the finished **tagged
  immediate** `(id << 32) | TAG_SYMBOL`, assigning ids in first-seen order
  **module-wide** (so the same name → the same id across functions). This is
  what makes `EQ`/`equal?` on symbols word equality on native — without any
  runtime interning or string-constant machinery (the native backend has
  none). General and language-agnostic: any lisp frontend's symbol literals
  intern the same way; the ids are module-local and need not match the VM's.
- **`lisp_repr`** now recognises a symbol immediate (`type_hint == "symbol"`)
  as a tagged `LispyValue` — it joins `boxed_regs` (so it propagates through
  `mov`, drives `COND` truthiness, etc.) but is **never boxed** (a `<< 3`
  would corrupt the id/tag).
- 5 new tests: same-name→same-id, the `(id<<32)|tag` encoding, module-wide
  ids, non-symbol consts untouched, and the `lisp_repr` "tagged-but-not-boxed"
  guard.

> Runtime `make_symbol` + string-literal emission (needed only to *print* a
> symbol's name or create symbols dynamically) remains deferred — static
> programs observe a symbol *value* via `EQ`, which compile-time interning
> fully supports.

---

## [0.6.0] — 2026-06-04

### Added (LANG77 — ATOM/EQ predicates + COND truthiness, McCarthy L3b-2c-2)

- **`heap::lower_heap_builtins_runtime`** now also renames the *unambiguous*
  predicates `pair?` → `lispy_pair_p` and `equal?` → `lispy_equal`
  (`EQ` = `equal?`). `not` is **not** renamed here — it is also a *numeric*
  builtin (Twig's machine boolean-not), so renaming it unconditionally would
  hijack Twig. Instead `lisp_repr` renames `not` → `lispy_not` **type-directed**
  (`rename_lisp_not`): only when its argument is a `lispy_*` result — exactly
  the `ATOM` = `not(pair?)` shape — leaving Twig's `not` for the numeric pass.
- **`lisp_repr::lower_lisp_repr`** extended:
  - The predicate builtins join the lisp-arg set, so an integer atom flowing
    into `(ATOM 5)` / `(EQ 5 5)` boxes.
  - The tagged-register classification is now a **bidirectional `mov`
    fixpoint** — a `COND` funnels every clause's value into one register, so a
    raw integer-literal clause result `mov`-tied to the (tagged) nil
    fallthrough is itself boxed, keeping the funnel register uniformly tagged
    (and the exit-unbox correct).
  - New `wrap_tagged_conditions`: a `jmp_if_false` whose condition holds a
    tagged `LispyValue` (a `COND` predicate's `#t`/`#f`) is rewritten to test
    `lispy_truthy(cond)` (raw `0`/`1`), so the branch follows lisp truthiness.
    A raw machine condition (Twig's `cmp` result) is left untouched.
- 6 new unit tests: predicate-arg boxing, truthy-wrap of a tagged condition,
  raw condition left unwrapped, `mov` propagation for unbox, and the
  COND-mixing (literal + nil) bidirectional-box case.

---

## [0.5.0] — 2026-06-04

### Added (LANG77 — type-directed lisp-value representation, McCarthy L3b-2c-1)

- **`src/lisp_repr.rs`** + `lower_lisp_repr` (re-exported at the crate root):
  a **gate-free, type-directed** pass that gives native lisp values their
  NaN-box tag. A raw integer's low 3 bits (`111` for `7`) collide with the
  heap tag, so `pair?`/`ATOM` would misread it as a pointer — integers
  destined for lisp positions must be boxed (`n << 3`, tag `000`).
- The rule is **use-site directed, not per-language**: a `const Int(n) : i64`
  is boxed iff its register feeds a `lispy_*` call (`lispy_cons`/`car`/`cdr`);
  the nil sentinel (`Int(0) : ref<LispyPair>`) becomes `TAG_NIL` (`0b001`); a
  register holding a lisp-builtin result is tagged. At the machine boundary —
  the **entry function's** `ret` of a boxed value — an unbox is inserted
  (`lispy_unbox_int`), so the process exit code is the raw integer. McCarthy
  (no arithmetic) boxes every atom; a Twig/Nib program whose integers feed
  `add`/`print_i64` (never a `lispy_*` call) is left byte-for-byte unchanged.
  Out-of-range ints (beyond ±2⁶⁰) are left raw rather than truncated.
- 7 unit tests: boxed cons/car round-trip + unbox, scalar-int untouched,
  machine arithmetic untouched, nil-tag, non-entry not unboxed, out-of-range,
  and end-to-end composition with `lower_heap_builtins_runtime`.

---

## [0.4.0] — 2026-06-04

### Added (LANG77 — native runtime-call heap lowering, McCarthy L3b-2b)

- **`heap::lower_heap_builtins_runtime`** (+ `lower_heap_function_runtime`),
  re-exported at the crate root. The **target-aware** counterpart of
  `lower_heap_builtins`: instead of expanding `cons` to `alloc` + two
  `field_store`s (the structural form the managed wasm/jvm/clr/beam backends
  consume), it **renames** `cons`/`car`/`cdr` → `call_builtin
  "lispy_cons"/"lispy_car"/"lispy_cdr"`, which the native aarch64/x86_64
  backends dispatch to `__twig_lispy_*` in the linked C lisp runtime
  (`twig-aot/runtime/lispy_runtime.c`, LANG77). This keeps the value
  NaN-box **tagged** (a heap-tagged pointer), the prerequisite for
  `pair?`/`ATOM`/`EQ`/symbols (L3b-2c).
- The transform is a pure in-place rename (arg order already matches the C
  ABI), allocation-free, and a no-op for any module without those builtins —
  so every non-lisp program is unchanged. Nothing here is language-specific:
  any lisp-family frontend (McCarthy Lisp, Twig, future lisps) reaches both
  the managed (structural) and native (runtime-call) worlds from the same
  `call_builtin "cons"` IIR. `null?`/`make_nil`/`pair?`/`not`/`equal?`/
  `make_symbol` are intentionally left for L3b-2c.
- 5 new unit tests covering the rename, dest/arg preservation, the
  left-unchanged builtins, and the non-lisp no-op.

## [0.3.0] — 2026-05-12

### Added (LANG34 — Phase 4 Closure Builtin Lowering)

#### New `src/closure.rs` module

Phase 4 of the builtin-lowering pipeline.  Rewrites legacy
`call_builtin "make_closure"` / `"apply_closure"` instructions — emitted by
pre-LANG34 compilers and hand-built tests — to first-class LANG34 opcodes:

| Legacy form | LANG34 form |
|-------------|-------------|
| `call_builtin "make_closure" fn_name_reg cap0…` | `alloc_closure(Str(fn_name), cap0…) : "closure"` |
| `call_builtin "apply_closure" handle arg0…` | `call_closure(handle, arg0…) : "any"` |

**Algorithm:** two-pass per function.  Pass 1 builds a
`HashMap<register, literal_text>` from `const` instructions.  Pass 2 rewrites
`make_closure`/`apply_closure` and drops `const` instructions that become
dead (single-use, only consumed by the rewritten `make_closure`).

**Infallible:** `make_closure` with an unresolvable fn_name register is left
unchanged for the twig-vm fallback / backend validator.

Public API: `pub fn lower_closure_builtins(module: &mut IIRModule)` +
re-exported at crate root as `lower_closure_builtins`.

10 unit tests covering: zero-capture rewrite, two-capture rewrite, multi-use
const preservation, unresolvable case, apply_closure rewrite, mixed forms,
idempotency, already-lowered no-op.

#### `lower_builtins` Phase 4 call

`lower_builtins` in `lib.rs` now calls `closure::lower_closure_builtins`
after Phase 3 (global/IO lowering).

#### Updated test_73 comment

`test_73_make_closure_left_unchanged` renamed to
`test_73_make_closure_unresolvable_left_unchanged` with updated comment
explaining the LANG34 Phase 4 behavior for unresolvable cases.

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O Phase 3 lowering)

#### New `src/global_io.rs` module

Phase 3 of the builtin-lowering pipeline rewrites three `call_builtin` opcodes
to typed IIR opcodes that all four native backends (`iir-to-beam`,
`iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`) understand
directly.

**Look-back lowering algorithm**

The twig-ir-compiler encodes global variable names as string-as-Var `const`
instructions (`const %n1 = Var("x")`), then passes the register to
`call_builtin "global_set"`.  The Phase 3 pass runs two sub-passes per
function:

1. **Pass 1** — build `const_str_map: HashMap<register, literal_text>` for
   every `const` instruction whose `srcs[0]` is `Operand::Var(text)`.
2. **Pass 2** — rewrite each `call_builtin "global_set"/%"global_get"/%"print"`
   using the resolved name from the map:
   - `call_builtin "global_set", %n, %v` → `global_store Str("name"), Var(%v)`
   - `call_builtin "global_get", %n` → `global_load Str("name")`
   - `call_builtin "print", %v` → `io_out Var(%v)`

Unresolvable instructions (name register not in const_str_map, missing srcs)
are left as `call_builtin` so the backend validator can surface a clear error.

**Exported entry points**

- `lower_global_io_function(fn_: &mut IIRFunction)` — single-function entry point.
- `lower_global_io(module: &mut IIRModule)` — whole-module entry point, wired
  into `lower_builtins()` as Phase 3.

**Tests** — 22 new tests in `src/global_io.rs`:

- `global_set` rewrites with resolvable and unresolvable name registers.
- `global_get` rewrites with resolvable and unresolvable name registers.
- `print` is always rewritten (no look-back needed).
- Multiple globals in one function.
- `call_builtin` for unknown builtins left unchanged.
- Non-`call_builtin` instructions left unchanged.
- Multiple functions in one module.
- Empty function / empty module edge cases.
- Type hints and profiling fields preserved through rewrite.

#### `src/lib.rs` changes

- `pub mod global_io;` added.
- `pub use global_io::lower_global_io;` re-exported from crate root.
- `lower_builtins()` now calls `global_io::lower_global_io(module)` as Phase 3,
  after Phase 1 (numeric) and Phase 2 (heap).

---

## [0.1.0] — 2026-05-11

### Added

- Initial release: Phase 1 numeric builtin lowering pass (LANG31 §1.1).
- `lower_builtins(module: &mut IIRModule) -> Vec<BuiltinLoweringError>` —
  mutating entry point.
- `lower_builtins_cloned(module: &IIRModule) -> (IIRModule, Vec<BuiltinLoweringError>)` —
  non-destructive entry point that preserves the original.
- `lower_builtins_checked(module: &mut IIRModule) -> Result<(), Vec<BuiltinLoweringError>>` —
  convenience wrapper that returns `Err` on any error.
- `BuiltinLoweringError` enum with two variants:
  - `WrongArity` — emitted when a numeric builtin is called with the wrong
    number of arguments.
  - `UntypedBuiltin` — emitted when a numeric builtin's `type_hint` is still
    `"any"`, indicating the pipeline ordering is wrong.
- `src/numeric.rs` — the 18-entry lowering table and in-place instruction
  rewrite logic.
- `src/error.rs` — `BuiltinLoweringError` enum and `Display` / `Error` impls.
- `src/lower.rs` — original simple lowering pass (no arity/type checking),
  kept for backward compatibility.
- `tests/test_lowering.rs` — 50 comprehensive tests covering:
  - All 18 numeric builtins (add, sub, mul, div, mod, neg, cmp_eq, cmp_ne,
    cmp_lt, cmp_le, cmp_gt, cmp_ge, and, or, not, shl, shr, xor).
  - Binary op invariants: dest preserved, srcs stripped, type_hint preserved.
  - Unary op invariants (neg, not).
  - Unknown builtins left unchanged.
  - Non-call_builtin instructions left unchanged.
  - `may_alloc` cleared after lowering.
  - WrongArity and UntypedBuiltin error cases.
  - Multi-function modules.
  - Empty modules and empty functions.
  - Mixed call_builtin / non-call_builtin instruction streams.
  - `lower_builtins_cloned` preserves original.
  - `lower_builtins_checked` returns Ok/Err correctly.
  - Profiling fields (observation_count, observed_type, ic_slot) preserved.
  - Multiple errors accumulated across functions.

### Not yet implemented (Phase 2)

- `src/heap.rs` — heap builtin lowering (`"cons"`, `"car"`, `"cdr"`,
  `"null?"`, `"pair?"`) is tracked in LANG31 Phase 2.
