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

