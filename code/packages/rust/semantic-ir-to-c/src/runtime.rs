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

/* Collections slice 1: String transforms.  Each returns a FRESH arena buffer
 * (leak-on-exit, like every heap value) so the source string is never mutated.
 * Case mapping is ASCII-only — no `<ctype.h>` locale surprises, matching the
 * conformance corpus. */
char *_sir_str_upcase(const char *s) {
    size_t n = strlen(s), i;
    char *p = (char *)_sir_alloc(n + 1);
    for (i = 0; i < n; i++) { char c = s[i]; p[i] = (c >= 'a' && c <= 'z') ? (char)(c - 32) : c; }
    p[n] = '\0';
    return p;
}
char *_sir_str_downcase(const char *s) {
    size_t n = strlen(s), i;
    char *p = (char *)_sir_alloc(n + 1);
    for (i = 0; i < n; i++) { char c = s[i]; p[i] = (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c; }
    p[n] = '\0';
    return p;
}
char *_sir_str_reverse(const char *s) {
    size_t n = strlen(s), i;
    char *p = (char *)_sir_alloc(n + 1);
    for (i = 0; i < n; i++) p[i] = s[n - 1 - i];
    p[n] = '\0';
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

/* SIR27 milestone 9 numeric conversions (the `to_f`/`to_i` builtins the C
 * frontend inserts at int<->double boundaries).  `_sir_to_f` widens any numeric
 * value to a `double` (C's implicit int->double promotion).  `_sir_to_i`
 * truncates a `double` toward zero, exactly like C's `(int)double` cast — the
 * frontend then narrows the int64 to the target width with a `Convert`
 * (`_sir_iN`/`_sir_uN`), so the two together reproduce a C float->integer cast
 * bit-for-bit for values that fit the destination. */
SirValue _sir_to_f(SirValue v) { return _sir_float(_sir_as_num(v)); }
SirValue _sir_to_i(SirValue v) { return _sir_int(_sir_as_int(v)); }

/* SIR27 milestone 10 — faithful printf float formatting (the `fmt_float`
 * builtin).  Render `v` as C's printf would for conversion `kind`
 * ('f'/'F'/'e'/'E'/'g'/'G') and `prec` digits of precision.  The format string
 * is chosen by a switch over the fixed `kind` character — it is NEVER built
 * from source text — so there is no format-string vulnerability.  The output
 * is measured first (snprintf with a NULL buffer), then arena-allocated to the
 * exact size, so any precision fits without truncation. */
SirValue _sir_fmt_float_c(SirValue v, SirValue prec, SirValue kind) {
    double  x = _sir_as_num(v);
    int     p = (int)_sir_as_int(prec);
    char    k = (kind.tag == SIR_STR && kind.as.s[0]) ? kind.as.s[0] : 'f';
    const char *fmt;
    int need;
    char *buf;
    if (p < 0) p = 0;
    switch (k) {
        case 'F': fmt = "%.*F"; break;
        case 'e': fmt = "%.*e"; break;
        case 'E': fmt = "%.*E"; break;
        case 'g': fmt = "%.*g"; break;
        case 'G': fmt = "%.*G"; break;
        case 'f':
        default:  fmt = "%.*f"; break;
    }
    need = snprintf(NULL, 0, fmt, p, x);
    if (need < 0) return _sir_str("");
    buf = (char *)_sir_alloc((size_t)need + 1);
    snprintf(buf, (size_t)need + 1, fmt, p, x);
    return _sir_str(buf);
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
 * beyond any real (non-cyclic) nesting, yet sized to fit the SMALLEST common
 * C stack — Windows' 1 MB default (Linux/macOS give 8 MB). Each recursion level
 * is one `_sir_value_eq_d` frame, so ~500 levels stays well under 1 MB even in
 * an unoptimised build. (A cap of 5000 recursed ~875 KB deep on the display path
 * and stack-overflowed on Windows before the cap ever tripped.) */
#define SIR_MAX_EQ_DEPTH 500

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

static SirValue _sir_map_wrap(SirMap *m) {
    SirValue v; v.tag = SIR_MAP; v.as.map = m;
    return v;
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
 * single-threaded, so a plain static counter is sufficient). Sized like
 * `SIR_MAX_EQ_DEPTH` to fit Windows' 1 MB stack: a `_sir_fmt`↔`_sir_fmt_map`
 * pair is ~175 B per level, so ~500 levels is ~90 KB — far under the limit,
 * where 5000 overran the 1 MB stack before this cap could print the ellipsis. */
#define SIR_MAX_FMT_DEPTH 500
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

/* OOP slice 6: the CLASS whose method is currently executing — how a method body
 * resolves `@@x` (a class variable belongs to a class, not an instance).  Set by
 * `_sir_call_method` (to the receiver's class) and `_sir_call_class_method` (to
 * the dispatched class), restored after.  NULL at top level. */
static const char *_sir_current_class = NULL;

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
/* OOP slice 7: module mixins.  A module's methods are registered exactly like a
 * class's (via `__def_method__`, keyed on the module NAME), so a mixin needs no
 * new method storage — only a record of WHICH modules a class mixes in:
 *   - `include M` → M's INSTANCE methods join the class's instance-method lookup;
 *   - `extend  M` → M's instance methods become the class's CLASS methods.
 * Two `(class, module)` tables capture that; the resolvers consult them. */
#define SIR_MIXIN_MAX 4096
static struct { const char *cls; const char *module; } _sir_include_tab[SIR_MIXIN_MAX];
static int _sir_include_n = 0;
static struct { const char *cls; const char *module; } _sir_extend_tab[SIR_MIXIN_MAX];
static int _sir_extend_n = 0;

void _sir_register_include(const char *cls, const char *module) {
    if (_sir_include_n < SIR_MIXIN_MAX) {
        _sir_include_tab[_sir_include_n].cls = _sir_intern(cls);
        _sir_include_tab[_sir_include_n].module = _sir_intern(module);
        _sir_include_n++;
    }
}

void _sir_register_extend(const char *cls, const char *module) {
    if (_sir_extend_n < SIR_MIXIN_MAX) {
        _sir_extend_tab[_sir_extend_n].cls = _sir_intern(cls);
        _sir_extend_tab[_sir_extend_n].module = _sir_intern(module);
        _sir_extend_n++;
    }
}

/* Resolve `method` for an INSTANCE of `cls`: walk the ancestry, and at each class
 * check the class's own methods THEN its included modules' methods (most-recently
 * included first, matching Ruby's precedence).  Bounded by SIR_ANCESTRY_MAX. */
static SirValue _sir_resolve_method(const char *cls, const char *method) {
    const char *cur = cls;
    int steps = 0;
    while (cur && steps++ < SIR_ANCESTRY_MAX) {
        const char *c = _sir_intern(cur);
        int i;
        SirValue fn = _sir_lookup_method(cur, method);
        if (fn.tag == SIR_CLOSURE) return fn;
        for (i = _sir_include_n - 1; i >= 0; i--) {
            if (_sir_include_tab[i].cls == c) {
                fn = _sir_lookup_method(_sir_include_tab[i].module, method);
                if (fn.tag == SIR_CLOSURE) return fn;
            }
        }
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
        const char *saved_class = _sir_current_class;
        _sir_current_self = recv;
        _sir_current_class = recv.as.inst->sir_class;  /* slice 6: `@@x` owner */
        r = fn.as.clo->fn(fn.as.clo->caps, args, argc);
        _sir_current_self = saved_self;
        _sir_current_class = saved_class;
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

/* ---- OOP slice 5: class (singleton) methods --------------------------------
 *
 * A class method (`def self.m`) belongs to the class's OWN namespace — Ruby's
 * singleton class — NOT the instance-method table.  Keeping a SEPARATE table
 * means an instance method `m` and a class method `m` on the same class never
 * collide (both are legal, and distinct, in Ruby).  Class methods ARE inherited
 * (`class A; def self.m; end; end; class B < A; end; B.m` works), so dispatch
 * walks the SAME user-ancestry (`_sir_class_super`) as instance methods. */
static struct { const char *cls; const char *method; SirValue fn; } _sir_class_method_tab[SIR_METHOD_MAX];
static int _sir_class_method_n = 0;

SirValue _sir_def_class_method(const char *cls, const char *method, SirValue fn) {
    const char *c = _sir_intern(cls), *m = _sir_intern(method);
    int i;
    for (i = 0; i < _sir_class_method_n; i++) {
        if (_sir_class_method_tab[i].cls == c && _sir_class_method_tab[i].method == m) {
            _sir_class_method_tab[i].fn = fn;
            return fn;
        }
    }
    if (_sir_class_method_n < SIR_METHOD_MAX) {
        _sir_class_method_tab[_sir_class_method_n].cls = c;
        _sir_class_method_tab[_sir_class_method_n].method = m;
        _sir_class_method_tab[_sir_class_method_n].fn = fn;
        _sir_class_method_n++;
    }
    return fn;
}

/* Look up a class method on THIS class only (no ancestry). */
static SirValue _sir_lookup_class_method(const char *cls, const char *method) {
    const char *c = _sir_intern(cls), *m = _sir_intern(method);
    int i;
    for (i = 0; i < _sir_class_method_n; i++) {
        if (_sir_class_method_tab[i].cls == c && _sir_class_method_tab[i].method == m) {
            return _sir_class_method_tab[i].fn;
        }
    }
    return _sir_nil();
}

/* Resolve a class method starting at `cls`, walking UP the ancestry (so a
 * subclass inherits its parent's class methods).  Bounded by SIR_ANCESTRY_MAX. */
static SirValue _sir_resolve_class_method(const char *cls, const char *method) {
    const char *cur = cls;
    int steps = 0;
    while (cur && steps++ < SIR_ANCESTRY_MAX) {
        const char *c = _sir_intern(cur);
        int i;
        SirValue fn = _sir_lookup_class_method(cur, method);
        if (fn.tag == SIR_CLOSURE) return fn;
        /* OOP slice 7: `extend M` makes M's INSTANCE methods (keyed on the module
         * name in the instance-method table) callable as this class's class
         * methods — so consult the extended modules, most-recent first. */
        for (i = _sir_extend_n - 1; i >= 0; i--) {
            if (_sir_extend_tab[i].cls == c) {
                fn = _sir_lookup_method(_sir_extend_tab[i].module, method);
                if (fn.tag == SIR_CLOSURE) return fn;
            }
        }
        cur = _sir_class_super(cur);
    }
    return _sir_nil();
}

/* Dispatch a class method `cls.method(args…)`.  A class method has NO instance
 * receiver, so `self` is bound to NIL for the body (and restored after) — it
 * never leaks the caller's instance `self` into a class method (e.g. when an
 * instance method calls `Foo.bar`).  Unresolved => a (rescuable) NoMethodError. */
SirValue _sir_call_class_method(const char *cls, const char *method, int argc, ...) {
    va_list ap;
    SirValue *args, fn, r;
    fn = _sir_resolve_class_method(cls, method);
    if (fn.tag != SIR_CLOSURE) {
        return _sir_raise(_sir_error(
            "NoMethodError", _sir_str(_sir_cat("undefined class method ", method))));
    }
    va_start(ap, argc);
    args = _sir_va_collect(argc, ap);
    va_end(ap);
    {
        SirValue saved_self = _sir_current_self;
        const char *saved_class = _sir_current_class;
        _sir_current_self = _sir_nil();
        _sir_current_class = _sir_intern(cls);  /* slice 6: `@@x` owner is the class */
        r = fn.as.clo->fn(fn.as.clo->caps, args, argc);
        _sir_current_self = saved_self;
        _sir_current_class = saved_class;
    }
    if (args) free(args);
    return r;
}

/* ---- Collections slice 3: 0-arg Array query/transform methods --------------
 *
 * Non-mutating queries/transforms over a `SIR_SEQ` receiver — each returns a
 * FRESH sequence (or scalar); the receiver's backing array is never touched
 * (unlike `SeqSet`).  Ordering/equality reuse `_sir_lt`/`_sir_gt`/`_sir_value_eq`
 * — the SAME comparators `<`/`>`/`==` use — so `sort`/`min`/`max`/`uniq` agree
 * with the rest of the runtime instead of inventing a second notion of
 * "less"/"equal". */

static SirValue _sir_seq_wrap(SirSeq *s) {
    SirValue v; v.tag = SIR_SEQ; v.as.seq = s;
    return v;
}

static SirValue _sir_array_reverse(SirSeq *s) {
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = s->len;
    r->items = (s->len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)s->len) : NULL;
    for (int64_t i = 0; i < s->len; i++) r->items[i] = s->items[s->len - 1 - i];
    return _sir_seq_wrap(r);
}

/* Insertion sort via `_sir_lt` — O(n^2), fine for the v0 collection sizes this
 * runtime targets (matches the rest of this backend's "correctness over
 * micro-perf" v0 stance; see the sir-bench findings). */
static SirValue _sir_array_sort(SirSeq *s) {
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = s->len;
    r->items = (s->len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)s->len) : NULL;
    for (int64_t i = 0; i < s->len; i++) r->items[i] = s->items[i];
    for (int64_t i = 1; i < r->len; i++) {
        SirValue key = r->items[i];
        int64_t j = i - 1;
        while (j >= 0 && _sir_truthy(_sir_lt(key, r->items[j]))) {
            r->items[j + 1] = r->items[j];
            j--;
        }
        r->items[j + 1] = key;
    }
    return _sir_seq_wrap(r);
}

static SirValue _sir_array_min(SirSeq *s) {
    if (s->len == 0) return _sir_nil();
    SirValue best = s->items[0];
    for (int64_t i = 1; i < s->len; i++) {
        if (_sir_truthy(_sir_lt(s->items[i], best))) best = s->items[i];
    }
    return best;
}
static SirValue _sir_array_max(SirSeq *s) {
    if (s->len == 0) return _sir_nil();
    SirValue best = s->items[0];
    for (int64_t i = 1; i < s->len; i++) {
        if (_sir_truthy(_sir_gt(s->items[i], best))) best = s->items[i];
    }
    return best;
}
/* Ruby `Array#sum` defaults the accumulator to integer 0; `_sir_plus_v`'s
 * existing int/float promotion (the same rule `+` uses) carries a float
 * element through, so `[1, 2.5].sum` promotes correctly. */
static SirValue _sir_array_sum(SirSeq *s) {
    SirValue acc = _sir_int(0);
    for (int64_t i = 0; i < s->len; i++) {
        SirValue pair[2];
        pair[0] = acc;
        pair[1] = s->items[i];
        acc = _sir_plus_v(pair, 2);
    }
    return acc;
}
/* `sum { |x| ... }` — Ruby's `Array#sum` accepts an optional block that
 * transforms each element before summing (`[1, 2].sum { |x| x * 2 }` == 6).
 * The 0-arg form above ignored `argc`/`args` entirely, so a block call
 * silently fell through to it and summed the RAW elements instead — the
 * same latent-shadowing shape the slice-3 `count` gap had (fixed in slice
 * 5) before this one was ever exercised with a block. Snapshots `len`/
 * `items` before the loop like every other block-taking helper here. */
static SirValue _sir_array_sum_by(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    SirValue acc = _sir_int(0);
    for (int64_t i = 0; i < n; i++) {
        SirValue pair[2];
        pair[0] = acc;
        pair[1] = _sir_apply(block, 1, items[i]);
        acc = _sir_plus_v(pair, 2);
    }
    return acc;
}
/* First-occurrence order preserved (matches Ruby); dedup via `_sir_value_eq`
 * (structural, so `[[1], [1]].uniq` collapses to one `[1]`) — O(n^2), same
 * trade-off as `sort` above. Over-allocates to `s->len` (a safe upper bound on
 * the unique count) rather than a second exact-size pass. */
static SirValue _sir_array_uniq(SirSeq *s) {
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->items = (s->len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)s->len) : NULL;
    int64_t k = 0;
    for (int64_t i = 0; i < s->len; i++) {
        int dup = 0;
        for (int64_t j = 0; j < k; j++) {
            if (_sir_value_eq(s->items[i], r->items[j])) { dup = 1; break; }
        }
        if (!dup) r->items[k++] = s->items[i];
    }
    r->len = k;
    return _sir_seq_wrap(r);
}
static SirValue _sir_array_compact(SirSeq *s) {
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->items = (s->len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)s->len) : NULL;
    int64_t k = 0;
    for (int64_t i = 0; i < s->len; i++) {
        if (s->items[i].tag != SIR_NIL) r->items[k++] = s->items[i];
    }
    r->len = k;
    return _sir_seq_wrap(r);
}

/* `flatten` is a recursive aggregate walk over a structure a `SeqSet` can make
 * self-referential (`a[0] = a`).  A DEPTH cap alone (`SIR_MAX_EQ_DEPTH`, as
 * `_sir_value_eq`/`_sir_fmt` use) is NOT enough here: those two only ever
 * recurse into a FIXED small arity (a pair's two sides, or one seq/map
 * paired element-for-element against another of the SAME length), so depth
 * bounds their total work too. `flatten` instead recurses into EVERY element
 * of EVERY nested array, so a self-referential array with two or more
 * elements that all point back to itself (`a=[1,2]; a[0]=a; a[1]=a`) fans out
 * ~branching^depth calls — with depth capped at 500 that is astronomically
 * more work than the cap was sized for, and the resulting count can overflow
 * `int64_t`, in turn under-allocating `_sir_array_flatten`'s output buffer
 * for the (enormous) number of writes `_sir_flatten_fill` actually performs.
 *
 * So `flatten` ALSO threads a total-work `budget`, decremented once per call
 * — leaf or container — BEFORE it looks at `depth`/`tag`. Once the budget
 * runs out, the current node is treated as opaque (same as a past-depth-cap
 * node), so no more calls are spawned from it. That bounds the TOTAL number
 * of calls across the whole traversal to `SIR_MAX_FLATTEN_NODES`, regardless
 * of fan-out — unlike the depth cap alone, which only bounds one AXIS of the
 * traversal. The count and fill passes decrement from the SAME starting
 * budget in the SAME traversal order, so they agree on exactly which nodes
 * are opaque and produce a matching element count and write count. */
#define SIR_MAX_FLATTEN_NODES 1000000
static int64_t _sir_flatten_count(SirValue v, int depth, int64_t *budget) {
    (*budget)--;
    if (*budget < 0 || v.tag != SIR_SEQ || depth > SIR_MAX_EQ_DEPTH) return 1;
    int64_t n = 0;
    for (int64_t i = 0; i < v.as.seq->len; i++) {
        n += _sir_flatten_count(v.as.seq->items[i], depth + 1, budget);
    }
    return n;
}
static void _sir_flatten_fill(SirValue v, int depth, int64_t *budget, SirValue *out, int64_t *idx) {
    (*budget)--;
    if (*budget < 0 || v.tag != SIR_SEQ || depth > SIR_MAX_EQ_DEPTH) { out[(*idx)++] = v; return; }
    for (int64_t i = 0; i < v.as.seq->len; i++) {
        _sir_flatten_fill(v.as.seq->items[i], depth + 1, budget, out, idx);
    }
}
static SirValue _sir_array_flatten(SirValue recv) {
    int64_t count_budget = SIR_MAX_FLATTEN_NODES;
    int64_t n = _sir_flatten_count(recv, 0, &count_budget);
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = n;
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    int64_t idx = 0;
    int64_t fill_budget = SIR_MAX_FLATTEN_NODES;
    _sir_flatten_fill(recv, 0, &fill_budget, r->items, &idx);
    return _sir_seq_wrap(r);
}

/* ---- Collections slice 5: Array block methods (closure-calling) -----------
 *
 * The first Array methods that take a trailing BLOCK argument: the Ruby
 * frontend appends a `MakeClosure` as the LAST `__method__` call arg for
 * `recv.meth { |x| ... }` (RB1 in `ruby-to-semantic-ir`), so it reaches this
 * runtime as an ordinary `SIR_CLOSURE` value. Each element-wise call goes
 * through the EXISTING `_sir_apply` — the same dispatcher a first-class
 * `Proc`/`sir_apply(f, ...)` call already uses — so a block is called
 * exactly like any other closure value; no new calling convention. */

/* SECURITY (retrofitted for slice 4's `push`/`pop`/`shift`, which made
 * `SirSeq.items`/`.len` MUTABLE after construction — see `_sir_array_push`/
 * `_sir_array_pop`/`_sir_array_shift` below): every helper here that invokes
 * a block MUST snapshot BOTH `s->len` AND `s->items` into locals (`n`/
 * `items`) ONCE, before its loop, and use ONLY those locals — never
 * `s->len`/`s->items` directly — for the output-buffer size and every
 * element read. Snapshotting `len` alone is NOT enough: `push` reallocates
 * its NEW buffer sized to the CURRENT (live) `s->len`, so if a block first
 * shrinks the receiver (`pop`/`shift`, in place, no reallocation) and THEN
 * pushes, the fresh buffer `push` allocates is sized to the SHRUNK length —
 * smaller than a `len` this helper already snapshotted before the block ran.
 * Continuing to read the LIVE `s->items[i]` for `i` up to the stale, larger
 * `n` would then run past that fresh (smaller) allocation — a heap
 * out-of-bounds read (caught by security review; a real, exploitable gap in
 * an earlier draft of this fix that only snapshotted `len`). Snapshotting
 * the ITEMS POINTER too closes it: `items[i]` for `i < n` always reads the
 * buffer that existed AT SNAPSHOT TIME, which `push`'s copy-then-append
 * preserves byte-for-byte at indices `0..old_len-1` — and since this arena
 * never frees, that original buffer stays validly allocated for the whole
 * loop regardless of what `s->items` is reassigned to afterward. Matches the
 * "iterate a snapshot" convention `_sir_seq_iter` already uses for
 * `ForEach`. */

static SirValue _sir_array_each(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    for (int64_t i = 0; i < n; i++) _sir_apply(block, 1, items[i]);
    return _sir_seq_wrap(s);  /* Array#each returns the receiver */
}
static SirValue _sir_array_map(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = n;
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    for (int64_t i = 0; i < n; i++) r->items[i] = _sir_apply(block, 1, items[i]);
    return _sir_seq_wrap(r);
}
/* Shared by `select` (keep_if_truthy=1) and `reject` (keep_if_truthy=0). */
static SirValue _sir_array_filter(SirSeq *s, SirValue block, int keep_if_truthy) {
    int64_t n = s->len;
    SirValue *items = s->items;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    int64_t k = 0;
    for (int64_t i = 0; i < n; i++) {
        int truthy = _sir_truthy(_sir_apply(block, 1, items[i]));
        if (truthy == keep_if_truthy) r->items[k++] = items[i];
    }
    r->len = k;
    return _sir_seq_wrap(r);
}
static SirValue _sir_array_any(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    for (int64_t i = 0; i < n; i++) {
        if (_sir_truthy(_sir_apply(block, 1, items[i]))) return _sir_bool(1);
    }
    return _sir_bool(0);
}
static SirValue _sir_array_all(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    for (int64_t i = 0; i < n; i++) {
        if (!_sir_truthy(_sir_apply(block, 1, items[i]))) return _sir_bool(0);
    }
    return _sir_bool(1);
}
static SirValue _sir_array_none(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    for (int64_t i = 0; i < n; i++) {
        if (_sir_truthy(_sir_apply(block, 1, items[i]))) return _sir_bool(0);
    }
    return _sir_bool(1);
}
/* Schwartzian transform: compute each element's sort key ONCE (a naive
 * `_sir_lt(block(a), block(b))` comparator would re-invoke the block O(n log n)
 * or worse per comparison), then insertion-sort (key, value) pairs together via
 * `_sir_lt` -- the SAME comparator plain `sort` uses -- and return the values in
 * the new order. */
static SirValue _sir_array_sort_by(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    SirValue *keys = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = n;
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    for (int64_t i = 0; i < n; i++) {
        keys[i] = _sir_apply(block, 1, items[i]);
        r->items[i] = items[i];
    }
    for (int64_t i = 1; i < n; i++) {
        SirValue key = keys[i], val = r->items[i];
        int64_t j = i - 1;
        while (j >= 0 && _sir_truthy(_sir_lt(key, keys[j]))) {
            keys[j + 1] = keys[j];
            r->items[j + 1] = r->items[j];
            j--;
        }
        keys[j + 1] = key;
        r->items[j + 1] = val;
    }
    return _sir_seq_wrap(r);
}
static SirValue _sir_array_each_with_index(SirSeq *s, SirValue block) {
    int64_t n = s->len;
    SirValue *items = s->items;
    for (int64_t i = 0; i < n; i++) _sir_apply(block, 2, items[i], _sir_int(i));
    return _sir_seq_wrap(s);
}
/* `reduce`/`inject`: `argc==1` is block-only (Ruby seeds the accumulator with
 * the FIRST element and folds from the second -- `[].reduce { }` => nil, no
 * element to seed with); `argc==2` is `(initial, block)` (an empty receiver
 * just returns `initial` untouched, matching Ruby). `args[argc-1]` is always
 * the block (the caller already checked it's a closure before calling in). */
static SirValue _sir_array_reduce(SirSeq *s, int argc, SirValue *args) {
    int64_t n = s->len;
    SirValue *items = s->items;
    SirValue block = args[argc - 1];
    SirValue acc;
    int64_t start;
    if (argc >= 2) {
        acc = args[argc - 2];
        start = 0;
    } else {
        if (n == 0) return _sir_nil();
        acc = items[0];
        start = 1;
    }
    for (int64_t i = start; i < n; i++) acc = _sir_apply(block, 2, acc, items[i]);
    return acc;
}

/* ---- Collections slice 4: Array mutation (push/<</pop/shift) + 1-arg
 * query methods -----------------------------------------------------------
 *
 * `push`/`<<` are the FIRST operations that grow a `SirSeq` after
 * construction. Each call reallocates a fresh buffer sized to the exact new
 * length (no spare capacity tracked) rather than adding a `cap` field to the
 * shared `SirSeq` struct — that would require auditing and updating every
 * existing `_sir_alloc(sizeof(SirSeq))` call site (uninitialized-`cap` risk
 * for any missed one) for an amortized-growth win this v0 runtime doesn't
 * need; O(n) per push matches the rest of this backend's "correctness over
 * micro-perf" stance (see `sort`'s insertion sort, `sort_by`'s Schwartzian
 * transform). `pop`/`shift` mutate `len` (and, for `shift`, shift elements
 * down) IN PLACE with no reallocation. All three mutate the EXISTING SirSeq
 * box, like `SeqSet` — every binding sharing this array sees the change. */
static void _sir_array_push_one(SirSeq *s, SirValue v) {
    SirValue *ni = (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)(s->len + 1));
    for (int64_t i = 0; i < s->len; i++) ni[i] = s->items[i];
    ni[s->len] = v;
    s->items = ni;
    s->len++;
}
static SirValue _sir_array_pop(SirSeq *s) {
    if (s->len == 0) return _sir_nil();
    SirValue v = s->items[s->len - 1];
    s->len--;
    return v;
}
/* SECURITY FIX (found while implementing Hash#delete, slice 7 — see that
 * function's own note): `shift` must NOT compact `s->items` IN PLACE. A
 * block-taking helper (slice 5) that snapshots `s->items` as a POINTER
 * before its loop is only safe against a LATER `push` because `push`
 * reallocates a fresh buffer and leaves the old one untouched; the snapshot
 * and the live array are then *different* memory. An in-place shift instead
 * mutates the SAME memory the snapshot POINTS INTO, so the outer helper's
 * "frozen" view silently corrupts too (elements shift under it, some read
 * twice, some never read) — this was a live bug in the very first version
 * of this function (merged in slice 4, before any helper called into it
 * from inside a block's iteration). Reallocating a fresh, smaller buffer —
 * exactly like `push` growing one — restores the safe invariant: any
 * pointer snapshotted before this call keeps pointing at unmodified memory. */
