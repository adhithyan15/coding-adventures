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
    /* Length of the unterminated tail of `buf` (the bytes after the last LF).
     * Invariant: tail_len <= IRC_MAX_PENDING_BYTES after every feed. */
    size_t tail_len;
    /* Non-zero while we are skipping the rest of an overlong line: everything
     * up to and including the next LF is thrown away, unread. */
    int discarding;
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
    f->tail_len = 0;
    f->discarding = 0;
    return f;
}

void irc_framer_free(IrcFramer *f) {
    if (!f) return;
    free(f->buf);
    free(f);
}

/* Number of bytes of data[0..len) up to and including its last LF (0 if none). */
static size_t last_lf_end(const unsigned char *data, size_t len) {
    while (len > 0) {
        if (data[len - 1] == '\n') return len;
        len--;
    }
    return 0;
}

/* Memory bound (SEC-011). The unterminated tail never exceeds
 * IRC_MAX_PENDING_BYTES; past that the partial line is dropped and input is
 * skipped up to the next LF.
 *
 *     data = [ ...complete lines... LF ][ unterminated tail ]
 *                                       ^ only this part can be flooded
 *
 * Everything that can fail (the reserve) happens before any state changes, so
 * an allocation failure leaves the framer exactly as it was. */
int irc_framer_feed(IrcFramer *f, const unsigned char *data, size_t len) {
    if (len == 0) return 0; /* empty feed is a safe no-op */

    /* Step 1: while discarding an overlong line, skip to just past its LF. */
    if (f->discarding) {
        const unsigned char *nl = memchr(data, '\n', len);
        if (!nl) return 0; /* still inside the overlong line: keep nothing */
        len -= (size_t)(nl - data) + 1;
        data = nl + 1;
    }

    /* Step 2: split data at its last LF. The head (through that LF) completes
     * lines and is stored; only the tail can be flooded. */
    size_t head = last_lf_end(data, len);
    size_t tail = len - head;
    size_t pending = head ? 0 : f->tail_len;

    /* Step 3: store the tail only if the pending line still fits. We decide
     * *before* copying so a huge single feed is never copied. */
    int overflow = tail > (size_t)IRC_MAX_PENDING_BYTES - pending;
    if (!buf_reserve(f, head + (overflow ? 0 : tail))) return -1;

    /* Commit. */
    f->discarding = 0;
    if (head) {
        memcpy(f->buf + f->len, data, head);
        f->len += head;
        f->tail_len = 0;
    }
    if (overflow) {
        f->len -= f->tail_len; /* drop the partial line */
        f->tail_len = 0;
        f->discarding = 1;
    } else if (tail) {
        memcpy(f->buf + f->len, data + head, tail);
        f->len += tail;
        f->tail_len += tail;
    }
    return 0;
}

void irc_framer_reset(IrcFramer *f) {
    free(f->buf);
    f->buf = NULL;
    f->len = 0;
    f->cap = 0;
    f->tail_len = 0;
    f->discarding = 0;
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
    f->tail_len = f->len; /* only the unterminated tail remains */
    out->frames = arr;
    out->count = n;
    return 0;

oom:
    for (size_t i = 0; i < n; i++) free(arr[i].data);
    free(arr);
    /* Buffer left intact — the call can be retried. */
    return -1;
}
