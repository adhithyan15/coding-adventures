# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00::Handlers — all LSP request/notification handlers
# ================================================================
#
# This module is mixed into LspServer to provide all handler methods.
# Each handler follows the same pattern:
#
#   1. Parse the params hash.
#   2. Check if the bridge supports the feature (respond_to?).
#   3. Get the parse result from the cache.
#   4. Delegate to the bridge.
#   5. Convert the result to LSP wire format.
#
# Handlers are split into categories:
#   - Lifecycle: initialize, initialized, shutdown, exit
#   - Text document sync: didOpen, didChange, didClose, didSave
#   - Feature requests: hover, definition, references, completion,
#     rename, documentSymbol, semanticTokens, foldingRange,
#     signatureHelp, formatting
#
# ================================================================

module CodingAdventures
  module Ls00
    module Handlers
      # ── Lifecycle ──────────────────────────────────────────────────

      # handle_initialize processes the LSP initialize request.
      #
      # This is the server's first message. We return capabilities built
      # from the bridge's optional methods.
      def handle_initialize(_id, _params)
        @initialized = true

        caps = Ls00.build_capabilities(@bridge)

        {
          "capabilities" => caps,
          "serverInfo" => {
            "name" => "ls00-generic-lsp-server",
            "version" => "0.1.0"
          }
        }
      end

      # handle_initialized processes the "initialized" notification.
      #
      # This is the editor's acknowledgment that it received our capabilities.
      # No-op: the handshake is complete.
      def handle_initialized(_params)
        # No-op
      end

      # handle_shutdown processes the LSP shutdown request.
      #
      # After receiving shutdown, the server should stop processing new
      # requests. Returns nil (null in JSON).
      def handle_shutdown(_id, _params)
        @shutdown = true
        nil
      end

      # handle_exit processes the "exit" notification.
      #
      # Exit code semantics (from the LSP spec):
      #   0: shutdown was received before exit -> clean shutdown
      #   1: shutdown was NOT received -> abnormal termination
      def handle_exit(_params)
        exit(@shutdown ? 0 : 1)
      end

      # ── Text document synchronization ──────────────────────────────

      # handle_did_open is called when the editor opens a file.
      #
      # Params: {"textDocument" => {"uri" => "...", "languageId" => "...",
      #          "version" => 1, "text" => "..."}}
      def handle_did_open(params)
        return unless params.is_a?(Hash)

        td = params["textDocument"]
        return unless td.is_a?(Hash)

        uri = td["uri"]
        text = td["text"] || ""
        version = td["version"] || 1

        return if uri.nil? || uri.empty?

        @doc_manager.open(uri, text, version)

        result = @parse_cache.get_or_parse(uri, version, text, @bridge)
        publish_diagnostics(uri, version, result.diagnostics)
      end

      # handle_did_change is called when the user edits a file.
      #
      # Params: {"textDocument" => {"uri" => "...", "version" => 2},
      #          "contentChanges" => [...]}
      def handle_did_change(params)
        return unless params.is_a?(Hash)

        uri = parse_uri(params)
        return if uri.nil? || uri.empty?

        version = 0
        if (td = params["textDocument"]) && td.is_a?(Hash)
          version = td["version"].to_i if td["version"]
        end

        changes_raw = params["contentChanges"]
        return unless changes_raw.is_a?(Array)

        changes = changes_raw.filter_map do |change_map|
          next unless change_map.is_a?(Hash)

          new_text = change_map["text"] || ""
          range = nil
          if change_map.key?("range") && change_map["range"]
            range = parse_lsp_range(change_map["range"])
          end

          TextChange.new(range: range, new_text: new_text)
        end

        begin
          @doc_manager.apply_changes(uri, changes, version)
        rescue RuntimeError
          return # document wasn't open
        end

        doc = @doc_manager.get(uri)
        return unless doc

        result = @parse_cache.get_or_parse(uri, doc.version, doc.text, @bridge)
        publish_diagnostics(uri, version, result.diagnostics)
      end

      # handle_did_close is called when the editor closes a file.
      def handle_did_close(params)
        return unless params.is_a?(Hash)

        uri = parse_uri(params)
        return if uri.nil? || uri.empty?

        @doc_manager.close(uri)
        @parse_cache.evict(uri)

        # Clear diagnostics for the closed file by publishing an empty list.
        publish_diagnostics(uri, 0, [])
      end

      # handle_did_save is called when the editor saves a file.
      def handle_did_save(params)
        return unless params.is_a?(Hash)

        uri = parse_uri(params)
        return if uri.nil? || uri.empty?

        text = params["text"]
        return unless text.is_a?(String) && !text.empty?

        doc = @doc_manager.get(uri)
        return unless doc

        @doc_manager.close(uri)
        @doc_manager.open(uri, text, doc.version)
        result = @parse_cache.get_or_parse(uri, doc.version, text, @bridge)
        publish_diagnostics(uri, doc.version, result.diagnostics)
      end

      # ── Feature requests ───────────────────────────────────────────

      # handle_hover processes the textDocument/hover request.
      def handle_hover(_id, params)
        return nil unless params.is_a?(Hash)

        uri = parse_uri(params)
        pos = parse_position(params)

        return nil unless @bridge.respond_to?(:hover)

        _, parse_result = get_parse_result(uri)
        return nil unless parse_result&.ast

        hover_result = @bridge.hover(parse_result.ast, pos)
        return nil unless hover_result

        result = {
          "contents" => {
            "kind" => "markdown",
            "value" => hover_result.contents
          }
        }

        if hover_result.range
          result["range"] = range_to_lsp(hover_result.range)
        end

        result
      end

      # handle_definition processes the textDocument/definition request.
      def handle_definition(_id, params)
        return nil unless params.is_a?(Hash)

        uri = parse_uri(params)
        pos = parse_position(params)

        return nil unless @bridge.respond_to?(:definition)

        _, parse_result = get_parse_result(uri)
        return nil unless parse_result&.ast

        location = @bridge.definition(parse_result.ast, pos, uri)
        return nil unless location

        location_to_lsp(location)
      end

      # handle_references processes the textDocument/references request.
      def handle_references(_id, params)
        return [] unless params.is_a?(Hash)

        uri = parse_uri(params)
        pos = parse_position(params)

        include_decl = false
        if (ctx = params["context"]) && ctx.is_a?(Hash)
          include_decl = ctx["includeDeclaration"] == true
        end

        return [] unless @bridge.respond_to?(:references)

        _, parse_result = get_parse_result(uri)
        return [] unless parse_result&.ast

        locations = @bridge.references(parse_result.ast, pos, uri, include_decl)
        return [] unless locations

        locations.map { |loc| location_to_lsp(loc) }
      end

      # handle_completion processes the textDocument/completion request.
      def handle_completion(_id, params)
        empty_result = { "isIncomplete" => false, "items" => [] }
        return empty_result unless params.is_a?(Hash)

        uri = parse_uri(params)
        pos = parse_position(params)

        return empty_result unless @bridge.respond_to?(:completion)

        _, parse_result = get_parse_result(uri)
        return empty_result unless parse_result&.ast

        items = @bridge.completion(parse_result.ast, pos)
        return empty_result unless items

        lsp_items = items.map do |item|
          ci = { "label" => item.label }
          ci["kind"] = item.kind if item.kind && item.kind != 0
          ci["detail"] = item.detail if item.detail && !item.detail.empty?
          ci["documentation"] = item.documentation if item.documentation && !item.documentation.empty?
          ci["insertText"] = item.insert_text if item.insert_text && !item.insert_text.empty?
          ci["insertTextFormat"] = item.insert_text_format if item.insert_text_format && item.insert_text_format != 0
          ci
        end

        { "isIncomplete" => false, "items" => lsp_items }
      end

      # handle_rename processes the textDocument/rename request.
      def handle_rename(_id, params)
        return error_response(JsonRpc::ErrorCodes::INVALID_PARAMS, "invalid params") unless params.is_a?(Hash)

        uri = parse_uri(params)
        pos = parse_position(params)
        new_name = params["newName"]

        if new_name.nil? || new_name.empty?
          return error_response(JsonRpc::ErrorCodes::INVALID_PARAMS, "newName is required")
        end

        unless @bridge.respond_to?(:rename)
          return error_response(LspErrors::REQUEST_FAILED, "rename not supported")
        end

        _, parse_result = get_parse_result(uri)
        unless parse_result&.ast
          return error_response(LspErrors::REQUEST_FAILED, "no AST available")
        end

        edit = @bridge.rename(parse_result.ast, pos, new_name)
        unless edit
          return error_response(LspErrors::REQUEST_FAILED, "symbol not found at position")
        end

        lsp_changes = {}
        edit.changes.each do |edit_uri, edits|
          lsp_changes[edit_uri] = edits.map do |te|
            { "range" => range_to_lsp(te.range), "newText" => te.new_text }
          end
        end

        { "changes" => lsp_changes }
      end

      # handle_document_symbol processes the textDocument/documentSymbol request.
      def handle_document_symbol(_id, params)
        return [] unless params.is_a?(Hash)

        uri = parse_uri(params)

        return [] unless @bridge.respond_to?(:document_symbols)

        _, parse_result = get_parse_result(uri)
        return [] unless parse_result&.ast

        symbols = @bridge.document_symbols(parse_result.ast)
        return [] unless symbols

        convert_document_symbols(symbols)
      end

      # handle_semantic_tokens_full processes the textDocument/semanticTokens/full request.
      def handle_semantic_tokens_full(_id, params)
        empty_result = { "data" => [] }
        return empty_result unless params.is_a?(Hash)

        uri = parse_uri(params)

        return empty_result unless @bridge.respond_to?(:semantic_tokens)

        doc = @doc_manager.get(uri)
        return empty_result unless doc

        tokens = @bridge.tokenize(doc.text)
        return empty_result unless tokens

        sem_tokens = @bridge.semantic_tokens(doc.text, tokens)
        return empty_result unless sem_tokens

        data = Ls00.encode_semantic_tokens(sem_tokens)
        { "data" => data }
      end

      # handle_folding_range processes the textDocument/foldingRange request.
      def handle_folding_range(_id, params)
        return [] unless params.is_a?(Hash)

        uri = parse_uri(params)

        return [] unless @bridge.respond_to?(:folding_ranges)

        _, parse_result = get_parse_result(uri)
        return [] unless parse_result&.ast

        ranges = @bridge.folding_ranges(parse_result.ast)
        return [] unless ranges

        ranges.map do |fr|
          m = { "startLine" => fr.start_line, "endLine" => fr.end_line }
          m["kind"] = fr.kind if fr.kind && !fr.kind.empty?
          m
        end
      end

      # handle_signature_help processes the textDocument/signatureHelp request.
      def handle_signature_help(_id, params)
        return nil unless params.is_a?(Hash)

        uri = parse_uri(params)
        pos = parse_position(params)

        return nil unless @bridge.respond_to?(:signature_help)

        _, parse_result = get_parse_result(uri)
        return nil unless parse_result&.ast

        sig_help = @bridge.signature_help(parse_result.ast, pos)
        return nil unless sig_help

        lsp_sigs = sig_help.signatures.map do |sig|
          lsp_params = (sig.parameters || []).map do |param|
            pp = { "label" => param.label }
            pp["documentation"] = param.documentation if param.documentation && !param.documentation.empty?
            pp
          end
          s = { "label" => sig.label, "parameters" => lsp_params }
          s["documentation"] = sig.documentation if sig.documentation && !sig.documentation.empty?
          s
        end

        {
          "signatures" => lsp_sigs,
          "activeSignature" => sig_help.active_signature,
          "activeParameter" => sig_help.active_parameter
        }
      end

      # handle_formatting processes the textDocument/formatting request.
      def handle_formatting(_id, params)
        return [] unless params.is_a?(Hash)

        uri = parse_uri(params)

        return [] unless @bridge.respond_to?(:format)

        doc = @doc_manager.get(uri)
        return [] unless doc

        edits = @bridge.format(doc.text)
        return [] unless edits

        edits.map do |edit|
          { "range" => range_to_lsp(edit.range), "newText" => edit.new_text }
        end
      end

      private

      # ── LSP type conversion helpers ────────────────────────────────

      # position_to_lsp converts a Position to a JSON-serializable hash.
      def position_to_lsp(pos)
        { "line" => pos.line, "character" => pos.character }
      end

      # range_to_lsp converts an LspRange to a JSON-serializable hash.
      def range_to_lsp(r)
        { "start" => position_to_lsp(r.start), "end" => position_to_lsp(r.end_pos) }
      end

      # location_to_lsp converts a Location to a JSON-serializable hash.
      def location_to_lsp(loc)
        { "uri" => loc.uri, "range" => range_to_lsp(loc.range) }
      end

      # parse_position extracts a Position from JSON params.
      def parse_position(params)
        pos = params["position"] || {}
        Position.new(
          line: (pos["line"] || 0).to_i,
          character: (pos["character"] || 0).to_i
        )
      end

      # parse_uri extracts the document URI from params.
      def parse_uri(params)
        td = params["textDocument"]
        return nil unless td.is_a?(Hash)

        td["uri"]
      end

      # parse_lsp_range parses a raw JSON range hash from the LSP protocol.
      def parse_lsp_range(raw)
        return LspRange.new(start: Position.new(line: 0, character: 0),
                            end_pos: Position.new(line: 0, character: 0)) unless raw.is_a?(Hash)

        start_map = raw["start"] || {}
        end_map = raw["end"] || {}

        LspRange.new(
          start: Position.new(
            line: (start_map["line"] || 0).to_i,
            character: (start_map["character"] || 0).to_i
          ),
          end_pos: Position.new(
            line: (end_map["line"] || 0).to_i,
            character: (end_map["character"] || 0).to_i
          )
        )
      end

      # error_response creates a ResponseError for returning from request handlers.
      def error_response(code, message)
        JsonRpc::ResponseError.new(code: code, message: message)
      end

      # convert_document_symbols recursively converts DocumentSymbol arrays
      # to JSON-serializable hashes for the LSP response.
      def convert_document_symbols(symbols)
        symbols.map do |sym|
          m = {
            "name" => sym.name,
            "kind" => sym.kind,
            "range" => range_to_lsp(sym.range),
            "selectionRange" => range_to_lsp(sym.selection_range)
          }
          if sym.children && !sym.children.empty?
            m["children"] = convert_document_symbols(sym.children)
          end
          m
        end
      end
    end
  end
end
