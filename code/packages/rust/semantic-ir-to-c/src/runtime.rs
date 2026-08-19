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
    SIR_INSTANCE,
    /* SIR22: a dense, rank <= 2, COLUMN-MAJOR numeric array (`SirNDArray`) —
     * see the "SIR22 array/matrix domain" section near the end of this file
     * for the full value model and every `_sir_array_*` op. A dedicated tag
     * (rather than reusing `SIR_SEQ`) mirrors `SIR_INSTANCE`'s own reasoning:
     * no built-in Sequence helper should ever mis-handle an NDArray, and the
     * two have different storage (a flat `double*` here, not a `SirValue*`). */
    SIR_ARRAY
} SirTag;

typedef struct SirValue SirValue;
typedef struct SirPair SirPair;
typedef struct SirClosure SirClosure;
typedef struct SirSeq SirSeq;
typedef struct SirMap SirMap;
typedef struct SirError SirError;
typedef struct SirInstance SirInstance;
typedef struct SirNDArray SirNDArray;

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
        SirNDArray *arr;  /* SIR_ARRAY */
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

/* SIR21 T3b-2: true (float) division -- always coerces both operands to
 * double and divides, regardless of operand tag (unlike `_sir_divide_v`,
 * which floors when both operands happen to be Int -- that's `div_floor`'s
 * job, this is `div_true`'s). Fails loudly on a zero divisor, matching
 * every other division builtin in this file (`_sir_ifloordiv`/`_sir_itdiv`/
 * `_sir_utdiv`) rather than the OLDER `_sir_divide_v`'s float path, which
 * silently produces IEEE inf/nan on `x / 0.0` -- `div_true` models
 * Python's `/`, and Python's `ZeroDivisionError` fires unconditionally,
 * not just when both operands happen to be integers, so IEEE inf/nan is
 * never a silently-produced result here. */
SirValue _sir_true_div(SirValue a, SirValue b) {
    double bn = _sir_as_num(b);
    if (bn == 0.0) { fprintf(stderr, "sir: divided by 0\n"); exit(1); }
    return _sir_float(_sir_as_num(a) / bn);
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
        /* Reference identity only (like `SIR_CLOSURE`/`SIR_ERROR` above) —
         * a full element-wise structural comparison is out of this slice's
         * scope; this at least makes `a == a` true rather than silently
         * falling to the `default: return 0` below. */
        case SIR_ARRAY:   return a.as.arr == b.as.arr;
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
        /* A full `[1 2; 3 4]`-style rendering is out of this slice's scope
         * (see the "SIR22 array/matrix domain" section below) — every base-
         * cut test reads a SCALAR element back out via `IndexGet` instead,
         * exactly like the JS/Ruby references' own test suites. This
         * placeholder mirrors `SIR_CLOSURE`'s `#<closure>` precedent. */
        case SIR_ARRAY: fputs("#<Array>", out); break;
        default: break;
    }
    _sir_fmt_depth--;
}

/* ---- SIR18 string interpolation: value display AS A STRING -- */

/* `"a#{x}b"` needs each part's Ruby `to_s`-style display rendered into a
 * fresh string to concatenate, not written to a `FILE*` — so this is a
 * SEPARATE function from `_sir_fmt` above, not a refactor of it: `_sir_fmt`
 * backs the already-tested `puts`/`print` path, and duplicating its per-tag
 * rendering here (into `_sir_cat`-built strings instead of `fputs` calls)
 * keeps that path completely untouched. The two are kept in lockstep by
 * inspection — same tag list, same per-tag text, same recursion structure —
 * so `#{arr}` and `puts arr` always agree. */
char *_sir_display_str(SirValue v);

char *_sir_display_seq(SirValue v) {
    char *acc = _sir_dup("[");
    int64_t i;
    for (i = 0; i < v.as.seq->len; i++) {
        if (i) acc = _sir_cat(acc, ", ");
        acc = _sir_cat(acc, _sir_display_str(v.as.seq->items[i]));
    }
    return _sir_cat(acc, "]");
}

char *_sir_display_map(SirValue v) {
    char *acc = _sir_dup("{");
    int64_t i;
    for (i = 0; i < v.as.map->len; i++) {
        if (i) acc = _sir_cat(acc, ", ");
        acc = _sir_cat(acc, _sir_display_str(v.as.map->entries[i].key));
        acc = _sir_cat(acc, ": ");
        acc = _sir_cat(acc, _sir_display_str(v.as.map->entries[i].val));
    }
    return _sir_cat(acc, "}");
}

char *_sir_display_float(double f) {
    char buf[64];
    if (f != f)                    return _sir_dup("NaN");
    if (f == f * 0.5 && f != 0.0)  return _sir_dup(f < 0 ? "-Infinity" : "Infinity");
    snprintf(buf, sizeof(buf), "%.17g", f);
    if (!strchr(buf, '.') && !strchr(buf, 'e') && !strchr(buf, 'E') &&
        !strchr(buf, 'n') && !strchr(buf, 'N')) {
        size_t L = strlen(buf);
        if (L + 2 < sizeof(buf)) { buf[L] = '.'; buf[L + 1] = '0'; buf[L + 2] = '\0'; }
    }
    return _sir_dup(buf);
}

char *_sir_display_pair(SirValue v) {
    SirValue cur = v;
    char *acc = _sir_dup("(");
    int first = 1;
    for (;;) {
        if (cur.tag == SIR_PAIR) {
            if (!first) acc = _sir_cat(acc, " ");
            first = 0;
            acc = _sir_cat(acc, _sir_display_str(cur.as.pair->car));
            cur = cur.as.pair->cdr;
        } else if (cur.tag == SIR_NIL) {
            break;
        } else {
            acc = _sir_cat(acc, " . ");
            acc = _sir_cat(acc, _sir_display_str(cur));
            break;
        }
    }
    return _sir_cat(acc, ")");
}

/* Bounded exactly like `_sir_fmt_depth`/`SIR_MAX_FMT_DEPTH` above — a
 * self-referential sequence/map (constructible via `SeqSet`/`MapSet`) would
 * otherwise recurse forever. */
#define SIR_MAX_DISPLAY_DEPTH 500
static int _sir_display_depth = 0;

char *_sir_display_str(SirValue v) {
    char buf[32];
    char *result;
    if (_sir_display_depth > SIR_MAX_DISPLAY_DEPTH) return _sir_dup("[...]");
    _sir_display_depth++;
    switch (v.tag) {
        case SIR_NIL:   result = _sir_dup(SIR_DISPLAY_RUBY ? "" : "nil"); break;
        case SIR_BOOL:  result = _sir_dup(v.as.b ? (SIR_DISPLAY_RUBY ? "true" : "#t")
                                                  : (SIR_DISPLAY_RUBY ? "false" : "#f")); break;
        case SIR_INT:   snprintf(buf, sizeof(buf), "%lld", (long long)v.as.i); result = _sir_dup(buf); break;
        case SIR_FLOAT: result = _sir_display_float(v.as.f); break;
        case SIR_STR:   result = _sir_dup(v.as.s); break;
        case SIR_SYM:   result = _sir_dup(v.as.s); break;
        case SIR_PAIR:  result = _sir_display_pair(v); break;
        case SIR_SEQ:   result = _sir_display_seq(v); break;
        case SIR_MAP:   result = _sir_display_map(v); break;
        case SIR_CLOSURE: result = _sir_dup("#<closure>"); break;
        case SIR_ERROR:
            result = (v.as.err->msg.tag == SIR_NIL)
                ? _sir_dup(v.as.err->sir_class)
                : _sir_display_str(v.as.err->msg);
            break;
        case SIR_INSTANCE:
            result = _sir_cat(_sir_cat("#<", v.as.inst->sir_class), ">");
            break;
        /* See `_sir_fmt`'s matching `SIR_ARRAY` case above for why this is a
         * placeholder, not a full `[1 2; 3 4]` rendering. */
        case SIR_ARRAY: result = _sir_dup("#<Array>"); break;
        default: result = _sir_dup("");
    }
    _sir_display_depth--;
    return result;
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
 * included first, per SIR25 §2.2/§2.4's precedence — matching Ruby's).
 * Bounded by SIR_ANCESTRY_MAX. */
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

/* `Foo.new(args…)` — allocate a `cls` instance and run its `initialize`.
 *
 * Allocates via `_sir_new_instance`, then — if `initialize` resolves on `cls`
 * or an ancestor (`_sir_resolve_method`, the same ancestry walk `_sir_call_
 * method` uses) — binds `self` to the new object and invokes it with `args`,
 * so constructor-body `@ivar` assignments land on the new object (mirroring
 * the Go/Rust/Ruby backends, which have always run `initialize` here).  Self
 * is restored afterward, same save/restore as `_sir_call_method`.  The object
 * is always returned, even with no `initialize` registered — a plain
 * allocation, per SIR25 §2.1's no-op-default construction (matching Ruby's
 * default `Object#initialize`). */
SirValue _sir_call_new(const char *cls, int argc, ...) {
    va_list ap;
    SirValue *args, obj, fn;
    obj = _sir_new_instance(cls);
    fn = _sir_resolve_method(cls, "initialize");
    if (fn.tag == SIR_CLOSURE) {
        va_start(ap, argc);
        args = _sir_va_collect(argc, ap);
        va_end(ap);
        {
            SirValue saved_self = _sir_current_self;
            const char *saved_class = _sir_current_class;
            _sir_current_self = obj;
            _sir_current_class = obj.as.inst->sir_class;
            fn.as.clo->fn(fn.as.clo->caps, args, argc);
            _sir_current_self = saved_self;
            _sir_current_class = saved_class;
        }
        if (args) free(args);
    }
    return obj;
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

/* Forward declaration -- defined below in the Numeric-methods section (it's
   the shared saturating float->int64 cast every `floor`/`ceil`/`round`
   there also uses); the padding-width extractor just below needs it too. */
static int64_t _sir_f64_to_i64_saturating(double f);

/* Deferred-from-slice-8: char-set methods (`count`/`delete`/`squeeze`) and
 * padding methods (`ljust`/`rjust`/`center`). Semantics matched against the
 * Python/TS `sir-runtime-oop` reference catalog, same discipline as every
 * other String method in this file. Each `charset` argument is treated
 * LITERALLY as the set of characters it contains -- Ruby's char-RANGE
 * (`"a-z"`) and NEGATION (`"^abc"`) forms are a documented follow-up, the
 * same literal-only scope precedent `tr`/`sub`/`gsub` already use above.
 * Multiple charset arguments INTERSECT (Ruby's rule). This runtime is
 * byte-oriented throughout (matching `bytes`/`length`), so "characters"
 * below means bytes -- fits a flat 256-entry membership table, no `<ctype.h>`
 * locale surprises. */

/* Computes, for each byte value, whether it appears in EVERY `argc` String
   argument (the charset intersection) -- non-String arguments are ignored
   (matching the reference's `isinstance(a, str)` filter), and ZERO String
   arguments yields an all-empty set (Ruby's `count`/`delete` need at least
   one charset; `squeeze`'s no-charset case is handled separately below,
   NOT via an all-empty set, since "in no set" must mean "squeeze nothing"
   there, not "squeeze everything"). */
static void _sir_charset_membership(int argc, SirValue *args, unsigned char *in_set /* [256] */) {
    /* `int`, not `unsigned char` -- a `count`/`delete`/`squeeze` call with
       more than 255 String charset arguments would wrap an `unsigned char`
       counter modulo 256, silently under-counting the intersection for a
       byte present in all of them (wrong answer, not a memory-safety bug,
       but avoided outright since the extra stack is negligible). */
    int counts[256];
    int nsets = 0, i, c;
    memset(counts, 0, sizeof(counts));
    for (i = 0; i < argc; i++) {
        unsigned char seen[256];
        const char *p;
        if (args[i].tag != SIR_STR) continue;
        memset(seen, 0, sizeof(seen));
        for (p = args[i].as.s; *p; p++) seen[(unsigned char)*p] = 1;
        for (c = 0; c < 256; c++) if (seen[c]) counts[c]++;
        nsets++;
    }
    for (c = 0; c < 256; c++) in_set[c] = (nsets > 0 && counts[c] == nsets) ? 1 : 0;
}

static SirValue _sir_str_count_charset(const char *s, int argc, SirValue *args) {
    unsigned char in_set[256];
    int64_t n = 0;
    const char *p;
    _sir_charset_membership(argc, args, in_set);
    for (p = s; *p; p++) if (in_set[(unsigned char)*p]) n++;
    return _sir_int(n);
}

static char *_sir_str_delete_charset(const char *s, int argc, SirValue *args) {
    unsigned char in_set[256];
    size_t n = strlen(s), w = 0, i;
    char *out = (char *)_sir_alloc(n + 1);
    _sir_charset_membership(argc, args, in_set);
    for (i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!in_set[c]) out[w++] = (char)c;
    }
    out[w] = '\0';
    return out;
}

/* `squeeze(charset=nil)` -- collapse consecutive runs. With NO charset
   argument, collapses runs of ANY char (every `argc == 0` call); with one+
   charset arguments, only runs of chars in the (intersected) set collapse.
   The `has_set` flag -- not `_sir_charset_membership`'s own empty-set
   result -- distinguishes these, since a truly empty intersection (e.g.
   two disjoint charset arguments) must squeeze NOTHING, while no charset
   at all must squeeze EVERYTHING; both look identical downstream unless
   kept as separate cases. */
static char *_sir_str_squeeze(const char *s, int argc, SirValue *args) {
    unsigned char in_set[256];
    int has_set = argc > 0;
    size_t n = strlen(s), w = 0, i;
    char *out = (char *)_sir_alloc(n + 1);
    if (has_set) _sir_charset_membership(argc, args, in_set);
    for (i = 0; i < n; i++) {
        unsigned char c = (unsigned char)s[i];
        int in_this_set = has_set ? in_set[c] : 1;
        if (w > 0 && (unsigned char)out[w - 1] == c && in_this_set) continue;
        out[w++] = (char)c;
    }
    out[w] = '\0';
    return out;
}

/* Extracts a `ljust`/`rjust`/`center` width argument as a plain `int64_t`,
   the same UB-avoidance discipline `round(ndigits)` would need for its own
   numeric argument: never a bare `(int64_t)v.as.f` cast (UB for a
   non-finite/out-of-range Float), routed through the shared saturating
   helper instead. */
static int64_t _sir_str_width_arg(SirValue v) {
    if (v.tag == SIR_INT) return v.as.i;
    if (v.tag == SIR_FLOAT) return _sir_f64_to_i64_saturating(v.as.f);
    return 0;
}

/* Builds a FRESH buffer of exactly `n` bytes by repeating `pad` cyclically
   (truncating the final repeat); `n <= 0` returns `""`. The current caller
   (`_sir_str_justify`) already guarantees a non-empty `pad`, but `pl == 0`
   is guarded here too, self-contained, rather than trusted purely by
   caller discipline -- an empty `pad` would otherwise divide by zero. */
static char *_sir_str_pad_buf(const char *pad, int64_t n) {
    size_t pl, i;
    char *out;
    if (n <= 0) return _sir_dup("");
    pl = strlen(pad);
    if (pl == 0) { pad = " "; pl = 1; }
    out = (char *)_sir_alloc((size_t)n + 1);
    for (i = 0; i < (size_t)n; i++) out[i] = pad[i % pl];
    out[n] = '\0';
    return out;
}

/* SIR_MAX_PAD_LEN mirrors the Python/TS reference's `_MAX_REPEAT_LEN`: a
   deficit above this is CLAMPED, not rejected, so a hostile width (e.g.
   `"".ljust(10**18)`) cannot exhaust memory -- `_sir_alloc` itself only
   guards against a FAILED allocation (aborts cleanly), not a succeeding
   multi-gigabyte one, so the cap has to happen here, before the alloc. */
#define SIR_MAX_PAD_LEN 100000000

