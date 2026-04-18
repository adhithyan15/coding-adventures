package httpcore

import "testing"

func TestParseHttpVersion(t *testing.T) {
	version, err := ParseHttpVersion("HTTP/1.1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if version.Major != 1 || version.Minor != 1 {
		t.Fatalf("unexpected version: %#v", version)
	}
	if version.String() != "HTTP/1.1" {
		t.Fatalf("unexpected render: %s", version.String())
	}
}

func TestFindHeaderCaseInsensitive(t *testing.T) {
	value, ok := FindHeader([]Header{{Name: "Content-Type", Value: "text/plain"}}, "content-type")
	if !ok || value != "text/plain" {
		t.Fatalf("expected case-insensitive match, got %q %v", value, ok)
	}
}

func TestParseContentHelpers(t *testing.T) {
	headers := []Header{
		{Name: "Content-Length", Value: "42"},
		{Name: "Content-Type", Value: "text/html; charset=utf-8"},
	}

	length, ok := ParseContentLength(headers)
	if !ok || length != 42 {
		t.Fatalf("expected content length 42, got %d %v", length, ok)
	}

	mediaType, charset, ok := ParseContentType(headers)
	if !ok || mediaType != "text/html" || charset != "utf-8" {
		t.Fatalf("unexpected content type parse: %q %q %v", mediaType, charset, ok)
	}
}

func TestBodyKindConstructors(t *testing.T) {
	if got := NoBody(); got.Mode != BodyNone {
		t.Fatalf("expected BodyNone, got %#v", got)
	}
	if got := ContentLengthBody(7); got.Mode != BodyContentLength || got.Length != 7 {
		t.Fatalf("expected content-length body, got %#v", got)
	}
}

func TestHeadsDelegateToHelpers(t *testing.T) {
	request := RequestHead{
		Method:  "POST",
		Target:  "/submit",
		Version: HttpVersion{Major: 1, Minor: 1},
		Headers: []Header{{Name: "Content-Length", Value: "5"}},
	}
	response := ResponseHead{
		Version: HttpVersion{Major: 1, Minor: 0},
		Status:  200,
		Reason:  "OK",
		Headers: []Header{{Name: "Content-Type", Value: "application/json"}},
	}

	if length, ok := request.ContentLength(); !ok || length != 5 {
		t.Fatalf("expected request content length 5, got %d %v", length, ok)
	}

	mediaType, charset, ok := response.ContentType()
	if !ok || mediaType != "application/json" || charset != "" {
		t.Fatalf("unexpected response content type: %q %q %v", mediaType, charset, ok)
	}
}
