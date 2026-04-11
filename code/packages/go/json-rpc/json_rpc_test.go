package jsonrpc_test

// json_rpc_test.go — Comprehensive tests for the jsonrpc package.
//
// Test Strategy
// -------------
//
// Tests are organized into five groups:
//
//  1. ParseMessage / MessageToMap — round-trips and error cases.
//  2. MessageReader — framing, back-to-back messages, EOF, error paths.
//  3. MessageWriter — correct Content-Length, UTF-8 payload, CRLF separator.
//  4. Server — dispatch, unknown method, handler errors, notifications.
//  5. Round-trip — write then read back through a shared bytes.Buffer.
//
// Each test name follows the pattern Test<Component>_<Scenario>.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"strings"
	"testing"

	jsonrpc "github.com/coding-adventures/json-rpc"
)

// ===========================================================================
// Helpers
// ===========================================================================

// makeFramed builds a Content-Length-framed message from a JSON string.
func makeFramed(payload string) []byte {
	encoded := []byte(payload)
	header := fmt.Sprintf("Content-Length: %d\r\n\r\n", len(encoded))
	return append([]byte(header), encoded...)
}

// makeReader builds a MessageReader backed by a single framed payload string.
func makeReader(payload string) *jsonrpc.MessageReader {
	return jsonrpc.NewReader(bytes.NewReader(makeFramed(payload)))
}

// readResponse reads the first framed JSON-RPC response from a bytes.Buffer and
// returns it as a raw map decoded directly from JSON. We decode via the raw JSON
// bytes (not via MessageToMap) so that all values carry JSON-native types:
// numbers arrive as float64, strings as string, etc. This keeps type assertions
// in tests predictable and consistent with what a real JSON-RPC client would see.
func readResponse(buf *bytes.Buffer) map[string]interface{} {
	reader := jsonrpc.NewReader(bytes.NewReader(buf.Bytes()))
	raw, err := reader.ReadRaw()
	if err != nil {
		panic(fmt.Sprintf("readResponse ReadRaw: %v", err))
	}
	var d map[string]interface{}
	if err := json.Unmarshal([]byte(raw), &d); err != nil {
		panic(fmt.Sprintf("readResponse Unmarshal: %v", err))
	}
	return d
}

// ===========================================================================
// 1. ParseMessage / MessageToMap
// ===========================================================================

