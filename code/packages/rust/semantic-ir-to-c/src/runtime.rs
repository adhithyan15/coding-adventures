//! Inlined C runtime — pasted verbatim into every emitted artifact.
//!
//! The C backend produces **self-contained** output: every generated `.c`
//! file embeds the runtime helpers it needs, so `cc <file>.c -o <file>`
//! builds a working program with no external dependency beyond the C standard
//! library.  This mirrors the Go and Rust backends (which inline their runtime
//! the same way) rather than the Python/TypeScript `sir-runtime-*` import
//! model.
//!
//! ## Portability
//!
//! The runtime is **ISO C99** with **no compiler-specific extensions** — it
//! compiles on MSVC (`/std:c11`), GCC, and Clang.  Every heap box is
//! `malloc`'d and never freed (arena / leak-on-exit): an emitted program is a
//! batch program that runs and exits, so the OS reclaims everything.
//!
//! Runtime helper *functions* have external linkage (not `static`): the whole
//! runtime is inlined into every artifact but a small program uses only part
//! of it, and external linkage keeps `-Wunused-function` quiet on GCC/Clang
//! without any compiler-specific `unused` attribute.  The file-scope *data*
//! (intern table, global store) stays `static` for namespace hygiene.
//!
//! ## Display convention
//!
//! The single placeholder `__SIR_DISPLAY_RUBY__` is substituted by the emitter
//! with the integer literal `1` (Ruby-sourced module → booleans render as
//! `true`/`false`, `nil` as the empty string) or `0` (the default Lisp
//! rendering — `#t`/`#f`, `nil`).  The substitution is a **boolean-selected
//! literal**, never source-derived text, so it can never inject into the
//! emitted C.

/// The C runtime, as a single string constant.  `emit::emit_module` prepends
/// it (after the `#include`s) to every artifact, first replacing the
/// `__SIR_DISPLAY_RUBY__` placeholder.
pub const RUNTIME: &str = r####"/* ============================================================
 *  Inlined SIR runtime (semantic-ir-to-c).  ISO C99, no
 *  compiler-specific extensions.  Arena / leak-on-exit memory.
 * ============================================================ */

/* Display convention: 1 => Ruby (true/false, empty nil), 0 => Lisp (#t/#f). */
#define SIR_DISPLAY_RUBY __SIR_DISPLAY_RUBY__

typedef enum {
    SIR_NIL, SIR_BOOL, SIR_INT, SIR_FLOAT,
    SIR_STR, SIR_SYM, SIR_PAIR, SIR_CLOSURE, SIR_SEQ, SIR_MAP,
    /* An INTERNAL "argument was omitted" sentinel for SIR19 default parameters.
     * A `DirectCall` that leaves a trailing defaulted argument off pads the call
     * with `_sir_missing()`; the callee's prologue replaces each such parameter
     * with its default expression BEFORE the body runs, so a `SIR_MISSING` value
     * is never observed by user code (never printed, compared, or stored). */
    SIR_MISSING,
    /* A SIR17 exception value (`raise`d and `rescue`d) — a class name plus an
     * optional message. */
    SIR_ERROR,
    /* A SIR OOP instance (`Foo.new`) — a heap object carrying its class name.
     * Unlike the Go/Rust backends (which hold an integer id into a side-table
     * because their value type is `Copy`), C stores the `SirInstance *` INLINE in
     * the union: the pointer IS the handle, so pointer-identity is object
     * identity.  A dedicated tag means no built-in-type helper mis-handles it. */
    SIR_INSTANCE
} SirTag;

typedef struct SirValue SirValue;
typedef struct SirPair SirPair;
typedef struct SirClosure SirClosure;
typedef struct SirSeq SirSeq;
typedef struct SirMap SirMap;
typedef struct SirError SirError;
typedef struct SirInstance SirInstance;

struct SirValue {
    SirTag tag;
    union {
        int b;            /* SIR_BOOL (0/1) */
        int64_t i;        /* SIR_INT */
        double f;         /* SIR_FLOAT */
        const char *s;    /* SIR_STR / SIR_SYM (interned) */
        SirPair *pair;    /* SIR_PAIR */
        SirClosure *clo;  /* SIR_CLOSURE */
        SirSeq *seq;      /* SIR_SEQ */
        SirMap *map;      /* SIR_MAP */
        SirError *err;    /* SIR_ERROR */
        SirInstance *inst;/* SIR_INSTANCE */
    } as;
};

struct SirPair { SirValue car; SirValue cdr; };

/* A SIR17 exception (`raise`/`rescue`).  `sir_class` is the interned exception
 * class name (`"RuntimeError"`, `"StandardError"`, …), matched against a
 * `rescue` clause via the baked-in ancestry table.  `msg` is the message (a
 * `SIR_STR`, or nil for a bare `raise Class` — then the class name is shown). */
struct SirError { const char *sir_class; SirValue msg; };

/* A SIR OOP instance.  `sir_class` is the interned class name (`"Foo"`) — how
 * method dispatch keys the method table.  `ivars` is a lazily-allocated
 * `@name -> value` map (NULL until the first `@x = …`; an unset `@x` reads nil). */
struct SirInstance { const char *sir_class; SirMap *ivars; };

/* A SIR16 sequence (`[1, 2, 3]`) — a heap-boxed dynamic array. `items` points
 * at `len` `SirValue`s (arena-allocated, never freed like every other heap
 * value). Boxed so the value is a shared, mutable handle: a `SeqSet` through
 * one binding is visible through every alias (matching the Go/Rust `*Seq`). */
struct SirSeq { SirValue *items; int64_t len; };

/* A SIR16 map (`{k => v, …}`) — a heap-boxed, insertion-ordered ASSOC-ARRAY:
 * `entries[0 .. len)` are the key/value pairs in first-insertion order, backed
 * by a `cap`-slot array that DOUBLES on overflow (a `MapSet` of a new key
 * appends — unlike the fixed-length `SeqSet`). It is a linear-scan assoc-array,
 * NOT a hash table, exactly like the Go/Rust reference (`[]MapEntry` /
 * `Vec<(Value, Value)>`): lookups are O(n) but structural keys (`[1, 2]`) and
 * insertion-order iteration/printing come for free with no `Hash`/`Eq` bound on
 * the value type. Boxed so it is a shared, mutable handle: a `MapSet` through
 * one binding is visible through every alias (matching the Go/Rust `*Map`). */
struct SirMapEntry { SirValue key; SirValue val; };
struct SirMap { struct SirMapEntry *entries; int64_t len; int64_t cap; };

/* A closure's function takes its captured environment and the call args. */
typedef SirValue (*SirFn)(SirValue *caps, SirValue *args, int argc);
struct SirClosure { SirFn fn; int ncap; SirValue *caps; };

/* ---- small allocation + string helpers ---------------------- */

void *_sir_alloc(size_t n) {
    void *p = malloc(n);
    if (!p) { fprintf(stderr, "sir: out of memory\n"); exit(1); }
    return p;
}

char *_sir_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)_sir_alloc(n);
    memcpy(p, s, n);
    return p;
}

char *_sir_cat(const char *a, const char *b) {
    size_t la = strlen(a), lb = strlen(b);
    char *p = (char *)_sir_alloc(la + lb + 1);
    memcpy(p, a, la);
    memcpy(p + la, b, lb + 1);
    return p;
}

/* ---- symbol interning --------------------------------------- */

#define SIR_INTERN_MAX 8192
static const char *_sir_intern_tab[SIR_INTERN_MAX];
static int _sir_intern_n = 0;

const char *_sir_intern(const char *s) {
    int i;
    for (i = 0; i < _sir_intern_n; i++) {
        if (strcmp(_sir_intern_tab[i], s) == 0) return _sir_intern_tab[i];
    }
    {
        const char *d = _sir_dup(s);
        if (_sir_intern_n < SIR_INTERN_MAX) _sir_intern_tab[_sir_intern_n++] = d;
        return d;
    }
}

/* ---- value constructors ------------------------------------- */

