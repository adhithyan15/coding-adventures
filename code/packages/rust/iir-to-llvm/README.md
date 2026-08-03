# iir-to-llvm

IIR → textual LLVM IR backend.  Emits a `.ll` source string for an LLVM
target triple, without depending on `llvm-sys` or a native LLVM install.

**Status: v0.44.0 — LANG-FULL E4 runtime string ordering.**  The backend now
lowers scalar control/data ops, Brainfuck byte-tape I/O, arrays, globals,
numeric conversions, and the `str_const` + `print_str` literal-output slice.
It also materialises `str_len`, `str_index`, `str_eq`, `str_cmp`, and literal
`str_concat` for direct literals from compile-time metadata, including derived concat
constants that feed `print_str` and `str_len`-computed indexes that feed
`str_index`. A non-literal `str_cmp` calls the shared length-prefixed runtime helper,
so procedure results and branch-selected locals can drive lexical branches. Other
dynamic byte-string ops remain outside this release.

## Where it fits

| Backend                       | Target                                |
|-------------------------------|---------------------------------------|
| `iir-to-wasm`                 | WebAssembly 1.0 bytecode              |
| `iir-to-jvm-class-file`       | JVM class file                        |
| `iir-to-cil-bytecode`         | CLR CIL bytecode                      |
| `iir-to-beam`                 | Erlang BEAM bytecode                  |
| **`iir-to-llvm` (this crate)**| LLVM textual IR (`.ll`)               |

The first four target *managed* runtimes (each runtime owns register
allocation, GC, etc.).  This crate is the first **AOT-native** IIR backend
that doesn't hand-roll its own machine encoder — instead it hands a `.ll`
string to LLVM (`opt` + `llc`) and lets LLVM produce native code for any
CPU LLVM ships a backend for.

The hand-rolled `aarch64-backend` / `x86_64-backend` crates remain the
right call when we want full encoding control (e.g. for the AOT debugger
story).  This crate is the right call when we want world-class O2
optimization for free.

## Why textual `.ll`, not `llvm-sys`?

- **Zero build-time dep.**  CI doesn't need LLVM installed; emit a string.
- **Debuggability.**  The output is the human-readable form.
- **Forward-compat.**  A `llvm-sys` emitter can be added later as a sibling
  without breaking callers.

The cost is that `.ll` is slower to ingest than bitcode.  For a hobby
codebase compiling small modules, that's irrelevant.

## Quick start

```rust
use interpreter_ir::IIRModule;
use iir_to_llvm::{validate_for_llvm, lower_iir_to_llvm, IIRLlvmConfig};

let module = IIRModule {
    name: "demo".into(),
    functions: vec![],
    entry_point: None,
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_llvm(&module).is_empty());

let ll = lower_iir_to_llvm(&module, &IIRLlvmConfig::default())
    .expect("lowering should succeed");
println!("{ll}");
// ; ModuleID = 'iir_module'
// target triple = "x86_64-unknown-linux-gnu"
```

## Configuration

`IIRLlvmConfig` has two knobs:

- `module_name` — emitted in the `; ModuleID = '<name>'` comment.
- `target_triple` — emitted in `target triple = "<triple>"`.

The default triple is a **fixed** string (`"x86_64-unknown-linux-gnu"`)
rather than a host-derived value.  This keeps test output byte-identical
across CI runners.  Override via `.with_target("riscv32-unknown-elf")` when
you actually intend to run `llc` for a non-default architecture.

## Roadmap

