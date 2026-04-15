# frozen_string_literal: true

# ================================================================
# CodingAdventures::Rpc::Client
# ================================================================
#
# The Client sends requests to a remote server and waits for their
# responses. It also sends fire-and-forget notifications.
#
# Architecture
# ────────────
#
#   Application code
#       ↓ request(method, params)
#   Client.request              ← auto-generates id; writes frame; loops
#       ↓ payload bytes
#   RpcCodec.encode             ← RpcRequest → bytes
#       ↓ payload bytes
#   RpcFramer.write_frame       ← adds framing envelope
#       ↓ raw bytes → network →
#   RpcFramer.read_frame        ← reads response frame
#       ↓ payload bytes
#   RpcCodec.decode             ← bytes → RpcMessage
#       ↓ RpcResponse or RpcErrorResponse
#   Client.request returns      ← returns result or raises RpcError
#
# The client holds no connection state beyond a monotonically
# increasing request id counter. It is designed for *synchronous*
# use: send one request, wait for the matching response, return.
#
# Id management
# ─────────────
#
# The client maintains a counter starting at 1. Each call to
# #request increments the counter and uses the new value as the
# request id.  The server echoes the id in its response; the client
# loops until it finds a response with the matching id.
#
#   next_id:  Integer (starts at 1, incremented per request)
#
# Server-push notifications
# ─────────────────────────
#
# While waiting for a response, the server may send unsolicited
# notifications (e.g., LSP's textDocument/publishDiagnostics).
# The client handles these via #on_notification: any notification
# that arrives during the blocking read loop is dispatched to its
# registered handler, and the loop continues waiting.
#
# Concurrency note
# ────────────────
#
# This client is *not* thread-safe. All three operations (#request,
# #notify, reading the response) use the same framer without locking.
# For concurrent use, wrap in a Mutex or use a dedicated reader thread
# with a pending-request map.
#
# Example usage
# ─────────────
#
#   codec  = MyJsonCodec.new
#   framer = MyContentLengthFramer.new(socket, socket)
#
#   client = CodingAdventures::Rpc::Client.new(codec, framer)
#     .on_notification("log/message") { |params| puts params["message"] }
#
#   result = client.request("add", { "a" => 1, "b" => 2 })
#   # => {"sum" => 3}
#
#   client.notify("window/logMessage", { "type" => 3, "message" => "hi" })
#
# ================================================================

require_relative "errors"
require_relative "message"

module CodingAdventures
  module Rpc
    class Client
      # Create a new Client.
      #
      # @param codec  [#encode, #decode] codec for message serialisation
      # @param framer [#read_frame, #write_frame] framer for stream splitting
      def initialize(codec, framer)
        @codec   = codec
        @framer  = framer
        @next_id = 1
        @notification_handlers = {}
      end

      # ----------------------------------------------------------------
      # on_notification — register a handler for server-push notifications
      # ----------------------------------------------------------------
      #
      # Server-push notifications arrive while the client is blocked in
      # #request.  Register handlers for methods the server sends
      # unprompted.
      #
      # Returns +self+ so calls can be chained fluently.
      #
      # @param method  [String] notification method name to handle
      # @yieldparam params codec-native params value (or nil)
      # @return [self]
      #
      # Example:
      #   client.on_notification("textDocument/publishDiagnostics") do |params|
      #     record_diagnostics(params["diagnostics"])
      #   end
      #
      def on_notification(method, &handler)
        @notification_handlers[method] = handler
        self
      end

      # ----------------------------------------------------------------
      # request — send a request and wait for the matching response
      # ----------------------------------------------------------------
      #
      # Encodes and writes an RpcRequest with an auto-generated integer
      # id, then loops reading frames until a response with the matching
      # id arrives.  While waiting, any server-push notifications are
      # dispatched to registered handlers.
      #
      # Return / raise semantics:
      #   - Returns the +result+ field of an RpcResponse on success.
      #   - Raises RpcError if the server replies with RpcErrorResponse.
      #   - Raises RpcError (INTERNAL_ERROR) if the connection closes
      #     before the matching response arrives.
      #
      # @param method [String]      RPC method name
      # @param params [Object, nil] codec-native params (Hash, Array, nil…)
      # @return [Object]            the decoded result value
      # @raise [RpcError]           on server error or connection closed
      #
      # Example:
      #   result = client.request("textDocument/hover",
      #                           { "position" => { "line" => 10 } })
      #
      def request(method, params = nil)
        id  = @next_id
        @next_id += 1

        req = RpcRequest.new(id: id, method: method, params: params)
        @framer.write_frame(@codec.encode(req))

        # Blocking loop: read frames until we find the response for +id+.
        loop do
          bytes = @framer.read_frame
          if bytes.nil?
            raise RpcError.new(ErrorCodes::INTERNAL_ERROR,
                               "Connection closed before response to request #{id}")
          end

          msg = @codec.decode(bytes)

          case msg
          when RpcResponse
            if msg.id == id
              return msg.result
            end
            # Response for a different id — ignore and keep waiting.

          when RpcErrorResponse
            if msg.id == id
              raise RpcError.new(msg.code, msg.message)
            end
            # Error for a different id — ignore and keep waiting.

          when RpcNotification
            # Server-push notification received while waiting for our response.
            # Dispatch it to the registered handler (if any) and continue.
            handler = @notification_handlers[msg.method]
            if handler
              begin
                handler.call(msg.params)
              rescue Exception # rubocop:disable Lint/RescueException
                # Swallow handler errors — they must not abort the wait loop.
                nil
              end
            end

          when RpcRequest
            # Unexpected: server sent us a request while we are waiting.
            # Discard it — this client is request-only; it does not handle
            # incoming requests.
            nil
          end
        end
      end

      # ----------------------------------------------------------------
      # notify — send a notification (fire and forget)
      # ----------------------------------------------------------------
      #
      # Encodes and writes an RpcNotification.  No response is expected
      # or waited for.  Returns immediately after the frame is written.
      #
      # @param method [String]      notification method name
      # @param params [Object, nil] codec-native params (or nil)
      # @return [nil]
      #
      # Example:
      #   client.notify("window/logMessage", { "type" => 3, "message" => "hello" })
      #
      def notify(method, params = nil)
        notif = RpcNotification.new(method: method, params: params)
        @framer.write_frame(@codec.encode(notif))
        nil
      end
    end
  end
end
