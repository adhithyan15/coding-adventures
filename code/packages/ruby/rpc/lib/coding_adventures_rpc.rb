# frozen_string_literal: true

# ================================================================
# coding_adventures_rpc — top-level entry point
# ================================================================
#
# Codec-agnostic RPC primitive for building protocol-specific
# packages (json-rpc, msgpack-rpc, protobuf-rpc, …).
#
# This gem defines the abstract vocabulary:
#   • Four message types    — RpcRequest, RpcResponse, RpcErrorResponse,
#                             RpcNotification
#   • Error code constants  — PARSE_ERROR, INVALID_REQUEST, …
#   • Interface contracts   — RpcCodec, RpcFramer (duck-typed, documented)
#   • Server                — read-dispatch-write loop with handler registry
#   • Client                — request/notify with blocking id-correlation
#
# Architecture diagram (three layers):
#
#   Application
#       ↕  RpcMessage objects
#   [coding_adventures_rpc]
#   RpcServer / RpcClient
#       ↕  RpcMessage objects
#   RpcCodec      ← pluggable: JSON, MessagePack, Protobuf, …
#       ↕  raw payload bytes
#   RpcFramer     ← pluggable: Content-Length, newline, length-prefix, …
#       ↕  framed bytes
#   Transport (IO stream)
#
# Quick start:
#
#   require "coding_adventures_rpc"
#   CA = CodingAdventures::Rpc
#
#   server = CA::Server.new(my_codec, my_framer)
#     .on_request("ping") { |_id, _params| "pong" }
#     .on_notification("log") { |params| puts params }
#   server.serve
#
# Files loaded in leaf-to-root order to satisfy Ruby's require_relative
# rules (each file may only reference symbols from files loaded before it):
#
#   version.rb   — VERSION constant; no other dependencies
#   errors.rb    — ErrorCodes, RpcError; depends on nothing
#   message.rb   — RpcRequest, RpcResponse, …; depends on errors
#   codec.rb     — RpcCodec module docs; depends on errors + message
#   framer.rb    — RpcFramer module docs; depends on errors
#   server.rb    — Server class; depends on errors + message
#   client.rb    — Client class; depends on errors + message
#
# ================================================================

require_relative "coding_adventures/rpc/version"
require_relative "coding_adventures/rpc/errors"
require_relative "coding_adventures/rpc/message"
require_relative "coding_adventures/rpc/codec"
require_relative "coding_adventures/rpc/framer"
require_relative "coding_adventures/rpc/server"
require_relative "coding_adventures/rpc/client"
