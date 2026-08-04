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

    return ISO_TEST_RESULT();
}