SirValue _sir_nil(void)        { SirValue v; v.tag = SIR_NIL;  v.as.i = 0;   return v; }
SirValue _sir_bool(int b)      { SirValue v; v.tag = SIR_BOOL; v.as.b = b ? 1 : 0; return v; }
SirValue _sir_int(int64_t i)   { SirValue v; v.tag = SIR_INT;  v.as.i = i;   return v; }
SirValue _sir_float(double f)  { SirValue v; v.tag = SIR_FLOAT; v.as.f = f;  return v; }
SirValue _sir_str(const char *s) { SirValue v; v.tag = SIR_STR; v.as.s = s;  return v; }
SirValue _sir_sym(const char *s) { SirValue v; v.tag = SIR_SYM; v.as.s = _sir_intern(s); return v; }

/* ---- OOP: instances & constants ----------------------------- */

/* Construct a fresh instance of class `cls` (`Foo.new`).  The class name is
 * interned so later method dispatch keys the method table by pointer/`strcmp`.
 * Arena-allocated, never freed (like every heap box). */
SirValue _sir_new_instance(const char *cls) {
    SirInstance *o = (SirInstance *)_sir_alloc(sizeof(SirInstance));
    o->sir_class = _sir_intern(cls);
    o->ivars = NULL;  /* lazily allocated on the first `@x = …` */
    { SirValue v; v.tag = SIR_INSTANCE; v.as.inst = o; return v; }
}

/* A SIR constant (`PI = 3`, referenced as `PI`).  Ruby constants are named,
 * top-level, and set-once-at-runtime; C has no such construct, so a tiny
 * name -> value table backs them.  The name is an interned string emitted from a
 * quoted C string literal (no injection).  A read of an undefined constant is a
 * `NameError` (matching Ruby), routed through the existing exception path. */
#define SIR_CONST_MAX 4096
static struct { const char *name; SirValue val; } _sir_const_tab[SIR_CONST_MAX];
static int _sir_const_n = 0;

SirValue _sir_const_set(const char *name, SirValue v) {
    const char *n = _sir_intern(name);
    int i;
    for (i = 0; i < _sir_const_n; i++) {
        if (_sir_const_tab[i].name == n) { _sir_const_tab[i].val = v; return v; }
    }
    if (_sir_const_n < SIR_CONST_MAX) {
        _sir_const_tab[_sir_const_n].name = n;
        _sir_const_tab[_sir_const_n].val = v;
        _sir_const_n++;
    }
    return v;
}

/* The SIR19 "argument omitted" sentinel (see `SIR_MISSING`).  `_sir_missing()`
 * is passed at a call site for each trailing defaulted parameter left off; the
 * callee's prologue tests `_sir_is_missing` and substitutes the default. */
SirValue _sir_missing(void)      { SirValue v; v.tag = SIR_MISSING; v.as.i = 0; return v; }
int _sir_is_missing(SirValue v)  { return v.tag == SIR_MISSING; }

/* ---- truthiness --------------------------------------------- */

/* SIR truthiness is the Lisp/Ruby convention: only `false` and `nil` are
 * falsy.  0, "", the empty list, etc. are all truthy — unlike C's notion, so
 * every condition routes through here rather than a bare `if`. */
int _sir_truthy(SirValue v) {
    if (v.tag == SIR_NIL) return 0;
    if (v.tag == SIR_BOOL) return v.as.b;
    return 1;
}

/* ---- numeric helpers ---------------------------------------- */

int _sir_is_num(SirValue v) { return v.tag == SIR_INT || v.tag == SIR_FLOAT; }

double _sir_as_num(SirValue v) {
    if (v.tag == SIR_FLOAT) return v.as.f;
    if (v.tag == SIR_INT)   return (double)v.as.i;
    return 0.0;
}

/* Truncating integer view of a numeric value — the loop-counter form used by
 * `ForRange` (which counts in int64, matching the Go/Rust backends).  A float
 * bound truncates toward zero; a non-number yields 0. */
int64_t _sir_as_int(SirValue v) {
    if (v.tag == SIR_INT)   return v.as.i;
    if (v.tag == SIR_FLOAT) return (int64_t)v.as.f;
    return 0;
}

const char *_sir_str_of(SirValue v) {
    return (v.tag == SIR_STR || v.tag == SIR_SYM) ? v.as.s : "";
}

/* ---- SIR26 integer conversions ------------------------------ */

/* Reduce an int64 to `bits` (8/16/32/64/128) by two's-complement
 * reinterpretation: mask to the low `bits`, then sign-fold when `is_signed`.
 * Pure int64/uint64 arithmetic (no reliance on native fixed-width casts), so it
 * behaves identically on every compiler.  For bits >= 64 the value is the int64
 * identity — the u64/i64 storage floor, where u64 values above 2^63 are the
 * documented bignum frontier shared with the Go/Rust backends. */
int64_t _sir_mask_to(int64_t v, int bits, int is_signed) {
    uint64_t mask, m;
    if (bits >= 64) return v;
    mask = ((uint64_t)1 << bits) - 1u;
    m = (uint64_t)v & mask;
    if (is_signed && (m & ((uint64_t)1 << (bits - 1)))) {
        return (int64_t)(m - ((uint64_t)1 << bits)); /* sign-extend */
    }
    return (int64_t)m;
}

/* The rendering of Expr::Convert: reduce an integer SirValue to the target
 * width/signedness (a non-integer passes through, defensively). */
SirValue _sir_convert(SirValue v, int bits, int is_signed) {
    if (v.tag != SIR_INT) return v;
    return _sir_int(_sir_mask_to(v.as.i, bits, is_signed));
}

/* Collect varargs into a heap array (freed by the caller).  Returns NULL
 * for n == 0. */
SirValue *_sir_va_collect(int n, va_list ap) {
    SirValue *xs = NULL;
    if (n > 0) {
        int i;
        xs = (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n);
        for (i = 0; i < n; i++) xs[i] = va_arg(ap, SirValue);
    }
    return xs;
}

/* ---- arithmetic (array cores + variadic wrappers) ----------- */

/* `+` is polymorphic: string operands concatenate, else numeric add with
 * int -> float promotion once any operand is a float. */
SirValue _sir_plus_v(SirValue *xs, int n) {
    int i;
    if (n <= 0) return _sir_int(0);
    if (xs[0].tag == SIR_STR) {
        const char *acc = xs[0].as.s;
        for (i = 1; i < n; i++) acc = _sir_cat(acc, _sir_str_of(xs[i]));
        return _sir_str(acc);
    }
    {
        int64_t iacc = (xs[0].tag == SIR_INT) ? xs[0].as.i : 0;
        double  facc = _sir_as_num(xs[0]);
        int promoted = (xs[0].tag == SIR_FLOAT);
        for (i = 1; i < n; i++) {
            if (!promoted && xs[i].tag == SIR_FLOAT) { promoted = 1; facc = (double)iacc; }
            if (promoted) facc += _sir_as_num(xs[i]);
            else          iacc += xs[i].as.i;
        }
        return promoted ? _sir_float(facc) : _sir_int(iacc);
    }
}

SirValue _sir_minus_v(SirValue *xs, int n) {
    int i;
    if (n <= 0) return _sir_int(0);
    if (n == 1) {
        if (xs[0].tag == SIR_FLOAT) return _sir_float(-xs[0].as.f);
        return _sir_int(-xs[0].as.i);
    }
    {
        int64_t iacc = (xs[0].tag == SIR_INT) ? xs[0].as.i : 0;
        double  facc = _sir_as_num(xs[0]);
        int promoted = (xs[0].tag == SIR_FLOAT);
        for (i = 1; i < n; i++) {
            if (!promoted && xs[i].tag == SIR_FLOAT) { promoted = 1; facc = (double)iacc; }
            if (promoted) facc -= _sir_as_num(xs[i]);
            else          iacc -= xs[i].as.i;
        }
        return promoted ? _sir_float(facc) : _sir_int(iacc);
    }
}

