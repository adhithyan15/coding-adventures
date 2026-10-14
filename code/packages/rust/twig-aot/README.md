# twig-aot

String folding excludes mutable integer indices, and runtime string indexing calls the existing bounds-checked helper instead of folding a loop to its first byte.

Runtime string slices now call the existing checked helper when source or bounds cannot fold; reassignment invalidates stale literal lengths.

Twig ahead-of-time compiler.  Reads a Twig source file, produces a native
ARM64 Mach-O executable on macOS that you can run directly.

On Windows x86-64, executable linking uses the dynamic MSVC CRT with normal
startup. Do not mix in `libvcruntime.lib` or `libucrt.lib`: their static
definitions conflict with the dynamic libraries. Run
`cargo test -p twig-aot --test windows_x86_64_smoke -- --nocapture` on Windows
to exercise compilation, linking, and actual executable results. That suite
still explicitly skips two precise-GC frame-walk differentials; a green test
summary does not prove those unfinished paths.

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

### Windows x86-64

A Windows linker on `PATH`, probed in this order (`find_windows_linker`):
`link.exe` (MSVC, from Visual Studio Build Tools' "Desktop development with
C++" workload — note some shells (git-bash/MSYS) ship an unrelated POSIX
`link(1)` under the same name; it's detected by banner and skipped), then
`lld-link.exe` (LLVM), then `gcc.exe` (MinGW). An MSVC install also needs its
environment loaded (`vcvars64.bat`, or the Developer Command Prompt) so `LIB`/
`INCLUDE` point at the CRT and Windows SDK import libraries — `link.exe`
resolves purely by name on `PATH` and doesn't auto-detect an unregistered MSVC
install (no `vswhere` probing in V1).

### Required Windows CI execution (VM-032)

PR CI selects the native Windows smoke suite when the build plan's Windows
closure contains `rust/twig-aot` or `rust/lang-aot`, including their affected
dependencies. Edits to the gate or MSVC bootstrap also select it. The runner
activates the MSVC developer environment and runs
`cargo test -p twig-aot --test windows_x86_64_smoke -- --nocapture` with
`LANG_REQUIRE_WINDOWS_AOT=1`; missing linkers fail instead of silently skipping.
Without that flag, local hosts retain optional-toolchain skips. Of eight tests,
five link and execute programs, one inspects the PE object, and two explicitly
return early for the separately tracked precise-GC frame-walk gap (VM-031).

FLOW-MATIC EOF support (VM-039a): `input_more()` uses a non-consuming stdio
peek alongside `input_i64`. The native/LLVM input stream proof includes empty
input and repeated EOF reads; this does not change existing integer parsing.


## Captured-array JVM validation checkpoint

Eight exact existing ALGOL captured/forwarded array fixtures pass through the real native preparation/inference/specialisation/stack-map compiler for diagnostic purposes, with an own in-repository ALGOL frontend dev dependency. The original physical-register refusal receipts are preserved. Every prepared function now compiles successfully for both MsX64 and SysV; separate simulator and real Windows controls establish the bounded stack-argument behavior.

Actual c5b00f83 validation completed: full JVM200 tests/docs across3 targets
(69 library,124 integration,7 docs), the original8 real JVM rows, positive/
negative float-array verification, real IEEE84 checks and strictJVM/LANG/Twig
alltargets passed. Focused controls are included in200, not additive. Earlier
source da45489b unfiltered LANG validation actually failed:463PASS9 native ABI
refusals. Latest full/native/hosted/merge acceptance remains open, along with
general-hop32/32 work failures. Miri **DEFERRED/PENDING**, never passed; frozen
run/source/inputs/target/receipts preserved and unpolled. No new crates.io deps.

## Bounded native word-call validation

Internal integer-word direct calls and function parameters now support
up to eight words on MsX64 and SysV. A fixed aligned outgoing tail follows
shadow space and stays below all negative RBP locals; callee extras spill from
RBP+16+shadow+8index. Frame/displacement arithmetic is checked; nine arguments
or formals refuse. Builtin physical-register marshalling is unchanged. Exact
both-ABI simulator controls check5/6/7/8 order, fullRAX, stackwords, shadow
clobbering, preserved locals, alignment/restoration, recursion and refusal.
Root metadata retainsp7 RBP-64 and actual external returnPC. Real Windows
six/eight-position and eight original captured/forwarded array rows execute
Exit42. Simulator evidence does not establish Linux/macOS hardware or forced
GC execution, and this bounded internal convention does not claim arbitrary
arity or native C floating argument interoperability.

Actual isolated source316e7878 validation completed: nine affected packages
1226PASS2FAIL1existingignore/81targets; only the obsolete five/seven-formal
refusal oracles failed. Full unfiltered LANG476/0/0, strict nine-package
alltargets, standalone Closure1093/0/0 across181targets and strict passed.
Spec-first test-only correction7126217d retains those accepted counts and eight,
adds exact ABI/got9/max8 plus public-refusal checks. New complete backend
85/0/0 over2 targets and strict nine-package alltargets passed.
Whole production prefix and every other Rust/instruction object remain exact;
unchanged runtime evidence is explicitly attributed to316e. These counts are
separate source receipts, not additive, and the original failed run stays failed.
The prior5ba run exhausted disk before tests; its failed receipts are preserved.
Retry disabled debug symbols/incremental compilation, preserving all inputs,
assertions and limits. Registry376 tuples remain exact. Miri
**DEFERRED/PENDING**, never passed; frozen run/input/target/receipts untouched.
General-hop32 resource debt, hosted CI/conflict/review/merge, full Closure parity
and output-source/full transformation lineage remain OPEN.
