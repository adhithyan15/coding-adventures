package ls00

// server.go — LspServer: the main coordinator
//
// LspServer wires together:
//   - The LanguageBridge (language-specific logic)
//   - The DocumentManager (tracks open file contents)
//   - The ParseCache (avoids redundant parses)
//   - The JSON-RPC Server (protocol layer)
//
// It registers all LSP request and notification handlers with the JSON-RPC
// server, then calls Serve() to start the blocking read-dispatch-write loop.
//
// # Server Lifecycle
//
//   Client (editor)              Server (us)
//     │                               │
//     ├──initialize──────────────►    │  store clientInfo, return capabilities
//     │ ◄─────────────────result─     │
//     │                               │
//     ├──initialized (notif)──────►   │  no-op (handshake complete)
//     │                               │
//     ├──textDocument/didOpen──────►  │  open doc, parse, push diagnostics
//     ├──textDocument/didChange────►  │  apply change, re-parse, push diagnostics
//     ├──textDocument/hover────────►  │  get parse result, call bridge.Hover
//     │ ◄─────────────────result─     │
//     │                               │
//     ├──shutdown───────────────────► │  set shutdown flag, return null
//     ├──exit (notif)───────────────► │  os.Exit(0) or os.Exit(1)
//
// # Sending Notifications to the Editor
//
// The JSON-RPC Server (from the json-rpc package) handles request/response
// pairs. But the LSP server also needs to PUSH notifications to the editor
// (e.g., textDocument/publishDiagnostics). We do this by holding a reference
// to the JSON-RPC MessageWriter and calling WriteMessage directly.

import (
	"io"
	"sync"

	jsonrpc "github.com/coding-adventures/json-rpc"
)

// LspServer is the main LSP server.
//
// Create it with NewLspServer(), then call Serve() to start serving.
// It is designed to be used once per process — start it, it blocks, it exits.
type LspServer struct {
	bridge     LanguageBridge
	docManager *DocumentManager
	parseCache *ParseCache
	rpcServer  *jsonrpc.Server
	writer     *jsonrpc.MessageWriter // for sending server-initiated notifications

	// shutdown tracks whether the editor has sent "shutdown".
	// After shutdown, the server must not process further requests (except "exit").
	shutdown bool

	// initialized tracks whether the initialize handshake is complete.
	// The server must reject requests before initialize is received.
	initialized bool

	// mu protects shutdown and initialized from concurrent access.
	// (In practice the server is single-threaded, but this is good practice.)
	mu sync.Mutex
}

// NewLspServer creates an LspServer wired to read from in and write to out.
//
// Typically:
//
//	server := ls00.NewLspServer(myBridge, os.Stdin, os.Stdout)
//	server.Serve()
//
// For testing, pass bytes.Buffer or pipe pairs as in and out.
func NewLspServer(bridge LanguageBridge, in io.Reader, out io.Writer) *LspServer {
	rpcServer := jsonrpc.NewServer(in, out)
	writer := jsonrpc.NewWriter(out)

	s := &LspServer{
		bridge:     bridge,
		docManager: NewDocumentManager(),
		parseCache: NewParseCache(),
		rpcServer:  rpcServer,
		writer:     writer,
	}

	s.registerHandlers()
	return s
}

// Serve starts the blocking JSON-RPC read-dispatch-write loop.
//
// This call blocks until the editor closes the connection (EOF on stdin).
// All LSP messages are handled synchronously in this loop.
func (s *LspServer) Serve() {
	s.rpcServer.Serve()
}

// sendNotification sends a server-initiated notification to the editor.
//
// LSP servers push certain events proactively without the editor asking.
// The most important is textDocument/publishDiagnostics, which is sent after
// every parse to update the editor's squiggle underlines.
//
// Notifications have no "id" and the editor sends no response. From the
// JSON-RPC spec: "A Notification is a Request object without an 'id' member."
func (s *LspServer) sendNotification(method string, params interface{}) error {
	notif := &jsonrpc.Notification{
		Method: method,
		Params: params,
	}
	return s.writer.WriteMessage(notif)
}

