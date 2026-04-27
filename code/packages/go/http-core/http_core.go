// Package httpcore defines shared HTTP request/response head types.
//
// The wire syntax of HTTP/1, HTTP/2, and HTTP/3 is very different, but the
// semantic shapes that applications consume are much more stable. A browser or
// client still needs headers, versions, status codes, and body framing
// instructions no matter which HTTP version produced them.
package httpcore

import (
	"fmt"
	"strconv"
	"strings"
)

// Header preserves one header line in arrival order.
type Header struct {
	Name  string
	Value string
}

// HttpVersion stores the semantic major/minor version numbers.
type HttpVersion struct {
	Major int
	Minor int
}

// ParseHttpVersion converts "HTTP/1.1" into a structured version value.
func ParseHttpVersion(text string) (HttpVersion, error) {
	if !strings.HasPrefix(text, "HTTP/") {
		return HttpVersion{}, fmt.Errorf("invalid HTTP version: %s", text)
	}

	parts := strings.SplitN(text[5:], ".", 2)
	if len(parts) != 2 {
		return HttpVersion{}, fmt.Errorf("invalid HTTP version: %s", text)
	}

	major, majorErr := strconv.Atoi(parts[0])
	minor, minorErr := strconv.Atoi(parts[1])
	if majorErr != nil || minorErr != nil {
		return HttpVersion{}, fmt.Errorf("invalid HTTP version: %s", text)
	}

	return HttpVersion{Major: major, Minor: minor}, nil
}

func (version HttpVersion) String() string {
	return fmt.Sprintf("HTTP/%d.%d", version.Major, version.Minor)
}

// BodyMode identifies how the caller should read the payload bytes.
type BodyMode string

const (
	BodyNone          BodyMode = "none"
	BodyContentLength BodyMode = "content-length"
	BodyUntilEOF      BodyMode = "until-eof"
	BodyChunked       BodyMode = "chunked"
)

// BodyKind describes the framing rule for the body.
type BodyKind struct {
	Mode   BodyMode
	Length int
}

func NoBody() BodyKind {
	return BodyKind{Mode: BodyNone}
}

func ContentLengthBody(length int) BodyKind {
	return BodyKind{Mode: BodyContentLength, Length: length}
}

func UntilEOFBody() BodyKind {
	return BodyKind{Mode: BodyUntilEOF}
}

func ChunkedBody() BodyKind {
	return BodyKind{Mode: BodyChunked}
}

// RequestHead is the semantic shape of an HTTP request head.
type RequestHead struct {
	Method  string
	Target  string
	Version HttpVersion
	Headers []Header
}

func (head RequestHead) Header(name string) (string, bool) {
	return FindHeader(head.Headers, name)
}

func (head RequestHead) ContentLength() (int, bool) {
	return ParseContentLength(head.Headers)
}

func (head RequestHead) ContentType() (string, string, bool) {
	return ParseContentType(head.Headers)
}

// ResponseHead is the semantic shape of an HTTP response head.
type ResponseHead struct {
	Version HttpVersion
	Status  int
	Reason  string
	Headers []Header
}

func (head ResponseHead) Header(name string) (string, bool) {
	return FindHeader(head.Headers, name)
}

func (head ResponseHead) ContentLength() (int, bool) {
	return ParseContentLength(head.Headers)
}

func (head ResponseHead) ContentType() (string, string, bool) {
	return ParseContentType(head.Headers)
}

// FindHeader performs ASCII-insensitive lookup and returns the first match.
func FindHeader(headers []Header, name string) (string, bool) {
	for _, header := range headers {
		if strings.EqualFold(header.Name, name) {
			return header.Value, true
		}
	}
	return "", false
}

// ParseContentLength returns the non-negative Content-Length value when valid.
func ParseContentLength(headers []Header) (int, bool) {
	value, ok := FindHeader(headers, "Content-Length")
	if !ok {
		return 0, false
	}
	length, err := strconv.Atoi(value)
	if err != nil || length < 0 {
		return 0, false
	}
	return length, true
}

// ParseContentType splits Content-Type into media type and optional charset.
func ParseContentType(headers []Header) (string, string, bool) {
	value, ok := FindHeader(headers, "Content-Type")
	if !ok {
		return "", "", false
	}

	pieces := strings.Split(value, ";")
	mediaType := strings.TrimSpace(pieces[0])
	if mediaType == "" {
		return "", "", false
	}

	charset := ""
	for _, piece := range pieces[1:] {
		key, rawValue, found := strings.Cut(piece, "=")
		if found && strings.EqualFold(strings.TrimSpace(key), "charset") {
			charset = strings.Trim(strings.TrimSpace(rawValue), `"`)
			break
		}
	}

	return mediaType, charset, true
}