SirValue _sir_times_v(SirValue *xs, int n) {
    int i;
    if (n <= 0) return _sir_int(1);
    {
        int64_t iacc = (xs[0].tag == SIR_INT) ? xs[0].as.i : 0;
        double  facc = _sir_as_num(xs[0]);
        int promoted = (xs[0].tag == SIR_FLOAT);
        for (i = 1; i < n; i++) {
            if (!promoted && xs[i].tag == SIR_FLOAT) { promoted = 1; facc = (double)iacc; }
            if (promoted) facc *= _sir_as_num(xs[i]);
            else          iacc *= xs[i].as.i;
        }
        return promoted ? _sir_float(facc) : _sir_int(iacc);
    }
}

/* Integer division floors toward negative infinity (Ruby `/`); a float
 * operand switches to true division.  Division by zero has no v0 exception
 * path, so it fails loudly rather than invoking undefined behaviour. */
int64_t _sir_ifloordiv(int64_t a, int64_t b) {
    int64_t q, r;
    if (b == 0) { fprintf(stderr, "sir: divided by 0\n"); exit(1); }
    q = a / b; r = a % b;
    if (r != 0 && ((r < 0) != (b < 0))) q -= 1;
    return q;
}

/* C truncating division / remainder (the SIR27 `tdiv`/`tmod` builtins).  C's
 * native int64 `/` and `%` already truncate toward zero and give a remainder
 * with the sign of the dividend, so these are thin wrappers.  Two guards keep
 * them UB-free: division by zero (UB in C — fail loudly) and INT64_MIN / -1
 * (signed-overflow UB — return the two's-complement wrap that `-fwrapv` gives,
 * so it agrees with the reference and the width `Convert` then narrows it). */
SirValue _sir_itdiv(SirValue a, SirValue b) {
    if (b.as.i == 0) { fprintf(stderr, "sir: divided by 0\n"); exit(1); }
    if (a.as.i == INT64_MIN && b.as.i == -1) return _sir_int(INT64_MIN);
    return _sir_int(a.as.i / b.as.i);
}
SirValue _sir_itmod(SirValue a, SirValue b) {
    if (b.as.i == 0) { fprintf(stderr, "sir: divided by 0\n"); exit(1); }
    if (a.as.i == INT64_MIN && b.as.i == -1) return _sir_int(0);
    return _sir_int(a.as.i % b.as.i);
}
/* Unsigned truncating division / remainder.  A uint64_t whose top bit is set is
 * a negative int64, so signed division would be wrong — do it over uint64.  No
 * INT64_MIN/-1 guard is needed: unsigned division never overflows or traps. */
SirValue _sir_utdiv(SirValue a, SirValue b) {
    if (b.as.i == 0) { fprintf(stderr, "sir: divided by 0\n"); exit(1); }
    return _sir_int((int64_t)((uint64_t)a.as.i / (uint64_t)b.as.i));
}
SirValue _sir_utmod(SirValue a, SirValue b) {
    if (b.as.i == 0) { fprintf(stderr, "sir: divided by 0\n"); exit(1); }
    return _sir_int((int64_t)((uint64_t)a.as.i % (uint64_t)b.as.i));
}

SirValue _sir_divide_v(SirValue *xs, int n) {
    int i;
    if (n <= 0) return _sir_int(1);
    {
        int anyf = 0;
        for (i = 0; i < n; i++) if (xs[i].tag == SIR_FLOAT) anyf = 1;
        if (anyf) {
            double acc = _sir_as_num(xs[0]);
            for (i = 1; i < n; i++) acc /= _sir_as_num(xs[i]);
            return _sir_float(acc);
        }
        {
            int64_t acc = xs[0].as.i;
            for (i = 1; i < n; i++) acc = _sir_ifloordiv(acc, xs[i].as.i);
            return _sir_int(acc);
        }
    }
}

SirValue _sir_plus(int n, ...)   { va_list ap; SirValue *xs; SirValue r; va_start(ap, n); xs = _sir_va_collect(n, ap); va_end(ap); r = _sir_plus_v(xs, n);   if (xs) free(xs); return r; }
SirValue _sir_minus(int n, ...)  { va_list ap; SirValue *xs; SirValue r; va_start(ap, n); xs = _sir_va_collect(n, ap); va_end(ap); r = _sir_minus_v(xs, n);  if (xs) free(xs); return r; }
SirValue _sir_times(int n, ...)  { va_list ap; SirValue *xs; SirValue r; va_start(ap, n); xs = _sir_va_collect(n, ap); va_end(ap); r = _sir_times_v(xs, n);  if (xs) free(xs); return r; }
SirValue _sir_divide(int n, ...) { va_list ap; SirValue *xs; SirValue r; va_start(ap, n); xs = _sir_va_collect(n, ap); va_end(ap); r = _sir_divide_v(xs, n); if (xs) free(xs); return r; }

/* ---- comparison + equality ---------------------------------- */

SirValue _sir_lt(SirValue a, SirValue b) {
    if (a.tag == SIR_STR && b.tag == SIR_STR) return _sir_bool(strcmp(a.as.s, b.as.s) < 0);
    return _sir_bool(_sir_as_num(a) < _sir_as_num(b));
}
SirValue _sir_gt(SirValue a, SirValue b) {
    if (a.tag == SIR_STR && b.tag == SIR_STR) return _sir_bool(strcmp(a.as.s, b.as.s) > 0);
    return _sir_bool(_sir_as_num(a) > _sir_as_num(b));
}
SirValue _sir_le(SirValue a, SirValue b) {
    if (a.tag == SIR_STR && b.tag == SIR_STR) return _sir_bool(strcmp(a.as.s, b.as.s) <= 0);
    return _sir_bool(_sir_as_num(a) <= _sir_as_num(b));
}
SirValue _sir_ge(SirValue a, SirValue b) {
    if (a.tag == SIR_STR && b.tag == SIR_STR) return _sir_bool(strcmp(a.as.s, b.as.s) >= 0);
    return _sir_bool(_sir_as_num(a) >= _sir_as_num(b));
}

/* Structural equality can recurse through nested sequences/pairs. `SeqSet` is
 * this backend's FIRST mutable heap aggregate, so — unlike the immutable cons
 * pairs, which cannot form a cycle (there is no `set-car!`) — a sequence CAN be
 * made self-referential (`a[0] = a`). A depth cap keeps such a structure from
 * overflowing the C stack: past the cap two values are ASSUMED equal (the
 * co-inductive answer, so a cyclic comparison terminates). The cap is far
 * beyond any real (non-cyclic) nesting and comfortably under the stack limit. */
#define SIR_MAX_EQ_DEPTH 5000

