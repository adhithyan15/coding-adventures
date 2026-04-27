package http1

import (
	"testing"

	httpcore "github.com/adhithyan15/coding-adventures/code/packages/go/http-core"
)

func TestParseRequestHead(t *testing.T) {
	parsed, err := ParseRequestHead([]byte("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n"))
	if err != nil {
		t.Fatalf("ParseRequestHead returned error: %v", err)
	}

	if parsed.Head.Method != "GET" || parsed.Head.Target != "/" {
		t.Fatalf("unexpected request head: %#v", parsed.Head)
	}
	if parsed.BodyKind != httpcore.NoBody() {
		t.Fatalf("unexpected body kind: %#v", parsed.BodyKind)
	}
}

func TestParsePostRequestWithLength(t *testing.T) {
	parsed, err := ParseRequestHead([]byte("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello"))
	if err != nil {
		t.Fatalf("ParseRequestHead returned error: %v", err)
	}
	if parsed.BodyKind != httpcore.ContentLengthBody(5) {
		t.Fatalf("unexpected body kind: %#v", parsed.BodyKind)
	}
}

func TestParseResponseHead(t *testing.T) {
	parsed, err := ParseResponseHead([]byte("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody"))
	if err != nil {
		t.Fatalf("ParseResponseHead returned error: %v", err)
	}
	if parsed.Head.Status != 200 || parsed.Head.Reason != "OK" {
		t.Fatalf("unexpected response head: %#v", parsed.Head)
	}
	if parsed.BodyKind != httpcore.ContentLengthBody(4) {
		t.Fatalf("unexpected body kind: %#v", parsed.BodyKind)
	}
}

func TestResponseWithoutLengthUsesUntilEOF(t *testing.T) {
	parsed, err := ParseResponseHead([]byte("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n"))
	if err != nil {
		t.Fatalf("ParseResponseHead returned error: %v", err)
	}
	if parsed.BodyKind != httpcore.UntilEOFBody() {
		t.Fatalf("unexpected body kind: %#v", parsed.BodyKind)
	}
}

func TestBodylessStatusCodes(t *testing.T) {
	parsed, err := ParseResponseHead([]byte("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n"))
	if err != nil {
		t.Fatalf("ParseResponseHead returned error: %v", err)
	}
	if parsed.BodyKind != httpcore.NoBody() {
		t.Fatalf("unexpected body kind: %#v", parsed.BodyKind)
	}
}

func TestAcceptsLFOnlyAndDuplicateHeaders(t *testing.T) {
	parsed, err := ParseResponseHead([]byte("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload"))
	if err != nil {
		t.Fatalf("ParseResponseHead returned error: %v", err)
	}
	if len(parsed.Head.Headers) != 2 {
		t.Fatalf("expected duplicate headers to be preserved: %#v", parsed.Head.Headers)
	}
}

func TestRejectsInvalidHeader(t *testing.T) {
	if _, err := ParseRequestHead([]byte("GET / HTTP/1.1\r\nHost example.com\r\n\r\n")); err == nil {
		t.Fatal("expected invalid header error")
	}
}

func TestRejectsInvalidContentLength(t *testing.T) {
	if _, err := ParseResponseHead([]byte("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n")); err == nil {
		t.Fatal("expected invalid content length error")
	}
}
