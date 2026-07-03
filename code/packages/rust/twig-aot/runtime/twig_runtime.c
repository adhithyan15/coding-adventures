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
 * | `__twig_str_len`      | E4-dyn: byte length of a length-prefixed string.     |
 * | `__twig_str_concat`   | E4-dyn: fresh block = a's bytes then b's.            |
 * | `__twig_str_slice`    | E4-dyn: fresh block = bytes [start,end); traps OOB.  |
 * | `__twig_str_index`    | E4-dyn: byte at index i (0..255); traps OOB.         |
 * | `__twig_str_cmp`      | E4-dyn: lexicographic byte compare → -1 / 0 / 1.     |
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
    /* Null-pointer guard: a 0 value means the caller passed an uninitialized
     * slot (e.g. from a compiler bug).  Treat as "not equal" rather than
     * crashing on the dereference below. */
    if (a == 0 || b == 0) return 0;
    int64_t len_a = *(int64_t *)a;
    int64_t len_b = *(int64_t *)b;
    /* Negative-length guard: if the header is negative (corrupt buffer or
     * adversarial write), casting to size_t would produce a huge value and
     * cause memcmp to over-read the heap.  Treat as "not equal". */
    if (len_a < 0 || len_b < 0) return 0;
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

/* ─── LANG-FULL E4-dyn: runtime (dynamic) string helpers ──────────────────
 *
 * A runtime string is the SAME length-prefixed heap block E5 arrays use:
 *   offset 0 : int64_t length  (byte count, always >= 0)
 *   offset 8 : the `length` bytes
 * The "handle" is the block base pointer carried as an int64_t (identical to
 * __twig_alloc_bytes / __twig_str_eq above).  These helpers let a string built
 * at RUN TIME (a concat of two variables, a slice of an input, a value chosen
 * by a branch) be a first-class value on the static backends — the same way
 * __twig_str_eq already compares runtime blocks.
 *
 * Immutability: every helper that *produces* a string allocates a FRESH block
 * and copies into it; it never writes through an operand handle.  This upholds
 * E4's immutable-value contract.
 *
 * Bounds: __twig_str_slice / __twig_str_index enforce the E4 bounds contract
 * (§2.2 of lang-full-e4-strings.md) and `abort()` on an out-of-range access —
 * the runtime twin of the static backends' `udf`/`ud2`/`llvm.trap` array-bounds
 * trap.  The metadata helpers (`__twig_str_len`, `__twig_str_cmp`) stay
 * permissive on a null/corrupt handle (treat as the empty string), matching
 * __twig_str_eq, so a compiler bug degrades to a wrong-but-safe answer rather
 * than a crash.
 */

/* twig_str_len_checked — read a block's length header, validating the handle.
 * Returns the (non-negative) length, or -1 when the handle is null or the
 * header is negative (a corrupt or adversarially-written buffer).  A negative
 * header cast to size_t would be enormous and make a downstream memcpy/memcmp
 * over-read the heap, so every helper routes through this guard. */
static int64_t twig_str_len_checked(int64_t s) {
    if (s == 0) return -1;
    int64_t len = *(const int64_t *)(intptr_t)s;
    if (len < 0) return -1;
    return len;
}

/* __twig_str_len — the byte length of a runtime string (the offset-0 header).
 * A null/corrupt handle yields 0 (permissive, matching __twig_str_eq). */
int64_t __twig_str_len(int64_t s) {
    int64_t len = twig_str_len_checked(s);
    return len < 0 ? 0 : len;
}

/* __twig_str_concat — a fresh block holding a's bytes followed by b's.
 * Null/corrupt operands are treated as the empty string.  The `+ 8` and the
 * length sum are overflow-guarded before reaching __twig_alloc_bytes (which
 * would otherwise cast a wrapped negative to a huge size_t); a real string
 * never approaches INT64_MAX, so the guards only fire on corruption, where an
 * `abort()` trap is the safe outcome.  Allocation failure also traps rather
 * than returning a NULL handle a caller would dereference. */
int64_t __twig_str_concat(int64_t a, int64_t b) {
    int64_t la = twig_str_len_checked(a);
    int64_t lb = twig_str_len_checked(b);
    if (la < 0) la = 0;
    if (lb < 0) lb = 0;
    if (la > INT64_MAX - lb) abort();          /* la + lb overflow */
    int64_t total = la + lb;
    if (total > INT64_MAX - 8) abort();         /* total + 8 overflow */
    int64_t handle = __twig_alloc_bytes(total + 8);
    if (handle == 0) abort();                   /* OOM */
    *(int64_t *)(intptr_t)handle = total;
    char *dst = (char *)(intptr_t)handle + 8;
    if (la > 0) memcpy(dst,      (const char *)(intptr_t)a + 8, (size_t)la);
    if (lb > 0) memcpy(dst + la, (const char *)(intptr_t)b + 8, (size_t)lb);
    return handle;
}

/* __twig_str_slice — a fresh block holding bytes [start, end) of `s`.
 * E4 bounds contract: 0 <= start <= end <= len, else TRAP.  A null/corrupt
 * source also traps (there is no in-range slice of nothing).  `end - start`
 * cannot overflow because both are bounded by `len >= 0`. */
int64_t __twig_str_slice(int64_t s, int64_t start, int64_t end) {
    int64_t len = twig_str_len_checked(s);
    if (len < 0) abort();
    if (start < 0 || end < start || end > len) abort();
    int64_t n = end - start;
    int64_t handle = __twig_alloc_bytes(n + 8);
    if (handle == 0) abort();
    *(int64_t *)(intptr_t)handle = n;
    if (n > 0) memcpy((char *)(intptr_t)handle + 8,
                      (const char *)(intptr_t)s + 8 + start, (size_t)n);
    return handle;
}

/* __twig_str_index — the byte at index `i` of `s`, zero-extended to int64_t
 * (an unsigned 0..255 byte value).  E4 bounds contract: 0 <= i < len, else
 * TRAP. */
int64_t __twig_str_index(int64_t s, int64_t i) {
    int64_t len = twig_str_len_checked(s);
    if (len < 0) abort();
    if (i < 0 || i >= len) abort();
    unsigned char byte = *((const unsigned char *)(intptr_t)s + 8 + i);
    return (int64_t)byte;
}

/* __twig_str_cmp — lexicographic byte comparison: -1 if a<b, 0 if equal,
 * 1 if a>b.  Compares the first min(len) bytes; on a tie the shorter string is
 * "less" (strcmp / String.compareTo semantics over unsigned byte values).
 * Null/corrupt operands are treated as the empty string, so a null handle
 * never reaches the guarded memcmp (n collapses to 0). */
int64_t __twig_str_cmp(int64_t a, int64_t b) {
    int64_t la = twig_str_len_checked(a);
    int64_t lb = twig_str_len_checked(b);
    if (la < 0) la = 0;
    if (lb < 0) lb = 0;
    int64_t n = la < lb ? la : lb;
    if (n > 0) {
        int c = memcmp((const char *)(intptr_t)a + 8,
                       (const char *)(intptr_t)b + 8, (size_t)n);
        if (c < 0) return -1;
        if (c > 0) return 1;
    }
    if (la < lb) return -1;
    if (la > lb) return 1;
    return 0;
}
