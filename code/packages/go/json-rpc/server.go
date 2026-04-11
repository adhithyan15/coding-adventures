package jsonrpc

// server.go — Server: the read-dispatch-write loop
//
// The Server is the highest-level abstraction in this package. It wraps a
// MessageReader and a MessageWriter with a method dispatch table — a map
// from method names to handler functions.
//
// # Architecture
//
//	stdin ──► MessageReader
//	              │
//	              │ Message
//	              ▼
//	         Server.Serve() dispatch loop
//	              │
//	         ┌────┴──────────────┐
//	         │                   │
//	      Request?           Notification?
//	         │                   │
//	    find handler         find handler
//	    call(id, params)     call(params)
//	    write Response       (no response!)
//	         │
//	         ▼
//	    MessageWriter ──► stdout
//
// # Why Single-Threaded?
//
// The Serve() loop processes one message at a time. This matches the LSP model:
// editors send requests one at a time and wait for a response before sending
// the next (with the exception of notifications and $/cancelRequest).
//
// A single-threaded server is simple (no locks), correct, and predictable.
// If future servers need concurrent request handling, a goroutine-per-request
// variant can be layered on top without changing the handler API.
//
// # Handler Contract
//
// Request handlers:
//
//	func(id, params interface{}) (interface{}, *ResponseError)
//	  - Return (result, nil)         → success Response
//	  - Return (nil, responseError)  → error Response
//
// Notification handlers:
//
//	func(params interface{})
//	  - Fire-and-forget; return value ignored.
//	  - Must NOT return an error (per spec, notifications never get responses).

import (
	"io"
)

// RequestHandler is the signature for a JSON-RPC request handler.
//
// The handler receives the request id and params, and must return either:
//   - (result, nil) — any JSON-serializable value as a success result.
//   - (nil, *ResponseError) — an error to send back to the client.
type RequestHandler func(id interface{}, params interface{}) (interface{}, *ResponseError)

// NotificationHandler is the signature for a JSON-RPC notification handler.
//
// The handler receives the notification params. No return value is needed
// because notifications never get a response.
type NotificationHandler func(params interface{})

// Server combines a MessageReader and MessageWriter with a method dispatch table.
//
// Typical usage:
//
//	server := jsonrpc.NewServer(os.Stdin, os.Stdout)
//	server.OnRequest("initialize", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
//	    return map[string]interface{}{"capabilities": map[string]interface{}{}}, nil
//	})
//	server.OnNotification("textDocument/didOpen", func(params interface{}) {
//	    // handle document open event
//	})
//	server.Serve()
type Server struct {
	reader                *MessageReader
	writer                *MessageWriter
	requestHandlers       map[string]RequestHandler
	notificationHandlers  map[string]NotificationHandler
}

// NewServer creates a Server that reads from in and writes to out.
//
// In production: pass os.Stdin and os.Stdout.
// In tests: pass bytes.Buffer or bytes.Reader instances.
func NewServer(in io.Reader, out io.Writer) *Server {
	return &Server{
		reader:               NewReader(in),
		writer:               NewWriter(out),
		requestHandlers:      make(map[string]RequestHandler),
		notificationHandlers: make(map[string]NotificationHandler),
	}
}

// OnRequest registers a handler for a JSON-RPC request method.
//
// Returns the Server so calls can be chained:
//
//	server.OnRequest("foo", fooHandler).OnRequest("bar", barHandler)
func (s *Server) OnRequest(method string, handler RequestHandler) *Server {
	s.requestHandlers[method] = handler
	return s
}

// OnNotification registers a handler for a JSON-RPC notification method.
//
// Returns the Server so calls can be chained.
func (s *Server) OnNotification(method string, handler NotificationHandler) *Server {
	s.notificationHandlers[method] = handler
	return s
}

// Serve starts the blocking read-dispatch-write loop.
//
// Reads messages until EOF (when the client closes the input stream). For each:
//
//   - Request → look up handler by method. Call it and send the Response.
//     If no handler: send -32601 Method not found.
//   - Notification → look up handler by method. Call it silently.
//     If no handler: do nothing (spec requires silence for unknown notifications).
//   - Response → silently ignored (server mode; a future client API would forward
//     to a pending-request table).
//   - Parse/framing error → send -32700 or -32600 error response with id=nil.
func (s *Server) Serve() {
	for {
		msg, err := s.reader.ReadMessage()

		if err == io.EOF {
			// Clean EOF — client disconnected. Shut down gracefully.
			break
		}

		if err != nil {
			// Framing or parse error. We cannot determine the request id, so
			// we send an error response with id=nil.
			if respErr, ok := err.(*ResponseError); ok {
				s.sendError(nil, respErr.Code, respErr.Message)
			} else {
				s.sendError(nil, ParseError, err.Error())
			}
			continue
		}

		s.dispatch(msg)
	}
}

// dispatch routes a parsed message to the appropriate handler.
func (s *Server) dispatch(msg Message) {
	switch m := msg.(type) {
	case *Request:
		s.handleRequest(m)
	case *Notification:
		s.handleNotification(m)
	// *Response is silently ignored in server mode.
	}
}

// handleRequest invokes the registered handler for a Request and writes the Response.
func (s *Server) handleRequest(req *Request) {
	handler, ok := s.requestHandlers[req.Method]
	if !ok {
		// Spec §5.1: unknown method → -32601 Method not found.
		s.sendError(req.Id, MethodNotFound, "Method not found")
		return
	}

	// Call the handler. We use a deferred recover to convert panics into
	// -32603 Internal error responses, keeping the server alive despite
	// buggy handlers.
	result, respErr := s.callRequestHandler(handler, req.Id, req.Params)

	if respErr != nil {
		resp := &Response{Id: req.Id, Error: respErr}
		_ = s.writer.WriteMessage(resp) //nolint:errcheck
		return
	}

	resp := &Response{Id: req.Id, Result: result}
	_ = s.writer.WriteMessage(resp) //nolint:errcheck
}

// callRequestHandler calls a request handler with panic recovery.
//
// If the handler panics, it returns (nil, InternalError).
func (s *Server) callRequestHandler(handler RequestHandler, id, params interface{}) (result interface{}, respErr *ResponseError) {
	defer func() {
		if r := recover(); r != nil {
			respErr = &ResponseError{
				Code:    InternalError,
				Message: "Internal error: handler panicked",
				Data:    r,
			}
		}
	}()
	return handler(id, params)
}

// handleNotification invokes the registered handler for a Notification.
// No response is ever sent for a notification.
func (s *Server) handleNotification(notif *Notification) {
	handler, ok := s.notificationHandlers[notif.Method]
	if !ok {
		// Per spec: silently ignore unknown notifications.
		return
	}
	// Notification handler errors are swallowed — we must not send an
	// error response for a notification.
	defer func() { recover() }() //nolint:errcheck
	handler(notif.Params)
}

// sendError writes an error Response to the output stream.
// id may be nil if the original request id could not be determined.
func (s *Server) sendError(id interface{}, code int, message string) {
	resp := &Response{
		Id:    id,
		Error: &ResponseError{Code: code, Message: message},
	}
	_ = s.writer.WriteMessage(resp) //nolint:errcheck
}
