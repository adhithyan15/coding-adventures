# frozen_string_literal: true

# ================================================================
# coding_adventures_json_rpc — top-level entry point
# ================================================================
#
# JSON-RPC 2.0 over stdin/stdout with Content-Length framing.
#
# This gem implements the transport layer beneath the Language
# Server Protocol (LSP). Any LSP server in this repository
# delegates all framing and dispatch to this gem.
#
# Architecture:
#
#   stdin  →  MessageReader  →  Server  →  MessageWriter  →  stdout
#                                  ↓
#                          on_request / on_notification blocks
#
# Quick start:
#
#   require "coding_adventures_json_rpc"
#   CA = CodingAdventures::JsonRpc
#
#   CA::Server.new(STDIN, STDOUT)
#     .on_request("initialize") { |_id, _params| { capabilities: {} } }
#     .on_notification("textDocument/didOpen") { |params| ... }
#     .serve
#
# ================================================================

require_relative "coding_adventures/json_rpc/version"
require_relative "coding_adventures/json_rpc/errors"
require_relative "coding_adventures/json_rpc/message"
require_relative "coding_adventures/json_rpc/reader"
require_relative "coding_adventures/json_rpc/writer"
require_relative "coding_adventures/json_rpc/server"
