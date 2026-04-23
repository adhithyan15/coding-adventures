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
# from a malicious or buggy peer.
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
      end

      # Append raw bytes from the network to the internal buffer.
      #
      # @param data [String] raw bytes (any encoding; forced to binary)
      # @return [self]
      def feed(data)
        @buf << data.b
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

        result
      end

      # Clear the internal buffer.  Call after a connection is closed.
      #
      # @return [self]
      def reset
        @buf = "".b
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
