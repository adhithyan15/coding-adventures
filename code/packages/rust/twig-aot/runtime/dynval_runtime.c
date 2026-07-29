/* lispy_runtime.c — the shared lisp value model for AOT-compiled programs.
 *
 * This is the **native counterpart** of the Rust `lispy-runtime` crate that
 * the VM and JIT use.  Both implement the *same* documented ABI — a
 * `LispyValue` is a single 64-bit word with a 3-bit tag in the low bits —
 * exactly as `__twig_print_i64` here is the native counterpart of the VM's
 * print.  An AOT executable is self-contained: it never shares memory with
 * the Rust runtime, so it carries its own (small, leaking) heap and intern
 * table.  Interop is *by contract* — same tag layout, same encodings — and
 * that contract is pinned by a golden test (`src/lispy_runtime_golden.rs`)
 * that asserts every constant and encoding here matches `lispy-runtime`'s
 * `pub` constants and constructors (the golden test lives at
 * `src/lispy_runtime_golden.rs`).  If the two ever drift, `cargo test` goes
 * red.  See `code/specs/LANG77-lisp-native-runtime.md`.
 *
 * Why C and not "link the Rust crate"?  `lispy-runtime` is full-`std`
 * (Mutex / HashMap / OnceLock / Box::leak); linking it as a staticlib would
 * drag `std` + panic machinery into every AOT binary (which today links
 * only libc), and the repo has no mechanism to build a Rust staticlib from
 * a dependent crate's build.rs.  This C translation unit adds zero new
 * runtime dependency and reuses the existing `cc`-built archive verbatim.
 *
 * ───────────────────────────────────────────────────────────────────────
 *  Tag layout (low 3 bits), from lispy-runtime/src/value.rs
 * ───────────────────────────────────────────────────────────────────────
 *
 *   ┌──────────────────────── payload (high 61 bits) ────────────┬─tag─┐
 *   63                                                          3 2   0
 *
 *   | tag   | kind             | encoding                  | decode          |
 *   |-------|------------------|---------------------------|-----------------|
 *   | 0b000 | integer          | (n << 3)                  | arithmetic >> 3 |
 *   | 0b001 | nil singleton    | whole word == 1           | x == 1          |
 *   | 0b010 | interned symbol  | (id << 32) | 0b010        | (x >> 32)       |
 *   | 0b011 | #f singleton     | whole word == 3           | x == 3          |
 *   | 0b101 | #t singleton     | whole word == 5           | x == 5          |
 *   | 0b111 | heap pointer     | ptr | 0b111 (ptr 8-aligned)| x & ~0b111     |
 *
 * Truthiness (Scheme/lispy): a value is FALSE iff it is #f or nil; every
 * other value (including the integer 0) is TRUE.
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* TWIG-GC (twig_gc.c) — used by __dyn_cons to allocate cons cells on
 * the managed heap instead of leaking via calloc. */
extern int64_t __twig_gc_alloc(int64_t n);

/* Precise-GC kind registration (gc-core-capi). A cons cell is TWO reference
 * fields — car at byte 0, cdr at byte 8 — so it is allocated under a registered
 * HeapKind whose field map is {0, 8}. This makes the cell MOVABLE by the
 * compacting collector (a kind-0 conservative allocation would be pinned) and
 * lets the collector trace + relocate its children precisely. `__gc_register_kind`
 * returns a 1-based kind id; `__gc_alloc_kind(n, kind)` allocates `n` zeroed,
 * 16-aligned bytes tagged with that kind. Both are exported by gc-core-capi. */
extern int64_t __gc_register_kind(const int64_t *field_offsets, int64_t count);
extern int64_t __gc_alloc_kind(int64_t n, uint16_t kind);

/* ── Tag constants ──────────────────────────────────────────────────────
 *
 * These mirror lispy-runtime/src/value.rs.  The golden test reads them back
 * through the `__dyn_tag_*` accessors below and asserts each equals
 * the corresponding `pub const` in the Rust crate.
 */
