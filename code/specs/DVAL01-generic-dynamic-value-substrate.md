# DVAL01 — a generic dynamic-value primitive substrate

**Status:** Draft — 2026-07-11 (spec-first; sign-off = merge)
**Supersedes the branding of:** `lang-full-e6-dispatch.md` §2 (the `lispy_*` catalog)

## 0. One-paragraph summary

The AOT chain already runs a full **tagged dynamic-value** model — a 64-bit word
that is an integer, `nil`, a boolean, an interned symbol, or a heap pointer
(pair / record / closure / string) — on all five code-gen backends. But that
substrate is **named for one language**: the runtime is `__twig_lispy_*`, the IIR
builtins are `lispy_cons` / `lispy_box_int` / …, and the passes are `lisp_repr`.
Nothing about a tagged int, a heap pair, `box`/`unbox`, truthiness, or exit-code
coercion is lisp-specific — Python, Ruby, JavaScript, and Scheme all need exactly
these. This spec **renames the substrate to a language-neutral `dyn` layer** and
**decouples the passes from any hardcoded builtin set**, so *any* dynamic frontend
lowers its values to the same primitives. It is a de-tunnel, not a rewrite: the
tag encodings, the ABI, and the runtime behaviour are unchanged — only names and
one coupling point move.

## 1. Goal & non-goals

### 1.1 Goal

A **generic dynamic-value (`DynValue`) primitive substrate** that:

1. Any dynamic-language frontend targets — the primitives carry **no language
   name**.
2. Keeps the existing tag layout and runtime behaviour **byte-for-byte** (this is
   a rename + one decoupling, so every backend stays green with no value-model
   change).
3. Makes the boxing/exit-boundary logic **producer-agnostic**: a register holds a
   `DynValue` because of *what it is*, not because its producer's name is on a
   lisp allow-list — which is the concrete generality that unblocks dynamic
   arithmetic (E6d-2b) and every future dynamic op.

### 1.2 Non-goals (explicit follow-ups)

- **New tag kinds / float boxing.** The integer/bool/nil/symbol/heap contract is
  unchanged; NaN-boxed doubles are a later layer.
- **A second frontend.** This spec makes the substrate *ready* for Python/Ruby/JS
  but ships only the Twig/lisp frontend that already rides it. Proving a second
  frontend on the neutral primitives is a follow-up.
- **Changing the structural value model.** WASM `anyref`/`i31ref`, JVM `Object`/
  `Integer`, CLR `object`/boxed-int32 stay as they are.

## 2. Current state (surveyed)

### 2.1 The value model is already generic — only the names aren't

`lispy-runtime` (the golden reference) + `twig-aot/runtime/lispy_runtime.c` define
a 64-bit word with a 3-bit low tag:

| tag `0b` | kind    | encoding            | decode            |
|----------|---------|---------------------|-------------------|
| `000`    | integer | `n << 3`            | arithmetic `>> 3` |
| `001`    | nil     | whole word `1`      | —                 |
| `010`    | symbol  | `(id << 32) | 010`  | `x >> 32`         |
| `011`    | false   | whole word `3`      | —                 |
| `101`    | true    | whole word `5`      | —                 |
| `111`    | heap    | pointer `| 0b111`   | `x & ~7`          |

This is a textbook tagged dynamic value. It is the **native / LLVM** world's
representation; the **structural** backends (WASM/JVM/CLR) represent the same
`DynValue` as `anyref` / `Object` / `object` with `i31ref` / `Integer` /
boxed-int32 atoms.

### 2.2 Where the lisp branding lives (the rename surface)

- **Runtime C ABI** (`twig-aot/runtime/lispy_runtime.c`): `__twig_lispy_box_int`,
  `_unbox_int`, `_cons`, `_car`, `_cdr`, `_pair_p`, `_equal`, `_not`, `_nil`,
  `_make_symbol`, `_truthy`, `_to_exit_code`, `_tag_*`.
- **Golden reference crate** `lispy-runtime` (the Rust mirror of the tag layout,
  used by `twig-vm`).
- **IIR builtin names**: `lispy_cons` / `lispy_car` / `lispy_cdr` /
  `lispy_box_int` / `lispy_unbox_int` / `lispy_pair_p` / `lispy_equal` /
  `lispy_not` / `lispy_nil` / `lispy_truthy` / `lispy_to_exit_code`, wired in
  `iir-builtin-lowering` (`heap.rs` `RUNTIME_RENAMES`, `lisp_repr.rs`), the
  `iir-to-llvm` `LISPY_BUILTINS` table, and the native `V1_BUILTINS`
  (`aarch64-backend` / `x86_64-backend`).
