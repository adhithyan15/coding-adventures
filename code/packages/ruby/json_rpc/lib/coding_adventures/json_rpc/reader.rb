# frozen_string_literal: true

# ================================================================
# CodingAdventures::JsonRpc::MessageReader
# ================================================================
#
# Reads one Content-Length-framed JSON-RPC message at a time from
# an IO object (typically STDIN, but any IO works — including
# StringIO for testing).
#
# Wire format:
#
#   Content-Length: 97\r\n
#   \r\n
#   {"jsonrpc":"2.0","id":1,"method":"textDocument/hover",...}
#
# Why Content-Length framing?
# ---------------------------
# JSON has no self-delimiting structure at the byte stream level.
# You cannot tell where one JSON object ends without parsing the
# whole thing. Content-Length solves this: read headers, find the
# length, then read exactly that many bytes — no buffering beyond
# what is needed.
#
# Implementation notes
# --------------------
# We read the stream with IO#read(n), which blocks until n bytes
# are available or EOF is reached. For the header we read one byte
# at a time until we see the \r\n\r\n sentinel. This is simple and
# correct; performance is fine for the LSP use case where messages
# are infrequent relative to editor interaction.
#
# Production note: call STDIN.binmode before creating a reader so
# that Ruby does not apply line-ending translation or encoding
# conversion to the raw bytes.
#
# ================================================================

require "json"
require_relative "errors"
require_relative "message"

module CodingAdventures
  module JsonRpc
    class MessageReader
      # @param stream [IO] any readable IO object (STDIN, StringIO, pipe, etc.)
      def initialize(stream)
        @stream = stream
      end

      # ----------------------------------------------------------------
      # read_message — read, frame, parse, return typed Message
      # ----------------------------------------------------------------
      #
      # Reads the next Content-Length-framed message from the stream and
      # returns a typed Request, Notification, or Response.
      #
      # Returns nil on clean EOF (stream closed with no partial message).
      # Raises JsonRpc::Error(-32700) on malformed JSON.
      # Raises JsonRpc::Error(-32600) on valid JSON that is not a message.
      #
      def read_message
        raw = read_raw
        return nil if raw.nil?

        parsed = begin
          JSON.parse(raw)
        rescue JSON::ParserError => e
          raise Error.new(
            ErrorCodes::PARSE_ERROR,
            "Parse error: invalid JSON — #{e.message}"
          )
        end

        JsonRpc.parse_message(parsed)
      end

      # ----------------------------------------------------------------
      # read_raw — read one framed message as a raw JSON string
      # ----------------------------------------------------------------
      #
      # Returns the JSON string without parsing it. Useful for testing
      # or for proxy scenarios where the caller controls parsing.
      #
      # Returns nil on EOF.
      #
      def read_raw
        # Step 1: Read headers up to the \r\n\r\n blank line.
        header_bytes = read_until_blank_line
        return nil if header_bytes.nil?

        # Step 2: Extract Content-Length from the headers.
        content_length = parse_content_length(header_bytes)

        # Step 3: Read exactly content_length bytes as the JSON payload.
        payload = @stream.read(content_length)
        if payload.nil? || payload.bytesize < content_length
          raise Error.new(
            ErrorCodes::PARSE_ERROR,
            "Parse error: stream ended before payload was complete"
          )
        end

        # Force UTF-8 encoding so that string operations work correctly.
        payload.force_encoding("UTF-8")
      end

      private

      # Read bytes one at a time until we see the sequence \r\n\r\n.
      #
      # Returns the bytes before the blank line (the header block) as a
      # String. Returns nil if the stream ends before any bytes arrive.
      #
      # We scan for \r\n\r\n byte-by-byte because the header can contain
      # arbitrary Content-Type lines we want to skip. Reading one byte at
      # a time is safe here — headers are short (< 100 bytes typically).
      #
      def read_until_blank_line
        buffer = "".b # binary string
        sentinel = "\r\n\r\n".b

        loop do
          byte = @stream.read(1)
          # EOF before any bytes → clean EOF
          return nil if byte.nil? && buffer.empty?
          # EOF mid-header → treat as clean EOF (no partial message)
          return nil if byte.nil?

          buffer << byte

          # Check if we have seen the blank-line sentinel at the end.
          if buffer.end_with?(sentinel)
            # Return everything before the sentinel.
            return buffer[0, buffer.bytesize - sentinel.bytesize]
          end
        end
      end

      # Parse the Content-Length value from the header block string.
      #
      # The header block looks like:
      #   Content-Length: 97\r\n
      #   Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n
      #
      # We split on \r\n and search for the Content-Length line
      # (case-insensitive, following the HTTP convention).
      #
      def parse_content_length(header_bytes)
        header_str = header_bytes.force_encoding("ASCII-8BIT")
        lines = header_str.split("\r\n")
        lines.each do |line|
          if line.downcase.start_with?("content-length:")
            value = line.split(":", 2).last.strip
            n = Integer(value, 10)
            raise Error.new(ErrorCodes::PARSE_ERROR, "Parse error: Content-Length must be non-negative") if n < 0
            return n
          end
        end
        raise Error.new(
          ErrorCodes::PARSE_ERROR,
          "Parse error: missing Content-Length header"
        )
      rescue ArgumentError
        raise Error.new(
          ErrorCodes::PARSE_ERROR,
          "Parse error: invalid Content-Length value"
        )
      end
    end
  end
end
