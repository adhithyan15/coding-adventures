# Changelog — `twig-aot`

## 0.5.0 — 2026-05-16 (`--emit-object` for cross-OS workflows)

**Cross-OS object emission via a new `--emit-object` flag.**

The third follow-up from PR #3203.  The `.o` / `.obj` object format
is fully portable — only the *link step* is bound to the target
host's toolchain.  This release exposes that asymmetry: produce the
object file on any host, then copy it to a target machine and link
it there.

```
# On Windows: produce a Linux ELF .o
twig-aot foo.twig --target=linux-x86_64 --emit-object -o out/foo

# Output:
# twig-aot: emitted object: out/foo.o
# twig-aot: NOTE: runtime archive for LinuxX86_64 was not built on
#           this host (1-byte stub).  Build twig-aot on a
#           LinuxX86_64 host or rebuild the runtime from
#           `twig-aot/runtime/twig_runtime.c` on the target machine.
```

When the runtime archive *is* available for the target (i.e. the
twig-aot binary was built on a matching host), `--emit-object` also
writes the archive alongside and prints the exact link command:

```
# On Windows: produce a Windows .obj + .lib (host == target)
twig-aot foo.twig --target=windows-x86_64 --emit-object -o out/bar

# Output:
# twig-aot: emitted object: out/bar.obj
# twig-aot: emitted runtime archive: out/bar_runtime.lib
# twig-aot: link on the target host with:
#   link.exe /OUT:<exe>.exe /ENTRY:main /SUBSYSTEM:CONSOLE \
#            out/bar.obj out/bar_runtime.lib libcmt.lib legacy_stdio_definitions.lib
```

### New API

- **`EmitObjectTarget`** enum — `MacosArm64`, `LinuxX86_64`,
  `WindowsX86_64`.  Selects which object format and runtime archive
  the helper produces.
- **`EmittedObject`** struct — `{ object_path, runtime_archive_path,
  target }` returned from `emit_object_to_disk`.  Callers (e.g. the
  CLI) print human-readable paths.
- **`emit_object_to_disk(src, out_base, target) -> EmittedObject`**
  — writes the relocatable object and (if available on this build
  host) the runtime archive next to it.  Works from any (host,
  target) combination because object emission doesn't need the
  target's toolchain.

### CLI changes

- New `-c` / `--emit-object` boolean flag.  When set, the binary
  writes the object + (optional) runtime archive instead of
  invoking the system linker.  Combines with `--target` so the
  user can write a Linux `.o` on a Windows host (or vice versa).

### Tests

- `emit_object_to_disk_writes_linux_o`: verifies the `.o` extension
  and ELF magic.
- `emit_object_to_disk_writes_windows_obj`: verifies the `.obj`
  extension and `IMAGE_FILE_MACHINE_AMD64` (0x8664) at byte 0.
- `emit_object_runtime_path_is_none_when_archive_is_stub`: iterates
  the three targets and asserts at least one yields a real archive
  (the host) and at least one yields a stub.

### Out of scope (deferred to V2)

Full cross-OS *linking* — taking a Linux ELF source on a Windows
host all the way to a runnable `<exe>` without copying — would need
either a bundled `clang+lld` + sysroot toolchain, or a `zig cc`
dependency.  Both are substantial.  `--emit-object` covers the
common case (build farm produces objects, target machine links)
without that complexity.

## 0.4.0 — 2026-05-16 (`--target` CLI flag)

**Expose the LANG46 multi-target driver to end users via a `--target`
CLI flag on the `twig-aot` binary.**

Previously the CLI only invoked `compile_file_macos_arm64`; the
Linux/Windows entry points existed but were unreachable from the
command line.  This release adds a `--target` flag and host-aware
dispatch:

```
twig-aot foo.twig                      # auto-picks the host target
twig-aot foo.twig --target=linux-x86_64
twig-aot foo.twig --target=windows-x86_64
twig-aot foo.twig --target=macos-arm64
```