| Version | Scope                                                       |
|---------|-------------------------------------------------------------|
| v0.1.0  | Crate skeleton: empty module header. *(this release)*       |
| v0.2.0  | Function signatures + `ret`/`ret_void`/`const`/`mov`.       |
| v0.3.0  | Typed arithmetic (`add`/`sub`/`mul`/...) + cmp + branches.  |
| v0.4.0  | `call` + `call_builtin print_i64` extern declarations.      |
| v0.5.0  | Tagged-word lisp `cons`/`car`/`cdr` → `call @__twig_lispy_*` (McCarthy W12b-1). |
| v0.6.0  | `COND` via stack-slot (`alloca`) SSA-merge + `jmp_if` void-cond + empty-block `br` (McCarthy W12b-3). |
| v0.7.0  | Lisp symbols — `symbol` type → `i64` tagged immediate (McCarthy W13a). |
| v0.8.0  | Lisp lambda (F7) — declare `lispy_to_exit_code` runtime switch; **LLVM McCarthy-complete F1–F7** (McCarthy W13b). |
| v0.9.0  | Byte-tape ops `alloc_bytes`→`@calloc`, `load_byte`/`store_byte` (zext/trunc at the byte boundary), `putchar`/`getchar` libc builtins, + slot-dest SSA rename. **Brainfuck runs on LLVM** (LANG-MATRIX LM-L Brainfuck). |
| v0.10.0 | Reassigned **parameters** are promoted to i64 stack slots (initialised from the incoming argument, narrow args zext'd) — a parameter accumulated across a loop back-edge is no longer silently dropped (LANG-FULL — LLVM first-class). |
| v0.11.0 | **Narrow unsigned arithmetic wraps mod-2ⁿ** (LANG-FULL E2). A `u4`/`u8`/`u16`/`u32` op computes at i64 then `and i64 …, <mask>` (see below). Adds `u4` to the supported types. |
| v0.12.0 | **Bitwise `not`** — synthesised as `xor x, -1` (LLVM has no `not`); a narrow width masks the result (`~0u8 = 255`). Unblocks Nib N3-`~` / Oct O2-`~`. |
| v0.13.0 | **`f64` variable slots** (LANG-FULL E3). An `f64` local gets an `alloca double` slot (`store/load double`); a float `cmp_*` result `zext i1 → i64` (not the invalid `→ double`); `f64` literals render as LLVM's exact hex double `0x…`. **ALGOL 60 reals run on LLVM.** |
| v0.14.0 | **Bounds-checked arrays** (LANG-FULL E5, static model). `alloc_array`→length-prefixed `@calloc` `[i64 len][elems…]`; `array_get`/`array_set` emit an explicit `icmp uge idx, len` + `br` to a `call void @llvm.trap()` block (OOB → trap), then a typed `getelementptr`+`load`/`store`; `array_len` reads the header. Declares `@calloc`/`@llvm.trap` on demand. |
| v0.15.0 | **Typed module globals** (LANG-FULL E6 layer 1). `global_load`/`global_store` lower: each distinct global name → a module-level `@__twig_global_N = internal global i64 0` (index-based, zero-init), with `load`/`store i64` at use sites — so a function reads/writes a global. Verified end-to-end on real `clang` (⇒ exit 42). |
| v0.17.0 | **String literal output** (LANG-FULL E4 / BA4). `str_const` emits a private length-prefixed string constant and `print_str` calls `@__print_str(ptr,i64)` with the payload pointer plus byte length. Richer byte-string ops stay rejected. |
| v0.18.0 | **Literal string metadata** (LANG-FULL E4). `str_len`/`str_eq`/`str_concat` over direct `str_const` literals lower to compile-time results, proving `(string-length "HELLO")`, `(string=? "HELLO" "HELLO")`, and `(string-length (string-append "AB" "CDE"))` on LLVM while dynamic string algebra stays rejected. |
| v0.22.0 | **Computed string indexes** (LANG-FULL E4). Literal-only `str_len` constants can flow through typed `i64` arithmetic and feed `str_index`. |
| v0.24.0 | **Literal string comparison** (LANG-FULL E4). Literal-only `str_cmp` lowers to the shared `-1`/`0`/`1` ordering result. |
| v0.42.0 | **Structural heap ops + name quoting** (LANG-FULL E6d-6). `alloc`→`call i64 @__twig_gc_alloc`, `field_store`/`field_load`→`inttoptr`+`getelementptr i64`+`store`/`load` (a field is at `idx*8`), `is_null`→`icmp eq …, 0`+`zext` — the native backend's word-granular model. `llvm_fn_ident` quotes special-char names (`@"point-x"`, `@"Some?"`). **Twig records run on LLVM** (exit 42). Union `match` on native/LLVM is a documented follow-up (E6d-6b). |
| v0.48.0 | **Twig GC completion, Part 2.** `alloc_bytes`/`alloc_array`→`call i64 @__twig_alloc_bytes` (GC-tracked, replacing raw never-freed `@calloc` — a confirmed leak). New `gc_live_bytes` `call_builtin`→`@__twig_gc_live_bytes()`, proving (by an actual running end-to-end test, not by reading C source) that `alloc`/`gc_alloc` already auto-collect under real allocation pressure via `gc-core-capi`'s pre-allocation `should_collect` check. |
| (later) | Debug info via `!dbg`. |

### Bounds-checked arrays (v0.14.0; GC-tracked since v0.48.0)

An IIR array (LANG-FULL E5) is a single `@__twig_alloc_bytes`-allocated block
laid out as a length header followed by the elements; the **handle** is a
`ptr` to the payload (`base + 8`), so element access is a typed
`getelementptr <T>` and the length lives at `handle − 8`:

```
base ──► [ i64 length | element 0 | element 1 | … ]   (zero-filled)
         └─ 8 bytes ──┘ ▲ handle
```

`@__twig_alloc_bytes` returns an i64 handle (`inttoptr`'d to `base`
immediately, the same convention `alloc`'s own handle uses) rather than the
`@calloc` this called through v0.47.0 — `@calloc` was never freed or traced,
a genuine, confirmed leak. `find_header` resolves the `base + 8` interior
handle back to its enclosing block correctly, so this stays a valid,
collectible root.

**Known gap (found by security review, not fixed):** the array's block is
always registered under `__twig_alloc_bytes`'s no-ref `HeapKind`, which is
only correct for genuinely scalar element types. `array<str>`/`array<any>`/
`array<symbol>` elements are i64 *handles* to separately GC-managed blocks —
`llvm_type_for` maps all of these down to the same `"i64"` LLVM type plain
integers use, so they pass `array_elem_llvm`'s check too, and a
string/symbol reachable only via such an array element isn't traced as a
root. Pre-existing (the old `@calloc` block was equally untraced) and
cross-backend (`aarch64-backend`/`x86_64-backend` share it); tracked
separately, not attempted here. See `code/specs/AOT00-T1w-llvm-gc-completion.md`
§5.

Unlike the JVM/CLR managed-array backends (whose runtime bounds-checks every
element access for free), the native/LLVM target has no such runtime, so each
`array_get`/`array_set` emits an **explicit** unsigned compare against the stored
length (`icmp uge i64 idx, len` — a single check that catches both `>= len` and a
negative index, since a negative `i64` is a huge unsigned value) and branches to a
`call void @llvm.trap(); unreachable` block when out of range. This is the
static-backend realisation of E5's "out-of-bounds → trap" rule. The element type
(`i64`/`double`/`i32`/`float`) comes from the op's `type_hint`; the index is always
`i64`.

### Byte-tape memory (v0.9.0; GC-tracked since v0.48.0)

Brainfuck builds an implicit byte tape; `lower_brainfuck_for_aot` (in `lang-aot`)
rewrites it into the same `alloc_bytes` / `load_byte` / `store_byte` ops the
native x86_64 backend already uses (LANG76). This crate's lowering:

| IIR op | LLVM emitted | Notes |
|--------|--------------|-------|
| `alloc_bytes d <- n` | `%r = call i64 @__twig_alloc_bytes(i64 n)` + `%d = inttoptr i64 %r to ptr` | zero-filled, GC-tracked tape |
| `load_byte d <- base, i` | `getelementptr i8` + `load i8` + `zext i8…i64` | cell → word |
| `store_byte base, i, v` | `getelementptr i8` + `trunc i64…i8` + `store i8` | word → cell (8-bit wrap) |
| `call_builtin putchar v` | `trunc i64…i32` + `call i32 @putchar(i32)` | libc; Brainfuck `.` |
| `call_builtin getchar -> d` | `call i32 @getchar()` + `sext i32…i64` | libc; Brainfuck `,` |
| `call_builtin gc_live_bytes -> d` | `%d = call i64 @__twig_gc_live_bytes()` | diagnostic (v0.48.0), mirrors aarch64/x86_64-backend |

Byte width lives **only at the tape boundary** (the `zext`/`trunc`); every register
in between is a uniform `i64`, which is what lets the i64-only stack-slot model
consume Brainfuck's reassigned `ptr`/`v` without a width mismatch.

### Narrow-width register arithmetic (v0.11.0, LANG-FULL E2)

The same "uniform i64 in registers" model is exactly why narrow **unsigned**
arithmetic must wrap with a *value mask*, not a narrow-typed op. A `u8` add
whose operands are `i64` SSA values cannot be `add i8 %a, %b` — that is invalid
IR `clang` rejects. So `add`/`sub`/`mul`/`div`/`mod` and `and`/`or`/`xor` on a
`u4`/`u8`/`u16`/`u32` `type_hint` compute at i64 and mask the result back into
the width:

```llvm
  %__nw1 = add i64 200, 100     ; compute wide
  %v     = and i64 %__nw1, 255  ; 300 & 0xFF = 44   (u8 wrap)
```

| type_hint | mask         |  | type_hint | mask         |
|-----------|--------------|--|-----------|--------------|
| `u4`      | `0xF`        |  | `u16`     | `0xFFFF`     |
| `u8`      | `0xFF`       |  | `u32`     | `0xFFFFFFFF` |

`u64`/`i64` (full word), signed narrow widths, and floats get no mask. This
mirrors the VM/JIT/wasm/JVM/CLR backends (each masks the narrow result by
`type_hint`) and generalises the byte-tape's 8-bit `store_byte` wrap to register
arithmetic. Verified by RUNNING the emitted `.ll` through real `clang`:
`200u8 + 100u8` exits `44`.

See [`code/specs/iir-to-llvm.md`](../../../specs/iir-to-llvm.md) for the
full spec and [`code/specs/MULTILANG-BACKEND-PLAN.md`](../../../specs/MULTILANG-BACKEND-PLAN.md)
§LLVM for how this fits the broader plan.

## Tests

```sh
cargo test -p iir-to-llvm
```

7 tests at v0.1.0 covering validator stub, output shape, config defaults,
and error display.
