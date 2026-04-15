package rpc

// client.go — RpcClient: the request-response correlation loop
//
// # What does the client do?
//
// The RpcClient is the counterpart to RpcServer. Where the server waits for
// incoming messages and dispatches them to handlers, the client initiates
// requests and waits for the matching responses.
//
// Think of the client like a customer placing a phone order:
//   - They call the restaurant (Request).
//   - They wait on hold while the kitchen prepares the food.
//   - Occasionally the hold music is interrupted by unrelated announcements
//     (server-initiated Notifications, e.g., "We're running 10 min late").
//   - Eventually the restaurant comes back on the line with the order
//     confirmation (Response with matching id) or an error ("sorry, we're
//     out of that dish").
//
// # Id correlation
//
// Each Request gets a unique integer id, starting at 1 and incrementing
// monotonically. The client loops reading frames until it receives a Response
// or ErrorResponse with the matching id. Frames with other ids are skipped
// (or dispatched if they are Notifications).
//
// Why ids? Consider two requests sent quickly:
//   - Request 1: "initialize" (id=1, might take 200ms)
//   - Request 2: "ping" (id=2, instant)
//
// The server might reply out of order. Without ids the client cannot tell
// which response belongs to which request. With ids it can correlate exactly.
//
// # Blocking model
//
// The current implementation is synchronous: Request() blocks until the
// matching response arrives. This is appropriate for the LSP use case where
// the editor sends one request at a time. For concurrent use, a
// goroutine-per-request model with a shared pending map would be layered on
// top — the handler API would not change.
//
// # Server-push notifications
//
// The server may send Notifications at any time — even while the client is
// blocked waiting for a response. The client registers notification handlers
// with OnNotification. When a Notification frame arrives during the Request()
// wait loop, it is dispatched to the matching handler, then the loop continues
// waiting for the actual response.
//
// Example: an LSP server sends "textDocument/publishDiagnostics" notifications
// while the client is blocked waiting for a "textDocument/hover" response.

import "io"

// RpcClient sends requests to a remote server and receives responses.
//
// Construct with NewRpcClient. Register server-push notification handlers
// with OnNotification. Send blocking requests with Request. Send fire-and-
// forget notifications with Notify.
//
// Type parameter V is the codec's native value type.
type RpcClient[V any] struct {
	codec                RpcCodec[V]
	framer               RpcFramer
	notificationHandlers map[string]func(*V)

	// nextId is the next request id to use. Starts at 1, increments by 1
	// for each Request call. Never reset. Not goroutine-safe.
	nextId int
}

// NewRpcClient creates a new RpcClient with the given codec and framer.
//
// The codec and framer must be the same type as the server's — they must
// speak the same protocol. A JsonCodec client talking to a MsgpackCodec
// server will fail to decode responses.
//
// Example:
//
//	client := rpc.NewRpcClient(myJsonCodec, myContentLengthFramer)
//	result, errResp, err := client.Request("ping", nil)
func NewRpcClient[V any](codec RpcCodec[V], framer RpcFramer) *RpcClient[V] {
	return &RpcClient[V]{
		codec:                codec,
		framer:               framer,
		notificationHandlers: make(map[string]func(*V)),
		nextId:               1,
	}
}

// OnNotification registers a handler for server-initiated notifications.
//
// When the server sends a Notification while the client is blocked in
// Request(), the notification is dispatched to the matching handler before
// the wait loop continues.
//
// This is used for server-push events: LSP's publishDiagnostics, log
// messages, progress notifications, etc.
//
// Returns the client itself so calls can be chained:
//
//	client.
//	    OnNotification("log", logHandler).
//	    OnNotification("progress", progressHandler)
func (c *RpcClient[V]) OnNotification(method string, handler func(*V)) *RpcClient[V] {
	c.notificationHandlers[method] = handler
	return c
}