Accepted values (and full target-triple aliases):
| Short | Triple |
|---|---|
| `auto` (default) | (build host) |
| `macos-arm64` | `aarch64-apple-darwin` |
| `linux-x86_64` | `x86_64-unknown-linux-gnu` |
| `windows-x86_64` | `x86_64-pc-windows-msvc` |

Cross-OS dispatch (e.g. `--target=linux-x86_64` on a Windows host)
errors out cleanly:

```
$ twig-aot --target=linux-x86_64 foo.twig    # on Windows
twig-aot: --target=linux-x86_64 requires a Linux x86-64 host in V1
         (cross-OS compilation is a separate follow-up)
```

Unknown targets produce an enumerated error:

```
$ twig-aot --target=bogus foo.twig
twig-aot: unknown target "bogus"; expected one of: auto, macos-arm64,
         linux-x86_64, windows-x86_64
```

## 0.3.1 — 2026-05-16 (multi-function x86_64 cross-fn patching)

**Patch cross-function `call` sites in place during the x86_64
two-pass compile.**

Previous v0.3.0 release noted that multi-function programs were
deferred — every `call` instruction surfaced as a `PltRel32`
external relocation, which only resolved correctly when the callee
was a runtime helper (e.g. `__twig_print_i64`).  Cross-module call
sites resolved fine because the system linker still found the
symbol via the function's exported symbol-table entry, but the
extra reloc overhead and the dependency on every internal function
having a global symbol were both incidental.

`compile_module_x86_64_to_text` now mirrors `aarch64-backend`'s
Pass 2 strategy:

- After concatenating per-function bytes via `aot_core::link::link`,
  walk every per-function reloc.
- If the reloc names another function in the same module
  (`offsets.contains_key`) AND is a `PltRel32` (CALL rel32),
  resolve in place: write `callee_off - patch_offset - 4` into the
  disp32 slot.  The reloc is consumed; the linker never sees it.
- Everything else (runtime helpers, possibly-external globals)
  passes through to the packager unchanged.

This unblocks real Twig programs (mutual-recursion, helpers, etc.)
on both Linux and Windows hosts.

### Tests

- `x86_64_cross_function_call_patched_in_place` — compiles a
  two-function module (`main` calls `helper`), verifies the CALL
  site's disp32 was patched to the correct PC-relative offset, and
  confirms no external reloc for `helper` is emitted.
- `x86_64_external_call_remains_in_relocs` — verifies that calls
  to runtime helpers like `__twig_print_i64` still surface as
  external relocs even when multi-function patching is otherwise
  active.

## 0.3.0 — 2026-05-14 (LANG46 phase 2 — multi-target driver)

**End-to-end Twig source → native binary on Linux x86-64 and Windows
x86-64.** This is the final piece of the x86-64 port — after this
release, the same Twig programs that compile on macOS ARM64 compile
and run on Linux x86-64 and Windows x86-64 hosts.

### New entry points

- `compile_module_linux_x86_64_object(module)` / `compile_linux_x86_64_object(source, name)`
  — emit an ELF64 `ET_REL` object file via `x86_64-backend` (System V
  AMD64 ABI) + `code-packager::pack_elf64_object_x86_64`.
- `compile_module_windows_x86_64_object(module)` / `compile_windows_x86_64_object(source, name)`
  — emit a PE/COFF `IMAGE_FILE_MACHINE_AMD64` object file via
  `x86_64-backend` (Microsoft x64 ABI) +
  `code-packager::pack_pe_object_x86_64`.
- `compile_file_linux_x86_64(src, out)` (`#[cfg(target_os = "linux")]`)
  — full pipeline: source → IR → x86_64 bytes → ELF object → `cc` →
  runnable ELF executable.
- `compile_file_windows_x86_64(src, out)` (`#[cfg(target_os = "windows")]`)
  — full pipeline: source → IR → x86_64 bytes → PE/COFF object →
  linker probe (`link.exe` → `lld-link.exe` → `gcc.exe`) → runnable
  `.exe`.

### Windows linker probe

