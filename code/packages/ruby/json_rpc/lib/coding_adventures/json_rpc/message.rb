# frozen_string_literal: true

# ================================================================
# CodingAdventures::JsonRpc — Message Types
# ================================================================
#
# JSON-RPC 2.0 defines four message shapes. All carry "jsonrpc": "2.0".
# The shape is determined by which fields are present:
#
#   ┌──────────────┬──────┬──────────┬────────┬───────┐
#   │ Shape        │  id  │  method  │ result │ error │
#   ├──────────────┼──────┼──────────┼────────┼───────┤
#   │ Request      │  yes │   yes    │   —    │   —   │
#   │ Notification │   —  │   yes    │   —    │   —   │
#   │ Response OK  │  yes │    —     │  yes   │   —   │
#   │ Response Err │  yes │    —     │   —    │  yes  │
#   └──────────────┴──────┴──────────┴────────┴───────┘
#
# We use Ruby's Data.define (introduced in Ruby 3.2) for all four types.
# Data objects are immutable value objects — perfect for messages that
# flow through the system without mutation.
#
# Public API:
#   CodingAdventures::JsonRpc::Request.new(id:, method:, params: nil)
#   CodingAdventures::JsonRpc::Notification.new(method:, params: nil)
#   CodingAdventures::JsonRpc::Response.new(id:, result: nil, error: nil)
#   CodingAdventures::JsonRpc::ResponseError.new(code:, message:, data: nil)
#
#   parse_message(hash)  → Message  (raises Error on invalid input)
#   message_to_h(msg)    → Hash     (wire-format hash for JSON.generate)
#
# ================================================================

require_relative "errors"

