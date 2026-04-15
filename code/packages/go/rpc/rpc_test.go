package rpc_test

// rpc_test.go — Tests for the abstract rpc package
//
// # Strategy
//
// Because the rpc package defines only interfaces (RpcCodec, RpcFramer) and
// generic structs (RpcServer, RpcClient), we cannot test it without concrete
// implementations. This file provides two unexported mock implementations:
//
//   - mockCodec: encodes/decodes RpcMessage[string] using encoding/json.
//     The payload format is a flat JSON object with discriminating keys.
//     (Using encoding/json only in tests is fine — the point is the codec is
//     pluggable; the rpc package itself never imports encoding/json.)
//
//   - mockFramer: wraps two bytes.Buffer values (one for input, one for output).
//     WriteFrame appends a 4-byte big-endian length prefix then the payload.
//     ReadFrame reads the prefix, then reads exactly that many bytes.
//
// # Test groups
//
//	TestServer_*  — RpcServer behaviour (dispatch, errors, panics, notifications)
//	TestClient_*  — RpcClient behaviour (request/response, notify, notifications)
//	TestErrorCodes— Verify the exported constants have the right values
//	TestRpcErrorResponse_Error — RpcErrorResponse.Error() string formatting

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"strings"
	"testing"
	"time"

	"github.com/coding-adventures/rpc"
)

// ============================================================================
// mockCodec — encodes/decodes RpcMessage[string]
// ============================================================================
//
// Wire format: a flat JSON object. Discriminator rules:
//
//	Has "result" key                          → RpcResponse
//	Has "error" key                           → RpcErrorResponse
//	Has "method" and "id" (not "result/err")  → RpcRequest
//	Has "method" only                         → RpcNotification
//
// String params/results are stored under "params" and "result" respectively.
// The "id" field is an integer.

type mockCodec struct{}

// mockWire is the intermediate JSON shape used by mockCodec.
type mockWire struct {
	ID      any     `json:"id,omitempty"`
	Method  string  `json:"method,omitempty"`
	Params  *string `json:"params,omitempty"`
	Result  *string `json:"result,omitempty"`
	IsError bool    `json:"is_error,omitempty"`
	Code    int     `json:"code,omitempty"`
	Message string  `json:"message,omitempty"`
	Data    *string `json:"data,omitempty"`
}

func (c *mockCodec) Encode(msg rpc.RpcMessage[string]) ([]byte, error) {
	var w mockWire
	switch m := msg.(type) {
	case *rpc.RpcRequest[string]:
		w.ID = m.Id
		w.Method = m.Method
		w.Params = m.Params
	case *rpc.RpcResponse[string]:
		w.ID = m.Id
		w.Result = m.Result
	case *rpc.RpcErrorResponse[string]:
		w.ID = m.Id
		w.IsError = true
		w.Code = m.Code
		w.Message = m.Message
		w.Data = m.Data
	case *rpc.RpcNotification[string]:
		w.Method = m.Method
		w.Params = m.Params
	default:
		return nil, fmt.Errorf("mockCodec.Encode: unknown message type %T", msg)
	}
	return json.Marshal(w)
}

func (c *mockCodec) Decode(data []byte) (rpc.RpcMessage[string], error) {
	var w mockWire
	if err := json.Unmarshal(data, &w); err != nil {
		return nil, &rpc.RpcErrorResponse[string]{
			Code:    rpc.ParseError,
			Message: fmt.Sprintf("parse error: %s", err.Error()),
		}
	}

	if w.IsError {
		return &rpc.RpcErrorResponse[string]{
			Id:      normalizeTestID(w.ID),
			Code:    w.Code,
			Message: w.Message,
			Data:    w.Data,
		}, nil
	}
	if w.Result != nil {
		return &rpc.RpcResponse[string]{
			Id:     normalizeTestID(w.ID),
			Result: w.Result,
		}, nil
	}
	if w.Method != "" && w.ID != nil {
		return &rpc.RpcRequest[string]{
			Id:     normalizeTestID(w.ID),
			Method: w.Method,
			Params: w.Params,
		}, nil
	}
	if w.Method != "" {
		return &rpc.RpcNotification[string]{
			Method: w.Method,
			Params: w.Params,
		}, nil
	}

	return nil, &rpc.RpcErrorResponse[string]{
		Code:    rpc.InvalidRequest,
		Message: "invalid request: cannot determine message type",
	}
}