The Windows path detects an actual MSVC `link.exe` by parsing the
banner ("Microsoft" + "Linker") rather than just checking program
spawnability — git-bash hosts ship a POSIX `link(1)` utility with the
same name on `PATH`, which would otherwise be (incorrectly) chosen.

### End-to-end smoke tests

- `tests/linux_x86_64_smoke.rs` (`#[cfg(target_os = "linux")]`):
  compiles small typed Twig programs (`42`, `(+ 30 12)`, `(* 6 7)`,
  branches), links via `cc`, runs the resulting ELF executable,
  asserts the exit code matches `main`'s return value.
- `tests/windows_x86_64_smoke.rs` (`#[cfg(target_os = "windows")]`):
  same suite via `link.exe` and a `.exe` output.  Each test
  gracefully skips when no real Windows linker is detected on
  `PATH` (e.g. MSVC dev env not activated).
- `tests/macos_arm64_smoke.rs` (existing): unchanged and still
  passes; verifies the macOS path didn't regress.

Each smoke test runs only on its respective CI runner; the suite
covers Linux + macOS + Windows end-to-end without cross-compilation.

## 0.2.0 — 2026-05-14 (LANG46 phase 1 — per-host runtime archives)

**Extend `build.rs` to produce per-host runtime archives plus stubs for
non-host targets.**

Sets up the runtime-archive layer that phase 10's multi-target driver
will consume.  After this release, `twig-aot` compiled on any of the
three V1-supported hosts exports three env vars
(`TWIG_RUNTIME_ARCHIVE_MACOS_ARM64`,
`TWIG_RUNTIME_ARCHIVE_LINUX_X86_64`,
`TWIG_RUNTIME_ARCHIVE_WINDOWS_X86_64`), each pointing at either the
real archive (for the build host's target) or a 1-byte stub (for
other targets).

The phase 10 driver uses these env vars with `include_bytes!` to bake
all three runtime archives into the `twig-aot` binary; at AOT compile
time, it picks the right one based on `--target` and refuses to emit
for a target whose archive is a stub with a clear "no runtime archive
for X on this host" error.

### Host-targets-host policy

V1 supports only host-targets-host AOT.  Each CI runner builds for
its own host and verifies its respective smoke test.  Cross-OS
compilation is deferred — adding it requires bundling cross
toolchains with `twig-aot` or detecting them on the host.

### Backwards compatibility

The existing `TWIG_RUNTIME_ARCHIVE` env var is preserved as an alias
for the host's archive (or a legacy stub on unsupported hosts), so
the existing `compile_file_macos_arm64` entry point continues to
work without changes.

## 0.1.9 — 2026-05-13 (LANG42)

**Wire the refinement obligation checker into the AOT pipeline.**

LANG23 built a complete refinement-type infrastructure (solver, checker, type
annotations on `IIRFunction`), but the IIR never reached the checker —
annotations silently did nothing.  LANG42 fixes this by adding a pre-codegen
pass that runs immediately after `twig-ir-compiler` emits the `IIRModule`,
before any lowering, and discharges every proof obligation through the existing
`lang-refinement-checker` API.

### New dependency

- **`iir-refinement-pass = { path = "../iir-refinement-pass" }`** — new crate
  that implements `check_module(module, mode) -> Vec<RefinementError>`.

### New `AotError` variant

- **`AotError::RefinementViolations(Vec<iir_refinement_pass::RefinementError>)`** —
  returned when one or more proof obligations are `ProvenUnsafe` (Lenient mode)
  or `ProvenUnsafe | Unknown` (Strict mode).

### Changed

- **`compile_module_macos_arm64_object`** now calls `check_refinements` before
  `compile_module_to_text`.  In `Lenient` mode (default) only `ProvenUnsafe`
  outcomes abort compilation.

- **`compile_module_macos_arm64_object_with_mode`** — new public function
  accepting an explicit `RefinementMode`.  The old function delegates to it
  with `Lenient`.

### Tests added