#define LISPY_TAG_BITS   0x7ULL  /* mask covering the low 3 bits           */
#define LISPY_TAG_INT    0x0ULL  /* 0b000                                  */
#define LISPY_TAG_NIL    0x1ULL  /* 0b001  — whole word                    */
#define LISPY_TAG_SYMBOL 0x2ULL  /* 0b010                                  */
#define LISPY_TAG_FALSE  0x3ULL  /* 0b011  — whole word                    */
#define LISPY_TAG_TRUE   0x5ULL  /* 0b101  — whole word                    */
#define LISPY_TAG_HEAP   0x7ULL  /* 0b111                                  */

/* The three immediate singletons as whole-word constants. */
#define LISPY_NIL    LISPY_TAG_NIL    /* 1 */
#define LISPY_FALSE  LISPY_TAG_FALSE  /* 3 */
#define LISPY_TRUE   LISPY_TAG_TRUE   /* 5 */

/* ── Integer box / unbox ────────────────────────────────────────────────
 *
 * Boxing is a left shift by 3 (tag 0b000 needs no OR).  Unboxing is an
 * *arithmetic* right shift so the sign extends — we cast through int64_t to
 * request the arithmetic variant (signed >> is arithmetic on every
 * platform we target).  Range is ±2^60, matching the Rust INT_MIN/INT_MAX.
 *
 *   box_int(7)   = 0b...0111_000 = 56
 *   unbox_int(56) = 7
 *   unbox_int(box_int(-1)) = -1   (sign-extended)
 */
uint64_t __dyn_box_int(int64_t n) {
    return ((uint64_t)n) << 3;
}

int64_t __dyn_unbox_int(uint64_t v) {
    return ((int64_t)v) >> 3;
}

/* The nil singleton.  Provided as a function for the lowering/boundary even
 * though it is a compile-time constant — keeps the backend uniform. */
uint64_t __dyn_nil(void) {
    return LISPY_NIL;
}

/* ── Cons cells ─────────────────────────────────────────────────────────
 *
 * A pair is two consecutive 64-bit words on the heap:
 *
 *     ┌────────────┬────────────┐
 *     │  [0] car   │  [8] cdr   │
 *     └────────────┴────────────┘
 *
 * `__twig_gc_alloc(16)` returns memory aligned to at least 16 bytes (the GC
 * header is 32 bytes, so the payload is always 16-byte–aligned), so the low
 * 3 bits of the pointer are zero — which the OR-with-tag (0b111) scheme
 * requires.  The allocation is managed by TWIG-GC (twig_gc.c) and will be
 * collected when the cell becomes unreachable.  Out-of-memory returns nil
 * rather than crashing inside the runtime.
 */
uint64_t __dyn_cons(uint64_t car, uint64_t cdr) {
    /* Register the cons-cell kind once (its two fields are references at bytes
     * 0 and 8), then allocate the cell under that kind so the compacting collector
     * may RELOCATE it (a kind-0 conservative cell would pin). The runtime is
     * single-threaded, so the lazy `static` init needs no synchronisation, and
     * registering immediately before the first allocation of that kind keeps the
     * ordering trivially correct. Size (16 bytes = 2 words) is unchanged; a
     * kind-tagged block has the same 16-aligned payload as `__twig_gc_alloc`. */
    static int64_t cons_kind = 0; /* 0 = not yet registered */
    if (cons_kind == 0) {
        int64_t offsets[2] = {0, 8};
        cons_kind = __gc_register_kind(offsets, 2); /* 1-based id */
    }
    int64_t ptr = __gc_alloc_kind(2 * (int64_t)sizeof(uint64_t), (uint16_t)cons_kind);
    if (ptr == 0) {
        return LISPY_NIL;
    }
    uint64_t *cell = (uint64_t *)(intptr_t)ptr;
    cell[0] = car;
    cell[1] = cdr;
    return ((uint64_t)(uintptr_t)cell) | LISPY_TAG_HEAP;
}