// normalizeTestID converts float64 JSON ids to int to match what RpcClient sends.
func normalizeTestID(id any) any {
	if f, ok := id.(float64); ok {
		return int(f)
	}
	return id
}

// ============================================================================
// mockFramer — length-prefixed in-memory framer
// ============================================================================
//
// Uses two bytes.Buffer values: `in` is pre-loaded with incoming frames;
// `out` accumulates outgoing frames written by the server/client.
//
// Frame format: 4-byte big-endian uint32 length, then `length` payload bytes.

type mockFramer struct {
	in  *bytes.Buffer
	out *bytes.Buffer
}

func newMockFramer() *mockFramer {
	return &mockFramer{
		in:  &bytes.Buffer{},
		out: &bytes.Buffer{},
	}
}

// WriteFrame appends a length-prefixed frame to `out`.
func (f *mockFramer) WriteFrame(data []byte) error {
	var lenBuf [4]byte
	binary.BigEndian.PutUint32(lenBuf[:], uint32(len(data)))
	f.out.Write(lenBuf[:])
	f.out.Write(data)
	return nil
}

// ReadFrame reads the next length-prefixed frame from `in`.
func (f *mockFramer) ReadFrame() ([]byte, error) {
	var lenBuf [4]byte
	if _, err := io.ReadFull(f.in, lenBuf[:]); err != nil {
		if err == io.EOF || err == io.ErrUnexpectedEOF {
			return nil, io.EOF
		}
		return nil, err
	}
	length := binary.BigEndian.Uint32(lenBuf[:])
	payload := make([]byte, length)
	if _, err := io.ReadFull(f.in, payload); err != nil {
		return nil, err
	}
	return payload, nil
}

// writeFrame is a helper that appends a length-prefixed frame to a buffer
// without a mockFramer (used to manually build `in` for server tests).
func writeTestFrame(buf *bytes.Buffer, data []byte) {
	var lenBuf [4]byte
	binary.BigEndian.PutUint32(lenBuf[:], uint32(len(data)))
	buf.Write(lenBuf[:])
	buf.Write(data)
}

// readTestFrame reads one frame from a buffer (helper for reading `out`).
func readTestFrame(buf *bytes.Buffer) ([]byte, error) {
	var lenBuf [4]byte
	if _, err := io.ReadFull(buf, lenBuf[:]); err != nil {
		return nil, err
	}
	length := binary.BigEndian.Uint32(lenBuf[:])
	payload := make([]byte, length)
	_, err := io.ReadFull(buf, payload)
	return payload, err
}

// ============================================================================
// Helpers
// ============================================================================

func newServerWithFramer() (*rpc.RpcServer[string], *mockFramer) {
	f := newMockFramer()
	s := rpc.NewRpcServer[string](&mockCodec{}, f)
	return s, f
}

func newClientWithFramer() (*rpc.RpcClient[string], *mockFramer) {
	f := newMockFramer()
	c := rpc.NewRpcClient[string](&mockCodec{}, f)
	return c, f
}

// encodeFrame serializes a message and appends a length-prefixed frame to buf.
func encodeFrame(t *testing.T, buf *bytes.Buffer, msg rpc.RpcMessage[string]) {
	t.Helper()
	codec := &mockCodec{}
	data, err := codec.Encode(msg)
	if err != nil {
		t.Fatalf("encodeFrame: %v", err)
	}
	writeTestFrame(buf, data)
}

// decodeFrame reads one frame from buf and decodes it.
func decodeFrame(t *testing.T, buf *bytes.Buffer) rpc.RpcMessage[string] {
	t.Helper()
	data, err := readTestFrame(buf)
	if err != nil {
		t.Fatalf("decodeFrame read: %v", err)
	}
	codec := &mockCodec{}
	msg, err := codec.Decode(data)
	if err != nil {
		t.Fatalf("decodeFrame decode: %v", err)
	}
	return msg
}

// ============================================================================
// TestErrorCodes — constants have correct values
// ============================================================================

func TestErrorCodes(t *testing.T) {
	tests := []struct {
		name string
		got  int
		want int
	}{
		{"ParseError", rpc.ParseError, -32700},
		{"InvalidRequest", rpc.InvalidRequest, -32600},
		{"MethodNotFound", rpc.MethodNotFound, -32601},
		{"InvalidParams", rpc.InvalidParams, -32602},
		{"InternalError", rpc.InternalError, -32603},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.got != tt.want {
				t.Errorf("got %d, want %d", tt.got, tt.want)
			}
		})
	}
}