- `refinement_violation_becomes_aot_error` — a literal that violates a
  `(Int 0 128)` annotation returns `Err(AotError::RefinementViolations)`.
- `safe_annotated_program_compiles_ok` — a literal within range compiles
  normally.

---

## 0.1.8 — 2026-05-13 (LANG41)

**Replace macOS-specific `emit_print_helper` injection with a portable C
runtime archive linked via the system linker.**

LANG40 injected a 208-byte ARM64 subroutine with hardcoded macOS `write(2)`
syscall numbers (`x16=4`, `SVC #0x80`) into user code before linking.
LANG41 removes that approach entirely; `__twig_print_i64` is now defined in
a portable C file compiled at `cargo build` time and embedded in the
`twig-aot` binary, then written to a temp file and passed to `ld` for each
AOT compilation.

### New files

- **`runtime/twig_runtime.c`** — defines `__twig_print_i64(int64_t val)` using
  `printf("%lld\n", (long long)val)` + `fflush(stdout)`.  Pure POSIX — no raw
  syscall numbers, no platform ifdefs.  On macOS, `printf` routes through
  `libSystem`; on Linux, it routes through `libc`.  The same source file works
  on both platforms without change.

- **`build.rs`** — uses the `cc` crate to compile `runtime/twig_runtime.c`
  into `$OUT_DIR/libtwig_aot_runtime.a` at `cargo build` time.
  Exports `cargo:rustc-env=TWIG_RUNTIME_ARCHIVE=<path>` so the archive path
  is available to `include_bytes!` at compile time.
  `cargo:rerun-if-changed=runtime/twig_runtime.c` invalidates only when the
  C source changes.

### Changed

- **`[build-dependencies]`**: `cc = "1"` added to `Cargo.toml`.

- **`RUNTIME_ARCHIVE`** static: `include_bytes!(env!("TWIG_RUNTIME_ARCHIVE"))`
  embeds the archive in the binary.  Zero disk overhead at runtime (extracted
  only during AOT compilation).

- **`compile_module_to_text_raw`** return type is now a 5-tuple:
  `(text, offsets, n_global_slots, global_byte_relocs, extern_branch_relocs)`.
  The fifth element replaces the old "fail on unresolved external" logic;
  unresolved `BL` targets are now collected and forwarded to the packager.

- **`compile_module_macos_arm64_object`** always calls
  `pack_object_with_globals_and_externals` (no more conditional on whether
  globals are present).

- **`invoke_ld`** writes `RUNTIME_ARCHIVE` to a temp file (`twig_aot_runtime_<pid>.a`)
  and passes it as an argument to `ld` before cleanup.

- **`emit_print_helper` injection removed** — the old "inject helper if any
  function references `__twig_print_i64`" block in
  `compile_module_to_text_raw` is gone.

### Tests

All existing tests pass.  Integration tests in `tests/macos_arm64_smoke.rs`
exercise the full pipeline including `end_to_end_object_through_ld_returns_42`.

---

## 0.1.7 — 2026-05-13 (LANG40)

**AOT `io_out` — integer print to stdout via `__twig_print_i64`.**

Twig programs that use `(print n)` now compile to native ARM64 code without
`BackendRefused`.  Previously the `io_out` CIR opcode had no ARM64 handler;
LANG40 adds end-to-end support across three crates.

### Pipeline change — helper injection

`compile_module_to_text_raw` gains a new step between Pass 1 and the linker:

```rust
let needs_print_helper = fn_results.iter().any(|(_, _, relocs, _)| {
    relocs.iter().any(|r| r.symbol == "__twig_print_i64")
});
if needs_print_helper {
    fn_results.push(("__twig_print_i64".to_string(), emit_print_helper(), vec![], vec![]));
}
```

If any compiled function contains a `BL __twig_print_i64` placeholder (from
the `io_out` handler in `aarch64-backend`), the 208-byte self-contained print
helper is appended to `fn_results` before `link()` runs.  The existing
two-pass BL patcher then resolves the symbol and patches the correct
PC-relative offset automatically — zero new linker infrastructure needed.