int _sir_value_eq_d(SirValue a, SirValue b, int depth) {
    if (depth > SIR_MAX_EQ_DEPTH) return 1;
    if (_sir_is_num(a) && _sir_is_num(b)) {
        if (a.tag == SIR_INT && b.tag == SIR_INT) return a.as.i == b.as.i;
        return _sir_as_num(a) == _sir_as_num(b);
    }
    if (a.tag != b.tag) return 0;
    switch (a.tag) {
        case SIR_NIL:     return 1;
        case SIR_BOOL:    return a.as.b == b.as.b;
        case SIR_STR:     return strcmp(a.as.s, b.as.s) == 0;
        case SIR_SYM:     return a.as.s == b.as.s;  /* interned: pointer eq */
        case SIR_PAIR:    return _sir_value_eq_d(a.as.pair->car, b.as.pair->car, depth + 1)
                              && _sir_value_eq_d(a.as.pair->cdr, b.as.pair->cdr, depth + 1);
        case SIR_SEQ: {
            /* STRUCTURAL: equal length and element-wise equal (`[1, 2] ==
             * [1, 2]` is true). The identical-handle fast path short-circuits
             * `a == a` (and the common self-referential `a[0] = a`); the depth
             * cap above bounds a comparison of two DISTINCT cyclic sequences. */
            SirSeq *sa = a.as.seq, *sb = b.as.seq;
            if (sa == sb) return 1;
            if (sa->len != sb->len) return 0;
            for (int64_t i = 0; i < sa->len; i++)
                if (!_sir_value_eq_d(sa->items[i], sb->items[i], depth + 1)) return 0;
            return 1;
        }
        case SIR_MAP: {
            /* STRUCTURAL and POSITIONAL: equal length, then entry-wise in
             * INSERTION ORDER — `entries[i]` keys AND values equal — exactly
             * mirroring the Go (`[]MapEntry` zip) and Rust (`iter().zip()`)
             * reference backends. (Ruby's own `Hash#==` is order-INsensitive;
             * all three source-emitting backends are positional, so they agree
             * with each other — a uniform, documented divergence from real Ruby
             * on the untested reordered-map case, not a C-only bug.) The
             * identical-handle fast path short-circuits `m == m` and the
             * self-referential `m[k] = m`; the depth cap bounds two DISTINCT
             * cyclic maps (constructible now that `MapSet` mutates in place). */
            SirMap *ma = a.as.map, *mb = b.as.map;
            if (ma == mb) return 1;
            if (ma->len != mb->len) return 0;
            for (int64_t i = 0; i < ma->len; i++) {
                if (!_sir_value_eq_d(ma->entries[i].key, mb->entries[i].key, depth + 1)) return 0;
                if (!_sir_value_eq_d(ma->entries[i].val, mb->entries[i].val, depth + 1)) return 0;
            }
            return 1;
        }
        case SIR_CLOSURE: return a.as.clo == b.as.clo;
        /* Two exceptions are equal only when they are the SAME handle (Ruby's
         * default `==` is object identity), so a `rescue => e` binding compares
         * equal to itself. */
        case SIR_ERROR:   return a.as.err == b.as.err;
        /* Two instances are equal only when they are the SAME object (Ruby's
         * default `==` on a user object is identity) — the inline pointer IS the
         * identity, so this is a plain pointer compare. */
        case SIR_INSTANCE: return a.as.inst == b.as.inst;
        default:          return 0;
    }
}

int _sir_value_eq(SirValue a, SirValue b) { return _sir_value_eq_d(a, b, 0); }
SirValue _sir_eq(SirValue a, SirValue b) { return _sir_bool(_sir_value_eq(a, b)); }
SirValue _sir_ne(SirValue a, SirValue b) { return _sir_bool(!_sir_value_eq(a, b)); }

/* Logical negation: `!v` under SIR truthiness (only nil/false are falsy). */
SirValue _sir_not(SirValue v) { return _sir_bool(!_sir_truthy(v)); }

/* Bitwise / shift on the int64 value domain.  The frontend wraps every result
 * in a `Convert` to enforce the C width, so these operate on the full int64 and
 * leave the masking to `_sir_convert`.
 *   - `&`/`|`/`^`/`~` are pure bit operations.
 *   - `<<` shifts the *bit pattern*: done through uint64 so a shift that moves
 *     bits into or past the sign position is well-defined (no signed-overflow
 *     UB), matching the two's-complement result the mask then reduces.
 *   - `>>` uses the native int64 shift, which is arithmetic for a negative
 *     (signed) operand and logical for a masked non-negative (unsigned) one —
 *     exactly C's rule, since the operand carries its signedness. */
SirValue _sir_band(SirValue a, SirValue b) { return _sir_int(a.as.i & b.as.i); }
SirValue _sir_bor(SirValue a, SirValue b)  { return _sir_int(a.as.i | b.as.i); }
SirValue _sir_bxor(SirValue a, SirValue b) { return _sir_int(a.as.i ^ b.as.i); }
SirValue _sir_bnot(SirValue v)             { return _sir_int(~v.as.i); }
SirValue _sir_shl(SirValue a, SirValue b) {
    return _sir_int((int64_t)((uint64_t)a.as.i << (b.as.i & 63)));
}
SirValue _sir_shr(SirValue a, SirValue b) {
    return _sir_int(a.as.i >> (b.as.i & 63));
}
/* Logical right shift for *unsigned* operands: shift the bit pattern through
 * uint64 so a value whose top bit is set (a `uint64_t` stored as a negative
 * int64) does not sign-extend.  `int64_t >>` would be arithmetic there. */
SirValue _sir_lshr(SirValue a, SirValue b) {
    return _sir_int((int64_t)((uint64_t)a.as.i >> (b.as.i & 63)));
}

/* Ruby `===` (case subsumption).  For the v0 value set — no classes, ranges, or
 * regexps yet — `pattern === value` reduces to structural equality, so `case`
 * / `when` over literals behaves correctly.  Later batches extend this for
 * class / range / regexp receivers. */
SirValue _sir_case_eq(SirValue a, SirValue b) { return _sir_bool(_sir_value_eq(a, b)); }

/* ---- pairs + type predicates -------------------------------- */

SirValue _sir_cons(SirValue a, SirValue b) {
    SirPair *p = (SirPair *)_sir_alloc(sizeof(SirPair));
    SirValue v;
    p->car = a; p->cdr = b;
    v.tag = SIR_PAIR; v.as.pair = p;
    return v;
}
SirValue _sir_car(SirValue v) { return v.tag == SIR_PAIR ? v.as.pair->car : _sir_nil(); }
SirValue _sir_cdr(SirValue v) { return v.tag == SIR_PAIR ? v.as.pair->cdr : _sir_nil(); }

/* ---- SIR16 sequences ---------------------------------------- */

/* `[e0, e1, …]` — box `n` values into a fresh heap sequence. The variadic
 * elements are copied into an arena-allocated array. */
SirValue _sir_seq_lit(int n, ...) {
    SirSeq *s = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    s->len = (int64_t)n;
    s->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    va_list ap;
    va_start(ap, n);
    for (int i = 0; i < n; i++) s->items[i] = va_arg(ap, SirValue);
    va_end(ap);
    SirValue v;
    v.tag = SIR_SEQ; v.as.seq = s;
    return v;
}

/* `a[i]` (read). Ruby's `Array#[]`: a negative index counts from the end, and
 * an index outside `0 .. len-1` yields nil (it does NOT raise — that is
 * `fetch`). Matches the Go/Rust `_sir_seq_index`. A non-sequence yields nil. */
SirValue _sir_seq_index(SirValue seq, SirValue idx) {
    if (seq.tag != SIR_SEQ) return _sir_nil();
    int64_t n = seq.as.seq->len;
    int64_t i = _sir_as_int(idx);
    if (i < 0) i += n;
    if (i < 0 || i >= n) return _sir_nil();
    return seq.as.seq->items[i];
}

/* `a.length` — the element count. A non-sequence has length 0. */
SirValue _sir_seq_len(SirValue seq) {
    return _sir_int(seq.tag == SIR_SEQ ? seq.as.seq->len : 0);
}

/* `a[i] = v` (write). Unlike the lenient read, the SIR reference treats ONLY
 * `0 <= i < len` as valid and traps on a negative or out-of-range index
 * (matching the Go/Rust `_sir_seq_set`, which panic). Mutates the shared box
 * and returns the value (an indexed assignment evaluates to its RHS). */
SirValue _sir_seq_set(SirValue seq, SirValue idx, SirValue val) {
    if (seq.tag != SIR_SEQ) {
        fprintf(stderr, "sir: []= on a non-sequence\n");
        exit(1);
    }
    int64_t n = seq.as.seq->len;
    int64_t i = _sir_as_int(idx);
    if (i < 0 || i >= n) {
        fprintf(stderr, "sir: sequence index out of range\n");
        exit(1);
    }
    seq.as.seq->items[i] = val;
    return val;
}

/* Normalise an iterable to a SNAPSHOT sequence for `ForEach`. A real sequence
 * is copied (so a body that mutates the original does not disturb the
 * iteration — matching the Go/Rust snapshot semantics); a cons-list is
 * flattened into a sequence; anything else yields an empty sequence (zero
 * iterations — the lenient choice, consistent with `_sir_car` returning nil on
 * a non-pair). Lets the emitter render a `ForEach` body ONCE, over one array. */
