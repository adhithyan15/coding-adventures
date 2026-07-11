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
| `pair_p`                  | `dyn_is_pair`                     | heap-tag test |
| `equal` / `not`           | `dyn_eq` / `dyn_not`              | value eq / logical not |
| `nil` / `make_symbol`     | `dyn_nil` / `dyn_make_symbol`     | nil / interned symbol |
| `truthy`                  | `dyn_truthy`                      | tagged → raw 0/1 |
| `to_exit_code`            | `dyn_to_exit_code`               | tagged → process exit int |
| `tag_*`                   | `dyn_tag_*`                       | tag constants (test ABI) |

The generic **IIR ops** (`box`/`unbox`/`alloc`/`field_*`/`is_null`) are already
neutral and stay. Whether a target uses the *ops* (structural) or the *runtime
calls* (native/LLVM) for box/unbox is a per-backend representation choice
(mirroring `lisp_repr_structural` vs `lisp_repr`), not a language choice.

### 3.2 Runtime & crate re-homing

- `twig-aot/runtime/lispy_runtime.c` → `dynval_runtime.c`, symbols `__dyn_*`.
  (`twig_gc.c` / `twig_runtime.c` stay — GC and I/O are already neutral.)
- Rust golden crate `lispy-runtime` → `dynval-runtime` (same tag mirror + Miri
  golden tests). **`twig-vm` depends on the unsafe here** — per project policy the
  local `scripts/miri-twig-vm.sh` runs before pushing any PR that touches it.
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

1. **DVAL01-0 — this spec.** Sign-off = merge.
2. **DVAL01-1 — rename the native runtime + ABI.** `lispy_runtime.c` →
   `dynval_runtime.c`, `__twig_lispy_*` → `__dyn_*`; `lispy-runtime` crate →
   `dynval-runtime`; update `build.rs`, the golden Miri tests, and the
   `iir-to-llvm` `LISPY_BUILTINS` / native `V1_BUILTINS` tables. Pure rename;
   full matrix green + `scripts/miri-twig-vm.sh`.
3. **DVAL01-2 — rename the IIR builtin names + passes.** `lispy_*` → `dyn_*` in
   `heap.rs` `RUNTIME_RENAMES`, `lisp_repr*` → `dyn_repr*`; update `lang-aot`
   wiring. Pure rename; matrix green.
4. **DVAL01-3 — producer-agnostic classification (§3.3).** Decouple `dyn_repr`'s
   `DynValue` seed set from the builtin allow-list; unit-test that a `box_int`
   result is exit-unboxed. Matrix green (superset behaviour).
5. **Resume E6 on the neutral substrate** — dynamic arithmetic on native/LLVM
   (the ex-E6d-2b, now trivially correct once #3.3 lands + `dyn_box_int`/
   `_unbox_int` are the emitted primitives), then lists / symbols / records /
   unions / closures / dynamic globals (`lang-full-e6-dispatch.md` §4, retargeted
   to `dyn_*`).

Rationale: renames first (mechanical, low-risk, keep every backend green), then
the one real generalisation (§3.3), then the feature work rides a clean substrate
that a future Python/Ruby/JS frontend can target unchanged.

## 5. Verification

Every rename PR is proven by the existing `lang-aot` matrix staying green on all
five code-gen backends (the tag ABI is unchanged, so a correct rename is a no-op
in behaviour) plus the `dynval-runtime` Miri golden tests and, for any twig-vm
-touching change, `scripts/miri-twig-vm.sh`. The §3.3 generalisation adds a unit
test (a boxed non-cons `DynValue` is exit-unboxed) and is proven end-to-end by the
resumed dynamic-arithmetic cell reaching NativeAot + LLVM.

## 6. Out of scope (later)

Float/double boxing, a bignum heap kind, a second concrete frontend
(Python/Ruby/JS/Scheme) lowered onto `dyn_*`, and the non-arithmetic E6 layer-2
slices (their design stays in `lang-full-e6-dispatch.md`, retargeted to the
neutral names by DVAL01-2).