static SirValue _sir_array_shift(SirSeq *s) {
    if (s->len == 0) return _sir_nil();
    SirValue v = s->items[0];
    int64_t new_len = s->len - 1;
    SirValue *ni = (new_len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)new_len) : NULL;
    for (int64_t i = 0; i < new_len; i++) ni[i] = s->items[i + 1];
    s->items = ni;
    s->len = new_len;
    return v;
}
static SirValue _sir_array_include(SirSeq *s, SirValue needle) {
    int64_t n = s->len;
    for (int64_t i = 0; i < n; i++) {
        if (_sir_value_eq(s->items[i], needle)) return _sir_bool(1);
    }
    return _sir_bool(0);
}
static SirValue _sir_array_index(SirSeq *s, SirValue needle) {
    int64_t n = s->len;
    for (int64_t i = 0; i < n; i++) {
        if (_sir_value_eq(s->items[i], needle)) return _sir_int(i);
    }
    return _sir_nil();
}
/* `fetch(i)` — like `a[i]` but RAISES on an out-of-range index instead of
 * returning nil (matching Ruby's `Array#fetch`, and this backend's
 * `_sir_seq_set`, which already traps rather than silently no-ops). Supports
 * the same negative-from-end indexing `_sir_seq_index` does. */
static SirValue _sir_array_fetch(SirSeq *s, SirValue idx) {
    int64_t n = s->len;
    int64_t i = _sir_as_int(idx);
    if (i < 0) i += n;
    if (i < 0 || i >= n) {
        return _sir_raise(_sir_error("IndexError", _sir_str("index out of range")));
    }
    return s->items[i];
}
/* `values_at(i0, i1, ...)` — a fresh array of the elements at each given
 * index (each independently negative-from-end and nil-on-OOB, exactly like
 * `_sir_seq_index`, NOT `fetch`'s raising form — matching Ruby). */