SirValue _sir_seq_iter(SirValue it) {
    SirSeq *s = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    if (it.tag == SIR_SEQ) {
        int64_t n = it.as.seq->len;
        s->len = n;
        s->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
        for (int64_t i = 0; i < n; i++) s->items[i] = it.as.seq->items[i];
    } else if (it.tag == SIR_PAIR) {
        int64_t n = 0;
        SirValue cur = it;
        while (cur.tag == SIR_PAIR) { n++; cur = cur.as.pair->cdr; }
        s->len = n;
        s->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
        cur = it;
        for (int64_t i = 0; cur.tag == SIR_PAIR; i++) { s->items[i] = cur.as.pair->car; cur = cur.as.pair->cdr; }
    } else {
        s->len = 0;
        s->items = NULL;
    }
    SirValue v;
    v.tag = SIR_SEQ; v.as.seq = s;
    return v;
}

/* ---- SIR16 maps --------------------------------------------- */

/* A fresh map with room for `cap` entries (cap 0 => NULL backing store). */
static SirMap *_sir_map_new(int64_t cap) {
    SirMap *m = (SirMap *)_sir_alloc(sizeof(SirMap));
    m->len = 0;
    m->cap = cap;
    m->entries = (cap > 0)
        ? (struct SirMapEntry *)_sir_alloc(sizeof(struct SirMapEntry) * (size_t)cap)
        : NULL;
    return m;
}

/* Linear scan for `key` by STRUCTURAL equality (`_sir_value_eq`, so a composite
 * key like `[1, 2]` matches by value, not identity). Returns the entry index,
 * or -1 if absent. O(n) — an assoc-array, matching the Go/Rust reference. */
static int64_t _sir_map_find(SirMap *m, SirValue key) {
    for (int64_t i = 0; i < m->len; i++)
        if (_sir_value_eq(m->entries[i].key, key)) return i;
    return -1;
}

/* Insert or update `key => val`, PRESERVING insertion order: an existing key's
 * value is overwritten in its current slot; a new key is APPENDED, growing the
 * backing array (capacity doubles, from 4, when full). The arena never frees,
 * so the outgrown array simply leaks like every other reallocation here. */
static void _sir_map_put(SirMap *m, SirValue key, SirValue val) {
    int64_t at = _sir_map_find(m, key);
    if (at >= 0) { m->entries[at].val = val; return; }
    if (m->len == m->cap) {
        int64_t ncap = (m->cap > 0) ? m->cap * 2 : 4;
        struct SirMapEntry *ne =
            (struct SirMapEntry *)_sir_alloc(sizeof(struct SirMapEntry) * (size_t)ncap);
        for (int64_t i = 0; i < m->len; i++) ne[i] = m->entries[i];
        m->entries = ne;
        m->cap = ncap;
    }
    m->entries[m->len].key = key;
    m->entries[m->len].val = val;
    m->len++;
}

/* `{k0 => v0, k1 => v1, …}` — build a map from `n` key/value pairs passed as
 * `2*n` variadic `SirValue`s in `k0, v0, k1, v1, …` order. A later duplicate
 * key OVERWRITES the earlier entry (via `_sir_map_put`), matching Ruby's Hash
 * literal and the Go/Rust `_sir_map_lit`, so `{1 => 1, 1 => 2}` is `{1 => 2}`. */
SirValue _sir_map_lit(int n, ...) {
    SirMap *m = _sir_map_new((int64_t)n);
    va_list ap;
    va_start(ap, n);
    for (int i = 0; i < n; i++) {
        SirValue k = va_arg(ap, SirValue);
        SirValue val = va_arg(ap, SirValue);
        _sir_map_put(m, k, val);
    }
    va_end(ap);
    SirValue v;
    v.tag = SIR_MAP; v.as.map = m;
    return v;
}

/* `h[k]` (read). A MISSING key yields nil — Ruby's default-less `Hash#[]` does
 * not raise, matching the Go/Rust `_sir_map_get`. A non-map also yields nil
 * (the lenient read, mirroring this backend's own `_sir_seq_index`). */
SirValue _sir_map_get(SirValue map, SirValue key) {
    if (map.tag != SIR_MAP) return _sir_nil();
    int64_t at = _sir_map_find(map.as.map, key);
    return (at >= 0) ? map.as.map->entries[at].val : _sir_nil();
}

/* `h[k] = v` (write). Insert-or-update, mutating the SHARED map so a write
 * through one binding is visible through every alias (the value is a handle,
 * like the Go/Rust `*Map`). A map has no bounds, so — unlike `_sir_seq_set` —
 * there is no index to trap on; a non-map is the only error. Returns the
 * assigned value (an indexed assignment evaluates to its RHS). */
SirValue _sir_map_set(SirValue map, SirValue key, SirValue val) {
    if (map.tag != SIR_MAP) {
        fprintf(stderr, "sir: []= on a non-map\n");
        exit(1);
    }
    _sir_map_put(map.as.map, key, val);
    return val;
}

SirValue _sir_is_null(SirValue v)   { return _sir_bool(v.tag == SIR_NIL); }
SirValue _sir_is_pair(SirValue v)   { return _sir_bool(v.tag == SIR_PAIR); }
SirValue _sir_is_number(SirValue v) { return _sir_bool(_sir_is_num(v)); }
SirValue _sir_is_symbol(SirValue v) { return _sir_bool(v.tag == SIR_SYM); }

/* ---- closures ----------------------------------------------- */

SirValue _sir_make_closure(SirFn fn, int ncap, ...) {
    SirClosure *c = (SirClosure *)_sir_alloc(sizeof(SirClosure));
    SirValue v;
    va_list ap;
    int i;
    c->fn = fn; c->ncap = ncap;
    c->caps = (ncap > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)ncap) : NULL;
    va_start(ap, ncap);
    for (i = 0; i < ncap; i++) c->caps[i] = va_arg(ap, SirValue);
    va_end(ap);
    v.tag = SIR_CLOSURE; v.as.clo = c;
    return v;
}

SirValue _sir_apply(SirValue target, int argc, ...) {
    va_list ap; SirValue *args; SirValue r;
    va_start(ap, argc);
    args = _sir_va_collect(argc, ap);
    va_end(ap);
    if (target.tag != SIR_CLOSURE) {
        fprintf(stderr, "sir: attempt to call a non-closure value\n");
        exit(1);
    }
    r = target.as.clo->fn(target.as.clo->caps, args, argc);
    if (args) free(args);
    return r;
}

/* ---- global store (string-keyed) ---------------------------- */

#define SIR_GLOBALS_MAX 4096
static const char *_sir_gkey[SIR_GLOBALS_MAX];
static SirValue     _sir_gval[SIR_GLOBALS_MAX];
static int          _sir_gn = 0;

SirValue _sir_global_set_s(const char *name, SirValue val) {
    const char *k = _sir_intern(name);
    int i;
    for (i = 0; i < _sir_gn; i++) if (_sir_gkey[i] == k) { _sir_gval[i] = val; return val; }
    if (_sir_gn < SIR_GLOBALS_MAX) { _sir_gkey[_sir_gn] = k; _sir_gval[_sir_gn] = val; _sir_gn++; }
    return val;
}
SirValue _sir_global_get_s(const char *name) {
    const char *k = _sir_intern(name);
    int i;
    for (i = 0; i < _sir_gn; i++) if (_sir_gkey[i] == k) return _sir_gval[i];
    return _sir_nil();
}
SirValue _sir_global_set(SirValue name, SirValue val) { return _sir_global_set_s(_sir_str_of(name), val); }
SirValue _sir_global_get(SirValue name)               { return _sir_global_get_s(_sir_str_of(name)); }

/* ---- display / output --------------------------------------- */

void _sir_fmt(FILE *out, SirValue v);

/* A sequence renders as `[e0, e1, …]` (bracket, comma-space separator),
 * matching the Go/Rust backends. Elements render through `_sir_fmt`, whose
 * depth counter bounds a self-referential sequence (constructible via the
 * mutable `SeqSet`). */
void _sir_fmt_seq(FILE *out, SirValue v) {
    fputc('[', out);
    for (int64_t i = 0; i < v.as.seq->len; i++) {
        if (i) fputs(", ", out);
        _sir_fmt(out, v.as.seq->items[i]);
    }
    fputc(']', out);
}