/* Recover the cell pointer by clearing the tag bits, then read the field.
 * Reading car/cdr of a non-pair is undefined (V1 has no type checking — the
 * frontend is responsible for only calling car/cdr on pairs), matching the
 * permissive contract of the rest of the runtime. */
uint64_t __dyn_car(uint64_t pair) {
    uint64_t *cell = (uint64_t *)(uintptr_t)(pair & ~LISPY_TAG_BITS);
    return cell[0];
}

uint64_t __dyn_cdr(uint64_t pair) {
    uint64_t *cell = (uint64_t *)(uintptr_t)(pair & ~LISPY_TAG_BITS);
    return cell[1];
}

/* `pair?` — true iff the value is heap-tagged.  Returns a *tagged* boolean
 * (#t/#f), not a C 0/1, so the result is itself a LispyValue. */
uint64_t __dyn_pair_p(uint64_t v) {
    return ((v & LISPY_TAG_BITS) == LISPY_TAG_HEAP) ? LISPY_TRUE : LISPY_FALSE;
}

/* `null?` — true iff the value is the nil sentinel (the empty list).
 *
 * Returns a *tagged* boolean (#t/#f), exactly like `__dyn_pair_p`, so `null?` is
 * a first-class lisp value: `(null? (list))` as a whole program exit-codes as #t
 * (→ 1) through the runtime tag switch, not misread as the nil word.
 *
 * Inside the cons-walk helpers (`length`, `append`, …) the result feeds a
 * `jmp_if_false`; that is safe because the compiler tracks a `dyn_null_p` result
 * as a tagged `LispyValue` (it is in `dyn_repr`'s LISP_BUILTINS) and inserts a
 * `dyn_truthy` before the branch — so a tagged #t/#f is normalised to a raw 0/1
 * there. (A raw #t/#f fed straight to `jmp_if_false` would be wrong, since both
 * LISPY_TRUE=5 and LISPY_FALSE=3 are non-zero.)
 *
 * It compares against the whole-word LISPY_NIL (1), not 0: nil is a tagged
 * immediate here, so the native `is_null` opcode's zero-test would never match.
 */
uint64_t __dyn_null_p(uint64_t v) {
    return (v == LISPY_NIL) ? LISPY_TRUE : LISPY_FALSE;
}

/* ── Booleans ───────────────────────────────────────────────────────────
 *
 * `not` follows lispy truthiness: a value is false iff it is #f or nil.
 * `not(x)` returns #t when x is false-y, else #f.
 *
 *   not(#f)  = #t      not(nil) = #t
 *   not(#t)  = #f      not(0)   = #f   (0 is a truthy integer)
 */
uint64_t __dyn_not(uint64_t v) {
    int is_falsey = (v == LISPY_FALSE) || (v == LISPY_NIL);
    return is_falsey ? LISPY_TRUE : LISPY_FALSE;
}

/* __dyn_truthy — normalise a tagged value to a RAW machine boolean
 * (0 or 1) for a conditional branch.
 *
 * Unlike `not`, this does NOT return a tagged `LispyValue` — it returns a
 * plain `int64_t` 0/1 so the backend's `jmp_if_false` (which tests a raw
 * machine word against zero) branches correctly on a `LispyValue` condition.
 * It is the bridge `COND` needs: a McCarthy predicate evaluates to a tagged
 * value (#t/#f, a symbol, a pair, …), and lisp truthiness is "false iff #f
 * or nil, true otherwise" — including the integer 0 and the empty… no, nil
 * is false, but a *boxed* integer 0 (whole word 0b000 = 0) is truthy.
 *
 *   truthy(#f)  = 0      truthy(nil)      = 0
 *   truthy(#t)  = 1      truthy(box_int 0)= 1   (0 is a truthy atom)
 *   truthy('A)  = 1      truthy(pair)     = 1
 */
