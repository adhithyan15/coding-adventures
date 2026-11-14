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
// MEMORY BOUND (SEC-011, CWE-400/770). The 510-byte cap alone is applied only
// once a line's terminator is seen, so on its own it would let a peer that
// streams bytes with no LF grow the buffer without limit. feed() therefore also
// caps the *unterminated tail* (the bytes after the last LF) at
// MAX_PENDING_BYTES = 511: 510 content bytes plus a CR whose LF has not
// arrived yet (RFC 1459 and RFC 2812 §2.3: 512 bytes including CRLF). One more
// byte with no LF and the line cannot be legal, so the partial line is dropped
// and the framer discards input up to and including the next LF, keeping no
// data. This is a silent, fail-closed discard like the one for complete
// overlong lines, so the API is unchanged. Complete lines that are already
// buffered are kept; the caller bounds those by calling frames() after each
// read.
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

    // The most bytes held for one line that has not yet seen its LF: 510
    // content bytes plus a CR (SEC-011).
    static constexpr std::size_t MAX_PENDING_BYTES = MAX_CONTENT_BYTES + 1;

    Framer() = default;

    // Append raw bytes to the internal buffer. A zero length is a safe no-op.
    // The unterminated tail never exceeds MAX_PENDING_BYTES; past that the
    // partial line is dropped and input is skipped up to the next LF.
    //
    //     data = [ ...complete lines... LF ][ unterminated tail ]
    //                                       ^ only this part can be flooded
    void feed(const unsigned char* data, std::size_t len) {
        // Step 1: while discarding an overlong line, skip to just past its LF.
        if (discarding_) {
            const unsigned char* end = data + len;
            const unsigned char* nl = std::find(data, end, static_cast<unsigned char>('\n'));
            if (nl == end) return;  // still inside the overlong line
            discarding_ = false;
            len -= static_cast<std::size_t>(nl - data) + 1;
            data = nl + 1;
        }

        // Step 2: split data at its last LF. The head (through that LF)
        // completes lines and is stored; only the tail can be flooded.
        std::size_t head = len;
        while (head > 0 && data[head - 1] != '\n') head--;
        std::size_t tail = len - head;
        if (head > 0) {
            buf_.insert(buf_.end(), data, data + head);
            tail_len_ = 0;
        }

        // Step 3: store the tail only if the pending line still fits. We check
        // *before* inserting so a huge single feed is never copied.
        if (tail > MAX_PENDING_BYTES - tail_len_) {
            buf_.resize(buf_.size() - tail_len_);  // drop the partial line
            tail_len_ = 0;
            discarding_ = true;
        } else {
            buf_.insert(buf_.end(), data + head, data + len);
            tail_len_ += tail;
        }
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
        tail_len_ = buf_.size();  // only the unterminated tail remains
        return result;
    }

    // Discard all buffered data, including the skip-to-LF discard state.
    void reset() {
        buf_.clear();
        tail_len_ = 0;
        discarding_ = false;
    }

    // Number of bytes currently held in the internal buffer.
    std::size_t buffer_size() const { return buf_.size(); }

private:
    std::vector<unsigned char> buf_;
    // Length of the unterminated tail of buf_ (bytes after the last LF).
    // Invariant: tail_len_ <= MAX_PENDING_BYTES after every feed.
    std::size_t tail_len_ = 0;
    // True while skipping the rest of an overlong line: everything up to and
    // including the next LF is thrown away, unread.
    bool discarding_ = false;
};

}  // namespace irc
}  // namespace ca

#endif  // CA_IRC_FRAMING_HPP