// Request sends an RpcRequest to the server and blocks until the matching
// response arrives.
//
// Id management is handled internally: each call increments an internal
// counter and uses the new value as the request id. The counter starts at 1.
//
// Return values:
//   - (*V, nil, nil)       — server returned a successful result.
//   - (nil, *errResp, nil) — server returned an RpcErrorResponse.
//   - (nil, nil, err)      — I/O error or connection closed before response.
//
// While waiting for the response, the client dispatches any server-initiated
// Notifications to their registered OnNotification handlers.
//
// Example:
//
//	result, errResp, err := client.Request("add", &myParams)
//	if err != nil {
//	    log.Fatal("connection error:", err)
//	}
//	if errResp != nil {
//	    log.Printf("server error %d: %s", errResp.Code, errResp.Message)
//	    return
//	}
//	fmt.Println("result:", *result)
func (c *RpcClient[V]) Request(method string, params *V) (*V, *RpcErrorResponse[V], error) {
	// Allocate the next id. Simple monotonic counter — safe for single-threaded use.
	id := c.nextId
	c.nextId++

	// Build and send the request frame.
	req := &RpcRequest[V]{Id: id, Method: method, Params: params}
	data, err := c.codec.Encode(req)
	if err != nil {
		return nil, nil, err
	}
	if err := c.framer.WriteFrame(data); err != nil {
		return nil, nil, err
	}

	// Wait loop: keep reading frames until we find the one with our id.
	// Other message types (notifications, responses for other ids) are handled
	// or skipped, then the loop continues.
	for {
		frameData, err := c.framer.ReadFrame()
		if err == io.EOF {
			// Clean EOF before our response arrived: connection closed.
			return nil, nil, io.EOF
		}
		if err != nil {
			return nil, nil, err
		}

		msg, decErr := c.codec.Decode(frameData)
		if decErr != nil {
			// Decode error on a frame we received while waiting. Skip it and
			// keep waiting — perhaps the next frame is our response.
			continue
		}

		switch m := msg.(type) {
		case *RpcResponse[V]:
			// Is this the response to our request?
			if idsEqual(m.Id, id) {
				return m.Result, nil, nil
			}
			// Response for a different request id (e.g., a concurrent caller
			// in a future multi-request implementation). Ignore and keep waiting.

		case *RpcErrorResponse[V]:
			// Is this the error response to our request?
			if idsEqual(m.Id, id) {
				return nil, m, nil
			}
			// Error for a different id. Ignore and keep waiting.

		case *RpcNotification[V]:
			// Server-initiated push notification received while we wait.
			// Dispatch to the registered handler (if any), then keep waiting.
			if handler, ok := c.notificationHandlers[m.Method]; ok {
				func() {
					defer func() { recover() }() //nolint:errcheck
					handler(m.Params)
				}()
			}

		// RpcRequest from server (server-initiated request — rare, used in some
		// bidirectional protocols). We ignore it in this basic client model.
		}
	}
}

// Notify sends an RpcNotification to the server. No response is expected
// or waited for. This is a fire-and-forget operation.
//
// Use Notify for one-way events where the client does not need confirmation:
// "file was saved", "cursor moved", "settings changed".
//
// Example:
//
//	err := client.Notify("textDocument/didOpen", &didOpenParams)
func (c *RpcClient[V]) Notify(method string, params *V) error {
	notif := &RpcNotification[V]{Method: method, Params: params}
	data, err := c.codec.Encode(notif)
	if err != nil {
		return err
	}
	return c.framer.WriteFrame(data)
}

// idsEqual compares two RpcId values for equality.
//
// RpcId is `any`, which in Go means interface{}. Direct == comparison works
// for same-type pairs (int == int, string == string) but fails for mixed types
// (int(1) != float64(1.0) even though they represent the same number).
//
// JSON codecs often decode integer ids as float64 (Go's default for untyped
// JSON numbers). This helper normalises float64 → int before comparing, so
// that id=1 in the request matches id=1.0 decoded from the response JSON.
func idsEqual(a, b any) bool {
	// Normalise float64 → int (JSON integer ids arrive as float64).
	a = normalizeId(a)
	b = normalizeId(b)
	return a == b
}

// normalizeId converts float64 id values to int, leaving strings and ints
// unchanged. This handles the JSON decoding edge case where integer ids
// (e.g., 1) are decoded as float64 (1.0) by Go's encoding/json.
func normalizeId(id any) any {
	if f, ok := id.(float64); ok {
		return int(f)
	}
	return id
}