int64_t __dyn_truthy(uint64_t v) {
    return (v == LISPY_FALSE || v == LISPY_NIL) ? 0 : 1;
}

/* __dyn_to_exit_code — coerce ANY tagged LispyValue to a raw exit code,
 * dispatching on its RUNTIME tag.  This is the program-exit boundary for a
 * value whose tag the compiler cannot know statically — a lambda result (F7),
 * which the frontend types as the polymorphic `any`.  The static coercions each
 * cover one tag; this switch covers them all at once:
 *
 *   tag (v & 0b111)  value           exit code
 *   ───────────────  ──────────────  ──────────────────────────────────────
 *   0b000  INT       boxed integer   arithmetic  v >> 3   (sign-extended)
 *   0b101  TRUE      #t              1
 *   0b011  FALSE     #f              0
 *   0b001  NIL       ()              0   (nil is falsy, like #f)
 *   0b010  SYMBOL    interned atom   the tagged word verbatim (stable id+tag)
 *   0b111  HEAP      cons pair       the tagged word verbatim (stable pointer)
 *
 * Agreement with the static helpers (so this is a safe superset):
 *   to_exit_code(box_int n) == unbox_int(box_int n) == n
 *   to_exit_code(#t/#f/nil) == truthy(#t/#f/nil)    == 1/0/0
 *   to_exit_code(symbol)    == (symbol returned verbatim)
 */
int64_t __dyn_to_exit_code(uint64_t v) {
    switch (v & LISPY_TAG_BITS) {
        case LISPY_TAG_INT:   return ((int64_t)v) >> 3;  /* integer atom    */
        case LISPY_TAG_TRUE:  return 1;                  /* #t              */
        case LISPY_TAG_FALSE: return 0;                  /* #f              */
        case LISPY_TAG_NIL:   return 0;                  /* () is falsy     */
        default:              return (int64_t)v;         /* symbol / pair   */
    }
}

/* `equal?` — structural deep equality, returning a tagged boolean.
 *
 *   - Two atoms (neither is a pair) are equal iff their bits are equal.
 *     This makes integer/symbol/nil/bool equality fall out for free,
 *     because each has a unique bit pattern (interning guarantees one id
 *     per symbol name).
 *   - Two pairs are equal iff their cars are equal AND their cdrs are
 *     equal (recursive).
 *   - A pair and an atom are never equal.
 *
 * Recursion depth is bounded by the list/tree depth the program built;
 * pathological cyclic structures cannot occur because V1 cons is
 * write-once at construction with no mutation primitive.
 */
uint64_t __dyn_equal(uint64_t a, uint64_t b) {
    int a_pair = (a & LISPY_TAG_BITS) == LISPY_TAG_HEAP;
    int b_pair = (b & LISPY_TAG_BITS) == LISPY_TAG_HEAP;

    if (!a_pair && !b_pair) {
        return (a == b) ? LISPY_TRUE : LISPY_FALSE;
    }
    if (a_pair != b_pair) {
        return LISPY_FALSE;
    }
    /* both pairs — recurse on car and cdr */
    if (__dyn_equal(__dyn_car(a), __dyn_car(b)) != LISPY_TRUE) {
        return LISPY_FALSE;
    }
    return __dyn_equal(__dyn_cdr(a), __dyn_cdr(b));
}

/* ── Symbol interning ───────────────────────────────────────────────────
 *
 * Symbols carry a 32-bit id in the high bits.  Interning guarantees that
 * the same name always maps to the same id, so `EQ`/`equal?` on symbols is
 * just bitwise equality.  The table is a fixed-capacity open-addressing
 * hash (FNV-1a over the name bytes, linear probing).  Ids are assigned in
 * first-seen order (0, 1, 2, …) — they need not match the Rust runtime's
 * ids because the AOT binary is self-contained; only *consistency within a
 * single program* matters, which interning provides.
 *
 * Names are copied (malloc + memcpy) and leaked, matching V1's no-free
 * policy.  AOT programs are single-threaded in V1, so no locking is needed
 * (the Rust runtime uses a Mutex because the VM can be multi-threaded).
 *
 * Capacity is 1<<16 slots — far beyond any realistic symbol count for an
 * AOT'd script.  Overflow aborts rather than silently colliding.
 */
