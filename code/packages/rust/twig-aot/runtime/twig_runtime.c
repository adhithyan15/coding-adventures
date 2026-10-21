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
 * | `__twig_write_i64`    | LANG83: print a signed 64-bit integer, no newline. |
 * | `__twig_write_f64_py` | LANG83 PR6a: a double as Python `repr`, no newline. |
 * | `__twig_write_f64_js` | LANG83 PR6a: a double as JS `Number#toString`, no newline. |
 * | `__twig_trap`         | LANG83: flush stdout, report the trap kind on stderr, abort. |
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

#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
/* Windows' CRT opens stdout in TEXT mode by default, which silently
 * translates every `\n` a program writes to `\r\n` -- so a Twig program's
 * output (and every other AOT frontend that shares this runtime) came out
 * "Hi\r\n" instead of "Hi\n" on Windows, diverging from every other
 * platform/backend's byte-for-byte output. `_setmode(..., _O_BINARY)`
 * disables that translation; called once per process (the `done` guard),
 * lazily on first I/O rather than at a fixed init point, since there is no
 * existing startup hook in this runtime to call it from and a
 * GCC/Clang-only `__attribute__((constructor))` wouldn't fire under
 * MSVC's `cl.exe`, which `cc` picks by default on this target. */
static void twig_runtime_ensure_stdout_binary(void) {
    static int done = 0;
    if (!done) {
        _setmode(_fileno(stdout), _O_BINARY);
        done = 1;
    }
}
#else
static void twig_runtime_ensure_stdout_binary(void) {}
#endif

/* Precise-GC kind registration + allocation (gc-core-capi), used by
 * `__twig_alloc_bytes` so runtime string blocks are managed by the collector
 * (traced + reclaimed) instead of leaking via `calloc`. A Twig string is a
 * `[int64 len][bytes]` opaque BLOB with no interior GC references, so it is
 * registered under a HeapKind with an EMPTY field map — the collector then
 * scans none of its bytes (precise: no look-alike-pointer false pinning) and
 * treats it as a movable leaf. `__gc_register_kind(NULL, 0)` returns a 1-based
 * kind id for that no-ref layout; `__gc_alloc_kind(n, kind)` returns `n`
 * zeroed, 16-aligned bytes tagged with it. Both are exported by gc-core-capi
 * and linked into the AOT image via `libgc_core_capi.a`. */
extern int64_t __gc_register_kind(const int64_t *field_offsets, int64_t count);
extern int64_t __gc_alloc_kind(int64_t n, uint16_t kind);
/* `__gc_register_ref_array_kind(fixed, fixed_count, tail_from)`: registers a
 * variable-length REFERENCE-array HeapKind -- every aligned 8-byte word in
 * `[tail_from, size)` of the allocated instance is traced as a possible heap
 * reference (validated against the live-block table before being followed,
 * so this can only ever over-retain, never crash or under-trace -- see
 * `__twig_alloc_ref_array_bytes` below). Used by `alloc_array` (LANG-FULL E5)
 * so an array whose element type carries a GC reference (`str`/`any`/
 * `symbol`/`ref<T>`) doesn't leave its elements invisible to the collector. */
extern int64_t __gc_register_ref_array_kind(const int64_t *fixed, int64_t fixed_count, int64_t tail_from);

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
    twig_runtime_ensure_stdout_binary();
    printf("%lld\n", (long long)val);
    fflush(stdout);
}

/* __twig_write_i64 — LANG83 `write_i64`: a signed 64-bit integer in decimal,
 * with NO newline (Perl `print`, Python `print(x, end="")`).
 *
 * Same `FILE *` as `__twig_print_i64` / `__twig_putchar` /
 * `__twig_print_string`, so mixed output is one ordered stream. Like
 * `__twig_putchar` (and unlike `__twig_print_i64`) it does not flush per call:
 * a no-newline writer is typically called in a loop, and every exit path
 * flushes — normal `exit`, and `__twig_trap` below.
 */
