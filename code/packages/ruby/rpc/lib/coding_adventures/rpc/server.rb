# frozen_string_literal: true

# ================================================================
# CodingAdventures::Rpc::Server
# ================================================================
#
# The Server owns a codec, a framer, and two dispatch tables — one
# for request handlers and one for notification handlers.  Its
# #serve method drives the read-dispatch-write loop until the
# stream closes.
#
# Architecture
# ────────────
#
#   Transport (IO)
#       ↓ raw bytes
#   RpcFramer.read_frame      ← splits stream into payload chunks
#       ↓ payload bytes
#   RpcCodec.decode           ← bytes → RpcMessage
#       ↓ RpcMessage
#   Server dispatch table     ← method name → handler block
#       ↓ result / error
#   RpcCodec.encode           ← RpcMessage → bytes
#       ↓ payload bytes
#   RpcFramer.write_frame     ← wraps payload in framing envelope
#       ↓ raw bytes
#   Transport (IO)
#
# The Server never touches bytes directly — it only sees RpcMessage
# objects.  Swapping the codec or framer is a one-line change.
#
# Dispatch rules
# ──────────────
#
#   Request received:
#     handler found    → call it; send RpcResponse (result or error)
#     no handler       → send RpcErrorResponse -32601 Method not found
#     handler raises   → send RpcErrorResponse -32603 Internal error
#
#   Notification received:
#     handler found    → call it; send NOTHING (fire-and-forget)
#     no handler       → silently ignore (forbidden to send an error
#                        response for unknown notifications per spec)
#
#   Response/ErrorResponse received:
#     discarded in server-only mode (bidirectional peers route these
#     to the pending-request table in the Client)
#
# Panic safety
# ────────────
#
# Ruby uses exceptions for all runtime errors. Handler panics must
# not kill the server process. #serve rescues Exception (not just
# StandardError) to also catch ScriptError, SignalException subclasses
# that are safe to recover from, and other unusual raises.
#
# A recovered panic returns -32603 Internal error with the exception's
# message as the data field so callers can diagnose what went wrong.
#
# Concurrency
# ───────────
#
# #serve is single-threaded — it processes one message at a time.
# This is correct for the LSP use case where editors send one
# request and wait for a response before the next.  For concurrent
# use, spawn a thread per connection or use a thread-per-message
# architecture with a mutex-protected dispatch table.
#
# Example usage
# ─────────────
#
#   codec  = MyJsonCodec.new
#   framer = MyContentLengthFramer.new(STDIN, STDOUT)
#
#   CodingAdventures::Rpc::Server.new(codec, framer)
#     .on_request("initialize") { |_id, _params| { capabilities: {} } }
#     .on_notification("textDocument/didOpen") { |params| log(params) }
#     .serve
#
# ================================================================

require_relative "errors"
require_relative "message"