func TestParseMessage_Request(t *testing.T) {
	raw := `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"x":1}}`
	msg, err := jsonrpc.ParseMessage(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	req, ok := msg.(*jsonrpc.Request)
	if !ok {
		t.Fatalf("expected *Request, got %T", msg)
	}
	if req.Id != 1 {
		t.Errorf("id: got %v, want 1", req.Id)
	}
	if req.Method != "initialize" {
		t.Errorf("method: got %q, want %q", req.Method, "initialize")
	}
	params, _ := req.Params.(map[string]interface{})
	if params["x"] != float64(1) {
		t.Errorf("params.x: got %v, want 1", params["x"])
	}
}

func TestParseMessage_RequestStringID(t *testing.T) {
	raw := `{"jsonrpc":"2.0","id":"abc","method":"shutdown"}`
	msg, err := jsonrpc.ParseMessage(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	req, ok := msg.(*jsonrpc.Request)
	if !ok {
		t.Fatalf("expected *Request, got %T", msg)
	}
	if req.Id != "abc" {
		t.Errorf("id: got %v, want abc", req.Id)
	}
}

func TestParseMessage_Notification(t *testing.T) {
	raw := `{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"file:///a.bf"}}`
	msg, err := jsonrpc.ParseMessage(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	notif, ok := msg.(*jsonrpc.Notification)
	if !ok {
		t.Fatalf("expected *Notification, got %T", msg)
	}
	if notif.Method != "textDocument/didOpen" {
		t.Errorf("method: got %q", notif.Method)
	}
}

func TestParseMessage_NotificationNoParams(t *testing.T) {
	raw := `{"jsonrpc":"2.0","method":"exit"}`
	msg, err := jsonrpc.ParseMessage(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	notif, ok := msg.(*jsonrpc.Notification)
	if !ok {
		t.Fatalf("expected *Notification, got %T", msg)
	}
	if notif.Params != nil {
		t.Errorf("params should be nil, got %v", notif.Params)
	}
}

func TestParseMessage_ResponseSuccess(t *testing.T) {
	raw := `{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}`
	msg, err := jsonrpc.ParseMessage(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	resp, ok := msg.(*jsonrpc.Response)
	if !ok {
		t.Fatalf("expected *Response, got %T", msg)
	}
	if resp.Id != 1 {
		t.Errorf("id: got %v, want 1", resp.Id)
	}
	if resp.Error != nil {
		t.Errorf("error should be nil")
	}
}

func TestParseMessage_ResponseError(t *testing.T) {
	raw := `{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}`
	msg, err := jsonrpc.ParseMessage(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	resp, ok := msg.(*jsonrpc.Response)
	if !ok {
		t.Fatalf("expected *Response, got %T", msg)
	}
	if resp.Error == nil {
		t.Fatal("error should not be nil")
	}
	if resp.Error.Code != -32601 {
		t.Errorf("error code: got %d, want -32601", resp.Error.Code)
	}
}

func TestParseMessage_InvalidJSON(t *testing.T) {
	_, err := jsonrpc.ParseMessage("{not valid json}")
	if err == nil {
		t.Fatal("expected error for invalid JSON")
	}
	respErr, ok := err.(*jsonrpc.ResponseError)
	if !ok {
		t.Fatalf("expected *ResponseError, got %T", err)
	}
	if respErr.Code != jsonrpc.ParseError {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.ParseError)
	}
}

func TestParseMessage_MissingJsonrpcField(t *testing.T) {
	_, err := jsonrpc.ParseMessage(`{"id":1,"method":"foo"}`)
	if err == nil {
		t.Fatal("expected error for missing jsonrpc field")
	}
	respErr, _ := err.(*jsonrpc.ResponseError)
	if respErr.Code != jsonrpc.InvalidRequest {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.InvalidRequest)
	}
}

func TestParseMessage_WrongJsonrpcVersion(t *testing.T) {
	_, err := jsonrpc.ParseMessage(`{"jsonrpc":"1.0","id":1,"method":"foo"}`)
	if err == nil {
		t.Fatal("expected error for wrong jsonrpc version")
	}
	respErr, _ := err.(*jsonrpc.ResponseError)
	if respErr.Code != jsonrpc.InvalidRequest {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.InvalidRequest)
	}
}

func TestParseMessage_MissingMethod(t *testing.T) {
	_, err := jsonrpc.ParseMessage(`{"jsonrpc":"2.0","id":1}`)
	if err == nil {
		t.Fatal("expected error for missing method")
	}
	respErr, _ := err.(*jsonrpc.ResponseError)
	if respErr.Code != jsonrpc.InvalidRequest {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.InvalidRequest)
	}
}

func TestParseMessage_NullIDRaisesInvalidRequest(t *testing.T) {
	// "id": null in a Request is not valid per spec.
	_, err := jsonrpc.ParseMessage(`{"jsonrpc":"2.0","id":null,"method":"foo"}`)
	if err == nil {
		t.Fatal("expected error for null id in request")
	}
	respErr, _ := err.(*jsonrpc.ResponseError)
	if respErr.Code != jsonrpc.InvalidRequest {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.InvalidRequest)
	}
}

func TestMessageToMap_Request(t *testing.T) {
	msg := &jsonrpc.Request{Id: 1, Method: "foo", Params: map[string]interface{}{"a": 1}}
	d, err := jsonrpc.MessageToMap(msg)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if d["jsonrpc"] != "2.0" {
		t.Errorf("jsonrpc: got %v", d["jsonrpc"])
	}
	if d["id"] != 1 {
		t.Errorf("id: got %v", d["id"])
	}
	if d["method"] != "foo" {
		t.Errorf("method: got %v", d["method"])
	}
}

func TestMessageToMap_RequestNoParams(t *testing.T) {
	msg := &jsonrpc.Request{Id: 2, Method: "bar"}
	d, err := jsonrpc.MessageToMap(msg)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if _, hasParams := d["params"]; hasParams {
		t.Error("params should not be present when nil")
	}
}

func TestMessageToMap_Notification(t *testing.T) {
	msg := &jsonrpc.Notification{Method: "event"}
	d, err := jsonrpc.MessageToMap(msg)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if _, hasID := d["id"]; hasID {
		t.Error("notification should not have id")
	}
}

func TestMessageToMap_ErrorResponse(t *testing.T) {
	err := &jsonrpc.ResponseError{Code: -32601, Message: "Not found"}
	msg := &jsonrpc.Response{Id: 1, Error: err}
	d, mapErr := jsonrpc.MessageToMap(msg)
	if mapErr != nil {
		t.Fatalf("unexpected error: %v", mapErr)
	}
	if _, hasResult := d["result"]; hasResult {
		t.Error("error response should not have result key")
	}
	errObj := d["error"].(map[string]interface{})
	if errObj["code"] != -32601 {
		t.Errorf("error code: got %v", errObj["code"])
	}
}

// ===========================================================================
// 2. MessageReader
// ===========================================================================

func TestMessageReader_SingleRequest(t *testing.T) {
	raw := `{"jsonrpc":"2.0","id":1,"method":"foo"}`
	reader := makeReader(raw)
	msg, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	req, ok := msg.(*jsonrpc.Request)
	if !ok {
		t.Fatalf("expected *Request, got %T", msg)
	}
	if req.Method != "foo" {
		t.Errorf("method: got %q", req.Method)
	}
}

func TestMessageReader_EOFReturnsEOF(t *testing.T) {
	reader := jsonrpc.NewReader(strings.NewReader(""))
	_, err := reader.ReadMessage()
	if err != io.EOF {
		t.Errorf("expected io.EOF, got %v", err)
	}
}

func TestMessageReader_ReadRawReturnsNilOnEOF(t *testing.T) {
	reader := jsonrpc.NewReader(strings.NewReader(""))
	raw, err := reader.ReadRaw()
	if err != io.EOF {
		t.Errorf("expected io.EOF, got %v", err)
	}
	if raw != "" {
		t.Errorf("expected empty string on EOF, got %q", raw)
	}
}

func TestMessageReader_BackToBackMessages(t *testing.T) {
	msg1 := `{"jsonrpc":"2.0","id":1,"method":"foo"}`
	msg2 := `{"jsonrpc":"2.0","method":"bar"}`
	data := append(makeFramed(msg1), makeFramed(msg2)...)
	reader := jsonrpc.NewReader(bytes.NewReader(data))

	first, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("first read: %v", err)
	}
	if _, ok := first.(*jsonrpc.Request); !ok {
		t.Errorf("first: expected *Request, got %T", first)
	}

	second, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("second read: %v", err)
	}
	if _, ok := second.(*jsonrpc.Notification); !ok {
		t.Errorf("second: expected *Notification, got %T", second)
	}

	_, err = reader.ReadMessage()
	if err != io.EOF {
		t.Errorf("third read: expected io.EOF, got %v", err)
	}
}

