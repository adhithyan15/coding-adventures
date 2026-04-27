// ============================================================================
// Server.swift — LspServer: the main coordinator
// ============================================================================
//
// LspServer wires together:
//   - The LanguageBridge (language-specific logic)
//   - The DocumentManager (tracks open file contents)
//   - The ParseCache (avoids redundant parses)
//   - The JSON-RPC Server (protocol layer)
//
// It registers all LSP request and notification handlers with the JSON-RPC
// server, then calls serve() to start the blocking read-dispatch-write loop.
//
// # Server Lifecycle
//
//   Client (editor)              Server (us)
//     |                               |
//     |--initialize--------------->   |  store clientInfo, return capabilities
//     | <-----------------result-     |
//     |                               |
//     |--initialized (notif)------>   |  no-op (handshake complete)
//     |                               |
//     |--textDocument/didOpen------>  |  open doc, parse, push diagnostics
//     |--textDocument/didChange---->  |  apply change, re-parse, push diagnostics
//     |--textDocument/hover-------->  |  get parse result, call bridge.hover
//     | <-----------------result-     |
//     |                               |
//     |--shutdown------------------>  |  set shutdown flag, return null
//     |--exit (notif)-------------->  |  exit process
//
// # Sending Notifications to the Editor
//
// The JSON-RPC Server handles request/response pairs. But the LSP server also
// needs to PUSH notifications to the editor (e.g., publishDiagnostics). We do
// this by holding a reference to the MessageWriter and calling writeMessage.
//
// ============================================================================

import Foundation
import JsonRpc

/// The main LSP server coordinating the bridge, document manager, parse cache,
/// and JSON-RPC communication.
///
/// Create with `LspServer(bridge:inputData:output:)`, then call `serve()`.
/// Designed for one-shot use: start, block, exit.
public class LspServer {
    let bridge: LanguageBridge
    let docManager: DocumentManager
    let parseCache: ParseCache
    private let rpcServer: JsonRpc.Server
    private let writer: JsonRpc.MessageWriter

    /// Whether the editor has sent "shutdown".
    var isShutdown = false

    /// Whether the initialize handshake is complete.
    var isInitialized = false

    /// Create an LspServer wired to read from inputData and write to output.
    ///
    /// Typically:
    ///   let server = LspServer(bridge: myBridge, inputData: stdinBytes, output: stdoutTarget)
    ///   server.serve()
    ///
    /// For testing, pass Data and DataOutput.
    public init(bridge: LanguageBridge, inputData: Data, output: OutputTarget) {
        self.bridge = bridge
        self.docManager = DocumentManager()
        self.parseCache = ParseCache()
        self.rpcServer = JsonRpc.Server(inputData: inputData, output: output)
        self.writer = JsonRpc.MessageWriter(output)

        registerHandlers()
    }

    /// Start the blocking JSON-RPC read-dispatch-write loop.
    ///
    /// Blocks until the editor closes the connection (EOF on stdin).
    /// All LSP messages are handled synchronously in this loop.
    public func serve() {
        rpcServer.serve()
    }

    // ----------------------------------------------------------------
    // sendNotification — push a server-initiated notification
    // ----------------------------------------------------------------
    //
    // LSP servers push certain events proactively. The most important is
    // textDocument/publishDiagnostics, sent after every parse.
    //

    /// Send a server-initiated notification to the editor.
    ///
    /// Notifications have no "id" and the editor sends no response.
    func sendNotification(method: String, params: Any) {
        let notif = JsonRpc.Notification(
            method: method,
            params: JsonRpc.AnySendable(params)
        )
        try? writer.writeMessage(.notification(notif))
    }

    // ----------------------------------------------------------------
    // registerHandlers — wire all LSP methods to handlers
    // ----------------------------------------------------------------

