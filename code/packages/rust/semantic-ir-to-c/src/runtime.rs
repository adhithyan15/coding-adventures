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
    SIR_STR, SIR_SYM, SIR_PAIR, SIR_CLOSURE
} SirTag;

typedef struct SirValue SirValue;
typedef struct SirPair SirPair;
typedef struct SirClosure SirClosure;

struct SirValue {
    SirTag tag;
    union {
        int b;            /* SIR_BOOL (0/1) */
        int64_t i;        /* SIR_INT */
        double f;         /* SIR_FLOAT */
        const char *s;    /* SIR_STR / SIR_SYM (interned) */
        SirPair *pair;    /* SIR_PAIR */
        SirClosure *clo;  /* SIR_CLOSURE */
    } as;
};

struct SirPair { SirValue car; SirValue cdr; };

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

int _sir_value_eq(SirValue a, SirValue b) {
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
        case SIR_PAIR:    return _sir_value_eq(a.as.pair->car, b.as.pair->car)
                              && _sir_value_eq(a.as.pair->cdr, b.as.pair->cdr);
        case SIR_CLOSURE: return a.as.clo == b.as.clo;
        default:          return 0;
    }
}
SirValue _sir_eq(SirValue a, SirValue b) { return _sir_bool(_sir_value_eq(a, b)); }
SirValue _sir_ne(SirValue a, SirValue b) { return _sir_bool(!_sir_value_eq(a, b)); }

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

void _sir_fmt(FILE *out, SirValue v) {
    char buf[32];
    switch (v.tag) {
        case SIR_NIL:   fputs(SIR_DISPLAY_RUBY ? "" : "nil", out); break;
        case SIR_BOOL:  fputs(v.as.b ? (SIR_DISPLAY_RUBY ? "true" : "#t")
                                     : (SIR_DISPLAY_RUBY ? "false" : "#f"), out); break;
        case SIR_INT:   snprintf(buf, sizeof(buf), "%lld", (long long)v.as.i); fputs(buf, out); break;
        case SIR_FLOAT: _sir_fmt_float(out, v.as.f); break;
        case SIR_STR:   fputs(v.as.s, out); break;
        case SIR_SYM:   fputs(v.as.s, out); break;
        case SIR_PAIR:  _sir_fmt_pair(out, v); break;
        case SIR_CLOSURE: fputs("#<closure>", out); break;
        default: break;
    }
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
