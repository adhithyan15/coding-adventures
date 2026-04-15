# frozen_string_literal: true

# ================================================================
# CodingAdventures::Rpc::RpcFramer — interface contract (duck typing)
# ================================================================
#
# A *framer* knows how to split a raw byte stream into discrete
# chunks (frames), and how to wrap outgoing byte chunks in the
# framing envelope before writing them to the stream.
#
# The framer sits between the codec and the transport:
#
#   RpcServer/RpcClient
#         ↕  (RpcMessage objects)
#      RpcCodec
#         ↕  (raw payload bytes — no framing envelope)
#      RpcFramer         ← YOU ARE HERE
#         ↕  (framed bytes — with length header or delimiter)
#      Transport (IO stream, e.g., STDIN/STDOUT, TCPSocket, StringIO)
#
# The framer knows *nothing* about the content of the payload bytes —
# it only concerns itself with boundaries. It is like a postal service
# that knows how to put letters in envelopes and tear them open, but
# has no idea what the letters say.
#
# ----------------------------------------------------------------
# Interface contract (duck typing — no formal base class required)
# ----------------------------------------------------------------
#
# Any object used as a framer MUST respond to these two methods:
#
#   #read_frame → String | nil
#     Read the next payload (frame body) from the underlying stream.
#     Returns a binary-safe String (Encoding::BINARY) on success.
#     Returns nil on clean EOF (the stream closed normally).
#     Raises RpcError with a suitable code on framing errors
#     (e.g., malformed Content-Length header, truncated frame).
#
#   #write_frame(bytes)
#     Write +bytes+ to the stream, wrapped in the framing envelope
#     (e.g., prefixed with "Content-Length: N\r\n\r\n").
#     Returns nil (callers ignore the return value).
#     May raise on I/O errors.
#
# ----------------------------------------------------------------
# Concrete implementations
# ----------------------------------------------------------------
#
#   ContentLengthFramer  — "Content-Length: N\r\n\r\n<payload>"
#                          Used by LSP (Language Server Protocol).
#   LengthPrefixFramer   — 4-byte big-endian length + payload
#                          Compact TCP variant.
#   NewlineFramer        — payload + "\n" delimiter
#                          Used by NDJSON streaming.
#   PassthroughFramer    — no framing; each write_frame is one complete
#                          stream (useful when HTTP handles framing).
#
# ----------------------------------------------------------------
# Anatomy of a framer implementation
# ----------------------------------------------------------------
#
#   class MyFramer
#     def initialize(in_stream, out_stream)
#       @in  = in_stream
#       @out = out_stream
#     end
#
#     # Read the next payload from the stream.
#     #
#     # @return [String, nil] binary payload bytes, or nil on clean EOF
#     # @raise [RpcError] on framing errors
#     def read_frame
#       # 1. Read enough bytes to parse the frame header/delimiter.
#       # 2. Determine the payload length from the header.
#       # 3. Read exactly that many bytes.
#       # 4. Return the payload bytes (Encoding::BINARY).
#       # Return nil if the stream is at EOF with no pending data.
#       raise NotImplementedError, "#{self.class}#read_frame not implemented"
#     end
#
#     # Write a payload to the stream with framing envelope.
#     #
#     # @param bytes [String] binary payload bytes from the codec
#     # @return [nil]
#     def write_frame(bytes)
#       # 1. Build the framing envelope (header or delimiter).
#       # 2. Write envelope + payload to @out.
#       # 3. Flush if necessary.
#       raise NotImplementedError, "#{self.class}#write_frame not implemented"
#     end
#   end
#
# ================================================================

require_relative "errors"

module CodingAdventures
  module Rpc
    # RpcFramer is a documentation-only module.  Concrete framers
    # implement the same interface via duck typing and do NOT include
    # this module.
    #
    # @abstract
    module RpcFramer
      # Read the next payload (frame body) from the underlying stream.
      #
      # @return [String] binary payload bytes on success
      # @return [nil]    on clean EOF
      # @raise [RpcError] on framing errors (malformed header, truncation)
      def read_frame
        raise NotImplementedError, "#{self.class}#read_frame is not implemented"
      end

      # Write a payload to the underlying stream with framing envelope.
      #
      # @param bytes [String] binary payload from the codec
      # @return [nil]
      def write_frame(bytes)
        raise NotImplementedError, "#{self.class}#write_frame is not implemented"
      end
    end
  end
end
