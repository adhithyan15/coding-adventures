/* twig_runtime.c — Portable I/O helpers for the Twig AOT runtime.
 *
 * These functions are called by AOT-compiled programs (Twig, Nib, Brainfuck,
 * BASIC, Oct — anything that targets the LANG VM AOT chain) via CALL / BL
 * instructions whose targets are declared as undefined external symbols in
 * the object files produced by `code-packager`.  The system linker (`ld`,
 * `lld`, `link.exe`) resolves them from this static archive, which is
 * compiled at `twig-aot` build time via `build.rs` using the `cc` crate.
 *
 * Design rationale
 * ────────────────
 * The LANG40 prototype hard-coded macOS `write(2)` syscall numbers
 * directly into the ARM64 backend.  That approach is non-portable: the
 * syscall ABI differs between macOS and Linux, and embeds kernel-version
 * assumptions into compiled-in machine code.
 *
 * LANG41 replaced that with this runtime library, which uses standard
 * POSIX `<stdio.h>` functions.  On macOS these resolve through
 * `-lSystem`; on Linux through `-lc`; on Windows through `libcmt.lib` (or
 * `msvcrt.lib` for MinGW).  The backends only emit a single CALL/BL with
 * an external relocation — no platform-specific bytes.
 *
 * LANG75 generalises this from one hard-coded `print_i64` helper to a V1
 * helper table.  Frontends emit a single CIR opcode `call_builtin
 * "<name>", <args>`; both x86_64 and aarch64 backends prepend `__twig_`
 * to form the linker symbol and emit the call.  No backend changes are
 * required to add a new helper — just add the C function here and (if
 * the helper has a new signature pattern) extend the V1 table in each
 * backend.
 *
 * V1 helpers
 * ──────────
 * | Symbol                | Purpose                                      |
 * |-----------------------|----------------------------------------------|
 * | `__twig_print_i64`    | Print a signed 64-bit integer + newline.     |
 * | `__twig_putchar`      | Write one byte to stdout.                    |
 * | `__twig_getchar`      | Read one byte from stdin (-1 on EOF).        |
 * | `__twig_print_string` | Write `len` bytes from `ptr` to stdout.      |
 * | `__twig_input_i64`    | Read a line and parse it as a signed int64.  |
 * | `__twig_exit`         | Terminate the program with the given code.   |
 * | `__twig_str_eq`       | LANG-STR-RT: byte-equal two length-prefixed strings. |
 *
 * Adding new runtime helpers
 * ─────────────────────────
 * 1. Add a function here (use `int32_t` / `int64_t` from `<stdint.h>` for
 *    Twig values so the calling convention is unambiguous).
 * 2. Add a `BuiltinSig` entry to `V1_BUILTINS` in `x86_64-backend` and
 *    `aarch64-backend` so `call_builtin "<name>"` dispatches to the new
 *    symbol.
 * 3. No changes needed to `code-packager` or `twig-aot`'s linker pass —
 *    unresolved CALL / BL targets are automatically collected and emitted
 *    as `R_X86_64_PLT32` / `IMAGE_REL_AMD64_REL32` / `ARM64_RELOC_BRANCH26`
 *    records.
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* __twig_print_i64 — print a signed 64-bit integer followed by a newline.
 *
 * Calling convention:
 *   SysV / AAPCS64: val arrives in rdi / x0.
 *   MS x64:         val arrives in rcx.
 *
 * `fflush(stdout)` ensures the output appears even when stdout is
 * line-buffered (common when redirected to a file or pipe).
 */
void __twig_print_i64(int64_t val) {
    printf("%lld\n", (long long)val);
    fflush(stdout);
}

/* __twig_putchar — write one byte to stdout.
 *
 * The argument is an `int32_t` so the calling convention is unambiguous
 * across ABIs (32-bit ints are passed in the low half of the same arg
 * register on every supported target).  Only the low 8 bits are written;
 * higher bits are ignored.
 *
 * Note: no fflush — hot loops (`+++++.`) that emit many bytes would be
 * unbearably slow with a per-byte flush.  Callers that need synchronous
 * output should follow up with their own flush helper (TBD; not in V1).
 */
void __twig_putchar(int32_t c) {
    fputc((unsigned char)c, stdout);
}