/* `ljust`/`rjust`/`center(width, pad=" ")` -- pad `s` to `width` bytes using
   `pad` repeated cyclically; `width <= len(s)` is a no-op. `center` puts
   any odd leftover pad byte on the RIGHT (Ruby's rule -- the opposite of
   Python's single-char-only `str.center`). `mode`: 0 = ljust, 1 = rjust,
   2 = center. The `width <= 0` short-circuit below is load-bearing, not
   just an optimization: `width` can be `INT64_MIN` (a saturated hostile
   Float argument), and `width - (int64_t)len` on that value is
   signed-overflow UB -- returning before ever computing the subtraction
   sidesteps it entirely (any non-positive width means "no padding needed"
   regardless, so the short-circuit is also semantically correct, not just
   a safety patch). */
static char *_sir_str_justify(const char *s, int64_t width, const char *pad, int mode) {
    size_t len;
    int64_t deficit, left, right;
    char *lp, *rp, *mid;
    if (width <= 0) return _sir_dup(s);
    len = strlen(s);
    deficit = width - (int64_t)len;
    if (deficit > SIR_MAX_PAD_LEN) deficit = SIR_MAX_PAD_LEN;
    if (deficit <= 0) return _sir_dup(s);
    if (mode == 0) return _sir_cat(s, _sir_str_pad_buf(pad, deficit));
    if (mode == 1) return _sir_cat(_sir_str_pad_buf(pad, deficit), s);
    left = deficit / 2;
    right = deficit - left;
    lp = _sir_str_pad_buf(pad, left);
    mid = _sir_cat(lp, s);
    rp = _sir_str_pad_buf(pad, right);
    return _sir_cat(mid, rp);
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

/* A `double`-to-`int64_t` cast is UB (platform-dependent -- e.g. saturates
 * on arm64, gives INT64_MIN "integer indefinite" on x86) once the value is
 * non-finite or outside int64 range. `floor`/`ceil`/`round` on a hostile
 * huge/inf/nan Float must not depend on which CPU the C got compiled for,
 * so every cast below goes through this guard first: out-of-range/non-finite
 * saturates to INT64_MAX/INT64_MIN (never-raise floor, matching this
 * runtime's other numeric conversions, e.g. `_sir_mask_to`/`_sir_convert`
 * never trap either). */
static int64_t _sir_f64_to_i64_saturating(double f) {
    if (!(f == f)) return 0;                 /* NaN -> 0 */
    if (f >= 9223372036854775807.0) return INT64_MAX;
    if (f < -9223372036854775808.0) return INT64_MIN;
    return (int64_t)f;
}

static SirValue _sir_num_floor(SirValue v) {
    if (v.tag == SIR_INT) return v;
    if (v.tag == SIR_FLOAT) return _sir_int(_sir_f64_to_i64_saturating(floor(v.as.f)));
    return v;
}
static SirValue _sir_num_ceil(SirValue v) {
    if (v.tag == SIR_INT) return v;
    if (v.tag == SIR_FLOAT) return _sir_int(_sir_f64_to_i64_saturating(ceil(v.as.f)));
    return v;
}
/* Round-half-AWAY-from-zero (Ruby's rule; unlike Python's banker's rounding,
 * already the convention this runtime's `sir-runtime-oop` reference uses). */
static SirValue _sir_num_round(SirValue v) {
    if (v.tag == SIR_INT) return v;
    if (v.tag == SIR_FLOAT) {
        double f = v.as.f;
        double r = (f >= 0.0) ? floor(f + 0.5) : ceil(f - 0.5);
        return _sir_int(_sir_f64_to_i64_saturating(r));
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
        /* Two DISTINCT overflow hazards, both avoided below rather than
           computed and patched up:
           (1) `a / b` / `a % b` (C's own operators, which `_sir_ifloordiv`
               used verbatim) are signed-overflow UB for `a == INT64_MIN,
               b == -1` -- so that ONE combination is special-cased first:
               it divides evenly (remainder always 0), so the quotient is
               just `-a`, saturated to `INT64_MIN` for the single input
               (`a == INT64_MIN`) where `-a` itself would overflow --
               mirroring the wraparound convention `_sir_itdiv` already uses
               for this exact pair.
           (2) For every OTHER `b`, computing the floored remainder via
               `a - q * b` (an earlier version of this function) re-invites
               overflow through the BACK door: e.g. `a = INT64_MIN, b = 3`
               floors to `q = -3074457345618258603`, and `q * b` alone
               overflows `int64_t` by 1 computing an intermediate product
               that the FINAL remainder (1) never needed. Adjusting the
               truncating remainder directly (`tr + b`, bounded in
               magnitude by `b` and hence always in range) instead of
               multiplying sidesteps this -- the standard truncating-to-
               floored conversion, done without a multiply. */
        if (b == -1) {
            q = (a == INT64_MIN) ? INT64_MIN : -a;
            return _sir_seq_lit(2, _sir_int(q), _sir_int(0));
        }
        {
            int64_t tq = a / b, tr = a % b;
            if (tr != 0 && ((tr < 0) != (b < 0))) { q = tq - 1; r = tr + b; }
            else { q = tq; r = tr; }
        }
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

/* `-INT64_MIN` is signed-overflow UB (its magnitude, 2^63, has no positive
 * `int64_t` representation) -- confirmed to actually misbehave under
 * optimization (wrong answers, and in one configuration a hardware trap) on
 * this project's own toolchain, not just a theoretical hazard. Every caller
 * that needs "the magnitude of a possibly-INT64_MIN value" goes through this
 * helper instead of a bare unary `-`, computing it in `uint64_t` (well-
 * defined two's-complement wraparound covers the full range including
 * `INT64_MIN`'s magnitude, which does not fit back in an `int64_t`). */
static uint64_t _sir_i64_abs_u(int64_t n) {
    return (n < 0) ? (uint64_t)0 - (uint64_t)n : (uint64_t)n;
}

/* Extracts a `round(ndigits)` argument as a plain `int64_t`, WITHOUT going
 * through `_sir_as_int`'s bare `(int64_t)v.as.f` cast (UB for a non-finite
 * or out-of-range Float -- see `_sir_f64_to_i64_saturating`'s doc comment
 * above). A hostile NaN/Infinity argument saturates to 0/`INT64_MAX`/
 * `INT64_MIN` instead, each of which the bounds checks in
 * `_sir_num_round_ndigits` below turn into a safe, harmless outcome. */
static int64_t _sir_round_ndigits_arg(SirValue v) {
    if (v.tag == SIR_INT) return v.as.i;
    if (v.tag == SIR_FLOAT) return _sir_f64_to_i64_saturating(v.as.f);
    return 0;
}

/* Ten-to-the-`k` as a `uint64_t`. Every caller below only ever passes `k` in
   `0..=18` (each checks a "dwarfs the value" bound first), so this never
   approaches `UINT64_MAX` (~1.8e19) -- the largest value returned is 10^18,
   comfortably inside both `uint64_t` and `int64_t` range. */
static uint64_t _sir_pow10_u(int k) {
    uint64_t r = 1;
    while (k-- > 0) r *= 10;
    return r;
}

/* `round(ndigits)` -- the multi-digit form. Ruby dispatches on BOTH the
 * receiver's own type and the sign of `ndigits`:
 *
 *   Integer, ndigits >= 0  -> receiver unchanged (already exact at any
 *                             decimal place an Integer could round to)
 *   Integer, ndigits <  0  -> round to the nearest 10^(-ndigits), e.g.
 *                             1234.round(-2) == 1200
 *   Float,   ndigits >  0  -> round to `ndigits` decimal places, stays a
 *                             Float, e.g. 3.14159.round(2) == 3.14
 *   Float,   ndigits <= 0  -> round to the nearest 10^(-ndigits) and
 *                             CONVERT to an Integer, e.g.
 *                             1234.5.round(-2) == 1200 (Integer, not Float)
 *
 * Magnitude caps keep every path inside `int64_t`/`double` range without a
 * bignum, mirroring `_sir_num_digits`'s "no separate DoS cap needed"
 * reasoning:
 *   - Integer negative-`ndigits` path: `|recv|` is always < 10^19 (an
 *     `int64_t` magnitude has at most 19 decimal digits), so once the
 *     rounding place reaches 10^19 the result is unconditionally 0 --
 *     capped at `-ndigits >= 19` rather than computed per-receiver, and
 *     kept to a `factor` of at most 10^18 so a carry from rounding up
 *     (e.g. `9223372036854775807.round(-1)`, which would need one MORE
 *     digit than `int64_t` holds) is caught by the explicit saturating
 *     check below rather than silently wrapping.
 *   - Float paths (either sign of `ndigits`): capped at the ~17
 *     significant decimal digits a `double` can actually represent --
 *     beyond that, rounding is meaningless (the receiver is already exact
 *     at that many digits, or precision was lost upstream of this call),
 *     so the receiver is returned unchanged / dwarfed to 0 rather than
 *     manufacturing false precision or calling `pow()` with a wild
 *     exponent.
 */
static SirValue _sir_num_round_ndigits(SirValue recv, int64_t ndigits) {
    if (recv.tag == SIR_INT) {
        uint64_t k, factor, mag, q, rem, result;
        int neg;
        if (ndigits >= 0) return recv;
        k = _sir_i64_abs_u(ndigits);
        if (k >= 19) return _sir_int(0);
        factor = _sir_pow10_u((int)k);
        mag = _sir_i64_abs_u(recv.as.i);
        q = mag / factor;
        rem = mag % factor;
        if (rem >= factor - rem) q += 1;  /* half-away-from-zero; no `rem*2` overflow */
        result = q * factor;
        neg = recv.as.i < 0;
        if (neg) {
            if (result >= (uint64_t)INT64_MAX + 1u) return _sir_int(INT64_MIN);
            return _sir_int(-(int64_t)result);
        }
        if (result > (uint64_t)INT64_MAX) return _sir_int(INT64_MAX);
        return _sir_int((int64_t)result);
    }
    if (recv.tag == SIR_FLOAT) {
        double f = recv.as.f, factor, scaled, r;
        if (ndigits > 0) {
            if (ndigits > 17) return recv;
            factor = pow(10.0, (double)ndigits);
            scaled = f * factor;
            r = (scaled >= 0.0) ? floor(scaled + 0.5) : ceil(scaled - 0.5);
            return _sir_float(r / factor);
        }
        {
            /* `-ndigits` is signed-overflow UB when `ndigits == INT64_MIN`
               (reachable: `_sir_round_ndigits_arg` saturates a hostile
               huge-negative Float ndigits argument to exactly INT64_MIN) --
               the SAME hazard the Integer branch above avoids via
               `_sir_i64_abs_u` instead of a bare unary `-`. */
            uint64_t k = _sir_i64_abs_u(ndigits);
            if (k > 18) return _sir_int(0);
            factor = pow(10.0, (double)(int)k);
            scaled = f / factor;
            r = (scaled >= 0.0) ? floor(scaled + 0.5) : ceil(scaled - 0.5);
            return _sir_int(_sir_f64_to_i64_saturating(r * factor));
        }
    }
    return recv;
}

static SirValue _sir_num_gcd(SirValue recv, SirValue other) {
    uint64_t a = _sir_i64_abs_u(_sir_as_int(recv));
    uint64_t b = _sir_i64_abs_u(_sir_as_int(other));
    while (b != 0) { uint64_t t = b; b = a % b; a = t; }
    /* `a` is the gcd's magnitude, bounded by `min(|recv|,|other|)` when
       NEITHER is 0, but by `max(|recv|,|other|)` when one IS 0 (Ruby:
       `0.gcd(x) == x.abs`) -- and `|INT64_MIN|` is exactly `2^63`, one past
       `INT64_MAX` (`2^63-1`). A bare `(int64_t)a` narrowing there is an
       out-of-range conversion (confirmed to silently wrap to `INT64_MIN` in
       practice) -- e.g. `0.gcd(INT64_MIN)`. Since this runtime has no
       bignum to hold the true value, this saturates to `INT64_MAX` instead,
       the same never-raise-by-saturating convention `_sir_f64_to_i64_
       saturating` uses just above for the analogous "true value doesn't
       fit" case. */
    return _sir_int(a > (uint64_t)INT64_MAX ? INT64_MAX : (int64_t)a);
}

/* `digits` -- base-10 digits, LEAST-significant first (`123.digits ==
 * [3, 2, 1]`). Bounded by construction: an `int64_t` magnitude has at most
 * 19 decimal digits, so no separate DoS cap is needed (unlike the Python
 * reference's arbitrary-precision receiver). `0.digits == [0]`. */
static SirValue _sir_num_digits(SirValue recv) {
    uint64_t n = _sir_i64_abs_u(_sir_as_int(recv));
    SirValue buf[20];
    int64_t k = 0;
    SirSeq *r;
    if (n == 0) buf[k++] = _sir_int(0);
    while (n > 0) { buf[k++] = _sir_int((int64_t)(n % 10)); n /= 10; }
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
/* `i++`/`i--` in a `for (; i <= n; i++)`-shaped loop is signed-overflow UB
 * the moment `i == INT64_MAX` (`upto`) or `i == INT64_MIN` (`downto`) --
 * reachable from plain Ruby source (`INT64_MAX.upto(INT64_MAX) { |i| .. }`
 * is one iteration whose loop-continuation test would still increment past
 * the top). Both loops below instead apply the block FIRST, then check for
 * "was that the last (boundary) value" and `break` BEFORE ever advancing
 * past it -- so the increment/decrement is only ever reached when doing so
 * is safe. */
static SirValue _sir_num_upto(SirValue recv, SirValue limit, SirValue block) {
    int64_t i = _sir_as_int(recv), n = _sir_as_int(limit);
    if (i > n) return recv;
    for (;;) {
        _sir_apply(block, 1, _sir_int(i));
        if (i == n) break;
        i++;
    }
    return recv;
}
static SirValue _sir_num_downto(SirValue recv, SirValue limit, SirValue block) {
    int64_t i = _sir_as_int(recv), n = _sir_as_int(limit);
    if (i < n) return recv;
    for (;;) {
        _sir_apply(block, 1, _sir_int(i));
        if (i == n) break;
        i--;
    }
    return recv;
}
/* `v + st` would be signed-overflow UB if it crossed int64 range -- checked
 * BEFORE performing the addition (computing it first and inspecting the
 * result would already be the UB), so this is safe to call unconditionally. */
static int _sir_i64_add_overflows(int64_t a, int64_t b) {
    return (b >= 0) ? (a > INT64_MAX - b) : (a < INT64_MIN - b);
}
/* `step(limit, stride = 1) { |i| .. }` -- a stride of 0 would loop forever
 * (never crosses `limit`), so it is a documented no-op (zero iterations)
 * rather than a hang, the same DoS-safety floor `sub`/`gsub`'s empty-pattern
 * guard holds for string scanning. A stride that would carry `v` past
 * int64 range (e.g. a huge stride near `INT64_MAX`) stops the iteration
 * there instead of overflowing -- there is no next in-range value to visit
 * anyway. Promotes to float stepping if EITHER the receiver or the stride
 * is a Float (matches Ruby: `1.step(2, 0.5)` yields floats), otherwise
 * stays in exact int64 arithmetic. */
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
        if (st > 0) {
            for (; v <= lim; ) {
                _sir_apply(block, 1, _sir_int(v));
                if (_sir_i64_add_overflows(v, st)) break;
                v += st;
            }
        } else {
            for (; v >= lim; ) {
                _sir_apply(block, 1, _sir_int(v));
                if (_sir_i64_add_overflows(v, st)) break;
                v += st;
            }
        }
    }
    return recv;
}

/* ---- Collections slice 10: Symbol + Object/Bool generic methods -------------
 *
 * Symbol's `to_s`/`length`/`size`/`upcase`/`downcase`/`empty?` reuse the
 * SAME helpers slice 1/8 built for String (a `SIR_SYM`'s name is stored the
 * identical way a `SIR_STR`'s is, `.as.s`) — only the WRAPPING differs:
 * `upcase`/`downcase` re-intern the result as a fresh Symbol (Ruby:
 * `:foo.upcase == :FOO`, a Symbol, not a String), everything else returns
 * the same type String's arm does. `inspect` is Symbol-specific (prefixes
 * `:`, no String equivalent). `to_sym` is the identity (already a Symbol). */

static SirValue _sir_sym_inspect(const char *name) {
    return _sir_str(_sir_cat(":", name));
}

/* `equal?` -- Ruby's OBJECT-IDENTITY comparison (`a.equal?(b)`), distinct
 * from `==`'s structural/value equality (`_sir_value_eq`, unaffected by this
 * slice). For a heap-boxed type (String/Symbol/Array/Hash/Pair/Closure/
 * Instance) identity IS pointer identity -- two SEPARATELY built values with
 * equal content are NOT `equal?`, only two handles to the SAME allocation
 * are (`x = [1]; y = [1]; x.equal?(y)` is false; `y = x; x.equal?(y)` is
 * true). For a scalar (nil/bool/int/float) Ruby has no separate identity
 * from value, so this compares by value -- which for a `SIR_SYM` also
 * reduces to pointer identity, since symbols are interned (`_sir_intern`
 * hands back the SAME pointer for the same name). */
static SirValue _sir_object_equal_p(SirValue a, SirValue b) {
    if (a.tag != b.tag) return _sir_bool(0);
    switch (a.tag) {
        case SIR_NIL:      return _sir_bool(1);
        case SIR_BOOL:     return _sir_bool(a.as.b == b.as.b);
        case SIR_INT:      return _sir_bool(a.as.i == b.as.i);
        case SIR_FLOAT:    return _sir_bool(a.as.f == b.as.f);
        case SIR_STR:      /* fallthrough */
        case SIR_SYM:      return _sir_bool(a.as.s == b.as.s);
        case SIR_PAIR:     return _sir_bool(a.as.pair == b.as.pair);
        case SIR_CLOSURE:  return _sir_bool(a.as.clo == b.as.clo);
        case SIR_SEQ:      return _sir_bool(a.as.seq == b.as.seq);
        case SIR_MAP:      return _sir_bool(a.as.map == b.as.map);
        case SIR_ERROR:    return _sir_bool(a.as.err == b.as.err);
        case SIR_INSTANCE: return _sir_bool(a.as.inst == b.as.inst);
        case SIR_ARRAY:    return _sir_bool(a.as.arr == b.as.arr);
        default:           return _sir_bool(0);  /* SIR_MISSING: never user-observed */
    }
}

/* `frozen?` -- v0 has no mutability tracking, so this reports the
 * ALWAYS-immutable primitives as frozen (matching Ruby: small Integers,
 * Symbols, `true`/`false`/`nil`, and Floats are unconditionally frozen) and
 * everything else (String/Array/Hash/Instance, all mutable in this runtime)
 * as not — a fixed, receiver-type-only answer, not a real per-object flag. */
static SirValue _sir_object_frozen_p(SirValue v) {
    switch (v.tag) {
        case SIR_NIL: case SIR_BOOL: case SIR_INT: case SIR_FLOAT: case SIR_SYM:
            return _sir_bool(1);
        default:
            return _sir_bool(0);
    }
}

/* `<<` -- Ruby's shift operator, polymorphic like `+`:
 *   Array    -- push each RHS operand in place (`_sir_array_push_one`,
 *               slice 4's growth helper), returns the (mutated) receiver.
 *               Chains left-to-right: `a << 1 << 2` pushes both.
 *   Integer  -- bitwise shift; see `_sir_shift_left_i64` below for the
 *               overflow/negative-amount handling.
 *   String   -- concatenates and returns a NEW string (via `_sir_str_of` +
 *               `_sir_cat`, the SAME helper `_sir_plus_v` already uses for
 *               `+`'s String-receiver case). This diverges from real Ruby
 *               in two ways, both DOCUMENTED, not silent: (1) true
 *               `String#<<` mutates in place with shared-reference
 *               visibility (like Array's push) -- this runtime's `SIR_STR`
 *               is a bare `const char *` with no heap box/pointer identity
 *               (unlike `SIR_SEQ`/`SIR_MAP`), so in-place mutation isn't
 *               representable without a different String representation
 *               entirely, a materially larger undertaking; (2) real Ruby's
 *               `"a" << 98` appends the CHARACTER at codepoint 98 (`"ab"`),
 *               not the stringified integer -- deferred (needs UTF-8
 *               encoding of an arbitrary codepoint). `_sir_str_of` only
 *               recognizes `SIR_STR`/`SIR_SYM` (returns `""` for anything
 *               else, e.g. an Integer) -- so a non-String/Symbol RHS is
 *               silently DROPPED (contributes nothing to the result),
 *               exactly matching `_sir_plus_v`'s existing behavior for
 *               `"a" + 5` in this same runtime (which also drops the `5`
 *               rather than stringifying it -- real Ruby raises `TypeError`
 *               for that expression instead; this runtime never raises
 *               here, matching its established never-raise-on-`+` floor).
 */

/* Extracts a shift-amount argument as a plain `int64_t`, the same
   UB-avoidance discipline `round(ndigits)`/`ljust` use for their own
   numeric arguments: never a bare `(int64_t)v.as.f` cast (UB for a
   non-finite/out-of-range Float), routed through the shared saturating
   helper instead. Real Ruby truncates a Float shift amount toward zero
   (`5 << 2.5 == 5 << 2`); `_sir_f64_to_i64_saturating` already truncates
   for any in-range value and saturates for a hostile huge/non-finite one. */
static int64_t _sir_shift_amount_arg(SirValue v) {
    if (v.tag == SIR_INT) return v.as.i;
    if (v.tag == SIR_FLOAT) return _sir_f64_to_i64_saturating(v.as.f);
    return 0;
}

/* Bitwise-shifts `n` by `amount`, matching real Ruby's rules:
 *   - `amount == 0` or `n == 0`: identity (no-op).
 *   - `amount < 0`: a NEGATIVE shift amount REVERSES direction -- it's a
 *     RIGHT shift by `|amount|` (Ruby: `5 << -1 == 5 >> 1 == 2`). `n >> k`
 *     for a negative `n` is implementation-defined (not UB) in C99; every
 *     platform this project targets (gcc/clang/MSVC on x86/ARM) implements
 *     it as an arithmetic (sign-extending) shift, matching Ruby's floor
 *     semantics (`-8 << -1 == -4`) -- accepted as a documented platform
 *     assumption, the same class of "implementation-defined but
 *     universally consistent in practice" already relied on elsewhere in
 *     this runtime.
 *   - `amount > 0`: LEFT shift, no bignum growth (unlike real Ruby, which
 *     grows arbitrarily -- `1 << 63 == 9223372036854775808`, one past this
 *     runtime's `INT64_MAX`), so this SATURATES at `INT64_MAX`/`INT64_MIN`
 *     once the true mathematical result would not fit, rather than
 *     silently wrapping -- the same never-raise-by-saturating convention
 *     `round`/`gcd`/`abs` already use. The overflow check happens BEFORE
 *     any left-shift is performed on the magnitude (a raw `mag << k` where
 *     `k` approaches 64 risks shifting bits out of a 64-bit register,
 *     which is well-defined for `uint64_t` -- shifts wrap-discard rather
 *     than trap -- but would silently lose the very bits this check needs
 *     to notice), so the "would this overflow" test is done with a
 *     right-shift-and-compare rather than by inspecting the (already
 *     lossy) left-shifted result.
 *   - A shift amount whose magnitude is `>= 64` (either direction) drains
 *     every bit: saturates to 0/-1 (right) or `INT64_MAX`/`INT64_MIN`
 *     (left, `n != 0`) rather than reaching a C shift-amount-exceeds-width
 *     UB (shifting by `>= 64` on a 64-bit type is UB in C, so this always
 *     shifts by a checked, in-range amount).
 */
static int64_t _sir_shift_left_i64(int64_t n, int64_t amount) {
    if (amount == 0 || n == 0) return n;
    if (amount < 0) {
        uint64_t k = _sir_i64_abs_u(amount);
        if (k >= 64) return (n < 0) ? -1 : 0;
        return n >> (int)k;
    }
    {
        uint64_t k = (uint64_t)amount;
        int neg = n < 0;
        uint64_t mag;
        if (k >= 64) return neg ? INT64_MIN : INT64_MAX;
        mag = _sir_i64_abs_u(n);
        if ((mag >> (64 - k)) != 0) return neg ? INT64_MIN : INT64_MAX;
        {
            uint64_t shifted = mag << k;
            uint64_t limit = neg ? ((uint64_t)INT64_MAX + 1u) : (uint64_t)INT64_MAX;
            if (shifted > limit) return neg ? INT64_MIN : INT64_MAX;
            if (neg) return (shifted == limit) ? INT64_MIN : -(int64_t)shifted;
            return (int64_t)shifted;
        }
    }
}

SirValue _sir_shift_left_v(SirValue *xs, int n) {
    int i;
    if (n <= 0) return _sir_int(0);
    if (xs[0].tag == SIR_SEQ) {
        for (i = 1; i < n; i++) _sir_array_push_one(xs[0].as.seq, xs[i]);
        return xs[0];
    }
    if (xs[0].tag == SIR_STR) {
        const char *acc = xs[0].as.s;
        for (i = 1; i < n; i++) acc = _sir_cat(acc, _sir_str_of(xs[i]));
        return _sir_str(acc);
    }
    {
        int64_t acc = (xs[0].tag == SIR_INT) ? xs[0].as.i : 0;
        for (i = 1; i < n; i++) acc = _sir_shift_left_i64(acc, _sir_shift_amount_arg(xs[i]));
        return _sir_int(acc);
    }
}
SirValue _sir_shift_left(int n, ...) {
    va_list ap; SirValue *xs; SirValue r;
    va_start(ap, n); xs = _sir_va_collect(n, ap); va_end(ap);
    r = _sir_shift_left_v(xs, n);
    if (xs) free(xs);
    return r;
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
        if (recv.tag == SIR_STR || recv.tag == SIR_SYM) return _sir_int((int64_t)strlen(recv.as.s));
        if (recv.tag == SIR_SEQ) return _sir_int(recv.as.seq->len);
        if (recv.tag == SIR_MAP) return _sir_int(recv.as.map->len);
    } else if (strcmp(m, "upcase") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_upcase(recv.as.s));
        /* Symbol#upcase re-interns the result as a fresh SYMBOL, not a
           String (Ruby: `:foo.upcase == :FOO`) -- the one arm here that
           can't just widen the String helper's return type. */
        if (recv.tag == SIR_SYM) return _sir_sym(_sir_str_upcase(recv.as.s));
    } else if (strcmp(m, "downcase") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_downcase(recv.as.s));
        if (recv.tag == SIR_SYM) return _sir_sym(_sir_str_downcase(recv.as.s));
    } else if (strcmp(m, "reverse") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_reverse(recv.as.s));
        if (recv.tag == SIR_SEQ) return _sir_array_reverse(recv.as.seq);
    } else if (strcmp(m, "empty?") == 0) {
        if (recv.tag == SIR_STR || recv.tag == SIR_SYM) return _sir_bool(recv.as.s[0] == '\0');
        if (recv.tag == SIR_SEQ) return _sir_bool(recv.as.seq->len == 0);
        if (recv.tag == SIR_MAP) return _sir_bool(recv.as.map->len == 0);
    } else if (strcmp(m, "to_s") == 0) {
        if (recv.tag == SIR_STR) return recv;  /* String#to_s is the string itself */
        if (recv.tag == SIR_SYM) return _sir_str(recv.as.s);  /* Symbol#to_s -- the bare name, no `:` */
    } else if (strcmp(m, "inspect") == 0) {
        if (recv.tag == SIR_SYM) return _sir_sym_inspect(recv.as.s);
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
        /* Deferred-from-slice-8: String#count(charset, ...) -- how many
           chars of `recv` lie in the (intersected) char-set argument(s). */
        if (recv.tag == SIR_STR && argc >= 1) return _sir_str_count_charset(recv.as.s, argc, args);
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
        /* Deferred-from-slice-8: String#delete(charset, ...) -- remove
           every char in the (intersected) char-set argument(s). */
        if (recv.tag == SIR_STR && argc >= 1)
            return _sir_str(_sir_str_delete_charset(recv.as.s, argc, args));
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
        /* A Float receiver must NOT reach the generic `_sir_to_i` here: it
           bottoms out in a bare `(int64_t)v.as.f` cast (`_sir_as_int`,
           pre-existing, used by many unrelated call sites this PR does not
           touch), UB for a non-finite or out-of-int64-range Float -- the
           SAME hazard `floor`/`ceil`/`round` already guard against just
           below. Routed through the same saturating helper instead. */
        if (recv.tag == SIR_FLOAT) return _sir_int(_sir_f64_to_i64_saturating(recv.as.f));
        if (recv.tag == SIR_INT) return _sir_to_i(recv);
    } else if (strcmp(m, "to_f") == 0) {
        if (recv.tag == SIR_STR) return _sir_float(_sir_str_to_f(recv.as.s));
        if (_sir_is_num(recv)) return _sir_to_f(recv);
    } else if (strcmp(m, "to_sym") == 0) {
        if (recv.tag == SIR_STR) return _sir_sym(recv.as.s);
        if (recv.tag == SIR_SYM) return recv;  /* Symbol#to_sym is the identity */
    } else if (strcmp(m, "tr") == 0) {
        if (recv.tag == SIR_STR && argc == 2 && args[0].tag == SIR_STR && args[1].tag == SIR_STR)
            return _sir_str(_sir_str_tr(recv.as.s, args[0].as.s, args[1].as.s));
    }
    /* Deferred-from-slice-8: char-set methods (`count`/`delete`/`squeeze`)
     * and padding methods (`ljust`/`rjust`/`center`). `count`/`delete`
     * share their names with slice 3/6's Array#count and Hash#delete --
     * merged into THOSE existing `else if` arms below (a SECOND `else if`
     * on the same method name in this if/else-if chain would be dead code:
     * the first match wins regardless of whether its body returns, so a
     * later arm for the same `strcmp` never runs). `squeeze` has no
     * existing arm, so it gets its own. */
    else if (strcmp(m, "squeeze") == 0) {
        if (recv.tag == SIR_STR) return _sir_str(_sir_str_squeeze(recv.as.s, argc, args));
    } else if (strcmp(m, "ljust") == 0 || strcmp(m, "rjust") == 0 || strcmp(m, "center") == 0) {
        if (recv.tag == SIR_STR && argc >= 1 && _sir_is_num(args[0])) {
            int64_t width = _sir_str_width_arg(args[0]);
            const char *pad = (argc > 1 && args[1].tag == SIR_STR && args[1].as.s[0] != '\0')
                                   ? args[1].as.s
                                   : " ";
            int mode = (strcmp(m, "ljust") == 0) ? 0 : (strcmp(m, "rjust") == 0) ? 1 : 2;
            return _sir_str(_sir_str_justify(recv.as.s, width, pad, mode));
        }
    }
    /* Collections slice 9: Numeric methods. */
    else if (strcmp(m, "abs") == 0) {
        /* Same `-INT64_MIN` hazard `_sir_i64_abs_u`'s doc comment describes
           (confirmed to misbehave under optimization) -- reused here rather
           than a bare unary `-`, saturating to INT64_MAX for the one input
           whose true magnitude (2^63) doesn't fit (this runtime has no
           bignum), matching `gcd`'s convention for the same situation. */
        if (recv.tag == SIR_INT) {
            uint64_t u = _sir_i64_abs_u(recv.as.i);
            return _sir_int(u > (uint64_t)INT64_MAX ? INT64_MAX : (int64_t)u);
        }
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
        /* `INT64_MIN - 1` is signed-overflow UB (confirmed to wrap to
           INT64_MAX in practice, the opposite end of the range) -- there is
           no smaller representable Integer, so this saturates at the floor
           rather than wrapping, the same never-raise convention every other
           fix in this slice uses. */
        if (recv.tag == SIR_INT) return _sir_int(recv.as.i == INT64_MIN ? INT64_MIN : recv.as.i - 1);
    } else if (strcmp(m, "floor") == 0) {
        if (_sir_is_num(recv)) return _sir_num_floor(recv);
    } else if (strcmp(m, "ceil") == 0) {
        if (_sir_is_num(recv)) return _sir_num_ceil(recv);
    } else if (strcmp(m, "round") == 0) {
        if (_sir_is_num(recv) && argc == 0) return _sir_num_round(recv);
        if (_sir_is_num(recv) && argc == 1 && _sir_is_num(args[0]))
            return _sir_num_round_ndigits(recv, _sir_round_ndigits_arg(args[0]));
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
    /* Collections slice 10: universal Object methods + Bool operators. */
    else if (strcmp(m, "nil?") == 0) {
        return _sir_bool(recv.tag == SIR_NIL);
    } else if (strcmp(m, "equal?") == 0) {
        if (argc == 1) return _sir_object_equal_p(recv, args[0]);
    } else if (strcmp(m, "itself") == 0) {
        if (argc == 0) return recv;
    } else if (strcmp(m, "frozen?") == 0) {
        if (argc == 0) return _sir_object_frozen_p(recv);
    } else if (strcmp(m, "&") == 0) {
        /* TrueClass/FalseClass#& -- EAGER (both operands are already-computed
           SirValues by the time a __method__ dispatch reaches here), Ruby-
           truthiness-coercing logical AND. Distinct from the SHORT-CIRCUITING
           `&&`, which the frontend lowers to `If`, never to a method call. */
        if (recv.tag == SIR_BOOL && argc == 1)
            return _sir_bool(_sir_truthy(recv) && _sir_truthy(args[0]));
    } else if (strcmp(m, "|") == 0) {
        if (recv.tag == SIR_BOOL && argc == 1)
            return _sir_bool(_sir_truthy(recv) || _sir_truthy(args[0]));
    } else if (strcmp(m, "^") == 0) {
        if (recv.tag == SIR_BOOL && argc == 1)
            return _sir_bool(_sir_truthy(recv) != _sir_truthy(args[0]));
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

/* `puts`'s ARRAY-UNPACKING rule -- distinct from every OTHER display path
 * here (`print`, `_sir_fmt`'s general case, `_sir_fmt_seq` nested inside a
 * larger structure), which all bracket-display a Seq (`[1, 2, 3]`). Real
 * Ruby's `Kernel#puts` special-cases an Array argument: each element gets
 * its OWN line, RECURSIVELY flattening nested arrays, and an EMPTY array
 * prints nothing at all (not even a blank line) -- `puts [1, [2, 3], 4]` ->
 * "1\n2\n3\n4\n"; `puts []` -> (nothing); `puts [[]]` -> (nothing, the
 * empty nested array also contributes zero lines). A Hash argument is NOT
 * unpacked (only Array gets this treatment), so this checks `SIR_SEQ`
 * specifically, not any container tag. Shares `_sir_fmt`'s depth counter/
 * cap (`_sir_fmt_depth`/`SIR_MAX_FMT_DEPTH`, just above) so a
 * self-referential array (`a[0] = a`) terminates instead of recursing
 * forever, matching the safety floor every other display path here holds. */
static void _sir_puts_one(FILE *out, SirValue v) {
    if (v.tag == SIR_SEQ) {
        int64_t i;
        if (_sir_fmt_depth > SIR_MAX_FMT_DEPTH) {
            fputs("[...]\n", out);
            return;
        }
        _sir_fmt_depth++;
        for (i = 0; i < v.as.seq->len; i++) _sir_puts_one(out, v.as.seq->items[i]);
        _sir_fmt_depth--;
        return;
    }
    _sir_fmt(out, v);
    fputc('\n', out);
}

/* SIR28 §2.1: `__sys_write__`, the general console-output primitive every
 * frontend lowers `print`/`puts`/`console.log`/etc. to.  It generalizes
 * what used to be several backend-hardcoded newline policies into ONE
 * operation parameterized by policy flags carried as DATA (validated by
 * `semantic-ir`'s validator against a closed enum, SIR28 §2.2) -- the root
 * cause SIR28 exists to fix: real Ruby's `print` never newline-terminates,
 * Python's `print()`/JS's `console.log` always do, but before SIR28 all
 * three lowered to the identical `BuiltinCall("print", ...)` this backend
 * had no way to tell apart.
 *
 * `terminator`: 0 = none (write each value's display form back to back, no
 * newline -- matches Ruby's `print`), 1 = per_value (one newline per value,
 * honouring `unpack_arrays` -- matches Ruby's `puts`), 2 = once (Python
 * `print`/JS `console.log` -- space-join every value, one trailing
 * newline).  `unpack_arrays` is only consulted under `terminator == 1`,
 * matching SIR28 §2.1's table exactly. */
SirValue _sir_write_v(FILE *out, SirValue *xs, int n, int terminator, int unpack_arrays) {
    int i;
    switch (terminator) {
        case 1: /* per_value ("puts") */
            if (n <= 0) { fputc('\n', out); return _sir_nil(); }
            for (i = 0; i < n; i++) {
                if (unpack_arrays) {
                    _sir_puts_one(out, xs[i]);
                } else {
                    _sir_fmt(out, xs[i]);
                    fputc('\n', out);
                }
            }
            return _sir_nil();
        case 2: /* once ("print"/"console.log") */
            for (i = 0; i < n; i++) {
                if (i > 0) fputc(' ', out);
                _sir_fmt(out, xs[i]);
            }
            fputc('\n', out);
            return _sir_nil();
        default: /* none ("print") */
            for (i = 0; i < n; i++) _sir_fmt(out, xs[i]);
            return _sir_nil();
    }
}
/* `stream`: 0 = stdout, 1 = stderr.  Both `stream` and `terminator` are
 * compile-time constants baked in by the emitter from a validated `StrLit`
 * (never source-derived text reaching a dynamic file-handle lookup). */
SirValue _sir_write(int stream, int terminator, int unpack_arrays, int n, ...) {
    va_list ap; SirValue *xs; SirValue r;
    FILE *out = stream ? stderr : stdout;
    va_start(ap, n);
    xs = _sir_va_collect(n, ap);
    va_end(ap);
    r = _sir_write_v(out, xs, n, terminator, unpack_arrays);
    if (xs) free(xs);
    return r;
}

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
    /* `"<<"` here is Ruby's polymorphic shift operator (Array push/String
       concat/saturating-Integer-shift) -- matching `variadic_helper`'s
       `"<<" => "_sir_shift_left"` mapping in emit.rs, for a builtin
       referenced BY NAME (`Scope::Builtin`, e.g. a hypothetical
       `arr.reduce(:<<)`) rather than emitted inline. `"c<<"` is the C
       frontend's OWN raw bitwise left shift -- kept as a DISTINCT name so
       the two never collide (the bug this split fixes: both frontends
       used to share the bare `"<<"` name, so the C-emit-time dispatch had
       to pick ONE meaning and silently applied Ruby's saturating semantics
       to C-sourced shifts too -- see this crate's CHANGELOG). */
    if (strcmp(name, "<<") == 0)       return _sir_shift_left_v(args, argc);
    if (strcmp(name, "c<<") == 0)      return _sir_shl(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, ">>") == 0)       return _sir_shr(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "u>>") == 0)      return _sir_lshr(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "tmod") == 0)     return _sir_itmod(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "utmod") == 0)    return _sir_utmod(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    /* SIR21 T3b-2: div_floor is `_sir_divide_v` under a new name (identical
       floor-int/true-divide-float dispatch, zero new logic). div_trunc/
       udiv_trunc are the OLD `tdiv`/`utdiv` names under their new canonical
       spelling -- the bare `"tdiv"`/`"utdiv"` dispatch entries that used to
       be here are removed (Slice 7 cleanup) now that their only emitter
       (`c-to-semantic-ir`) has migrated (Slice 6); `_sir_itdiv`/
       `_sir_utdiv` themselves stay, since these entries still call them.
       div_true is genuinely new (see `_sir_true_div`'s own doc comment). */
    if (strcmp(name, "div_floor") == 0)  return _sir_divide_v(args, argc);
    if (strcmp(name, "div_trunc") == 0)  return _sir_itdiv(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "udiv_trunc") == 0) return _sir_utdiv(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "div_true") == 0)   return _sir_true_div(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "cons") == 0)     return _sir_cons(_sir_arg(args, argc, 0), _sir_arg(args, argc, 1));
    if (strcmp(name, "car") == 0)      return _sir_car(_sir_arg(args, argc, 0));
    if (strcmp(name, "cdr") == 0)      return _sir_cdr(_sir_arg(args, argc, 0));
    if (strcmp(name, "null?") == 0)    return _sir_is_null(_sir_arg(args, argc, 0));
    if (strcmp(name, "pair?") == 0)    return _sir_is_pair(_sir_arg(args, argc, 0));
    if (strcmp(name, "number?") == 0)  return _sir_is_number(_sir_arg(args, argc, 0));
    if (strcmp(name, "symbol?") == 0)  return _sir_is_symbol(_sir_arg(args, argc, 0));
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

