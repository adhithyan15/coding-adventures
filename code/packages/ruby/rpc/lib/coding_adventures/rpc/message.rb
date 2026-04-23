# frozen_string_literal: true

# ================================================================
# CodingAdventures::Rpc — Message Types
# ================================================================
#
# Four message types model every possible RPC exchange between a
# client and a server. Together they form a discriminated union —
# exactly one type applies to any given message on the wire.
#
# Visual overview:
#
#   Client                          Server
#   ──────                          ──────
#   RpcRequest(id, method, params)  ───→   Server dispatches to handler
#                                   ←───   RpcResponse(id, result)
#                                    or    RpcErrorResponse(id, code, msg)
#
#   RpcNotification(method, params) ───→   Server dispatches; no reply
#
# Key invariants:
#
#   • RpcRequest   always has an id   (string or integer)
#   • RpcResponse  always has an id   (matching the request)
#   • RpcErrorResponse has an id      (nil only if the request was so
#                                       malformed that id extraction failed)
#   • RpcNotification  never has an id
#
# The params and result fields hold the codec's native dynamic value.
# For a JSON codec that is a Hash/Array/String/Integer/nil; for a
# MessagePack codec it would be a MessagePack::Ext; and so on.
# The RPC layer never inspects those values — it passes them through
# unchanged, like a letter carrier that doesn't read the letters.
#
# ================================================================

require_relative "errors"

module CodingAdventures
  module Rpc
    # ------------------------------------------------------------------
    # RpcRequest
    # ------------------------------------------------------------------
    #
    # A call from client to server that expects a response. The +id+
    # field correlates this request with its eventual response.
    #
    # Fields:
    #   id     — String or Integer; must be unique within a session
    #   method — the procedure name (e.g. "initialize", "ping")
    #   params — codec-native value describing the call arguments, or nil
    #
    # Example (JSON codec, params as Hash):
    #   RpcRequest.new(id: 1, method: "textDocument/hover",
    #                  params: { "line" => 10, "character" => 5 })
    #
    RpcRequest = Struct.new(:id, :method, :params, keyword_init: true)

    # ------------------------------------------------------------------
    # RpcResponse
    # ------------------------------------------------------------------
    #
    # A successful response from server to client. The +id+ matches the
    # corresponding RpcRequest.
    #
    # Fields:
    #   id     — matches the RpcRequest id
    #   result — codec-native value; the return value of the procedure
    #
    # Example:
    #   RpcResponse.new(id: 1, result: { "hover" => "String#upcase" })
    #
    RpcResponse = Struct.new(:id, :result, keyword_init: true)

    # ------------------------------------------------------------------
    # RpcErrorResponse
    # ------------------------------------------------------------------
    #
    # An error response from server to client. The +id+ matches the
    # corresponding RpcRequest (nil only when the request id could not
    # be extracted, e.g., PARSE_ERROR).
    #
    # Fields:
    #   id      — matches the RpcRequest id, or nil if id was unreadable
    #   code    — Integer error code (see ErrorCodes)
    #   message — human-readable description of the error
    #   data    — optional codec-native value with extra diagnostic info
    #
    # Example:
    #   RpcErrorResponse.new(
    #     id: 1, code: ErrorCodes::METHOD_NOT_FOUND,
    #     message: "Method not found: textDocument/rainbow",
    #     data: nil
    #   )
    #
    RpcErrorResponse = Struct.new(:id, :code, :message, :data, keyword_init: true)

    # ------------------------------------------------------------------
    # RpcNotification
    # ------------------------------------------------------------------
    #
    # A fire-and-forget message from client to server (or server to
    # client for server-push). No response is ever generated.
    #
    # Fields:
    #   method — the notification name (e.g. "textDocument/didOpen")
    #   params — codec-native value describing the event payload, or nil
    #
    # Example:
    #   RpcNotification.new(
    #     method: "textDocument/didOpen",
    #     params: { "uri" => "file:///main.rb" }
    #   )
    #
    RpcNotification = Struct.new(:method, :params, keyword_init: true)
  end
end
