# frozen_string_literal: true

# ================================================================
# CodingAdventures::JsonRpc::Server
# ================================================================
#
# The Server combines a MessageReader and MessageWriter with a
# method dispatch table. It drives the read-dispatch-write loop,
# isolating the application from all framing, parsing, and
# error-handling details.
#
# Lifecycle:
#   1. Create a Server with input and output IO streams.
#   2. Register handlers with #on_request and #on_notification.
#   3. Call #serve — it blocks until the stream closes.
#
# Dispatch rules:
#
#   Request received:
#     handler found    → call it; send result or ResponseError as Response
#     no handler       → send -32601 (Method not found) Response
#     handler raises   → send -32603 (Internal error) Response
#
#   Notification received:
#     handler found    → call it; send NOTHING
#     no handler       → silently ignore (spec forbids error responses for notifications)
#
#   Response received:
#     discarded in server-only mode (future client support would look up pending reqs)
#
# Concurrency:
#   #serve is single-threaded — it processes one message at a time.
#   This is correct for LSP, where editors send one request and
#   wait before sending the next.
#
# Example:
#   Server.new(STDIN, STDOUT)
#     .on_request("initialize") { |_id, _params| { capabilities: {} } }
#     .on_notification("textDocument/didOpen") { |params| ... }
#     .serve
#
# ================================================================

require_relative "errors"
require_relative "message"
require_relative "reader"
require_relative "writer"

module CodingAdventures
  module JsonRpc
    class Server
      def initialize(in_stream, out_stream)
        @reader = MessageReader.new(in_stream)
        @writer = MessageWriter.new(out_stream)
        @request_handlers      = {}
        @notification_handlers = {}
      end

      # ----------------------------------------------------------------
      # on_request — register a handler for a Request method
      # ----------------------------------------------------------------
      #
      # The block receives (id, params) and must return either:
      #   - A plain Ruby value (serialised as the +result+ field)
      #   - A ResponseError instance (serialised as the +error+ field)
      #
      # Returns +self+ for chaining.
      #
      # Example:
      #   server.on_request("ping") { |_id, _params| "pong" }
      #
      def on_request(method, &handler)
        @request_handlers[method] = handler
        self
      end

      # ----------------------------------------------------------------
      # on_notification — register a handler for a Notification method
      # ----------------------------------------------------------------
      #
      # The block receives (params) and returns nothing. Even if it
      # raises, no response is sent.
      #
      # Returns +self+ for chaining.
      #
      # Example:
      #   server.on_notification("textDocument/didOpen") { |params| ... }
      #
      def on_notification(method, &handler)
        @notification_handlers[method] = handler
        self
      end

      # ----------------------------------------------------------------
      # serve — run the read-dispatch-write loop
      # ----------------------------------------------------------------
      #
      # Blocks until the input stream closes (returns nil from
      # read_message). Processes one message per iteration.
      #
      def serve
        loop do
          msg = begin
            @reader.read_message
          rescue Error => e
            # Framing or parse error — send error response (id unknown → nil)
            send_error(nil, e.code, e.message)
            next
          rescue StandardError
            send_error(nil, ErrorCodes::INTERNAL_ERROR, "Internal error")
            next
          end

          break if msg.nil? # clean EOF

          dispatch(msg)
        end
      end

      private

      def dispatch(msg)
        case msg
        when Request
          handle_request(msg)
        when Notification
          handle_notification(msg)
        when Response
          # Server-only mode: discard incoming Responses.
          nil
        end
      end

      def handle_request(req)
        handler = @request_handlers[req.method]

        unless handler
          send_error(req.id, ErrorCodes::METHOD_NOT_FOUND, "Method not found: #{req.method}")
          return
        end

        result = begin
          handler.call(req.id, req.params)
        rescue StandardError => e
          send_error(req.id, ErrorCodes::INTERNAL_ERROR, e.message)
          return
        end

        if result.is_a?(ResponseError)
          @writer.write_message(Response.new(id: req.id, error: result))
        else
          @writer.write_message(Response.new(id: req.id, result: result))
        end
      end

      def handle_notification(notif)
        handler = @notification_handlers[notif.method]
        return unless handler # silently ignore unknown notifications

        begin
          handler.call(notif.params)
        rescue StandardError
          # Swallow — notifications must never produce responses
        end
      end

      def send_error(id, code, message, data = nil)
        err = ResponseError.new(code: code, message: message, data: data)
        @writer.write_message(Response.new(id: id, error: err))
      end
    end
  end
end
