// ============================================================================
// Handlers.swift — all LSP handler implementations
// ============================================================================
//
// This file contains all handler methods for the LspServer. Each handler
// corresponds to one LSP method (initialize, shutdown, didOpen, hover, etc.).
//
// Handlers are split into three categories:
//   1. Lifecycle: initialize, initialized, shutdown, exit
//   2. Text document sync: didOpen, didChange, didClose, didSave
//   3. Feature requests: hover, definition, references, completion, rename,
//      documentSymbol, semanticTokens, foldingRange, signatureHelp, formatting
//
// ============================================================================

import Foundation
import JsonRpc

extension LspServer {

    // ====================================================================
    // LIFECYCLE HANDLERS
    // ====================================================================

    /// Handle the LSP initialize request.
    ///
    /// The first message from the editor. We return our capabilities
    /// built dynamically from the bridge's protocol conformance.
    func handleInitialize(id: Any, params: Any?) -> (Any?, ResponseError?) {
        isInitialized = true

        let caps = buildCapabilities(bridge)

        let result: [String: Any] = [
            "capabilities": caps,
            "serverInfo": [
                "name": "ls00-generic-lsp-server",
                "version": "0.1.0",
            ] as [String: Any],
        ]

        return (result, nil)
    }

    /// Handle the "initialized" notification.
    ///
    /// The editor's acknowledgment that the handshake is complete.
    /// No action needed -- the initialize handler already set our flag.
    func handleInitialized(params: Any?) {
        // No-op: handshake complete. Normal operation begins.
    }

    /// Handle the LSP shutdown request.
    ///
    /// Sets the shutdown flag and returns null. The server must NOT exit
    /// here -- it waits for the "exit" notification.
    func handleShutdown(id: Any, params: Any?) -> (Any?, ResponseError?) {
        isShutdown = true
        return (nil, nil)
    }

    /// Handle the "exit" notification.
    ///
    /// Terminates the process. Exit code 0 if shutdown was received first,
    /// 1 if exit arrives without a prior shutdown.
    func handleExit(params: Any?) {
        // In a real server we would call exit(). For testability, we
        // just set the flag. The serve() loop will end at EOF anyway.
        // Production servers should call: exit(isShutdown ? 0 : 1)
    }

    // ====================================================================
    // TEXT DOCUMENT SYNCHRONIZATION HANDLERS
    // ====================================================================

    /// Handle textDocument/didOpen.
    ///
    /// The editor sends the full file content when a file is opened.
    /// We store it and immediately parse + push diagnostics.
    func handleDidOpen(params: Any?) {
        guard let p = params as? [String: Any],
              let td = p["textDocument"] as? [String: Any],
              let uri = td["uri"] as? String, !uri.isEmpty else { return }

        let text = td["text"] as? String ?? ""
        let version = td["version"] as? Int
            ?? (td["version"] as? Double).map { Int($0) }
            ?? 1

        docManager.open(uri: uri, text: text, version: version)

        let result = parseCache.getOrParse(uri: uri, version: version, source: text, bridge: bridge)
        publishDiagnostics(uri: uri, version: version, diagnostics: result.diagnostics)
    }

    /// Handle textDocument/didChange.
    ///
    /// The editor sends incremental changes. We apply them, re-parse,
    /// and push updated diagnostics.
    func handleDidChange(params: Any?) {
        guard let p = params as? [String: Any] else { return }

        let uri = LspServer.parseURI(p)
        guard !uri.isEmpty else { return }

        var version = 0
        if let td = p["textDocument"] as? [String: Any] {
            version = td["version"] as? Int
                ?? (td["version"] as? Double).map { Int($0) }
                ?? 0
        }

        // Parse content changes. JSONSerialization may produce [Any] rather than
        // [[String: Any]], so we cast the outer array first and each element individually.
        let changesArray = p["contentChanges"] as? [Any] ?? []
        var changes: [TextChange] = []

        for changeRaw in changesArray {
            guard let changeMap = changeRaw as? [String: Any] else { continue }
            let newText = changeMap["text"] as? String ?? ""
            var range: Range? = nil

            if let rangeRaw = changeMap["range"] {
                range = LspServer.parseLSPRange(rangeRaw)
            }

            changes.append(TextChange(range: range, newText: newText))
        }

        if docManager.applyChanges(uri: uri, changes: changes, version: version) != nil {
            return // document wasn't open
        }

        guard let doc = docManager.get(uri: uri) else { return }

        let result = parseCache.getOrParse(uri: uri, version: doc.version, source: doc.text, bridge: bridge)
        publishDiagnostics(uri: uri, version: version, diagnostics: result.diagnostics)
    }

    /// Handle textDocument/didClose.
    ///
    /// Remove the document from tracking and clear its diagnostics.
    func handleDidClose(params: Any?) {
        guard let p = params as? [String: Any] else { return }
        let uri = LspServer.parseURI(p)
        guard !uri.isEmpty else { return }

        docManager.close(uri: uri)
        parseCache.evict(uri: uri)

        // Clear diagnostics by publishing an empty list.
        publishDiagnostics(uri: uri, version: 0, diagnostics: [])
    }