/* __twig_getchar — read one byte from stdin.
 *
 * Returns the byte zero-extended to an `int32_t`, or -1 on EOF / error.
 * The -1 sentinel matches the C standard `getchar()` so existing BF
 * programs that loop on `,` semantics work without modification.
 */
int32_t __twig_getchar(void) {
    int c = fgetc(stdin);
    return (int32_t)c;
}

/* __twig_print_string — write `len` bytes from `ptr` to stdout.
 *
 * The frontend supplies the byte length explicitly; this helper does NOT
 * null-terminate or scan for `\0`, so it is safe with string slices that
 * contain embedded NULs.  Returns nothing; `fwrite` errors are silently
 * swallowed (matches the LANG40 `print_i64` behaviour — V1 is permissive).
 *
 * Null pointer or zero length is a no-op.
 */
void __twig_print_string(const char *s, int64_t len) {
    if (s != NULL && len > 0) {
        fwrite(s, 1, (size_t)len, stdout);
    }
}

/* __twig_input_i64 — read one line from stdin and parse a signed int64.
 *
 * Reads up to 63 bytes plus the terminating NUL into a stack buffer,
 * then calls `sscanf(... "%lld" ...)` on the result.  On parse failure
 * or EOF, returns 0.  This matches the spec's "V1 is intentionally
 * permissive — security-hardened input parsing is a follow-up".
 *
 * The fixed-size stack buffer protects against unbounded input; if the
 * user types more than 63 characters, the remainder stays in the stdio
 * buffer and is consumed on the next call (or by the program's exit).
 */
int64_t __twig_input_i64(void) {
    char buf[64];
    if (fgets(buf, sizeof(buf), stdin) == NULL) {
        return 0;
    }
    long long v = 0;
    /* sscanf returns the number of successfully-parsed conversions;
     * we don't check it because the spec says "0 on parse failure". */
    sscanf(buf, "%lld", &v);
    return (int64_t)v;
}

/* __twig_exit — terminate the program with the given exit code.
 *
 * Marked `noreturn` so the optimiser doesn't generate dead code after
 * the call site.  The code is an `int32_t` so the calling convention
 * matches `putchar` and `getchar`.
 */
#if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn))
#elif defined(_MSC_VER)
__declspec(noreturn)
#endif
void __twig_exit(int32_t code) {
    exit((int)code);
}

/* __twig_str_eq — compare two LANG-STR-RT strings for equality (LANG-STR-RT).
 *
 * Every native-AOT string buffer has the layout:
 *   offset 0  : int64_t length
 *   offset 8  : char bytes[0..length)
 *
 * Returns 1 (true) when the strings have equal length and identical bytes,
 * 0 (false) otherwise.  Both pointers must be non-null and valid (callers
 * are responsible for ensuring this — V1 does not null-check).
 */
int64_t __twig_str_eq(int64_t a, int64_t b) {
    int64_t len_a = *(int64_t *)a;
    int64_t len_b = *(int64_t *)b;
    if (len_a != len_b) return 0;
    if (len_a == 0) return 1;
    return memcmp((const char *)a + 8, (const char *)b + 8, (size_t)len_a) == 0 ? 1 : 0;
}

/* __twig_alloc_bytes — allocate `n` zero-initialised bytes on the heap
 * (LANG76).
 *
 * Returns a 64-bit pointer.  The pointer is valid until process exit;
 * V1 has no `__twig_free` and intentionally leaks — fine for AOT'd
 * command-line scripts.
 *
 * `calloc(1, n)` is used (rather than `malloc(n) + memset`) so the
 * compiler/libc can take the zero-initialised-page fast path when one
 * is available.  Negative or zero `n` returns NULL — programs that
 * dereference the result without a null check will crash, which is
 * acceptable per the V1 "no bounds checking, no null checking"
 * contract.
 *
 * The returned `void*` is treated as `int64_t` by the frontend (the
 * AAPCS64 / System V / MS x64 ABIs all return pointers in `x0` / `rax`
 * regardless of pointer-vs-integer typing, so no type coercion is
 * needed at the call site).
 */
int64_t __twig_alloc_bytes(int64_t n) {
    if (n <= 0) return 0;
    void *p = calloc(1, (size_t)n);
    return (int64_t)(intptr_t)p;
}