// ============================================================================
// TestRpcErrorResponse_Error — Error() string formatting
// ============================================================================

func TestRpcErrorResponse_Error(t *testing.T) {
	e := &rpc.RpcErrorResponse[string]{
		Id:      1,
		Code:    rpc.MethodNotFound,
		Message: "Method not found",
	}
	got := e.Error()
	if !strings.Contains(got, "-32601") {
		t.Errorf("Error() = %q, want to contain -32601", got)
	}
	if !strings.Contains(got, "Method not found") {
		t.Errorf("Error() = %q, want to contain 'Method not found'", got)
	}
}

// ============================================================================
// TestServer_DispatchRequest — request is routed to handler, response written
// ============================================================================

func TestServer_DispatchRequest(t *testing.T) {
	server, framer := newServerWithFramer()

	// Register an "echo" handler that returns its params unchanged.
	server.OnRequest("echo", func(id any, params *string) (*string, *rpc.RpcErrorResponse[string]) {
		return params, nil
	})

	// Write a request frame into the framer's input buffer.
	p := "hello"
	encodeFrame(t, framer.in, &rpc.RpcRequest[string]{Id: 1, Method: "echo", Params: &p})

	// Run Serve in a goroutine; it will block after processing the one frame
	// when it hits EOF. We need EOF after the single message.
	go server.Serve()

	// Wait for the response to appear in the output buffer.
	resp := decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)

	respMsg, ok := resp.(*rpc.RpcResponse[string])
	if !ok {
		t.Fatalf("expected *RpcResponse[string], got %T", resp)
	}
	if respMsg.Result == nil || *respMsg.Result != "hello" {
		t.Errorf("expected result 'hello', got %v", respMsg.Result)
	}
	if respMsg.Id != 1 {
		t.Errorf("expected id 1, got %v", respMsg.Id)
	}
}

// ============================================================================
// TestServer_MethodNotFound — unknown method → -32601 error response
// ============================================================================

func TestServer_MethodNotFound(t *testing.T) {
	server, framer := newServerWithFramer()
	// No handlers registered.

	p := "ignored"
	encodeFrame(t, framer.in, &rpc.RpcRequest[string]{Id: 2, Method: "nonexistent", Params: &p})

	go server.Serve()

	resp := decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)

	errMsg, ok := resp.(*rpc.RpcErrorResponse[string])
	if !ok {
		t.Fatalf("expected *RpcErrorResponse[string], got %T", resp)
	}
	if errMsg.Code != rpc.MethodNotFound {
		t.Errorf("expected code %d, got %d", rpc.MethodNotFound, errMsg.Code)
	}
	if errMsg.Id != 2 {
		t.Errorf("expected id 2, got %v", errMsg.Id)
	}
}

// ============================================================================
// TestServer_HandlerReturnsError — handler returns RpcErrorResponse
// ============================================================================

func TestServer_HandlerReturnsError(t *testing.T) {
	server, framer := newServerWithFramer()

	server.OnRequest("fail", func(id any, params *string) (*string, *rpc.RpcErrorResponse[string]) {
		return nil, &rpc.RpcErrorResponse[string]{
			Code:    rpc.InvalidParams,
			Message: "bad params",
		}
	})

	p := "x"
	encodeFrame(t, framer.in, &rpc.RpcRequest[string]{Id: 3, Method: "fail", Params: &p})
	go server.Serve()

	resp := decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)

	errMsg, ok := resp.(*rpc.RpcErrorResponse[string])
	if !ok {
		t.Fatalf("expected *RpcErrorResponse[string], got %T", resp)
	}
	if errMsg.Code != rpc.InvalidParams {
		t.Errorf("expected code %d, got %d", rpc.InvalidParams, errMsg.Code)
	}
	if errMsg.Id != 3 {
		t.Errorf("expected id 3, got %v", errMsg.Id)
	}
}

// ============================================================================
// TestServer_PanicRecovery — panicking handler → -32603 InternalError
// ============================================================================

func TestServer_PanicRecovery(t *testing.T) {
	server, framer := newServerWithFramer()

	server.OnRequest("bomb", func(id any, params *string) (*string, *rpc.RpcErrorResponse[string]) {
		panic("oops: something went wrong")
	})

	p := "trigger"
	encodeFrame(t, framer.in, &rpc.RpcRequest[string]{Id: 4, Method: "bomb", Params: &p})
	go server.Serve()

	resp := decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)

	errMsg, ok := resp.(*rpc.RpcErrorResponse[string])
	if !ok {
		t.Fatalf("expected *RpcErrorResponse[string], got %T", resp)
	}
	if errMsg.Code != rpc.InternalError {
		t.Errorf("expected InternalError (%d), got %d", rpc.InternalError, errMsg.Code)
	}
	if errMsg.Id != 4 {
		t.Errorf("expected id 4, got %v", errMsg.Id)
	}
}

