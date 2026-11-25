/*
 * fuzz_sig.c -- crash descriptions, signatures, failure ids and TAP lines (VER06 sections 5.2, 6).
 *
 * WHY A SIGNATURE
 * ---------------
 * A fuzzer finds the same bug thousands of times.  To report it once it needs a
 * short name for "this bug": the signature.  Two crashes with the same kind and
 * the same stack of frames are the same bug.
 *
 *      sig = low 32 bits of fnv1a64( kind ":" frame1 ":" frame2 ":" ... )
 *
 * FNV-1a is a tiny byte-at-a-time hash:
 *
 *      h = 0xcbf29ce484222325
 *      for each byte b:  h = (h XOR b) * 0x100000001b3     (mod 2^64)
 *
 * Frames are NOT escaped, so one frame "x:y" and two frames "x", "y" hash alike.
 * This engine never produces a frame containing ':' (normalisation below), so the
 * ambiguity cannot arise from its own targets.
 *
 * PRIVACY
 * -------
 * The only public failure line is   not ok N - target [failure-id:hex8]   :
 * no input bytes, no seed, no message.  Every argument is reduced to its grammar
 * so a hostile target name containing a newline cannot forge a second TAP line.
 */
#include <stdio.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"

/* ---- character classes -------------------------------------------------------------------------------- */

static int is_id_byte(unsigned char c)
{
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '.' || c == '_' ||
           c == '/' || c == '-';
}

/* Crash kinds and frames also allow < > $ (generated names).  ':' is NOT allowed. */
static int is_name_byte(unsigned char c)
{
    return is_id_byte(c) || c == '<' || c == '>' || c == '$';
}

static int is_hex_byte(unsigned char c)
{
    return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f');
}

/* Copy up to cap bytes of src (NUL terminated, but never read past srcmax), mapping each disallowed byte to '_'.
 * dst gets cap+1 bytes of room.  Returns the length. */
static size_t sanitize_into(char *dst, size_t cap, const char *src, size_t srcmax)
{
    size_t n = 0;
    while (n < cap && n < srcmax && src[n] != '\0') {
        dst[n] = is_name_byte((unsigned char)src[n]) ? src[n] : '_';
        n++;
    }
    dst[n] = '\0';
    return n;
}

/* ---- crash descriptions --------------------------------------------------------------------------------- */

void fz_crash_set(fz_crash *c, const char *kind)
{
    if (kind == NULL) {
        kind = "";
    }
    (void)sanitize_into(c->kind, FZ_KIND_MAX, kind, (size_t)-1);
}

void fz_crash_push(fz_crash *c, const char *frame)
{
    char tmp[FZ_FRAME_MAX + 1];
    size_t n;
    if (frame == NULL || c->nframes >= FZ_MAX_FRAMES) {
        return;
    }
    n = sanitize_into(tmp, FZ_FRAME_MAX, frame, (size_t)-1);
    if (n == 0 || (c->nframes > 0 && strncmp(c->frames[c->nframes - 1], tmp, FZ_FRAME_MAX + 1) == 0)) {
        return; /* empty, or a repeat of the previous frame: a recursive crash hashes alike at any depth */
    }
    memcpy(c->frames[c->nframes], tmp, n + 1);
    c->nframes++;
}

/*
 * Whatever a target wrote is untrusted: it may have scribbled over the struct directly.  Re-terminate every
 * string inside its array, re-sanitise, clamp the frame count, rebuild the frame list without empties or
 * consecutive repeats, and give an empty kind a name.
 */
void fz_crash_normalize(fz_crash *c)
{
    fz_crash clean;
    size_t i;
    size_t n = c->nframes > FZ_MAX_FRAMES ? FZ_MAX_FRAMES : c->nframes;
    memset(&clean, 0, sizeof clean);
    c->kind[FZ_KIND_MAX] = '\0';
    fz_crash_set(&clean, c->kind);
    if (clean.kind[0] == '\0') {
        memcpy(clean.kind, "unspecified", sizeof "unspecified");
    }
    for (i = 0; i < n; i++) {
        c->frames[i][FZ_FRAME_MAX] = '\0';
        fz_crash_push(&clean, c->frames[i]);
    }
    *c = clean;
}

/* ---- signatures ------------------------------------------------------------------------------------------ */

static uint64_t fnv_bytes(uint64_t h, const char *p, size_t n)
{
    size_t i;
    for (i = 0; i < n; i++) {
        h ^= (unsigned char)p[i];
        h *= 0x100000001b3u;
    }
    return h;
}

