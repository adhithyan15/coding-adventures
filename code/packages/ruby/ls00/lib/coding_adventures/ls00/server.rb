# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00::LspServer — the main coordinator
# ================================================================
#
# LspServer wires together:
#   - The LanguageBridge (language-specific logic via duck typing)
#   - The DocumentManager (tracks open file contents)
#   - The ParseCache (avoids redundant parses)
#   - The JSON-RPC Server (protocol layer)
#
# It registers all LSP request and notification handlers with the
# JSON-RPC server, then calls serve to start the blocking
# read-dispatch-write loop.
#
# # Server Lifecycle
#
#   Client (editor)              Server (us)
#     |                               |
#     |--initialize-------------->    |  store clientInfo, return capabilities
#     | <-----------------result-     |
#     |                               |
#     |--initialized (notif)------>   |  no-op (handshake complete)
#     |                               |
#     |--textDocument/didOpen------>  |  open doc, parse, push diagnostics
#     |--textDocument/didChange---->  |  apply change, re-parse, push diagnostics
#     |--textDocument/hover-------->  |  get parse result, call bridge.hover
#     | <-----------------result-     |
#     |                               |
#     |--shutdown------------------> |  set shutdown flag, return null
#     |--exit (notif)---------------> |  exit(0) or exit(1)
#
# # Sending Notifications to the Editor
#
# The JSON-RPC Server handles request/response pairs. But the LSP server
# also needs to PUSH notifications to the editor (e.g.,
# textDocument/publishDiagnostics). We do this by holding a reference to
# the JSON-RPC MessageWriter and calling write_message directly.
#
# ================================================================

require "coding_adventures_json_rpc"
require_relative "handlers"

module CodingAdventures
  module Ls00
    class LspServer
      include Handlers

      # Create an LspServer wired to read from +in_stream+ and write to
      # +out_stream+.
      #
      # Typically:
      #   server = CodingAdventures::Ls00::LspServer.new(my_bridge, STDIN, STDOUT)
      #   server.serve
      #
      # For testing, pass StringIO or pipe pairs as in_stream and out_stream.
      def initialize(bridge, in_stream, out_stream)
        @bridge = bridge
        @doc_manager = DocumentManager.new
        @parse_cache = ParseCache.new
        @rpc_server = JsonRpc::Server.new(in_stream, out_stream)
        @writer = JsonRpc::MessageWriter.new(out_stream)
        @shutdown = false
        @initialized = false

        register_handlers
      end

      # serve starts the blocking JSON-RPC read-dispatch-write loop.
      #
      # This call blocks until the editor closes the connection (EOF on stdin).
      # All LSP messages are handled synchronously in this loop.
      def serve
        @rpc_server.serve
      end

      private

      # send_notification sends a server-initiated notification to the editor.
      #
      # LSP servers push certain events proactively without the editor asking.
      # The most important is textDocument/publishDiagnostics, which is sent
      # after every parse to update the editor's squiggle underlines.
      def send_notification(method, params)
        notif = JsonRpc::Notification.new(method: method, params: params)
        @writer.write_message(notif)
      end

      # register_handlers wires all LSP method names to their Ruby handler methods.
      def register_handlers
        # -- Lifecycle --
        @rpc_server.on_request("initialize") { |id, params| handle_initialize(id, params) }
        @rpc_server.on_notification("initialized") { |params| handle_initialized(params) }
        @rpc_server.on_request("shutdown") { |id, params| handle_shutdown(id, params) }
        @rpc_server.on_notification("exit") { |params| handle_exit(params) }

        # -- Text document synchronization --
        @rpc_server.on_notification("textDocument/didOpen") { |params| handle_did_open(params) }
        @rpc_server.on_notification("textDocument/didChange") { |params| handle_did_change(params) }
        @rpc_server.on_notification("textDocument/didClose") { |params| handle_did_close(params) }
        @rpc_server.on_notification("textDocument/didSave") { |params| handle_did_save(params) }

        # -- Feature requests (all conditional on bridge capability) --
        @rpc_server.on_request("textDocument/hover") { |id, params| handle_hover(id, params) }
        @rpc_server.on_request("textDocument/definition") { |id, params| handle_definition(id, params) }
        @rpc_server.on_request("textDocument/references") { |id, params| handle_references(id, params) }
        @rpc_server.on_request("textDocument/completion") { |id, params| handle_completion(id, params) }
        @rpc_server.on_request("textDocument/rename") { |id, params| handle_rename(id, params) }
        @rpc_server.on_request("textDocument/documentSymbol") { |id, params| handle_document_symbol(id, params) }
        @rpc_server.on_request("textDocument/semanticTokens/full") { |id, params| handle_semantic_tokens_full(id, params) }
        @rpc_server.on_request("textDocument/foldingRange") { |id, params| handle_folding_range(id, params) }
        @rpc_server.on_request("textDocument/signatureHelp") { |id, params| handle_signature_help(id, params) }
        @rpc_server.on_request("textDocument/formatting") { |id, params| handle_formatting(id, params) }
      end

      # get_parse_result retrieves the current parse result for a document.
      #
      # This is the hot path for all feature handlers. It:
      #  1. Gets the current document text from the DocumentManager
      #  2. Returns the cached ParseResult (or re-parses if needed)
      #
      # Returns [doc, parse_result] or [nil, nil] if the document is not open.
      def get_parse_result(uri)
        doc = @doc_manager.get(uri)
        return [nil, nil] unless doc

        result = @parse_cache.get_or_parse(uri, doc.version, doc.text, @bridge)
        [doc, result]
      end

      # publish_diagnostics sends the textDocument/publishDiagnostics
      # notification to the editor.
      #
      # Called after every didOpen and didChange event to update the
      # squiggle underlines in the editor.
      def publish_diagnostics(uri, version, diagnostics)
        lsp_diags = diagnostics.map do |d|
          diag = {
            "range" => range_to_lsp(d.range),
            "severity" => d.severity,
            "message" => d.message
          }
          diag["code"] = d.code if d.code && !d.code.empty?
          diag
        end

        params = { "uri" => uri, "diagnostics" => lsp_diags }
        params["version"] = version if version > 0

        # Best-effort: if the write fails, there's nothing we can do.
        begin
          send_notification("textDocument/publishDiagnostics", params)
        rescue StandardError
          # swallow
        end
      end
    end
  end
end
