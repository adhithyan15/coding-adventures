/* twig_runtime.c — Portable I/O helpers for the Twig AOT runtime.
 *
 * These functions are called by AOT-compiled Twig programs via BL
 * instructions whose targets are declared as undefined external symbols
 * in the Mach-O object file produced by `code-packager`.  The system
 * linker (`ld`) resolves them from this static archive, which is compiled
 * at `twig-aot` build time via `build.rs` using the `cc` crate.
 *
 * Design rationale
 * ────────────────
 * The LANG40 prototype hard-coded macOS `write(2)` syscall numbers
 * (`x16 = 4`, `SVC #0x80`) directly into the ARM64 backend
 * (`emit_print_helper`).  That approach is non-portable: the syscall ABI
 * differs between macOS and Linux, and embeds kernel-version assumptions
 * into compiled-in machine code.
 *
 * LANG41 replaces that with this runtime library, which uses standard
 * POSIX `<stdio.h>` functions.  On macOS these resolve through
 * `-lSystem`; on Linux through `-lc`.  The ARM64 backend only emits a
 * `BL __twig_print_i64` external reloc — no platform-specific bytes.
 *
 * Adding new runtime helpers
 * ─────────────────────────
 * 1. Add a function here (use `int64_t` from `<stdint.h>` for Twig values).
 * 2. Emit a `BL __your_helper` external reloc from the appropriate CIR
 *    opcode handler in `aarch64-backend/src/lib.rs`.
 * 3. No changes needed to `code-packager` or `twig-aot`'s linker pass —
 *    unresolved BL targets are automatically collected and emitted as
 *    `ARM64_RELOC_BRANCH26` records.
 */

#include <stdio.h>
#include <stdint.h>

/* __twig_print_i64 — print a signed 64-bit integer followed by a newline.
 *
 * Calling convention (AAPCS64):
 *   val arrives in x0 (the first integer argument register).
 *   The return value (void) is ignored.
 *
 * The ARM64 backend's `io_out` CIR opcode handler emits:
 *
 *   LDR  X0, [sp, #<slot>]   ; load Twig integer value into X0
 *   BL   __twig_print_i64    ; call this function
 *
 * `fflush(stdout)` ensures the output appears even if stdout is line-
 * buffered (common when redirected to a file or pipe).
 */
void __twig_print_i64(int64_t val) {
    printf("%lld\n", (long long)val);
    fflush(stdout);
}