static SirValue _sir_array_values_at(SirSeq *s, int argc, SirValue *args) {
    SirValue seq_val = _sir_seq_wrap(s);
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = argc;
    r->items = (argc > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)argc) : NULL;
    for (int i = 0; i < argc; i++) r->items[i] = _sir_seq_index(seq_val, args[i]);
    return _sir_seq_wrap(r);
}
/* `rotate(n = 1)` — a FRESH array with elements shifted left by `n`
 * (negative `n` rotates right), matching Ruby (never mutates the
 * receiver). `n` is reduced modulo the length first (Ruby allows any
 * magnitude); an empty array rotates to itself. */
static SirValue _sir_array_rotate(SirSeq *s, int64_t by) {
    int64_t n = s->len;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = n;
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    if (n > 0) {
        int64_t k = ((by % n) + n) % n;  /* normalise to [0, n) even for negative `by` */
        for (int64_t i = 0; i < n; i++) r->items[i] = s->items[(i + k) % n];
    }
    return _sir_seq_wrap(r);
}
/* `zip(other1, other2, ...)` — a fresh array of arrays, pairing `self[i]`
 * with each `otherN[i]`; a shorter `other` pads with nil past its own
 * length (matching Ruby). Non-Array `other` arguments are treated as
 * length-0 (every pairing position is nil), the same lenient-nil-on-OOB
 * convention `_sir_seq_index` already uses for a non-sequence. */
static SirValue _sir_array_zip(SirSeq *s, int argc, SirValue *args) {
    int64_t n = s->len;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = n;
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    for (int64_t i = 0; i < n; i++) {
        SirSeq *row = (SirSeq *)_sir_alloc(sizeof(SirSeq));
        row->len = argc + 1;
        row->items = (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)(argc + 1));
        row->items[0] = s->items[i];
        for (int a = 0; a < argc; a++) {
            SirValue other = args[a];
            row->items[a + 1] = (other.tag == SIR_SEQ && i < other.as.seq->len)
                ? other.as.seq->items[i]
                : _sir_nil();
        }
        r->items[i] = _sir_seq_wrap(row);
    }
    return _sir_seq_wrap(r);
}

/* ---- Collections slice 6: Hash non-block methods ---------------------------
 *
 * `keys`/`values`/`to_a` walk `m->entries` in INSERTION order (the same
 * order `_sir_fmt_map`/iteration already use), so they agree with how a map
 * prints. None of these take a block (that's slice 7), so — unlike the
 * Array block helpers — there is no closure call mid-loop that could mutate
 * the receiver; no snapshot retrofit is needed here. */

static SirValue _sir_hash_keys(SirMap *m) {
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = m->len;
    r->items = (m->len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)m->len) : NULL;
    for (int64_t i = 0; i < m->len; i++) r->items[i] = m->entries[i].key;
    return _sir_seq_wrap(r);
}
static SirValue _sir_hash_values(SirMap *m) {
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = m->len;
    r->items = (m->len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)m->len) : NULL;
    for (int64_t i = 0; i < m->len; i++) r->items[i] = m->entries[i].val;
    return _sir_seq_wrap(r);
}
/* `fetch(k)` — like `h[k]` but RAISES `KeyError` on a missing key instead of
 * returning nil (matching Ruby's `Hash#fetch`, and this backend's
 * `Array#fetch`, which raises `IndexError` the same way). */
