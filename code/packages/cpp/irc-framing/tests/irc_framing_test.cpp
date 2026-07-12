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

    return ISO_TEST_RESULT();
}