/* A map renders as `{k0: v0, k1: v1, …}` in insertion order — a brace-wrapped,
 * colon-space entry list, EXACTLY mirroring the Go (`_sir_format_map`) and Rust
 * (`format_map_d`) backends so the three source targets print maps identically.
 * (Real Ruby's `Hash#inspect` uses ` => ` for non-symbol keys and `key:` only
 * for symbol keys; all three emitting backends use a uniform `: ` — a
 * documented family-wide divergence on the untested whole-map print, kept for
 * cross-backend agreement.) Keys and values render through `_sir_fmt`, whose
 * depth counter bounds a self-referential map (constructible via `MapSet`). */
void _sir_fmt_map(FILE *out, SirValue v) {
    fputc('{', out);
    for (int64_t i = 0; i < v.as.map->len; i++) {
        if (i) fputs(", ", out);
        _sir_fmt(out, v.as.map->entries[i].key);
        fputs(": ", out);
        _sir_fmt(out, v.as.map->entries[i].val);
    }
    fputc('}', out);
}

void _sir_fmt_float(FILE *out, double f) {
    /* Ruby keeps a trailing .0 on integral floats and prints non-finite
     * values as Infinity / NaN.  A plain "%g" would drop the .0, so format
     * and patch. */
    char buf[64];
    if (f != f)               { fputs("NaN", out); return; }
    if (f == f * 0.5 && f != 0.0) { fputs(f < 0 ? "-Infinity" : "Infinity", out); return; }
    snprintf(buf, sizeof(buf), "%.17g", f);
    if (!strchr(buf, '.') && !strchr(buf, 'e') && !strchr(buf, 'E') &&
        !strchr(buf, 'n') && !strchr(buf, 'N')) {
        size_t L = strlen(buf);
        if (L + 2 < sizeof(buf)) { buf[L] = '.'; buf[L + 1] = '0'; buf[L + 2] = '\0'; }
    }
    fputs(buf, out);
}

void _sir_fmt_pair(FILE *out, SirValue v) {
    SirValue cur = v;
    int first = 1;
    fputc('(', out);
    for (;;) {
        if (cur.tag == SIR_PAIR) {
            if (!first) fputc(' ', out);
            first = 0;
            _sir_fmt(out, cur.as.pair->car);
            cur = cur.as.pair->cdr;
        } else if (cur.tag == SIR_NIL) {
            break;
        } else {
            fputs(" . ", out);
            _sir_fmt(out, cur);
            break;
        }
    }
    fputc(')', out);
}

/* Rendering recurses through nested sequences/pairs. A self-referential
 * sequence (`a[0] = a`, now constructible via the mutable `SeqSet`) would
 * otherwise recurse forever, so a static depth counter bounds it: past the cap
 * a `[...]` ellipsis is printed instead of descending (the emitted program is
 * single-threaded, so a plain static counter is sufficient). */
#define SIR_MAX_FMT_DEPTH 5000
static int _sir_fmt_depth = 0;

void _sir_fmt(FILE *out, SirValue v) {
    char buf[32];
    if (_sir_fmt_depth > SIR_MAX_FMT_DEPTH) {
        fputs("[...]", out);
        return;
    }
    _sir_fmt_depth++;
    switch (v.tag) {
        case SIR_NIL:   fputs(SIR_DISPLAY_RUBY ? "" : "nil", out); break;
        case SIR_BOOL:  fputs(v.as.b ? (SIR_DISPLAY_RUBY ? "true" : "#t")
                                     : (SIR_DISPLAY_RUBY ? "false" : "#f"), out); break;
        case SIR_INT:   snprintf(buf, sizeof(buf), "%lld", (long long)v.as.i); fputs(buf, out); break;
        case SIR_FLOAT: _sir_fmt_float(out, v.as.f); break;
        case SIR_STR:   fputs(v.as.s, out); break;
        case SIR_SYM:   fputs(v.as.s, out); break;
        case SIR_PAIR:  _sir_fmt_pair(out, v); break;
        case SIR_SEQ:   _sir_fmt_seq(out, v); break;
        case SIR_MAP:   _sir_fmt_map(out, v); break;
        case SIR_CLOSURE: fputs("#<closure>", out); break;
        /* An exception prints as its message (Ruby's `exception.message`): the
         * raised message when present, else the class name — so `rescue => e;
         * print(e)` shows something meaningful. */
        case SIR_ERROR:
            if (v.as.err->msg.tag == SIR_NIL) fputs(v.as.err->sir_class, out);
            else _sir_fmt(out, v.as.err->msg);
            break;
        /* An instance prints as `#<Foo>` (its class name).  Deterministic — no
         * address — so tests can assert on it (Ruby's default `#<Foo:0x…>` would
         * embed a non-reproducible pointer). */
        case SIR_INSTANCE:
            fputs("#<", out); fputs(v.as.inst->sir_class, out); fputc('>', out); break;
        default: break;
    }
    _sir_fmt_depth--;
}

/* ---- SIR17 exceptions (setjmp/longjmp handler stack) -------- */

/* Construct an exception value.  `class` is interned; `msg` is a `SIR_STR` (or
 * nil for a bare `raise Class`, whose message defaults to the class name). */
SirValue _sir_error(const char *sir_class, SirValue msg) {
    SirError *e = (SirError *)_sir_alloc(sizeof(SirError));
    e->sir_class = _sir_intern(sir_class);
    e->msg = msg;
    SirValue v;
    v.tag = SIR_ERROR; v.as.err = e;
    return v;
}

/* OOP slice 4: user-declared inheritance (`class Dog < Animal`), registered in
 * program order by the emitted `_sir_register_super`.  A class has ONE super, so
 * this is a `sub -> super` table (update-or-append on the interned sub name).
 * `_sir_class_super` consults it FIRST, so the SAME `super_of` relation drives
 * BOTH `rescue`-by-class matching AND OOP method resolution (a user class that
 * subclasses a built-in exception is picked up by rescue too). */
#define SIR_USER_SUPER_MAX 4096
static struct { const char *sub; const char *sup; } _sir_user_super_tab[SIR_USER_SUPER_MAX];
static int _sir_user_super_n = 0;

void _sir_register_super(const char *sub, const char *sup) {
    const char *b = _sir_intern(sub), *p = _sir_intern(sup);
    int i;
    for (i = 0; i < _sir_user_super_n; i++) {
        if (_sir_user_super_tab[i].sub == b) { _sir_user_super_tab[i].sup = p; return; }
    }
    if (_sir_user_super_n < SIR_USER_SUPER_MAX) {
        _sir_user_super_tab[_sir_user_super_n].sub = b;
        _sir_user_super_tab[_sir_user_super_n].sup = p;
        _sir_user_super_n++;
    }
}

/* An ancestry walk is bounded by this many steps: a well-formed hierarchy is far
 * shorter, but a HAND-BUILT module could register a cycle (`A<B`, `B<A`) the Ruby
 * frontend never emits (Ruby rejects cyclic inheritance) — the cap turns that
 * into a clean "not found" instead of an infinite loop (DoS). */
#define SIR_ANCESTRY_MAX 4096

/* The class ancestry (`sub → super`): a user-declared super wins; otherwise the
 * built-in exception hierarchy (baked in so a `rescue StandardError` matches a
 * raised `RuntimeError`).  A NULL super terminates the chain (`Exception` /
 * `Object` is the root).  An unlisted class has no super (matches only itself /
 * a bare rescue). */
static const char *_sir_class_super(const char *cls) {
    {
        const char *c = _sir_intern(cls);
        int i;
        for (i = 0; i < _sir_user_super_n; i++) {
            if (_sir_user_super_tab[i].sub == c) return _sir_user_super_tab[i].sup;
        }
    }
    static const struct { const char *sub; const char *sup; } A[] = {
        { "RuntimeError", "StandardError" },
        { "ArgumentError", "StandardError" },
        { "TypeError", "StandardError" },
        { "NameError", "StandardError" },
        { "NoMethodError", "NameError" },
        { "IndexError", "StandardError" },
        { "KeyError", "IndexError" },
        { "RangeError", "StandardError" },
        { "ZeroDivisionError", "StandardError" },
        { "IOError", "StandardError" },
        { "StopIteration", "StandardError" },
        { "NotImplementedError", "StandardError" },
        { "StandardError", "Exception" },
    };
    for (size_t i = 0; i < sizeof(A) / sizeof(A[0]); i++) {
        if (strcmp(cls, A[i].sub) == 0) return A[i].sup;
    }
    return NULL;
}

