# frozen_string_literal: true

# ================================================================
# CodingAdventures::JsonRpc::ErrorCodes
# ================================================================
#
# JSON-RPC 2.0 defines a set of reserved integer error codes.
# Each code has a standard meaning:
#
#   -32700  Parse error      — bytes could not be parsed as JSON at all
#   -32600  Invalid Request  — valid JSON but not a well-formed message
#   -32601  Method not found — the method name is not registered
#   -32602  Invalid params   — the handler received unexpected arguments
#   -32603  Internal error   — unexpected exception inside a handler
#   -32099..-32000  Server errors — reserved for implementation use
#
# LSP reserves -32899..-32800 for LSP-specific codes. This package
# intentionally leaves that range alone.
#
# Usage:
#   code = CodingAdventures::JsonRpc::ErrorCodes::METHOD_NOT_FOUND
#   # => -32601
#
# ================================================================

module CodingAdventures
  module JsonRpc
    module ErrorCodes
      # The bytes are not valid JSON at all.
      # Example: "{broken json" arrives on stdin.
      PARSE_ERROR = -32_700

      # Valid JSON, but not a recognisable JSON-RPC message shape.
      # Example: the payload is a JSON array instead of an object.
      INVALID_REQUEST = -32_600

      # The method name in the Request is not registered on the server.
      # The server MUST send this error — it must not silently drop the request.
      # (Silently dropping is only allowed for unknown Notifications.)
      METHOD_NOT_FOUND = -32_601

      # The method was found but the supplied params are wrong.
      # Handlers should return this when required fields are missing or types
      # do not match what the method expects.
      INVALID_PARAMS = -32_602

      # An unexpected error occurred inside the handler.
      # Catch-all for server-side bugs. Ask: "was the request well-formed?"
      # If yes → InternalError. If no → InvalidParams.
      INTERNAL_ERROR = -32_603
    end

    # ================================================================
    # Error — exception raised by the JSON-RPC transport layer
    # ================================================================
    #
    # Raised by MessageReader when framing or parsing fails.
    # Carries a numeric +code+ in addition to the standard message.
    #
    # Example:
    #   begin
    #     reader.read_message
    #   rescue CodingAdventures::JsonRpc::Error => e
    #     puts "code=#{e.code} message=#{e.message}"
    #   end
    #
    class Error < StandardError
      # @return [Integer] one of the ErrorCodes constants
      attr_reader :code

      def initialize(code, message)
        super(message)
        @code = code
      end
    end
  end
end