// registerHandlers wires all LSP method names to their Go handler functions.
//
// Requests (have an id, get a response):
//   initialize, shutdown, textDocument/hover, textDocument/definition,
//   textDocument/references, textDocument/completion, textDocument/rename,
//   textDocument/documentSymbol, textDocument/semanticTokens/full,
//   textDocument/foldingRange, textDocument/signatureHelp,
//   textDocument/formatting
//
// Notifications (no id, no response):
//   initialized, textDocument/didOpen, textDocument/didChange,
//   textDocument/didClose, textDocument/didSave
func (s *LspServer) registerHandlers() {
	// ── Lifecycle ────────────────────────────────────────────────────────────
	s.rpcServer.OnRequest("initialize", s.handleInitialize)
	s.rpcServer.OnNotification("initialized", s.handleInitialized)
	s.rpcServer.OnRequest("shutdown", s.handleShutdown)
	s.rpcServer.OnNotification("exit", s.handleExit)

	// ── Text document synchronization ────────────────────────────────────────
	s.rpcServer.OnNotification("textDocument/didOpen", s.handleDidOpen)
	s.rpcServer.OnNotification("textDocument/didChange", s.handleDidChange)
	s.rpcServer.OnNotification("textDocument/didClose", s.handleDidClose)
	s.rpcServer.OnNotification("textDocument/didSave", s.handleDidSave)

	// ── Feature requests (all conditional on bridge capability) ──────────────
	s.rpcServer.OnRequest("textDocument/hover", s.handleHover)
	s.rpcServer.OnRequest("textDocument/definition", s.handleDefinition)
	s.rpcServer.OnRequest("textDocument/references", s.handleReferences)
	s.rpcServer.OnRequest("textDocument/completion", s.handleCompletion)
	s.rpcServer.OnRequest("textDocument/rename", s.handleRename)
	s.rpcServer.OnRequest("textDocument/documentSymbol", s.handleDocumentSymbol)
	s.rpcServer.OnRequest("textDocument/semanticTokens/full", s.handleSemanticTokensFull)
	s.rpcServer.OnRequest("textDocument/foldingRange", s.handleFoldingRange)
	s.rpcServer.OnRequest("textDocument/signatureHelp", s.handleSignatureHelp)
	s.rpcServer.OnRequest("textDocument/formatting", s.handleFormatting)
}

// getParseResult retrieves the current parse result for a document.
//
// This is the hot path for all feature handlers. It:
//  1. Gets the current document text from the DocumentManager
//  2. Returns the cached ParseResult (or re-parses if needed)
//
// Returns (nil, error) if the document is not open.
func (s *LspServer) getParseResult(uri string) (*Document, *ParseResult, error) {
	doc, ok := s.docManager.Get(uri)
	if !ok {
		return nil, nil, &jsonrpc.ResponseError{
			Code:    RequestFailed,
			Message: "document not open: " + uri,
		}
	}

	result := s.parseCache.GetOrParse(uri, doc.Version, doc.Text, s.bridge)
	return doc, result, nil
}

// publishDiagnostics sends the textDocument/publishDiagnostics notification
// to the editor with the current diagnostic list for a document.
//
// This is called after every didOpen and didChange event to update the
// squiggle underlines in the editor. It is proactive — the server pushes
// diagnostics without the editor asking.
//
// The LSP spec guarantees that the editor will update its squiggles immediately
// upon receiving this notification.
func (s *LspServer) publishDiagnostics(uri string, version int, diagnostics []Diagnostic) {
	// Convert our internal Diagnostic slice to LSP format.
	// The LSP spec uses integer severity codes; our Diagnostic type already uses them.
	lspDiags := make([]interface{}, len(diagnostics))
	for i, d := range diagnostics {
		diag := map[string]interface{}{
			"range":    rangeToLSP(d.Range),
			"severity": int(d.Severity),
			"message":  d.Message,
		}
		if d.Code != "" {
			diag["code"] = d.Code
		}
		lspDiags[i] = diag
	}

	params := map[string]interface{}{
		"uri":         uri,
		"diagnostics": lspDiags,
	}
	if version > 0 {
		params["version"] = version
	}

	// Best-effort: if the write fails, there's nothing we can do. The editor
	// will just show stale diagnostics until the next successful publish.
	_ = s.sendNotification("textDocument/publishDiagnostics", params)
}

// ─── LSP type conversion helpers ─────────────────────────────────────────────

// positionToLSP converts our Position to a JSON-serializable map.
func positionToLSP(p Position) map[string]interface{} {
	return map[string]interface{}{
		"line":      p.Line,
		"character": p.Character,
	}
}

// rangeToLSP converts our Range to a JSON-serializable map.
func rangeToLSP(r Range) map[string]interface{} {
	return map[string]interface{}{
		"start": positionToLSP(r.Start),
		"end":   positionToLSP(r.End),
	}
}

// locationToLSP converts a Location to a JSON-serializable map.
func locationToLSP(l Location) map[string]interface{} {
	return map[string]interface{}{
		"uri":   l.URI,
		"range": rangeToLSP(l.Range),
	}
}

// parsePosition extracts a Position from a JSON params object.
// The LSP sends positions as {"line": N, "character": N}.
func parsePosition(params map[string]interface{}) Position {
	pos, _ := params["position"].(map[string]interface{})
	line, _ := pos["line"].(float64)
	char, _ := pos["character"].(float64)
	return Position{Line: int(line), Character: int(char)}
}

// parseURI extracts the document URI from params that have a textDocument field.
func parseURI(params map[string]interface{}) string {
	td, _ := params["textDocument"].(map[string]interface{})
	uri, _ := td["uri"].(string)
	return uri
}
