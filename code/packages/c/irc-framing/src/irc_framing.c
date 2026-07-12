/*
 * irc_framing.c — implementation of the stateful IRC line framer.
 * ===========================================================================
 *
 * The framer holds a single growable byte buffer. `feed` appends to it;
 * `frames` scans it for line terminators, copies out each complete CRLF-stripped
 * line, and drains the consumed prefix in one move at the end.
 *
 * The Rust original drains the buffer after every extracted line (an O(n²) shift
 * pattern on a busy connection). This port scans with a cursor and drains the
 * whole consumed prefix exactly once — the observable result is identical, and
 * on an allocation failure mid-scan the buffer is left completely intact, so the
 * call is safely retryable.
 */
#include "irc_framing.h"

#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  The growable byte buffer
 * =========================================================================== */

struct IrcFramer {
    unsigned char *buf;
    size_t len;
    size_t cap;
};

/* Ensure room for `extra` more bytes, guarding the doubling against overflow.
 * Returns 1 on success, 0 on allocation failure or size overflow. */
static int buf_reserve(IrcFramer *f, size_t extra) {
    if (extra > (size_t)-1 - f->len) return 0; /* len + extra would overflow */
    size_t need = f->len + extra;
    if (need <= f->cap) return 1;
    size_t cap = f->cap ? f->cap : 16;
    while (cap < need) {
        if (cap > ((size_t)-1) / 2) {
            cap = need; /* stop doubling near the ceiling; grow exactly */
            break;
        }
        cap *= 2;
    }
    unsigned char *nb = realloc(f->buf, cap);
    if (!nb) return 0;
    f->buf = nb;
    f->cap = cap;
    return 1;
}

/* Remove the first `k` bytes, shifting the remainder down. */
static void buf_drain_front(IrcFramer *f, size_t k) {
    if (k == 0) return;
    if (k >= f->len) {
        f->len = 0;
        return;
    }
    memmove(f->buf, f->buf + k, f->len - k);
    f->len -= k;
}

/* ===========================================================================
 *  Public API
 * =========================================================================== */

IrcFramer *irc_framer_new(void) {
    IrcFramer *f = malloc(sizeof *f);
    if (!f) return NULL;
    f->buf = NULL;
    f->len = 0;
    f->cap = 0;
    return f;
}

void irc_framer_free(IrcFramer *f) {
    if (!f) return;
    free(f->buf);
    free(f);
}

int irc_framer_feed(IrcFramer *f, const unsigned char *data, size_t len) {
    if (len == 0) return 0; /* empty feed is a safe no-op */
    if (!buf_reserve(f, len)) return -1;
    memcpy(f->buf + f->len, data, len);
    f->len += len;
    return 0;
}

void irc_framer_reset(IrcFramer *f) {
    free(f->buf);
    f->buf = NULL;
    f->len = 0;
    f->cap = 0;
}

size_t irc_framer_buffer_size(const IrcFramer *f) { return f->len; }

void irc_frames_free(IrcFrames *frames) {
    if (!frames) return;
    for (size_t i = 0; i < frames->count; i++) free(frames->frames[i].data);
    free(frames->frames);
    frames->frames = NULL;
    frames->count = 0;
}

int irc_framer_frames(IrcFramer *f, IrcFrames *out) {
    out->frames = NULL;
    out->count = 0;

    IrcFrame *arr = NULL;
    size_t n = 0, cap = 0;
    size_t cursor = 0;

    for (;;) {
        if (cursor >= f->len) break;
        /* Find the first LF at or after the cursor. */
        unsigned char *nl = memchr(f->buf + cursor, '\n', f->len - cursor);
        if (!nl) break;
        size_t lf_pos = (size_t)(nl - f->buf);

        /* Exclude a CR immediately before the LF (within the unconsumed region;
         * the byte at `cursor - 1`, if any, is always the previous frame's LF). */
        size_t content_end =
            (lf_pos > cursor && f->buf[lf_pos - 1] == '\r') ? lf_pos - 1 : lf_pos;
        size_t line_len = content_end - cursor;
        size_t next = lf_pos + 1;

        /* Discard overlong lines (RFC 1459 §2.3); still consume them. */
        if (line_len <= (size_t)IRC_MAX_CONTENT_BYTES) {
            unsigned char *line = malloc(line_len ? line_len : 1);
            if (!line) goto oom;
            memcpy(line, f->buf + cursor, line_len);

            if (n == cap) {
                size_t ncap = cap ? cap * 2 : 4;
                if (cap > ((size_t)-1) / 2 / sizeof *arr) {
                    free(line);
                    goto oom; /* array size would overflow */
                }
                IrcFrame *na = realloc(arr, ncap * sizeof *arr);
                if (!na) {
                    free(line);
                    goto oom;
                }
                arr = na;
                cap = ncap;
            }
            arr[n].data = line;
            arr[n].len = line_len;
            n++;
        }
        cursor = next;
    }

    /* Success: drain the whole consumed prefix in one move. */
    buf_drain_front(f, cursor);
    out->frames = arr;
    out->count = n;
    return 0;

oom:
    for (size_t i = 0; i < n; i++) free(arr[i].data);
    free(arr);
    /* Buffer left intact — the call can be retried. */
    return -1;
}
