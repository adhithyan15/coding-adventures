package jsonrpc

// message.go — JSON-RPC 2.0 Message Types
//
// This file defines the four message shapes used in JSON-RPC 2.0 communication,
// plus helper functions to convert between raw JSON maps and typed structs.
//
// # The Four Message Types
//
// Think of a JSON-RPC session like a restaurant:
//
//   - Request — the customer places an order (has an "id" so the waiter can
//     bring it back to the right table, and a "method" naming the dish).
//   - Response — the kitchen sends the food back ("id" matches the order,
//     "result" is the food or "error" is what went wrong).
//   - Notification — a broadcast announcement ("kitchen closing in 5 min").
//     No "id", no reply expected.
//   - ResponseError — the error envelope inside a failed Response (declared in errors.go).
//
// # Discriminating Message Types
//
// The JSON-RPC wire format has no explicit "type" discriminator. Instead:
//
//	Has "result" or "error"  →  Response
//	Has "id" (and "method")  →  Request
//	Has only "method"        →  Notification
//
// # Wire Format Examples
//
// Request:
//
//	{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{"line":5}}
//
// Response (success):
//
//	{"jsonrpc":"2.0","id":1,"result":{"contents":"**INC**"}}
//
// Response (error):
//
//	{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
//
// Notification:
//
//	{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"file:///a.bf"}}

import (
	"encoding/json"
	"fmt"
)

// ============================================================================
// Message interface
// ============================================================================

// Message is the discriminated union of the three inbound message types.
// (ResponseError is not a top-level message — it lives inside Response.)
//
// All three concrete types implement this interface via the private
// isMessage() marker method, which prevents external types from accidentally
// satisfying the interface.
type Message interface {
	isMessage()
}

// ============================================================================
// Request
// ============================================================================

// Request is a JSON-RPC 2.0 request from client to server expecting a response.
//
// The Id ties the response back to this request. The server must reply with
// a Response carrying the same Id.
//
// Fields:
//   - Id: unique identifier for this in-flight request (string or integer).
//   - Method: the procedure name (e.g. "textDocument/hover").
//   - Params: optional arguments — any JSON-representable value.
//
// Wire form:
//
//	{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
type Request struct {
	Id     interface{}
	Method string
	Params interface{}
}

func (r *Request) isMessage() {}

// ============================================================================
// Response
// ============================================================================

// Response is a JSON-RPC 2.0 response sent by the server after handling a Request.
//
// Exactly one of Result or Error must be non-nil (never both). If the handler
// succeeded, set Result to any JSON-representable value. If it failed, set
// Error to a *ResponseError.
//
// The Id must match the originating Request's Id. It is nil only when the
// original request was so malformed that its Id could not be recovered.
//
// Wire form (success):
//
//	{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}
//
// Wire form (error):
//
//	{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
type Response struct {
	Id     interface{}
	Result interface{}
	Error  *ResponseError
}

func (r *Response) isMessage() {}

// ============================================================================
// Notification
// ============================================================================

// Notification is a JSON-RPC 2.0 one-way message with no expected response.
//
// Used for events the client fires without waiting for an answer (e.g.,
// "textDocument/didChange"). The server must not send a response to a
// Notification — not even an error response for unknown methods.
//
// Wire form:
//
//	{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"file:///a.bf"}}
type Notification struct {
	Method string
	Params interface{}
}

func (n *Notification) isMessage() {}

// ============================================================================
// ParseMessage — raw JSON map → typed Message
// ============================================================================

// ParseMessage converts a raw JSON string into a typed Message.
//
// It applies two layers of validation:
//
//  1. JSON parsing — if raw is not valid JSON, returns an error with ParseError code.
//  2. Schema validation — if the JSON is not a valid JSON-RPC shape, returns an
//     error with InvalidRequest code.
//
// The return value is one of *Request, *Response, or *Notification.
// The caller can use a type switch to handle each case.
//
// Example:
//
//	msg, err := jsonrpc.ParseMessage(`{"jsonrpc":"2.0","id":1,"method":"foo"}`)
//	switch m := msg.(type) {
//	case *jsonrpc.Request:
//	    fmt.Println("Request:", m.Method)
//	case *jsonrpc.Notification:
//	    fmt.Println("Notification:", m.Method)
//	case *jsonrpc.Response:
//	    fmt.Println("Response for id:", m.Id)
//	}
func ParseMessage(raw string) (Message, error) {
	// Step 1: Parse JSON into a generic map. Any error here means the bytes
	// were not valid UTF-8 JSON.
	var obj map[string]interface{}
	if err := json.Unmarshal([]byte(raw), &obj); err != nil {
		return nil, &ResponseError{
			Code:    ParseError,
			Message: fmt.Sprintf("Parse error: %s", err.Error()),
		}
	}

	// Step 2: The "jsonrpc" field must be exactly "2.0".
	// This guards against accidentally connecting a JSON-RPC 1.0 client.
	v, ok := obj["jsonrpc"]
	if !ok {
		return nil, &ResponseError{
			Code:    InvalidRequest,
			Message: `Invalid Request: missing "jsonrpc" field`,
		}
	}
	if v != "2.0" {
		return nil, &ResponseError{
			Code:    InvalidRequest,
			Message: `Invalid Request: "jsonrpc" must be "2.0"`,
		}
	}

	// Step 3: Discriminate on key presence.
	// Responses have "result" or "error". Requests/Notifications have "method".

	_, hasResult := obj["result"]
	_, hasError := obj["error"]

	if hasResult || hasError {
		return parseResponse(obj)
	}

	return parseRequestOrNotification(obj)
}