// ============================================================================
// TestServer_NotificationDispatched — notification reaches handler, no response
// ============================================================================

func TestServer_NotificationDispatched(t *testing.T) {
	server, framer := newServerWithFramer()

	received := make(chan string, 1)
	server.OnNotification("event", func(params *string) {
		if params != nil {
			received <- *params
		} else {
			received <- "(nil)"
		}
	})

	p := "payload"
	encodeFrame(t, framer.in, &rpc.RpcNotification[string]{Method: "event", Params: &p})
	go server.Serve()

	select {
	case got := <-received:
		if got != "payload" {
			t.Errorf("expected 'payload', got %q", got)
		}
	case <-makeTimeout(500 * time.Millisecond):
		t.Fatal("timeout waiting for notification handler to be called")
	}

	// Give a moment to ensure no response was written.
	<-makeTimeout(50 * time.Millisecond)
	if framer.out.Len() != 0 {
		t.Errorf("expected no output for notification, but got %d bytes", framer.out.Len())
	}
}

// ============================================================================
// TestServer_UnknownNotification — unknown notification is silently dropped
// ============================================================================

func TestServer_UnknownNotification(t *testing.T) {
	server, framer := newServerWithFramer()
	// No notification handlers registered.

	p := "x"
	encodeFrame(t, framer.in, &rpc.RpcNotification[string]{Method: "unknown.event", Params: &p})
	go server.Serve()

	// Wait a bit, then check that nothing was written to output.
	<-makeTimeout(50 * time.Millisecond)
	if framer.out.Len() != 0 {
		t.Errorf("expected no output for unknown notification, but got %d bytes", framer.out.Len())
	}
}

// ============================================================================
// TestServer_DecodeError — codec decode failure → ParseError with null id
// ============================================================================

func TestServer_DecodeError(t *testing.T) {
	server, framer := newServerWithFramer()

	// Write garbage bytes (not valid JSON) as a frame.
	writeTestFrame(framer.in, []byte("NOT VALID JSON {{{"))
	go server.Serve()

	resp := decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)

	errMsg, ok := resp.(*rpc.RpcErrorResponse[string])
	if !ok {
		t.Fatalf("expected *RpcErrorResponse[string], got %T", resp)
	}
	if errMsg.Code != rpc.ParseError {
		t.Errorf("expected ParseError (%d), got %d", rpc.ParseError, errMsg.Code)
	}
	// id must be nil because we could not determine it from the malformed payload.
	if errMsg.Id != nil {
		t.Errorf("expected nil id for parse error, got %v", errMsg.Id)
	}
}

// ============================================================================
// TestServer_MultipleRequests — server handles sequential requests correctly
// ============================================================================

func TestServer_MultipleRequests(t *testing.T) {
	server, framer := newServerWithFramer()

	server.OnRequest("greet", func(id any, params *string) (*string, *rpc.RpcErrorResponse[string]) {
		result := "Hello, " + *params
		return &result, nil
	})

	// Write two requests back-to-back.
	p1 := "Alice"
	p2 := "Bob"
	encodeFrame(t, framer.in, &rpc.RpcRequest[string]{Id: 10, Method: "greet", Params: &p1})
	encodeFrame(t, framer.in, &rpc.RpcRequest[string]{Id: 11, Method: "greet", Params: &p2})
	go server.Serve()

	// Read both responses.
	resp1 := decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)
	resp2 := decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)

	r1, ok1 := resp1.(*rpc.RpcResponse[string])
	r2, ok2 := resp2.(*rpc.RpcResponse[string])
	if !ok1 || !ok2 {
		t.Fatalf("expected two RpcResponse, got %T and %T", resp1, resp2)
	}
	if r1.Id != 10 || r2.Id != 11 {
		t.Errorf("ids mismatch: got %v, %v; want 10, 11", r1.Id, r2.Id)
	}
	if r1.Result == nil || *r1.Result != "Hello, Alice" {
		t.Errorf("resp1 result = %v, want 'Hello, Alice'", r1.Result)
	}
	if r2.Result == nil || *r2.Result != "Hello, Bob" {
		t.Errorf("resp2 result = %v, want 'Hello, Bob'", r2.Result)
	}
}