    /// Handle textDocument/didSave.
    ///
    /// If the editor sends full text in didSave, apply it.
    /// Otherwise just republish from current state.
    func handleDidSave(params: Any?) {
        guard let p = params as? [String: Any] else { return }
        let uri = LspServer.parseURI(p)
        guard !uri.isEmpty else { return }

        if let text = p["text"] as? String, !text.isEmpty {
            if let doc = docManager.get(uri: uri) {
                docManager.close(uri: uri)
                docManager.open(uri: uri, text: text, version: doc.version)
                let result = parseCache.getOrParse(uri: uri, version: doc.version, source: text, bridge: bridge)
                publishDiagnostics(uri: uri, version: doc.version, diagnostics: result.diagnostics)
            }
        }
    }

    // ====================================================================
    // FEATURE HANDLERS
    // ====================================================================

    /// Handle textDocument/hover.
    ///
    /// Returns hover tooltip content for the symbol at the cursor position.
    func handleHover(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)
        let pos = LspServer.parsePosition(p)

        guard let hp = bridge as? HoverProvider else { return (nil, nil) }
        guard let (_, parseResult) = getParseResult(uri: uri) else { return (nil, nil) }
        guard parseResult.ast != nil else { return (nil, nil) }

        let (hoverResult, _) = hp.hover(ast: parseResult.ast!, pos: pos)
        guard let hoverResult else { return (nil, nil) }

        var result: [String: Any] = [
            "contents": [
                "kind": "markdown",
                "value": hoverResult.contents,
            ] as [String: Any],
        ]

        if let range = hoverResult.range {
            result["range"] = Self.rangeToLSP(range)
        }