func TestMessageReader_MalformedJSONReturnsParseError(t *testing.T) {
	payload := "not-json!!"
	data := makeFramed(payload)
	reader := jsonrpc.NewReader(bytes.NewReader(data))
	_, err := reader.ReadMessage()
	if err == nil {
		t.Fatal("expected error for malformed JSON")
	}
	respErr, ok := err.(*jsonrpc.ResponseError)
	if !ok {
		t.Fatalf("expected *ResponseError, got %T", err)
	}
	if respErr.Code != jsonrpc.ParseError {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.ParseError)
	}
}

func TestMessageReader_ValidJSONNotAMessageReturnsInvalidRequest(t *testing.T) {
	raw := `"just a string"`
	reader := makeReader(raw)
	_, err := reader.ReadMessage()
	if err == nil {
		t.Fatal("expected error")
	}
	respErr, _ := err.(*jsonrpc.ResponseError)
	if respErr.Code != jsonrpc.InvalidRequest {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.InvalidRequest)
	}
}

func TestMessageReader_MissingContentLengthReturnsError(t *testing.T) {
	// A header block with no Content-Length line.
	data := []byte("Content-Type: text/plain\r\n\r\nhello")
	reader := jsonrpc.NewReader(bytes.NewReader(data))
	_, err := reader.ReadMessage()
	if err == nil {
		t.Fatal("expected error for missing Content-Length")
	}
	respErr, _ := err.(*jsonrpc.ResponseError)
	if respErr.Code != jsonrpc.ParseError {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.ParseError)
	}
}

