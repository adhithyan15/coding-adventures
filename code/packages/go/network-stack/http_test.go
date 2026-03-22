package networkstack

import (
	"strings"
	"testing"
)

func TestHTTPRequestSerializeGET(t *testing.T) {
	req := &HTTPRequest{
		Method:  "GET",
		Path:    "/index.html",
		Headers: map[string]string{"Host": "example.com"},
	}
	raw := string(req.Serialize())
	if !strings.HasPrefix(raw, "GET /index.html HTTP/1.1\r\n") {
		t.Errorf("bad request line: %q", raw)
	}
	if !strings.Contains(raw, "Host: example.com\r\n") {
		t.Errorf("missing Host header")
	}
	if !strings.HasSuffix(raw, "\r\n\r\n") {
		t.Errorf("should end with blank line")
	}
}

func TestHTTPRequestSerializePOST(t *testing.T) {
	req := &HTTPRequest{
		Method:  "POST",
		Path:    "/api",
		Headers: map[string]string{"Host": "api.local"},
		Body:    []byte(`{"key":"value"}`),
	}
	raw := string(req.Serialize())
	if !strings.Contains(raw, "Content-Length: 15\r\n") {
		t.Errorf("missing Content-Length")
	}
	if !strings.HasSuffix(raw, `{"key":"value"}`) {
		t.Errorf("missing body")
	}
}

func TestHTTPResponseSerialize(t *testing.T) {
	resp := &HTTPResponse{
		StatusCode: 200,
		StatusText: "OK",
		Headers:    map[string]string{"Content-Type": "text/html"},
		Body:       []byte("<h1>Hello</h1>"),
	}
	raw := string(resp.Serialize())
	if !strings.HasPrefix(raw, "HTTP/1.1 200 OK\r\n") {
		t.Errorf("bad status line")
	}
	if !strings.Contains(raw, "Content-Type: text/html") {
		t.Errorf("missing Content-Type")
	}
}

func TestHTTPResponseDeserialize(t *testing.T) {
	raw := []byte("HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nPage not found")
	resp, err := DeserializeHTTPResponse(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.StatusCode != 404 {
		t.Errorf("status: got %d", resp.StatusCode)
	}
	if resp.StatusText != "Not Found" {
		t.Errorf("status text: got %q", resp.StatusText)
	}
	if resp.Headers["Content-Type"] != "text/plain" {
		t.Errorf("header mismatch")
	}
	if string(resp.Body) != "Page not found" {
		t.Errorf("body: got %q", string(resp.Body))
	}
}

func TestHTTPResponseDeserializeNoBody(t *testing.T) {
	raw := []byte("HTTP/1.1 204 No Content\r\n\r\n")
	resp, err := DeserializeHTTPResponse(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.StatusCode != 204 {
		t.Errorf("status: got %d", resp.StatusCode)
	}
	if len(resp.Body) != 0 {
		t.Errorf("expected empty body")
	}
}

func TestHTTPResponseRoundtrip(t *testing.T) {
	original := &HTTPResponse{
		StatusCode: 200,
		StatusText: "OK",
		Headers:    map[string]string{"Server": "test"},
		Body:       []byte("hello"),
	}
	raw := original.Serialize()
	recovered, err := DeserializeHTTPResponse(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if recovered.StatusCode != 200 {
		t.Errorf("status mismatch")
	}
	if string(recovered.Body) != "hello" {
		t.Errorf("body mismatch")
	}
}

func TestHTTPResponseDeserializeEmpty(t *testing.T) {
	_, err := DeserializeHTTPResponse([]byte(""))
	if err == nil {
		t.Error("expected error for empty input")
	}
}

func TestHTTPClientBuildRequest(t *testing.T) {
	dns := NewDNSResolver()
	dns.AddStatic("example.com", 0x5DB8D822)
	client := NewHTTPClient(dns)

	host, port, req, err := client.BuildRequest("GET", "http://example.com/page", nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if host != "example.com" {
		t.Errorf("host: got %q", host)
	}
	if port != 80 {
		t.Errorf("port: got %d", port)
	}
	if req.Path != "/page" {
		t.Errorf("path: got %q", req.Path)
	}
	if req.Headers["Host"] != "example.com" {
		t.Errorf("Host header missing")
	}
}

func TestHTTPClientBuildRequestWithPort(t *testing.T) {
	dns := NewDNSResolver()
	client := NewHTTPClient(dns)

	host, port, req, _ := client.BuildRequest("GET", "http://localhost:8080/api", nil)
	if host != "localhost" {
		t.Errorf("host: got %q", host)
	}
	if port != 8080 {
		t.Errorf("port: got %d", port)
	}
	if req.Path != "/api" {
		t.Errorf("path: got %q", req.Path)
	}
}

func TestHTTPClientBuildRequestNoPath(t *testing.T) {
	dns := NewDNSResolver()
	client := NewHTTPClient(dns)

	_, _, req, _ := client.BuildRequest("GET", "http://example.com", nil)
	if req.Path != "/" {
		t.Errorf("path should default to /")
	}
}

func TestHTTPClientParseResponse(t *testing.T) {
	dns := NewDNSResolver()
	client := NewHTTPClient(dns)

	raw := []byte("HTTP/1.1 200 OK\r\n\r\nbody")
	resp, err := client.ParseResponse(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp.StatusCode != 200 {
		t.Errorf("status: got %d", resp.StatusCode)
	}
}

func TestHTTPClientHTTPSStripped(t *testing.T) {
	dns := NewDNSResolver()
	client := NewHTTPClient(dns)

	host, _, req, _ := client.BuildRequest("GET", "https://secure.com/path", nil)
	if host != "secure.com" {
		t.Errorf("host: got %q", host)
	}
	if req.Path != "/path" {
		t.Errorf("path: got %q", req.Path)
	}
}

func TestHTTPResponseAutoContentLength(t *testing.T) {
	resp := &HTTPResponse{
		StatusCode: 200, StatusText: "OK",
		Headers: make(map[string]string),
		Body:    []byte("test"),
	}
	raw := string(resp.Serialize())
	if !strings.Contains(raw, "Content-Length: 4") {
		t.Errorf("missing auto Content-Length")
	}
}