/* ============================================================
 * SIR22 array/matrix domain (Phase A Slice 2, second-wave backend
 * rollout — see the SIR22 spec's "Backend impact" section)
 * ============================================================
 *
 * `ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet` (and
 * `Stmt::IndexSet`) — the SIR22 "base cut" — lower to calls into the
 * `_sir_array_*` helpers below: an inlined port of
 * `semantic-ir-to-javascript`'s own already-proven `ArrayRt` sub-runtime
 * (itself a plain-JS port of the published `@coding-adventures/
 * sir-runtime-array` package), following this crate's existing
 * inlined-runtime convention (this backend always inlines, unlike the
 * TS/Python imported-package model). The 9-node SIR22 "APL addendum"
 * (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/
 * `IndexOf`/`Ravel`/`Catenate`, Phase A Slice 3) is implemented further
 * below, in its own "SIR22 addendum: APL primitive operators" section —
 * ported 1:1 from the SAME `semantic-ir-to-javascript` reference's own
 * addendum section (`runtime.rs`'s `reduce`/`scan`/`outer`/`shape`/
 * `reshape`/`indexGenerator`/`indexOf`/`ravel`/`catenate`), adapted to
 * this file's rank-0-or-2-only value model (see "Vector representation
 * in the addendum" below for exactly how).
 *
 * ## Vector representation in the addendum
 *
 * The JS/Ruby references have a genuine RANK-1 shape (`shape.length ===
 * 1`) that several addendum functions produce (`shape`/`ravel`/
 * `indexGenerator`/`catenate`'s vector cases/`reduce`'s matrix-branch
 * output) and others dispatch on (`outer`/`indexOf`/`catenate` all branch
 * on true rank 0 vs 1 vs 2). This file has no rank-1 at all (see "Value
 * model" above) — by design, since the base cut's own constructors never
 * needed one. Extending that design forward: every addendum function
 * that would produce a JS/Ruby rank-1 "vector" here constructs a `1 x n`
 * RANK-2 row instead (`_sir_array_new_matrix(1, n, …)`), exactly
 * mirroring `_sir_array_range`'s and `_sir_array_index_get`'s own
 * existing "vector = row" convention — so a caller who then does a
 * single-scalar-argument `IndexGet` (linear indexing) on an addendum
 * result sees the identical layout a base-cut vector would give.
 *
 * For the INPUT side, `_sir_array_logical_rank` below classifies a
 * `SirNDArray` into this domain's three logical ranks: 0 (a true `rank ==
 * 0` scalar), 1 (`rank == 2` with `rows == 1` — this file's stand-in for
 * the references' genuine rank-1), or 2 (`rank == 2` with `rows != 1` —
 * an unambiguous "real" matrix, INCLUDING a `cols == 1` column shape).
 * Every addendum function that branches on rank in the references
 * (`outer`/`shape`/`reshape`/`indexOf`/`catenate`) uses this classifier.
 *
 * This is DELIBERATELY ASYMMETRIC between rows and cols, not "rows == 1
 * OR cols == 1": every vector this backend ever actually constructs —
 * `_sir_array_range`, `_sir_array_index_get`'s non-scalar 1-argument
 * result, and every addendum function below that produces a "vector"
 * result — is a `1 x n` ROW, never a column (see the paragraph above).
 * A `cols == 1, rows > 1` shape, by contrast, only ever arises from a
 * genuine 2-D construct this backend's OWN base cut already treats as a
 * real matrix (a literal column via `ArrayLit`, e.g. `[[5],[6]]`, or an
 * `IndexGet` whole-row/scalar-column selection) — and the JS/Ruby
 * references treat that SAME construct as a genuine rank-2 matrix too
 * (their own `ArrayLit`/`fromRows` never produce a rank-1 result either;
 * see the base cut's own module doc above), so classifying it as
 * logical rank 2, not 1, keeps this port's dispatch faithful to the
 * references for every shape this backend can actually produce — not
 * merely a "most useful" judgment call, but the reading that actually
 * matches JS/Ruby behaviour for every reachable input. (`_sir_array_
 * logical_rank`'s own doc comment notes the earlier, symmetric version
 * of this rule this replaced, and the `catenate` matrix-matrix case it
 * silently broke.)
 *
 * `reduce`/`scan` need no such classification at all: their "fold/scan
 * each row across its columns" loop already generalizes correctly to a
 * 1-row or 1-column input without a special case (folding a single row
 * of `n` columns IS folding the whole vector; folding `n` rows of a
 * single column each is a correct no-op, matching the references' own
 * rank-2 branch exactly).
 *
 * `emit.rs`'s pre-emit scan (`scan_expr_for_builtin`) no longer rejects
 * these nine node kinds — that dedicated rejection arm, present only
 * through Slice 2, is removed now that real `emit_expr` arms exist for
 * all nine (mirroring the base cut's own treatment).
 *
 * ## Value model
 *
 * `SirNDArray { rank, rows, cols, data, data_len }` — dense, rectangular,
 * COLUMN-MAJOR storage (Fortran/MATLAB order), mirroring
 * `array_runtime::value::Array` field-for-field. `rank == 0` is a scalar
 * (`data_len == 1`, `rows`/`cols` unused); `rank == 2` is a matrix
 * (`rows` x `cols`, `data_len == rows * cols`) — this port's whole scope,
 * like the JS/TS/Ruby references, is rank <= 2. (A JS/Ruby-style
 * "rank-1 vector" shape is never actually produced by any base-cut
 * constructor here: `range`/`indexGet`'s 1-argument form both return a
 * `1 x n` RANK-2 row, and `fromRows` is always 2-D — so this port omits a
 * rank-1 representation entirely rather than carrying dead code for a
 * shape nothing constructs.)
 *
 * Unlike the Ruby port (which preserves native Integer/Float propagation
 * through `+`/`-`/`*`, only forcing Float for `Div`/`Pow`), this C port
 * follows the JS reference's ACTUAL internal representation: every
 * element is a plain C `double`, unconditionally — not JS's cosmetic
 * integer-display shortcut (`19` instead of `19.0`), just its real
 * `Float64Array` storage. `SirValue` here is statically tagged
 * (`SIR_INT` xor `SIR_FLOAT`), so an element read off an NDArray always
 * becomes `_sir_float(d)`, and an all-integer computation like a 2x2
 * `matmul` therefore prints WITH a trailing ".0" (`_sir_fmt_float`'s
 * existing convention for every other Float in this backend), unlike the
 * Ruby port's int-preserving `19`. This is the simplest correct choice
 * for C's statically-tagged value model: it avoids a second int/float
 * dispatch path through `_sir_array_apply_op` (which the Ruby port needs
 * precisely because Ruby's `Numeric` types aren't statically tracked),
 * at the cost of a cosmetic ".0" no reference backend's tests need to
 * match (each backend's own `tests/sir22_array.rs` asserts against ITS
 * OWN emitted output).
 *
 * ## The C-specific overflow hazard neither JS nor Ruby has
 *
 * JS `Number`s are IEEE doubles (lose precision silently past 2^53, but
 * never WRAP), and Ruby `Integer` is arbitrary-precision (never wraps,
 * never overflows) — so `rows * cols` computing an element count is safe
 * in both references without an explicit overflow check. C's `int64_t`
 * has neither property: `rows * cols` can silently wrap (undefined
 * behaviour, in fact, for a signed overflow) BEFORE it is ever compared
 * against the `SIR_ARRAY_MAX_ELEMENTS` cap, turning a huge requested
 * shape into a small (or negative, or UB) computed size and then
 * `malloc`ing too little for what the caller believes it got — a classic
 * heap-overflow setup. `_sir_array_checked_size` below therefore checks
 * `rows > SIR_ARRAY_MAX_ELEMENTS / cols` (rejecting) BEFORE ever
 * computing `rows * cols`, not after — every allocation in this file
 * routes through it (or a caller that already routed its own inputs
 * through it) before calling `_sir_alloc`.
 */