func TestMessageReader_TruncatedPayloadReturnsError(t *testing.T) {
	// Content-Length says 100 but only 5 bytes follow.
	header := []byte("Content-Length: 100\r\n\r\nhello")
	reader := jsonrpc.NewReader(bytes.NewReader(header))
	_, err := reader.ReadMessage()
	if err == nil {
		t.Fatal("expected error for truncated payload")
	}
	respErr, _ := err.(*jsonrpc.ResponseError)
	if respErr.Code != jsonrpc.ParseError {
		t.Errorf("code: got %d, want %d", respErr.Code, jsonrpc.ParseError)
	}
}

func TestMessageReader_IgnoresContentTypeHeader(t *testing.T) {
	// A valid message with an extra Content-Type header should be accepted.
	payload := []byte(`{"jsonrpc":"2.0","method":"ping"}`)
	header := fmt.Sprintf(
		"Content-Length: %d\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n",
		len(payload),
	)
	data := append([]byte(header), payload...)
	reader := jsonrpc.NewReader(bytes.NewReader(data))
	msg, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if _, ok := msg.(*jsonrpc.Notification); !ok {
		t.Errorf("expected *Notification, got %T", msg)
	}
}

func TestMessageReader_UnicodePayload(t *testing.T) {
	// Japanese characters are multi-byte in UTF-8.
	raw := `{"jsonrpc":"2.0","method":"test","params":{"msg":"日本語"}}`
	reader := makeReader(raw)
	msg, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	notif, ok := msg.(*jsonrpc.Notification)
	if !ok {
		t.Fatalf("expected *Notification, got %T", msg)
	}
	params := notif.Params.(map[string]interface{})
	if params["msg"] != "日本語" {
		t.Errorf("params.msg: got %v", params["msg"])
	}
}

// ===========================================================================
// 3. MessageWriter
// ===========================================================================

func TestMessageWriter_CorrectContentLength(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	msg := &jsonrpc.Request{Id: 1, Method: "foo"}
	if err := writer.WriteMessage(msg); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	data := buf.Bytes()
	headerEnd := bytes.Index(data, []byte("\r\n\r\n"))
	if headerEnd == -1 {
		t.Fatal("no CRLF separator found")
	}
	headerBlock := string(data[:headerEnd])
	payload := data[headerEnd+4:]

	var declaredLength int
	for _, line := range strings.Split(headerBlock, "\r\n") {
		if strings.HasPrefix(strings.ToLower(line), "content-length:") {
			fmt.Sscanf(strings.TrimSpace(line[len("content-length:"):]), "%d", &declaredLength)
		}
	}

	if declaredLength != len(payload) {
		t.Errorf("Content-Length %d != actual payload length %d", declaredLength, len(payload))
	}
}

func TestMessageWriter_CRLFSeparator(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	if err := writer.WriteRaw(`{"jsonrpc":"2.0","method":"ping"}`); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !bytes.Contains(buf.Bytes(), []byte("\r\n\r\n")) {
		t.Error("missing CRLF separator between headers and payload")
	}
}