module CodingAdventures
  module Rpc
    class Server
      # Create a new Server.
      #
      # @param codec  [#encode, #decode] codec for message serialisation
      # @param framer [#read_frame, #write_frame] framer for stream splitting
      def initialize(codec, framer)
        @codec   = codec
        @framer  = framer
        @request_handlers      = {}
        @notification_handlers = {}
      end

      # ----------------------------------------------------------------
      # on_request — register a handler for a named request method
      # ----------------------------------------------------------------
      #
      # The block receives +(id, params)+ and must return either:
      #   - A plain Ruby value → serialised as the +result+ field of
      #     an RpcResponse.
      #   - An RpcErrorResponse → serialised as an error response.
      #
      # Registering the same method twice replaces the earlier handler.
      # Returns +self+ so calls can be chained fluently.
      #
      # @param method  [String]   RPC method name to handle
      # @yieldparam id     the request id (String or Integer)
      # @yieldparam params the decoded params (codec-native value, or nil)
      # @yieldreturn       a result value or RpcErrorResponse
      # @return [self] for fluent chaining
      #
      # Example:
      #   server.on_request("ping") { |_id, _params| "pong" }
      #   server.on_request("add")  { |_id, params| params["a"] + params["b"] }
      #
      def on_request(method, &handler)
        @request_handlers[method] = handler
        self
      end

      # ----------------------------------------------------------------
      # on_notification — register a handler for a named notification
      # ----------------------------------------------------------------
      #
      # The block receives +(params)+. Its return value is ignored.
      # Even if the handler raises, no error response is ever sent
      # (notifications are fire-and-forget by spec).
      #
      # Returns +self+ so calls can be chained fluently.
      #
      # @param method  [String]   notification method name to handle
      # @yieldparam params the decoded params (codec-native value, or nil)
      # @return [self] for fluent chaining
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
      # Blocks until the framer signals clean EOF (returns nil from
      # #read_frame).  Processes one message per loop iteration.
      #
      # Loop behaviour:
      #
      #   1. Read the next frame from the framer.
      #   2. If nil → EOF → break.
      #   3. Ask the codec to decode the frame into an RpcMessage.
      #   4. Dispatch the message via the appropriate private method.
      #   5. If codec raises RpcError → send error response with nil id.
      #   6. Go to step 1.
      #
      # @return [nil]
      #
      def serve
        loop do
          bytes = begin
            @framer.read_frame
          rescue RpcError => e
            # Framing error (e.g., malformed Content-Length header).
            # Send error response with nil id since we have no request to correlate.
            send_error(nil, e.code, e.message)
            next
          rescue StandardError => e
            send_error(nil, ErrorCodes::INTERNAL_ERROR, e.message)
            next
          end

          break if bytes.nil? # clean EOF — stream closed normally

          msg = begin
            @codec.decode(bytes)
          rescue RpcError => e
            # Codec could not decode the payload bytes (parse or shape error).
            # Reply with a null-id error response and continue.
            send_error(nil, e.code, e.message)
            next
          rescue StandardError => e
            send_error(nil, ErrorCodes::INTERNAL_ERROR, e.message)
            next
          end

          dispatch(msg)
        end
      end

      private

      # Route an RpcMessage to the appropriate handler.
      #
      # @param msg [RpcRequest, RpcResponse, RpcErrorResponse, RpcNotification]
      def dispatch(msg)
        case msg
        when RpcRequest
          handle_request(msg)
        when RpcNotification
          handle_notification(msg)
        when RpcResponse, RpcErrorResponse
          # Server-only mode: incoming responses are discarded.
          # A bidirectional peer would route these to a pending-request table.
          nil
        end
      end

      # Handle a request: look up its handler, call it, write the response.
      #
      # @param req [RpcRequest]
      def handle_request(req)
        handler = @request_handlers[req.method]

        unless handler
          # Spec mandates: unknown method → -32601 error response.
          send_error(req.id, ErrorCodes::METHOD_NOT_FOUND, "Method not found: #{req.method}")
          return
        end

        # rescue Exception (not just StandardError) to also catch
        # ScriptError, Interrupt subclasses that are safe here, etc.
        # The spec calls this "panic recovery".
        result = begin
          handler.call(req.id, req.params)
        rescue Exception => e # rubocop:disable Lint/RescueException
          send_error(req.id, ErrorCodes::INTERNAL_ERROR, e.message)
          return
        end

        if result.is_a?(RpcErrorResponse)
          # Handler explicitly returned an error response — forward it.
          @framer.write_frame(@codec.encode(result))
        else
          # Handler returned a plain value — wrap it in a success response.
          @framer.write_frame(@codec.encode(RpcResponse.new(id: req.id, result: result)))
        end
      end

      # Handle a notification: call its handler (if any) and write nothing.
      #
      # Unknown notifications are silently dropped — the spec forbids
      # sending an error response for unrecognised notifications.
      #
      # @param notif [RpcNotification]
      def handle_notification(notif)
        handler = @notification_handlers[notif.method]
        return unless handler # silently drop unknown notifications

        begin
          handler.call(notif.params)
        rescue Exception # rubocop:disable Lint/RescueException
          # Swallow all exceptions — notifications must never produce responses.
          nil
        end
      end

      # Build and write an RpcErrorResponse frame.
      #
      # @param id      [String, Integer, nil] request id (nil for framing errors)
      # @param code    [Integer] one of ErrorCodes::*
      # @param message [String]  human-readable description
      # @param data    [Object, nil] optional extra diagnostic data
      def send_error(id, code, message, data = nil)
        err = RpcErrorResponse.new(id: id, code: code, message: message, data: data)
        @framer.write_frame(@codec.encode(err))
      end
    end
  end
end
