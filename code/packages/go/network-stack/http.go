package networkstack

// HTTP — Layer 7 (Application)
//
// HTTP is the language of the web. A client sends a request, the server sends
// a response. Both are plain text (in HTTP/1.1), making them easy to debug.
//
// # Request Format
//
//	GET /index.html HTTP/1.1\r\n
//	Host: example.com\r\n
//	\r\n
//	(optional body)
//
// # Response Format
//
//	HTTP/1.1 200 OK\r\n
//	Content-Type: text/html\r\n
//	Content-Length: 13\r\n
//	\r\n
//	Hello, World!
//
// The \r\n (CRLF) line endings are required by the spec. A blank line
// separates headers from body.

import (
	"fmt"
	"strconv"
	"strings"
)

// HTTPRequest represents a client's request to a server.
type HTTPRequest struct {
	Method  string            // GET, POST, PUT, DELETE, etc.
	Path    string            // e.g., "/index.html"
	Headers map[string]string // e.g., {"Host": "example.com"}
	Body    []byte            // Request body (empty for GET)
}

// NewHTTPRequest creates a GET request to "/" with no headers.
func NewHTTPRequest() *HTTPRequest {
	return &HTTPRequest{
		Method:  "GET",
		Path:    "/",
		Headers: make(map[string]string),
	}
}

// Serialize converts the request to raw HTTP bytes.
func (r *HTTPRequest) Serialize() []byte {
	var b strings.Builder
	b.WriteString(fmt.Sprintf("%s %s HTTP/1.1\r\n", r.Method, r.Path))

	headers := make(map[string]string)
	for k, v := range r.Headers {
		headers[k] = v
	}
	if len(r.Body) > 0 {
		if _, ok := headers["Content-Length"]; !ok {
			headers["Content-Length"] = strconv.Itoa(len(r.Body))
		}
	}

	for k, v := range headers {
		b.WriteString(fmt.Sprintf("%s: %s\r\n", k, v))
	}
	b.WriteString("\r\n")

	result := []byte(b.String())
	result = append(result, r.Body...)
	return result
}

// HTTPResponse represents a server's response to a client.
type HTTPResponse struct {
	StatusCode int               // e.g., 200, 404, 500
	StatusText string            // e.g., "OK", "Not Found"
	Headers    map[string]string // Response headers
	Body       []byte            // Response body
}

// NewHTTPResponse creates a 200 OK response with no body.
func NewHTTPResponse() *HTTPResponse {
	return &HTTPResponse{
		StatusCode: 200,
		StatusText: "OK",
		Headers:    make(map[string]string),
	}
}

// Serialize converts the response to raw HTTP bytes.
func (r *HTTPResponse) Serialize() []byte {
	var b strings.Builder
	b.WriteString(fmt.Sprintf("HTTP/1.1 %d %s\r\n", r.StatusCode, r.StatusText))

	headers := make(map[string]string)
	for k, v := range r.Headers {
		headers[k] = v
	}
	if len(r.Body) > 0 {
		if _, ok := headers["Content-Length"]; !ok {
			headers["Content-Length"] = strconv.Itoa(len(r.Body))
		}
	}

	for k, v := range headers {
		b.WriteString(fmt.Sprintf("%s: %s\r\n", k, v))
	}
	b.WriteString("\r\n")

	result := []byte(b.String())
	result = append(result, r.Body...)
	return result
}

// DeserializeHTTPResponse parses raw HTTP bytes into an HTTPResponse.
func DeserializeHTTPResponse(data []byte) (*HTTPResponse, error) {
	text := string(data)

	var headerSection, bodyText string
	if idx := strings.Index(text, "\r\n\r\n"); idx >= 0 {
		headerSection = text[:idx]
		bodyText = text[idx+4:]
	} else {
		headerSection = text
		bodyText = ""
	}

	lines := strings.Split(headerSection, "\r\n")
	if len(lines) == 0 {
		return nil, fmt.Errorf("empty HTTP response")
	}

	// Parse status line: "HTTP/1.1 200 OK"
	parts := strings.SplitN(lines[0], " ", 3)
	if len(parts) < 2 {
		return nil, fmt.Errorf("invalid HTTP status line: %s", lines[0])
	}

	code, err := strconv.Atoi(parts[1])
	if err != nil {
		return nil, fmt.Errorf("invalid status code: %s", parts[1])
	}

	statusText := ""
	if len(parts) > 2 {
		statusText = parts[2]
	}

	headers := make(map[string]string)
	for _, line := range lines[1:] {
		if idx := strings.Index(line, ": "); idx >= 0 {
			headers[line[:idx]] = line[idx+2:]
		}
	}

	return &HTTPResponse{
		StatusCode: code,
		StatusText: statusText,
		Headers:    headers,
		Body:       []byte(bodyText),
	}, nil
}

// HTTPClient builds HTTP requests and parses responses. It uses a DNS
// resolver to convert hostnames to IP addresses.
type HTTPClient struct {
	DNS *DNSResolver
}

// NewHTTPClient creates a client with the given DNS resolver.
func NewHTTPClient(dns *DNSResolver) *HTTPClient {
	return &HTTPClient{DNS: dns}
}

// BuildRequest creates an HTTP request from a URL.
//
// URL format: http://hostname[:port]/path
//
// Returns (hostname, port, request, error).
func (c *HTTPClient) BuildRequest(method, url string, body []byte) (string, int, *HTTPRequest, error) {
	// Strip scheme
	u := url
	if strings.HasPrefix(u, "http://") {
		u = u[7:]
	} else if strings.HasPrefix(u, "https://") {
		u = u[8:]
	}

	// Split host and path
	var hostPart, path string
	if idx := strings.Index(u, "/"); idx >= 0 {
		hostPart = u[:idx]
		path = u[idx:]
	} else {
		hostPart = u
		path = "/"
	}

	// Split host and port
	hostname := hostPart
	port := 80
	if idx := strings.Index(hostPart, ":"); idx >= 0 {
		hostname = hostPart[:idx]
		p, err := strconv.Atoi(hostPart[idx+1:])
		if err != nil {
			return "", 0, nil, fmt.Errorf("invalid port: %s", hostPart[idx+1:])
		}
		port = p
	}

	req := &HTTPRequest{
		Method:  method,
		Path:    path,
		Headers: map[string]string{"Host": hostname},
		Body:    body,
	}

	return hostname, port, req, nil
}

// ParseResponse is a convenience wrapper around DeserializeHTTPResponse.
func (c *HTTPClient) ParseResponse(data []byte) (*HTTPResponse, error) {
	return DeserializeHTTPResponse(data)
}
