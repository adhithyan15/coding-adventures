package ls00

// handlers.go — initialize, initialized, shutdown, exit
//
// These four handlers implement the LSP server lifecycle. Every LSP session
// begins with initialize and ends with shutdown+exit. No other requests should
// be processed before initialize completes, or after shutdown is received.
//
// # initialize
//
// The initialize request is the first thing the editor sends. It includes:
//   - processId: the editor's process ID (for orphan detection)
//   - clientInfo: editor name and version
//   - capabilities: what the editor supports
//   - rootUri: the workspace root directory
//
// The server responds with its own capabilities (what features it supports).
// BuildCapabilities() in capabilities.go builds this dynamically from the bridge.
//
// # initialized
//
// After receiving the server's capabilities, the editor sends an "initialized"
// notification. This completes the handshake. From this point on, normal
// operation begins (didOpen, hover, etc.).
//
// # shutdown
//
// The editor sends "shutdown" before it disconnects. The server must:
//   1. Set a "shutdown" flag.
//   2. Return a null result (not an error).
//
// After shutdown, the server must reject any further requests (except "exit")
// with ServerNotInitialized error. The server does NOT exit on "shutdown" —
// it waits for "exit".
//
// # exit
//
// The "exit" notification tells the server to terminate.
//   - If "shutdown" was received before "exit": exit with code 0 (clean exit).
//   - If "shutdown" was NOT received before "exit": exit with code 1 (abnormal).
//
// This two-step shutdown (shutdown request + exit notification) allows the
// editor to receive confirmation that the shutdown request was processed before
// the server exits.

import (
	"os"

	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleInitialize processes the LSP initialize request.
//
// This is the server's first message. We store the client info (for logging)
// and return our capabilities built from the bridge.
func (s *LspServer) handleInitialize(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	s.mu.Lock()
	s.initialized = true
	s.mu.Unlock()

	// Build the server capabilities from the bridge's optional interfaces.
	// The editor uses these to decide which requests to send us.
	caps := BuildCapabilities(s.bridge)

	result := map[string]interface{}{
		"capabilities": caps,
		"serverInfo": map[string]interface{}{
			"name":    "ls00-generic-lsp-server",
			"version": "0.1.0",
		},
	}

	return result, nil
}

// handleInitialized processes the "initialized" notification.
//
// This is the editor's acknowledgment that it received our capabilities and
// the handshake is complete. We don't need to do anything here — the
// handleInitialize already set our initialized flag.
//
// Notification handlers return nothing (no response is ever sent for a notification).
func (s *LspServer) handleInitialized(params interface{}) {
	// No-op: the handshake is complete. Normal operation begins now.
	// Some servers use this to proactively scan the workspace for diagnostics,
	// but we wait for explicit didOpen events.
}

// handleShutdown processes the LSP shutdown request.
//
// After receiving shutdown, the server should:
//   - Stop processing new requests
//   - Return null as the result (not an error)
//   - Wait for the "exit" notification before actually exiting
//
// The JSON-RPC spec allows any result value; LSP specifies null/nil.
func (s *LspServer) handleShutdown(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	s.mu.Lock()
	s.shutdown = true
	s.mu.Unlock()

	// Return null result. In JSON this becomes {"jsonrpc":"2.0","id":N,"result":null}.
	return nil, nil
}

// handleExit processes the "exit" notification.
//
// This is a fire-and-forget notification — the editor doesn't wait for a response.
// We call os.Exit() directly.
//
// Exit code semantics (from the LSP spec):
//   - 0: shutdown was received before exit → clean shutdown
//   - 1: shutdown was NOT received → abnormal termination
func (s *LspServer) handleExit(params interface{}) {
	s.mu.Lock()
	wasShutdown := s.shutdown
	s.mu.Unlock()

	if wasShutdown {
		os.Exit(0)
	} else {
		os.Exit(1)
	}
}
