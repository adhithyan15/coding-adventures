# Changelog — iir-builtin-lowering

## 0.42.0 - 2026-08-27 - propagate boxed dynamic returns through call signatures

`lower_dynamic_arith` now widens a function return and every matching call
result to `ref<any>` when the rewritten return value is produced by a dynamic
`box`. This keeps the IIR signature aligned with the value that actually
crosses the boundary, so the tagged native/LLVM exit pass unboxes a helper's
dynamic integer result instead of exposing the tagged word as the process exit
code. A fixed-point propagation covers wrappers that return another dynamic
helper's result.

## 0.41.0 - 2026-08-26 - fix: raw-model closure body with no heap evidence of its own never got concretized

`closure_identity_returns_captured_value` (`lang-aot`'s
`tests/e6d7a_wasm_closures.rs`) has been failing on `((lambda (x) x) 42)` —
"the minimal apply" — for at least four consecutive PRs in this campaign,
each re-confirming it as pre-existing/unrelated via a `git stash` comparison
and moving on without fixing it. Root-caused and fixed for real this time.

Two compounding bugs, both in how a Twig/Nib **raw-model** closure body (one
whose parameters and return value are plain machine words, not the
uniform-`anyref` tagged model — see `closure_heap.rs`'s own doc comment)
is classified and type-checked once its own body has **no heap evidence at
all**:

1. `lower_dyn_repr_structural::lisp_functions` (this pass's own per-function
   partition of "does this function use the tagged/heap value model")
   correctly excludes such a body (e.g. `fn __lambda_0(x: any) { ret x }` —
   no `alloc`/`box`/heap builtin anywhere), leaving it for
   `lang-aot::concretize_scalar_any_for_wasm` to concretize instead. But that
   sibling pass recomputed its OWN separate whole-module heuristic ("does ANY
   function anywhere in the module show heap evidence?") instead of reusing
   this pass's partition — and the module's synthesized closure dispatcher
   (`__dyn_call_closure`) always shows heap evidence elsewhere in the same
   module, so the whole-module check skipped concretizing *every* function,
   including this one. `lisp_functions` is now `pub` (re-exported from the
   crate root) specifically so `concretize_scalar_any_for_wasm` can partition
   against the exact same set instead of a disagreeing heuristic — the two
   passes together are supposed to leave every function's value concretely
   typed, and now they structurally cannot disagree about which one owns a
   given function.
2. Even with (1) fixed, the synthesized dispatcher's own `call` to such a
   body (`closure_heap::build_dispatcher`) hardcodes the call's type hint to
   `ref<any>` unconditionally (it runs before this pass and cannot yet know
   which bodies will turn out raw vs. tagged) — a real mismatch once the
   callee is concretized to a genuine `i64` machine return. `iir-to-wasm` saw
   a `box`'s source claim `ref<any>` while the real WASM `call` it lowers to
   actually pushes an `i64` (`TypeMismatch: expected Anyref, found I64`).
   `lower_structural_function` now corrects a stale `ref<...>`-hinted `call`
   to a callee it has independently determined is NOT on the tagged model,
   letting the existing defensive `any`/`polymorphic` sweep concretize it
   correctly. Deliberately NOT fixed in `closure_heap.rs` itself — that pass
   is shared with the native/LLVM pipeline (its own doc comment says so), and
   an earlier attempt to make the hardcoded hint conditional there
   passed the WASM case but broke `closures_run_on_native`/`_llvm` (42 → 80),
   confirming the native tagged-word model needs a different, unrelated
   invariant from this hint that this fix does not touch.

`concretize_scalar_any_for_wasm` (in `lang-aot`) also gained the missing
parameter-narrowing step its JVM twin (`concretize_scalar_any_for_jvm`)
already had — narrowing only the return type and instruction hints left a
newly-reachable raw closure body's own parameter at `any`, which `iir-to-wasm`
falls back to mapping as `i32`, a second real mismatch against the
dispatcher's already-unboxed `i64` argument.

New regression test in `dyn_repr_structural.rs`:
`stale_ref_any_hint_on_a_call_to_a_non_lisp_callee_is_corrected`, exercising
the fix directly on the IIR shape, independent of the full compile-and-run
integration test in `lang-aot`.

## 0.40.0 - 2026-08-25 - nullable structural entry results

The managed structural Lisp representation now maps a nullable entry result
to process exit code zero before unboxing non-null integer atoms. This keeps
`nil`-returning `COND` programs compatible with the process boundary while
preserving the spec-required `i31.get_s` trap on null references elsewhere.

## 0.39.0 - 2026-08-18 - lower_dynamic_arith handles a unary `call_builtin "-"` (negate)

`lower_dynamic_arith_function` only ever matched the binary (2-operand) shape
of a dynamic arithmetic `call_builtin`. Macsyma's `macsyma-iir-compiler`
(`macsyma-iir-vm.md` Wave 4) always emits `call_builtin "+"/"-"/"*"` — even
for two already-concrete literal operands, and for unary negate (`-x`) as a
**one**-operand `call_builtin "-"`. That one-operand shape fell through this
pass unrewritten and reached the code-gen backends' own `call_builtin`
whitelist, which only knows heap/predicate builtins — surfaced as a WASM
`UnsupportedOp` validation failure for `-7$`.

Fixed by adding a unary-negate case (matched before the binary path, on
`name == "-"` with exactly one `Var` operand): `unbox` (if boxed) → `neg` →
`box`, reusing the raw typed `neg` IIR op every backend already implements
(it's already in `interpreter_ir::opcodes::is_arithmetic`, and each of
`iir-to-wasm`/`iir-to-llvm`/`iir-to-jvm-class-file`/`iir-to-cil-bytecode`/the
native backends already lowers it) — no backend change needed for this half
of the fix; see `clr-simulator` 0.5.0 for the other half (a genuine
CLR-simulator gap this surfaced).

## 0.38.0 - 2026-08-13 - unbox a tagged parameter before doing arithmetic on it

The function boundary declares every lisp parameter `ref<any>` and callers box
for it — but a body that then reads the parameter directly in an `add`/`cmp_*`/
bitwise op was operating on the TAG:

```
fn __lambda_0(x: ref<any>)
    add _r2.raw1 = x + 1   i64      <- x is boxed; this adds 1 to (41 << 3)
```

`((lambda (x) (+ x 1)) 41)` returned 329 instead of 42 on every managed backend.
That is why the JVM and CIL pipelines refused `alloc_closure` outright rather
than lower it: the refusal was hiding a wrong answer.

`unbox_machine_uses_of_tagged_params` now inserts `unbox %x.rawparam = %x : i64`
at entry and rewrites only the machine-op operands to use it. Two properties
make this safe:

* **The signature does not change**, so every caller keeps boxing exactly as
  before and no call site is touched. An earlier attempt retyped the PARAMETER
  to a raw `i64` instead; security review showed that is unsound, because this
  pass can insert a `box` (step 1 boxes any non-reference flowing into a
  lisp-value position) but has no way to insert an `unbox` at a call site. A
  forwarding wrapper `f(y) = g(y)`, or `(g (car p))`, would hand `g` a reference
  with no conversion anywhere. Repairing the body cannot have that failure mode.
* **The machine-op list is a positive list**, so an operation missing from it
  keeps reading the tagged parameter — the pre-existing behaviour for that
  shape, not a new miscompile. Reference uses (`field_load` bases, cons values,
  `pair?` arguments) are never rewritten.

Also collapses a **double box**: `lower_closures_to_heap` runs while the
parameter is still bare `any` and boxes a capture it believes is a raw word.
Once the parameter is `ref<any>` that is a second box over an already-tagged
value, and the JVM rejected the capturing closure with `Register 0 contains
wrong type`. Boxing a tagged value is never right, so it becomes a copy.

## 0.37.0 - 2026-08-13 - the passes no longer ask which language wrote the module

`dyn_repr::is_lisp_language` — literally `language == "mccarthy-lisp"` — is
deleted, along with all three of its decision sites. No pass in this crate reads
`module.language` any more.

It existed to disambiguate bare `any`, which McCarthy used for "a tagged
LispyValue" and Twig/Nib used for "statically unresolved, passed raw". The gate's
own comment stated the problem: *"Twig also types untyped params `any` and
shares this pass, so the `any`-param heuristic alone would mis-flag a Twig
`(define (fib n) …)` as lisp and corrupt it."*

The frontends now say which they mean. McCarthy stamps `ref<any>` — already the
unambiguous "boxed dynamic value" in every language — and bare `any` means raw
everywhere. `is_boxed(hint)` is `hint == "ref<any>"`, and `closure_heap` reads
each lambda body's own declared parameter types instead of a module-wide flag.

### The gate was load-bearing for a wrong rule

Removing it exposed that `lisp_functions` seeded the "callers must box for this
callee" set partly from **heap ops in the body**. A body that allocates tells you
the function uses the heap; it tells you nothing about the ABI its callers must
satisfy. A Twig union constructor `(union Opt (Some (v : int)) …)` allocates a
cons cell internally while taking its argument raw, so `call Some(42)` boxed the
42 and the `match` that extracted it returned the tagged word — the program
exited `(42 << 3) & 0xFF = 80` instead of 42.

The language gate had hidden this by forcing the set empty for every non-lisp
module. New `tagged_boundary_functions` seeds from declared parameter types AND
return types; `lisp_functions` keeps the heap-body clause for the structural work
that wants it.

### Three more found in security review, each exposed by fixing the last

The reviewer built both revisions, emitted all 294 matrix-corpus programs from
each, and ran them — finding a silent wrong answer the corpus does not contain.

* **Nullary tagged functions were missed.** Seeding only from parameters leaves
  `((LAMBDA () (ATOM 7)))` outside the set, so the entry coerced its result with
  the static `dyn_unbox_int` (`>> 3`) instead of the runtime tag switch. `#t` is
  the whole word `0b101`, and `5 >> 3 = 0`: the program reported FALSE for a
  true predicate, on native-AOT and LLVM, with no diagnostic. A boundary has two
  sides; the declared return type is the other one.
* **A tagged function never boxed a raw scalar return** (pre-existing). The
  managed pass has always done this; the tagged pass never grew the counterpart.
  `((LAMBDA () 5))` exited 1 — `5` is `0b101`, read as the `#t` tag. It stayed
  hidden because the old coercion was also wrong, just differently.
* **…which then double-boxed a returned parameter.** A tagged *parameter* is
  already a `LispyValue`, and `boxed_regs` is built from instructions so never
  contains one. `((LAMBDA (X) X) 5)` exited `5 << 3 = 40`.

## 0.36.0 - 2026-08-13 - drop the name-carrying `const` once its consumer is lowered

`const %n1 = Operand::Var("g")` is not a real instruction: it is how the
twig-ir-compiler smuggles a *string literal* to the `call_builtin` that follows.
`lower_global_io` reads it, folds the name into `global_load Str("g")` — and
then pushed the now-consumerless `const` through untouched, because pass 2
filters on `op != "call_builtin"`.

Native, LLVM, the VM and the JIT tolerated the leftover. The JVM, CLR and WASM
backends read it literally, saw a `const` whose source names a variable, and
refused the whole module:

    JVM   "const instruction has a Var source — use load_reg instead"
    CLR   "const expects an integer literal, got Some(Var(\"g\"))"

So **every Twig program that reads a module-level global** failed on those three
backends. The matrix names its case a "forward-referenced global", which is a
red herring — declaration order is irrelevant; `(define g 42) (define (f) g) (f)`
and `(define (f) g) (define g 42) (f)` produce identical IIR and both failed.

Only *unreferenced* name carriers are dropped. One that some instruction still
mentions is still doing work — a dynamic global this pass could not resolve, or
a symbol a later pass will consume — and removing it would turn a compile error
into a dangling reference.

### Three issues found in security review, fixed before merge

* **`source_map` lockstep.** Passes 1 and 2 are strictly 1-to-1, so pass 3 is
  the first thing here that changes the instruction count — and it did not
  touch the map, shifting every later line attribution by one and accumulating
  per global access. Nothing downstream would have caught it: `aot-debug` walks
  `min(len, len)` and `iir-coverage` bounds-checks against the *instructions*,
  so both silently misattribute rather than panic, and on the managed path
  `materialize_immediate_operands` later rebuilds the map at the correct
  *length*, laundering the wrong content past any length assertion. This is the
  same lockstep bug that pass had already fixed for itself; a bug class fixed in
  one pass is not fixed in the next one.
* **Liveness ignored `Operand::Str`.** `closure.rs` deliberately counts both
  `Var` and `Str` when deciding whether a const-string register is dead. Nothing
  today re-encodes a live register as `Str`, but the failure mode if something
  ever does is a dangling reference, and over-retention is the safe direction.
* **The carriers polluted their own liveness set.** Each contributed its literal
  payload — the global's *name* — to a set consulted as if it held register
  names, so a name colliding with a register would read as live and revive the
  backend rejection. They are now excluded from the scan that judges them.

Verified by running: matrix cell #20 now passes on LLVM, JVM and CLR, and the
three-program probe agrees with the VM oracle (42, 42, 8) on every backend.


## 0.35.0 - 2026-08-13 - materialize immediate value operands for the stack backends

New pass `immediates::materialize_immediate_operands`, wired into the JVM, CLR
and WASM pipelines in `lang-aot`.

An IIR source operand is *either* a variable name or an immediate literal — the
`Operand` type says so, and COBOL's `ADD 7 TO R` takes it at its word, lowering
to `add _acc0, 7` with the literal inline. The native, LLVM, VM and JIT backends
honour that. The three stack backends do not: their lowering assumes every value
operand names a slot to `iload`/`ldloc`/`local.get`, so a literal makes them
refuse the whole module.

    JVM   InvalidOperand { detail: "add expects Var operands, got immediate" }
    CLR   InvalidOperand { detail: "cmp_eq src[1] must be a variable, got Some(Int(1))" }
    WASM  InvalidOperand { detail: "expected Var at src[1], got Int(1)" }

That cost 24 matrix cells, and would have cost more as frontends multiply —
each one learning by breaking which half of the contract is real.

Teaching the three backends to fold literals is the other way to close it, and
is arguably what the contract asks for; it is also the same work three times, in
three instruction encoders, for every opcode family, and it leaves the next
backend to make the same choice again. Normalizing is one pass, deterministic
and language-agnostic: `add _acc0, 7` becomes `const 7 → __imm1` + `add _acc0,
__imm1`, a shape the stack backends already lower. The backends that implement
the full contract keep folding literals inline and emit the tighter code.

Two things the pass is careful about:

* **Only value operands.** An immediate that is part of an instruction's
  addressing — `field_load`'s field index, `jmp`'s target label,
  `call_builtin`'s builtin name, `const`'s own source — is not a value, and
  rewriting it changes what the instruction means. The pass is opt-in per opcode
  family (`is_arithmetic`, `is_bitwise`, `is_cmp`, `mov`) rather than a blanket
  rewrite of every `Operand::Int` it can reach.
* **Operand type, not result type.** A comparison's `type_hint` is `bool`, which
  describes its result while the operands are what is being compared; emitting
  `const 1 : bool` for `cmp_eq x, 1` would hand the backend a boolean where it
  wants an integer — the same operand-width-versus-result-width confusion that
  once made BASIC's comparisons lower to `icmp i1`. The literal's own kind is
  used instead, preferring a sibling variable's concrete producer type when
  there is one so the width matches what the surrounding code agreed on.

Runs last in each pipeline, after every other pass has finished rewriting
instructions — an earlier position would miss the operands its successors
introduce. A no-op for a module whose frontend already materialized everything,
which is every language that was already green.

### Four issues found in security review, fixed before merge

* **Silent numeric truncation.** The sibling-width rule adopted a narrower type
  with no representability check, and every backend's `const` lowering narrows
  with a lossy `as` cast rather than a checked conversion — so `add x:i32,
  5_000_000_000` would have compiled quietly to `705032704`. Newly reachable
  *because* of this pass: before it, that shape was rejected outright, which was
  at least loud. A candidate type is now adopted only if the literal fits it.
* **Unary ops got the wrong width.** `mov`/`neg`/`not` have no sibling, so they
  fell back to `i64` — putting a two-slot `long` local under an `i32`-model
  function's `INEG`, which the JVM verifier rejects. The "never use
  `type_hint`" rule exists only because a comparison's hint is its `bool`
  *result*; for the unary families the hint IS the operand type, so it is used.
* **Generated names could shadow existing ones.** The monotonic counter kept the
  temporaries distinct from each other but consulted nothing already in the
  function; a frontend variable named `__imm1_add` would have been silently
  overwritten. Names in use are now collected and avoided.
* **`source_map` lockstep.** `IIRFunction::source_map` is documented as indexed
  in lockstep with `instructions`; inserting instructions without extending it
  shifted every later entry. The map is now rebuilt alongside, with each
  materialized `const` attributed to the statement whose operand it is.

Sixteen unit tests cover both operand positions, two-literal instructions, the
comparison typing rule, sibling-width inference, float/bool literals, the
addressing-immediate exclusions, the no-op case, and one per security finding.

## 0.34.0 - 2026-08-12 - closure calling convention + chained dynamic arithmetic

Two confirmed miscompiles, both instances of the same bug class as 0.32.0 and
0.33.0: **a value's recorded type did not describe what the value actually is
after lowering.** Both reproduced only on the tagged-word backends (native-AOT
and LLVM), because a backend whose `box` is the identity cannot tell the two
representations apart — which is why the generic VM computed the right answer
and agreed with nobody.

### `closure_heap` — the cons chain held two representations at once

A closure's captures and arguments travel through a cons chain, and a cons cell
holds tagged `DynValue`s, so a raw-model (Twig/Nib) value must be boxed on the
way in. That boxing was left to `dyn_repr`, which boxes what it can *prove* is
raw: it proved a captured literal (`41` went in as the tagged word `328`) and
could not prove a captured bare-`any` parameter (which went in untagged). The
chain then held both forms and no single extraction rule was right for both.

Symptoms, all silent wrong answers rather than crashes:

* `((lambda (x) (+ x 1)) 41)` exited **73** — `(41 << 3) + 1 = 329`, `& 0xFF`.
* `(((lambda (x) (lambda (y) (+ x y))) 40) 2)` exited **80**.
* `(((lambda (a b) (lambda (c) (+ (+ a b) c))) 10 20) 12)` exited **252**.

`lower_closures_to_heap` now owns *both* ends of the representation, gated on
the same `is_lisp_language` test `dyn_repr`/`dynamic_arith` use for the same
ambiguity: for a raw-model module it boxes every capture and argument into the
chain explicitly and unboxes every value the dispatcher pulls back out. A lisp
module is unchanged — its bodies genuinely take and return tagged words, so
nothing is inserted.

### `dynamic_arith` — the pass misread its own output

`producer_types` was seeded once from the incoming instructions and never
updated, but every dynamic op this pass rewrites ends in `box … : ref<any>`.
So a *nested* expression classified the inner result by its pre-pass hint — the
frontend's bare `any`, which for a non-lisp module means raw — and added the
tagged word directly. `(+ (+ a b) c)` with `a,b,c = 10,20,12` computed
`(30 << 3) + 12 = 252`.

The map is now updated as the rewrite proceeds, so a later operand in the same
function sees the boxed type its producer actually has.

### The invariant the uniform shift relies on

`box`/`unbox` are `<< 3`/`>> 3`, so a *value* round-trips for anything under 61
bits — a heap address included. But a cons cell is **traced by the collector**,
and a heap handle is `addr | 0b111`; shifted left by 3 it resolves to no live
block under either interpretation the precise-kind scan applies. The collector
would stop tracing through the chain while the chain is the only thing holding
the referent.

So this representation is sound only while every raw-model capture and argument
is a non-pointer — true of every closure program today, and the interim contract
until closure lowering grows tag-directed extraction. `store` now asserts it
rather than describing it: capturing a string, a cons, or another closure from a
raw-model closure fires — in release as well as debug, since what it guards is a
memory-safety property of the *generated* code — instead of silently producing a
dangling reference. The predicate covers `ref<…>`, `closure` and `str`; bare
`any` remains indistinguishable from a machine integer and is the residual hole
that tag-directed extraction closes. (Found in security review, in two rounds:
the first predicate tested only `ref<…>` and missed the `closure` and `str`
hints a Twig frontend actually stamps on pointer destinations.)

### Tests

Three new unit tests pin the invariants: chained dynamic arithmetic emits
exactly one unbox and it consumes the inner result; a raw-model closure boxes
into the chain and unboxes out of it; a lisp-model closure does neither.

## 0.33.0 - 2026-08-12 - `dyn_car`/`dyn_cons`/`dyn_cdr` retype fix (boxed-`car` arithmetic miscompiled)

Fixes a second, closely-related confirmed bug: `(+ (car (cons 41 0)) 1)` returned
`329` instead of `42` on native x86-64 AND LLVM Windows builds — the `+` ran
directly on the *boxed* (tagged, `<<3`'d) `car` result instead of unboxing it
first.

Root cause: `heap.rs::lower_heap_function_runtime` renames `car`/`cons`/`cdr`/
`pair?`/`equal?`/`null?` to their `dyn_*` runtime-call form but never updated
the instruction's `type_hint` — it stayed at whatever bare `"any"` the
frontend originally gave it. That bare `"any"` then reached `dynamic_arith.rs`'s
`is_boxed` (fixed in 0.32.0 to only treat bare `"any"` as boxed for lisp
modules, correctly closing the *parameter*-comparison bug that release fixed)
— so a genuinely-boxed `dyn_car` result was now *also* misclassified as
unboxed for Twig, the exact same symptom via a different producer. `ref<any>`
(what a `dyn_car` result should have been typed) is treated as unconditionally
boxed regardless of language, so this was never affected by the 0.32.0 gate
itself — it's a gap that predates it, just newly visible once the parameter
case stopped masking it as "boxed-by-default."

Fix: `lower_heap_function_runtime` now stamps every `RUNTIME_RENAMES` result
`"ref<any>"` — matching `dynamic_arith.rs`'s own doc comment, which already
described `ref<any>` as "a heap-typed dynamic value ... never a placeholder,"
it just was never actually stamped that way. New regression test
`runtime_renames_retype_result_to_ref_any`; existing
`runtime_car_and_cdr_are_renamed` was previously silent on `type_hint`.

Verified against three independent oracles per the fix (not just a hardcoded
literal): native Windows x86-64 AOT, LLVM/clang, and `vm-core` (the shared
cross-language interpreter, structurally immune to this bug since it never
runs this lowering pass at all) all now agree `(+ (car (cons 41 0)) 1)` = 42
— see `lang-aot` 0.222.0's `e6d2b_dynamic_arith.rs`.

## 0.32.0 - 2026-08-12 - `lower_dynamic_arith` language-gate fix (bare-`any` Twig parameters miscompiled)

Fixes a confirmed correctness bug: any Twig comparison or arithmetic op with a
function **parameter** as an operand (e.g. `(define (classify n) (if (< n 2)
111 222))`) silently miscompiled on every native-AOT/LLVM backend. `n=10`
compared `< 2` should be false, but native AOT printed as if it were true.

Root cause: `dynamic_arith.rs`'s `is_boxed(hint)` treated the bare string
`"any"` as always denoting a boxed/tagged dynamic value, unconditionally
inserting an `unbox` (`>> 3`) before the typed op. That's correct for
McCarthy Lisp — whose `any`-typed parameters genuinely are tagged
`LispyValue`s — but wrong for Twig, where every untyped function parameter is
also declared `any` yet passed as a **raw, unboxed** machine `i64`. The
`unbox` corrupted the parameter's value before the comparison ever ran.
`dyn_repr.rs` had already solved this exact `ref<any>` vs. bare-`any`
ambiguity correctly, gating on the module's source language
(`is_lisp_language`) — `dynamic_arith.rs` just never got the same gate. A
comment already in the codebase (`twig-ir-compiler/src/compiler.rs:2524-2528`,
and `lang-aot/tests/e6d2b_dynamic_arith.rs`) shows this was a known footgun
worked around at individual call sites, not fixed at the root.

Fix: `is_lisp_language` is now `pub(crate)` in `dyn_repr.rs` and reused by
`dynamic_arith.rs`. `is_boxed(hint, is_lisp)` now only treats bare `any` as
boxed when `is_lisp` is true (`ref<any>` stays unconditionally boxed in every
language — it's genuine heap-op provenance, e.g. a `car`/`cdr` result, never a
placeholder). `lower_dynamic_arith(module)` derives `is_lisp` once from
`module.language`; `lower_dynamic_arith_function` takes it as a new parameter.
No public API break: `lower_dynamic_arith`'s signature is unchanged, so none
of its 7 call sites (across `twig-aot` and `lang-aot`) needed updates.

Two new unit tests (`twig_any_param_operand_is_not_unboxed`,
`mccarthy_any_param_operand_is_still_unboxed`) lock in both directions of the
fix. Verified end-to-end: `(define (classify n) (if (< n 2) 111 222))
(classify 10)` now correctly exits `222` on native Windows x86-64 AOT (was
`111`) — see `lang-aot` 0.221.0 and `twig-aot` 0.51.0 for the accompanying
integration test and the Windows-linker fixes that let it actually run there.

## 0.31.0 - 2026-07-20 — null? runtime lowering + list-ref/assoc index boxing + predicate exit-coercion (native/LLVM lisp fixes)

Part of the fix restoring McCarthy-lisp list programs on the native-AOT / LLVM backends (`lang-aot` `lang_matrix`). See the umbrella commit for the full story: `null?` was never routed to a runtime call on the tagged native/LLVM path (breaking every cons-walk helper), `list-ref`/`assoc` unboxed a raw-int index/key (→ wrong element), a top-level `(null? …)` predicate result was unboxed instead of truthy-coerced, and cons-cell field access failed the JVM verifier. Verified end-to-end: native list-ref/assoc/length/reverse/append/null? all correct.
## 0.30.0 - 2026-07-14 (E6d-7a: closures -> cons-heap + synthesized dispatcher)

New pass `lower_closures_to_heap` (closure_heap.rs): lowers `alloc_closure`/`call_closure` entirely at the IIR level for the backends that lack a native closure model (WASM + NativeAot; LLVM is a follow-up). A closure becomes a cons-chain `(box(dispatch_index) . (caps...))`; `call_closure` boxes its args into a second chain and calls a synthesized `__dyn_call_closure` dispatcher — a chain of dynamic `=` index tests (the proven E6d-6 match/union tag pattern) over statically-known lambda bodies, each a direct `call` threading captures ++ args. Reuses only `cons`/`car`/`cdr`/`=`/`call`/`jmp_if_false` (no new backend codegen; no `call_indirect`/funcref). Dispatch indices are assigned alphabetically (deterministic). Unit-tested; run-verified on WASM + native (exit 42).

## 0.29.0 - 2026-07-13 (E6d-6: boxed-bool `jmp_if_false` branches on the raw bool)

Both dynamic-representation passes now recognise a `jmp_if_false` whose guard is a
**boxed machine bool** — a boxed comparison result (`= tag …` in a Twig `match`,
or any `(if (= a b) …)` forced onto the dynamic path) — and branch on its RAW
pre-box `bool`, instead of applying McCarthy nil-truthiness.

Why: `lower_dynamic_arith` boxes every comparison result (`cmp_eq → bool` then
`box → ref<any>`). Both passes then saw a `ref<any>`/tagged condition and wrapped
it — the structural pass (WASM/JVM/CLR/BEAM) as `not(is_null(cond))`, the native
pass (NativeAot/LLVM) as `dyn_truthy(cond)`. But a boxed `#f` is a **non-nil**
value (`ref.i31(0)` on the structural side; `dyn_box_int(0)` = tagged integer 0 on
the native side), and nil-truthiness / McCarthy-truthiness both read it as **true**
— so every `match` arm's tag test passed and dispatch was wrong (E6d-6).

Fix: in `lower_dyn_repr_structural` (new `boxed_bool_source`/`boxed_bool_conditions`
helpers) and in `lower_dyn_repr`'s `wrap_tagged_conditions`, a guard that is a
`box` of a `"bool"`-typed value is repointed to the raw source and the jump is
emitted unwrapped. General fix — helps any comparison-as-branch-condition on the
dynamic path, not only `match`. One new unit test; 149 lib tests pass.

## 0.28.0 - 2026-07-13 (E6d-3b COMPLETE: `assoc` via a synthesized alist-search helper)

`lower_list_ops` gains its fifth and final list operation, `assoc` — completing
the E6d-3b list-operation set (`length`, `list-ref`, `append`, `reverse`,
`assoc`). It rewrites `call_builtin "assoc" key alist` to a call to a synthesized
recursive helper `__dyn_list_assoc` (injected once) that searches an association
list (a list of `(k . v)` cons pairs):

```
__dyn_list_assoc(key, alist) = if null?(alist) then nil
    else if key == car(car(alist)) then car(alist)
    else __dyn_list_assoc(key, cdr(alist))
```

The key comparison must yield a raw machine bool for `jmp_if_false`. Since the
`equal?` builtin lowers unevenly across the managed (`equal?`) and native
(`dyn_equal`) paths and its result is not a plain branch bool, V1 `assoc`
**unboxes both keys to `i64` and compares with a typed `cmp_eq`** — the exact
technique `list-ref` uses for its index. That scopes V1 `assoc` to **integer
keys** (every E6d-3b proof uses integer atoms); symbol keys arrive with E6d-4
(interned symbols → `eq?` bit-equality). The alist cells, the returned pair, and
`nil` are all references, so no boxing on the value path. Five new unit tests
(rewrite, single injection, pair-search shape with 2 unboxes / 2 branches / 2
cars / 3 rets, all-five-ops coexistence).

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