The helper is **not** emitted when no `io_out` instructions are present,
so programs without printing incur zero overhead.

### Tests (2 new)

| Test | Asserts |
|------|---------|
| `print_program_compiles_ok` | `(print 42)` compiles to a valid `MH_OBJECT` Mach-O |
| `print_program_is_valid_macho` | compiled object is ≥ 400 bytes (helper present) |

### Upstream dependency versions

| Crate | Old | New |
|-------|-----|-----|
| `aarch64-encoder` | 0.2.1 | 0.2.2 (adds `strb_pre_neg1`) |
| `aarch64-backend` | 0.2.1 | 0.2.2 (adds `io_out` handler + `emit_print_helper`) |

## 0.1.6 — 2026-05-13 (LANG39)

**First-class global variable support — `(define x 5) x` now compiles to native code.**

Twig programs that use top-level value defines (`(define x 5)`, `(define counter 0)`)
previously failed with `AotError::BackendRefused` because the V1 ARM64 backend didn't
know how to handle `global_set` / `global_get` builtins.  LANG39 closes that gap
end-to-end across all four affected crates.

### New dependency

- `iir-builtin-lowering` added to `Cargo.toml` (provides `lower_global_io`).

### Pipeline changes

#### `prepare_module_for_aot`

Two new phases prepend the existing four-step AOT preparation pipeline:

**Phase 0 — `lower_global_io(module)`**
Converts `call_builtin "global_set"/%n/%v` → `global_store Str("name") Var(val_reg)` and
`call_builtin "global_get"/%n` → `global_load Str("name")` (imported from `iir-builtin-lowering`).
Must run before arithmetic pre-lowering so the const-string look-back can see the full instruction list.

**Phase 0b — `strip_dead_string_consts(func)`**
The twig-ir-compiler emits `const %n = Var("x")` (name-register) before each
`global_set`/`global_get` call.  After Phase 0, those call_builtins are gone
but the `const %n` instruction remains dead in the list.  `aot_specialise` would
convert it to `const_str` which the ARM64 backend cannot lower.

This new pass removes every `const` instruction whose source is `Operand::Var(_)`
(the string-literal-as-Var encoding) **and** whose dest register is never referenced
in any other instruction's `srcs`.  Registers that are still read (e.g. name args to
un-lowered `call_builtin "make_closure"`) are retained.

#### `compile_module_to_text` / `compile_module_to_text_raw`

Return type extended from `(Vec<u8>, HashMap<String, usize>)` to
`(Vec<u8>, HashMap<String, usize>, usize, Vec<GlobalByteReloc>)`.

New fields:
- `n_global_slots` — number of unique globals found by `collect_global_slots`.
- `global_byte_relocs` — `Vec<GlobalByteReloc>` containing the byte offsets of
  every `ADRP + ADD` instruction pair in the linked text section (for `ld`'s
  ARM64 relocation records).

#### `compile_module_macos_arm64_object`

When `n_global_slots > 0`, now calls `pack_object_with_globals` (from
`code-packager 0.2.1`) instead of `pack_object`.  This emits a two-section
Mach-O object file (`__TEXT/__text` + `__DATA/__data`) with:
- A zero-initialised `__data` section (8 bytes per global slot).
- An exported `_twig_globals` symbol pointing to the start of that section.
- `ARM64_RELOC_PAGE21` + `ARM64_RELOC_PAGEOFF12` relocation records per `GlobalByteReloc`.

When `n_global_slots == 0`, the original single-section `pack_object` path is used unchanged.

#### `collect_global_slots(module)`

New internal helper.  Scans all `global_load`/`global_store` instructions in a
post-`lower_global_io` module for `Operand::Str(name)` in `srcs[0]`.  Assigns each
unique global name a consecutive 0-based slot index (slot `i` lives at `_twig_globals + i*8`).

### Test changes

- `untyped_twig_returns_backend_refused` → renamed to `global_define_compiles_ok`.
  The old test expected `(define x 5) x` to fail.  With LANG39 it must now succeed
  and produce a valid `MH_OBJECT` Mach-O.