void __twig_write_i64(int64_t val) {
    twig_runtime_ensure_stdout_binary();
    printf("%lld", (long long)val);
}

/* ─────────────────────────────────────────────────────────────────────────
 * LANG83 PR6a — shortest round-trip float output
 * ─────────────────────────────────────────────────────────────────────────
 *
 * `__twig_write_f64_py(bits)` prints a double the way Python's `repr` does,
 * `__twig_write_f64_js(bits)` the way JavaScript's `Number#toString` does,
 * both with NO newline. The argument is the double's raw bit pattern: the
 * native backends keep an `f64` in a frame slot as its 8 bytes and pass a
 * builtin argument in a general-purpose register (`rdi` / `x0`), so the
 * generic `call_builtin` marshalling needs no float-register path.
 *
 * Two steps, as in every LANG83 backend (spec, "PR6a"):
 *
 *   1. DIGITS — the shortest decimal string D (and exponent k, value =
 *      0.D × 10^k) that reads back as exactly x; on a length tie the closest
 *      one; on an exact tie the even one. `printf("%.17g")` is NOT this: it
 *      prints 17 digits, `0.1` as `0.10000000000000001`.
 *   2. LAYOUT — where the point goes, which is where Python and JavaScript
 *      differ (`1e+16` vs `10000000000000000`, `1.0` vs `1`).
 *
 * The digits come from the Burger–Dybvig free-format algorithm (the
 * Steele–White / Dragon4 family): exact big-integer arithmetic, no tables.
 * Picture the real line around x:
 *
 *            x - m-          x           x + m+
 *      ───────[──────────────●──────────────]───────
 *        every decimal inside this interval reads back as x
 *
 * m- and m+ are half the gaps to the neighbouring doubles (m- is half as
 * wide when x is a power of two: the gap below shrinks). The algorithm
 * emits digits of x one at a time and stops at the first digit where it
 * can round to a decimal inside the interval: that is the shortest string.
 * Everything is kept as integers over a common denominator s:
 *
 *      x = r / s,   m+ = mp / s,   m- = mm / s
 *
 * ── Big integers ──────────────────────────────────────────────────────────
 * A twig_bn is TWIG_BN_LIMBS base-2^32 digits, least significant first,
 * each held in a 64-bit word so `limb × m + carry` never overflows
 * (m < 2^32). 40 limbs = 1280 bits; the largest value any double needs is
 * below 2^1084 (measured over every power of two, the subnormal range and
 * random bit patterns — LANG83 "PR6a"), so they cannot overflow.
 */
#define TWIG_BN_LIMBS 40
#define TWIG_BN_MASK 0xFFFFFFFFull

typedef struct {
    uint64_t limb[TWIG_BN_LIMBS];
} twig_bn;

/* a = v  (v < 2^64: two limbs). */
static void twig_bn_set(twig_bn *a, uint64_t v) {
    for (int i = 0; i < TWIG_BN_LIMBS; i++) a->limb[i] = 0;
    a->limb[0] = v & TWIG_BN_MASK;
    a->limb[1] = v >> 32;
}

/* a *= m  (m < 2^32). Schoolbook: each limb times m plus the carry in. */
static void twig_bn_mul_small(twig_bn *a, uint64_t m) {
    uint64_t carry = 0;
    for (int i = 0; i < TWIG_BN_LIMBS; i++) {
        uint64_t t = a->limb[i] * m + carry;
        a->limb[i] = t & TWIG_BN_MASK;
        carry = t >> 32;
    }
}

/* a *= 2^n, 31 bits per pass (2^31 < 2^32 keeps mul_small exact). */
static void twig_bn_mul_pow2(twig_bn *a, int64_t n) {
    while (n > 0) {
        int64_t c = n < 31 ? n : 31;
        twig_bn_mul_small(a, (uint64_t)1 << c);
        n -= c;
    }
}