static SirValue _sir_hash_fetch(SirMap *m, SirValue key) {
    int64_t at = _sir_map_find(m, key);
    if (at < 0) return _sir_raise(_sir_error("KeyError", _sir_str("key not found")));
    return m->entries[at].val;
}
static SirValue _sir_hash_to_a(SirMap *m) {
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = m->len;
    r->items = (m->len > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)m->len) : NULL;
    for (int64_t i = 0; i < m->len; i++) r->items[i] = _sir_seq_lit(2, m->entries[i].key, m->entries[i].val);
    return _sir_seq_wrap(r);
}
/* `dig(k0, k1, ...)` — looks up `k0`, then recurses INTO the result for each
 * remaining key if it is itself diggable (a Hash or an Array); anything else
 * (including running out of structure early) yields nil — the same lenient
 * OOB-is-nil convention `_sir_seq_index`/`_sir_map_get` already use, rather
 * than real Ruby's `TypeError` on a non-diggable intermediate. Polymorphic
 * over the STARTING receiver too, so it doubles as `Array#dig`. */
static SirValue _sir_dig(SirValue recv, int argc, SirValue *args) {
    SirValue cur = recv;
    for (int i = 0; i < argc; i++) {
        if (cur.tag == SIR_MAP) cur = _sir_map_get(cur, args[i]);
        else if (cur.tag == SIR_SEQ) cur = _sir_seq_index(cur, args[i]);
        else return _sir_nil();
    }
    return cur;
}
/* `merge(other)` — a FRESH map with `self`'s entries, then `other`'s entries
 * put on top (a shared key takes `other`'s value, matching Ruby's no-block
 * `Hash#merge`); a non-Hash `other` is ignored (lenient, mirroring
 * `Array#zip`'s treatment of a non-Array `other` elsewhere in this file). */
static SirValue _sir_hash_merge(SirMap *m, SirValue other) {
    int64_t other_len = (other.tag == SIR_MAP) ? other.as.map->len : 0;
    SirMap *r = _sir_map_new(m->len + other_len);
    for (int64_t i = 0; i < m->len; i++) _sir_map_put(r, m->entries[i].key, m->entries[i].val);
    for (int64_t i = 0; i < other_len; i++) {
        _sir_map_put(r, other.as.map->entries[i].key, other.as.map->entries[i].val);
    }
    return _sir_map_wrap(r);
}
/* `delete(k)` — the FIRST Hash method that mutates the receiver: removes the
 * entry and returns its value, or nil if `k` wasn't present.
 *
 * SECURITY: reallocates a fresh, one-smaller `entries` buffer rather than
 * compacting `m->entries` IN PLACE (`Array#shift`'s ORIGINAL shape, and a
 * real bug there — see that function's own note, fixed alongside this one).
 * A block-taking helper (slice 7, below) snapshots `m->entries` as a POINTER
 * before its loop; that snapshot is only safe against a mutator that
 * REALLOCATES (the mutator's new buffer and the snapshot's old one are then
 * different memory — the old one, still valid in this never-freeing arena,
 * keeps reading exactly what it saw at snapshot time). An in-place compact
 * instead mutates the SAME memory the snapshot points into, silently
 * corrupting an in-flight outer iteration (entries shift under it — some
 * read twice, some skipped). Reallocating avoids that entirely. */
static SirValue _sir_hash_delete(SirMap *m, SirValue key) {
    int64_t at = _sir_map_find(m, key);
    if (at < 0) return _sir_nil();
    SirValue v = m->entries[at].val;
    int64_t new_len = m->len - 1;
    struct SirMapEntry *ne =
        (new_len > 0) ? (struct SirMapEntry *)_sir_alloc(sizeof(struct SirMapEntry) * (size_t)new_len) : NULL;
    int64_t j = 0;
    for (int64_t i = 0; i < m->len; i++) {
        if (i == at) continue;
        ne[j++] = m->entries[i];
    }
    m->entries = ne;
    m->len = new_len;
    /* SECURITY: `cap` (unlike `SirSeq`, `SirMap` DOES track spare capacity
     * for `_sir_map_put`'s amortized growth) must stay in sync with the
     * buffer just allocated — it is now tightly sized to `new_len`, with NO
     * spare slots. Leaving `m->cap` at its old, larger value would desync
     * `_sir_map_put`'s `if (m->len == m->cap)` grow check: a later `put`
     * would see `len < cap`, skip growing, and write directly at
     * `entries[len]` — one past the end of THIS tightly-sized buffer, a
     * heap out-of-bounds write. (Caught by security review: reallocating
     * without updating `cap` is a real, ordinary-Ruby-reachable bug, not a
     * theoretical one — `h.delete(k); h[new_key] = v` triggers it.) */
    m->cap = new_len;
    return v;
}
/* `clear` — removes every entry IN PLACE (just resets `len`; the backing
 * array is never freed, matching `Array#pop`'s style) and returns the
 * (now-empty) receiver, matching Ruby. */
static SirValue _sir_hash_clear(SirMap *m) {
    m->len = 0;
    return _sir_map_wrap(m);
}
/* `invert` — a FRESH map with keys and values swapped; a later duplicate
 * (by resulting key, i.e. an earlier VALUE) overwrites the earlier one via
 * `_sir_map_put`, matching Ruby (last entry in insertion order wins). */
static SirValue _sir_hash_invert(SirMap *m) {
    SirMap *r = _sir_map_new(m->len);
    for (int64_t i = 0; i < m->len; i++) _sir_map_put(r, m->entries[i].val, m->entries[i].key);
    return _sir_map_wrap(r);
}

/* ---- Collections slice 7: Hash block methods -------------------------------
 *
 * SECURITY: exactly the discipline slice 4 established for Array (and had to
 * retrofit there, twice, after security review) — applied HERE FROM THE
 * START since slice 6's `delete`/`clear` mutators already exist by the time
 * these block-taking helpers are added. Every helper snapshots BOTH `m->len`
 * AND the `entries` pointer into locals ONCE, before its loop, and reads
 * only through those locals — never `m->len`/`m->entries` directly — so a
 * block that calls `delete`/`clear` on the SAME map it's iterating can't
 * read past a buffer `delete`'s in-place shift (or a future growing
 * mutator) has since invalidated. Each block is called with TWO arguments,
 * `(key, value)` — matching `Array#each_with_index`'s existing 2-arg
 * precedent — regardless of how many params the Ruby block declares (extra
 * args a 1-param block doesn't bind are simply unused, the same block-arity
 * flexibility Ruby itself has). */

static SirValue _sir_hash_each(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    for (int64_t i = 0; i < n; i++) _sir_apply(block, 2, entries[i].key, entries[i].val);
    return _sir_map_wrap(m);  /* Hash#each returns the receiver */
}
static SirValue _sir_hash_each_key(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    for (int64_t i = 0; i < n; i++) _sir_apply(block, 1, entries[i].key);
    return _sir_map_wrap(m);
}
static SirValue _sir_hash_each_value(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    for (int64_t i = 0; i < n; i++) _sir_apply(block, 1, entries[i].val);
    return _sir_map_wrap(m);
}
/* `map` returns an ARRAY of the block's results (matching Ruby's
 * `Enumerable#map` over a Hash — NOT a re-keyed Hash). */
static SirValue _sir_hash_map(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = n;
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    for (int64_t i = 0; i < n; i++) r->items[i] = _sir_apply(block, 2, entries[i].key, entries[i].val);
    return _sir_seq_wrap(r);
}
/* Shared by `select` (keep_if_truthy=1) / `reject` (keep_if_truthy=0) — both
 * return a FRESH HASH (unlike `Array#select`/`reject`, which return an
 * Array), matching Ruby's `Hash#select`/`Hash#reject`. */
static SirValue _sir_hash_filter(SirMap *m, SirValue block, int keep_if_truthy) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    SirMap *r = _sir_map_new(n);
    for (int64_t i = 0; i < n; i++) {
        int truthy = _sir_truthy(_sir_apply(block, 2, entries[i].key, entries[i].val));
        if (truthy == keep_if_truthy) _sir_map_put(r, entries[i].key, entries[i].val);
    }
    return _sir_map_wrap(r);
}
/* Schwartzian transform over `[k, v]` PAIRS (mirrors `Array#sort_by`).
 * Returns an ARRAY of `[k, v]` pairs sorted by the block's key — Ruby's
 * `Hash#sort_by` (via `Enumerable`) always returns an Array, not a re-sorted
 * Hash (Hash order isn't independently `<=>`-able). */
static SirValue _sir_hash_sort_by(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    SirValue *keys = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    SirValue *pairs = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    for (int64_t i = 0; i < n; i++) {
        keys[i] = _sir_apply(block, 2, entries[i].key, entries[i].val);
        pairs[i] = _sir_seq_lit(2, entries[i].key, entries[i].val);
    }
    for (int64_t i = 1; i < n; i++) {
        SirValue key = keys[i], val = pairs[i];
        int64_t j = i - 1;
        while (j >= 0 && _sir_truthy(_sir_lt(key, keys[j]))) {
            keys[j + 1] = keys[j];
            pairs[j + 1] = pairs[j];
            j--;
        }
        keys[j + 1] = key;
        pairs[j + 1] = val;
    }
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->len = n;
    r->items = pairs;
    return _sir_seq_wrap(r);
}
/* `group_by` — a FRESH Hash mapping each distinct block result to an ARRAY
 * of the `[k, v]` pairs that produced it, in first-encountered group order
 * (matching Ruby). A group's value array must GROW across multiple matching
 * entries (unlike a plain `_sir_map_put`, which overwrites), so an existing
 * group is appended to via `_sir_array_push_one` (the SAME growth helper
 * `Array#push` uses); a new group starts as a fresh 1-element array. */
static SirValue _sir_hash_group_by(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    SirMap *r = _sir_map_new(0);
    for (int64_t i = 0; i < n; i++) {
        SirValue key = _sir_apply(block, 2, entries[i].key, entries[i].val);
        SirValue pair = _sir_seq_lit(2, entries[i].key, entries[i].val);
        int64_t at = _sir_map_find(r, key);
        if (at >= 0) {
            _sir_array_push_one(r->entries[at].val.as.seq, pair);
        } else {
            _sir_map_put(r, key, _sir_seq_lit(1, pair));
        }
    }
    return _sir_map_wrap(r);
}
/* `partition` — `[matching_pairs, non_matching_pairs]`, each a fresh Array
 * of `[k, v]` pairs (mirrors `Enumerable#partition` over a Hash's pairs). */
static SirValue _sir_hash_partition(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    SirSeq *yes = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    SirSeq *no = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    yes->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    no->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)n) : NULL;
    int64_t ny = 0, nn = 0;
    for (int64_t i = 0; i < n; i++) {
        SirValue pair = _sir_seq_lit(2, entries[i].key, entries[i].val);
        if (_sir_truthy(_sir_apply(block, 2, entries[i].key, entries[i].val))) yes->items[ny++] = pair;
        else no->items[nn++] = pair;
    }
    yes->len = ny;
    no->len = nn;
    return _sir_seq_lit(2, _sir_seq_wrap(yes), _sir_seq_wrap(no));
}
/* `sum { |k, v| ... }` — sums the block's return value over every entry,
 * starting the accumulator at integer `0` (mirrors `Array#sum`'s default,
 * reusing the SAME `_sir_plus_v` int/float promotion `+` uses). */