### Upstream dependency versions

| Crate | Old | New |
|-------|-----|-----|
| `aarch64-encoder` | 0.2.0 | 0.2.1 (adds `adrp_placeholder`) |
| `aarch64-backend` | 0.2.0 | 0.2.1 (adds `compile_with_globals`, `GlobalWordReloc`) |
| `code-packager` | 0.2.0 | 0.2.1 (adds `pack_object_with_globals`, `GlobalByteReloc`) |

## 0.1.5 — 2026-05-13

**Default integer type changed from `u64` to `i64` — all typing states now correct.**

Twig integers are semantically signed 64-bit values.  The previous default of
`"u64"` for untyped params caused `(< x 0)` to emit an unsigned ARM64 `CMP`,
which treated `-5` as a very large positive number and returned wrong results
for programs that compared against negative numbers.

### Changes

- **`normalize_params_to_i64`** (was `normalize_params_to_u64`): promotes
  `"any"` / `"polymorphic"` params to `"i64"` instead of `"u64"`.
- **`default_any_to_i64`** (was `default_any_to_u64`): defaults remaining
  `"any"` arithmetic/mov hints to `"i64"` instead of `"u64"`.
- **`infer_aot_type`**: integer literal constants (`Operand::Int(_)`) now infer
  `"i64"` instead of `"u64"`, so constant expressions propagate signed types.
- **`compile_typed_module_to_arm64_bytes`**: now calls `normalize_params_to_i64`
  before `propagate_aot_types`, ensuring that unannotated params (still `"any"`
  after the caller has set annotation-derived types) also get `"i64"` semantics.
  Previously this function relied entirely on the caller to set all param types,
  which left unannotated functions with unsigned comparisons.

### Effect

All three optional-typing states (untyped / partially typed / fully typed) now
produce correct results for programs that compare against negative numbers.
Type annotations are purely additive: they document intent and may enable future
optimisations, but are never required for correctness.

## 0.1.4 — 2026-05-13

**In-process ARM64 execution + typed i64 pipeline.**

### New public APIs

#### `compile_module_to_arm64_bytes(module) → Result<(Vec<u8>, HashMap<String, usize>), AotError>`

Returns raw ARM64 machine code bytes and a function-name→byte-offset map.
Uses the full preparation pipeline (builtin pre-lowering + i64 param
normalisation + type propagation + default-any-to-i64).  Suitable for
in-process execution via `call_arm64_function_in_process`.

#### `compile_typed_module_to_arm64_bytes(module) → Result<(Vec<u8>, HashMap<String, usize>), AotError>`

Like `compile_module_to_arm64_bytes` but uses caller-supplied type
annotations.  The caller pre-lowers builtins and may set params to `"i64"`;
this function first normalises any remaining `"any"` params to `"i64"`, then
propagates types.  Comparison instructions emit `cmp_lt_i64` (signed ARM64
condition code).  Correct for negative numbers whether or not the caller
pre-annotated params.

#### `pre_lower_aot_builtins_on_module(module: &mut IIRModule)`

Exposes the `pre_lower_aot_builtins` pass at the module level so callers
can pre-lower before running their own type-inference pass.

#### `call_arm64_function_in_process(code, offsets, fn_name, arg) → Result<i64, AotError>`

*macOS/ARM64 only.*  Execute compiled ARM64 code in-process:
1. Allocates an anonymous `PROT_READ | PROT_WRITE` mapping.
2. Copies code bytes into it.
3. `mprotect`s to `PROT_READ | PROT_EXEC` (no `MAP_JIT` entitlement required).
4. Calls `fn_name(arg)` via AAPCS64 (`x0` in/out) and returns the result.

This avoids the full `ld` + subprocess path (~200ms ld + ~30ms exec),
bringing per-call overhead to <1ms.

### Bug fix: comparison type inference (`infer_aot_type`)