/* SECURITY: every constructor below validates a shape/output size BEFORE
 * allocating — a compiled program's array sizes come from potentially
 * attacker-influenced runtime values (loop counts, parsed input, ...), not
 * fixed compile-time constants, so an unbounded or malformed shape must fail
 * cleanly (a `stderr` message + `exit(1)`, matching this file's existing
 * `_sir_seq_set`/`_sir_divide_v` "trap" convention) rather than let a huge or
 * wrapped `malloc` size corrupt the heap or exhaust memory. Mirrors the JS
 * backend's own `MAX_ELEMENTS` bound exactly, so the cap is identical across
 * every backend that ports this runtime. */
#define SIR_ARRAY_MAX_ELEMENTS ((int64_t)1 << 26) /* 67,108,864 */

/* Tolerance for the inclusive-stop boundary check in `_sir_array_range`,
 * matching `matlab-runtime`'s own `eval_colon` and the JS/Ruby ports exactly
 * — a floating step (e.g. `1:0.1:2`) can drift a few ULPs short of `stop` by
 * the final iteration, and MATLAB's `a:step:b` is inclusive of `b`. */
#define SIR_ARRAY_RANGE_EPSILON 1e-9

struct SirNDArray {
    int rank;            /* 0 (scalar) or 2 (matrix) — see the module doc above */
    int64_t rows, cols;  /* valid only when rank == 2 */
    double *data;        /* `data_len` doubles, column-major when rank == 2 */
    int64_t data_len;
};

