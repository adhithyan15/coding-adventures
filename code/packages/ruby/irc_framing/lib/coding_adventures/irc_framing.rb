# frozen_string_literal: true

# irc_framing — TCP byte-stream → complete IRC lines
#
# == Overview
#
# TCP is a *stream* protocol.  The kernel may deliver bytes in arbitrarily
# sized chunks, so a single +recv+ call might return half a line, one line,
# or several lines at once.  This package solves that problem: it accumulates
# raw bytes and yields complete lines one at a time.
#
# == Framing rule (RFC 1459 §2.3)
#
# IRC messages are separated by CRLF ("\r\n").  Some clients send only LF
# ("\n"); we handle both.  The delimiter is stripped before yielding.
#
# == Overlong line protection
#
# RFC 1459 limits messages to 512 bytes (including CRLF).  Lines longer than
# 510 bytes (content limit) are silently discarded to prevent memory exhaustion
# from a malicious or buggy peer.  RFC 2812 section 2.3 keeps the same limit.
#
# That limit alone is not enough: it is only applied to lines that already
# ended in LF.  A peer that never sends LF would grow the buffer without bound
# (SEC-011, CWE-400/770).  So a line still lacking its LF after 511 bytes
# (510 plus a CR) is dropped early and the framer skips input up to the next LF.
#
# == Binary encoding
#
# IRC carries arbitrary byte sequences (nick names from different locales,
# DCC transfers, etc.).  We tag the internal buffer as +ASCII-8BIT+ (binary)
# via the +""+ +".b"+ idiom to avoid Ruby's encoding subsystem rejecting bytes
# > 127.

require_relative "irc_framing/version"

module CodingAdventures
  module IrcFraming
    # Maximum content length (bytes) per RFC 1459.
    # Lines longer than this are discarded.
    MAX_LINE_BYTES = 510

    # The most bytes we will hold for one line that has not yet seen its LF.
    #
    # A legal line can be 510 content bytes plus a CR whose LF has not arrived
    # yet, i.e. 511 bytes.  One more byte with no LF and the line can no longer
    # be legal (SEC-011), so we stop remembering it.
    MAX_PENDING_BYTES = MAX_LINE_BYTES + 1

    # Stateful byte-stream framer for IRC.
    #
    # Typical usage in a read loop:
    #
    #   framer = Framer.new
    #   loop do
    #     raw = socket.recv(4096)
    #     break if raw.empty?          # peer closed
    #     framer.feed(raw)
    #     framer.frames.each do |line|
    #       handle(line)               # line has no CRLF
    #     end
    #   end
    class Framer
      def initialize
        # Binary buffer — accepts any byte value without encoding errors.
        @buf = "".b
        # Length of the unterminated tail of @buf (bytes after the last LF).
        # Invariant: @tail_len <= MAX_PENDING_BYTES after every feed.
        @tail_len = 0
        # True while we are skipping the rest of an overlong line: everything
        # up to and including the next LF is thrown away, unread.
        @discarding = false
      end

      # Append raw bytes from the network to the internal buffer.
      #
      # == Memory bound (SEC-011)
      #
      # The unterminated tail (bytes after the last LF) never exceeds
      # MAX_PENDING_BYTES = 511.  When it would, the partial line is dropped and
      # the framer discards input up to the next LF -- fail closed, the same
      # silent discard that already applies to a complete overlong line, so the
      # API is unchanged.  Complete lines already buffered are kept; the caller
      # bounds those by calling +frames+ after each read.
      #
      #   data = [ ...complete lines... LF ][ unterminated tail ]
      #                                     ^ only this part can be flooded
      #
      # @param data [String] raw bytes (any encoding; forced to binary)
      # @return [self]
      def feed(data)
        data = data.b

        # Step 1: while discarding an overlong line, skip to just past its LF.
        if @discarding
          pos = data.index("\n".b)
          return self if pos.nil? # still inside the overlong line

          @discarding = false
          data = data.byteslice(pos + 1, data.bytesize - pos - 1)
        end

        # Step 2: split data at its last LF.  The head (through that LF)
        # completes lines and is stored; only the tail can be flooded.
        last_lf = data.rindex("\n".b)
        head_len = last_lf.nil? ? 0 : last_lf + 1
        tail = data.byteslice(head_len, data.bytesize - head_len)
        if head_len > 0
          @buf << data.byteslice(0, head_len)
          @tail_len = 0
        end

        # Step 3: store the tail only if the pending line still fits.  We check
        # *before* appending so a huge single feed is never copied.
        if @tail_len + tail.bytesize > MAX_PENDING_BYTES
          @buf = @buf.byteslice(0, @buf.bytesize - @tail_len) # drop partial line
          @tail_len = 0
          @discarding = true
        else
          @buf << tail
          @tail_len += tail.bytesize
        end
        self
      end

      # Extract and return all complete lines currently in the buffer.
      #
      # Each returned string has the CRLF (or bare LF) stripped and is encoded
      # in UTF-8 (with invalid bytes replaced) for downstream processing.
      #
      # Overlong lines (> MAX_LINE_BYTES bytes) are silently discarded.
      #
      # @return [Array<String>]
      def frames
        result = []

        loop do
          # Search for a newline (LF = 0x0A).
          nl = @buf.index("\n".b)
          break if nl.nil?

          # Extract the content up to (but not including) the LF.
          content = @buf.byteslice(0, nl)

          # Consume the content + LF from the buffer.
          @buf = @buf.byteslice(nl + 1, @buf.bytesize - nl - 1) || "".b

          # Strip trailing CR if present (CRLF → CR gone; bare LF → nothing).
          content = content.byteslice(0, content.bytesize - 1) \
            if content.end_with?("\r".b)

          # Discard overlong lines.
          next if content.bytesize > MAX_LINE_BYTES

          # Convert to UTF-8 for downstream (replace invalid bytes).
          result << content.encode("UTF-8", "binary",
                                   invalid: :replace, undef: :replace)
        end

        # Only the unterminated tail remains in the buffer.
        @tail_len = @buf.bytesize
        result
      end

      # Clear the internal buffer.  Call after a connection is closed.
      #
      # @return [self]
      def reset
        @buf = "".b
        @tail_len = 0
        @discarding = false
        self
      end

      # Current number of bytes in the internal buffer.
      #
      # Useful for enforcing a maximum buffer size per connection.
      #
      # @return [Integer]
      def buffer_size
        @buf.bytesize
      end
    end
  end
end