// parseResponse builds a *Response from a JSON object that contains "result" or "error".
func parseResponse(obj map[string]interface{}) (*Response, error) {
	id := normalizeID(obj["id"]) // normalise float64→int, matching parseRequestOrNotification

	resp := &Response{Id: id}

	// Parse the error object if present.
	if errObj, ok := obj["error"]; ok && errObj != nil {
		errMap, isMap := errObj.(map[string]interface{})
		if !isMap {
			return nil, &ResponseError{
				Code:    InvalidRequest,
				Message: `Invalid Request: "error" must be a JSON object`,
			}
		}
		code, hasCode := errMap["code"]
		if !hasCode {
			return nil, &ResponseError{
				Code:    InvalidRequest,
				Message: `Invalid Request: error object missing "code" field`,
			}
		}
		// JSON numbers unmarshal as float64 in Go's generic interface{}.
		codeFloat, isFloat := code.(float64)
		if !isFloat {
			return nil, &ResponseError{
				Code:    InvalidRequest,
				Message: `Invalid Request: error "code" must be an integer`,
			}
		}
		msg, _ := errMap["message"].(string)
		resp.Error = &ResponseError{
			Code:    int(codeFloat),
			Message: msg,
			Data:    errMap["data"],
		}
	} else {
		// Success response: result may be nil/null.
		resp.Result = obj["result"]
	}

	return resp, nil
}

// parseRequestOrNotification builds a *Request or *Notification from an object with "method".
func parseRequestOrNotification(obj map[string]interface{}) (Message, error) {
	methodVal, ok := obj["method"]
	if !ok {
		return nil, &ResponseError{
			Code:    InvalidRequest,
			Message: `Invalid Request: missing "method" field`,
		}
	}
	method, isStr := methodVal.(string)
	if !isStr {
		return nil, &ResponseError{
			Code:    InvalidRequest,
			Message: `Invalid Request: "method" must be a string`,
		}
	}

	params := obj["params"] // nil if absent

	if id, hasID := obj["id"]; hasID {
		// A message with "id" is a Request. The id must be string or number (not null).
		// JSON numbers arrive as float64.
		switch id.(type) {
		case string, float64:
			// valid — convert float64 id to int if it is a whole number
			finalID := normalizeID(id)
			return &Request{Id: finalID, Method: method, Params: params}, nil
		default:
			return nil, &ResponseError{
				Code:    InvalidRequest,
				Message: `Invalid Request: "id" must be a string or integer`,
			}
		}
	}

	// No "id" → Notification.
	return &Notification{Method: method, Params: params}, nil
}

// normalizeID converts a JSON-decoded id to int if it is a whole float64,
// otherwise returns it unchanged (string). This makes id==1 comparable to
// the integer 1 rather than the float 1.0.
func normalizeID(id interface{}) interface{} {
	if f, ok := id.(float64); ok {
		return int(f)
	}
	return id
}

// ============================================================================
// MessageToMap — typed Message → map ready for json.Marshal
// ============================================================================

// MessageToMap serializes a Message to a plain map[string]interface{} suitable
// for json.Marshal. This is the inverse of ParseMessage.
//
// The "jsonrpc":"2.0" marker is always included.
//
// Examples:
//
//	MessageToMap(&Request{Id: 1, Method: "foo"})
//	// → map["jsonrpc":"2.0", "id":1, "method":"foo"]
//
//	MessageToMap(&Response{Id: 1, Result: 42})
//	// → map["jsonrpc":"2.0", "id":1, "result":42]
//
//	MessageToMap(&Notification{Method: "bar"})
//	// → map["jsonrpc":"2.0", "method":"bar"]
func MessageToMap(msg Message) (map[string]interface{}, error) {
	switch m := msg.(type) {
	case *Request:
		d := map[string]interface{}{
			"jsonrpc": "2.0",
			"id":      m.Id,
			"method":  m.Method,
		}
		if m.Params != nil {
			d["params"] = m.Params
		}
		return d, nil

	case *Response:
		d := map[string]interface{}{
			"jsonrpc": "2.0",
			"id":      m.Id,
		}
		if m.Error != nil {
			// Error response: include error object, omit result.
			errObj := map[string]interface{}{
				"code":    m.Error.Code,
				"message": m.Error.Message,
			}
			if m.Error.Data != nil {
				errObj["data"] = m.Error.Data
			}
			d["error"] = errObj
		} else {
			// Success response: include result (may be nil/null).
			d["result"] = m.Result
		}
		return d, nil

	case *Notification:
		d := map[string]interface{}{
			"jsonrpc": "2.0",
			"method":  m.Method,
		}
		if m.Params != nil {
			d["params"] = m.Params
		}
		return d, nil

	default:
		return nil, fmt.Errorf("unknown message type: %T", msg)
	}
}
