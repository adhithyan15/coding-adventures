package rpc

// message.go — RPC Message Types (codec-agnostic sum type)
//
// # What is an RpcMessage?
//
// When two processes communicate over RPC, they exchange messages. There are
// exactly four kinds of messages:
//
//  1. RpcRequest — "please do X and tell me the result" (has an id for correlation)
//  2. RpcResponse — "here is the result of request id N" (success path)
//  3. RpcErrorResponse — "request id N failed, here is why" (error path)
//  4. RpcNotification — "FYI: event Y happened" (one-way, no response expected)
//
// Think of RPC like an office intercom system:
//   - Request:      "Alice to Bob, can you check inventory item #42? [ticket 7]"
//   - Response:     "[ticket 7] Bob to Alice: item #42 has 3 units in stock"
//   - ErrorResponse:"[ticket 7] Bob to Alice: item #42 does not exist"
//   - Notification: "IT to all: system maintenance tonight at 10pm"
//
// The V type parameter is the codec's native "any value" type — for JSON it is
// a parsed JSON value (map, slice, string, number…), for MessagePack it is an
// rmpv.Value, etc. The rpc layer never inspects V; it only passes it to handlers.
//
// # Sum type via interface + sealed marker
//
// Go does not have algebraic data types, so we simulate a sealed sum type:
//
//   - RpcMessage[V] is a private interface with a private marker method rpcMessage().
//   - The four concrete types implement that private method.
//   - Because the method is unexported, no external type can satisfy the interface.
//
// Callers use a type switch to handle each variant:
//
//	switch m := msg.(type) {
//	case *rpc.RpcRequest[V]:
//	    // ...
//	case *rpc.RpcResponse[V]:
//	    // ...
//	case *rpc.RpcErrorResponse[V]:
//	    // ...
//	case *rpc.RpcNotification[V]:
//	    // ...
//	}
//
// # Why a pointer params field (*V) instead of a plain V?
//
// Params and results can be absent (null/nil in JSON, absent in Protobuf).
// Using *V captures the three-way distinction:
//   - nil   → field was absent (omitted from the message entirely)
//   - &zero → field was present but the zero value of V
//   - &val  → field was present with value val
//
// This is important: a request with no params (omit "params") is different
// from a request with explicit null params. Codecs must preserve this.

// RpcId is the type of a request/response correlation id.
//
// Ids are always string or integer. The rpc layer accepts any (any) to remain
// compatible with both — callers should only assign strings or ints. The null
// id (nil) is reserved for error responses to malformed requests where the
// original id could not be recovered.
type RpcId = any

// RpcMessage is the sealed sum type for all four RPC message variants.
//
// The private rpcMessage() method ensures only the four concrete types in this
// package satisfy the interface. Callers use a type switch to dispatch on the
// specific variant.
type RpcMessage[V any] interface {
	// rpcMessage is a private marker method that seals this interface.
	// It has no parameters or return values; its sole purpose is to prevent
	// external types from accidentally implementing RpcMessage[V].
	rpcMessage()
}

// ============================================================================
// RpcRequest — a call that expects a response
// ============================================================================

// RpcRequest is a request from a client to a server. The server must reply
// with exactly one RpcResponse or RpcErrorResponse carrying the same Id.
//
// Fields:
//   - Id:     unique correlation token (string or int). Assigned by the client.
//   - Method: the procedure name, e.g. "textDocument/hover" or "add".
//   - Params: optional arguments. nil means "no params"; &v means "params = v".
//
// Wire analogy: like a postal letter with a tracking number (Id) and an
// address/subject line (Method). The recipient keeps the tracking number to
// write on the reply envelope.
type RpcRequest[V any] struct {
	// Id is the client-assigned correlation token. The server echoes it on the
	// response so the client can match response to request.
	Id RpcId

	// Method is the name of the procedure being called, e.g. "echo" or
	// "textDocument/hover". Case-sensitive. Must not be empty.
	Method string

	// Params holds the input arguments for the procedure. nil means the call
	// was sent without a params field; &v means params = v.
	Params *V
}

// rpcMessage seals RpcRequest[V] as a member of the RpcMessage[V] sum type.
func (r *RpcRequest[V]) rpcMessage() {}

// ============================================================================
// RpcResponse — the success reply to a request
// ============================================================================

// RpcResponse is the success reply sent by the server after handling an
// RpcRequest. The Id must equal the originating RpcRequest's Id.
//
// If the handler failed, the server sends RpcErrorResponse instead — never
// both RpcResponse and RpcErrorResponse for the same request.
//
// Wire analogy: the reply envelope with the original tracking number (Id) and
// the goods the client asked for (Result).
type RpcResponse[V any] struct {
	// Id echoes the originating RpcRequest's Id so the client can correlate.
	Id RpcId

	// Result holds the procedure's return value. nil means the procedure
	// returned an explicit null/empty result (not an error). &v means result = v.
	Result *V
}

// rpcMessage seals RpcResponse[V] as a member of the RpcMessage[V] sum type.
func (r *RpcResponse[V]) rpcMessage() {}

// ============================================================================
// RpcNotification — a one-way push message with no expected response
// ============================================================================

// RpcNotification is a one-way fire-and-forget message. It has no Id because
// no response is expected. The receiver must never send a response — not even
// an error response for unknown methods.
//
// Used for event streams: the server pushing diagnostics to the editor, the
// client notifying the server that a file was saved, a metrics emitter logging
// events without waiting for acknowledgement.
//
// Wire analogy: a broadcast announcement over the PA system. No tracking
// number; no reply required; the sender does not wait.
type RpcNotification[V any] struct {
	// Method is the notification event name, e.g. "textDocument/publishDiagnostics".
	Method string

	// Params holds the event payload. nil means the notification was sent
	// without params; &v means params = v.
	Params *V
}

// rpcMessage seals RpcNotification[V] as a member of the RpcMessage[V] sum type.
func (n *RpcNotification[V]) rpcMessage() {}