/* a *= 10^n, nine decimal digits per pass (10^9 < 2^32). */
static void twig_bn_mul_pow10(twig_bn *a, int64_t n) {
    while (n > 0) {
        int64_t c = n < 9 ? n : 9;
        uint64_t p = 1;
        for (int64_t i = 0; i < c; i++) p *= 10;
        twig_bn_mul_small(a, p);
        n -= c;
    }
}

/* d = a + b. */
static void twig_bn_add(twig_bn *d, const twig_bn *a, const twig_bn *b) {
    uint64_t carry = 0;
    for (int i = 0; i < TWIG_BN_LIMBS; i++) {
        uint64_t t = a->limb[i] + b->limb[i] + carry;
        d->limb[i] = t & TWIG_BN_MASK;
        carry = t >> 32;
    }
}

/* a -= b, for a >= b. Borrowing is done without signed arithmetic:
 * t = a_i + 2^32 - b_i - borrow is always >= 0; its bit 32 is set exactly
 * when no borrow was needed. */
static void twig_bn_sub(twig_bn *a, const twig_bn *b) {
    uint64_t borrow = 0;
    for (int i = 0; i < TWIG_BN_LIMBS; i++) {
        uint64_t t = a->limb[i] + ((uint64_t)1 << 32) - b->limb[i] - borrow;
        a->limb[i] = t & TWIG_BN_MASK;
        borrow = 1 - (t >> 32);
    }
}

/* -1, 0 or 1 as a <, ==, > b: compare from the most significant limb. */
static int twig_bn_cmp(const twig_bn *a, const twig_bn *b) {
    for (int i = TWIG_BN_LIMBS - 1; i >= 0; i--) {
        if (a->limb[i] != b->limb[i]) return a->limb[i] < b->limb[i] ? -1 : 1;
    }
    return 0;
}

/* The shortest digits of the finite, positive double whose bits are `mag`.
 * Writes the digits (values 0..9, not characters) to `out`, returns their
 * count and stores k (value = 0.D × 10^k). */
