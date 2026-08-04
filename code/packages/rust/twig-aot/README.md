# twig-aot

Twig ahead-of-time compiler.  Reads a Twig source file, produces a native
ARM64 Mach-O executable on macOS that you can run directly.

## Usage

```bash
twig-aot fib.twig -o fib
./fib
echo $?    # → main()'s return value modulo 256
```

## How it works

1. Parse the source with `twig-ir-compiler` → `IIRModule`
2. For each function: infer types (`aot-core`), specialise to typed CIR
3. Lower CIR → ARM64 bytes (`aarch64-backend`)
4. Link per-function bytes (`aot-core::link`)
5. Wrap in a Mach-O object file (`code-packager::macho_object`)
6. Shell out to `/usr/bin/ld` for the final link → trusted provenance,
   ad-hoc code signature, dyld stub

The final `ld` invocation is what makes the binary actually launch on
modern macOS — see CHANGELOG for the trust-model background.

## The runtime archive

AOT-compiled programs call into a small static runtime that `build.rs`
compiles (via the `cc` crate) and embeds in the `twig-aot` binary, then
writes to a temp file and hands to the system linker at compile time. It has
two translation units:

- `runtime/twig_runtime.c` — portable I/O + heap helpers (`__twig_print_i64`,
  `__twig_print_string`, `__twig_putchar`, `__twig_alloc_bytes`, …).
- `runtime/lispy_runtime.c` — the **shared lisp value model** (LANG77):
  `__twig_lispy_cons`/`car`/`cdr`/`pair_p`/`equal`/`not`/`truthy`/`make_symbol`/`nil`
  plus int box/unbox, implementing `dynval-runtime`'s 3-bit-tagged 64-bit
  `LispyValue` ABI. This is what lets *any* lisp-family frontend (Twig,
  McCarthy Lisp, future lisps) compile cons cells and interned symbols to a
  native binary — it is a language-agnostic primitive, not tied to one
  frontend.

The C lisp runtime and the Rust `dynval-runtime` crate (used by the VM/JIT)
are two implementations of one documented ABI. The `lispy_runtime_golden`
unit test pins the C side to the Rust `pub const`s/constructors so they can
never silently diverge. See `code/specs/LANG77-lisp-native-runtime.md`.

- **Garbage collector** — the conservative mark-and-sweep collector is **no longer
  a C file in this crate**. `runtime/twig_gc.c` was retired (#118b-2b); the collector
  now lives in the `gc-core-capi` crate (`gc-core`'s flat mark-and-sweep model behind
  a C ABI), which exports both the generic `__gc_*` names and the `__twig_gc_*` compat
  aliases the emitted code and `dynval_runtime.c` reference. `build.rs` builds
  `libgc_core_capi.a` and embeds it; each AOT link site writes it to a temp `.a` and
  passes it to the linker alongside the runtime archive, so every emitted binary's
  `__twig_gc_alloc` / `__twig_gc_safepoint` references resolve against one generic
  collector. `__dyn_cons` still allocates cons cells via `__twig_gc_alloc` — that ABI
  is unchanged; only its implementation moved. See `code/specs/AOT00-T1-precise-gc.md`
  and `code/specs/LANG16-gc-core.md`.

LANG-FULL E4 / BA4 literal string output reuses this runtime path: native AOT
preparation lowers `str_const` + `print_str` to `alloc_bytes`, `store_byte`, and
`call_builtin "print_string"`, so source-level BASIC `PRINT "HELLO"` runs through
the same object/link/runtime pipeline as byte-tape programs. Direct-literal
`str_len`, `str_index`, `str_eq`, `str_cmp`, literal `str_concat`, and literal `str_slice`
metadata over direct literals fold before machine-code lowering, so Twig
`(string-length "HELLO")`, `(string-ref "ABC" 1)`,
`(string=? "HELLO" "HELLO")`, `(string<? "ALPHA" "BETA")`,
`(string-length (string-append "AB" "CDE"))`,
and `(string-ref (substring "ABCDE" 1 4) 1)` run natively through the same
preparation pass. Folded `str_len` metadata can also flow through typed integer
arithmetic, so `(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))`
folds to byte `69` before native lowering.

When either operand is a runtime string handle, `str_eq` and `str_cmp` instead
lower to the matching `__twig_str_*` helper. This preserves equality and signed
lexical ordering for procedure results and branch-selected locals without
discarding the literal folding fast path.

As of 0.10.0, `prepare_module_for_aot` lowers a lisp frontend's `cons`/`car`/
`cdr` to **calls into this runtime** (via
`iir_builtin_lowering::lower_heap_builtins_runtime`), so a McCarthy program
like `(CAR (CONS 7 9))` compiles to a native binary that calls
`__twig_lispy_cons`/`__twig_lispy_car` and exits 7 — with the cons cell as a
tagged `LispyValue`.

As of 0.11.0 it also runs `lower_lisp_repr`, a type-directed pass that boxes
the integer atoms feeding those calls (`n << 3`, so their tag is `000`
instead of the heap tag a raw int's low bits collide with) and unboxes the
program result for the exit code. So `(CAR (CONS 7 9))` now round-trips
through fully **tagged** values — the representation the `pair?`/`ATOM`/`EQ`
predicates build on. It keys on use-sites, so non-lisp programs are
untouched.

## Requirements

- Apple Silicon Mac running macOS 15+ (Sequoia / Tahoe)
- Xcode Command Line Tools (`/usr/bin/ld`, `xcrun`)