// ============================================================================
// TestClient_Request_Success — request returns server result
// ============================================================================

func TestClient_Request_Success(t *testing.T) {
	client, framer := newClientWithFramer()

	p := "world"
	// Pre-load the framer's `in` buffer with the response the server would send.
	// The client will write its request to `out`, then read the response from `in`.
	result := "Hello, world"
	encodeFrame(t, framer.in, &rpc.RpcResponse[string]{Id: 1, Result: &result})

	got, errResp, err := client.Request("greet", &p)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if errResp != nil {
		t.Fatalf("unexpected error response: %+v", errResp)
	}
	if got == nil || *got != "Hello, world" {
		t.Errorf("got result %v, want 'Hello, world'", got)
	}

	// Verify the client wrote a request frame with id=1 and method="greet".
	reqMsg := decodeFrame(t, framer.out)
	req, ok := reqMsg.(*rpc.RpcRequest[string])
	if !ok {
		t.Fatalf("expected *RpcRequest[string] in out, got %T", reqMsg)
	}
	if req.Id != 1 {
		t.Errorf("request id = %v, want 1", req.Id)
	}
	if req.Method != "greet" {
		t.Errorf("request method = %q, want 'greet'", req.Method)
	}
}

// ============================================================================
// TestClient_Request_ErrorResponse — server sends error → client returns it
// ============================================================================

func TestClient_Request_ErrorResponse(t *testing.T) {
	client, framer := newClientWithFramer()

	p := "x"
	encodeFrame(t, framer.in, &rpc.RpcErrorResponse[string]{
		Id:      1,
		Code:    rpc.MethodNotFound,
		Message: "not found",
	})

	got, errResp, err := client.Request("missing", &p)
	if err != nil {
		t.Fatalf("unexpected transport error: %v", err)
	}
	if got != nil {
		t.Errorf("expected nil result, got %v", got)
	}
	if errResp == nil {
		t.Fatal("expected errResp, got nil")
	}
	if errResp.Code != rpc.MethodNotFound {
		t.Errorf("errResp.Code = %d, want %d", errResp.Code, rpc.MethodNotFound)
	}
}

// ============================================================================
// TestClient_Request_ConnectionClosed — EOF before response → error
// ============================================================================

func TestClient_Request_ConnectionClosed(t *testing.T) {
	client, framer := newClientWithFramer()

	// Write nothing into `in`: the client will see EOF immediately after its request.
	p := "x"
	_ = framer // framer.in is empty

	_, _, err := client.Request("something", &p)
	if err == nil {
		t.Fatal("expected error on connection closed, got nil")
	}
}

// ============================================================================
// TestClient_Notify — notify writes a notification frame without waiting
// ============================================================================

func TestClient_Notify(t *testing.T) {
	client, framer := newClientWithFramer()

	p := "event-data"
	err := client.Notify("log", &p)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Verify the frame written to `out`.
	msgRaw := decodeFrame(t, framer.out)
	notif, ok := msgRaw.(*rpc.RpcNotification[string])
	if !ok {
		t.Fatalf("expected *RpcNotification[string], got %T", msgRaw)
	}
	if notif.Method != "log" {
		t.Errorf("method = %q, want 'log'", notif.Method)
	}
	if notif.Params == nil || *notif.Params != "event-data" {
		t.Errorf("params = %v, want 'event-data'", notif.Params)
	}
}

// ============================================================================
// TestClient_OnNotification — server-push notification dispatched during wait
// ============================================================================

func TestClient_OnNotification(t *testing.T) {
	client, framer := newClientWithFramer()

	// Register a handler for server-push notifications.
	pushReceived := make(chan string, 1)
	client.OnNotification("push", func(params *string) {
		if params != nil {
			pushReceived <- *params
		}
	})

	// Pre-load: first a server-push notification, then the actual response.
	pushData := "server says hi"
	result := "ok"
	encodeFrame(t, framer.in, &rpc.RpcNotification[string]{Method: "push", Params: &pushData})
	encodeFrame(t, framer.in, &rpc.RpcResponse[string]{Id: 1, Result: &result})

	p := "x"
	go func() {
		got, _, _ := client.Request("something", &p)
		if got == nil || *got != "ok" {
			t.Errorf("expected result 'ok', got %v", got)
		}
	}()

	select {
	case got := <-pushReceived:
		if got != "server says hi" {
			t.Errorf("push notification got %q, want 'server says hi'", got)
		}
	case <-makeTimeout(500 * time.Millisecond):
		t.Fatal("timeout waiting for push notification handler")
	}
}

