package rpc

// server.go — RpcServer: the read-dispatch-write loop
//
// # What does the server do?
//
// The RpcServer is the highest-level abstraction in this package. It binds a
// codec and a framer together with two handler registries (one for requests,
// one for notifications) and drives a blocking loop:
//
//	ReadFrame → Decode → Dispatch → Encode → WriteFrame
//	  (framer)   (codec)  (server)   (codec)   (framer)
//
// Each iteration processes exactly one message. The loop continues until the
// framer signals EOF (peer disconnected) or an unrecoverable error.
//
// # Architecture diagram
//
//	transport ──► framer.ReadFrame()
//	                    │ []byte
//	                    ▼
//	              codec.Decode()
//	                    │ RpcMessage[V]
//	                    ▼
//	             RpcServer.Serve() dispatch
//	                    │
//	        ┌───────────┴──────────────────┐
//	        │                              │
//	   RpcRequest?               RpcNotification?
//	        │                              │
//	  find handler               find handler (or drop)
//	  callWithRecover()          callWithRecover() — no response
//	  build RpcResponse          (spec: never respond to notification)
//	  codec.Encode()
//	  framer.WriteFrame()
//
// # Single-threaded model
//
// The Serve() loop processes one message at a time. This matches the LSP model
// (editor sends one request and waits) and is simple to reason about:
// no locks needed on the handler table, no race conditions in writes.
//
// If you need concurrency, the handlers themselves can spawn goroutines or
// use channels — the server's public API doesn't need to change.
//
// # Panic safety
//
// Handler panics are silently caught with recover(). The server sends an
// InternalError response and continues processing. A single buggy handler
// cannot crash the server process. This is critical for long-running servers
// (LSP servers, RPC daemons) where a restart would interrupt the client.

import "io"

// RpcServer combines a codec and a framer with two handler dispatch tables:
// one for request methods (expect a response) and one for notification methods
// (fire-and-forget, no response).
//
// Construct with NewRpcServer. Register handlers with OnRequest / OnNotification.
// Start the blocking loop with Serve.
//
// Type parameter V is the codec's native value type (e.g., map[string]interface{}
// for JSON, rmpv.Value for MessagePack).
type RpcServer[V any] struct {
	codec                RpcCodec[V]
	framer               RpcFramer
	requestHandlers      map[string]func(id any, params *V) (*V, *RpcErrorResponse[V])
	notificationHandlers map[string]func(params *V)
}

// NewRpcServer creates a new RpcServer with the given codec and framer.
//
// The codec translates RpcMessage[V] ↔ []byte.
// The framer reads and writes discrete byte chunks from the transport stream.
//
// Example:
//
//	server := rpc.NewRpcServer(myJsonCodec, myContentLengthFramer)
//	server.OnRequest("ping", func(id any, params *MyValue) (*MyValue, *rpc.RpcErrorResponse[MyValue]) {
//	    result := MyValue("pong")
//	    return &result, nil
//	})
//	server.Serve()
func NewRpcServer[V any](codec RpcCodec[V], framer RpcFramer) *RpcServer[V] {
	return &RpcServer[V]{
		codec:                codec,
		framer:               framer,
		requestHandlers:      make(map[string]func(id any, params *V) (*V, *RpcErrorResponse[V])),
		notificationHandlers: make(map[string]func(params *V)),
	}
}

// OnRequest registers a handler function for a named request method.
//
// The handler receives the request id and params. It must return either:
//   - (&result, nil)     → the server sends an RpcResponse with the result.
//   - (nil, &errResp)    → the server sends the RpcErrorResponse back to the client.
//
// Registering the same method twice silently replaces the earlier handler.
//
// Returns the server itself so calls can be chained:
//
//	server.
//	    OnRequest("add", addHandler).
//	    OnRequest("mul", mulHandler)
func (s *RpcServer[V]) OnRequest(method string, handler func(id any, params *V) (*V, *RpcErrorResponse[V])) *RpcServer[V] {
	s.requestHandlers[method] = handler
	return s
}

// OnNotification registers a handler function for a named notification method.
//
// The handler receives the notification params. No return value is expected —
// per spec, notifications never generate a response, not even an error.
//
// Registering the same method twice silently replaces the earlier handler.
//
// Returns the server itself so calls can be chained.
func (s *RpcServer[V]) OnNotification(method string, handler func(params *V)) *RpcServer[V] {
	s.notificationHandlers[method] = handler
	return s
}

// Serve starts the blocking read-dispatch-write loop.
//
// It reads frames from the framer, decodes them with the codec, dispatches
// each message to the appropriate handler, and writes any response frames.
//
// The loop exits on clean EOF from the framer (peer closed the connection).
// Framing errors, decode errors, and handler panics are all caught and handled
// gracefully — the loop continues after each recoverable error.
//
// Serve blocks the calling goroutine until the connection closes. Call it in
// the main goroutine or a dedicated goroutine:
//
//	go server.Serve()
func (s *RpcServer[V]) Serve() {
	for {
		// Step 1: read the next raw frame from the transport.
		data, err := s.framer.ReadFrame()

		if err == io.EOF {
			// Clean EOF — peer closed the connection. Shut down gracefully.
			// This is the normal exit path (e.g., editor closes stdin).
			return
		}

		if err != nil {
			// Framing error: malformed envelope (bad Content-Length header,
			// truncated length prefix, etc.). We cannot determine the request
			// id because we have no valid payload. Send ParseError with null id.
			s.sendError(nil, ParseError, err.Error())
			continue
		}

		// Step 2: decode the raw bytes into a typed RpcMessage[V].
		msg, decErr := s.codec.Decode(data)
		if decErr != nil {
			// Decode error: bytes were not valid for the codec's encoding format,
			// or valid bytes but not a recognizable RPC message shape.
			// Try to extract a structured error if the codec returned one.
			if errResp, ok := decErr.(*RpcErrorResponse[V]); ok {
				s.sendErrorResponse(errResp)
			} else {
				s.sendError(nil, ParseError, decErr.Error())
			}
			continue
		}

		// Step 3: dispatch to the appropriate handler.
		s.dispatch(msg)
	}
}

