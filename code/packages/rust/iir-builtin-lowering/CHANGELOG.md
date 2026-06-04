# Changelog — iir-builtin-lowering

All notable changes to this crate are documented here.

---

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