/* True iff exception class `actual` IS-A `target` — equal, or descends from it
 * through the ancestry chain. */
int _sir_class_is_a(const char *actual, const char *target) {
    const char *cur = actual;
    int steps = 0;
    while (cur && steps++ < SIR_ANCESTRY_MAX) {
        if (strcmp(cur, target) == 0) return 1;
        cur = _sir_class_super(cur);
    }
    return 0;
}

/* Does exception `err` match a `rescue` clause listing `n` class names?  An
 * empty list (`n == 0`, a bare `rescue`) catches every exception; otherwise the
 * error's class must be-a one of the listed classes. */
int _sir_rescue_matches(SirValue err, const char *const *classes, int n) {
    if (err.tag != SIR_ERROR) return 0;
    if (n == 0) return 1;
    for (int i = 0; i < n; i++) {
        if (_sir_class_is_a(err.as.err->sir_class, classes[i])) return 1;
    }
    return 0;
}

/* The handler stack.  A `TryCatch` pushes a `jmp_buf` and `setjmp`s it; `raise`
 * `longjmp`s to the top.  Single-threaded emitted program, so a plain static
 * stack is safe.  `_sir_current_error` holds the exception being handled (for a
 * bare re-`raise` and for the rescue-clause dispatch). */
#define SIR_MAX_HANDLERS 1024
static jmp_buf _sir_handlers[SIR_MAX_HANDLERS];
static int _sir_handler_top = -1;
static SirValue _sir_current_error;

/* Push a handler slot and return its index (to `setjmp`).  Overflowing the
 * fixed stack is a hard error (pathological handler nesting). */
int _sir_push_handler(void) {
    if (_sir_handler_top + 1 >= SIR_MAX_HANDLERS) {
        fprintf(stderr, "sir: exception handler stack overflow\n");
        exit(1);
    }
    return ++_sir_handler_top;
}
void _sir_pop_handler(void) {
    if (_sir_handler_top >= 0) _sir_handler_top--;
}

/* Raise (or re-raise) an exception.  Records it as the current error and
 * `longjmp`s to the top handler; with no handler installed, it is uncaught —
 * print `class: message` to stderr and exit non-zero (Ruby's default).  Returns
 * `SirValue` only to fit the builtin-call expression contract; it never returns
 * normally (`longjmp`/`exit`). */
SirValue _sir_raise(SirValue exc) {
    _sir_current_error = exc;
    if (_sir_handler_top >= 0) {
        longjmp(_sir_handlers[_sir_handler_top], 1);
    }
    if (exc.tag == SIR_ERROR) {
        fputs(exc.as.err->sir_class, stderr);
        if (exc.as.err->msg.tag != SIR_NIL) {
            fputs(": ", stderr);
            _sir_fmt(stderr, exc.as.err->msg);
        }
    }
    fputc('\n', stderr);
    exit(1);
    return _sir_nil(); /* unreachable */
}

/* `raise <value>`: an exception object is re-raised as-is; any other value
 * (typically a message string) becomes a `RuntimeError` carrying it — matching
 * Ruby's `raise "boom"`. */
SirValue _sir_raise_value(SirValue v) {
    if (v.tag == SIR_ERROR) return _sir_raise(v);
    return _sir_raise(_sir_error("RuntimeError", v));
}

/* Read a SIR constant by name (`_sir_const_set` populated it).  A read of an
 * undefined constant raises `NameError` — a rescuable exception (matching Ruby's
 * `uninitialized constant`), not a hard exit. */
SirValue _sir_const_get(const char *name) {
    const char *n = _sir_intern(name);
    int i;
    for (i = 0; i < _sir_const_n; i++) {
        if (_sir_const_tab[i].name == n) return _sir_const_tab[i].val;
    }
    return _sir_raise(_sir_error("NameError", _sir_str(_sir_cat("uninitialized constant ", name))));
}

/* ---- OOP: current self + instance variables (@x) ------------ */

/* The receiver of the method currently executing.  A method body runs in a
 * hoisted top-level function (no lexical `self`), so `_sir_call_method` sets this
 * before applying the closure and restores it after; `@x` and `self` read it.
 * Nil at top level (Ruby's `main`). */
static SirValue _sir_current_self = { SIR_NIL, { 0 } };

/* Instance variables (`@x`) when `self` is NOT an instance (top-level `main`).
 * Lazily allocated, matching Ruby's `main`-object ivars. */
static SirMap *_sir_toplevel_ivars = NULL;

SirValue _sir_self(void) { return _sir_current_self; }

/* The `@name -> value` map owner for the current `self`: the instance's own
 * `ivars` slot (an instance), else the top-level bag. */
static SirMap **_sir_ivar_owner(void) {
    if (_sir_current_self.tag == SIR_INSTANCE) return &_sir_current_self.as.inst->ivars;
    return &_sir_toplevel_ivars;
}

/* `@x` read — nil when unset (Ruby's semantics), never an error. */
SirValue _sir_ivar_get(const char *name) {
    SirMap *m = *_sir_ivar_owner();
    SirValue mv;
    if (!m) return _sir_nil();
    mv.tag = SIR_MAP; mv.as.map = m;
    return _sir_map_get(mv, _sir_sym(name));
}

/* `@x = v` write — lazily allocates the owner's ivar map.  The `@`-name is an
 * interned symbol key (so lookup is a pointer compare). */
SirValue _sir_ivar_set(const char *name, SirValue v) {
    SirMap **owner = _sir_ivar_owner();
    if (!*owner) *owner = _sir_map_new(4);
    _sir_map_put(*owner, _sir_sym(name), v);
    return v;
}

/* ---- OOP: instance-method table & dispatch ------------------ */

/* An EXPLICIT (class, method) -> closure table, populated by emitted
 * `__def_method__` registrations.  Dispatch (`__method__`) is a DATA lookup on
 * the (interned class, interned method) key — NEVER reflection on a
 * source-derived string — so a user method literally named `system`/`eval` is
 * only ever a table KEY, and an unresolved method is a controlled `NoMethodError`
 * (the anti-RCE invariant, SIR24 §Security #2).  Keys are interned, so the scan
 * is a pointer compare.  (Slice 2: a flat table; inheritance walks the ancestry
 * in a later slice.) */
#define SIR_METHOD_MAX 8192
static struct { const char *cls; const char *method; SirValue fn; } _sir_method_tab[SIR_METHOD_MAX];
static int _sir_method_n = 0;

SirValue _sir_def_method(const char *cls, const char *method, SirValue fn) {
    const char *c = _sir_intern(cls), *m = _sir_intern(method);
    int i;
    for (i = 0; i < _sir_method_n; i++) {
        if (_sir_method_tab[i].cls == c && _sir_method_tab[i].method == m) {
            _sir_method_tab[i].fn = fn;
            return fn;
        }
    }
    if (_sir_method_n < SIR_METHOD_MAX) {
        _sir_method_tab[_sir_method_n].cls = c;
        _sir_method_tab[_sir_method_n].method = m;
        _sir_method_tab[_sir_method_n].fn = fn;
        _sir_method_n++;
    }
    return fn;
}

/* Look up `(cls, method)` -> closure on THIS class only (no ancestry), or a
 * `SIR_NIL` sentinel on a miss. */
static SirValue _sir_lookup_method(const char *cls, const char *method) {
    const char *c = _sir_intern(cls), *m = _sir_intern(method);
    int i;
    for (i = 0; i < _sir_method_n; i++) {
        if (_sir_method_tab[i].cls == c && _sir_method_tab[i].method == m) {
            return _sir_method_tab[i].fn;
        }
    }
    return _sir_nil();
}