    private func registerHandlers() {
        // -- Lifecycle --
        rpcServer.onRequest("initialize") { [weak self] id, params in
            self?.handleInitialize(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onNotification("initialized") { [weak self] params in
            self?.handleInitialized(params: params)
        }
        rpcServer.onRequest("shutdown") { [weak self] id, params in
            self?.handleShutdown(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onNotification("exit") { [weak self] params in
            self?.handleExit(params: params)
        }

        // -- Text document synchronization --
        rpcServer.onNotification("textDocument/didOpen") { [weak self] params in
            self?.handleDidOpen(params: params)
        }
        rpcServer.onNotification("textDocument/didChange") { [weak self] params in
            self?.handleDidChange(params: params)
        }
        rpcServer.onNotification("textDocument/didClose") { [weak self] params in
            self?.handleDidClose(params: params)
        }
        rpcServer.onNotification("textDocument/didSave") { [weak self] params in
            self?.handleDidSave(params: params)
        }

        // -- Feature requests --
        rpcServer.onRequest("textDocument/hover") { [weak self] id, params in
            self?.handleHover(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/definition") { [weak self] id, params in
            self?.handleDefinition(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/references") { [weak self] id, params in
            self?.handleReferences(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/completion") { [weak self] id, params in
            self?.handleCompletion(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/rename") { [weak self] id, params in
            self?.handleRename(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/documentSymbol") { [weak self] id, params in
            self?.handleDocumentSymbol(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/semanticTokens/full") { [weak self] id, params in
            self?.handleSemanticTokensFull(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/foldingRange") { [weak self] id, params in
            self?.handleFoldingRange(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/signatureHelp") { [weak self] id, params in
            self?.handleSignatureHelp(id: id, params: params) ?? (nil, nil)
        }
        rpcServer.onRequest("textDocument/formatting") { [weak self] id, params in
            self?.handleFormatting(id: id, params: params) ?? (nil, nil)
        }
    }

    // ----------------------------------------------------------------
    // getParseResult — hot path for all feature handlers
    // ----------------------------------------------------------------

    /// Retrieve the current parse result for a document.
    ///
    /// Gets the document text from the DocumentManager, then returns the
    /// cached ParseResult (or re-parses if needed).
    ///
    /// - Returns: (document, parseResult) or nil if document not open.
    func getParseResult(uri: String) -> (Document, ParseResult)? {
        guard let doc = docManager.get(uri: uri) else {
            return nil
        }
        let result = parseCache.getOrParse(uri: uri, version: doc.version, source: doc.text, bridge: bridge)
        return (doc, result)
    }

    // ----------------------------------------------------------------
    // publishDiagnostics — push diagnostics to the editor
    // ----------------------------------------------------------------

    /// Send textDocument/publishDiagnostics to update squiggles in the editor.
    ///
    /// Called after every didOpen and didChange to update diagnostic display.
    func publishDiagnostics(uri: String, version: Int, diagnostics: [Diagnostic]) {
        let lspDiags: [[String: Any]] = diagnostics.map { d in
            var diag: [String: Any] = [
                "range": Self.rangeToLSP(d.range),
                "severity": d.severity.rawValue,
                "message": d.message,
            ]
            if let code = d.code {
                diag["code"] = code
            }
            return diag
        }

        var params: [String: Any] = [
            "uri": uri,
            "diagnostics": lspDiags,
        ]
        if version > 0 {
            params["version"] = version
        }

        sendNotification(method: "textDocument/publishDiagnostics", params: params)
    }

    // ----------------------------------------------------------------
    // LSP type conversion helpers
    // ----------------------------------------------------------------

    /// Convert a Position to an LSP-format dictionary.
    static func positionToLSP(_ p: Position) -> [String: Any] {
        return ["line": p.line, "character": p.character]
    }

    /// Convert a Range to an LSP-format dictionary.
    static func rangeToLSP(_ r: Range) -> [String: Any] {
        return ["start": Self.positionToLSP(r.start), "end": Self.positionToLSP(r.end)]
    }

    /// Convert a Location to an LSP-format dictionary.
    static func locationToLSP(_ l: Location) -> [String: Any] {
        return ["uri": l.uri, "range": Self.rangeToLSP(l.range)]
    }

    /// Extract a Position from JSON params.
    static func parsePosition(_ params: [String: Any]) -> Position {
        let pos = params["position"] as? [String: Any] ?? [:]
        let line = pos["line"] as? Int ?? (pos["line"] as? Double).map { Int($0) } ?? 0
        let char = pos["character"] as? Int ?? (pos["character"] as? Double).map { Int($0) } ?? 0
        return Position(line: line, character: char)
    }

    /// Extract the document URI from params with a textDocument field.
    static func parseURI(_ params: [String: Any]) -> String {
        let td = params["textDocument"] as? [String: Any] ?? [:]
        return td["uri"] as? String ?? ""
    }

    /// Parse an LSP range object from raw JSON.
    static func parseLSPRange(_ raw: Any) -> Range {
        guard let m = raw as? [String: Any] else { return Range(start: Position(line: 0, character: 0), end: Position(line: 0, character: 0)) }
        let startMap = m["start"] as? [String: Any] ?? [:]
        let endMap = m["end"] as? [String: Any] ?? [:]
        let startLine = startMap["line"] as? Int ?? (startMap["line"] as? Double).map { Int($0) } ?? 0
        let startChar = startMap["character"] as? Int ?? (startMap["character"] as? Double).map { Int($0) } ?? 0
        let endLine = endMap["line"] as? Int ?? (endMap["line"] as? Double).map { Int($0) } ?? 0
        let endChar = endMap["character"] as? Int ?? (endMap["character"] as? Double).map { Int($0) } ?? 0
        return Range(start: Position(line: startLine, character: startChar), end: Position(line: endLine, character: endChar))
    }
}

// Free-function wrappers for convenience in Handlers.swift
func positionToLSP(_ p: Position) -> [String: Any] { LspServer.positionToLSP(p) }
func rangeToLSP(_ r: Range) -> [String: Any] { LspServer.rangeToLSP(r) }
func locationToLSP(_ l: Location) -> [String: Any] { LspServer.locationToLSP(l) }
