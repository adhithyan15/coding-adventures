// Package jsonrpc implements the JSON-RPC 2.0 protocol over stdin/stdout
// using Content-Length framing.
//
// # Overview
//
// JSON-RPC 2.0 is a lightweight remote procedure call protocol that encodes
// calls as JSON objects. It is the wire protocol beneath the Language Server
// Protocol (LSP) and is used by every LSP server built in coding-adventures.
//
// # Building Blocks
//
// The package exposes three building blocks:
//
//  1. MessageReader — reads one Content-Length-framed message from a stream.
//  2. MessageWriter — writes one message to a stream with Content-Length framing.
//  3. Server — combines reader + writer with a method dispatch table.
//
// # Typical Usage
//
//	server := jsonrpc.NewServer(os.Stdin, os.Stdout)
//	server.OnRequest("initialize", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
//	    return map[string]interface{}{"capabilities": map[string]interface{}{}}, nil
//	})
//	server.OnNotification("textDocument/didOpen", func(params interface{}) {
//	    // store document
//	})
//	server.Serve() // blocks until stdin closes
//
// # Wire Format
//
// Each message is preceded by an HTTP-inspired header block:
//
//	Content-Length: <n>\r\n
//	\r\n
//	<UTF-8 JSON payload, exactly n bytes>
//
// Content-Length is a byte count, not a character count. For Unicode-heavy
// payloads (emoji, CJK characters) this distinction is critical.
package jsonrpc

import "fmt"

// ============================================================================
// Standard JSON-RPC 2.0 Error Codes
// ============================================================================
//
// The JSON-RPC 2.0 specification reserves a range of negative integer codes
// for well-known failure modes. These are analogous to HTTP status codes —
// a small vocabulary of integers that a receiver can switch on without
// needing to parse the human-readable message string.
//
// Standard error code table:
//
//	Code          Name               When to use
//	----          ----               -----------
//	-32700        Parse error        Payload is not valid JSON
//	-32600        Invalid Request    Valid JSON but not a Request object
//	-32601        Method not found   Method has no registered handler
//	-32602        Invalid params     Wrong parameter shape for a method
//	-32603        Internal error     Unhandled exception inside a handler
//
// Server-defined errors live in the range [-32099, -32000].
// LSP reserves [-32899, -32800] for protocol-level errors; this package
// should never emit codes in that range.

const (
	// ParseError (-32700): the incoming message body could not be parsed as JSON.
	// This happens before the message type is determined — the bytes on the wire
	// are not valid UTF-8 JSON. Example: Content-Length header points to "hello".
	ParseError = -32700

	// InvalidRequest (-32600): the JSON was valid, but the object does not satisfy
	// the Request schema. Examples: missing "jsonrpc" field, wrong "jsonrpc" value,
	// "method" is not a string.
	InvalidRequest = -32600

	// MethodNotFound (-32601): the requested method has no registered handler.
	// The server understood the request but does not know how to handle the
	// method name. Equivalent to HTTP 404 for a procedure call.
	MethodNotFound = -32601

	// InvalidParams (-32602): the method exists but the provided "params" have
	// the wrong shape. Use when a handler validates its own parameters and finds
	// missing required fields or wrong types. Equivalent to HTTP 422.
	InvalidParams = -32602

	// InternalError (-32603): an unexpected error occurred inside the server.
	// Use as a catch-all when a handler panics or returns an unexpected error.
	// Equivalent to HTTP 500.
	InternalError = -32603
)

// ============================================================================
// ResponseError — the error envelope
// ============================================================================

// ResponseError is an error object embedded inside a failed Response.
//
// It carries a numeric Code (from the standard table or a server-defined range),
// a human-readable Message, and an optional Data field for additional context.
//
// Example:
//
//	err := &ResponseError{
//	    Code:    MethodNotFound,
//	    Message: "Method not found",
//	    Data:    "textDocument/hover is not registered",
//	}
//
// Wire form:
//
//	{"code": -32601, "message": "Method not found", "data": "..."}
type ResponseError struct {
	Code    int
	Message string
	Data    interface{}
}

// Error implements the error interface so ResponseError can be used where
// a standard Go error is expected.
func (e *ResponseError) Error() string {
	return fmt.Sprintf("json-rpc error %d: %s", e.Code, e.Message)
}

// NewResponseError constructs a ResponseError. This is the preferred factory
// function for handler code since it enforces the required fields.
//
// Example:
//
//	return nil, jsonrpc.NewResponseError(jsonrpc.InvalidParams, "missing field 'uri'", nil)
func NewResponseError(code int, message string, data interface{}) *ResponseError {
	return &ResponseError{
		Code:    code,
		Message: message,
		Data:    data,
	}
}
