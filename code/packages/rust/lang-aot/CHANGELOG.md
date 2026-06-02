# Changelog — `lang-aot`

## 0.7.0 — 2026-06-02 (A1+++ — `--emit=riscv32` + iir-to-riscv wiring)

### Added — `--emit=riscv32` flag and `compile_file_to_riscv32_bin` API

Wires `iir-to-riscv` (v0.3.3) into the lang-aot driver.  Source files
for every supported language (Twig, Nib, Brainfuck, BASIC, Oct) can
now be lowered to a flat `.bin` of little-endian 32-bit RV32I
instruction words via:

```text
lang-aot path/to/input.bas --emit=riscv32 [-o out.bin]
```

Aliases accepted for the value: `riscv32` (canonical), `rv32`, `bin`.

When `-o` is omitted, the default output is the input with the
extension replaced by `.bin` (matching the conventional flat ELF-less
RV32I name downstream simulators / `qemu-riscv32` expect).

#### Downstream consumers

* [`riscv-simulator`](../riscv-simulator) — load + execute in-process.
* `qemu-riscv32 -kernel out.bin` — host-side simulation.
* Physical flash loader on a SiFive / ESP32-C3 / RISC-V board.

#### Wire format

Each emitted word is written as **little-endian** bytes per the
RISC-V spec (Volume I §1.4): bit `[7:0]` of the word goes to the
lowest-address byte.

#### Why cross-platform (no host gating)

The native-executable pipelines (`compile_file_to_{linux,windows,macos}_executable`)
are `cfg`-gated because they invoke the host linker.  RV32I `.bin`
emission is **pure byte output** — `compile_file_to_riscv32_bin` runs
on any host.  Downstream loading / running is the caller's job.

#### Public API added

* `pub fn compile_file_to_riscv32_bin(src: &Path, out: &Path,
   language: Language) -> Result<(), LangAotError>`
* `LangAotError::RiscvBackendError(String)` — wraps human-readable
  errors surfaced by `iir-to-riscv`.

#### CLI flag reference

```text
--emit=<MODE>     What to emit:
                    native           → host executable (default)
                    llvm-ir          → textual LLVM IR (.ll)
                    riscv32 | rv32 | bin
                                     → flat RV32I .bin
```

#### Tests added (28 total, was 27)

* `end_to_end_basic_print_emits_riscv32_bin_via_lang_aot` —
  cross-platform e2e: BASIC `PRINT 42` → `.bin`.  Asserts:
  non-empty, 4-byte aligned, last 4 bytes = `0x67 0x80 0x00 0x00`
  (canonical `ret` little-endian).  Tolerates not-yet-covered op
  gaps via the same skip pattern as the LLVM e2e test.

## 0.6.0 — 2026-06-01 (LLVM04 — `--emit=llvm-ir` + iir-to-llvm wiring)

### Added — `--emit=llvm-ir` flag and `compile_file_to_llvm_ir` API

Wires `iir-to-llvm` (v0.4.0) into the lang-aot driver.  Source files for
every supported language (Twig, Nib, Brainfuck, BASIC, Oct) can now be
lowered to textual LLVM IR (`.ll`) via:

```text
lang-aot path/to/input.bas --emit=llvm-ir [-o out.ll]
```

When `-o` is omitted the default output is the input with the extension
replaced by `.ll` (matching what downstream `llc` / `opt` expect).
Accepted aliases for the value: `llvm-ir` (canonical), `llvm`, `ll`.

#### Why cross-platform (no host gating)

The native-executable pipelines (`compile_file_to_{linux,windows,macos}_executable`)
are `cfg`-gated because they invoke the host linker.  LLVM IR emission
is **pure string output** — `compile_file_to_llvm_ir` is therefore
cross-platform and runs on any host.  Downstream `llc` / `opt`
invocations are the caller's job.

#### Public API surface added

* `pub fn compile_file_to_llvm_ir(src: &Path, out: &Path, language: Language)
   -> Result<(), LangAotError>`
* `LangAotError::LlvmBackendError(String)` — wraps human-readable errors
  surfaced by `iir-to-llvm`'s lowerer.

#### Tests added (27 total, was 25)

* `end_to_end_twig_emits_llvm_ir_via_lang_aot`
* `end_to_end_basic_print_emits_llvm_ir_with_print_extern`

Both cross-platform.  Tolerate unsupported-op / unsupported-type errors
from `iir-to-llvm` as "expected gaps" (a future LLVM05+ will broaden
coverage).  The BASIC test asserts on the `@__print_i64` extern shape.

## 0.5.0 — 2026-05-30 (AOT05 — BASIC + Oct smoke parity with Nib)

### Added — 6 new end-to-end smoke tests (BASIC + Oct)

Brings BASIC and Oct's lang-aot smoke coverage from 2 tests each to
5 tests each, matching Nib's breadth.  Closes task #32 from the
multi-language tooling parity work.

#### Oct — 3 new tests (was 2)