static SirValue _sir_hash_sum(SirMap *m, SirValue block) {
    int64_t n = m->len;
    struct SirMapEntry *entries = m->entries;
    SirValue acc = _sir_int(0);
    for (int64_t i = 0; i < n; i++) {
        SirValue pair[2];
        pair[0] = acc;
        pair[1] = _sir_apply(block, 2, entries[i].key, entries[i].val);
        acc = _sir_plus_v(pair, 2);
    }
    return acc;
}

/* ---- Collections slice 8: remaining String methods --------------------------
 *
 * Semantics matched against the Python/TS `sir-runtime-oop` reference catalog
 * (the cross-backend golden source this cascade's runtimes agree against),
 * not always byte-for-byte true Ruby -- e.g. `split(sep)` keeps trailing
 * empty fields like Python's `str.split`, not Ruby's drop-trailing-empties
 * rule. No `<ctype.h>` locale dependency (matches slice 1's ASCII-only case
 * mapping): whitespace/case checks are hand-rolled ASCII tests. */

static int _sir_is_ascii_ws(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' || c == '\v';
}

static char *_sir_str_capitalize(const char *s) {
    size_t n = strlen(s), i;
    char *p = (char *)_sir_alloc(n + 1);
    for (i = 0; i < n; i++) {
        char c = s[i];
        if (i == 0) p[i] = (c >= 'a' && c <= 'z') ? (char)(c - 32) : c;
        else        p[i] = (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c;
    }
    p[n] = '\0';
    return p;
}

static char *_sir_str_swapcase(const char *s) {
    size_t n = strlen(s), i;
    char *p = (char *)_sir_alloc(n + 1);
    for (i = 0; i < n; i++) {
        char c = s[i];
        if (c >= 'a' && c <= 'z')      p[i] = (char)(c - 32);
        else if (c >= 'A' && c <= 'Z') p[i] = (char)(c + 32);
        else                           p[i] = c;
    }
    p[n] = '\0';
    return p;
}

/* `strip`/`lstrip`/`rstrip` -- trim ASCII whitespace from either/both ends. */
static char *_sir_str_strip_range(const char *s, int left, int right) {
    size_t n = strlen(s);
    size_t start = 0, end = n;
    if (left)  while (start < end && _sir_is_ascii_ws(s[start])) start++;
    if (right) while (end > start && _sir_is_ascii_ws(s[end - 1])) end--;
    {
        size_t len = end - start;
        char *p = (char *)_sir_alloc(len + 1);
        memcpy(p, s + start, len);
        p[len] = '\0';
        return p;
    }
}

/* `chomp([sep])` -- no-arg form removes ONE trailing "\r\n", else one
 * trailing "\n" or "\r"; the 1-arg form removes a trailing LITERAL `sep`
 * only when the string actually ends with it. `sep == NULL` selects the
 * no-arg form. */
static char *_sir_str_chomp(const char *s, const char *sep) {
    size_t n = strlen(s);
    size_t cut = n;
    if (sep) {
        size_t sl = strlen(sep);
        if (sl > 0 && n >= sl && strcmp(s + n - sl, sep) == 0) cut = n - sl;
    } else if (n >= 2 && s[n - 2] == '\r' && s[n - 1] == '\n') {
        cut = n - 2;
    } else if (n >= 1 && (s[n - 1] == '\n' || s[n - 1] == '\r')) {
        cut = n - 1;
    }
    {
        char *p = (char *)_sir_alloc(cut + 1);
        memcpy(p, s, cut);
        p[cut] = '\0';
        return p;
    }
}

/* UTF-8 lead-byte sequence length (1-4), so `chars`/`each_char` split by
 * CHARACTER rather than byte -- unlike `bytes` below. Falls back to 1 on a
 * malformed or truncated sequence so a hostile/invalid byte string still
 * terminates in O(n), never over-reading past the NUL. */
static int _sir_utf8_char_len(const char *s) {
    unsigned char c = (unsigned char)s[0];
    int n, i;
    if (c < 0x80)             n = 1;
    else if ((c & 0xE0) == 0xC0) n = 2;
    else if ((c & 0xF0) == 0xE0) n = 3;
    else if ((c & 0xF8) == 0xF0) n = 4;
    else                       n = 1;
    for (i = 1; i < n; i++) {
        if (s[i] == '\0' || ((unsigned char)s[i] & 0xC0) != 0x80) return 1;
    }
    return n;
}

/* `chars` -- allocates `strlen(s)` slots (the worst case: every byte is its
 * own 1-byte character) and fills the actual (smaller, for multi-byte input)
 * count. */
static SirValue _sir_str_chars(const char *s) {
    size_t n = strlen(s), i = 0;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * n) : NULL;
    int64_t k = 0;
    while (i < n) {
        int len = _sir_utf8_char_len(s + i);
        char *buf = (char *)_sir_alloc((size_t)len + 1);
        memcpy(buf, s + i, (size_t)len);
        buf[len] = '\0';
        r->items[k++] = _sir_str(buf);
        i += (size_t)len;
    }
    r->len = k;
    return _sir_seq_wrap(r);
}

/* `bytes` -- the RAW byte values (0-255) as an Array of Integers, matching
 * the Python/TS reference (`list(recv.encode("utf-8"))`). */
static SirValue _sir_str_bytes(const char *s) {
    size_t n = strlen(s), i;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * n) : NULL;
    for (i = 0; i < n; i++) r->items[i] = _sir_int((int64_t)(unsigned char)s[i]);
    r->len = (int64_t)n;
    return _sir_seq_wrap(r);
}

/* `each_char { |c| .. }` -- UTF-8-aware like `chars`; returns the (immutable,
 * so unaliased) receiver, matching `Array#each`'s return-the-receiver rule. */
static SirValue _sir_str_each_char(const char *s, SirValue block) {
    size_t n = strlen(s), i = 0;
    while (i < n) {
        int len = _sir_utf8_char_len(s + i);
        char *buf = (char *)_sir_alloc((size_t)len + 1);
        memcpy(buf, s + i, (size_t)len);
        buf[len] = '\0';
        _sir_apply(block, 1, _sir_str(buf));
        i += (size_t)len;
    }
    return _sir_str(s);
}

/* `split` -- no-arg splits on RUNS of ASCII whitespace, dropping leading and
 * trailing empty fields (awk-style, matching Python's `str.split()`); with a
 * String separator, splits on LITERAL occurrences of it, KEEPING empty
 * fields between consecutive separators (matching Python's `str.split(sep)`
 * -- the chosen cross-backend reference, not Ruby's drop-trailing-empties
 * rule; see the file-header note above). An empty separator returns a
 * single-element Array of the whole string -- a documented degenerate case
 * that sidesteps a zero-length-match infinite loop. */
static SirValue _sir_str_split_ws(const char *s) {
    size_t n = strlen(s), i = 0;
    SirSeq *r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->items = (n > 0) ? (SirValue *)_sir_alloc(sizeof(SirValue) * n) : NULL;
    int64_t k = 0;
    while (i < n) {
        size_t start, len;
        char *buf;
        while (i < n && _sir_is_ascii_ws(s[i])) i++;
        if (i >= n) break;
        start = i;
        while (i < n && !_sir_is_ascii_ws(s[i])) i++;
        len = i - start;
        buf = (char *)_sir_alloc(len + 1);
        memcpy(buf, s + start, len);
        buf[len] = '\0';
        r->items[k++] = _sir_str(buf);
    }
    r->len = k;
    return _sir_seq_wrap(r);
}
static SirValue _sir_str_split_sep(const char *s, const char *sep) {
    size_t sl = strlen(s), pl = strlen(sep);
    SirSeq *r;
    int64_t k = 0;
    const char *p;
    if (pl == 0) return _sir_seq_lit(1, _sir_str(_sir_dup(s)));
    r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    /* Worst case (`sep` is a single char, every char is a separator): sl+1
       fields -- a safe, tight upper bound since pl >= 1 here. */
    r->items = (SirValue *)_sir_alloc(sizeof(SirValue) * (sl + 1));
    p = s;
    for (;;) {
        const char *hit = strstr(p, sep);
        size_t len = hit ? (size_t)(hit - p) : strlen(p);
        char *buf = (char *)_sir_alloc(len + 1);
        memcpy(buf, p, len);
        buf[len] = '\0';
        r->items[k++] = _sir_str(buf);
        if (!hit) break;
        p = hit + pl;
    }
    r->len = k;
    return _sir_seq_wrap(r);
}

/* `sub`/`gsub` -- literal (non-regex) first-occurrence / all-occurrences
 * replacement, matching the Python/TS reference (`str.replace`, no
 * back-reference expansion). `max_repl < 0` means unlimited (bounded
 * naturally: at most `strlen(s)/strlen(pat)` occurrences exist). An EMPTY
 * pattern is treated as "no match" -- the original string comes back
 * unchanged -- rather than Python's convention of inserting `repl` between
 * every character: that would need special-cased forward-progress handling
 * to avoid a zero-length-match infinite scan, and this keeps the helper
 * provably terminating on any input without it. */
static char *_sir_str_replace_n(const char *s, const char *pat, const char *repl, int64_t max_repl) {
    size_t pl = strlen(pat), rl = strlen(repl);
    int64_t count = 0;
    const char *p;
    if (pl == 0) return _sir_dup(s);
    p = s;
    while (max_repl < 0 || count < max_repl) {
        const char *hit = strstr(p, pat);
        if (!hit) break;
        count++;
        p = hit + pl;
    }
    if (count == 0) return _sir_dup(s);
    {
        size_t sl = strlen(s);
        size_t out_len = sl - (size_t)count * pl + (size_t)count * rl;
        char *out = (char *)_sir_alloc(out_len + 1);
        char *w = out;
        const char *r = s;
        int64_t done = 0;
        while (done < count) {
            const char *hit = strstr(r, pat);
            size_t pre = (size_t)(hit - r);
            memcpy(w, r, pre); w += pre;
            memcpy(w, repl, rl); w += rl;
            r = hit + pl;
            done++;
        }
        {
            size_t rest = strlen(r);
            memcpy(w, r, rest); w += rest;
        }
        *w = '\0';
        return out;
    }
}

/* `to_i`/`to_f` -- parse a LEADING numeric prefix (optional whitespace, sign,
 * digits), matching Ruby's never-raise `String#to_i`/`#to_f`: an
 * unparseable string yields `0`/`0.0` rather than an exception. `strtoll`/
 * `strtod` already implement exactly this "longest valid prefix, ignore the
 * rest" scan; only the "no digits at all" case needs an explicit check. */
static int64_t _sir_str_to_i(const char *s) {
    char *end;
    long long v = strtoll(s, &end, 10);
    return (end == s) ? 0 : (int64_t)v;
}
static double _sir_str_to_f(const char *s) {
    char *end;
    double v = strtod(s, &end);
    return (end == s) ? 0.0 : v;
}

/* `tr(from, to)` -- Ruby character-translation: each char of `recv` present
 * in `from` is replaced by the char at the SAME position in `to`; a shorter
 * `to` repeats its LAST char for extra `from` positions; an EMPTY `to`
 * deletes matching chars; a `from` char repeated later wins (last mapping).
 * The char-RANGE (`"a-z"`) and NEGATION (`"^abc"`) forms are a documented
 * follow-up -- literal-set-only, same scope precedent as `sub`/`gsub`. */
