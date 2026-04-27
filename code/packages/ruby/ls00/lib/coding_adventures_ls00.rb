# frozen_string_literal: true

# ================================================================
# coding_adventures_ls00 -- top-level entry point
# ================================================================
#
# Generic Language Server Protocol (LSP) framework.
#
# This gem implements the protocol boilerplate that every LSP server
# needs. A language author only writes a "bridge" object (see
# language_bridge.rb for the duck-type contract) that connects their
# lexer/parser to this framework.
#
# Architecture:
#
#   Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
#
# JSON-RPC over stdio:
#
#   Like the Debug Adapter Protocol (DAP), LSP speaks JSON-RPC over
#   stdio. Each message is Content-Length-framed (same format as HTTP
#   headers). The underlying transport is handled by the json_rpc gem.
#
# How to use this gem:
#
#   1. Create a bridge object implementing +tokenize+ and +parse+
#      (plus any optional methods like +hover+, +definition+, etc.)
#   2. Create a server:
#        server = CodingAdventures::Ls00::LspServer.new(bridge, STDIN, STDOUT)
#   3. Call server.serve -- it blocks until the editor closes the connection.
#
# ================================================================

require_relative "coding_adventures/ls00/version"
require_relative "coding_adventures/ls00/types"
require_relative "coding_adventures/ls00/language_bridge"
require_relative "coding_adventures/ls00/lsp_errors"
require_relative "coding_adventures/ls00/document_manager"
require_relative "coding_adventures/ls00/parse_cache"
require_relative "coding_adventures/ls00/capabilities"
require_relative "coding_adventures/ls00/handlers"
require_relative "coding_adventures/ls00/server"