func TestMessageWriter_ValidJSONPayload(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	msg := &jsonrpc.Response{Id: 1, Result: map[string]interface{}{"status": "ok"}}
	if err := writer.WriteMessage(msg); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	data := buf.Bytes()
	headerEnd := bytes.Index(data, []byte("\r\n\r\n"))
	payload := data[headerEnd+4:]

	var parsed map[string]interface{}
	if err := json.Unmarshal(payload, &parsed); err != nil {
		t.Fatalf("payload is not valid JSON: %v", err)
	}
	if parsed["jsonrpc"] != "2.0" {
		t.Errorf("jsonrpc: got %v", parsed["jsonrpc"])
	}
	result := parsed["result"].(map[string]interface{})
	if result["status"] != "ok" {
		t.Errorf("result.status: got %v", result["status"])
	}
}

func TestMessageWriter_UTF8ByteCount(t *testing.T) {
	// The emoji 🎸 is 1 character but 4 bytes in UTF-8.
	// Content-Length must count bytes, not characters.
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	if err := writer.WriteRaw(`{"jsonrpc":"2.0","method":"🎸"}`); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	data := buf.Bytes()
	headerEnd := bytes.Index(data, []byte("\r\n\r\n"))
	headerBlock := string(data[:headerEnd])
	payload := data[headerEnd+4:]

	var declaredLength int
	for _, line := range strings.Split(headerBlock, "\r\n") {
		if strings.HasPrefix(strings.ToLower(line), "content-length:") {
			fmt.Sscanf(strings.TrimSpace(line[len("content-length:"):]), "%d", &declaredLength)
		}
	}

	if declaredLength != len(payload) {
		t.Errorf("Content-Length %d != actual byte length %d", declaredLength, len(payload))
	}
}

// ===========================================================================
// 4. Server
// ===========================================================================

func makeServerWithMessages(payloads ...string) (*jsonrpc.Server, *bytes.Buffer) {
	var data []byte
	for _, p := range payloads {
		data = append(data, makeFramed(p)...)
	}
	in := bytes.NewReader(data)
	out := &bytes.Buffer{}
	server := jsonrpc.NewServer(in, out)
	return server, out
}

func TestServer_DispatchesRequestToHandler(t *testing.T) {
	req := `{"jsonrpc":"2.0","id":1,"method":"add","params":{"a":1,"b":2}}`
	server, out := makeServerWithMessages(req)
	server.OnRequest("add", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
		p := params.(map[string]interface{})
		return p["a"].(float64) + p["b"].(float64), nil
	})
	server.Serve()

	response := readResponse(out)
	if response["id"] != float64(1) {
		t.Errorf("id: got %v", response["id"])
	}
	if response["result"] != float64(3) {
		t.Errorf("result: got %v, want 3", response["result"])
	}
	if _, hasErr := response["error"]; hasErr {
		t.Error("error should not be present")
	}
}

func TestServer_SendsMethodNotFoundForUnknownRequest(t *testing.T) {
	req := `{"jsonrpc":"2.0","id":1,"method":"unknown"}`
	server, out := makeServerWithMessages(req)
	server.Serve()

	response := readResponse(out)
	errObj := response["error"].(map[string]interface{})
	if int(errObj["code"].(float64)) != jsonrpc.MethodNotFound {
		t.Errorf("code: got %v, want %d", errObj["code"], jsonrpc.MethodNotFound)
	}
}

func TestServer_DispatchesNotificationWithoutResponse(t *testing.T) {
	notif := `{"jsonrpc":"2.0","method":"ping"}`
	called := false
	server, out := makeServerWithMessages(notif)
	server.OnNotification("ping", func(params interface{}) {
		called = true
	})
	server.Serve()

	if !called {
		t.Error("notification handler was not called")
	}
	// No response should have been written.
	if out.Len() != 0 {
		t.Errorf("expected no output, got %d bytes", out.Len())
	}
}