- **Passes**: `lisp_repr.rs` / `lisp_repr_structural.rs` /
  `symbol_intern.rs`.

The **structural backends' generic ops** — `box` / `unbox` / `alloc` /
`field_load` / `field_store` / `is_null` — are **already language-neutral** and
are the model the rename converges the native world onto.

### 2.3 The coupling that must be decoupled (not just renamed)

`lisp_repr.rs` decides which registers hold a `DynValue` (and so which integer
consts to box, and which `ret` to unbox at the program exit) by seeding from a
**hardcoded lisp-builtin set** (`lispy_cons`/`car`/`cdr`/`pair_p`/`not`/`equal`).
A value produced by a *different* dynamic primitive — e.g. a boxed arithmetic
result (`box_int`) — is not recognised, so the exit-unbox is skipped and the
program returns a **tagged** word instead of the machine exit code. (This is the
exact failure the dynamic-arithmetic slice hit.) The fix (§3.3) is to classify a
`DynValue`-producing register by the **op's declared result type** (`any` /
`ref<any>`), not by a builtin name — the same producer-agnostic rule the
structural pass already uses.

## 3. Design

### 3.1 The neutral catalog

Rename every lisp-branded primitive to a `dyn`-prefixed, language-neutral name.
Behaviour, arity, and ABI are identical:

| today (`lispy_*` / `__twig_lispy_*`) | neutral (`dyn_*` / `__dyn_*`) | meaning |
|---|---|---|
| `box_int` / `unbox_int`   | `dyn_box_int` / `dyn_unbox_int`   | int ⇄ tagged word |
| `cons` / `car` / `cdr`    | `dyn_cons` / `dyn_car` / `dyn_cdr` | 2-field heap pair |
| `pair_p`                  | `dyn_pair_p`                      | heap-tag test |
| `equal` / `not`           | `dyn_equal` / `dyn_not`          | value eq / logical not |
| `nil` / `make_symbol`     | `dyn_nil` / `dyn_make_symbol`     | nil / interned symbol |
| `truthy`                  | `dyn_truthy`                      | tagged → raw 0/1 |
| `to_exit_code`            | `dyn_to_exit_code`               | tagged → process exit int |
| `tag_*`                   | `dyn_tag_*`                       | tag constants (test ABI) |

> **Implementation note (DVAL01-2, revised from the draft).** The rename is
> **prefix-preserving** — `pair_p → dyn_pair_p`, `equal → dyn_equal` (the draft
> proposed the semantic renames `dyn_is_pair` / `dyn_eq`). Reason: DVAL01-1a
> already shipped the C runtime symbols as `__dyn_pair_p` / `__dyn_equal`, so
> keeping the IIR name's suffix identical means the name maps to its runtime
> symbol by a trivial `__`-prefix rule (`dyn_pair_p` → `__dyn_pair_p`) with no
> divergent lookup table. The `is_pair`/`eq` beautification, if ever wanted, is a
> separate follow-up that would also re-mangle the runtime symbols.

> **Native emit fix (DVAL01-2).** DVAL01-1a renamed the C runtime symbols to
> `__dyn_*` and updated the tests, but the aarch64/x86_64 `call_builtin` emit
> still hard-coded `__twig_<name>` for *every* helper — so the tagged-value
> builtins emitted `__twig_lispy_cons`, a symbol the runtime does not export.
> Real programs were unaffected (they lower cons/car via the structural `alloc`
> path, not `call_builtin`), so the matrix stayed green while 4+4 direct-call
> unit tests sat red. DVAL01-2 routes `dyn_*` names to `__<name>` (= `__dyn_cons`)
> and everything else to `__twig_<name>`, aligning native with the runtime + the
> LLVM `DYN_BUILTINS` table and greening those tests.

The generic **IIR ops** (`box`/`unbox`/`alloc`/`field_*`/`is_null`) are already
neutral and stay. Whether a target uses the *ops* (structural) or the *runtime
calls* (native/LLVM) for box/unbox is a per-backend representation choice
(mirroring `dyn_repr_structural` vs `dyn_repr`), not a language choice.

### 3.2 Runtime & crate re-homing

- `twig-aot/runtime/lispy_runtime.c` → `dynval_runtime.c`, symbols `__dyn_*`.
  (`twig_gc.c` / `twig_runtime.c` stay — GC and I/O are already neutral.)
- Rust golden crate `lispy-runtime` → `dynval-runtime` (same tag mirror + Miri
  golden tests). **`twig-vm` depends on the unsafe here** — per project policy the
  local `code/scripts/miri-twig-vm.sh` runs before pushing any PR that touches it.