Previously `infer_aot_type` always returned `"bool"` for `cmp_*`
instructions.  This produced `cmp_lt_bool` in the CIR, which the ARM64
backend lowered with an **unsigned** condition code.  For non-negative
values this is harmless, but `cmp_lt_u64(-5, 0)` evaluates false because
`-5` is stored as `0xFFFFFFFFFFFFFFFF` — a large unsigned number.

The fix: `infer_aot_type` for `cmp_*` now returns the **operand type**
(resolved from the first source via `resolve_src_aot_type`), falling
back to `"bool"` only when operands are still unresolved.

- Untyped path (u64 params): `cmp_lt` → `cmp_lt_u64` (unsigned, same as before).
- Typed path (i64 params): `cmp_lt` → `cmp_lt_i64` (signed, correct for negatives).

### Internal: `compile_module_to_text_raw`

The existing `compile_module_to_text` (clone + prepare + compile) was
split into two functions: `compile_module_to_text` (prep delegates to
`compile_module_to_text_raw`) and `compile_module_to_text_raw` (raw
two-pass compile + link, no prep).  The typed API uses `_raw` directly.

### Why `compile_typed_module_to_arm64_bytes` still runs propagation

`iir-type-checker::infer_function` seeds its SSA environment only from
instruction dests — **not** from `func.params`.  Instructions of the
form `sub dest, param_var, const` therefore stay `"any"` after the type
checker.  `propagate_aot_types` (seeded from `func.params`) fills these
in.  The propagation pass deliberately does NOT call
`normalize_params_to_u64`, so typed i64 params propagate as `i64`.

## 0.1.3 — 2026-05-13

**AOT preparation pipeline — cross-function fib compiles and runs.**

The AOT pipeline previously failed to compile recursive Twig programs
(like fibonacci) because:
1. `call_builtin "+"` / `call_builtin "_move"` instructions were left
   unresolved when the ARM64 backend received the CIR — both are
   `UnsupportedOp` in V1.
2. All function parameters had type `"any"`, which blocked
   `aot_specialise`'s type-specialisation logic (it can only lower
   `call_builtin "+"` → `add_u64` when it knows the operand types).
3. The two-pass linker for cross-function `BL` patching was implemented
   but not yet exercised.

### New: `prepare_module_for_aot` pipeline

A three-step IIR preparation pass now runs before `aot_specialise`:

1. **`pre_lower_aot_builtins`** — converts `call_builtin "+" a b` →
   `add a b`, `call_builtin "_move" n` → `mov n`, etc.  (mirrors the
   JVM/CLR/WASM pre-lowering passes).
2. **`normalize_params_to_u64`** — promotes every `"any"` param type
   to `"u64"` so `infer_types` can seed the type environment from
   params and propagate concrete types through arithmetic chains.
3. **`propagate_aot_types` + `default_any_to_u64`** — fixed-point type
   propagation (seeds from params, handles `const`, `cmp_*`,
   arithmetic, `mov`) followed by defaulting any remaining `"any"`
   arithmetic instructions to `"u64"`.  This ensures `aot_specialise`
   never emits `type_assert` guards (which the ARM64 backend lowers to
   `udf` hard-traps).

### New: `"mov"` handling in `aot-core::specialise`

`aot_specialise` now lowers `mov dest, src` (produced by
`pre_lower_aot_builtins`) to `mov_<ty>` so the ARM64 backend can emit
a typed stack-spill load/store pair.

### End-to-end result

`fib(10)` compiles and executes natively, returning `55`.

```text
AOT (ARM64 native)    224 ms    55  ✅ PASS
```

The two-pass cross-function BL linker (landed in 0.1.2) is now
exercised for real by the mutual recursion in `fib` → `fib`.

New test: `fib_compiles_ok` — asserts the full fib program compiles to
a valid Mach-O object without error.

## 0.1.2 — 2026-05-10

**LANG25-25A — Windows compilation hygiene.**