func TestServer_IgnoresUnknownNotification(t *testing.T) {
	notif := `{"jsonrpc":"2.0","method":"unknown_event"}`
	server, out := makeServerWithMessages(notif)
	server.Serve()

	if out.Len() != 0 {
		t.Errorf("expected no output for unknown notification, got %d bytes", out.Len())
	}
}

func TestServer_HandlerReturnsResponseError(t *testing.T) {
	req := `{"jsonrpc":"2.0","id":1,"method":"fail"}`
	server, out := makeServerWithMessages(req)
	server.OnRequest("fail", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
		return nil, &jsonrpc.ResponseError{Code: -32602, Message: "Bad params"}
	})
	server.Serve()

	response := readResponse(out)
	errObj := response["error"].(map[string]interface{})
	if int(errObj["code"].(float64)) != -32602 {
		t.Errorf("code: got %v, want -32602", errObj["code"])
	}
	if errObj["message"] != "Bad params" {
		t.Errorf("message: got %v", errObj["message"])
	}
}

func TestServer_ChainingOnRequestOnNotification(t *testing.T) {
	req := `{"jsonrpc":"2.0","id":1,"method":"ping"}`
	server, out := makeServerWithMessages(req)
	server.
		OnRequest("ping", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
			return "pong", nil
		}).
		OnNotification("exit", func(params interface{}) {})
	server.Serve()

	response := readResponse(out)
	if response["result"] != "pong" {
		t.Errorf("result: got %v, want pong", response["result"])
	}
}

func TestServer_MultipleRequestsInSequence(t *testing.T) {
	req1 := `{"jsonrpc":"2.0","id":1,"method":"echo","params":"hello"}`
	req2 := `{"jsonrpc":"2.0","id":2,"method":"echo","params":"world"}`
	server, out := makeServerWithMessages(req1, req2)
	server.OnRequest("echo", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
		return params, nil
	})
	server.Serve()

	reader := jsonrpc.NewReader(bytes.NewReader(out.Bytes()))
	m1, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("read r1: %v", err)
	}
	m2, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("read r2: %v", err)
	}
	r1, _ := m1.(*jsonrpc.Response)
	r2, _ := m2.(*jsonrpc.Response)
	if r1 == nil || r2 == nil {
		t.Fatal("expected two Responses")
	}
	if r1.Result != "hello" {
		t.Errorf("r1.result: got %v", r1.Result)
	}
	if r2.Result != "world" {
		t.Errorf("r2.result: got %v", r2.Result)
	}
}

func TestServer_NullResultIsValid(t *testing.T) {
	req := `{"jsonrpc":"2.0","id":1,"method":"shutdown"}`
	server, out := makeServerWithMessages(req)
	server.OnRequest("shutdown", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
		return nil, nil
	})
	server.Serve()

	response := readResponse(out)
	if response["id"] != float64(1) {
		t.Errorf("id: got %v", response["id"])
	}
	// result key should be present (with null value)
	if _, ok := response["result"]; !ok {
		t.Error("result key should be present even when nil")
	}
}

func TestServer_ParseErrorSendsErrorResponse(t *testing.T) {
	// Craft a framed message where the payload is not valid JSON.
	badPayload := []byte("NOT JSON HERE")
	header := fmt.Sprintf("Content-Length: %d\r\n\r\n", len(badPayload))
	data := append([]byte(header), badPayload...)

	in := bytes.NewReader(data)
	out := &bytes.Buffer{}
	server := jsonrpc.NewServer(in, out)
	server.Serve()

	response := readResponse(out)
	errObj := response["error"].(map[string]interface{})
	if int(errObj["code"].(float64)) != jsonrpc.ParseError {
		t.Errorf("code: got %v, want %d", errObj["code"], jsonrpc.ParseError)
	}
}

// ===========================================================================
// 5. Round-trip tests
// ===========================================================================