static char *_sir_str_tr(const char *s, const char *from, const char *to) {
    size_t n = strlen(s), fl = strlen(from), tl = strlen(to), i;
    /* has[c]: 0 = passthrough, 1 = translate via table[c], 2 = delete. */
    unsigned char has[256];
    char table[256];
    char *out;
    size_t w = 0;
    memset(has, 0, sizeof(has));
    for (i = 0; i < fl; i++) {
        unsigned char c = (unsigned char)from[i];
        if (tl == 0) { has[c] = 2; continue; }
        table[c] = (i < tl) ? to[i] : to[tl - 1];
        has[c] = 1;
    }
    out = (char *)_sir_alloc(n + 1);
    for (i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        if (has[c] == 1)      out[w++] = table[c];
        else if (has[c] == 2) { /* delete: emit nothing */ }
        else                   out[w++] = (char)c;
    }
    out[w] = '\0';
    return out;
}

/* ---- Collections slice 9: Numeric methods ------------------------------------
 *
 * `Integer`/`Float` methods. Semantics matched against the Python/TS
 * `sir-runtime-oop` reference catalog, with one deliberate divergence: this
 * runtime's `SirValue` int is a fixed `int64_t` (never arbitrary precision),
 * so `digits` needs none of the reference's bignum-DoS bit-length cap — an
 * int64 magnitude produces at most 19 decimal digits, an already-bounded
 * output. `even?`/`odd?`/`pred` are gated to `SIR_INT` only (true Ruby: these
 * are `Integer`-only methods, unlike the reference's looser dynamic-typing
 * convention) — a `Float` receiver falls through to `NoMethodError`, matching
 * this backend's existing typed-dispatch discipline (e.g. `upcase` is
 * `SIR_STR`-only). `floor`/`ceil`/`round` take no `ndigits` argument in this
 * slice (the 0-arg form only); the multi-digit rounding form is a documented
 * follow-up, deferred for the same "keep the slice reviewable" reason slice
 * 8 deferred `ljust`/`rjust`/`center`. */

static SirValue _sir_num_floor(SirValue v) {
    if (v.tag == SIR_INT) return v;
    if (v.tag == SIR_FLOAT) return _sir_int((int64_t)floor(v.as.f));
    return v;
}
static SirValue _sir_num_ceil(SirValue v) {
    if (v.tag == SIR_INT) return v;
    if (v.tag == SIR_FLOAT) return _sir_int((int64_t)ceil(v.as.f));
    return v;
}
/* Round-half-AWAY-from-zero (Ruby's rule; unlike Python's banker's rounding,
 * already the convention this runtime's `sir-runtime-oop` reference uses). */
static SirValue _sir_num_round(SirValue v) {
    if (v.tag == SIR_INT) return v;
    if (v.tag == SIR_FLOAT) {
        double f = v.as.f;
        double r = (f >= 0.0) ? floor(f + 0.5) : ceil(f - 0.5);
        return _sir_int((int64_t)r);
    }
    return v;
}

/* `divmod(divisor)` -- `[quotient, remainder]`, quotient FLOORED (via the
 * existing `_sir_ifloordiv`, the SAME floor-division `/` uses), remainder
 * takes the DIVISOR's sign. A zero divisor raises a catchable
 * `ZeroDivisionError` (the class is already registered in the exception
 * hierarchy for `rescue`) rather than the raw `exit(1)` the primitive `/`
 * operator falls back to -- this is a NICER, more Ruby-faithful failure
 * mode, not a regression: nothing upstream of this new dispatch arm relied
 * on the exit-on-zero behavior. */
static SirValue _sir_num_divmod(SirValue recv, SirValue divisor) {
    if (recv.tag == SIR_INT && divisor.tag == SIR_INT) {
        int64_t a = recv.as.i, b = divisor.as.i;
        int64_t q, r;
        if (b == 0) {
            return _sir_raise(_sir_error("ZeroDivisionError", _sir_str("divided by 0")));
        }
        q = _sir_ifloordiv(a, b);
        r = a - q * b;
        return _sir_seq_lit(2, _sir_int(q), _sir_int(r));
    }
    {
        double a = _sir_as_num(recv), b = _sir_as_num(divisor);
        double q, r;
        if (b == 0.0) {
            return _sir_raise(_sir_error("ZeroDivisionError", _sir_str("divided by 0")));
        }
        q = floor(a / b);
        r = a - q * b;
        return _sir_seq_lit(2, _sir_float(q), _sir_float(r));
    }
}

/* `fdiv(other)` -- floating-point division; UNLIKE `/`/`divmod` this never
 * raises: a zero divisor yields Infinity/-Infinity/NaN (Ruby's Float
 * division never raises), matching the reference's never-raise-on-Float
 * floor. */
static SirValue _sir_num_fdiv(SirValue recv, SirValue divisor) {
    double a = _sir_as_num(recv), b = _sir_as_num(divisor);
    if (b == 0.0) {
        if (a == 0.0) return _sir_float(0.0 / 0.0); /* NaN */
        return _sir_float(a > 0.0 ? (1.0 / 0.0) : (-1.0 / 0.0)); /* +-Infinity */
    }
    return _sir_float(a / b);
}

static SirValue _sir_num_gcd(SirValue recv, SirValue other) {
    int64_t a = _sir_as_int(recv), b = _sir_as_int(other);
    if (a < 0) a = -a;
    if (b < 0) b = -b;
    while (b != 0) { int64_t t = b; b = a % b; a = t; }
    return _sir_int(a);
}

/* `digits` -- base-10 digits, LEAST-significant first (`123.digits ==
 * [3, 2, 1]`). Bounded by construction: an `int64_t` magnitude has at most
 * 19 decimal digits, so no separate DoS cap is needed (unlike the Python
 * reference's arbitrary-precision receiver). `0.digits == [0]`. */
static SirValue _sir_num_digits(SirValue recv) {
    int64_t n = _sir_as_int(recv);
    SirValue buf[20];
    int64_t k = 0;
    SirSeq *r;
    if (n < 0) n = -n;
    if (n == 0) buf[k++] = _sir_int(0);
    while (n > 0) { buf[k++] = _sir_int(n % 10); n /= 10; }
    r = (SirSeq *)_sir_alloc(sizeof(SirSeq));
    r->items = (SirValue *)_sir_alloc(sizeof(SirValue) * (size_t)k);
    memcpy(r->items, buf, sizeof(SirValue) * (size_t)k);
    r->len = k;
    return _sir_seq_wrap(r);
}

/* `times { |i| .. }` / `upto(n) { |i| .. }` / `downto(n) { |i| .. }` --
 * Integer-only block iteration; each returns the (unchanged) receiver,
 * matching `Array#each`'s return-the-receiver convention. Bounded by the
 * receiver/argument's own int64 magnitude -- no separate iteration cap
 * needed (a hostile huge count just runs a long time, the same cost profile
 * `ForRange`/`Array#each` already have for a huge input). */
static SirValue _sir_num_times(SirValue recv, SirValue block) {
    int64_t n = _sir_as_int(recv);
    for (int64_t i = 0; i < n; i++) _sir_apply(block, 1, _sir_int(i));
    return recv;
}
static SirValue _sir_num_upto(SirValue recv, SirValue limit, SirValue block) {
    int64_t i = _sir_as_int(recv), n = _sir_as_int(limit);
    for (; i <= n; i++) _sir_apply(block, 1, _sir_int(i));
    return recv;
}
static SirValue _sir_num_downto(SirValue recv, SirValue limit, SirValue block) {
    int64_t i = _sir_as_int(recv), n = _sir_as_int(limit);
    for (; i >= n; i--) _sir_apply(block, 1, _sir_int(i));
    return recv;
}
/* `step(limit, stride = 1) { |i| .. }` -- a stride of 0 would loop forever
 * (never crosses `limit`), so it is a documented no-op (zero iterations)
 * rather than a hang, the same DoS-safety floor `sub`/`gsub`'s empty-pattern
 * guard holds for string scanning. Promotes to float stepping if EITHER the
 * receiver or the stride is a Float (matches Ruby: `1.step(2, 0.5)` yields
 * floats), otherwise stays in exact int64 arithmetic. */
static SirValue _sir_num_step(SirValue recv, SirValue limit, SirValue stride, SirValue block) {
    int use_float = (recv.tag == SIR_FLOAT || stride.tag == SIR_FLOAT || limit.tag == SIR_FLOAT);
    if (use_float) {
        double v = _sir_as_num(recv), lim = _sir_as_num(limit), st = _sir_as_num(stride);
        if (st == 0.0) return recv;
        if (st > 0.0) { for (; v <= lim; v += st) _sir_apply(block, 1, _sir_float(v)); }
        else          { for (; v >= lim; v += st) _sir_apply(block, 1, _sir_float(v)); }
    } else {
        int64_t v = _sir_as_int(recv), lim = _sir_as_int(limit), st = _sir_as_int(stride);
        if (st == 0) return recv;
        if (st > 0) { for (; v <= lim; v += st) _sir_apply(block, 1, _sir_int(v)); }
        else        { for (; v >= lim; v += st) _sir_apply(block, 1, _sir_int(v)); }
    }
    return recv;
}

/* ---- Collections slice 1: built-in String methods --------------------------
 *
 * A `__method__` dispatch whose name is a KNOWN built-in method — and which the
 * module did NOT define as a user method — routes here instead of the user
 * method table.  Dispatch is an explicit `strcmp` switch on the method name plus
 * a receiver-type check (Ruby's built-in methods are polymorphic: `length` works
 * on a String, Array, or Hash), so a wrong-type receiver raises `NoMethodError`
 * exactly as Ruby would — never a crash.  The method name is a compiler-emitted
 * quoted literal, so this is not reflection (anti-RCE holds).  Slice 1 covers
 * common 0-arity methods; the `argc`/varargs are carried for later arg-taking
 * methods. */
/* The switch, over the already-collected `args` (so an arg-taking method reads
 * `args[0]` after guarding `argc >= 1` and its type).  `_sir_builtin_method`
 * wraps this to collect/free the varargs, exactly like `_sir_plus`. */
