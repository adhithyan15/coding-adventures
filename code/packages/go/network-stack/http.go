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
	result, _ := StartNew[*HTTPRequest]("network-stack.NewHTTPRequest", nil,
		func(op *Operation[*HTTPRequest], rf *ResultFactory[*HTTPRequest]) *OperationResult[*HTTPRequest] {
			return rf.Generate(true, false, &HTTPRequest{
				Method:  "GET",
				Path:    "/",
				Headers: make(map[string]string),
			})
		}).GetResult()
	return result
}

// Serialize converts the request to raw HTTP bytes.
func (r *HTTPRequest) Serialize() []byte {
	out, _ := StartNew[[]byte]("network-stack.HTTPRequest.Serialize", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
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
			return rf.Generate(true, false, result)
		}).GetResult()
	return out
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
	result, _ := StartNew[*HTTPResponse]("network-stack.NewHTTPResponse", nil,
		func(op *Operation[*HTTPResponse], rf *ResultFactory[*HTTPResponse]) *OperationResult[*HTTPResponse] {
			return rf.Generate(true, false, &HTTPResponse{
				StatusCode: 200,
				StatusText: "OK",
				Headers:    make(map[string]string),
			})
		}).GetResult()
	return result
}

// Serialize converts the response to raw HTTP bytes.
func (r *HTTPResponse) Serialize() []byte {
	out, _ := StartNew[[]byte]("network-stack.HTTPResponse.Serialize", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
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
			return rf.Generate(true, false, result)
		}).GetResult()
	return out
}

// DeserializeHTTPResponse parses raw HTTP bytes into an HTTPResponse.
func DeserializeHTTPResponse(data []byte) (*HTTPResponse, error) {
	return StartNew[*HTTPResponse]("network-stack.DeserializeHTTPResponse", nil,
		func(op *Operation[*HTTPResponse], rf *ResultFactory[*HTTPResponse]) *OperationResult[*HTTPResponse] {
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
				return rf.Fail(nil, fmt.Errorf("empty HTTP response"))
			}

			// Parse status line: "HTTP/1.1 200 OK"
			parts := strings.SplitN(lines[0], " ", 3)
			if len(parts) < 2 {
				return rf.Fail(nil, fmt.Errorf("invalid HTTP status line: %s", lines[0]))
			}

			code, err := strconv.Atoi(parts[1])
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("invalid status code: %s", parts[1]))
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

			return rf.Generate(true, false, &HTTPResponse{
				StatusCode: code,
				StatusText: statusText,
				Headers:    headers,
				Body:       []byte(bodyText),
			})
		}).GetResult()
}

// HTTPClient builds HTTP requests and parses responses. It uses a DNS
// resolver to convert hostnames to IP addresses.
type HTTPClient struct {
	DNS *DNSResolver
}

// NewHTTPClient creates a client with the given DNS resolver.
func NewHTTPClient(dns *DNSResolver) *HTTPClient {
	result, _ := StartNew[*HTTPClient]("network-stack.NewHTTPClient", nil,
		func(op *Operation[*HTTPClient], rf *ResultFactory[*HTTPClient]) *OperationResult[*HTTPClient] {
			return rf.Generate(true, false, &HTTPClient{DNS: dns})
		}).GetResult()
	return result
}

// BuildRequest creates an HTTP request from a URL.
//
// URL format: http://hostname[:port]/path
//
// Returns (hostname, port, request, error).
func (c *HTTPClient) BuildRequest(method, url string, body []byte) (string, int, *HTTPRequest, error) {
	type buildResult struct {
		hostname string
		port     int
		req      *HTTPRequest
	}
	br, err := StartNew[buildResult]("network-stack.HTTPClient.BuildRequest", buildResult{},
		func(op *Operation[buildResult], rf *ResultFactory[buildResult]) *OperationResult[buildResult] {
			op.AddProperty("method", method)
			op.AddProperty("url", url)
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
				p, parseErr := strconv.Atoi(hostPart[idx+1:])
				if parseErr != nil {
					return rf.Fail(buildResult{}, fmt.Errorf("invalid port: %s", hostPart[idx+1:]))
				}
				port = p
			}

			req := &HTTPRequest{
				Method:  method,
				Path:    path,
				Headers: map[string]string{"Host": hostname},
				Body:    body,
			}

			return rf.Generate(true, false, buildResult{hostname: hostname, port: port, req: req})
		}).GetResult()
	if err != nil {
		return "", 0, nil, err
	}
	return br.hostname, br.port, br.req, nil
}

// ParseResponse is a convenience wrapper around DeserializeHTTPResponse.
func (c *HTTPClient) ParseResponse(data []byte) (*HTTPResponse, error) {
	return StartNew[*HTTPResponse]("network-stack.HTTPClient.ParseResponse", nil,
		func(op *Operation[*HTTPResponse], rf *ResultFactory[*HTTPResponse]) *OperationResult[*HTTPResponse] {
			resp, err := DeserializeHTTPResponse(data)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, resp)
		}).GetResult()
}