static int twig_f64_digits(uint64_t mag, unsigned char out[32], int64_t *k_out) {
    twig_bn r, s, mp, mm, t;
    uint64_t mant = mag & ((((uint64_t)1) << 52) - 1);
    int64_t bexp = (int64_t)(mag >> 52);
    /* x = f × 2^e exactly. Subnormals have no hidden bit and e = -1074. */
    uint64_t f = bexp == 0 ? mant : (mant | (((uint64_t)1) << 52));
    int64_t e = bexp == 0 ? -1074 : bexp - 1075;
    /* Round-half-even readers (strtod, Python, JS) accept a decimal exactly
     * on the interval's edge when f is even, so `even` widens each test by
     * one: "c > -even" is "c >= 0" when even and "c > 0" when odd. */
    int even = (f & 1) == 0;
    /* At a power of two the gap below x is half the gap above: scale
     * everything by one more 2 so m- can be half of m+. */
    int64_t closer = (mant == 0 && bexp > 1) ? 1 : 0;
    int64_t le = e > 0 ? e : 0;
    int64_t se = e < 0 ? -e : 0;
    twig_bn_set(&r, f);  twig_bn_mul_pow2(&r, le + 1 + closer);  /* r  = f·2^(le+1+c) */
    twig_bn_set(&s, 1);  twig_bn_mul_pow2(&s, se + 1 + closer);  /* s  = 2^(se+1+c)   */
    twig_bn_set(&mp, 1); twig_bn_mul_pow2(&mp, le + closer);     /* m+ = 2^(le+c)     */
    twig_bn_set(&mm, 1); twig_bn_mul_pow2(&mm, le);              /* m- = 2^le         */

    /* k ≈ log10(x): floor((e+52) · log10 2), with 1233/4096 ≈ log10 2.
     * Floor division, written so it is exact C for negative values too. */
    int64_t kn = (e + 52) * 1233;
    int64_t k = kn >= 0 ? kn / 4096 : -((-kn + 4095) / 4096);
    if (k >= 0) {
        twig_bn_mul_pow10(&s, k);
    } else {
        twig_bn_mul_pow10(&r, -k);
        twig_bn_mul_pow10(&mp, -k);
        twig_bn_mul_pow10(&mm, -k);
    }
    /* The estimate can be off (it ignores a subnormal's missing bits):
     * fix k until 0.1 <= (r + m+)/s < 1, the edge counting as `even` says. */
    for (;;) {
        twig_bn_add(&t, &r, &mp);
        if (twig_bn_cmp(&t, &s) > -even) { twig_bn_mul_small(&s, 10); k++; }
        else break;
    }
    for (;;) {
        twig_bn_add(&t, &r, &mp);
        twig_bn_mul_small(&t, 10);
        if (twig_bn_cmp(&t, &s) < even) {
            twig_bn_mul_small(&r, 10); twig_bn_mul_small(&mp, 10); twig_bn_mul_small(&mm, 10);
            k--;
        } else break;
    }

    /* Generate. Each pass: the next digit d = floor(10r / s), then ask
     * whether stopping here lands inside the interval rounding down (low)
     * or rounding up (high). The bound n < 31 never binds (n <= 17 for
     * every double); it only keeps `out` in range by construction. */
    int n = 0;
    while (n < 31) {
        twig_bn_mul_small(&r, 10); twig_bn_mul_small(&mp, 10); twig_bn_mul_small(&mm, 10);
        int d = 0;
        while (twig_bn_cmp(&r, &s) >= 0) { twig_bn_sub(&r, &s); d++; }
        int low = twig_bn_cmp(&r, &mm) < even;           /* r <= m- (even) / r < m- */
        twig_bn_add(&t, &r, &mp);
        int high = twig_bn_cmp(&t, &s) > -even;          /* r + m+ >= s / > s      */
        if (!low && !high) { out[n++] = (unsigned char)d; continue; }
        if (low && high) {
            /* Both d and d+1 read back as x: take the closer, and on an
             * exact tie (2r == s) the even one — Python's and JS's rule. */
            twig_bn_add(&t, &r, &r);
            int c = twig_bn_cmp(&t, &s);
            if (c > 0 || (c == 0 && (d & 1))) d++;
        } else if (high) {
            d++;   /* never 10: r + m+ < s held on entry, so d + 1 <= 9 */
        }
        out[n++] = (unsigned char)d;
        break;
    }
    *k_out = k;
    return n;
}

/* Lay out and print one double. `js` = 0 for Python, 1 for JavaScript.
 *
 *   e = k - 1, the exponent of the first digit (123.4 → e = 2)
 *   fixed when -4 <= e < 16 (Python) / -6 <= e < 21 (JavaScript)
 *     e < 0          → "0." + (-e-1) zeros + D             0.0001
 *     len(D) <= e+1  → D + zeros up to the units place     100 (+ ".0" in Python)
 *     otherwise      → D with a point after e+1 digits     123.456
 *   else sci         → d[.ddd]e±X (Python pads X to two digits)
 */