/* OOP slice 4: resolve `method` starting at `cls` and walking UP the ancestry
 * (`_sir_class_super`) until a defining class is found — so an inherited method
 * dispatches to the closure the superclass registered.  Returns the closure or
 * `SIR_NIL`.  Bounded by `SIR_ANCESTRY_MAX` steps (a cyclic hand-built hierarchy
 * cannot hang). */
static SirValue _sir_resolve_method(const char *cls, const char *method) {
    const char *cur = cls;
    int steps = 0;
    while (cur && steps++ < SIR_ANCESTRY_MAX) {
        SirValue fn = _sir_lookup_method(cur, method);
        if (fn.tag == SIR_CLOSURE) return fn;
        cur = _sir_class_super(cur);
    }
    return _sir_nil();
}

/* Dispatch an instance method: resolve `(recv's class, method)` and apply the
 * closure to the args.  A non-instance receiver or an unresolved method is a
 * (rescuable) `NoMethodError`. */
SirValue _sir_call_method(SirValue recv, const char *method, int argc, ...) {
    va_list ap;
    SirValue *args, fn, r;
    if (recv.tag != SIR_INSTANCE) {
        return _sir_raise(_sir_error(
            "NoMethodError", _sir_str(_sir_cat("undefined method for a non-object receiver: ", method))));
    }
    /* Slice 4: resolve up the ancestry so an inherited method dispatches. */
    fn = _sir_resolve_method(recv.as.inst->sir_class, method);
    if (fn.tag != SIR_CLOSURE) {
        return _sir_raise(
            _sir_error("NoMethodError", _sir_str(_sir_cat("undefined method ", method))));
    }
    va_start(ap, argc);
    args = _sir_va_collect(argc, ap);
    va_end(ap);
    /* Bind `self` to the receiver for the method body (so `@x`/`self` see it),
     * restoring the caller's `self` afterwards.  Nested calls stack correctly via
     * these C-local saves.  If the body `raise`s, `longjmp` skips this restore —
     * an enclosing `TryCatch` restores `_sir_current_self` on the unwind path. */
    {
        SirValue saved_self = _sir_current_self;
        _sir_current_self = recv;
        r = fn.as.clo->fn(fn.as.clo->caps, args, argc);
        _sir_current_self = saved_self;
    }
    if (args) free(args);
    return r;
}

/* OOP slice 4: `super` from within `defining_class`'s method `method`.  Resolve
 * `method` starting at the SUPERCLASS of `defining_class` (so it does not
 * re-enter the same override) and apply it to the CURRENT `self` — `super` runs
 * in the same receiver (it does NOT rebind `self`), so `@x` and a nested method
 * call still see the original object.  No ancestor defines `method` => a
 * (rescuable) `NoMethodError`. */
SirValue _sir_call_super(const char *method, const char *defining_class, int argc, ...) {
    va_list ap;
    SirValue *args, fn, r;
    const char *sup = _sir_class_super(defining_class);
    fn = sup ? _sir_resolve_method(sup, method) : _sir_nil();
    if (fn.tag != SIR_CLOSURE) {
        return _sir_raise(_sir_error(
            "NoMethodError", _sir_str(_sir_cat("super: no superclass method ", method))));
    }
    va_start(ap, argc);
    args = _sir_va_collect(argc, ap);
    va_end(ap);
    r = fn.as.clo->fn(fn.as.clo->caps, args, argc);
    if (args) free(args);
    return r;
}

SirValue _sir_print_v(SirValue *xs, int n) {
    int i;
    for (i = 0; i < n; i++) _sir_fmt(stdout, xs[i]);
    return _sir_nil();
}
SirValue _sir_puts_v(SirValue *xs, int n) {
    int i;
    if (n <= 0) { fputc('\n', stdout); return _sir_nil(); }
    for (i = 0; i < n; i++) { _sir_fmt(stdout, xs[i]); fputc('\n', stdout); }
    return _sir_nil();
}
SirValue _sir_print(int n, ...) { va_list ap; SirValue *xs; SirValue r; va_start(ap, n); xs = _sir_va_collect(n, ap); va_end(ap); r = _sir_print_v(xs, n); if (xs) free(xs); return r; }
SirValue _sir_puts(int n, ...)  { va_list ap; SirValue *xs; SirValue r; va_start(ap, n); xs = _sir_va_collect(n, ap); va_end(ap); r = _sir_puts_v(xs, n);  if (xs) free(xs); return r; }

/* ---- builtin-as-value (VarRef Builtin) ---------------------- */

/* A builtin used in value position becomes a closure whose sole capture is
 * the builtin's NAME symbol; the dispatcher switches on that name.  The
 * switch IS the allowlist — an unknown name fails cleanly, never resolving
 * reflectively (the repo's anti-RCE discipline). */
/* Safe positional access: a builtin used as a first-class value and then
 * under-applied (fewer args than its fixed arity) must read `nil`, never index
 * out of bounds — `_sir_va_collect` returns NULL for argc == 0, so a bare
 * `args[i]` would dereference NULL.  The direct-call path already pads with
 * nil; this mirrors that for the indirect path. */
SirValue _sir_arg(SirValue *args, int argc, int i) {
    return (i < argc) ? args[i] : _sir_nil();
}
SirValue _sir_builtin_dispatch(SirValue *caps, SirValue *args, int argc) {
    const char *name = caps[0].as.s;
    if (strcmp(name, "+") == 0)        return _sir_plus_v(args, argc);
    if (strcmp(name, "-") == 0)        return _sir_minus_v(args, argc);
    if (strcmp(name, "*") == 0)        return _sir_times_v(args, argc);
    if (strcmp(name, "/") == 0)        return _sir_divide_v(args, argc);
    if (strcmp(name, "=") == 0)        return _sir_eq(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "<") == 0)        return _sir_lt(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, ">") == 0)        return _sir_gt(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "<=") == 0)       return _sir_le(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, ">=") == 0)       return _sir_ge(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "==") == 0)       return _sir_eq(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "!=") == 0)       return _sir_ne(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "not") == 0)      return _sir_not(_sir_arg(args, argc, 0));
    if (strcmp(name, "&") == 0)        return _sir_band(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "|") == 0)        return _sir_bor(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "^") == 0)        return _sir_bxor(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "~") == 0)        return _sir_bnot(_sir_arg(args, argc, 0));
    if (strcmp(name, "<<") == 0)       return _sir_shl(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, ">>") == 0)       return _sir_shr(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "u>>") == 0)      return _sir_lshr(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "tdiv") == 0)     return _sir_itdiv(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "tmod") == 0)     return _sir_itmod(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "utdiv") == 0)    return _sir_utdiv(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "utmod") == 0)    return _sir_utmod(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "cons") == 0)     return _sir_cons(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "car") == 0)      return _sir_car(_sir_arg(args, argc, 0));
    if (strcmp(name, "cdr") == 0)      return _sir_cdr(_sir_arg(args, argc, 0));
    if (strcmp(name, "null?") == 0)    return _sir_is_null(_sir_arg(args, argc, 0));
    if (strcmp(name, "pair?") == 0)    return _sir_is_pair(_sir_arg(args, argc, 0));
    if (strcmp(name, "number?") == 0)  return _sir_is_number(_sir_arg(args, argc, 0));
    if (strcmp(name, "symbol?") == 0)  return _sir_is_symbol(_sir_arg(args, argc, 0));
    if (strcmp(name, "print") == 0)    return _sir_print_v(args, argc);
    if (strcmp(name, "puts") == 0)     return _sir_puts_v(args, argc);
    fprintf(stderr, "sir: undefined builtin '%s'\n", name);
    exit(1);
}
SirValue _sir_builtin_closure(const char *name) {
    return _sir_make_closure(_sir_builtin_dispatch, 1, _sir_sym(name));
}

/* A builtin call the emitter does not lower directly fails loudly rather than
 * silently mis-compiling (no v0 program should reach this). */
SirValue _sir_unknown_builtin(const char *name) {
    fprintf(stderr, "sir: unsupported builtin '%s'\n", name);
    exit(1);
}
"####;
