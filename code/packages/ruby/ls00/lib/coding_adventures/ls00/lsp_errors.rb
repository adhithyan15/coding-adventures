# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00::LspErrors — LSP-specific error codes
# ================================================================
#
# The JSON-RPC 2.0 specification reserves error codes in the range
# [-32768, -32000]. The LSP specification further reserves
# [-32899, -32800] for LSP protocol-level errors.
#
# Standard JSON-RPC error codes (from the json_rpc package):
#   -32700  ParseError
#   -32600  InvalidRequest
#   -32601  MethodNotFound
#   -32602  InvalidParams
#   -32603  InternalError
#
# LSP-specific error codes are listed below.
#
# Reference:
# https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes
#
# ================================================================

module CodingAdventures
  module Ls00
    module LspErrors
      # The server has received a request before the initialize handshake
      # was completed.
      SERVER_NOT_INITIALIZED = -32_002

      # A generic error code for unknown errors.
      UNKNOWN_ERROR_CODE = -32_001

      # A request failed but not due to a protocol problem. For example,
      # the document requested was not found.
      REQUEST_FAILED = -32_803

      # The server cancelled the request.
      SERVER_CANCELLED = -32_802

      # The document content was modified before the request completed.
      # The client should retry.
      CONTENT_MODIFIED = -32_801

      # The client cancelled the request.
      REQUEST_CANCELLED = -32_800
    end
  end
end