        return (result, nil)
    }

    /// Handle textDocument/definition.
    func handleDefinition(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)
        let pos = LspServer.parsePosition(p)

        guard let dp = bridge as? DefinitionProvider else { return (nil, nil) }
        guard let (_, parseResult) = getParseResult(uri: uri) else { return (nil, nil) }
        guard parseResult.ast != nil else { return (nil, nil) }

        let (location, _) = dp.definition(ast: parseResult.ast!, pos: pos, uri: uri)
        guard let location else { return (nil, nil) }

        return (Self.locationToLSP(location), nil)
    }

    /// Handle textDocument/references.
    func handleReferences(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)
        let pos = LspServer.parsePosition(p)

        var includeDecl = false
        if let ctx = p["context"] as? [String: Any],
           let incl = ctx["includeDeclaration"] as? Bool {
            includeDecl = incl
        }

        guard let rp = bridge as? ReferencesProvider else { return ([] as [Any], nil) }
        guard let (_, parseResult) = getParseResult(uri: uri) else { return ([] as [Any], nil) }
        guard parseResult.ast != nil else { return ([] as [Any], nil) }

        let (locations, _) = rp.references(ast: parseResult.ast!, pos: pos, uri: uri, includeDecl: includeDecl)
        let result = locations.map { Self.locationToLSP($0) }
        return (result, nil)
    }

    /// Handle textDocument/completion.
    func handleCompletion(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)
        let pos = LspServer.parsePosition(p)
        let emptyResult: [String: Any] = ["isIncomplete": false, "items": [] as [Any]]

        guard let cp = bridge as? CompletionProvider else { return (emptyResult, nil) }
        guard let (_, parseResult) = getParseResult(uri: uri) else { return (emptyResult, nil) }
        guard parseResult.ast != nil else { return (emptyResult, nil) }

        let (items, _) = cp.completion(ast: parseResult.ast!, pos: pos)
        let lspItems: [[String: Any]] = items.map { item in
            var ci: [String: Any] = ["label": item.label]
            if let kind = item.kind { ci["kind"] = kind.rawValue }
            if let detail = item.detail { ci["detail"] = detail }
            if let doc = item.documentation { ci["documentation"] = doc }
            if let insertText = item.insertText { ci["insertText"] = insertText }
            if let fmt = item.insertTextFormat { ci["insertTextFormat"] = fmt }
            return ci
        }

        return (["isIncomplete": false, "items": lspItems] as [String: Any], nil)
    }

    /// Handle textDocument/rename.
    func handleRename(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)
        let pos = LspServer.parsePosition(p)
        let newName = p["newName"] as? String ?? ""

        if newName.isEmpty {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "newName is required"))
        }

        guard let rp = bridge as? RenameProvider else {
            return (nil, ResponseError(code: LspErrorCodes.requestFailed, message: "rename not supported"))
        }
        guard let (_, parseResult) = getParseResult(uri: uri) else {
            return (nil, ResponseError(code: LspErrorCodes.requestFailed, message: "document not open"))
        }
        guard parseResult.ast != nil else {
            return (nil, ResponseError(code: LspErrorCodes.requestFailed, message: "no AST available"))
        }

        let (edit, err) = rp.rename(ast: parseResult.ast!, pos: pos, newName: newName)
        if let err { return (nil, ResponseError(code: LspErrorCodes.requestFailed, message: err.localizedDescription)) }
        guard let edit else {
            return (nil, ResponseError(code: LspErrorCodes.requestFailed, message: "symbol not found at position"))
        }

        var lspChanges: [String: Any] = [:]
        for (editURI, edits) in edit.changes {
            let lspEdits: [[String: Any]] = edits.map { te in
                ["range": Self.rangeToLSP(te.range), "newText": te.newText]
            }
            lspChanges[editURI] = lspEdits
        }

        return (["changes": lspChanges] as [String: Any], nil)
    }

    /// Handle textDocument/documentSymbol.
    func handleDocumentSymbol(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)

        guard let dsp = bridge as? DocumentSymbolsProvider else { return ([] as [Any], nil) }
        guard let (_, parseResult) = getParseResult(uri: uri) else { return ([] as [Any], nil) }
        guard parseResult.ast != nil else { return ([] as [Any], nil) }

        let (symbols, _) = dsp.documentSymbols(ast: parseResult.ast!)
        return (convertDocumentSymbols(symbols), nil)
    }

    /// Handle textDocument/semanticTokens/full.
    func handleSemanticTokensFull(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)
        let emptyResult: [String: Any] = ["data": [] as [Int]]

        guard let stp = bridge as? SemanticTokensProvider else { return (emptyResult, nil) }
        guard let doc = docManager.get(uri: uri) else { return (emptyResult, nil) }

        let (tokens, _) = bridge.tokenize(source: doc.text)
        let (semTokens, _) = stp.semanticTokens(source: doc.text, tokens: tokens)
        let data = encodeSemanticTokens(semTokens)

        return (["data": data] as [String: Any], nil)
    }

    /// Handle textDocument/foldingRange.
    func handleFoldingRange(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)

        guard let frp = bridge as? FoldingRangesProvider else { return ([] as [Any], nil) }
        guard let (_, parseResult) = getParseResult(uri: uri) else { return ([] as [Any], nil) }
        guard parseResult.ast != nil else { return ([] as [Any], nil) }

        let (ranges, _) = frp.foldingRanges(ast: parseResult.ast!)
        let result: [[String: Any]] = ranges.map { fr in
            var m: [String: Any] = ["startLine": fr.startLine, "endLine": fr.endLine]
            if let kind = fr.kind { m["kind"] = kind }
            return m
        }

        return (result, nil)
    }

    /// Handle textDocument/signatureHelp.
    func handleSignatureHelp(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)
        let pos = LspServer.parsePosition(p)

        guard let shp = bridge as? SignatureHelpProvider else { return (nil, nil) }
        guard let (_, parseResult) = getParseResult(uri: uri) else { return (nil, nil) }
        guard parseResult.ast != nil else { return (nil, nil) }

        let (sigHelp, _) = shp.signatureHelp(ast: parseResult.ast!, pos: pos)
        guard let sigHelp else { return (nil, nil) }

        let lspSigs: [[String: Any]] = sigHelp.signatures.map { sig in
            let lspParams: [[String: Any]] = sig.parameters.map { param in
                var pp: [String: Any] = ["label": param.label]
                if let doc = param.documentation { pp["documentation"] = doc }
                return pp
            }
            var s: [String: Any] = ["label": sig.label, "parameters": lspParams]
            if let doc = sig.documentation { s["documentation"] = doc }
            return s
        }

        return ([
            "signatures": lspSigs,
            "activeSignature": sigHelp.activeSignature,
            "activeParameter": sigHelp.activeParameter,
        ] as [String: Any], nil)
    }

    /// Handle textDocument/formatting.
    func handleFormatting(id: Any, params: Any?) -> (Any?, ResponseError?) {
        guard let p = params as? [String: Any] else {
            return (nil, ResponseError(code: JsonRpcErrorCodes.invalidParams, message: "invalid params"))
        }

        let uri = LspServer.parseURI(p)

        guard let fp = bridge as? FormatProvider else { return ([] as [Any], nil) }
        guard let doc = docManager.get(uri: uri) else { return ([] as [Any], nil) }

        let (edits, err) = fp.format(source: doc.text)
        if let err {
            return (nil, ResponseError(code: LspErrorCodes.requestFailed, message: "formatting failed: \(err.localizedDescription)"))
        }

        let lspEdits: [[String: Any]] = edits.map { edit in
            ["range": Self.rangeToLSP(edit.range), "newText": edit.newText]
        }

        return (lspEdits, nil)
    }

    // ====================================================================
    // PRIVATE HELPERS
    // ====================================================================

    /// Recursively convert DocumentSymbol slices to JSON-serializable maps.
    private func convertDocumentSymbols(_ symbols: [DocumentSymbol]) -> [[String: Any]] {
        return symbols.map { sym in
            var m: [String: Any] = [
                "name": sym.name,
                "kind": sym.kind.rawValue,
                "range": Self.rangeToLSP(sym.range),
                "selectionRange": Self.rangeToLSP(sym.selectionRange),
            ]
            if !sym.children.isEmpty {
                m["children"] = convertDocumentSymbols(sym.children)
            }
            return m
        }
    }
}