- `compile_file_macos_arm64` is now defined on all platforms.  On non-Unix
  hosts (Windows) the function returns `AotError::Linker` with a clear
  "requires Unix host" message.  Previously the `#[cfg(unix)]` gate made the
  function undefined on Windows, causing the `twig-aot` binary to fail
  `cargo check` on that platform.

- `tests/macos_arm64_smoke.rs`: wrapped `use std::os::unix::fs::PermissionsExt`
  in `#[cfg(unix)]` so the test file compiles on Windows (all callers are
  already `#[cfg(all(target_os = "macos", ...))]` which is a strict subset
  of unix).

## 0.1.1 — 2026-05-05

Real Twig source programs now compile and run on Apple Silicon — not
just hand-built IIR.  This release does NOT touch `twig-aot` itself
but pulls in upstream improvements that turn typed Twig source into
fully-resolved CIR + native code:

- `aot-core::specialise` now lowers `call_builtin "+ / - / * / / / = /
  != / < / <= / > / >= / _move"` to typed CIR ops (`add_<ty>`,
  `cmp_eq_<ty>`, `mov_<ty>`) when operand types are known, eliminating
  runtime calls for primitive arithmetic.
- `aarch64-backend` adds `mov_<ty>` lowering and fixes a stack-frame
  bug where virtual register slot 0 collided with the saved `fp/lr`
  (binaries previously SIGSEGV'd at function return).

End-to-end demonstration:

```
$ cat hello.twig
(+ 30 12)
$ twig-aot hello.twig -o hello && ./hello; echo $?
42
```

The integration test suite now runs 8 typed Twig programs through the
full pipeline and asserts their exit codes (see
`tests/macos_arm64_smoke.rs::end_to_end_typed_twig_arithmetic_and_branches`).

## 0.1.0 — 2026-05-05

Initial release.  End-to-end ahead-of-time compiler for Twig: source
file in, runnable native ARM64 Mach-O executable out.

### Pipeline

```
Twig source
   ↓ twig-ir-compiler
IIRModule
   ↓ aot-core (infer + specialise) → CIR
   ↓ aarch64-backend (compile_function) → ARM64 bytes
Vec<(fn, bytes)>
   ↓ aot-core::link → (text, offsets)
   ↓ code-packager::macho_object → MH_OBJECT
.o object file
   ↓ ld -arch arm64 -platform_version macos 15.0 15.0 -e _main -lSystem
runnable Mach-O executable
```

### Why we shell out to `ld`

On macOS 15+ (Sequoia / Tahoe) the kernel attaches a "provenance" tag
to every executable file, recording which process wrote it.  Files
written by Apple's system linker (`/usr/bin/ld`) inherit a trusted
provenance and run normally; files written by random user code are
SIGKILL'd by `AppleSystemPolicy` regardless of how well-formed the
Mach-O is.  Delegating the final link to `ld` solves that — and as a
bonus `ld` handles dyld setup, ad-hoc code signing, and SDK
versioning for us.

### CLI

Argument parsing is driven by `cli-builder` with a JSON spec
(`twig_aot.cli.json`) embedded at compile time.  `--help` and
`--version` are auto-generated.

```
twig-aot <FILE.twig> [-o <OUT>]
twig-aot --help
twig-aot --version
```

### Test coverage

- `module_with_no_entry_point_errors` — error path unit test
- `untyped_twig_returns_backend_refused` — surfaces unsupported opcodes
- `empty_main_compiles_to_object_bytes` — object-file structure
- **`end_to_end_object_through_ld_returns_42`** — real `ld` invocation,
  binary writes to disk, kernel `exec()`s it, asserts exit code 42
- **`end_to_end_typed_twig_returns_42`** — typed-IIR-via-API flow

The two E2E tests are gated to `aarch64-darwin`.

### Known limitation

The V1 ARM64 backend (PR #2156) doesn't yet lower `global_set` /
closure / property opcodes, so any Twig source that uses top-level
value defines (`(define x 5)`) or closures fails with
`AotError::BackendRefused`.  Hand-built typed IIR (function defines)
works end-to-end today.
