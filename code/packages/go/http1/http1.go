// Package http1 parses HTTP/1 request and response heads.
//
// The job here is intentionally small and precise: consume bytes until the end
// of the HTTP/1 head, turn the start line and headers into semantic values, and
// tell the caller how the body should be consumed.
package http1

import (
	"bytes"
	"fmt"
	"strconv"
	"strings"

	httpcore "github.com/adhithyan15/coding-adventures/code/packages/go/http-core"
)

// ParsedRequestHead contains the semantic request head plus framing metadata.
type ParsedRequestHead struct {
	Head       httpcore.RequestHead
	BodyOffset int
	BodyKind   httpcore.BodyKind
}

// ParsedResponseHead contains the semantic response head plus framing metadata.
type ParsedResponseHead struct {
	Head       httpcore.ResponseHead
	BodyOffset int
	BodyKind   httpcore.BodyKind
}

// ParseRequestHead parses one HTTP/1 request head from raw bytes.
func ParseRequestHead(input []byte) (ParsedRequestHead, error) {
	lines, bodyOffset, err := splitHeadLines(input)
	if err != nil {
		return ParsedRequestHead{}, err
	}
	if len(lines) == 0 {
		return ParsedRequestHead{}, fmt.Errorf("invalid HTTP/1 start line")
	}

	parts := strings.Fields(string(lines[0]))
	if len(parts) != 3 {
		return ParsedRequestHead{}, fmt.Errorf("invalid HTTP/1 start line: %s", string(lines[0]))
	}

	version, err := httpcore.ParseHttpVersion(parts[2])
	if err != nil {
		return ParsedRequestHead{}, err
	}

	headers, err := parseHeaders(lines[1:])
	if err != nil {
		return ParsedRequestHead{}, err
	}
	bodyKind, err := requestBodyKind(headers)
	if err != nil {
		return ParsedRequestHead{}, err
	}

	return ParsedRequestHead{
		Head: httpcore.RequestHead{
			Method:  parts[0],
			Target:  parts[1],
			Version: version,
			Headers: headers,
		},
		BodyOffset: bodyOffset,
		BodyKind:   bodyKind,
	}, nil
}

// ParseResponseHead parses one HTTP/1 response head from raw bytes.
func ParseResponseHead(input []byte) (ParsedResponseHead, error) {
	lines, bodyOffset, err := splitHeadLines(input)
	if err != nil {
		return ParsedResponseHead{}, err
	}
	if len(lines) == 0 {
		return ParsedResponseHead{}, fmt.Errorf("invalid HTTP/1 status line")
	}

	parts := strings.Fields(string(lines[0]))
	if len(parts) < 2 {
		return ParsedResponseHead{}, fmt.Errorf("invalid HTTP/1 status line: %s", string(lines[0]))
	}

	version, err := httpcore.ParseHttpVersion(parts[0])
	if err != nil {
		return ParsedResponseHead{}, err
	}
	status, err := strconv.Atoi(parts[1])
	if err != nil {
		return ParsedResponseHead{}, fmt.Errorf("invalid HTTP status: %s", parts[1])
	}

	reason := ""
	if len(parts) > 2 {
		reason = strings.Join(parts[2:], " ")
	}

	headers, err := parseHeaders(lines[1:])
	if err != nil {
		return ParsedResponseHead{}, err
	}
	bodyKind, err := responseBodyKind(status, headers)
	if err != nil {
		return ParsedResponseHead{}, err
	}

	return ParsedResponseHead{
		Head: httpcore.ResponseHead{
			Version: version,
			Status:  status,
			Reason:  reason,
			Headers: headers,
		},
		BodyOffset: bodyOffset,
		BodyKind:   bodyKind,
	}, nil
}

func splitHeadLines(input []byte) ([][]byte, int, error) {
	index := 0
	for index < len(input) {
		if bytes.HasPrefix(input[index:], []byte("\r\n")) {
			index += 2
			continue
		}
		if input[index] == '\n' {
			index++
			continue
		}
		break
	}

	lines := make([][]byte, 0, 8)
	for {
		if index >= len(input) {
			return nil, 0, fmt.Errorf("incomplete HTTP/1 head")
		}

		lineStart := index
		for index < len(input) && input[index] != '\n' {
			index++
		}
		if index >= len(input) {
			return nil, 0, fmt.Errorf("incomplete HTTP/1 head")
		}

		lineEnd := index
		if lineEnd > lineStart && input[lineEnd-1] == '\r' {
			lineEnd--
		}
		line := input[lineStart:lineEnd]
		index++

		if len(line) == 0 {
			return lines, index, nil
		}
		lines = append(lines, line)
	}
}

func parseHeaders(lines [][]byte) ([]httpcore.Header, error) {
	headers := make([]httpcore.Header, 0, len(lines))
	for _, line := range lines {
		text := string(line)
		name, rawValue, found := strings.Cut(text, ":")
		if !found || strings.TrimSpace(name) == "" {
			return nil, fmt.Errorf("invalid HTTP/1 header: %s", text)
		}
		headers = append(headers, httpcore.Header{
			Name:  strings.TrimSpace(name),
			Value: strings.Trim(rawValue, " \t"),
		})
	}
	return headers, nil
}

func requestBodyKind(headers []httpcore.Header) (httpcore.BodyKind, error) {
	if hasChunkedTransferEncoding(headers) {
		return httpcore.ChunkedBody(), nil
	}

	length, ok, err := declaredContentLength(headers)
	if err != nil {
		return httpcore.BodyKind{}, err
	}
	if !ok || length == 0 {
		return httpcore.NoBody(), nil
	}
	return httpcore.ContentLengthBody(length), nil
}

func responseBodyKind(status int, headers []httpcore.Header) (httpcore.BodyKind, error) {
	if (status >= 100 && status < 200) || status == 204 || status == 304 {
		return httpcore.NoBody(), nil
	}
	if hasChunkedTransferEncoding(headers) {
		return httpcore.ChunkedBody(), nil
	}

	length, ok, err := declaredContentLength(headers)
	if err != nil {
		return httpcore.BodyKind{}, err
	}
	if !ok {
		return httpcore.UntilEOFBody(), nil
	}
	if length == 0 {
		return httpcore.NoBody(), nil
	}
	return httpcore.ContentLengthBody(length), nil
}

func declaredContentLength(headers []httpcore.Header) (int, bool, error) {
	value, ok := httpcore.FindHeader(headers, "Content-Length")
	if !ok {
		return 0, false, nil
	}
	length, err := strconv.Atoi(value)
	if err != nil || length < 0 {
		return 0, false, fmt.Errorf("invalid Content-Length: %s", value)
	}
	return length, true, nil
}

func hasChunkedTransferEncoding(headers []httpcore.Header) bool {
	for _, header := range headers {
		if !strings.EqualFold(header.Name, "Transfer-Encoding") {
			continue
		}
		for _, part := range strings.Split(header.Value, ",") {
			if strings.EqualFold(strings.TrimSpace(part), "chunked") {
				return true
			}
		}
	}
	return false
}
