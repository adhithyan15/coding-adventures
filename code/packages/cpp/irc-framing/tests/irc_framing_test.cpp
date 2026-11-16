// Tests for the C++ irc-framing Framer, using the header-only iso_test.h harness
// (pure ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <string>
#include <vector>

#include "irc_framing.hpp"

using ca::irc::Framer;
using Frame = std::vector<unsigned char>;

// Does `frame` equal the bytes of `want`?
static bool eq(const Frame& frame, const std::string& want) {
    if (frame.size() != want.size()) return false;
    for (std::size_t i = 0; i < frame.size(); i++) {
        if (frame[i] != static_cast<unsigned char>(want[i])) return false;
    }
    return true;
}

int main() {
    // ── a single CRLF-terminated message ─────────────────────────────────
    {
        Framer f;
        f.feed(std::string("NICK alice\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        ISO_CHECK(eq(fr[0], "NICK alice"));
    }

    // ── a lone LF is also a terminator ───────────────────────────────────
    {
        Framer f;
        f.feed(std::string("NICK alice\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        ISO_CHECK(eq(fr[0], "NICK alice"));
    }

    // ── several messages in one feed ─────────────────────────────────────
    {
        Framer f;
        f.feed(std::string("NICK alice\r\nUSER alice 0 * :Alice\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 2u);
        ISO_CHECK(eq(fr[0], "NICK alice"));
        ISO_CHECK(eq(fr[1], "USER alice 0 * :Alice"));
    }

    // ── a partial message is buffered until complete ─────────────────────
    {
        Framer f;
        f.feed(std::string("NICK al"));
        ISO_CHECK(f.frames().empty());
        ISO_CHECK_EQ_UINT(f.buffer_size(), 7u);
        f.feed(std::string("ice\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        ISO_CHECK(eq(fr[0], "NICK alice"));
        ISO_CHECK_EQ_UINT(f.buffer_size(), 0u);
    }

    // ── a feed split across the CR/LF boundary ───────────────────────────
    {
        Framer f;
        f.feed(std::string("NICK alice\r"));
        ISO_CHECK(f.frames().empty());
        f.feed(std::string("\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        ISO_CHECK(eq(fr[0], "NICK alice"));
    }

    // ── an empty feed is a no-op ─────────────────────────────────────────
    {
        Framer f;
        f.feed(std::string(""));
        ISO_CHECK_EQ_UINT(f.buffer_size(), 0u);
        ISO_CHECK(f.frames().empty());
    }

    // ── a bare CRLF yields one empty frame ───────────────────────────────
    {
        Framer f;
        f.feed(std::string("\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        ISO_CHECK_EQ_UINT(fr[0].size(), 0u);
    }

    // ── an overlong line (content > 510) is discarded ────────────────────
    {
        Framer f;
        f.feed(std::string(511, 'A'));
        f.feed(std::string("\r\n"));
        ISO_CHECK(f.frames().empty());
    }

    // ── exactly 510 bytes is accepted ────────────────────────────────────
    {
        Framer f;
        f.feed(std::string(510, 'A'));
        f.feed(std::string("\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        ISO_CHECK_EQ_UINT(fr[0].size(), 510u);
    }

    // ── an overlong line followed by a valid one ─────────────────────────
    {
        Framer f;
        f.feed(std::string(511, 'X'));
        f.feed(std::string("\r\nNICK alice\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        ISO_CHECK(eq(fr[0], "NICK alice"));
    }

    // ── reset discards buffered data ─────────────────────────────────────
    {
        Framer f;
        f.feed(std::string("partial data"));
        f.reset();
        ISO_CHECK_EQ_UINT(f.buffer_size(), 0u);
        ISO_CHECK(f.frames().empty());
    }

    // ── three messages split across two feeds ────────────────────────────
    {
        Framer f;
        f.feed(std::string("JOIN #one\r\nJOIN"));
        auto f1 = f.frames();
        ISO_CHECK_EQ_UINT(f1.size(), 1u);
        ISO_CHECK(eq(f1[0], "JOIN #one"));
        f.feed(std::string(" #two\r\nJOIN #three\r\n"));
        auto f2 = f.frames();
        ISO_CHECK_EQ_UINT(f2.size(), 2u);
        ISO_CHECK(eq(f2[0], "JOIN #two"));
        ISO_CHECK(eq(f2[1], "JOIN #three"));
    }

    // ════════════════════════════════════════════════════════════════════
    // SEC-011: a peer that never sends LF must not grow the buffer.
    //
    // The cap on an unterminated line is 511 bytes: 510 content bytes plus a
    // CR whose LF has not arrived yet (RFC 1459 / RFC 2812 section 2.3: 512
    // bytes including CRLF).  These tests mirror the Rust crate's.
    // ════════════════════════════════════════════════════════════════════

    // ── 16 MiB in 4 KiB chunks, no LF: the buffer never exceeds 512 ──────
    {
        Framer f;
        const std::string chunk(4096, 'A');
        std::size_t worst = 0;
        for (std::size_t i = 0; i < (16u * 1024u * 1024u) / chunk.size(); i++) {
            f.feed(chunk);
            if (f.buffer_size() > worst) worst = f.buffer_size();
        }
        ISO_CHECK(worst <= 512u);
        ISO_CHECK_EQ_UINT(f.frames().size(), 0u);
    }

    // ── 16 MiB in ONE feed, no LF: bounded ───────────────────────────────
    {
        Framer f;
        f.feed(std::string(16u * 1024u * 1024u, 'A'));
        ISO_CHECK(f.buffer_size() <= 512u);
        ISO_CHECK_EQ_UINT(f.frames().size(), 0u);
    }

    // ── recovery: the flooded line is dropped whole, the next frames ─────
    {
        Framer f;
        f.feed(std::string(4096, 'A'));
        f.feed(std::string(4096, 'B'));
        f.feed(std::string("tail\r\nNICK alice\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        if (fr.size() == 1) ISO_CHECK(eq(fr[0], "NICK alice"));
        ISO_CHECK_EQ_UINT(f.buffer_size(), 0u);
    }

    // ── 510 content bytes + CR is a legal pending line: kept ─────────────
    {
        Framer f;
        f.feed(std::string(510, 'A') + "\r");
        ISO_CHECK_EQ_UINT(f.buffer_size(), 511u);
        f.feed(std::string("\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        if (fr.size() == 1) ISO_CHECK_EQ_UINT(fr[0].size(), 510u);
    }

    // ── 512 bytes with no LF: dropped, then skip to the LF ───────────────
    {
        Framer f;
        f.feed(std::string(512, 'A'));
        ISO_CHECK_EQ_UINT(f.buffer_size(), 0u);
        f.feed(std::string("\r\nPING x\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        if (fr.size() == 1) ISO_CHECK(eq(fr[0], "PING x"));
    }

    // ── complete lines that arrived before the flood survive ─────────────
    {
        Framer f;
        f.feed(std::string("NICK a\r\n") + std::string(2000, 'A'));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        if (fr.size() == 1) ISO_CHECK(eq(fr[0], "NICK a"));
        ISO_CHECK_EQ_UINT(f.buffer_size(), 0u);
    }

    // ── reset() ends the skip-to-LF state ────────────────────────────────
    {
        Framer f;
        f.feed(std::string(600, 'A'));
        f.reset();
        f.feed(std::string("NICK a\r\n"));
        auto fr = f.frames();
        ISO_CHECK_EQ_UINT(fr.size(), 1u);
        if (fr.size() == 1) ISO_CHECK(eq(fr[0], "NICK a"));
    }

    return ISO_TEST_RESULT();
}