/* Extract a `double` out of a numeric `SirValue`, failing loudly (not
 * silently coercing to 0.0 the way this file's general-purpose `_sir_as_num`
 * does) on a non-numeric value — an ArrayLit element or Range bound that
 * turns out to be, say, a String is a genuine program error in this domain,
 * not a value this runtime should silently zero out. */
double _sir_array_num_of(SirValue v, const char *ctx) {
    if (v.tag == SIR_INT)   return (double)v.as.i;
    if (v.tag == SIR_FLOAT) return v.as.f;
    fprintf(stderr, "sir: %s: expected a number, got a non-numeric value\n", ctx);
    exit(1);
    return 0.0; /* unreachable; silences -Wreturn-type on toolchains that don't know exit() is noreturn */
}

/* Checked shape-size computation for a `rows` x `cols` matrix — see the
 * module doc's "C-specific overflow hazard" section for why the overflow
 * check happens BEFORE the multiply, not after. `rows`/`cols` are also
 * range-checked non-negative (a negative dimension is nonsensical and would
 * otherwise defeat the overflow check's own division). */
int64_t _sir_array_checked_size(int64_t rows, int64_t cols, const char *ctx) {
    if (rows < 0 || cols < 0) {
        fprintf(stderr, "sir: %s: shape (%lld, %lld) has a negative dimension\n",
                ctx, (long long)rows, (long long)cols);
        exit(1);
    }
    if (cols != 0 && rows > SIR_ARRAY_MAX_ELEMENTS / cols) {
        fprintf(stderr, "sir: %s: shape (%lld, %lld) exceeds the %lld-element cap\n",
                ctx, (long long)rows, (long long)cols, (long long)SIR_ARRAY_MAX_ELEMENTS);
        exit(1);
    }
    {
        int64_t n = rows * cols;
        if (n > SIR_ARRAY_MAX_ELEMENTS) {
            fprintf(stderr, "sir: %s: shape (%lld, %lld) (%lld elements) exceeds the %lld-element cap\n",
                    ctx, (long long)rows, (long long)cols, (long long)n, (long long)SIR_ARRAY_MAX_ELEMENTS);
            exit(1);
        }
        return n;
    }
}

/* Construct a rank-2 `rows` x `cols` NDArray from an already-populated
 * `data` buffer of exactly `rows * cols` doubles (the caller's
 * responsibility — every call site below allocates `data` via
 * `_sir_array_checked_size`'s own return value first). Re-validates the
 * shape (cheap, and keeps "every NDArray that exists passed validation" a
 * true invariant regardless of which constructor built it, not just the
 * ones that remembered to check). */
SirValue _sir_array_new_matrix(int64_t rows, int64_t cols, double *data, const char *ctx) {
    SirNDArray *a;
    (void)_sir_array_checked_size(rows, cols, ctx);
    a = (SirNDArray *)_sir_alloc(sizeof(SirNDArray));
    a->rank = 2; a->rows = rows; a->cols = cols; a->data = data; a->data_len = rows * cols;
    { SirValue v; v.tag = SIR_ARRAY; v.as.arr = a; return v; }
}

/* Construct a rank-0 scalar NDArray wrapping one `double`. */
SirValue _sir_array_scalar(double x) {
    SirNDArray *a = (SirNDArray *)_sir_alloc(sizeof(SirNDArray));
    double *data = (double *)_sir_alloc(sizeof(double));
    data[0] = x;
    a->rank = 0; a->rows = 0; a->cols = 0; a->data = data; a->data_len = 1;
    { SirValue v; v.tag = SIR_ARRAY; v.as.arr = a; return v; }
}

/* Construct a new NDArray with the SAME shape as `shape_src` but a fresh
 * `data` buffer (already populated by the caller with `shape_src->data_len`
 * doubles) — used by `_sir_array_elementwise`'s scalar-broadcast branches,
 * whose result shape is whichever operand was non-scalar. */
SirValue _sir_array_new_like(const SirNDArray *shape_src, double *data) {
    SirNDArray *a = (SirNDArray *)_sir_alloc(sizeof(SirNDArray));
    a->rank = shape_src->rank; a->rows = shape_src->rows; a->cols = shape_src->cols;
    a->data = data; a->data_len = shape_src->data_len;
    { SirValue v; v.tag = SIR_ARRAY; v.as.arr = a; return v; }
}

/* Rows, treating a scalar as `1x1`. */
int64_t _sir_array_nrows(const SirNDArray *a) { return a->rank == 0 ? 1 : a->rows; }
/* Columns, treating a scalar as `1x1`. */
int64_t _sir_array_ncols(const SirNDArray *a) { return a->rank == 0 ? 1 : a->cols; }

/* Coerce a `SirValue` operand that must ALREADY be an NDArray (`transpose`,
 * `indexGet`/`indexSet`'s `target`) — unlike `_sir_array_coerce` below,
 * there is no bare-scalar fallback here, matching the JS/Ruby references
 * (neither calls `toArrayValue` on these operands either). */
SirNDArray *_sir_array_require(SirValue v, const char *ctx) {
    if (v.tag != SIR_ARRAY) {
        fprintf(stderr, "sir: %s: expected an NDArray\n", ctx);
        exit(1);
    }
    return v.as.arr;
}

/* Coerce a bare numeric `SirValue` into a rank-0 scalar NDArray; an
 * already-NDArray value passes through unchanged. Needed because
 * `matlab-to-semantic-ir`'s lowerer emits a mixed operand pair for
 * `.* ./ .\` and for `* /` when exactly one side is scalar (e.g. `A .* 2`)
 * — the BARE scalar sub-expression is passed through `ElementwiseOp`/
 * `MatMul` unwrapped (a plain `IntLit`/`FloatLit`/arithmetic result), not
 * wrapped in an `ArrayLit` first. `_sir_array_elementwise`/`_sir_array_matmul`
 * both normalize through this first, so a raw `SirValue` never reaches
 * `->data`/`->rows` and fails loudly (via `_sir_array_require`'s cousin
 * check below) instead of dereferencing a non-`SIR_ARRAY` union member. */
SirNDArray *_sir_array_coerce(SirValue v, const char *ctx) {
    if (v.tag == SIR_ARRAY) return v.as.arr;
    return _sir_array_scalar(_sir_array_num_of(v, ctx)).as.arr;
}

int _sir_array_is_scalar(const SirNDArray *a) { return a->data_len == 1; }

/* Element `(r, c)` (column-major). Returns 1 and writes `*out` on success, 0
 * (leaving `*out` untouched) if out of bounds.
 *
 * SECURITY: written in AND-form (`r >= 0 && c >= 0 && r < nrows && c <
 * ncols`), matching the JS/Ruby references' own explicit warning: the
 * negated OR-form (`r < 0 || c < 0 || ...`) is NOT equivalent under
 * IEEE-754 if `r`/`c` ever carried a NaN (every relational comparison with
 * NaN is false, so an OR-form check would have every branch evaluate false,
 * silently skipping the bounds check). `r`/`c` are always already-validated
 * `int64_t` positions by the time they reach this function (never a raw
 * `double`), so integers can't be NaN and this specific call path is not
 * actually reachable with one — kept in AND-form anyway for the same
 * "don't rely on every future caller re-deriving the invariant" discipline
 * the JS `set` doc comment states, at zero extra cost either way. */
int _sir_array_get(const SirNDArray *a, int64_t r, int64_t c, double *out) {
    int64_t nr = _sir_array_nrows(a), nc = _sir_array_ncols(a);
    if (r >= 0 && c >= 0 && r < nr && c < nc) {
        *out = a->data[c * nr + r];
        return 1;
    }
    return 0;
}

/* Set element `(r, c)` IN PLACE (column-major) — mutates `a->data` directly,
 * matching MATLAB assignment semantics (`A(i,j) = v` rebinds one element of
 * the existing array, it does not produce a new one). This is why
 * `Stmt::IndexSet` is a statement, not a pure expression, in the SIR22 spec.
 * Returns 1 on success, 0 (no mutation) if out of bounds. Same AND-form
 * bounds check as `_sir_array_get`. */
int _sir_array_set(SirNDArray *a, int64_t r, int64_t c, double value) {
    int64_t nr = _sir_array_nrows(a), nc = _sir_array_ncols(a);
    if (r >= 0 && c >= 0 && r < nr && c < nc) {
        a->data[c * nr + r] = value;
        return 1;
    }
    return 0;
}

/* ── elementwise binary ops ─────────────────────────────────────────── */

/* Mirrors `ElementwiseOpKind` — `emit.rs`'s `elementwise_op_c_name` emits
 * exactly these constant names, so the two stay in lockstep by construction
 * (a Rust `match` over the same source enum, not a hand-maintained string
 * table the way the JS/Ruby ports' string-keyed dispatch needs). */
typedef enum {
    SIR_EW_ADD, SIR_EW_SUB, SIR_EW_MUL, SIR_EW_DIV, SIR_EW_POW,
    SIR_EW_MAX, SIR_EW_MIN, SIR_EW_EQ, SIR_EW_NE,
    SIR_EW_LT, SIR_EW_LE, SIR_EW_GE, SIR_EW_GT
} SirElementwiseOp;

/* Comparisons follow the same APL-style boolean convention
 * `array_runtime::BinOp` uses: `1.0` for true, `0.0` for false (never a
 * native C `_Bool`/`SIR_BOOL`), since the result must stay a plain array
 * element like every other value here. `Div` is ALWAYS a true float divide
 * (`a / b` over `double`s) — unlike this backend's OWN `_sir_divide_v`
 * (bare `/`), which FLOORS when both operands happen to be `SIR_INT`; that
 * integer-floor behaviour belongs to Ruby's `/`, not MATLAB's `./`, which
 * always real-divides. A zero divisor here silently produces IEEE
 * inf/nan (matching the JS/Ruby references' `applyOp` exactly), NOT this
 * file's OWN `_sir_true_div`, which fails loudly — that divergence is
 * deliberate: `_sir_true_div` models Python's `/` (raises), this models
 * MATLAB's `./` over `double`s (never raises, produces `Inf`/`NaN`, which
 * MATLAB itself does too). */
double _sir_array_apply_op(SirElementwiseOp op, double a, double b) {
    switch (op) {
        case SIR_EW_ADD: return a + b;
        case SIR_EW_SUB: return a - b;
        case SIR_EW_MUL: return a * b;
        case SIR_EW_DIV: return a / b;
        case SIR_EW_POW: return pow(a, b);
        case SIR_EW_MAX: return a > b ? a : b;
        case SIR_EW_MIN: return a < b ? a : b;
        case SIR_EW_EQ:  return a == b ? 1.0 : 0.0;
        case SIR_EW_NE:  return a != b ? 1.0 : 0.0;
        case SIR_EW_LT:  return a < b  ? 1.0 : 0.0;
        case SIR_EW_LE:  return a <= b ? 1.0 : 0.0;
        case SIR_EW_GE:  return a >= b ? 1.0 : 0.0;
        case SIR_EW_GT:  return a > b  ? 1.0 : 0.0;
    }
    fprintf(stderr, "sir: _sir_array_apply_op: unrecognised op %d\n", (int)op);
    exit(1);
    return 0.0; /* unreachable */
}

/* Elementwise binary op with scalar broadcasting. Either operand may be a
 * scalar; otherwise the shapes must match exactly (full NumPy/MATLAB
 * broadcasting is out of scope, same as the Rust/JS/Ruby references).
 * Result takes the non-scalar operand's shape (or the scalar's, if both
 * are). Normalizes both operands through `_sir_array_coerce` first — see
 * that function's doc for the bare-scalar-operand regression this guards. */
SirValue _sir_array_elementwise(SirElementwiseOp op, SirValue av, SirValue bv) {
    SirNDArray *a = _sir_array_coerce(av, "elementwise");
    SirNDArray *b = _sir_array_coerce(bv, "elementwise");
    if (_sir_array_is_scalar(a)) {
        int64_t n = b->data_len, i;
        double *data = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
        for (i = 0; i < n; i++) data[i] = _sir_array_apply_op(op, a->data[0], b->data[i]);
        return _sir_array_new_like(b, data);
    }
    if (_sir_array_is_scalar(b)) {
        int64_t n = a->data_len, i;
        double *data = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
        for (i = 0; i < n; i++) data[i] = _sir_array_apply_op(op, a->data[i], b->data[0]);
        return _sir_array_new_like(a, data);
    }
    if (a->rank != b->rank || a->rows != b->rows || a->cols != b->cols) {
        fprintf(stderr, "sir: elementwise: non-conformable arrays: (%lld, %lld) vs (%lld, %lld)\n",
                (long long)_sir_array_nrows(a), (long long)_sir_array_ncols(a),
                (long long)_sir_array_nrows(b), (long long)_sir_array_ncols(b));
        exit(1);
    }
    {
        int64_t n = a->data_len, i;
        double *data = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
        for (i = 0; i < n; i++) data[i] = _sir_array_apply_op(op, a->data[i], b->data[i]);
        return _sir_array_new_like(a, data);
    }
}