// ============================================================================
// TestClient_RequestIds_Autoincrement — ids start at 1 and increase
// ============================================================================

func TestClient_RequestIds_Autoincrement(t *testing.T) {
	client, framer := newClientWithFramer()

	// Provide responses for 3 consecutive requests.
	for i := 1; i <= 3; i++ {
		r := fmt.Sprintf("result-%d", i)
		encodeFrame(t, framer.in, &rpc.RpcResponse[string]{Id: i, Result: &r})
	}

	for i := 1; i <= 3; i++ {
		p := fmt.Sprintf("param-%d", i)
		_, _, err := client.Request("noop", &p)
		if err != nil {
			t.Fatalf("request %d failed: %v", i, err)
		}
	}

	// Check that the outbound requests had ids 1, 2, 3.
	for i := 1; i <= 3; i++ {
		reqMsg := decodeFrame(t, framer.out)
		req, ok := reqMsg.(*rpc.RpcRequest[string])
		if !ok {
			t.Fatalf("request %d: expected *RpcRequest, got %T", i, reqMsg)
		}
		if req.Id != i {
			t.Errorf("request %d: id = %v, want %d", i, req.Id, i)
		}
	}
}

// ============================================================================
// TestServer_Chaining — OnRequest / OnNotification return server for chaining
// ============================================================================

func TestServer_Chaining(t *testing.T) {
	server, _ := newServerWithFramer()

	// Verify that chaining works without panicking and returns the same server.
	result := server.
		OnRequest("a", func(id any, params *string) (*string, *rpc.RpcErrorResponse[string]) { return nil, nil }).
		OnRequest("b", func(id any, params *string) (*string, *rpc.RpcErrorResponse[string]) { return nil, nil }).
		OnNotification("c", func(params *string) {})

	if result != server {
		t.Error("OnRequest/OnNotification chaining did not return the same server")
	}
}

// ============================================================================
// TestClient_Chaining — OnNotification returns client for chaining
// ============================================================================

func TestClient_Chaining(t *testing.T) {
	client, _ := newClientWithFramer()

	result := client.
		OnNotification("x", func(params *string) {}).
		OnNotification("y", func(params *string) {})

	if result != client {
		t.Error("OnNotification chaining did not return the same client")
	}
}

// ============================================================================
// TestServer_NilParams — handler called with nil params when none provided
// ============================================================================

func TestServer_NilParams(t *testing.T) {
	server, framer := newServerWithFramer()

	var gotParams *string
	server.OnRequest("no-params", func(id any, params *string) (*string, *rpc.RpcErrorResponse[string]) {
		gotParams = params
		r := "ok"
		return &r, nil
	})

	// Request with no params field.
	encodeFrame(t, framer.in, &rpc.RpcRequest[string]{Id: 5, Method: "no-params", Params: nil})
	go server.Serve()

	decodeResponseWithRetry(t, framer.out, 500*time.Millisecond)
	if gotParams != nil {
		t.Errorf("expected nil params, got %v", gotParams)
	}
}

// ============================================================================
// TestEOFIsIo — rpc.EOF == io.EOF (exported convenience var)
// ============================================================================

func TestEOFIsIo(t *testing.T) {
	if rpc.EOF != io.EOF {
		t.Error("rpc.EOF should equal io.EOF")
	}
}

// ============================================================================
// Test helpers
// ============================================================================

// decodeResponseWithRetry polls buf up to `maxWait` duration waiting for a
// frame to arrive, then decodes and returns it. This lets tests run against
// a goroutine-based server without relying on fixed sleeps.
func decodeResponseWithRetry(t *testing.T, buf *bytes.Buffer, maxWait time.Duration) rpc.RpcMessage[string] {
	t.Helper()
	deadline := time.Now().Add(maxWait)
	for time.Now().Before(deadline) {
		if buf.Len() >= 4 {
			return decodeFrame(t, buf)
		}
		time.Sleep(2 * time.Millisecond)
	}
	t.Fatalf("decodeResponseWithRetry: no frame after %v", maxWait)
	return nil
}

// makeTimeout returns a channel that closes after d duration.
func makeTimeout(d time.Duration) <-chan time.Time {
	return time.After(d)
}