static SirValue _sir_builtin_method_v(SirValue recv, const char *m, int argc, SirValue *args) {
    if (strcmp(m, "length") == 0 || strcmp(m, "size") == 0) {
        if (recv.tag == SIR_STR) return _sir_int((int64_t)strlen(recv.as.s));
        if (recv.tag == SIR_SEQ) return _sir_int(recv.as.seq->len);
        if (recv.tag == SIR_MAP) return _sir_int(recv.as.map->len);
    } else if (strcmp(m, "upcase") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_upcase(recv.as.s));
    } else if (strcmp(m, "downcase") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_downcase(recv.as.s));
    } else if (strcmp(m, "reverse") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_reverse(recv.as.s));
        if (recv.tag == SIR_SEQ) return _sir_array_reverse(recv.as.seq);
    } else if (strcmp(m, "empty?") == 0) {
        if (recv.tag == SIR_STR) return _sir_bool(recv.as.s[0] == '\0');
        if (recv.tag == SIR_SEQ) return _sir_bool(recv.as.seq->len == 0);
        if (recv.tag == SIR_MAP) return _sir_bool(recv.as.map->len == 0);
    } else if (strcmp(m, "to_s") == 0) {
        if (recv.tag == SIR_STR) return recv;  /* String#to_s is the string itself */
    }
    /* Collections slice 3: 0-arg Array query/transform methods. */
    else if (strcmp(m, "count") == 0) {
        /* Slice 5 fix: `count` also has a BLOCK form (`arr.count { |x| .. }`,
           counting only matching elements) -- the slice-3 arm below ignored
           `argc`/`args` entirely, so it silently returned the total length
           for a block call too (wrong, not just unsupported). Guard on argc
           so the two forms route correctly instead of one shadowing. */
        if (recv.tag == SIR_SEQ) {
            if (argc == 0) return _sir_int(recv.as.seq->len);
            if (argc == 1 && args[0].tag == SIR_CLOSURE) {
                /* SECURITY (slice 4): snapshot len/items BEFORE the loop, like
                   every other block-taking helper -- see the doc comment on
                   the slice-5 helpers above. `push` (slice 4) can grow this
                   same receiver from inside the block; a live `recv.as.seq->
                   len` read in the loop condition would never terminate. */
                int64_t cnt_len = recv.as.seq->len;
                SirValue *cnt_items = recv.as.seq->items;
                int64_t n = 0;
                for (int64_t i = 0; i < cnt_len; i++) {
                    if (_sir_truthy(_sir_apply(args[0], 1, cnt_items[i]))) n++;
                }
                return _sir_int(n);
            }
        }
        if (recv.tag == SIR_MAP && argc == 0) return _sir_int(recv.as.map->len);
    } else if (strcmp(m, "first") == 0) {
        if (recv.tag == SIR_SEQ) return recv.as.seq->len > 0 ? recv.as.seq->items[0] : _sir_nil();
    } else if (strcmp(m, "last") == 0) {
        if (recv.tag == SIR_SEQ) {
            int64_t n = recv.as.seq->len;
            return n > 0 ? recv.as.seq->items[n - 1] : _sir_nil();
        }
    } else if (strcmp(m, "sort") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_sort(recv.as.seq);
    } else if (strcmp(m, "min") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_min(recv.as.seq);
    } else if (strcmp(m, "max") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_max(recv.as.seq);
    } else if (strcmp(m, "sum") == 0) {
        if (recv.tag == SIR_SEQ) {
            if (argc == 0) return _sir_array_sum(recv.as.seq);
            if (argc == 1 && args[0].tag == SIR_CLOSURE) return _sir_array_sum_by(recv.as.seq, args[0]);
        }
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_sum(recv.as.map, args[0]);
    } else if (strcmp(m, "uniq") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_uniq(recv.as.seq);
    } else if (strcmp(m, "compact") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_compact(recv.as.seq);
    } else if (strcmp(m, "flatten") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_flatten(recv);
    } else if (strcmp(m, "to_a") == 0) {
        if (recv.tag == SIR_SEQ) return recv;  /* Array#to_a is the array itself */
        if (recv.tag == SIR_MAP) return _sir_hash_to_a(recv.as.map);
    }
    /* Collections slice 5: Array block methods. Each requires exactly the
       trailing-block shape the frontend emits (`argc==1`, a closure) except
       `reduce`/`inject` (`argc` 1 or 2) -- anything else (missing/extra args,
       a non-closure last arg) falls through to the NoMethodError below rather
       than misbehaving on a malformed call. */
    else if (strcmp(m, "each") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_each(recv.as.seq, args[0]);
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_each(recv.as.map, args[0]);
    } else if (strcmp(m, "map") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_map(recv.as.seq, args[0]);
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_map(recv.as.map, args[0]);
    } else if (strcmp(m, "select") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_filter(recv.as.seq, args[0], 1);
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_filter(recv.as.map, args[0], 1);
    } else if (strcmp(m, "reject") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_filter(recv.as.seq, args[0], 0);
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_filter(recv.as.map, args[0], 0);
    } else if (strcmp(m, "any?") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_any(recv.as.seq, args[0]);
    } else if (strcmp(m, "all?") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_all(recv.as.seq, args[0]);
    } else if (strcmp(m, "none?") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_none(recv.as.seq, args[0]);
    } else if (strcmp(m, "sort_by") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_sort_by(recv.as.seq, args[0]);
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_sort_by(recv.as.map, args[0]);
    } else if (strcmp(m, "each_with_index") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_array_each_with_index(recv.as.seq, args[0]);
    } else if (strcmp(m, "reduce") == 0 || strcmp(m, "inject") == 0) {
        if (recv.tag == SIR_SEQ && argc >= 1 && argc <= 2 && args[argc - 1].tag == SIR_CLOSURE)
            return _sir_array_reduce(recv.as.seq, argc, args);
    }
    /* Collections slice 7: Hash block methods. */
    else if (strcmp(m, "each_key") == 0) {
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_each_key(recv.as.map, args[0]);
    } else if (strcmp(m, "each_value") == 0) {
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_each_value(recv.as.map, args[0]);
    } else if (strcmp(m, "group_by") == 0) {
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_group_by(recv.as.map, args[0]);
    } else if (strcmp(m, "partition") == 0) {
        if (recv.tag == SIR_MAP && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_hash_partition(recv.as.map, args[0]);
    }
    /* Collections slice 4: Array mutation + 1-arg query methods. */
    else if (strcmp(m, "push") == 0) {
        if (recv.tag == SIR_SEQ) {
            for (int i = 0; i < argc; i++) _sir_array_push_one(recv.as.seq, args[i]);
            return recv;  /* Array#push returns the (mutated) receiver */
        }
    } else if (strcmp(m, "pop") == 0) {
        if (recv.tag == SIR_SEQ && argc == 0) return _sir_array_pop(recv.as.seq);
    } else if (strcmp(m, "shift") == 0) {
        if (recv.tag == SIR_SEQ && argc == 0) return _sir_array_shift(recv.as.seq);
    } else if (strcmp(m, "fetch") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1) return _sir_array_fetch(recv.as.seq, args[0]);
        if (recv.tag == SIR_MAP && argc == 1) return _sir_hash_fetch(recv.as.map, args[0]);
    } else if (strcmp(m, "values_at") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_values_at(recv.as.seq, argc, args);
    } else if (strcmp(m, "rotate") == 0) {
        if (recv.tag == SIR_SEQ) {
            int64_t by = (argc >= 1) ? _sir_as_int(args[0]) : 1;
            return _sir_array_rotate(recv.as.seq, by);
        }
    } else if (strcmp(m, "zip") == 0) {
        if (recv.tag == SIR_SEQ) return _sir_array_zip(recv.as.seq, argc, args);
    }
    /* Collections slice 6: Hash non-block methods. */
    else if (strcmp(m, "keys") == 0) {
        if (recv.tag == SIR_MAP) return _sir_hash_keys(recv.as.map);
    } else if (strcmp(m, "values") == 0) {
        if (recv.tag == SIR_MAP) return _sir_hash_values(recv.as.map);
    } else if (strcmp(m, "to_h") == 0) {
        if (recv.tag == SIR_MAP) return recv;  /* Hash#to_h is the hash itself */
    } else if (strcmp(m, "dig") == 0) {
        if ((recv.tag == SIR_MAP || recv.tag == SIR_SEQ) && argc >= 1)
            return _sir_dig(recv, argc, args);
    } else if (strcmp(m, "merge") == 0) {
        if (recv.tag == SIR_MAP && argc >= 1) return _sir_hash_merge(recv.as.map, args[0]);
    } else if (strcmp(m, "delete") == 0) {
        if (recv.tag == SIR_MAP && argc == 1) return _sir_hash_delete(recv.as.map, args[0]);
    } else if (strcmp(m, "clear") == 0) {
        if (recv.tag == SIR_MAP && argc == 0) return _sir_hash_clear(recv.as.map);
    } else if (strcmp(m, "invert") == 0) {
        if (recv.tag == SIR_MAP && argc == 0) return _sir_hash_invert(recv.as.map);
    }
    /* Bug fix — `recv[k]` / `recv[k] = v` (Ruby's `[]`/`[]=`, real method
       syntax: `recv.[](k)` / `recv.[]=(k, v)`). The frontend used to guess
       Array-vs-Hash from the INDEX's syntactic shape at compile time (a
       heuristic that mis-typed a real, common case: a Hash with a
       non-string key, e.g. `h[2] = "b"` on an int-keyed Hash, routed to
       Array's `_sir_seq_set` regardless of the receiver's actual type,
       which EXITS on a non-sequence). Routing through the SAME `__method__`
       dispatch every other built-in uses instead checks the RECEIVER's
       ACTUAL tag here, at runtime — genuinely polymorphic, so it can never
       mis-route regardless of the index's type. */
    else if (strcmp(m, "[]") == 0) {
        if (recv.tag == SIR_SEQ && argc == 1) return _sir_seq_index(recv, args[0]);
        if (recv.tag == SIR_MAP && argc == 1) return _sir_map_get(recv, args[0]);
    } else if (strcmp(m, "[]=") == 0) {
        if (recv.tag == SIR_SEQ && argc == 2) return _sir_seq_set(recv, args[0], args[1]);
        if (recv.tag == SIR_MAP && argc == 2) return _sir_map_set(recv, args[0], args[1]);
    }
    /* Collections slice 2: 1-arg String queries (arg is a String); slice 4
       widens `include?`/`index` to accept an Array receiver too. */
    else if (strcmp(m, "include?") == 0) {
        if (recv.tag == SIR_STR && argc >= 1 && args[0].tag == SIR_STR)
            return _sir_bool(strstr(recv.as.s, args[0].as.s) != NULL);
        if (recv.tag == SIR_SEQ && argc >= 1) return _sir_array_include(recv.as.seq, args[0]);
    } else if (strcmp(m, "start_with?") == 0) {
        if (recv.tag == SIR_STR && argc >= 1 && args[0].tag == SIR_STR) {
            size_t pl = strlen(args[0].as.s);
            return _sir_bool(strncmp(recv.as.s, args[0].as.s, pl) == 0);
        }
    } else if (strcmp(m, "end_with?") == 0) {
        if (recv.tag == SIR_STR && argc >= 1 && args[0].tag == SIR_STR) {
            size_t rl = strlen(recv.as.s), sl = strlen(args[0].as.s);
            return _sir_bool(rl >= sl && strcmp(recv.as.s + rl - sl, args[0].as.s) == 0);
        }
    } else if (strcmp(m, "index") == 0) {
        if (recv.tag == SIR_STR && argc >= 1 && args[0].tag == SIR_STR) {
            const char *p = strstr(recv.as.s, args[0].as.s);
            return p ? _sir_int((int64_t)(p - recv.as.s)) : _sir_nil();
        }
        if (recv.tag == SIR_SEQ && argc >= 1) return _sir_array_index(recv.as.seq, args[0]);
    }
    /* Collections slice 8: remaining String methods. */
    else if (strcmp(m, "capitalize") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_capitalize(recv.as.s));
    } else if (strcmp(m, "swapcase") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_swapcase(recv.as.s));
    } else if (strcmp(m, "strip") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_strip_range(recv.as.s, 1, 1));
    } else if (strcmp(m, "lstrip") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_strip_range(recv.as.s, 1, 0));
    } else if (strcmp(m, "rstrip") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_strip_range(recv.as.s, 0, 1));
    } else if (strcmp(m, "chomp") == 0) {
        if (recv.tag == SIR_STR) {
            const char *sep = (argc >= 1 && args[0].tag == SIR_STR) ? args[0].as.s : NULL;
            return _sir_str(_sir_str_chomp(recv.as.s, sep));
        }
    } else if (strcmp(m, "chars") == 0) {
        if (recv.tag == SIR_STR) return _sir_str_chars(recv.as.s);
    } else if (strcmp(m, "bytes") == 0) {
        if (recv.tag == SIR_STR) return _sir_str_bytes(recv.as.s);
    } else if (strcmp(m, "each_char") == 0) {
        if (recv.tag == SIR_STR && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_str_each_char(recv.as.s, args[0]);
    } else if (strcmp(m, "split") == 0) {
        if (recv.tag == SIR_STR) {
            if (argc == 0) return _sir_str_split_ws(recv.as.s);
            if (argc >= 1 && args[0].tag == SIR_STR) return _sir_str_split_sep(recv.as.s, args[0].as.s);
        }
    } else if (strcmp(m, "replace") == 0) {
        if (recv.tag == SIR_STR && argc == 1 && args[0].tag == SIR_STR) return args[0];
    } else if (strcmp(m, "sub") == 0) {
        if (recv.tag == SIR_STR && argc == 2 && args[0].tag == SIR_STR && args[1].tag == SIR_STR)
            return _sir_str(_sir_str_replace_n(recv.as.s, args[0].as.s, args[1].as.s, 1));
    } else if (strcmp(m, "gsub") == 0) {
        if (recv.tag == SIR_STR && argc == 2 && args[0].tag == SIR_STR && args[1].tag == SIR_STR)
            return _sir_str(_sir_str_replace_n(recv.as.s, args[0].as.s, args[1].as.s, -1));
    } else if (strcmp(m, "to_i") == 0) {
        if (recv.tag == SIR_STR) return _sir_int(_sir_str_to_i(recv.as.s));
        if (_sir_is_num(recv)) return _sir_to_i(recv);
    } else if (strcmp(m, "to_f") == 0) {
        if (recv.tag == SIR_STR) return _sir_float(_sir_str_to_f(recv.as.s));
        if (_sir_is_num(recv)) return _sir_to_f(recv);
    } else if (strcmp(m, "to_sym") == 0) {
        if (recv.tag == SIR_STR) return _sir_sym(recv.as.s);
    } else if (strcmp(m, "tr") == 0) {
        if (recv.tag == SIR_STR && argc == 2 && args[0].tag == SIR_STR && args[1].tag == SIR_STR)
            return _sir_str(_sir_str_tr(recv.as.s, args[0].as.s, args[1].as.s));
    }
    /* Collections slice 9: Numeric methods. */
    else if (strcmp(m, "abs") == 0) {
        if (recv.tag == SIR_INT) return _sir_int(recv.as.i < 0 ? -recv.as.i : recv.as.i);
        if (recv.tag == SIR_FLOAT) return _sir_float(fabs(recv.as.f));
    } else if (strcmp(m, "even?") == 0) {
        if (recv.tag == SIR_INT) return _sir_bool(recv.as.i % 2 == 0);
    } else if (strcmp(m, "odd?") == 0) {
        if (recv.tag == SIR_INT) return _sir_bool(recv.as.i % 2 != 0);
    } else if (strcmp(m, "zero?") == 0) {
        if (_sir_is_num(recv)) return _sir_bool(_sir_as_num(recv) == 0.0);
    } else if (strcmp(m, "positive?") == 0) {
        if (_sir_is_num(recv)) return _sir_bool(_sir_as_num(recv) > 0.0);
    } else if (strcmp(m, "negative?") == 0) {
        if (_sir_is_num(recv)) return _sir_bool(_sir_as_num(recv) < 0.0);
    } else if (strcmp(m, "pred") == 0) {
        if (recv.tag == SIR_INT) return _sir_int(recv.as.i - 1);
    } else if (strcmp(m, "floor") == 0) {
        if (_sir_is_num(recv)) return _sir_num_floor(recv);
    } else if (strcmp(m, "ceil") == 0) {
        if (_sir_is_num(recv)) return _sir_num_ceil(recv);
    } else if (strcmp(m, "round") == 0) {
        if (_sir_is_num(recv) && argc == 0) return _sir_num_round(recv);
    } else if (strcmp(m, "divmod") == 0) {
        if (_sir_is_num(recv) && argc == 1 && _sir_is_num(args[0]))
            return _sir_num_divmod(recv, args[0]);
    } else if (strcmp(m, "fdiv") == 0) {
        if (_sir_is_num(recv) && argc == 1 && _sir_is_num(args[0]))
            return _sir_num_fdiv(recv, args[0]);
    } else if (strcmp(m, "clamp") == 0) {
        if (_sir_is_num(recv) && argc == 2 && _sir_is_num(args[0]) && _sir_is_num(args[1])) {
            if (_sir_truthy(_sir_lt(recv, args[0]))) return args[0];
            if (_sir_truthy(_sir_gt(recv, args[1]))) return args[1];
            return recv;
        }
    } else if (strcmp(m, "between?") == 0) {
        if (_sir_is_num(recv) && argc == 2 && _sir_is_num(args[0]) && _sir_is_num(args[1]))
            return _sir_bool(_sir_truthy(_sir_ge(recv, args[0])) && _sir_truthy(_sir_le(recv, args[1])));
    } else if (strcmp(m, "gcd") == 0) {
        if (recv.tag == SIR_INT && argc == 1 && args[0].tag == SIR_INT)
            return _sir_num_gcd(recv, args[0]);
    } else if (strcmp(m, "digits") == 0) {
        if (recv.tag == SIR_INT) return _sir_num_digits(recv);
    } else if (strcmp(m, "times") == 0) {
        if (recv.tag == SIR_INT && argc == 1 && args[0].tag == SIR_CLOSURE)
            return _sir_num_times(recv, args[0]);
    } else if (strcmp(m, "upto") == 0) {
        if (recv.tag == SIR_INT && argc == 2 && args[0].tag == SIR_INT && args[1].tag == SIR_CLOSURE)
            return _sir_num_upto(recv, args[0], args[1]);
    } else if (strcmp(m, "downto") == 0) {
        if (recv.tag == SIR_INT && argc == 2 && args[0].tag == SIR_INT && args[1].tag == SIR_CLOSURE)
            return _sir_num_downto(recv, args[0], args[1]);
    } else if (strcmp(m, "step") == 0) {
        if (_sir_is_num(recv) && argc == 2 && _sir_is_num(args[0]) && args[1].tag == SIR_CLOSURE)
            return _sir_num_step(recv, args[0], _sir_int(1), args[1]);
        if (_sir_is_num(recv) && argc == 3 && _sir_is_num(args[0]) && _sir_is_num(args[1])
            && args[2].tag == SIR_CLOSURE)
            return _sir_num_step(recv, args[0], args[1], args[2]);
    }
    return _sir_raise(
        _sir_error("NoMethodError", _sir_str(_sir_cat("undefined method ", m))));
}