- `end_to_end_oct_if_else_exits_zero` — `if x == 0 { x = 1; } else
  { x = 2; }` compiles, links, runs successfully.  Exercises typed
  `cmp_eq` + `jmp_if_false` + `mov` + `jmp` + `label` through native
  codegen.
- `end_to_end_oct_while_loop_exits_zero` — `while n < 10 { n = n + 1; }`
  compiles and runs to completion.  Exercises backward `jmp` (the
  AOT chain's branch-distance encoding) and the typed `cmp_lt`+`add`
  loop body.
- `end_to_end_oct_cross_fn_chain_exits_zero` — `add_one(add_one(8))`
  chains two cross-fn calls through the typed-argument reloc path.

#### BASIC — 3 new tests (was 2)

- `end_to_end_basic_arith_chain_prints_42` — `A + B + C` printed.
  Exercises multiple typed `add` ops through the AOT pipeline.
- `end_to_end_basic_if_then_prints_1` — `IF A > 5 THEN 100` takes
  the then branch, prints 1.  Exercises typed `cmp_gt` +
  `jmp_if_*` with line-label resolution.
- `end_to_end_basic_goto_prints_1` — `GOTO 100` skips the
  assignment on line 30, prints A's original value.  Exercises
  forward unconditional branch resolution.

### Coverage parity

| Language | Smoke tests (before) | Smoke tests (now) |
|---|---|---|
| Twig | 1 | 1 |
| Nib | 5 | 5 |
| Brainfuck | 1 | 1 |
| **BASIC** | **2** | **5** |
| **Oct** | **2** | **5** |

### Tests

All 17 smoke tests pass on the local host platform.  Each test is
gated to its host OS (`#[cfg(target_os = ...)]`) so CI runners only
execute the tests appropriate to their platform.

## 0.4.0 — 2026-05-20 (OCT02 phase 4 — Oct end-to-end on LANG VM)

Oct programs now compile end-to-end via `oct-iir-compiler` (OCT02 phase 3,
PR #3878).  Closes the final phase of the OCT02 four-phase plan — every
language in the LANG74 roadmap (Twig, Nib, Brainfuck, Dartmouth BASIC,
Oct) now ships through the shared LANG VM AOT chain.

**Dispatch wiring.**  `compile_source_to_iir`'s `Language::Oct` arm now
calls `oct_iir_compiler::compile_source` and surfaces frontend errors
(`Unsupported8008Intrinsic`, `Type`, `Parse`) through
`LangAotError::FrontendError`.  The `UnsupportedLanguage` arm is no
longer reachable for any built-in `Language` variant — kept in the
enum so adding a new variant remains a one-arm change.

**End-to-end smoke tests** on both Windows + Linux:

- `end_to_end_oct_minimal_main_exits_zero`: `fn main() { let x: u8 = 42; }`
  compiles + links + runs + exits with the synthesised i64-return code 0.
- `end_to_end_oct_user_fn_call_succeeds`: program with `fn double(a: u8) -> u8 { return a + a; }` and `fn main() { let x: u8 = double(21); }` exercises the cross-function `call` reloc.

Verified locally on Windows.

**Lib test updates.**  `oct_returns_clean_unsupported_error` →
`oct_compiles_to_iir`; new `oct_8008_intrinsic_reports_frontend_error`
confirms the rejection path still surfaces a clean error.

## 0.3.0 — 2026-05-20 (PL05 — Dartmouth BASIC end-to-end on LANG VM)

Dartmouth BASIC programs now compile end-to-end via the new
`dartmouth-basic-iir-compiler` crate.  `lang-aot foo.bas` produces a
native executable on Linux, Windows, and macOS — the same chain Twig,
Nib, and Brainfuck use.

**Wiring.**  The `Language::DartmouthBasic` arm in
`compile_source_to_iir` now calls
`dartmouth_basic_iir_compiler::compile_source` instead of returning
`UnsupportedLanguage`.  No other changes to the lang-aot surface —
the existing `compile_file_to_*_executable` entry points handle
BASIC transparently.

**V1 BASIC coverage.**  Integer-only programs with LET / PRINT /
INPUT / IF / GOTO / FOR / NEXT / END / REM.  GOSUB/RETURN, READ/
DATA, DIM/arrays, and DEF are deferred.  See
[`dartmouth-basic-iir-compiler/CHANGELOG.md`](../dartmouth-basic-iir-compiler/CHANGELOG.md)
for the full table.

**End-to-end smoke tests:**

- `end_to_end_basic_print_42_via_lang_aot` — `10 PRINT 42 / 20 END`
  exits cleanly and writes exactly `"42\n"`.
- `end_to_end_basic_for_loop_prints_1_2_3` — `FOR I = 1 TO 3 / PRINT
  I / NEXT I / END` writes exactly `"1\n2\n3\n"`.

Verified locally on Windows.

**Lib-test renamed.**  `dartmouth_basic_returns_clean_unsupported_error`
is gone; `dartmouth_basic_compiles_to_iir` asserts the new success
path.

## 0.2.0 — 2026-05-20 (BF07 — Brainfuck end-to-end on LANG VM)

Brainfuck programs now compile all the way to a native executable via
`lang-aot foo.bf`.

**New BF lowering pass.**  `lower_brainfuck_for_aot(&mut IIRModule)`
runs after `brainfuck_iir_compiler::compile_source` returns and
rewrites the BF-shaped IIR into a LANG76-shaped one without modifying
the frontend (so existing consumers — `vm-core`, `jit-core`,
`iir-to-wasm` — keep working unchanged):

- Prepends `const __bf_tape_size = 30000` + `alloc_bytes
  __bf_tape_size -> __bf_tape` to `main`.
- Rewrites `load_mem v, ptr` → `load_byte __bf_tape, ptr -> v`.
- Rewrites `store_mem ptr, v` → `store_byte __bf_tape, ptr, v`.
- Replaces the trailing `ret_void` with `const __bf_ret = 0; ret
  __bf_ret`, changing `main`'s return type from `void` to `i64` so
  the LANG VM AOT chain's entry-point convention (exit code = main's
  return value) is satisfied.

**End-to-end smoke test:** `end_to_end_brainfuck_prints_a_via_lang_aot`
on both Windows + Linux compiles `++++++++[>++++++++<-]>+.` (canonical
"print 'A'") through `lang-aot` and asserts stdout is exactly `"A"`.
This exercises every mechanic LANG75 + LANG76 deliver: pointer shift,
cell mutation, nested loops, the 30000-byte tape, and putchar.
Verified locally on Windows.

**Lib test:** `brainfuck_lowering_inserts_tape_and_byte_ops` asserts
the lowering pass produces the expected IIR shape (alloc_bytes
preamble, no leftover load_mem/store_mem, ret/i64 epilogue) without
needing the linker.

## 0.1.0 — 2026-05-20

Initial release.  Multi-language AOT driver that routes Twig, Nib, and
Brainfuck source through the shared LANG VM chain (frontend → IIR →
x86_64-backend / aarch64-backend → object → system linker → native
executable).

### What's wired

| Language | Extensions | Frontend |
|---|---|---|
| Twig | `.twig` | `twig-ir-compiler` |
| Nib  | `.nib`  | `nib-iir-compiler` |
| Brainfuck | `.bf`, `.b` | `brainfuck-iir-compiler` (IIR-emission works; AOT backend doesn't lower BF ops yet) |
| Dartmouth BASIC | `.bas`, `.basic` | placeholder — returns `UnsupportedLanguage` with guidance |
| Oct | `.oct` | placeholder — returns `UnsupportedLanguage` with guidance |

### API

- `Language` enum with `parse(&str)` and `Display`.
- `detect_language_from_path(&Path) -> Option<Language>` — by extension.
- `compile_source_to_iir(language, source, module_name) -> Result<IIRModule, LangAotError>`
  — frontend dispatch.
- `compile_file_to_{linux, windows, macos}_executable(src, out, lang)`
  — full pipeline, cfg-gated to the matching host (same host-targets-
  host policy as `twig-aot`).
- `LangAotError` with `UnsupportedLanguage { language, guidance }`,
  `FrontendError`, `AotError`, `Io` variants.

### Companion change in `twig-aot`

`twig-aot` exposes three new public functions:

- `compile_module_to_linux_executable(&IIRModule, &Path)` (Linux host).
- `compile_module_to_windows_executable(&IIRModule, &Path)` (Windows host).
- `compile_module_to_macos_executable(&IIRModule, &Path)` (Unix host).

…and three new public link helpers:

- `link_linux_x86_64_executable(obj_bytes, stem, out)`.
- `link_windows_x86_64_executable(obj_bytes, stem, out)`.
- `link_macos_arm64_executable(obj_bytes, stem, out)`.

The existing `compile_file_*` functions now delegate to these so the
link logic is shared between source-file input and module input.

### Tests

- 7 lib tests cover language parsing, extension detection, and the
  unsupported-language error paths.
- 3 end-to-end smoke tests (`tests/end_to_end_smoke.rs`) gated to
  the host's OS:
  - `end_to_end_twig_returns_42_via_lang_aot`
  - `end_to_end_nib_returns_42_via_lang_aot`
  - `end_to_end_nib_arithmetic_via_lang_aot` (`30+12`, `if 1==1`,
    `if 1==2`)

All tests pass on Windows x86-64 host.  CI will additionally verify
on `ubuntu-latest` and `macos-latest`.

### Known limitations

- **Host-targets-host only.** Same as `twig-aot` V1.
- **No `--target` / `--emit-object` CLI flags.** Coming in a follow-up.
- **Brainfuck end-to-end gap.** Frontend produces correct IIR, but the
  x86_64-backend and aarch64-backend don't lower BF-specific ops
  (`load_mem`, `putchar`, etc.).  Wiring is correct; backend extension
  is a separate piece of work.
- **Dartmouth BASIC and Oct stubs.** They surface
  `UnsupportedLanguage` errors with one-line guidance on what's needed
  to unblock each.