static void twig_write_f64(int64_t bits, int js) {
    uint64_t u = (uint64_t)bits;
    uint64_t mag = u & 0x7FFFFFFFFFFFFFFFull;
    twig_runtime_ensure_stdout_binary();
    if (mag > 0x7FF0000000000000ull) {           /* NaN, whatever its sign */
        fputs(js ? "NaN" : "nan", stdout);
        return;
    }
    if ((u >> 63) && !(js && mag == 0)) putchar('-');   /* JS prints -0 as "0" */
    if (mag == 0x7FF0000000000000ull) { fputs(js ? "Infinity" : "inf", stdout); return; }
    if (mag == 0) { fputs(js ? "0" : "0.0", stdout); return; }

    unsigned char d[32];
    int64_t k;
    int n = twig_f64_digits(mag, d, &k);
    int64_t e = k - 1;
    int64_t lo = js ? -6 : -4, hi = js ? 21 : 16;
    if (e >= lo && e < hi) {
        if (e < 0) {
            putchar('0'); putchar('.');
            for (int64_t i = 0; i < -e - 1; i++) putchar('0');
            for (int i = 0; i < n; i++) putchar('0' + d[i]);
        } else {
            for (int64_t i = 0; i <= e; i++) putchar(i < n ? '0' + d[i] : '0');
            if (n > e + 1) {
                putchar('.');
                for (int64_t i = e + 1; i < n; i++) putchar('0' + d[i]);
            } else if (!js) {
                putchar('.'); putchar('0');
            }
        }
    } else {
        putchar('0' + d[0]);
        if (n > 1) {
            putchar('.');
            for (int i = 1; i < n; i++) putchar('0' + d[i]);
        }
        putchar('e');
        putchar(e < 0 ? '-' : '+');
        int64_t a = e < 0 ? -e : e;   /* |e| <= 324 */
        if (!js && a < 10) putchar('0');
        if (a >= 100) putchar('0' + (int)(a / 100));
        if (a >= 10) putchar('0' + (int)((a / 10) % 10));
        putchar('0' + (int)(a % 10));
    }
}

/* __twig_write_f64_py — LANG83 PR6a `write_f64_py`: Python `repr(float)`,
 * no newline. Same FILE* as every other output helper (one ordered stream);
 * like `__twig_write_i64` it does not flush per call. */
void __twig_write_f64_py(int64_t bits) {
    twig_write_f64(bits, 0);
}

/* __twig_write_f64_js — LANG83 PR6a `write_f64_js`: JavaScript
 * `Number#toString`, no newline. */
void __twig_write_f64_js(int64_t bits) {
    twig_write_f64(bits, 1);
}

/* The closed LANG83 trap-kind table, indexed by stable id
 * (`interpreter_ir::exception_kind::trap_kind_id`): built-in kinds are 1..=6
 * in `ALL_TRAP_KINDS` order. Append-only; a Rust test
 * (`twig-aot` `runtime_trap_kind_table_matches_exception_kind`) pins this
 * array to the Rust table so the two cannot drift. Language kinds (ids from
 * 1000) are added by the PR that first emits one. */
static const char *const TWIG_TRAP_KINDS[] = {
    "Trap.Bounds",    /* 1 */
    "Trap.Null",      /* 2 */
    "Trap.DivZero",   /* 3 */
    "Trap.ConvRange", /* 4 */
    "Trap.Overflow",  /* 5 */
    "Trap.User",      /* 6 */
};

/* __twig_trap — LANG83 `trap kind` (and the defined traps of `i64_div_floor`
 * / `i64_mod_floor`): terminate the program observably.
 *
 * Order matters, and is the LANG83 contract:
 *   1. flush stdout — every byte the program wrote before the trap reaches
 *      its consumer (`putchar` / `write_i64` do not flush per call, and
 *      `abort` does not flush stdio);
 *   2. name the kind on stderr (`trap: Trap.DivZero`), so a harness can tell
 *      which trap fired;
 *   3. `abort()` — a non-zero, signal-terminated exit, like the backends'
 *      `ud2`/`udf` hardware traps.
 * The backends emit `mov edi, id; call __twig_trap; ud2` (x86-64) or
 * `mov w0, id; bl __twig_trap; udf` (AArch64); this never returns.
 */
#if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn))
#elif defined(_MSC_VER)
__declspec(noreturn)
#endif
void __twig_trap(int32_t kind_id) {
    const size_t n = sizeof(TWIG_TRAP_KINDS) / sizeof(TWIG_TRAP_KINDS[0]);
    fflush(stdout);
    if (kind_id >= 1 && (size_t)kind_id <= n) {
        fprintf(stderr, "trap: %s\n", TWIG_TRAP_KINDS[kind_id - 1]);
    } else {
        fprintf(stderr, "trap: unknown kind id %d\n", (int)kind_id);
    }
    fflush(stderr);
    abort();
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
    twig_runtime_ensure_stdout_binary();
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
    twig_runtime_ensure_stdout_binary();
    if (s != NULL && len > 0) {
        fwrite(s, 1, (size_t)len, stdout);
    }
}