- Passes `lisp_repr*` → `dyn_repr*`, `symbol_intern` stays (already neutral).

Per "break compat freely, clean restructures over shims" — a straight rename, no
aliases.

### 3.3 Producer-agnostic `DynValue` classification

`dyn_repr` (ex-`lisp_repr`) seeds its "boxed register" set from **any op whose
result type is `any`/`ref<any>`** (a `DynValue`) — `dyn_cons`/`car`/`box_int`/a
user lambda call/… — plus the nil/symbol consts, then runs the existing
`mov`-fixpoint. The exit-unbox then fires for a `ret` of *any* `DynValue`,
regardless of which primitive produced it. This is the one behavioural change and
it is a **generalisation** (a strict superset of today's seeds), so existing lisp
programs are unaffected.

## 4. PR breakdown (dependency-ordered; each small, run-verified, all 5 backends green)

1. **DVAL01-0 — this spec.** ✅ merged.
2. **DVAL01-1 — rename the native runtime + ABI.** ✅ merged (as -1a `__dyn_*`
   symbols, -1b `dynval_runtime.c` file, -1c `dynval-runtime` crate).
   `lispy_runtime.c` → `dynval_runtime.c`, `__twig_lispy_*` → `__dyn_*`;
   `lispy-runtime` crate → `dynval-runtime`.
3. **DVAL01-2 — rename the IIR builtin names + passes.** ✅ **done.** `lispy_*` →
   `dyn_*` (prefix-preserving — see §3.1 note) in `heap.rs` `RUNTIME_RENAMES`, the
   `iir-to-llvm` table (`LISPY_BUILTINS` → `DYN_BUILTINS`), the native
   `V1_BUILTINS` (aarch64/x86_64), `iir-to-cil`, the `dynval-runtime` ABI export
   symbols + `LispyBinding` registrations, and the VM dispatch (twig-vm,
   mccarthy-lisp-vm); `lisp_repr*` → `dyn_repr*` passes (files + `lower_*` fns);
   `lang-aot` wiring + tests. **Also fixed** the latent native emit bug (§3.1
   native-emit note) so `dyn_*` builtins target `__dyn_*`. Pure rename; the lisp
   cells stay green across VM/JIT/LLVM/native and cross-backend agreement holds.
4. **DVAL01-3 — producer-agnostic classification (§3.3).** ✅ **done.** `dyn_repr`
   now seeds its `DynValue` set from **any op whose result type is `any`/
   `ref<any>`** (gated on `is_lisp`, since Twig/Nib use `any` as a pre-resolution
   placeholder), not from the builtin allow-list. Unit-tested: a `dyn_box_int`
   result is exit-unboxed; a Twig `any` module is a no-op. Strict superset —
   existing lisp programs unaffected.
5. **Resume E6 on the neutral substrate** — ✅ **E6d-2b done**: dynamic
   arithmetic on native/LLVM. `lower_box_unbox_to_runtime_calls` rewrites the
   generic `box`/`unbox` ops to `dyn_box_int`/`dyn_unbox_int` runtime calls for
   the tagged-i64 world; §3.3's classification (refined so `ref<any>` seeds
   ungated) exit-unboxes the result even for Twig. `(+ (car (cons 41 0)) 1)` → 42
   on all 5 code-gen backends, run-verified native + LLVM. **Next**: lists /
   symbols / records / unions / closures / dynamic globals
   (`lang-full-e6-dispatch.md` §4, retargeted to `dyn_*`).

Rationale: renames first (mechanical, low-risk, keep every backend green), then
the one real generalisation (§3.3), then the feature work rides a clean substrate
that a future Python/Ruby/JS frontend can target unchanged.

## 5. Verification

Every rename PR is proven by the existing `lang-aot` matrix staying green on all
five code-gen backends (the tag ABI is unchanged, so a correct rename is a no-op
in behaviour) plus the `dynval-runtime` Miri golden tests and, for any twig-vm
-touching change, `code/scripts/miri-twig-vm.sh`. The §3.3 generalisation adds a unit
test (a boxed non-cons `DynValue` is exit-unboxed) and is proven end-to-end by the
resumed dynamic-arithmetic cell reaching NativeAot + LLVM.

## 6. Out of scope (later)

Float/double boxing, a bignum heap kind, a second concrete frontend
(Python/Ruby/JS/Scheme) lowered onto `dyn_*`, and the non-arithmetic E6 layer-2
slices (their design stays in `lang-full-e6-dispatch.md`, retargeted to the
neutral names by DVAL01-2).