module CodingAdventures
  module JsonRpc
    # ----------------------------------------------------------------
    # ResponseError — the structured error inside an error Response
    # ----------------------------------------------------------------
    #
    # Example wire object:
    #   { "code" => -32601, "message" => "Method not found", "data" => "..." }
    #
    # +code+    — integer; see ErrorCodes for standard values
    # +message+ — short human-readable description
    # +data+    — optional additional context (any JSON value)
    #
    ResponseError = Data.define(:code, :message, :data) do
      # Allow constructing without +data+.
      def initialize(code:, message:, data: nil)
        super(code: code, message: message, data: data)
      end
    end

    # ----------------------------------------------------------------
    # Request — a call that expects a Response
    # ----------------------------------------------------------------
    #
    # Example wire object:
    #   { "jsonrpc"=>"2.0", "id"=>1, "method"=>"textDocument/hover",
    #     "params"=>{ "position"=>{"line"=>0,"character"=>3} } }
    #
    # +id+     — String or Integer; ties the Response back to this call
    # +method+ — String; the procedure name
    # +params+ — optional Hash or Array
    #
    Request = Data.define(:id, :method, :params) do
      def initialize(id:, method:, params: nil)
        super(id: id, method: method, params: params)
      end
    end

    # ----------------------------------------------------------------
    # Notification — a one-way message with no Response
    # ----------------------------------------------------------------
    #
    # Example wire object:
    #   { "jsonrpc"=>"2.0", "method"=>"textDocument/didOpen",
    #     "params"=>{ "textDocument"=>{"uri"=>"file:///main.bf"} } }
    #
    # The server MUST NOT send a response, even on error.
    #
    Notification = Data.define(:method, :params) do
      def initialize(method:, params: nil)
        super(method: method, params: params)
      end
    end

    # ----------------------------------------------------------------
    # Response — the server's reply to a Request
    # ----------------------------------------------------------------
    #
    # Exactly one of +result+ or +error+ is present.
    #
    # +id+     — matches the originating Request; nil only when the
    #            server cannot determine the request id
    # +result+ — any value on success
    # +error+  — ResponseError on failure
    #
    Response = Data.define(:id, :result, :error) do
      def initialize(id:, result: nil, error: nil)
        super(id: id, result: result, error: error)
      end
    end

    # ================================================================
    # parse_message — raw Hash → typed message
    # ================================================================
    #
    # Converts a Hash (typically from JSON.parse) into one of the four
    # typed message structs. Raises Error(-32600) for unrecognised shapes.
    #
    # Recognition rules (mirrors the table at the top of the file):
    #   has "id" AND "method"               → Request
    #   has "method" but no "id"            → Notification
    #   has "id" AND ("result" OR "error")  → Response
    #   anything else                        → raise Invalid Request
    #
    # Example:
    #   hash  = JSON.parse('{"jsonrpc":"2.0","id":1,"method":"ping"}')
    #   msg   = CodingAdventures::JsonRpc.parse_message(hash)
    #   msg.is_a?(CodingAdventures::JsonRpc::Request)  # => true
    #   msg.method                                      # => "ping"
    #
    def self.parse_message(data)
      unless data.is_a?(Hash)
        raise Error.new(
          ErrorCodes::INVALID_REQUEST,
          "Invalid Request: message must be a JSON object, got #{data.class}"
        )
      end

      has_id     = data.key?("id")
      has_method = data.key?("method") && data["method"].is_a?(String)
      has_result = data.key?("result")
      has_error  = data.key?("error")

      if has_id && has_method
        # ---- Request ----
        id = data["id"]
        unless id.is_a?(String) || id.is_a?(Integer)
          raise Error.new(
            ErrorCodes::INVALID_REQUEST,
            "Invalid Request: id must be String or Integer, got #{id.class}"
          )
        end
        return Request.new(id: id, method: data["method"], params: data["params"])
      end

      if has_method && !has_id
        # ---- Notification ----
        return Notification.new(method: data["method"], params: data["params"])
      end

      if has_id && (has_result || has_error)
        # ---- Response ----
        id = data["id"]
        unless id.is_a?(String) || id.is_a?(Integer) || id.nil?
          raise Error.new(
            ErrorCodes::INVALID_REQUEST,
            "Invalid Request: response id must be String, Integer, or nil"
          )
        end

        error_obj = nil
        if has_error
          raw_err = data["error"]
          unless raw_err.is_a?(Hash)
            raise Error.new(
              ErrorCodes::INVALID_REQUEST,
              "Invalid Request: error must be a JSON object"
            )
          end
          unless raw_err["code"].is_a?(Integer) && raw_err["message"].is_a?(String)
            raise Error.new(
              ErrorCodes::INVALID_REQUEST,
              "Invalid Request: error must have integer code and string message"
            )
          end
          error_obj = ResponseError.new(
            code: raw_err["code"],
            message: raw_err["message"],
            data: raw_err["data"]
          )
        end

        return Response.new(id: id, result: data["result"], error: error_obj)
      end

      raise Error.new(
        ErrorCodes::INVALID_REQUEST,
        "Invalid Request: unrecognised message shape"
      )
    end

    # ================================================================
    # message_to_h — typed message → wire-format Hash
    # ================================================================
    #
    # Converts a typed message back to a plain Hash suitable for
    # JSON.generate. Adds "jsonrpc": "2.0".
    #
    # Example:
    #   req = Request.new(id: 1, method: "ping")
    #   CodingAdventures::JsonRpc.message_to_h(req)
    #   # => {"jsonrpc"=>"2.0", "id"=>1, "method"=>"ping"}
    #
    def self.message_to_h(msg)
      case msg
      when Request
        h = { "jsonrpc" => "2.0", "id" => msg.id, "method" => msg.method }
        h["params"] = msg.params unless msg.params.nil?
        h
      when Notification
        h = { "jsonrpc" => "2.0", "method" => msg.method }
        h["params"] = msg.params unless msg.params.nil?
        h
      when Response
        h = { "jsonrpc" => "2.0", "id" => msg.id }
        if msg.error
          err_h = { "code" => msg.error.code, "message" => msg.error.message }
          err_h["data"] = msg.error.data unless msg.error.data.nil?
          h["error"] = err_h
        else
          h["result"] = msg.result
        end
        h
      else
        raise ArgumentError, "Unknown message type: #{msg.class}"
      end
    end
  end
end