#define LISPY_INTERN_CAP (1u << 16)

typedef struct {
    const char *name; /* copied, NUL not required */
    int32_t     len;
    int32_t     id;       /* assigned id; valid only when `used` */
    int32_t     used;     /* 0 = empty slot, 1 = occupied        */
} lispy_intern_slot;

static lispy_intern_slot lispy_intern_table[LISPY_INTERN_CAP];
static int32_t           lispy_intern_next_id = 0;

/* FNV-1a hash of `len` bytes — small, fast, good enough for symbol names. */
static uint64_t lispy_fnv1a(const char *s, int64_t len) {
    uint64_t h = 1469598103934665603ULL; /* FNV offset basis */
    for (int64_t i = 0; i < len; i++) {
        h ^= (uint64_t)(unsigned char)s[i];
        h *= 1099511628211ULL;            /* FNV prime */
    }
    return h;
}

uint64_t __dyn_make_symbol(const char *name, int64_t len) {
    if (len < 0) {
        len = 0;
    }
    uint64_t mask = LISPY_INTERN_CAP - 1u;
    uint64_t idx  = lispy_fnv1a(name, len) & mask;

    for (uint64_t probe = 0; probe < LISPY_INTERN_CAP; probe++) {
        lispy_intern_slot *slot = &lispy_intern_table[idx];
        if (!slot->used) {
            /* First time we have seen this name — intern it. */
            char *copy = (char *)malloc((size_t)len);
            if (copy == NULL && len > 0) {
                /* Allocation failure: refuse rather than mark the slot used
                 * with a NULL name — a later colliding lookup would
                 * `memcmp(NULL, …)` and crash.  Unreachable in practice
                 * (a symbol name is a few bytes). */
                abort();
            }
            if (len > 0) {
                memcpy(copy, name, (size_t)len);
            }
            slot->name = copy;
            slot->len  = (int32_t)len;
            slot->id   = lispy_intern_next_id++;
            slot->used = 1;
            return (((uint64_t)(uint32_t)slot->id) << 32) | LISPY_TAG_SYMBOL;
        }
        if (slot->len == (int32_t)len &&
            (len == 0 || memcmp(slot->name, name, (size_t)len) == 0)) {
            /* Already interned — return the existing id. */
            return (((uint64_t)(uint32_t)slot->id) << 32) | LISPY_TAG_SYMBOL;
        }
        idx = (idx + 1u) & mask; /* linear probe */
    }

    /* Table full — refuse rather than silently collide.  Unreachable in
     * practice (64Ki distinct symbols in one AOT'd script). */
    abort();
}

/* ── Tag accessors (golden-test only) ───────────────────────────────────
 *
 * These exist solely so the Rust golden test can read what the C side
 * believes each tag constant is and assert it against lispy-runtime's
 * `pub const`s.  They are never called by compiled programs.
 */
uint64_t __dyn_tag_int(void)    { return LISPY_TAG_INT;    }
uint64_t __dyn_tag_nil(void)    { return LISPY_TAG_NIL;    }
uint64_t __dyn_tag_symbol(void) { return LISPY_TAG_SYMBOL; }
uint64_t __dyn_tag_false(void)  { return LISPY_TAG_FALSE;  }
uint64_t __dyn_tag_true(void)   { return LISPY_TAG_TRUE;   }
uint64_t __dyn_tag_heap(void)   { return LISPY_TAG_HEAP;   }
uint64_t __dyn_tag_mask(void)   { return LISPY_TAG_BITS;   }
