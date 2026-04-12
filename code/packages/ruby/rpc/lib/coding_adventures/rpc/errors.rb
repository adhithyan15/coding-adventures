# frozen_string_literal: true

# ================================================================
# CodingAdventures::Rpc::ErrorCodes
# ================================================================
#
# RPC defines a set of reserved integer error codes that are
# codec-agnostic — the same table applies whether the wire format
# is JSON, MessagePack, Protobuf, or anything else.
#
# Think of these codes like HTTP status codes: the number tells you
# *what went wrong* at the protocol level, independently of whether
# the body is HTML or JSON.
#
# Code table:
#
#   -32700  Parse error      — the framed bytes could not be decoded
#                              by the codec at all (garbage input)
#   -32600  Invalid Request  — decoded successfully but not a valid
#                              RPC message (wrong shape)
#   -32601  Method not found — no handler registered for the method
#   -32602  Invalid params   — handler rejected the params
#   -32603  Internal error   — unexpected exception inside a handler
#   -32099..-32000  Server errors — reserved for implementation use
#
# The LSP spec reserves -32899..-32800 for LSP-specific codes.
# This package must not use that range — it belongs to the
# application layer above.
#
# Usage:
#   code = CodingAdventures::Rpc::ErrorCodes::METHOD_NOT_FOUND
#   # => -32601
#
# ================================================================

module CodingAdventures
  module Rpc
    # Standard error code constants shared by all RPC codec instantiations.
    module ErrorCodes
      # The framed bytes could not be decoded by the codec at all.
      #
      # Example: a JSON codec receives "{broken json" — the JSON parser
      # throws, so we reply with PARSE_ERROR (-32700).
      PARSE_ERROR = -32_700

      # The bytes were decoded successfully, but the result is not a
      # recognisable RPC message.
      #
      # Example: a JSON codec decodes [1, 2, 3] — a valid JSON array —
      # but the RPC layer expects an object with known discriminating
      # keys (id, method, result, or error). Shape mismatch → INVALID_REQUEST.
      INVALID_REQUEST = -32_600

      # The requested method name is not registered on the server.
      #
      # Per the spec, the server MUST return this error — silently
      # dropping an unknown *request* is forbidden. (Unknown
      # *notifications* are silently dropped, because no response is
      # expected for them.)
      METHOD_NOT_FOUND = -32_601

      # The method was found but the supplied params are wrong.
      #
      # Handlers should return this when required fields are missing,
      # types do not match what the method expects, or a constraint
      # (e.g., non-negative integer) is violated.
      INVALID_PARAMS = -32_602

      # An unexpected error occurred inside the handler.
      #
      # This is the catch-all for server-side bugs or panics.
      # Ask yourself: "was the request well-formed?"
      #   If yes → INTERNAL_ERROR (the server is broken).
      #   If no  → INVALID_PARAMS (the client sent bad input).
      INTERNAL_ERROR = -32_603
    end

    # ================================================================
    # RpcError — exception raised by the RPC transport layer
    # ================================================================
    #
    # Raised by framers and codecs when something goes wrong at the
    # protocol level (bad frame, undecodable bytes, invalid message
    # shape). Carries a numeric +code+ (one of ErrorCodes) in addition
    # to the human-readable +message+.
    #
    # The server's +serve+ loop rescues this to build an error response
    # rather than crashing.
    #
    # Example:
    #   begin
    #     bytes = framer.read_frame
    #     msg   = codec.decode(bytes)
    #   rescue CodingAdventures::Rpc::RpcError => e
    #     puts "code=#{e.code} message=#{e.message}"
    #   end
    #
    class RpcError < StandardError
      # @return [Integer] one of the ErrorCodes constants
      attr_reader :code

      # @param code    [Integer] one of ErrorCodes::*
      # @param message [String]  human-readable description
      def initialize(code, message)
        super(message)
        @code = code
      end
    end
  end
end