/* Matrix product `[m, k] . [k, n] -> [m, n]` (column-major throughout). `m`
 * and `n` come from two INDEPENDENT operands (each individually under
 * `SIR_ARRAY_MAX_ELEMENTS`, but their product isn't bounded by that alone —
 * an outer-product-shaped call could still ask for a huge output), so
 * `_sir_array_checked_size` (inside `_sir_array_new_matrix`, via the
 * pre-check below) validates `[m, n]` BEFORE allocating `out`, not after.
 * Normalizes both operands through `_sir_array_coerce` first, same
 * reasoning as `_sir_array_elementwise`. */
SirValue _sir_array_matmul(SirValue av, SirValue bv) {
    SirNDArray *a = _sir_array_coerce(av, "matmul");
    SirNDArray *b = _sir_array_coerce(bv, "matmul");
    int64_t m = _sir_array_nrows(a), ka = _sir_array_ncols(a);
    int64_t kb = _sir_array_nrows(b), n = _sir_array_ncols(b);
    int64_t out_len;
    double *out;
    if (ka != kb) {
        fprintf(stderr, "sir: matmul: inner dimensions disagree (%lldx%lld . %lldx%lld)\n",
                (long long)m, (long long)ka, (long long)kb, (long long)n);
        exit(1);
    }
    out_len = _sir_array_checked_size(m, n, "matmul");
    out = (out_len > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)out_len) : NULL;
    {
        int64_t j, i, p;
        for (j = 0; j < n; j++) {
            for (i = 0; i < m; i++) {
                double acc = 0.0;
                for (p = 0; p < ka; p++) {
                    acc += a->data[p * m + i] * b->data[j * kb + p]; /* column-major indexing */
                }
                out[j * m + i] = acc;
            }
        }
    }
    return _sir_array_new_matrix(m, n, out, "matmul");
}

/* Matrix transpose. `conjugate` distinguishes MATLAB `'` (`1`) from `.'`
 * (`0`) — this runtime has no `Complex` value type yet (matching
 * `array-runtime`'s own real-only scope today), so a conjugate transpose of
 * real data is identical to a plain transpose; `conjugate` is accepted only
 * for call-shape parity with the SIR spec. `target` must already be an
 * NDArray (`_sir_array_require`, no bare-scalar coercion — matching the
 * JS/Ruby references, neither of which calls `toArrayValue` here either). */
SirValue _sir_array_transpose(SirValue targetv, int conjugate) {
    SirNDArray *a = _sir_array_require(targetv, "transpose");
    int64_t m = _sir_array_nrows(a), n = _sir_array_ncols(a);
    int64_t len = a->data_len;
    double *out = (len > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)len) : NULL;
    int64_t i, j;
    (void)conjugate;
    for (j = 0; j < n; j++) {
        for (i = 0; i < m; i++) {
            out[i * n + j] = a->data[j * m + i];
        }
    }
    return _sir_array_new_matrix(n, m, out, "transpose");
}

/* Materialize a MATLAB-style range `start:step:stop` (default `step = 1`)
 * as a `1 x n` row vector — MATLAB's `:` always produces a row, never a
 * column. Bounded by `SIR_ARRAY_MAX_ELEMENTS` so a compiled program's
 * `1:1e18`-style range can't exhaust memory before this function ever gets
 * to materialize anything: a first COUNTING pass establishes exactly how
 * many elements the range needs (failing loudly past the cap) before any
 * allocation happens, then a second pass fills a buffer sized to that exact
 * count — no realloc-and-grow, and no risk of the two passes disagreeing,
 * since both walk the identical deterministic `x += step` sequence from the
 * same `start`. */
SirValue _sir_array_range(SirValue startv, SirValue stopv, SirValue stepv) {
    double start = _sir_array_num_of(startv, "range");
    double stop  = _sir_array_num_of(stopv, "range");
    double step  = _sir_array_num_of(stepv, "range");
    int64_t count;
    double x;
    double *data;
    if (step == 0.0) {
        fprintf(stderr, "sir: range: step cannot be zero\n");
        exit(1);
    }
    /* SECURITY: reject non-finite bounds up front — the loop condition below
     * is false on its very first check whenever start/stop/step is NaN
     * (every relational comparison with NaN is false), so an unguarded NaN
     * bound would silently produce an empty range instead of erroring, the
     * same "NaN defeats a comparison-based check" hazard class the index
     * resolution functions below also guard against. An unguarded
     * Infinity bound would instead loop until the element cap trips, which
     * is merely slow, not wrong — but is rejected here too for the same
     * "fail loudly, don't fall through to a confusing downstream state"
     * discipline the JS/Ruby references both apply uniformly to all three
     * bounds. */
    if (!isfinite(start) || !isfinite(stop) || !isfinite(step)) {
        fprintf(stderr, "sir: range: start/stop/step must be finite numbers, got (%.17g, %.17g, %.17g)\n",
                start, stop, step);
        exit(1);
    }
    count = 0;
    x = start;
    while ((step > 0 && x <= stop + SIR_ARRAY_RANGE_EPSILON) ||
           (step < 0 && x >= stop - SIR_ARRAY_RANGE_EPSILON)) {
        if (count >= SIR_ARRAY_MAX_ELEMENTS) {
            fprintf(stderr, "sir: range: produces more than %lld elements\n",
                    (long long)SIR_ARRAY_MAX_ELEMENTS);
            exit(1);
        }
        count++;
        x += step;
    }
    data = (count > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)count) : NULL;
    x = start;
    { int64_t i; for (i = 0; i < count; i++) { data[i] = x; x += step; } }
    return _sir_array_new_matrix(1, count, data, "range");
}

/* ── ArrayLit ────────────────────────────────────────────────────────── */

/* `[1 2; 3 4]` (`Expr::ArrayLit`) — `rows_in`/`cols_in` are the literal's
 * dimensions (known at COMPILE time from the SIR `rows: Vec<Vec<Expr>>`
 * shape; `emit.rs`'s pre-emit scan rejects a ragged literal cleanly before
 * this function is ever emitted, so `rows_in * cols_in` always equals the
 * `total` varargs actually passed). Elements arrive ROW-MAJOR (matching the
 * literal syntax and the SIR node's own field order) and are stored
 * COLUMN-MAJOR (`Feature::ArrayColumnMajor`, per the SIR22 spec's "Storage
 * convention"). `rows_in == 0` is the empty literal `[]` -> a `0x0` array,
 * matching the JS/Ruby references' own special case. */
SirValue _sir_array_from_rows(int64_t rows_in, int64_t cols_in, int total, ...) {
    int64_t n, r, c;
    double *data;
    va_list ap;
    (void)total;
    if (rows_in == 0) {
        return _sir_array_new_matrix(0, 0, NULL, "fromRows");
    }
    n = _sir_array_checked_size(rows_in, cols_in, "fromRows");
    data = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
    va_start(ap, total);
    for (r = 0; r < rows_in; r++) {
        for (c = 0; c < cols_in; c++) {
            SirValue e = va_arg(ap, SirValue);
            data[c * rows_in + r] = _sir_array_num_of(e, "fromRows"); /* column-major store */
        }
    }
    va_end(ap);
    return _sir_array_new_matrix(rows_in, cols_in, data, "fromRows");
}

/* ── indexing ────────────────────────────────────────────────────────── */
/* One MATLAB-style index-position argument, mirroring the SIR22 spec's
 * `IndexArg` exactly: `Scalar(value)` / `Whole` / `Range(indices)`. `end`-
 * relative indices are never seen here — per SIR10 discipline, the
 * frontend resolves `end` to a concrete 0-based `Scalar` index before
 * emitting `IndexGet`/`IndexSet`. */
typedef enum { SIR_IDXARG_SCALAR, SIR_IDXARG_WHOLE, SIR_IDXARG_RANGE } SirIndexArgKind;
typedef struct {
    SirIndexArgKind kind;
    SirValue value; /* SCALAR: the index value. RANGE: the NDArray of indices. WHOLE: unused (nil). */
} SirIndexArg;

SirIndexArg _sir_array_idx_scalar(SirValue v) { SirIndexArg a; a.kind = SIR_IDXARG_SCALAR; a.value = v; return a; }
SirIndexArg _sir_array_idx_whole(void)        { SirIndexArg a; a.kind = SIR_IDXARG_WHOLE;  a.value = _sir_nil(); return a; }
SirIndexArg _sir_array_idx_range(SirValue v)  { SirIndexArg a; a.kind = SIR_IDXARG_RANGE;  a.value = v; return a; }

typedef struct { int64_t *pos; int64_t n; } SirArrayPositions;

/* Validate one resolved position is a real, finite integer, and return it
 * as an `int64_t`.
 *
 * SECURITY: unlike this file's general-purpose `_sir_as_int` (used e.g. by
 * `ForRange`'s loop counters), which casts a `SIR_FLOAT`'s `double` to
 * `int64_t` UNCONDITIONALLY, this function validates BEFORE ever casting —
 * casting a NaN, or a finite-but-out-of-`int64_t`-range, `double` to
 * `int64_t` is UNDEFINED BEHAVIOUR in C (unlike JS's `Number.isInteger`
 * guard or Ruby's `Float#to_i`, which both fail cleanly instead). The
 * magnitude bound (+-9.2e18, comfortably inside `int64_t`'s actual range but
 * far enough from the exact boundary that the comparison itself can't be
 * fooled by `double`'s limited precision at that magnitude) is checked
 * FIRST, so the subsequent `(int64_t)d` cast only ever runs on a value
 * already known to be in range. Every index position this array domain
 * resolves funnels through this single choke point (mirroring the JS
 * reference's `assertValidPosition`), so this NaN-safety fix, once made
 * here, covers `indexGet`/`indexSet`/range-selector resolution uniformly. */
int64_t _sir_array_assert_valid_position(SirValue v, const char *ctx) {
    double d = _sir_array_num_of(v, ctx);
    if (!(d == d) || d < -9.2e18 || d > 9.2e18 || d != (double)(int64_t)d) {
        fprintf(stderr, "sir: %s: index %.17g is not a finite integer\n", ctx, d);
        exit(1);
    }
    return (int64_t)d;
}

/* Resolve one `SirIndexArg` against a dimension of size `dim_size` into a
 * flat list of 0-based positions along that dimension (arena-allocated,
 * never freed, like every heap value in this file). */
SirArrayPositions _sir_array_resolve_positions(SirIndexArg arg, int64_t dim_size) {
    SirArrayPositions r;
    switch (arg.kind) {
        case SIR_IDXARG_SCALAR: {
            int64_t *p = (int64_t *)_sir_alloc(sizeof(int64_t));
            p[0] = _sir_array_assert_valid_position(arg.value, "resolvePositions");
            r.pos = p; r.n = 1;
            return r;
        }
        case SIR_IDXARG_WHOLE: {
            int64_t *p = (dim_size > 0) ? (int64_t *)_sir_alloc(sizeof(int64_t) * (size_t)dim_size) : NULL;
            int64_t i;
            for (i = 0; i < dim_size; i++) p[i] = i;
            r.pos = p; r.n = dim_size;
            return r;
        }
        case SIR_IDXARG_RANGE: {
            SirNDArray *ra;
            int64_t n, i;
            int64_t *p;
            if (arg.value.tag != SIR_ARRAY) {
                fprintf(stderr, "sir: resolvePositions: a range index argument must be an NDArray\n");
                exit(1);
            }
            ra = arg.value.as.arr;
            n = ra->data_len;
            p = (n > 0) ? (int64_t *)_sir_alloc(sizeof(int64_t) * (size_t)n) : NULL;
            for (i = 0; i < n; i++) {
                /* `trunc` toward zero (matching JS's `Math.trunc` / Ruby's
                 * `Float#truncate`); safe to call on any double (IEEE `trunc`
                 * of NaN/Infinity is defined to yield NaN/Infinity back, no
                 * UB) — `_sir_array_assert_valid_position` below is what
                 * actually rejects a non-finite/non-integral result. */
                p[i] = _sir_array_assert_valid_position(_sir_float(trunc(ra->data[i])), "resolvePositions");
            }
            r.pos = p; r.n = n;
            return r;
        }
    }
    fprintf(stderr, "sir: resolvePositions: unrecognised IndexArg kind %d\n", (int)arg.kind);
    exit(1);
    r.pos = NULL; r.n = 0; /* unreachable */
    return r;
}

/* `A(i)` / `A(i, j)` (`Expr::IndexGet`) — read one element or a sub-array.
 * `n` (the argument count) is a COMPILE-TIME constant baked in by
 * `emit.rs` (`indices.len()`, from the SIR AST — never attacker-influenced
 * at runtime) — `emit.rs`'s pre-emit scan already rejects any module whose
 * `IndexGet`/`IndexSet` has other than 1 or 2 `IndexArg`s, so the `n != 1 &&
 * n != 2` check below is defense-in-depth for a hand-built `Module` that
 * bypassed that scan, not a path any compiled program can reach.
 *
 * A single argument indexes `target`'s underlying column-major data
 * LINEARLY (MATLAB's own single-subscript convention, which is
 * column-major too); two arguments index `(row, col)`. Returns a SCALAR
 * `SirValue` (`_sir_float`) when every argument is `Scalar` (a single
 * element), otherwise a fresh `SIR_ARRAY`. */
SirValue _sir_array_index_get(SirValue targetv, int n, ...) {
    SirNDArray *a = _sir_array_require(targetv, "indexGet");
    SirIndexArg args[2];
    va_list ap;
    if (n != 1 && n != 2) {
        fprintf(stderr, "sir: indexGet: only 1 or 2 index arguments are supported (rank <= 2 scope), got %d\n", n);
        exit(1);
    }
    va_start(ap, n);
    { int i; for (i = 0; i < n; i++) args[i] = va_arg(ap, SirIndexArg); }
    va_end(ap);

    if (n == 1) {
        SirArrayPositions ps = _sir_array_resolve_positions(args[0], a->data_len);
        if (args[0].kind == SIR_IDXARG_SCALAR) {
            int64_t i = ps.pos[0];
            if (i < 0 || i >= a->data_len) {
                fprintf(stderr, "sir: indexGet: linear index %lld out of bounds\n", (long long)i);
                exit(1);
            }
            return _sir_float(a->data[i]);
        }
        {
            int64_t out_len = _sir_array_checked_size(1, ps.n, "indexGet");
            double *data = (out_len > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)out_len) : NULL;
            int64_t k;
            for (k = 0; k < ps.n; k++) {
                int64_t i = ps.pos[k];
                if (i < 0 || i >= a->data_len) {
                    fprintf(stderr, "sir: indexGet: linear index %lld out of bounds\n", (long long)i);
                    exit(1);
                }
                data[k] = a->data[i];
            }
            return _sir_array_new_matrix(1, ps.n, data, "indexGet");
        }
    }
    {
        SirArrayPositions rows = _sir_array_resolve_positions(args[0], _sir_array_nrows(a));
        SirArrayPositions cols = _sir_array_resolve_positions(args[1], _sir_array_ncols(a));
        if (args[0].kind == SIR_IDXARG_SCALAR && args[1].kind == SIR_IDXARG_SCALAR) {
            double v;
            if (!_sir_array_get(a, rows.pos[0], cols.pos[0], &v)) {
                fprintf(stderr, "sir: indexGet: (%lld, %lld) out of bounds for shape (%lld, %lld)\n",
                        (long long)rows.pos[0], (long long)cols.pos[0],
                        (long long)_sir_array_nrows(a), (long long)_sir_array_ncols(a));
                exit(1);
            }
            return _sir_float(v);
        }
        /* SECURITY: `rows.n`/`cols.n` are each individually bounded by `a`'s
         * own dimensions (a `Whole` selector) or by a `Range` NDArray's own
         * `SIR_ARRAY_MAX_ELEMENTS`-checked construction — but nothing bounds
         * their PRODUCT on its own, the exact outer-product-shaped
         * allocation `_sir_array_matmul` guards against, one level up.
         * Validate before allocating, not after. */
        {
            int64_t out_len = _sir_array_checked_size(rows.n, cols.n, "indexGet");
            double *data = (out_len > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)out_len) : NULL;
            int64_t ci, ri;
            for (ci = 0; ci < cols.n; ci++) {
                for (ri = 0; ri < rows.n; ri++) {
                    double v;
                    if (!_sir_array_get(a, rows.pos[ri], cols.pos[ci], &v)) {
                        fprintf(stderr, "sir: indexGet: (%lld, %lld) out of bounds for shape (%lld, %lld)\n",
                                (long long)rows.pos[ri], (long long)cols.pos[ci],
                                (long long)_sir_array_nrows(a), (long long)_sir_array_ncols(a));
                        exit(1);
                    }
                    data[ci * rows.n + ri] = v;
                }
            }
            return _sir_array_new_matrix(rows.n, cols.n, data, "indexGet");
        }
    }
}