func TestRoundTrip_Request(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	original := &jsonrpc.Request{Id: 42, Method: "textDocument/hover", Params: map[string]interface{}{"line": float64(5)}}
	if err := writer.WriteMessage(original); err != nil {
		t.Fatalf("write: %v", err)
	}

	reader := jsonrpc.NewReader(bytes.NewReader(buf.Bytes()))
	recovered, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	req, ok := recovered.(*jsonrpc.Request)
	if !ok {
		t.Fatalf("expected *Request, got %T", recovered)
	}
	if req.Id != 42 {
		t.Errorf("id: got %v, want 42", req.Id)
	}
	if req.Method != "textDocument/hover" {
		t.Errorf("method: got %q", req.Method)
	}
}

func TestRoundTrip_Notification(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	original := &jsonrpc.Notification{Method: "initialized", Params: map[string]interface{}{}}
	if err := writer.WriteMessage(original); err != nil {
		t.Fatalf("write: %v", err)
	}

	reader := jsonrpc.NewReader(bytes.NewReader(buf.Bytes()))
	recovered, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	notif, ok := recovered.(*jsonrpc.Notification)
	if !ok {
		t.Fatalf("expected *Notification, got %T", recovered)
	}
	if notif.Method != "initialized" {
		t.Errorf("method: got %q", notif.Method)
	}
}

func TestRoundTrip_ResponseSuccess(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	original := &jsonrpc.Response{Id: 1, Result: map[string]interface{}{"capabilities": map[string]interface{}{"hoverProvider": true}}}
	if err := writer.WriteMessage(original); err != nil {
		t.Fatalf("write: %v", err)
	}

	reader := jsonrpc.NewReader(bytes.NewReader(buf.Bytes()))
	recovered, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	resp, ok := recovered.(*jsonrpc.Response)
	if !ok {
		t.Fatalf("expected *Response, got %T", recovered)
	}
	if resp.Id != 1 {
		t.Errorf("id: got %v, want 1", resp.Id)
	}
	if resp.Error != nil {
		t.Errorf("error should be nil")
	}
}

func TestRoundTrip_ResponseError(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	respErr := &jsonrpc.ResponseError{Code: -32601, Message: "Method not found", Data: "details"}
	original := &jsonrpc.Response{Id: 3, Error: respErr}
	if err := writer.WriteMessage(original); err != nil {
		t.Fatalf("write: %v", err)
	}

	reader := jsonrpc.NewReader(bytes.NewReader(buf.Bytes()))
	recovered, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	resp, ok := recovered.(*jsonrpc.Response)
	if !ok {
		t.Fatalf("expected *Response, got %T", recovered)
	}
	if resp.Error == nil {
		t.Fatal("error should not be nil")
	}
	if resp.Error.Code != -32601 {
		t.Errorf("error code: got %d", resp.Error.Code)
	}
	if resp.Error.Message != "Method not found" {
		t.Errorf("error message: got %q", resp.Error.Message)
	}
	if resp.Error.Data != "details" {
		t.Errorf("error data: got %v", resp.Error.Data)
	}
}

func TestRoundTrip_UnicodeStrings(t *testing.T) {
	var buf bytes.Buffer
	writer := jsonrpc.NewWriter(&buf)
	original := &jsonrpc.Notification{Method: "test", Params: map[string]interface{}{"text": "日本語 🌸"}}
	if err := writer.WriteMessage(original); err != nil {
		t.Fatalf("write: %v", err)
	}

	reader := jsonrpc.NewReader(bytes.NewReader(buf.Bytes()))
	recovered, err := reader.ReadMessage()
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	notif, ok := recovered.(*jsonrpc.Notification)
	if !ok {
		t.Fatalf("expected *Notification, got %T", recovered)
	}
	params := notif.Params.(map[string]interface{})
	if params["text"] != "日本語 🌸" {
		t.Errorf("params.text: got %v", params["text"])
	}
}
