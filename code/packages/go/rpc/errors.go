// Package rpc defines the codec-agnostic Remote Procedure Call primitive.
//
// # What is rpc?
//
// JSON-RPC 2.0 is one concrete instance of a more general pattern: one process
// calls named procedures on another process, passing parameters and receiving
// results or errors. The *serialization format* (JSON, MessagePack, Protobuf)
// and the *framing scheme* (Content-Length headers, length-prefix, newlines) are
// separable concerns. The core RPC semantics — requests, responses, notifications,
// error codes, method dispatch, id correlation — are identical regardless of how
// the bytes look on the wire.
//
// This package captures those shared semantics. Codec-specific packages (json-rpc,
// msgpack-rpc, protobuf-rpc) are thin layers that supply a concrete RpcCodec and
// RpcFramer on top of this package.
//
// # Three-layer architecture
//
//	┌───────────────────────────────────────────────────────────────────┐
//	│  Application  (LSP server, test client, custom tool, …)           │
//	├───────────────────────────────────────────────────────────────────┤
//	│  rpc                                                              │
//	│  RpcServer / RpcClient                                            │
//	│  (method dispatch, id correlation, error handling,                │
//	│   handler registry, panic recovery)                               │
//	├───────────────────────────────────────────────────────────────────┤
//	│  RpcCodec                          ← JSON, Protobuf, MessagePack  │
//	│  (RpcMessage ↔ bytes)                                             │
//	├───────────────────────────────────────────────────────────────────┤
//	│  RpcFramer                         ← Content-Length, length-      │
//	│  (byte stream ↔ discrete chunks)     prefix, newline, WebSocket   │
//	├───────────────────────────────────────────────────────────────────┤
//	│  Transport                         ← stdin/stdout, TCP,           │
//	│  (raw byte stream)                   Unix socket, pipe             │
//	└───────────────────────────────────────────────────────────────────┘
//
// Each layer knows only about the layer immediately below it. The rpc layer
// never touches byte serialization. The codec never knows about method dispatch.
// The framer never knows about procedure names.
//
// # Usage example
//
//	// Construct an RpcServer using a JsonCodec and ContentLengthFramer
//	// (those come from the json-rpc package; this package only defines the
//	// interfaces they implement):
//	//
//	//   server := rpc.NewRpcServer(jsonCodec, contentLengthFramer)
//	//   server.OnRequest("add", func(id any, params *JsonValue) (*JsonValue, *rpc.RpcErrorResponse[JsonValue]) {
//	//       ...
//	//   })
//	//   server.Serve()
package rpc

import "fmt"

// ============================================================================
// Standard RPC Error Codes
// ============================================================================
//
// Error codes are codec-agnostic integers — the same table applies whether
// the wire format is JSON, MessagePack, or Protobuf. They are directly
// borrowed from the JSON-RPC 2.0 specification, which in turn borrowed the
// numbering scheme from XML-RPC.
//
// Think of these like HTTP status codes: a small vocabulary of integers that
// a receiver can branch on without needing to parse the human-readable message.
//
// Standard error code table:
//
//	Code     Name               Condition
//	------   ----------------   --------------------------------------------
//	-32700   ParseError         Framed bytes could not be decoded by codec
//	-32600   InvalidRequest     Decoded successfully, but not a valid message
//	-32601   MethodNotFound     No handler registered for the method name
//	-32602   InvalidParams      Handler rejected the params shape as wrong
//	-32603   InternalError      Unexpected panic inside a handler
//
// Server-defined errors may use the range [-32099, -32000].
// LSP reserves [-32899, -32800] for protocol-level errors; this layer must
// never emit codes in that range — it belongs to the application above.

const (
	// ParseError (-32700): the bytes produced by the framer could not be decoded
	// by the codec. This happens before the message type is determined — the raw
	// bytes are not valid for the chosen encoding format (e.g., not valid JSON,
	// not valid MessagePack).
	//
	// Example: the content-length header claims 42 bytes but the payload is
	// only 10 bytes of a truncated JSON object.
	ParseError = -32700

	// InvalidRequest (-32600): the codec decoded the bytes successfully, but the
	// resulting value does not satisfy the RPC message schema (missing required
	// fields, wrong field types, unrecognized shape).
	//
	// Example for JSON: the JSON was valid but the object had no "method" or
	// "result" field, so it could not be classified as any message type.
	InvalidRequest = -32600

	// MethodNotFound (-32601): the request was valid, but the server has no
	// registered handler for the requested method name. Analogous to HTTP 404
	// for a procedure call.
	//
	// The server sends this error back with the original request's id, so the
	// client can correlate it to the call that failed.
	MethodNotFound = -32601

	// InvalidParams (-32602): the method exists and has a handler, but the
	// provided params have the wrong shape — missing required fields, wrong
	// types, values out of range. Analogous to HTTP 422.
	//
	// This is returned by handlers that do their own parameter validation,
	// not by the RPC layer itself.
	InvalidParams = -32602

	// InternalError (-32603): an unexpected error occurred inside the server.
	// Used as a catch-all when a handler panics or returns an unrecoverable
	// error. Analogous to HTTP 500.
	//
	// The server recovers from the panic, sends this code, and continues
	// processing subsequent messages — the server process does not crash.
	InternalError = -32603
)

// ============================================================================
// RpcErrorResponse — the structured error type
// ============================================================================

// RpcErrorResponse is returned when an RPC call fails. It carries a numeric
// Code, a human-readable Message, and an optional Data payload of type V that
// can hold codec-native additional context (e.g., a stack trace as a JSON string,
// or a structured error object).
//
// RpcErrorResponse[V] also implements the RpcMessage[V] interface, so it can
// be used as a top-level message when a server sends an error response.
//
// Example:
//
//	errResp := &rpc.RpcErrorResponse[MyValue]{
//	    Id:      42,
//	    Code:    rpc.MethodNotFound,
//	    Message: "Method not found: foo",
//	    Data:    nil,
//	}
type RpcErrorResponse[V any] struct {
	// Id is the request id from the originating RpcRequest, so the client can
	// correlate this error to the call that failed. It is nil when the request
	// was so malformed that its id could not be recovered (ParseError or
	// InvalidRequest cases).
	Id any

	// Code is a signed integer error code from the standard table or the
	// server-defined range [-32099, -32000].
	Code int

	// Message is a short, human-readable description of the error. It is
	// intended for logging and debugging, not for programmatic branching (use
	// Code for that).
	Message string

	// Data is an optional codec-native value with additional context. For JSON,
	// this might be a string describing the panic message. For Protobuf, it
	// might be a structured proto.Message. The rpc layer never inspects Data.
	Data *V
}

// rpcMessage is the sealed interface marker. Only types in this package
// can implement RpcMessage[V].
func (e *RpcErrorResponse[V]) rpcMessage() {}

// Error implements the standard Go error interface so RpcErrorResponse can
// be returned from functions that return error.
//
// Example:
//
//	var err error = &rpc.RpcErrorResponse[MyValue]{Code: rpc.MethodNotFound, Message: "not found"}
//	fmt.Println(err) // → "rpc error -32601: not found"
func (e *RpcErrorResponse[V]) Error() string {
	return fmt.Sprintf("rpc error %d: %s", e.Code, e.Message)
}