SirValue _sir_builtin_method(SirValue recv, const char *m, int argc, ...) {
    va_list ap;
    SirValue *args, r;
    va_start(ap, argc);
    args = _sir_va_collect(argc, ap);
    va_end(ap);
    r = _sir_builtin_method_v(recv, m, argc, args);
    if (args) free(args);
    return r;
}

/* ---- OOP slice 6: class variables (@@x) ------------------------------------
 *
 * A class variable belongs to a CLASS and is shared down its hierarchy: `@@x`
 * defined in a parent is the SAME storage in every subclass.  Storage is a flat
 * `(class, @@name) -> value` table.  A method body resolves its class from
 * `_sir_current_class` (set by dispatch); a class-body initializer (`@@x = 0`
 * inside `class C`) names its class explicitly via `_sir_cvar_set_in`. */
#define SIR_CVAR_MAX 4096
static struct { const char *cls; const char *var; SirValue val; } _sir_cvar_tab[SIR_CVAR_MAX];
static int _sir_cvar_n = 0;

/* The class that OWNS `@@var` for `start_class`: the nearest ancestor (incl.
 * `start_class`) that already stores it — so a subclass shares a parent's `@@x`.
 * If none does, `start_class` itself (a fresh write creates it there).  Bounded
 * by SIR_ANCESTRY_MAX (a cyclic hand-built hierarchy cannot hang). */
static const char *_sir_cvar_owner(const char *start_class, const char *var) {
    const char *v = _sir_intern(var);
    const char *cur = start_class;
    int steps = 0;
    while (cur && steps++ < SIR_ANCESTRY_MAX) {
        const char *c = _sir_intern(cur);
        int i;
        for (i = 0; i < _sir_cvar_n; i++) {
            if (_sir_cvar_tab[i].cls == c && _sir_cvar_tab[i].var == v) return cur;
        }
        cur = _sir_class_super(cur);
    }
    return start_class;
}

/* Store `@@var = val` at class `cls` (update-in-place, else bounded append). */
static void _sir_cvar_store(const char *cls, const char *var, SirValue val) {
    const char *c = _sir_intern(cls), *v = _sir_intern(var);
    int i;
    for (i = 0; i < _sir_cvar_n; i++) {
        if (_sir_cvar_tab[i].cls == c && _sir_cvar_tab[i].var == v) {
            _sir_cvar_tab[i].val = val;
            return;
        }
    }
    if (_sir_cvar_n < SIR_CVAR_MAX) {
        _sir_cvar_tab[_sir_cvar_n].cls = c;
        _sir_cvar_tab[_sir_cvar_n].var = v;
        _sir_cvar_tab[_sir_cvar_n].val = val;
        _sir_cvar_n++;
    }
}

/* A class-body initializer `@@x = v` (in `class Cls`), where the class is known
 * statically — so it seeds the storage the class's methods later resolve to. */
SirValue _sir_cvar_set_in(const char *cls, const char *var, SirValue val) {
    _sir_cvar_store(cls, var, val);
    return val;
}

/* `@@x` read from a method body: resolve the owning class from
 * `_sir_current_class` (nil when there is no current class — e.g. a stray
 * top-level `@@x`, which Ruby forbids anyway). */
SirValue _sir_cvar_get(const char *var) {
    const char *v = _sir_intern(var);
    const char *owner;
    int i;
    if (!_sir_current_class) return _sir_nil();
    owner = _sir_intern(_sir_cvar_owner(_sir_current_class, var));
    for (i = 0; i < _sir_cvar_n; i++) {
        if (_sir_cvar_tab[i].cls == owner && _sir_cvar_tab[i].var == v) return _sir_cvar_tab[i].val;
    }
    return _sir_nil();
}

/* `@@x = v` write from a method body: store at the shared owner (so a subclass
 * write updates the parent's variable, matching Ruby). */
SirValue _sir_cvar_set(const char *var, SirValue val) {
    const char *owner = _sir_current_class ? _sir_cvar_owner(_sir_current_class, var) : "";
    _sir_cvar_store(owner, var, val);
    return val;
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
