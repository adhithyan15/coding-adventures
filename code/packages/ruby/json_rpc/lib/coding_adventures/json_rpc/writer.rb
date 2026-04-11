# frozen_string_literal: true

# ================================================================
# CodingAdventures::JsonRpc::MessageWriter
# ================================================================
#
# Writes one Content-Length-framed JSON-RPC message at a time to
# an IO object (typically STDOUT, but any IO works — including
# StringIO for testing).
#
# Framing format:
#
#   Content-Length: <n>\r\n
#   \r\n
#   <UTF-8 JSON payload, exactly n bytes>
#
# The Content-Length value is the BYTE length of the UTF-8-encoded
# JSON payload, NOT the character count. For ASCII-only JSON these
# are identical, but multi-byte Unicode characters (e.g. "€" is 3
# bytes in UTF-8) make them differ.
#
# Production note: call STDOUT.binmode before creating a writer so
# that Ruby does not apply line-ending translation. On Windows, the
# default "text" mode converts \n to \r\n — which would corrupt the
# \r\n framing delimiter.
#
# Example:
#   writer = MessageWriter.new(STDOUT)
#   writer.write_message(Response.new(id: 1, result: { ok: true }))
#
# ================================================================

require "json"
require_relative "message"

module CodingAdventures
  module JsonRpc
    class MessageWriter
      # @param stream [IO] any writable IO object (STDOUT, StringIO, pipe, etc.)
      def initialize(stream)
        @stream = stream
      end

      # ----------------------------------------------------------------
      # write_message — serialize and frame a typed message
      # ----------------------------------------------------------------
      #
      # Converts +msg+ to its wire-format Hash, serializes it with
      # JSON.generate (compact, no extra whitespace), then writes the
      # Content-Length header followed by the payload.
      #
      # Example:
      #   writer.write_message(Response.new(id: 1, result: { ok: true }))
      #   # Writes: "Content-Length: 38\r\n\r\n{\"jsonrpc\":\"2.0\",...}"
      #
      def write_message(msg)
        hash = JsonRpc.message_to_h(msg)
        json = JSON.generate(hash)
        write_raw(json)
      end

      # ----------------------------------------------------------------
      # write_raw — frame and write a pre-serialized JSON string
      # ----------------------------------------------------------------
      #
      # Use when you already have the JSON string and do not need message
      # parsing — for example, in tests or proxy scenarios.
      #
      # Example:
      #   writer.write_raw('{"jsonrpc":"2.0","id":1,"result":null}')
      #
      def write_raw(json)
        # Force binary encoding for byte-accurate length calculation.
        # A multi-byte Unicode character (e.g. "€" = 3 UTF-8 bytes) means
        # bytesize and length differ — Content-Length must use bytesize.
        payload = json.encode("UTF-8").b
        header  = "Content-Length: #{payload.bytesize}\r\n\r\n".b

        # Write header + payload as a single call to prevent interleaving
        # if multiple writers ever coexist on the same stream.
        # Both are binary (ASCII-8BIT) so concatenation is safe.
        @stream.write(header + payload)
      end
    end
  end
end