/* Broadcast a scalar-or-NDArray right-hand side to exactly `count` values
 * (mirrors `_sir_array_elementwise`'s scalar-broadcast rule). Returns a
 * buffer of `count` doubles — either a fresh fill (scalar / 1-element
 * NDArray source) or, when `value` already has exactly `count` elements,
 * the SAME buffer `value` owns (shared, not copied — matching the JS/Ruby
 * references' identical `return value.data`; safe here because every
 * caller only READS the returned buffer before the statement ends, never
 * retains it). */
double *_sir_array_broadcast_values(SirValue value, int64_t count, const char *ctx) {
    if (value.tag == SIR_INT || value.tag == SIR_FLOAT) {
        double x = _sir_array_num_of(value, ctx);
        double *out = (count > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)count) : NULL;
        int64_t i;
        for (i = 0; i < count; i++) out[i] = x;
        return out;
    }
    if (value.tag == SIR_ARRAY) {
        SirNDArray *v = value.as.arr;
        if (v->data_len == 1) {
            double x = v->data[0];
            double *out = (count > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)count) : NULL;
            int64_t i;
            for (i = 0; i < count; i++) out[i] = x;
            return out;
        }
        if (v->data_len != count) {
            fprintf(stderr, "sir: %s: value has %lld elements, expected %lld\n",
                    ctx, (long long)v->data_len, (long long)count);
            exit(1);
        }
        return v->data;
    }
    fprintf(stderr, "sir: %s: value must be a number or NDArray\n", ctx);
    exit(1);
    return NULL; /* unreachable */
}

/* `A(i) = v` / `A(i, j) = v` (`Stmt::IndexSet`) — write one element or a
 * sub-array, IN PLACE (see `_sir_array_set`'s doc comment above for why
 * this mutates rather than returns a new array). `value` may be a scalar
 * (broadcast to every selected position) or an NDArray with exactly as
 * many elements as positions are selected. `value` is passed BEFORE the
 * variadic index-argument pack (C requires `...` to be the LAST parameter,
 * so it can't sit between `target` and the indices the way the SIR node's
 * own `target, indices, value` field order would suggest) — `emit.rs`
 * documents this reordering at its one call site. Same `n != 1 && n != 2`
 * defense-in-depth as `_sir_array_index_get`. */
void _sir_array_index_set(SirValue targetv, SirValue value, int n, ...) {
    SirNDArray *a = _sir_array_require(targetv, "indexSet");
    SirIndexArg args[2];
    va_list ap;
    if (n != 1 && n != 2) {
        fprintf(stderr, "sir: indexSet: only 1 or 2 index arguments are supported (rank <= 2 scope), got %d\n", n);
        exit(1);
    }
    va_start(ap, n);
    { int i; for (i = 0; i < n; i++) args[i] = va_arg(ap, SirIndexArg); }
    va_end(ap);

    if (n == 1) {
        SirArrayPositions ps = _sir_array_resolve_positions(args[0], a->data_len);
        double *values = _sir_array_broadcast_values(value, ps.n, "indexSet");
        int64_t k;
        for (k = 0; k < ps.n; k++) {
            int64_t i = ps.pos[k];
            if (i < 0 || i >= a->data_len) {
                fprintf(stderr, "sir: indexSet: linear index %lld out of bounds\n", (long long)i);
                exit(1);
            }
            a->data[i] = values[k];
        }
        return;
    }
    {
        SirArrayPositions rows = _sir_array_resolve_positions(args[0], _sir_array_nrows(a));
        SirArrayPositions cols = _sir_array_resolve_positions(args[1], _sir_array_ncols(a));
        /* Same product-of-two-independent-selections gap `indexGet` closes
         * above — validate before `_sir_array_broadcast_values` allocates. */
        int64_t count = _sir_array_checked_size(rows.n, cols.n, "indexSet");
        double *values = _sir_array_broadcast_values(value, count, "indexSet");
        int64_t k = 0, ci, ri;
        for (ci = 0; ci < cols.n; ci++) {
            for (ri = 0; ri < rows.n; ri++) {
                if (!_sir_array_set(a, rows.pos[ri], cols.pos[ci], values[k])) {
                    fprintf(stderr, "sir: indexSet: (%lld, %lld) out of bounds for shape (%lld, %lld)\n",
                            (long long)rows.pos[ri], (long long)cols.pos[ci],
                            (long long)_sir_array_nrows(a), (long long)_sir_array_ncols(a));
                    exit(1);
                }
                k++;
            }
        }
    }
}

/* ── SIR22 addendum: APL primitive operators ─────────────────────────
 * `Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/
 * `IndexOf`/`Ravel`/`Catenate` (Phase A Slice 3) — see the module doc
 * comment above ("Vector representation in the addendum") for how this
 * file's rank-0-or-2-only value model stands in for the JS/Ruby
 * references' genuine rank-1 vector shape. Every function below is
 * ported 1:1 from `semantic-ir-to-javascript`'s own already-proven
 * addendum section (same function names, same doc-comment subtleties),
 * adapted to that representation and to this file's OWN overflow-safe
 * allocation discipline (`_sir_array_checked_size`, reused throughout —
 * see the module doc's "C-specific overflow hazard" section). */

/* Classify `a` into this domain's three logical ranks for addendum
 * dispatch — see the module doc's "Vector representation in the
 * addendum" section for the full rationale: 0 (a true `rank == 0`
 * scalar), 1 (`rank == 2` with `rows == 1` — this file's stand-in for
 * the JS/Ruby references' genuine rank-1 "vector" shape), or 2 (`rank ==
 * 2` with `rows != 1` — including a `cols == 1` COLUMN shape, e.g. from
 * `ArrayLit([[5],[6]])` or `IndexGet`'s whole-row/scalar-col selector).
 *
 * DELIBERATELY asymmetric between rows and cols: every vector this
 * backend ever actually constructs (`_sir_array_range`, `_sir_array_
 * index_get`'s non-scalar 1-argument result, and every one of THIS
 * file's own addendum functions below) is a `1 x n` ROW — never a
 * column — so `rows == 1` alone exactly captures "this is this
 * backend's vector representation". A `cols == 1, rows > 1` shape, by
 * contrast, only ever arises from a genuine 2-D construct (a literal
 * column via `ArrayLit`, or an `IndexGet` column selection) that the
 * JS/Ruby references ALSO treat as a true rank-2 matrix (their own
 * `ArrayLit`/`IndexGet` never produce a genuine rank-1 result either —
 * see the base cut's own module doc above) — so classifying it as
 * logical rank 2 here, not 1, keeps this port's addendum dispatch
 * faithful to the references for every shape this backend can actually
 * produce. (An earlier version of this function treated `cols == 1` as
 * a vector too, symmetrically with `rows == 1`; that broke `_sir_array_
 * catenate`'s matrix-matrix branch for a `2 x 1` right operand, which
 * this asymmetric rule fixes — a `2 x 1` `ArrayLit` is a real matrix,
 * not a vector.) */
int _sir_array_logical_rank(const SirNDArray *a) {
    if (a->rank == 0) return 0;
    if (a->rows == 1) return 1;
    return 2;
}

/* Flatten (rank <= 2, this domain's ceiling) `a` to ROW-major order —
 * last axis varies fastest. `a` itself stores COLUMN-major
 * (`_sir_array_get`'s own doc comment above), so a matrix must be walked
 * "row, then column" via `_sir_array_get` to produce true row-major
 * order; returning the raw column-major buffer would silently ravel in
 * the WRONG order. Writes the element count to `*out_len` and ALWAYS
 * returns a FRESH buffer (never `a->data` itself, even in the rank-0
 * no-op case) — mirrors `apl_runtime::builtins::flatten` returning an
 * owned `Vec`, not a borrow, so the result never accidentally aliases
 * `a`'s own buffer (relevant because `_sir_array_index_set` mutates an
 * `SirNDArray`'s data IN PLACE — a shared buffer here could let a later
 * mutation of `a` silently corrupt an already-returned ravel/reshape
 * result). */
double *_sir_array_flatten_row_major(const SirNDArray *a, int64_t *out_len) {
    if (a->rank == 0) {
        double *out = (double *)_sir_alloc(sizeof(double));
        out[0] = a->data[0];
        *out_len = 1;
        return out;
    }
    {
        int64_t r = a->rows, c = a->cols, row, col, k = 0;
        int64_t n = a->data_len;
        double *out = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
        for (row = 0; row < r; row++) {
            for (col = 0; col < c; col++) {
                double v;
                _sir_array_get(a, row, col, &v); /* always in-bounds by construction */
                out[k++] = v;
            }
        }
        *out_len = n;
        return out;
    }
}

/* Validate `x` is a finite non-negative integer and return it as
 * `int64_t` — shared by `_sir_array_reshape`'s shape-vector elements and
 * `_sir_array_index_generator`'s scalar argument (both APL/MATLAB "give
 * me a non-negative integer count/dimension" contexts). Reuses
 * `_sir_array_assert_valid_position`'s NaN-safe, magnitude-checked
 * finite-integer cast (see that function's own SECURITY doc above), then
 * additionally rejects negative — that function alone permits negative
 * integers (valid for e.g. a `Range` step), which neither caller here
 * accepts. */
int64_t _sir_array_require_nonneg_int(double x, const char *ctx) {
    int64_t n = _sir_array_assert_valid_position(_sir_float(x), ctx);
    if (n < 0) {
        fprintf(stderr, "sir: %s: value must be a non-negative integer, got %lld\n", ctx, (long long)n);
        exit(1);
    }
    return n;
}

/* `+/A` (APL reduce, dyadic-op monadic-adverb) — fold `target` with `op`
 * along its one axis. Ported 1:1 from `array_runtime::ops::reduce`:
 * - a true scalar (`rank == 0`): nothing to fold, returns `target`
 *   itself (the SAME `SirNDArray`, not a copy — matching the JS/Ruby
 *   references' own `return a;`).
 * - everything else (`rank == 2`, `rows` x `cols`): folds EACH ROW
 *   independently across its `cols` columns, producing a `1 x rows` row
 *   (one folded value per row of `target`) — no special "is this really
 *   a vector" branch is needed here (unlike `_sir_array_outer`/`_sir_
 *   array_shape`/etc. below): folding a single row of `cols` columns IS
 *   folding the whole vector, and folding `rows` rows of a single column
 *   each is a correct no-op, so this loop already generalizes correctly
 *   to every logical rank (see the module doc's "Vector representation
 *   in the addendum" section). An empty row (`cols == 0`) is a clean
 *   error — unlike `sum`/`mean` (which have a built-in identity, 0),
 *   `reduce` is generic over any `op`, and guessing an identity (`0` for
 *   `Add`? `1` for `Mul`? `-Infinity` for `Max`?) for an arbitrary,
 *   possibly-future op would be silently wrong for most of them.
 *
 * COLUMN-MAJOR storage means element `(row, col)` lives at `col * rows +
 * row` — the row loop reads `data[row]` as the seed (column 0) then
 * walks `data[col * rows + row]` for `col = 1..cols`; getting `row` and
 * `col` swapped here silently TRANSPOSES the result instead of throwing,
 * so this indexing is the single easiest place to introduce a
 * wrong-answer bug when reading this function (mirrors the JS
 * reference's own warning, verbatim).
 *
 * Named `_sir_array_apl_reduce`, not the shorter `_sir_array_reduce` every
 * sibling addendum function's naming would suggest — that plainer name is
 * already taken by the PRE-EXISTING `SirSeq` `Array#reduce`/`#inject`
 * helper a few thousand lines up (`_sir_array_` was this file's generic
 * "Ruby Array collection method" prefix well before SIR22 NDArrays
 * existed and claimed the same prefix for itself). `_sir_array_apl_reduce`
 * disambiguates without renaming that unrelated, already-shipped
 * function. */
SirValue _sir_array_apl_reduce(SirElementwiseOp op, SirValue targetv) {
    SirNDArray *a = _sir_array_coerce(targetv, "reduce");
    if (a->rank == 0) {
        SirValue v; v.tag = SIR_ARRAY; v.as.arr = a; return v;
    }
    {
        int64_t r = a->rows, c = a->cols, row;
        double *out;
        if (c == 0) {
            fprintf(stderr, "sir: reduce: cannot fold an empty vector or row (no identity element for an arbitrary op)\n");
            exit(1);
        }
        out = (r > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)r) : NULL;
        for (row = 0; row < r; row++) {
            double acc = a->data[row]; /* column-major: (row, 0) lives at plain `row` */
            int64_t col;
            for (col = 1; col < c; col++) {
                acc = _sir_array_apply_op(op, acc, a->data[col * r + row]);
            }
            out[row] = acc;
        }
        return _sir_array_new_matrix(1, r, out, "reduce");
    }
}

/* `+\A` (APL scan) — the same fold as `_sir_array_apl_reduce`, but keeping
 * EVERY intermediate result instead of only the last; output has the
 * SAME shape as `target`. Ported 1:1 from `array_runtime::ops::scan`.
 * Unlike `_sir_array_apl_reduce`, an empty row/vector (`cols == 0`) is NOT an
 * error here: there is simply nothing to scan, and the (empty) output
 * shape already says so. Same column-major `col * rows + row` indexing,
 * and same "no special vector branch needed" reasoning, as `_sir_array_
 * reduce` above. */
SirValue _sir_array_scan(SirElementwiseOp op, SirValue targetv) {
    SirNDArray *a = _sir_array_coerce(targetv, "scan");
    if (a->rank == 0) {
        SirValue v; v.tag = SIR_ARRAY; v.as.arr = a; return v;
    }
    {
        int64_t r = a->rows, c = a->cols, row;
        int64_t n = a->data_len; /* == r * c, already validated when `a` was built */
        double *out = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
        for (row = 0; row < r; row++) {
            double acc = 0.0;
            int started = 0;
            int64_t col;
            for (col = 0; col < c; col++) {
                double x = a->data[col * r + row]; /* column-major */
                acc = started ? _sir_array_apply_op(op, acc, x) : x;
                started = 1;
                out[col * r + row] = acc;
            }
        }
        return _sir_array_new_matrix(r, c, out, "scan");
    }
}

/* `A∘.×B` (APL outer product) — apply `op` to every pair `(aᵢ, bⱼ)`,
 * producing a result of combined logical rank. Ported 1:1 from
 * `array_runtime::ops::outer`, scoped identically to `rank(a) <= 1` and
 * `rank(b) <= 1` (`_sir_array_logical_rank` — see the module doc's
 * "Vector representation in the addendum" section for exactly what
 * counts as rank <= 1 here); any operand of logical rank 2 is a clean
 * "not yet supported" error, matching the Rust/JS references' own scope
 * limit. `_sir_array_checked_size` validates the `[m, n]` output shape
 * BEFORE allocating in the vector x vector case — `m`/`n` are two
 * INDEPENDENT operand lengths, each individually under `SIR_ARRAY_
 * MAX_ELEMENTS`, but nothing bounds their PRODUCT alone (the same
 * outer-product-shaped allocation `_sir_array_matmul`/`_sir_array_
 * index_get` above guard). */
