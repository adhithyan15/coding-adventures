/*
 * Tests for the C irc-framing framer, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own unit tests — CRLF and lone-LF
 * framing, partial buffering across feeds, the 510-byte overlong-line rule, and
 * reset.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "irc_framing.h"

/* Feed a NUL-terminated literal (its bytes, not the NUL). */
static int feed_str(IrcFramer *f, const char *s) {
    return irc_framer_feed(f, (const unsigned char *)s, strlen(s));
}

/* Assert frame `i` equals the bytes of `want` (compared by length + content). */
static void chk_frame(const IrcFrames *fr, size_t i, const char *want) {
    ISO_CHECK(i < fr->count);
    if (i < fr->count) {
        size_t wlen = strlen(want);
        ISO_CHECK_EQ_UINT(fr->frames[i].len, (unsigned)wlen);
        if (fr->frames[i].len == wlen && wlen > 0) {
            ISO_CHECK_MEM_EQ(fr->frames[i].data, want, wlen);
        }
    }
}

int main(void) {
    /* ── a single CRLF-terminated message ───────────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        ISO_CHECK(f != NULL);
        feed_str(f, "NICK alice\r\n");
        IrcFrames fr;
        ISO_CHECK(irc_framer_frames(f, &fr) == 0);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK alice");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── a lone LF (no CR) is also a terminator ─────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "NICK alice\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK alice");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── several messages in one feed ───────────────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "NICK alice\r\nUSER alice 0 * :Alice\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 2u);
        chk_frame(&fr, 0, "NICK alice");
        chk_frame(&fr, 1, "USER alice 0 * :Alice");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── a partial message is buffered until complete ───────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "NICK al");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 0u);
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 7u);
        irc_frames_free(&fr);

        feed_str(f, "ice\r\n");
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK alice");
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 0u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── a feed split across the CR/LF boundary ─────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "NICK alice\r");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 0u);
        irc_frames_free(&fr);

        feed_str(f, "\n");
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK alice");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── an empty feed is a no-op ───────────────────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "");
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 0u);
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 0u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── a bare CRLF yields one empty frame ─────────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        ISO_CHECK_EQ_UINT(fr.frames[0].len, 0u); /* empty line */
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── an overlong line (content > 510) is discarded ──────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        char overlong[511];
        memset(overlong, 'A', 511);
        irc_framer_feed(f, (const unsigned char *)overlong, 511);
        feed_str(f, "\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 0u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── exactly 510 bytes is accepted ──────────────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        char exact[510];
        memset(exact, 'A', 510);
        irc_framer_feed(f, (const unsigned char *)exact, 510);
        feed_str(f, "\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        ISO_CHECK_EQ_UINT(fr.frames[0].len, 510u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── an overlong line followed by a valid one ───────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        char over[511];
        memset(over, 'X', 511);
        irc_framer_feed(f, (const unsigned char *)over, 511);
        feed_str(f, "\r\nNICK alice\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK alice");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── reset discards buffered data ───────────────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "partial data");
        irc_framer_reset(f);
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 0u);
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 0u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── three messages split across two feeds ──────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        feed_str(f, "JOIN #one\r\nJOIN");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "JOIN #one");
        irc_frames_free(&fr);

        feed_str(f, " #two\r\nJOIN #three\r\n");
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 2u);
        chk_frame(&fr, 0, "JOIN #two");
        chk_frame(&fr, 1, "JOIN #three");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ════════════════════════════════════════════════════════════════════
     * SEC-011: a peer that never sends LF must not grow the buffer.
     *
     * The cap on an unterminated line is 511 bytes: 510 content bytes plus
     * a CR whose LF has not arrived yet (RFC 1459 / RFC 2812 section 2.3:
     * 512 bytes including CRLF).  These tests mirror the Rust crate's.
     * ════════════════════════════════════════════════════════════════════ */

    /* ── 16 MiB in 4 KiB chunks, no LF: the buffer never exceeds 512 ────── */
    {
        IrcFramer *f = irc_framer_new();
        unsigned char chunk[4096];
        memset(chunk, 'A', sizeof chunk);
        size_t worst = 0;
        for (size_t i = 0; i < (16u * 1024u * 1024u) / sizeof chunk; i++) {
            irc_framer_feed(f, chunk, sizeof chunk);
            if (irc_framer_buffer_size(f) > worst) worst = irc_framer_buffer_size(f);
        }
        ISO_CHECK(worst <= 512u);
        IrcFrames fr;
        ISO_CHECK(irc_framer_frames(f, &fr) == 0);
        ISO_CHECK_EQ_UINT(fr.count, 0u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── 16 MiB in ONE feed, no LF: bounded ─────────────────────────────── */
    {
        size_t big = 16u * 1024u * 1024u;
        unsigned char *data = malloc(big);
        ISO_CHECK(data != NULL);
        if (data) {
            memset(data, 'A', big);
            IrcFramer *f = irc_framer_new();
            ISO_CHECK(irc_framer_feed(f, data, big) == 0);
            ISO_CHECK(irc_framer_buffer_size(f) <= 512u);
            IrcFrames fr;
            irc_framer_frames(f, &fr);
            ISO_CHECK_EQ_UINT(fr.count, 0u);
            irc_frames_free(&fr);
            irc_framer_free(f);
            free(data);
        }
    }

    /* ── recovery: the flooded line is dropped whole, the next frames ───── */
    {
        IrcFramer *f = irc_framer_new();
        unsigned char blk[4096];
        memset(blk, 'A', sizeof blk);
        irc_framer_feed(f, blk, sizeof blk);
        memset(blk, 'B', sizeof blk);
        irc_framer_feed(f, blk, sizeof blk);
        feed_str(f, "tail\r\nNICK alice\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK alice");
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 0u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── 510 content bytes + CR is a legal pending line: kept ───────────── */
    {
        IrcFramer *f = irc_framer_new();
        unsigned char line[511];
        memset(line, 'A', 510);
        line[510] = '\r';
        irc_framer_feed(f, line, sizeof line);
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 511u);
        feed_str(f, "\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        if (fr.count == 1) ISO_CHECK_EQ_UINT(fr.frames[0].len, 510u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── 512 bytes with no LF: dropped, then skip to the LF ─────────────── */
    {
        IrcFramer *f = irc_framer_new();
        unsigned char line[512];
        memset(line, 'A', sizeof line);
        irc_framer_feed(f, line, sizeof line);
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 0u);
        feed_str(f, "\r\nPING x\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "PING x");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── complete lines that arrived before the flood survive ───────────── */
    {
        IrcFramer *f = irc_framer_new();
        unsigned char data[8 + 2000];
        memcpy(data, "NICK a\r\n", 8);
        memset(data + 8, 'A', 2000);
        irc_framer_feed(f, data, sizeof data);
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK a");
        ISO_CHECK_EQ_UINT(irc_framer_buffer_size(f), 0u);
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    /* ── reset() ends the skip-to-LF state ──────────────────────────────── */
    {
        IrcFramer *f = irc_framer_new();
        unsigned char line[600];
        memset(line, 'A', sizeof line);
        irc_framer_feed(f, line, sizeof line);
        irc_framer_reset(f);
        feed_str(f, "NICK a\r\n");
        IrcFrames fr;
        irc_framer_frames(f, &fr);
        ISO_CHECK_EQ_UINT(fr.count, 1u);
        chk_frame(&fr, 0, "NICK a");
        irc_frames_free(&fr);
        irc_framer_free(f);
    }

    return ISO_TEST_RESULT();
}
