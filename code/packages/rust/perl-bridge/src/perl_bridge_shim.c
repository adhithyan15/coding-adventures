#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

typedef struct PerlBridgeXsubFrame {
    SV **base;
    I32 ax;
    I32 items;
} PerlBridgeXsubFrame;

typedef void (*PerlBridgeRustXsub)(CV *cv);

void perl_bridge_xsub_frame(PerlBridgeXsubFrame *out) {
    dTHX;
    dXSARGS;

    out->base = PL_stack_base;
    out->ax = ax;
    out->items = items;
}

void perl_bridge_xsub_return(I32 ax, I32 count) {
    dTHX;

    if (count > 0) {
        PL_stack_sp = PL_stack_base + ax + (count - 1);
    } else {
        PL_stack_sp = PL_stack_base + ax - 1;
    }
}

static XS(perl_bridge_dispatch_xsub) {
    PerlBridgeRustXsub callback = (PerlBridgeRustXsub)CvXSUBANY(cv).any_ptr;
    callback(cv);
}

CV *perl_bridge_newXS(const char *name, PerlBridgeRustXsub subaddr, const char *filename) {
    dTHX;
    CV *cv = newXS(name, perl_bridge_dispatch_xsub, filename);
    CvXSUBANY(cv).any_ptr = (void *)subaddr;
    return cv;
}

SV *perl_bridge_newSViv(IV value) {
    dTHX;
    return newSViv(value);
}

SV *perl_bridge_newSVnv(NV value) {
    dTHX;
    return newSVnv(value);
}

SV *perl_bridge_newSVpv(const char *value, STRLEN length) {
    dTHX;
    return newSVpv(value, length);
}

SV *perl_bridge_newSVpvn(const char *value, STRLEN length) {
    dTHX;
    return newSVpvn(value, length);
}

SV *perl_bridge_newSVuv(UV value) {
    dTHX;
    return newSVuv(value);
}

IV perl_bridge_sv_2iv(SV *sv) {
    dTHX;
    return sv_2iv(sv);
}

NV perl_bridge_sv_2nv(SV *sv) {
    dTHX;
    return sv_2nv(sv);
}

char *perl_bridge_sv_2pv_flags(SV *sv, STRLEN *length, U32 flags) {
    dTHX;
    return sv_2pv_flags(sv, length, flags);
}

int perl_bridge_sv_true(SV *sv) {
    dTHX;
    return SvTRUE(sv) ? 1 : 0;
}

int perl_bridge_sv_iok(SV *sv) {
    return SvIOK(sv) ? 1 : 0;
}

int perl_bridge_sv_nok(SV *sv) {
    return SvNOK(sv) ? 1 : 0;
}

int perl_bridge_sv_pok(SV *sv) {
    return SvPOK(sv) ? 1 : 0;
}

SV *perl_bridge_sv_refcnt_inc(SV *sv) {
    dTHX;
    return SvREFCNT_inc(sv);
}

void perl_bridge_sv_refcnt_dec(SV *sv) {
    dTHX;
    SvREFCNT_dec(sv);
}

AV *perl_bridge_newAV(void) {
    dTHX;
    return newAV();
}

void perl_bridge_av_push(AV *av, SV *value) {
    dTHX;
    av_push(av, value);
}

SV *perl_bridge_av_pop(AV *av) {
    dTHX;
    return av_pop(av);
}

SV **perl_bridge_av_fetch(AV *av, SSize_t key, I32 lval) {
    dTHX;
    return av_fetch(av, key, lval);
}

SSize_t perl_bridge_av_len(AV *av) {
    dTHX;
    return av_len(av);
}

SV *perl_bridge_newRV_noinc(SV *value) {
    dTHX;
    return newRV_noinc(value);
}

SV *perl_bridge_sv_rv(SV *value) {
    return SvRV(value);
}

int perl_bridge_sv_rok(SV *value) {
    return SvROK(value) ? 1 : 0;
}

void perl_bridge_croak_message(const char *message) {
    dTHX;
    croak("%s", message);
}

void perl_bridge_warn_message(const char *message) {
    dTHX;
    warn("%s", message);
}

void perl_bridge_xs_boot_finish(I32 ax) {
    dTHX;
    Perl_xs_boot_epilog(aTHX_ ax);
}

/* ── Conduit (WEB11) additions: coderef calls, hashes, thread context ─────── */

/*
 * Call a Perl coderef with `argc` SV* arguments under G_SCALAR | G_EVAL.
 *
 * Returns the result SV (reference count incremented — the caller owns it), or
 * NULL if the sub died. On death, *err_out is set to a malloc'd NUL-terminated
 * copy of $@ stringified (the caller must free() it); on success *err_out is
 * left NULL. The arguments are passed through unchanged (mortalised copies are
 * pushed so the callee can't disturb the caller's refcounts).
 *
 * Must be called while holding the interpreter lock (Perl is single-threaded).
 */
SV *perl_bridge_call_coderef(SV *coderef, SV **argv, int argc, char **err_out) {
    dTHX;
    dSP;
    int i;
    SV *result = NULL;
    *err_out = NULL;

    ENTER;
    SAVETMPS;
    PUSHMARK(SP);
    for (i = 0; i < argc; i++) {
        XPUSHs(sv_2mortal(newSVsv(argv[i])));
    }
    PUTBACK;

    int count = call_sv(coderef, G_SCALAR | G_EVAL);

    SPAGAIN;

    SV *errsv = ERRSV; /* $@ */
    if (SvTRUE(errsv)) {
        STRLEN len;
        const char *msg = SvPV(errsv, len);
        char *copy = (char *)malloc(len + 1);
        if (copy != NULL) {
            memcpy(copy, msg, len);
            copy[len] = '\0';
        }
        *err_out = copy;
        for (i = 0; i < count; i++) {
            (void)POPs;
        }
        result = NULL;
    } else if (count > 0) {
        /* Copy the topmost return value so it survives FREETMPS. */
        result = newSVsv(POPs);
    } else {
        result = NULL;
    }

    PUTBACK;
    FREETMPS;
    LEAVE;
    return result;
}

HV *perl_bridge_newHV(void) {
    dTHX;
    return newHV();
}

/* Store `val` under `key` (length `klen`). Takes ownership of `val`'s refcount
 * (hv_store does not increment), matching newSV* producers. */
void perl_bridge_hv_store(HV *hv, const char *key, I32 klen, SV *val) {
    dTHX;
    (void)hv_store(hv, key, klen, val, 0);
}

SV *perl_bridge_newRV_inc(SV *value) {
    dTHX;
    return newRV_inc(value);
}

/* Capture the current interpreter context (aTHX) on the main Perl thread.
 * On a non-MULTIPLICITY build there is a single global interpreter and this
 * returns NULL (set_context is then a no-op). */
void *perl_bridge_get_context(void) {
#ifdef MULTIPLICITY
    return (void *)PERL_GET_CONTEXT;
#else
    return NULL;
#endif
}

/* Bind `ctx` as the interpreter for the current OS thread before any Perl API
 * use from a non-Perl (web-core I/O) thread. No-op on non-MULTIPLICITY. */
void perl_bridge_set_context(void *ctx) {
#ifdef MULTIPLICITY
    PERL_SET_CONTEXT((PerlInterpreter *)ctx);
#else
    (void)ctx;
#endif
}
