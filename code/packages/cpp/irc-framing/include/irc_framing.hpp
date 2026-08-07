// irc_framing.hpp — a stateful byte-stream-to-line-frame converter, in pure ISO
// C++17, header-only, in namespace ca::irc. A faithful port of the Rust
// `irc-framing` crate.
// ===========================================================================
//
// TCP delivers a byte stream, not messages: one read may hand you half a
// message, one, or several. IRC frames messages with a trailing CRLF (or a lone
// LF); this Framer absorbs raw byte chunks and emits complete, CRLF-stripped
// lines to the layer above (e.g. ca::irc::parse).
//
// RFC 1459 §2.3: a message is at most 512 bytes including CRLF — at most 510
// bytes of content. Lines whose content exceeds 510 bytes are silently
// discarded.
//
// A Framer is NOT thread-safe; each connection should own one. Frames are raw
// byte vectors (any byte value), so a frame is std::vector<unsigned char>.
//
// SECURITY / UNBOUNDED BUFFER. Like the Rust original, feed() buffers whatever
// it is given: the 510-byte cap only applies to a line once its terminator is
// seen, so a peer that streams bytes with no LF grows the buffer without limit.
// This is by design — the read layer above (which owns the socket) must bound
// how much it reads before a terminator, as in the Rust stack. Do not feed
// unbounded untrusted input without such a cap.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_IRC_FRAMING_HPP
#define CA_IRC_FRAMING_HPP

#include <algorithm>
#include <cstddef>
#include <string>
#include <vector>

namespace ca {
namespace irc {

class Framer {
public:
    // RFC 1459 §2.3: maximum line content is 510 bytes (512 − CRLF).
    static constexpr std::size_t MAX_CONTENT_BYTES = 510;

    Framer() = default;

    // Append raw bytes to the internal buffer. A zero length is a safe no-op.
    void feed(const unsigned char* data, std::size_t len) {
        buf_.insert(buf_.end(), data, data + len);
    }
    // Convenience overload for text (or any byte string).
    void feed(const std::string& data) {
        feed(reinterpret_cast<const unsigned char*>(data.data()), data.size());
    }

    // Drain all complete frames (CRLF stripped); overlong lines (content > 510
    // bytes) are silently discarded.
    std::vector<std::vector<unsigned char>> frames() {
        std::vector<std::vector<unsigned char>> result;
        std::size_t cursor = 0;
        while (cursor < buf_.size()) {
            // Find the first LF at or after the cursor.
            auto it = std::find(buf_.begin() + static_cast<std::ptrdiff_t>(cursor),
                                buf_.end(), static_cast<unsigned char>('\n'));
            if (it == buf_.end()) break;
            std::size_t lf_pos = static_cast<std::size_t>(it - buf_.begin());

            // Exclude a CR immediately before the LF (within the unconsumed
            // region; the byte at cursor-1, if any, is the previous frame's LF).
            std::size_t content_end =
                (lf_pos > cursor && buf_[lf_pos - 1] == '\r') ? lf_pos - 1 : lf_pos;
            std::size_t line_len = content_end - cursor;

            // Discard overlong lines (RFC 1459 §2.3); still consume them.
            if (line_len <= MAX_CONTENT_BYTES) {
                result.emplace_back(
                    buf_.begin() + static_cast<std::ptrdiff_t>(cursor),
                    buf_.begin() + static_cast<std::ptrdiff_t>(content_end));
            }
            cursor = lf_pos + 1;
        }
        // Drain the consumed prefix in one move.
        buf_.erase(buf_.begin(), buf_.begin() + static_cast<std::ptrdiff_t>(cursor));
        return result;
    }

    // Discard all buffered data.
    void reset() { buf_.clear(); }

    // Number of bytes currently held in the internal buffer.
    std::size_t buffer_size() const { return buf_.size(); }

private:
    std::vector<unsigned char> buf_;
};

}  // namespace irc
}  // namespace ca

#endif  // CA_IRC_FRAMING_HPP