SirValue _sir_array_outer(SirElementwiseOp op, SirValue lhsv, SirValue rhsv) {
    SirNDArray *a = _sir_array_coerce(lhsv, "outer");
    SirNDArray *b = _sir_array_coerce(rhsv, "outer");
    int ra = _sir_array_logical_rank(a);
    int rb = _sir_array_logical_rank(b);
    if (ra == 0 && rb == 0) {
        return _sir_array_scalar(_sir_array_apply_op(op, a->data[0], b->data[0]));
    }
    if (ra == 0 && rb == 1) {
        double x = a->data[0];
        int64_t n = b->data_len, i;
        double *out = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
        for (i = 0; i < n; i++) out[i] = _sir_array_apply_op(op, x, b->data[i]);
        return _sir_array_new_matrix(1, n, out, "outer");
    }
    if (ra == 1 && rb == 0) {
        double y = b->data[0];
        int64_t n = a->data_len, i;
        double *out = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
        for (i = 0; i < n; i++) out[i] = _sir_array_apply_op(op, a->data[i], y);
        return _sir_array_new_matrix(1, n, out, "outer");
    }
    if (ra == 1 && rb == 1) {
        int64_t m = a->data_len, n = b->data_len;
        int64_t out_len = _sir_array_checked_size(m, n, "outer");
        double *out = (out_len > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)out_len) : NULL;
        int64_t i, j;
        for (j = 0; j < n; j++) {
            for (i = 0; i < m; i++) {
                out[j * m + i] = _sir_array_apply_op(op, a->data[i], b->data[j]); /* column-major */
            }
        }
        return _sir_array_new_matrix(m, n, out, "outer");
    }
    fprintf(stderr, "sir: outer: operands of rank > 1 not yet supported\n");
    exit(1);
    return _sir_nil(); /* unreachable */
}

/* Monadic `⍴` (shape-of) — `target`'s dimensions as a vector. Ported 1:1
 * from `apl_runtime::builtins::shape`: a SCALAR has zero dimensions, so
 * its shape is the EMPTY vector (not a scalar!) — `⍴5` is a length-0
 * vector (a `1 x 0` row here — see the module doc's "Vector
 * representation in the addendum" section). A logical-rank-1 operand
 * (this file's vector stand-in — always `rows == 1`, see `_sir_array_
 * logical_rank`'s own doc comment) has shape `[n]` where `n` is its
 * element count (a `1 x 1` row result); a logical-rank-2 operand (`rows
 * != 1`, including a `cols == 1` column shape) has shape `[rows, cols]`
 * (a `1 x 2` row result) built from `target`'s REAL `rows`/`cols`
 * fields. */
SirValue _sir_array_shape(SirValue targetv) {
    SirNDArray *a = _sir_array_coerce(targetv, "shape");
    int lr = _sir_array_logical_rank(a);
    if (lr == 0) {
        return _sir_array_new_matrix(1, 0, NULL, "shape");
    }
    if (lr == 1) {
        double *out = (double *)_sir_alloc(sizeof(double));
        out[0] = (double)a->data_len;
        return _sir_array_new_matrix(1, 1, out, "shape");
    }
    {
        double *out = (double *)_sir_alloc(sizeof(double) * 2);
        out[0] = (double)a->rows;
        out[1] = (double)a->cols;
        return _sir_array_new_matrix(1, 2, out, "shape");
    }
}

/* Dyadic `⍴` (reshape) — reinterpret `target`'s data under the new
 * dimensions `shapev`. Ported 1:1 from `apl_runtime::builtins::reshape`.
 * `shapev` must itself have logical rank <= 1 (a scalar or vector — see
 * `_sir_array_logical_rank`) of non-negative integers; its ELEMENT COUNT
 * becomes the target's dimensionality (0 elements -> a scalar result, 1
 * element -> a `1 x n` row, 2 elements -> a genuine `r x c` matrix) and
 * is itself capped at <= 2 (this domain's ceiling — more elements is a
 * clean error, not a silent truncation). `target`'s elements are
 * ravelled (`_sir_array_flatten_row_major`) then cyclically repeated or
 * truncated to fill the target shape's element count.
 *
 * CRITICAL: the cyclic fill happens in ROW-major order (APL's reshape
 * fills the LAST axis fastest, same convention as ravel), but this
 * domain's storage is COLUMN-major — so for a 2-element target shape the
 * row-major `filled` sequence must be TRANSPOSED into column-major
 * storage (`data[col * r + row] = filled[row * c + col]`) before
 * constructing the result. Handing `filled` straight to `_sir_array_
 * new_matrix` would silently reshape column-major instead of APL's
 * row-major convention — a wrong answer that still LOOKS plausible
 * (right multiset of values, wrong positions). A 1-element target shape
 * needs NO such transpose: a `1 x n` row IS its own row-major order
 * (column-major storage of a single row is identical to linear order),
 * so `filled` is used as-is. */
SirValue _sir_array_reshape(SirValue shapev, SirValue targetv) {
    SirNDArray *shape_arr = _sir_array_coerce(shapev, "reshape");
    SirNDArray *target = _sir_array_coerce(targetv, "reshape");
    int64_t dims[2];
    int64_t ndims;
    int64_t total;
    int64_t source_len;
    double *source;
    double *filled;
    int64_t k;

    if (_sir_array_logical_rank(shape_arr) == 2) {
        fprintf(stderr, "sir: reshape: shape argument must be a scalar or vector (got a matrix)\n");
        exit(1);
    }
    ndims = shape_arr->data_len;
    if (ndims > 2) {
        fprintf(stderr, "sir: reshape: reshape to rank > 2 is not yet supported (%lld shape elements)\n",
                (long long)ndims);
        exit(1);
    }
    { int64_t i; for (i = 0; i < ndims; i++) dims[i] = _sir_array_require_nonneg_int(shape_arr->data[i], "reshape"); }

    if (ndims == 0) {
        total = 1;
    } else if (ndims == 1) {
        total = _sir_array_checked_size(1, dims[0], "reshape");
    } else {
        total = _sir_array_checked_size(dims[0], dims[1], "reshape");
    }

    source = _sir_array_flatten_row_major(target, &source_len);
    if (total > 0 && source_len == 0) {
        fprintf(stderr, "sir: reshape: cannot reshape an empty source into a non-empty shape\n");
        exit(1);
    }
    filled = (total > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)total) : NULL;
    for (k = 0; k < total; k++) {
        filled[k] = source[k % source_len];
    }

    if (ndims == 0) {
        return _sir_array_scalar(filled[0]);
    }
    if (ndims == 1) {
        return _sir_array_new_matrix(1, dims[0], filled, "reshape");
    }
    {
        int64_t r = dims[0], c = dims[1], row, col;
        double *data = (total > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)total) : NULL;
        for (row = 0; row < r; row++) {
            for (col = 0; col < c; col++) {
                data[col * r + row] = filled[row * c + col];
            }
        }
        return _sir_array_new_matrix(r, c, data, "reshape");
    }
}

/* Monadic `⍳` (index generator / iota) — `⍳n` is the 1-BASED vector `[1,
 * 2, …, n]`. Ported 1:1 from `apl_runtime::builtins::index_generator` —
 * note this is 1-based, unlike every 0-based index elsewhere in this
 * domain (`IndexGet`/`IndexSet`), because that is genuinely what APL's
 * `⍳` means at the SURFACE-SYNTAX level, confirmed directly against
 * `apl-runtime`'s own `index_generator_produces_one_based_run` test (NOT
 * the stale claim in `semantic-ir`'s own `Expr::IndexGenerator` doc
 * comment, which currently — incorrectly — describes this as 0-based;
 * the JS/Ruby backends' own addendum ports already settled on 1-based to
 * match the real `apl-runtime` behaviour, and this port matches them).
 * `_sir_array_is_scalar` (element count 1, NOT `rank == 0` — matching the
 * JS reference's own `isScalar`, which also checks element count rather
 * than true rank) accepts a bare number, a `rank == 0` scalar, or a
 * degenerate `1 x 1` NDArray alike. `_sir_array_checked_size([1, n])`
 * caps `n` at `SIR_ARRAY_MAX_ELEMENTS` before allocating — `n` is a
 * runtime value a compiled program computes, not a fixed constant, so
 * `⍳` of an absurd size must fail cleanly. */
SirValue _sir_array_index_generator(SirValue countv) {
    SirNDArray *a = _sir_array_coerce(countv, "indexGenerator");
    int64_t x, n, i;
    double *out;
    if (!_sir_array_is_scalar(a)) {
        fprintf(stderr, "sir: indexGenerator: monadic argument must be a scalar\n");
        exit(1);
    }
    x = _sir_array_require_nonneg_int(a->data[0], "indexGenerator");
    n = _sir_array_checked_size(1, x, "indexGenerator");
    out = (n > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)n) : NULL;
    for (i = 0; i < n; i++) out[i] = (double)(i + 1);
    return _sir_array_new_matrix(1, n, out, "indexGenerator");
}

/* Dyadic `⍳` (index-of / search) — for every element of `needle`, the
 * 1-based index of its first occurrence in the vector `haystack` (or
 * `haystack`'s element count + 1 if not found — "not found" is a valid,
 * always-in-range position, not `-1`). Ported 1:1 from `apl_runtime::
 * builtins::index_of`: plain EXACT `==` equality (no floating-point
 * tolerance — a NaN haystack element correctly never matches, same as
 * the Rust/JS references' own `==`/`indexOf`). `haystack` must have
 * logical rank <= 1 (`_sir_array_logical_rank`; a genuine matrix
 * haystack is a clean error). The result takes `needle`'s OWN ACTUAL
 * `rank`/`rows`/`cols` (NOT `_sir_array_logical_rank`) — mirrors the
 * references' `ndarray(b.shape, out)`, so a genuine-matrix needle
 * round-trips through with its shape unchanged.
 *
 * The work done is O(len(haystack) * len(needle)) (a full linear scan per
 * needle element) — `_sir_array_checked_size` is reused here PURELY for
 * its "product <= SIR_ARRAY_MAX_ELEMENTS" overflow-safe check (both
 * lengths are already valid non-negative `int64_t`s, so its own
 * dimension-validity half is a no-op) to cap the PRODUCT before
 * scanning, since each operand individually staying under the cap does
 * not bound their product (up to ~4.5 * 10^15 comparisons otherwise). */
SirValue _sir_array_index_of(SirValue haystackv, SirValue needlev) {
    SirNDArray *a = _sir_array_coerce(haystackv, "indexOf");
    SirNDArray *b = _sir_array_coerce(needlev, "indexOf");
    int64_t hn = a->data_len, nn = b->data_len, i;
    double *out;
    if (_sir_array_logical_rank(a) == 2) {
        fprintf(stderr, "sir: indexOf: left argument must be a scalar or vector (got a matrix)\n");
        exit(1);
    }
    (void)_sir_array_checked_size(hn, nn, "indexOf");
    out = (nn > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)nn) : NULL;
    for (i = 0; i < nn; i++) {
        double needle = b->data[i];
        int64_t pos = -1, k;
        for (k = 0; k < hn; k++) {
            if (a->data[k] == needle) { pos = k; break; }
        }
        out[i] = (pos >= 0) ? (double)(pos + 1) : (double)(hn + 1);
    }
    if (b->rank == 0) {
        return _sir_array_scalar(out[0]);
    }
    return _sir_array_new_matrix(b->rows, b->cols, out, "indexOf");
}

/* Monadic `,` (ravel) — flatten `target` to a `1 x n` row (see
 * `_sir_array_flatten_row_major`'s own doc comment for the
 * column-major-storage-vs-row-major-order subtlety this must respect).
 * Ported 1:1 from `apl_runtime::builtins::ravel`. */
SirValue _sir_array_ravel(SirValue targetv) {
    SirNDArray *a = _sir_array_coerce(targetv, "ravel");
    int64_t n;
    double *flat = _sir_array_flatten_row_major(a, &n);
    return _sir_array_new_matrix(1, n, flat, "ravel");
}

/* Dyadic `,` (catenate) — supports scalar/vector operands in any
 * combination (logical rank 0 or 1 on EITHER side, per `_sir_array_
 * logical_rank` — a "vector" here is specifically `rows == 1`; see the
 * module doc's "Vector representation in the addendum" section for why
 * that's asymmetric with `cols == 1`, and why a vector of either length
 * combines with a scalar or a differently-sized vector with no
 * equal-length requirement), all producing a `1 x n` row; and
 * matrix-matrix (BOTH operands logical rank 2 — including a `cols == 1`
 * column shape) WITH EQUAL ROW COUNTS (column/last-axis catenate,
 * producing `rows x (cols_a + cols_b)`). Any other combination (a vector
 * catenated with a genuine matrix, or mismatched-row matrices) is a
 * clean error. Ported 1:1 from `apl_runtime::builtins::catenate`.
 *
 * The combined-length cap check happens ONCE, up front, regardless of
 * which combination follows (mirroring the Rust/JS references' own
 * structure, and reusing `_sir_array_checked_size`'s overflow-safe
 * product check via a `1 x (na + nb)` framing) — neither operand alone
 * need be oversized for the RESULT to be, since code that repeatedly
 * catenates a value with itself (`a = [a, a]`) doubles the size every
 * line with no other ceiling. `na + nb` itself cannot overflow `int64_t`
 * (each of `na`/`nb` is already <= `SIR_ARRAY_MAX_ELEMENTS`, ~6.7e7, by
 * construction — their sum stays far below `int64_t`'s range), so only
 * the CAP comparison, not the addition, needs guarding here; the
 * `outer`/`indexOf` product checks above are the ones that need
 * `_sir_array_checked_size`'s multiplication-overflow guard
 * specifically. */
SirValue _sir_array_catenate(SirValue lhsv, SirValue rhsv) {
    SirNDArray *a = _sir_array_coerce(lhsv, "catenate");
    SirNDArray *b = _sir_array_coerce(rhsv, "catenate");
    int64_t na = a->data_len, nb = b->data_len;
    int ra = _sir_array_logical_rank(a);
    int rb = _sir_array_logical_rank(b);
    (void)_sir_array_checked_size(1, na + nb, "catenate");

    if (ra != 2 && rb != 2) {
        double *out = (na + nb > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)(na + nb)) : NULL;
        if (na > 0) memcpy(out, a->data, sizeof(double) * (size_t)na);
        if (nb > 0) memcpy(out + na, b->data, sizeof(double) * (size_t)nb);
        return _sir_array_new_matrix(1, na + nb, out, "catenate");
    }
    if (ra == 2 && rb == 2) {
        int64_t r = a->rows, ca = a->cols, cb = b->cols, row, col;
        int64_t out_len;
        double *data;
        if (r != b->rows) {
            fprintf(stderr, "sir: catenate: matrix catenate needs equal row counts (%lld vs %lld)\n",
                    (long long)r, (long long)b->rows);
            exit(1);
        }
        out_len = _sir_array_checked_size(r, ca + cb, "catenate");
        data = (out_len > 0) ? (double *)_sir_alloc(sizeof(double) * (size_t)out_len) : NULL;
        for (row = 0; row < r; row++) {
            for (col = 0; col < ca; col++) {
                data[col * r + row] = a->data[col * r + row];
            }
            for (col = 0; col < cb; col++) {
                data[(ca + col) * r + row] = b->data[col * r + row];
            }
        }
        return _sir_array_new_matrix(r, ca + cb, data, "catenate");
    }
    fprintf(stderr, "sir: catenate: catenate of a vector with a matrix is not yet supported\n");
    exit(1);
    return _sir_nil(); /* unreachable */
}
"####;