void fz_crash_signature_n(char out[9], fz_str kind, const fz_str *frames, size_t nframes)
{
    uint64_t h = 0xcbf29ce484222325u;
    size_t i;
    h = fnv_bytes(h, kind.p, kind.n);
    h = fnv_bytes(h, ":", 1);
    for (i = 0; i < nframes; i++) {
        if (i > 0) {
            h = fnv_bytes(h, ":", 1);
        }
        h = fnv_bytes(h, frames[i].p, frames[i].n);
    }
    if (fz_hooks.sig_post != NULL) {
        h = fz_hooks.sig_post(h);
    }
    (void)snprintf(out, 9, "%08lx", (unsigned long)(h & 0xFFFFFFFFu));
}

void fz_crash_signature(char out[9], const char *kind, const char *const *frames, size_t nframes)
{
    fz_str k;
    fz_str fs[FZ_MAX_FRAMES];
    size_t i;
    k.p = kind;
    k.n = strlen(kind);
    if (nframes > FZ_MAX_FRAMES) {
        nframes = FZ_MAX_FRAMES;
    }
    for (i = 0; i < nframes; i++) {
        fs[i].p = frames[i];
        fs[i].n = strlen(frames[i]);
    }
    fz_crash_signature_n(out, k, fs, nframes);
}

void fz_crash_signature_of(char out[9], const fz_crash *c)
{
    fz_str k;
    fz_str fs[FZ_MAX_FRAMES];
    size_t i;
    size_t n = c->nframes > FZ_MAX_FRAMES ? FZ_MAX_FRAMES : c->nframes;
    k.p = c->kind;
    k.n = strlen(c->kind);
    for (i = 0; i < n; i++) {
        fs[i].p = c->frames[i];
        fs[i].n = strlen(c->frames[i]);
    }
    fz_crash_signature_n(out, k, fs, n);
}

int fz_is_duplicate_crash(fz_str ka, const fz_str *fa, size_t na, fz_str kb, const fz_str *fb, size_t nb)
{
    char sa[9];
    char sb[9];
    fz_crash_signature_n(sa, ka, fa, na);
    fz_crash_signature_n(sb, kb, fb, nb);
    return memcmp(sa, sb, 9) == 0;
}

/* ---- failure ids ----------------------------------------------------------------------------------------- */

int fz_is_failure_id(const char *candidate, size_t len)
{
    static const char *const ids[5] = {FZ_FID_CRASH, FZ_FID_TIMEOUT, FZ_FID_OOM, FZ_FID_LEAK, FZ_FID_PANIC};
    size_t i;
    for (i = 0; i < 5; i++) {
        if (strlen(ids[i]) == len && memcmp(ids[i], candidate, len) == 0) {
            return 1;
        }
    }
    return 0;
}

/*
 * The only public failure line:  not ok N - target [failure-id:hex8]
 *
 * The target is truncated to 128 bytes and every byte outside [A-Za-z0-9._/-] (controls, newlines, multi-byte
 * UTF-8, an embedded NUL) becomes '_', so the result is one line of pure ASCII.  Well-formed arguments pass
 * through unchanged.
 */
int fz_format_tap_failure(char *buf, size_t cap, int test_num, const char *target, size_t target_len,
                          const char *failure_id, size_t failure_id_len, const char *sig_hash, size_t sig_len)
{
    char tgt[FZ_MAX_TARGET_LEN + 1];
    size_t i;
    int written;
    if (!fz_is_failure_id(failure_id, failure_id_len)) {
        return FZ_E_ARG;
    }
    if (sig_len != 8) {
        return FZ_E_ARG;
    }
    for (i = 0; i < 8; i++) {
        if (!is_hex_byte((unsigned char)sig_hash[i])) {
            return FZ_E_ARG;
        }
    }
    if (target_len > FZ_MAX_TARGET_LEN) {
        target_len = FZ_MAX_TARGET_LEN;
    }
    for (i = 0; i < target_len; i++) {
        tgt[i] = is_id_byte((unsigned char)target[i]) ? target[i] : '_';
    }
    tgt[target_len] = '\0';
    written = snprintf(buf, cap, "not ok %d - %s [%.*s:%.8s]", test_num, tgt, (int)failure_id_len, failure_id, sig_hash);
    if (written < 0 || (size_t)written >= cap) {
        if (cap > 0) {
            buf[0] = '\0';
        }
        return FZ_E_LIMIT;
    }
    return FZ_OK;
}