// dispatch routes a decoded RpcMessage to the right handler.
//
// The four message variants are handled as follows:
//   - RpcRequest:       find handler → call → write response (or MethodNotFound).
//   - RpcNotification:  find handler → call (no response, per spec).
//   - RpcResponse:      silently ignore (servers don't initiate requests in the
//     basic model; a bidirectional peer would forward to the pending table).
//   - RpcErrorResponse: silently ignore for the same reason.
func (s *RpcServer[V]) dispatch(msg RpcMessage[V]) {
	switch m := msg.(type) {
	case *RpcRequest[V]:
		s.handleRequest(m)
	case *RpcNotification[V]:
		s.handleNotification(m)
	// RpcResponse and RpcErrorResponse are silently ignored in pure server mode.
	}
}

// handleRequest invokes the registered handler for an RpcRequest and writes
// the result (or error) response back through the framer.
func (s *RpcServer[V]) handleRequest(req *RpcRequest[V]) {
	handler, ok := s.requestHandlers[req.Method]
	if !ok {
		// No handler registered for this method. Per spec §5.1: -32601.
		// The error response must carry the original request id so the client
		// can correlate it to the call that failed.
		s.sendError(req.Id, MethodNotFound, "Method not found")
		return
	}

	// Call the handler inside a recover() wrapper. If the handler panics
	// (e.g., nil pointer dereference, index out of range), we catch it and
	// return InternalError. The server continues running.
	result, errResp := s.callRequestHandler(handler, req.Id, req.Params)

	if errResp != nil {
		// Handler returned an explicit error — echo it back with the request id.
		errResp.Id = req.Id
		s.sendErrorResponse(errResp)
		return
	}

	// Success path: encode and write the response.
	resp := &RpcResponse[V]{Id: req.Id, Result: result}
	s.writeMessage(resp)
}

// callRequestHandler calls a request handler function with panic recovery.
//
// If the handler panics, callRequestHandler returns (nil, InternalError).
// The recover() must be in a separate function (not an inline defer in
// handleRequest) because deferred functions run when their enclosing function
// returns — a recover() in handleRequest would not catch panics in the handler
// unless it has its own defer stack.
func (s *RpcServer[V]) callRequestHandler(
	handler func(id any, params *V) (*V, *RpcErrorResponse[V]),
	id any,
	params *V,
) (result *V, errResp *RpcErrorResponse[V]) {
	defer func() {
		if r := recover(); r != nil {
			// A panic in the handler must not crash the server. Convert it to
			// InternalError. The panic value (r) is stored as a string in Data
			// if it can be expressed as a string; otherwise it is dropped.
			errResp = &RpcErrorResponse[V]{
				Code:    InternalError,
				Message: "Internal error: handler panicked",
			}
			result = nil
		}
	}()
	return handler(id, params)
}

// handleNotification dispatches a notification to its registered handler.
//
// Critical: the server must NEVER write a response to a notification, even
// if the method is unknown or the handler panics. The spec is explicit on this
// — sending a response to a notification would confuse clients that never
// expected one.
func (s *RpcServer[V]) handleNotification(notif *RpcNotification[V]) {
	handler, ok := s.notificationHandlers[notif.Method]
	if !ok {
		// Unknown notification: silently drop. Per spec, notifications for
		// unregistered methods are not an error — they may be sent by future
		// protocol extensions.
		return
	}

	// Call the handler with panic recovery. Even a panic must not produce
	// a response. We recover and discard.
	func() {
		defer func() { recover() }() //nolint:errcheck
		handler(notif.Params)
	}()
}

// sendError is a convenience helper that builds an RpcErrorResponse with the
// given id, code, and message, then encodes and writes it.
//
// id may be nil when the original request's id could not be recovered
// (ParseError or InvalidRequest before id extraction).
func (s *RpcServer[V]) sendError(id any, code int, message string) {
	s.sendErrorResponse(&RpcErrorResponse[V]{
		Id:      id,
		Code:    code,
		Message: message,
	})
}

// sendErrorResponse encodes and writes an RpcErrorResponse.
func (s *RpcServer[V]) sendErrorResponse(errResp *RpcErrorResponse[V]) {
	s.writeMessage(errResp)
}

// writeMessage encodes an RpcMessage and writes it as a frame.
// Errors from encoding or writing are silently discarded — if we cannot
// write to the transport, there is nothing useful we can do (logging would
// require a transport of its own, which is outside this package's scope).
func (s *RpcServer[V]) writeMessage(msg RpcMessage[V]) {
	data, err := s.codec.Encode(msg)
	if err != nil {
		return
	}
	_ = s.framer.WriteFrame(data) //nolint:errcheck
}
