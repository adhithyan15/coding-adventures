/*
 * irc_framing.h — a stateful byte-stream-to-line-frame converter, in pure ISO
 * C17. A faithful port of the Rust `irc-framing` crate.
 * ===========================================================================
 *
 * THE PROBLEM. TCP delivers a byte stream, not messages: one read() may hand
 * you half a message, one message, or several. IRC frames messages with a
 * trailing CRLF (or a lone LF); this framer absorbs raw byte chunks and emits
 * the complete, CRLF-stripped lines to the layer above (e.g. irc-proto).
 *
 * RFC 1459 §2.3. A message is at most 512 bytes including CRLF — so at most 510
 * bytes of content. Lines whose content exceeds 510 bytes are silently
 * discarded.
 *
 *     irc-proto    ← receives complete, CRLF-stripped bytes
 *          ↑
 *     irc-framing  ← THIS: irc_framer_feed(bytes); irc_framer_frames() → lines
 *          ↑
 *     socket read  ← feeds raw bytes upward
 *
 * A Framer is NOT thread-safe; each connection should own one.
 *
 * MEMORY BOUND (SEC-011, CWE-400/770). The 510-byte cap alone is applied only
 * once a line's terminator is seen, so on its own it would let a peer that
 * streams bytes with no LF grow the buffer without limit. `feed` therefore also
 * caps the *unterminated tail* (the bytes after the last LF) at
 * IRC_MAX_PENDING_BYTES = 511: 510 content bytes plus a CR whose LF has not
 * arrived yet (RFC 1459 and RFC 2812 §2.3: 512 bytes including CRLF). One more
 * byte with no LF and the line cannot be legal, so the partial line is dropped
 * and the framer discards input up to and including the next LF, keeping no
 * data. Memory for the pending line stays constant however much is fed. This
 * is a silent, fail-closed discard like the one for complete overlong lines, so
 * the API is unchanged. Complete lines that are already buffered are kept; the
 * caller bounds those by calling `irc_framer_frames` after each read.
 *
 * OWNERSHIP. Create with `irc_framer_new`, release with `irc_framer_free`.
 * `irc_framer_frames` returns an `IrcFrames` batch the caller releases with
 * `irc_frames_free`. Frame contents are raw bytes (any byte value, not
 * NUL-terminated); each carries its own length.
 *
 * PORTABILITY. Pure ISO C17 — no extensions. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_IRC_FRAMING_H
#define CA_IRC_FRAMING_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* RFC 1459 §2.3: maximum line content is 510 bytes (512 − CRLF). */
#define IRC_MAX_CONTENT_BYTES 510

/* The most bytes held for one line that has not yet seen its LF: 510 content
 * bytes plus a CR (SEC-011). */
#define IRC_MAX_PENDING_BYTES (IRC_MAX_CONTENT_BYTES + 1)

/* Opaque stateful framer, holding an internal byte buffer. */
typedef struct IrcFramer IrcFramer;

/* One complete, CRLF-stripped line. `data` is malloc'd raw bytes (may contain
 * any value, including embedded NULs); `len` is its length. A zero-length frame
 * (from an empty line) still has a non-NULL `data`. */
typedef struct {
    unsigned char *data;
    size_t len;
} IrcFrame;

/* A batch of frames drained from the buffer. */
typedef struct {
    IrcFrame *frames; /* NULL when count == 0 */
    size_t count;
} IrcFrames;

/* Create a framer with an empty buffer. Returns NULL on allocation failure. */
IrcFramer *irc_framer_new(void);

/* Release a framer and its buffer. */
void irc_framer_free(IrcFramer *f);

/* Append `len` bytes of `data` to the internal buffer. A zero length is a safe
 * no-op. The unterminated tail never exceeds IRC_MAX_PENDING_BYTES; past that
 * the partial line is dropped and input is skipped up to the next LF (see
 * MEMORY BOUND above). Returns 0 on success, -1 on allocation failure (state
 * unchanged). */
int irc_framer_feed(IrcFramer *f, const unsigned char *data, size_t len);

/* Drain all complete frames (CRLF stripped) into *out; overlong lines (content
 * > 510 bytes) are silently discarded. Returns 0 on success (release *out with
 * `irc_frames_free`), or -1 on allocation failure (the buffer is left intact so
 * the call can be retried). */
int irc_framer_frames(IrcFramer *f, IrcFrames *out);

/* Release a frame batch returned by `irc_framer_frames`. */
void irc_frames_free(IrcFrames *frames);

/* Discard all buffered data, including the skip-to-LF discard state. */
void irc_framer_reset(IrcFramer *f);

/* Number of bytes currently held in the internal buffer. */
size_t irc_framer_buffer_size(const IrcFramer *f);

#ifdef __cplusplus
}
#endif

#endif /* CA_IRC_FRAMING_H */