/* Peek through the same stdio stream consumed by input_i64. ISO C guarantees
 * one byte of pushback, so repeated peeks leave the next field intact. EOF is
 * sticky until more input is supplied; no field value is synthesized here. */
int64_t __twig_input_more(void) {
    int c = fgetc(stdin);
    if (c == EOF) return 0;
    if (ungetc(c, stdin) == EOF) abort();
    return 1;
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

/* __twig_alloc_bytes — allocate `n` zero-initialised bytes for a runtime
 * string block, MANAGED BY THE COLLECTOR (LANG76; gc-core convergence).
 *
 * Returns a 64-bit pointer to `n` zeroed, 16-aligned bytes.  Unlike the
 * original `calloc` version (which leaked every string until process exit),
 * the block is now a gc-core allocation: it is TRACED and RECLAIMED when it
 * becomes unreachable, so string-heavy programs no longer grow without bound.
 *
 * A string is a `[int64 len][bytes]` opaque blob — it holds NO interior GC
 * references — so it is allocated under a registered no-ref HeapKind (an empty
 * field map).  The collector therefore scans none of its bytes: a byte pattern
 * that merely looks like a pointer can never falsely pin another object
 * (precise tracing), and the block is a movable leaf.  Movability is safe under
 * the compacting collector by pin-when-unsure: a string reached only via a raw
 * `int64` handle in a non-reference slot is reached conservatively and is
 * PINNED (never moved, so the raw handle never dangles); a string reached via a
 * tagged heap-reference slot is moved and its slot fixed up.  Either way no
 * handle is left dangling.
 *
 * The kind is registered lazily on first use.  The runtime is single-threaded,
 * so the `static` init needs no synchronisation.  Negative or zero `n` returns
 * NULL (V1 "no null checking" contract, unchanged); OOM returns NULL.
 *
 * The returned pointer is treated as `int64_t` by the frontend (the AAPCS64 /
 * System V / MS x64 ABIs all return pointers in `x0` / `rax` regardless of
 * pointer-vs-integer typing, so no coercion is needed at the call site).
 */
int64_t __twig_alloc_bytes(int64_t n) {
    if (n <= 0) return 0;
    static int64_t blob_kind = 0; /* 0 = not yet registered */
    if (blob_kind == 0) {
        /* An empty field map: the string's bytes are never traced as pointers. */
        blob_kind = __gc_register_kind(NULL, 0); /* 1-based id */
    }
    return __gc_alloc_kind(n, (uint16_t)blob_kind);
}

/* __twig_alloc_ref_array_bytes -- like `__twig_alloc_bytes`, but for a
 * LANG-FULL E5 `alloc_array` block whose ELEMENTS themselves carry GC
 * references (a `str`/`any`/`symbol`/`ref<T>` element, each stored as an
 * 8-byte handle at `[8, 8 + count*8)`, past the 8-byte length header).
 *
 * Confirmed correctness gap this closes (found during LLVM's `alloc_bytes`/
 * `alloc_array` calloc-leak fix): `alloc_array` used to always register its
 * block under `__twig_alloc_bytes`'s no-ref HeapKind, correct only for
 * genuinely scalar elements (`i64`/`f64`). For a reference-carrying element,
 * the collector's precise tracer never scanned the array's payload for the
 * handles stored inside it, so a string/symbol reachable ONLY via an array
 * element (no separate long-lived reference elsewhere) could be collected
 * while the array still held a now-dangling handle.
 *
 * Fix: register under `__gc_register_ref_array_kind(NULL, 0, 8)` instead --
 * `tail_from = 8` skips this block's own length header, so every aligned
 * 8-byte word from offset 8 onward (i.e. every element slot) is traced as a
 * reference.
 *
 * This is used CONDITIONALLY, only when the frontend's ORIGINAL (pre-erasure)
 * IIR element type genuinely carries a reference -- currently wired on the
 * LLVM backend only (`iir-to-llvm::lower_alloc_array` / `elem_is_gc_reference`),
 * since LLVM still has access to that original type at the `alloc_array` call
 * site. A first attempt applied this allocator UNCONDITIONALLY, on the
 * (initially plausible) theory that `mark_word`'s `find_header` validation
 * makes over-tracing "always sound, just conservative" -- true for the
 * non-moving mark/sweep collector, but a security review of that design
 * caught that it is NOT sound against `gc-core::FlatHeap`'s COMPACTING
 * collector (`collect_compacting`, exposed to Twig source as the
 * `gc_collect_compacting` builtin on both native backends): during
 * compaction planning, every traced word -- including a genuinely scalar
 * array element that coincidentally matches another live, movable object's
 * exact base address -- is treated as a real edge (`classify_mobility`/
 * `precise_children`), and after that object is relocated, `fixup_ref_fields`
 * finds the scalar word's bit pattern matches the moved object's OLD address
 * and OVERWRITES it in place with the new one. That is silent, genuine data
 * corruption of the scalar array's contents, not harmless over-retention --
 * so unconditional application across every backend was reverted.
 *
 * The native backends (aarch64/x86_64) do NOT get this precise fix in this
 * round: the AOT specialiser collapses `array<T>` to `any` before native
 * codegen sees `alloc_array`, so the element type is genuinely unavailable
 * there, and they keep calling the plain `__twig_alloc_bytes` unconditionally
 * -- safe under compaction, but leaving the narrower, pre-existing
 * dangling-reference gap on those two backends as a tracked follow-up
 * (needs element-type plumbing through the specialiser). See
 * `AOT00-T7-array-reference-tracing.md` for the full writeup.
 */
int64_t __twig_alloc_ref_array_bytes(int64_t n) {
    if (n <= 0) return 0;
    static int64_t ref_array_kind = 0; /* 0 = not yet registered */
    if (ref_array_kind == 0) {
        ref_array_kind = __gc_register_ref_array_kind(NULL, 0, 8);
    }
    return __gc_alloc_kind(n, (uint16_t)ref_array_kind);
}

/* __twig_input_str — read one line from stdin as an E4-dyn runtime string
 * (LANG-FULL E4-dyn, BASIC `INPUT A$`).
 *
 * The string counterpart of `__twig_input_i64`: instead of parsing the line as
 * a number, it keeps the whole line (minus the trailing newline) AS the string
 * value.  It returns a handle to a fresh length-prefixed heap block — the SAME
 * `[int64 len][bytes]` repr `__twig_print_string` / `__twig_str_eq` already
 * consume — so `PRINT A$` reads the length from the header at run time.
 *
 * A single line is bounded by the fixed 4096-byte stack buffer; a longer line is
 * truncated and its tail stays in the stdio buffer (consumed on the next read),
 * mirroring `__twig_input_i64`'s 64-byte bound — the V1 permissive contract.
 * On EOF the buffer is emptied, so this returns a handle to a zero-length
 * ("") block (never a NULL handle), keeping a subsequent `print_str` safe.
 */
int64_t __twig_input_str(void) {
    char buf[4096];
    if (fgets(buf, sizeof(buf), stdin) == NULL) {
        buf[0] = '\0';
    }
    size_t len = strlen(buf);
    /* fgets keeps the terminating '\n'; a runtime string is the line content,
     * so strip a single trailing newline (but not embedded ones). */
    if (len > 0 && buf[len - 1] == '\n') {
        len--;
    }
    /* Block = [int64 len][len bytes].  alloc_bytes(8+len) is always > 0 here
     * (8-byte header even for the empty string), so the handle is non-NULL. */
    int64_t handle = __twig_alloc_bytes((int64_t)(8 + len));
    if (handle == 0) return 0;
    *(int64_t *)handle = (int64_t)len;
    if (len > 0) {
        memcpy((char *)handle + 8, buf, len);
    }
    return handle;
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
