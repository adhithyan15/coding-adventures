# frozen_string_literal: true

# ================================================================
# CodingAdventures::Rpc::RpcCodec — interface contract (duck typing)
# ================================================================
#
# A *codec* is responsible for translating between the four RPC
# message types and raw bytes. It sits between the RPC layer and
# the framer:
#
#   RpcServer/RpcClient
#         ↕  (RpcMessage objects)
#      RpcCodec          ← YOU ARE HERE
#         ↕  (raw bytes — no framing envelope)
#      RpcFramer
#         ↕  (framed bytes — with length prefix or delimiter)
#      Transport (IO stream)
#
# The codec is *stateless*. A single codec instance can be shared
# across many server/client instances without synchronisation issues,
# because it holds no mutable state — it is a pure function from
# message → bytes and from bytes → message.
#
# ----------------------------------------------------------------
# Interface contract (duck typing — no formal base class)
# ----------------------------------------------------------------
#
# Any object used as a codec MUST respond to these two methods:
#
#   #encode(msg) → String (binary-safe bytes)
#     Converts an RpcRequest, RpcResponse, RpcErrorResponse, or
#     RpcNotification into a byte string suitable for the framer.
#     The string encoding should be Encoding::BINARY (ASCII-8BIT).
#     Must not raise; all message types must be supported.
#
#   #decode(bytes) → RpcMessage  OR  raise RpcError
#     Converts a byte string (the payload of a single frame, no
#     framing envelope) into one of the four RPC message types.
#     Raises RpcError with code PARSE_ERROR if the bytes are not
#     decodable by this codec at all.
#     Raises RpcError with code INVALID_REQUEST if the bytes decoded
#     successfully but do not match any RPC message shape.
#
# ----------------------------------------------------------------
# Concrete implementations (live in their own gems)
# ----------------------------------------------------------------
#
#   JsonCodec   — encode/decode with JSON (json stdlib)
#   MsgpackCodec — encode/decode with MessagePack (msgpack gem)
#
# ----------------------------------------------------------------
# Anatomy of a codec implementation
# ----------------------------------------------------------------
#
# Here is a skeleton that shows the expected structure.  It is not
# used at runtime — it exists purely as documentation and as a
# reference implementation to copy from:
#
#   class MyCodec
#     # Encode an RPC message to bytes.
#     #
#     # @param msg [RpcRequest, RpcResponse, RpcErrorResponse, RpcNotification]
#     # @return [String] binary-safe byte string (Encoding::BINARY)
#     def encode(msg)
#       # 1. Convert +msg+ to a hash/struct/object in the codec's native
#       #    representation (e.g., a Hash for JSON).
#       # 2. Serialise that representation to bytes.
#       # 3. Return the bytes with encoding set to Encoding::BINARY.
#       raise NotImplementedError, "#{self.class}#encode not implemented"
#     end
#
#     # Decode bytes into an RPC message.
#     #
#     # @param bytes [String] binary-safe byte string from the framer
#     # @return [RpcRequest, RpcResponse, RpcErrorResponse, RpcNotification]
#     # @raise [RpcError] with PARSE_ERROR if bytes are not decodable
#     # @raise [RpcError] with INVALID_REQUEST if shape is unrecognised
#     def decode(bytes)
#       # 1. Deserialise bytes → native object.
#       #    On failure → raise RpcError.new(ErrorCodes::PARSE_ERROR, ...)
#       # 2. Inspect the native object to determine message type.
#       #    On unknown shape → raise RpcError.new(ErrorCodes::INVALID_REQUEST, ...)
#       # 3. Build and return the appropriate Struct.
#       raise NotImplementedError, "#{self.class}#decode not implemented"
#     end
#   end
#
# ================================================================

require_relative "errors"
require_relative "message"

module CodingAdventures
  module Rpc
    # RpcCodec is a documentation-only module.  It is never instantiated
    # directly.  Concrete codecs implement the same interface via duck
    # typing (they do NOT include this module).
    #
    # @abstract
    module RpcCodec
      # Encode an RPC message to a byte string.
      #
      # @param msg [RpcRequest, RpcResponse, RpcErrorResponse, RpcNotification]
      # @return [String] byte string with Encoding::BINARY
      # @raise [NotImplementedError] if the subclass forgot to override
      def encode(msg)
        raise NotImplementedError, "#{self.class}#encode is not implemented"
      end

      # Decode a byte string produced by the framer into an RPC message.
      #
      # @param bytes [String] raw payload bytes from the framer
      # @return [RpcRequest, RpcResponse, RpcErrorResponse, RpcNotification]
      # @raise [RpcError] PARSE_ERROR if bytes cannot be decoded
      # @raise [RpcError] INVALID_REQUEST if shape is unrecognised
      def decode(bytes)
        raise NotImplementedError, "#{self.class}#decode is not implemented"
      end
    end
  end
end
