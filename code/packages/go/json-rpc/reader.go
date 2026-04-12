package jsonrpc

// reader.go — MessageReader: framed byte stream → typed Message
//
// # The Problem: Byte Streams Have No Message Boundaries
//
// A TCP connection or stdin is a continuous byte stream. JSON has no self-
// delimiting syntax at the stream level: you cannot tell where one JSON
// object ends and the next begins without fully parsing every character.
//
// Consider two back-to-back JSON messages:
//
//	{"jsonrpc":"2.0","id":1,"method":"foo"}{"jsonrpc":"2.0","method":"bar"}
//
// Where does the first message end? At the "}" after "foo"}? But nested
// objects also end with "}" — brace counting requires a full JSON parser
// just to find the message boundary.
//
// # The Solution: Content-Length Framing
//
// The LSP spec borrows HTTP's Content-Length header to pre-announce the
// byte length of each payload:
//
//	Content-Length: <n>\r\n
//	\r\n
//	<UTF-8 JSON payload, exactly n bytes>
//
// The reader reads headers line by line until a blank line, extracts the
// Content-Length value, then reads exactly that many bytes. No brace
// counting, no buffering ambiguity.
//
// # EOF Handling
//
// If the stream reaches EOF while reading the header (i.e., between messages),
// ReadMessage returns (nil, io.EOF) — the server's Serve() loop uses this as
// the clean shutdown signal.
//
// EOF mid-message (after the header but before all payload bytes) is an error
// because the message is incomplete.

import (
	"bufio"
	"fmt"
	"io"
	"strconv"
	"strings"
)

// MessageReader reads Content-Length-framed JSON-RPC messages from a stream.
//
// Each call to ReadMessage reads exactly one message. The reader is not
// goroutine-safe; use one reader per goroutine if concurrent reading is needed
// (the single-threaded server model makes this unnecessary in practice).
//
// Example:
//
//	reader := jsonrpc.NewReader(os.Stdin)
//	for {
//	    msg, err := reader.ReadMessage()
//	    if err == io.EOF {
//	        break  // stdin closed
//	    }
//	    if err != nil {
//	        // framing or parse error
//	    }
//	    // handle msg
//	}
type MessageReader struct {
	// bufio.Reader wraps the underlying stream with a small read buffer.
	// This makes readline() efficient — without it, every character read
	// would be a separate syscall.
	r *bufio.Reader
}

// NewReader creates a new MessageReader that reads from r.
//
// The underlying stream is wrapped in a bufio.Reader for efficient line
// reading. Pass os.Stdin (or any io.Reader) here.
func NewReader(r io.Reader) *MessageReader {
	return &MessageReader{r: bufio.NewReader(r)}
}

// ReadRaw reads one framed message and returns the raw JSON string.
//
// This low-level method reads the Content-Length header block, then reads
// exactly that many bytes of payload, and returns the decoded UTF-8 string.
//
// Returns ("", io.EOF) on clean end-of-stream between messages.
// Returns ("", *ResponseError) on malformed framing.
func (mr *MessageReader) ReadRaw() (string, error) {
	// --- Phase 1: Read headers -------------------------------------------
	// Headers are ASCII lines terminated by \r\n. The header block ends with
	// a blank line (\r\n alone). We read lines until we see that blank line.

	var contentLength int = -1 // -1 means "not yet found"

	for {
		line, err := mr.r.ReadString('\n')

		if err == io.EOF {
			// If we haven't read any header yet, this is a clean EOF
			// between messages — signal the caller with io.EOF.
			if contentLength == -1 && line == "" {
				return "", io.EOF
			}
			// EOF in the middle of the header block is an error.
			return "", &ResponseError{
				Code:    ParseError,
				Message: "Parse error: unexpected EOF in header block",
			}
		}

		if err != nil {
			return "", &ResponseError{
				Code:    ParseError,
				Message: fmt.Sprintf("Parse error: reading header: %s", err.Error()),
			}
		}

		// Trim trailing \r\n (or just \n).
		line = strings.TrimRight(line, "\r\n")

		// A blank line signals the end of the header block.
		if line == "" {
			break
		}

		// Parse the header field. We only care about Content-Length.
		// The comparison is case-insensitive per HTTP convention.
		if idx := strings.Index(line, ":"); idx >= 0 {
			name := strings.TrimSpace(line[:idx])
			value := strings.TrimSpace(line[idx+1:])
			if strings.EqualFold(name, "content-length") {
				cl, parseErr := strconv.Atoi(value)
				if parseErr != nil {
					return "", &ResponseError{
						Code:    ParseError,
						Message: fmt.Sprintf("Parse error: invalid Content-Length value %q", value),
					}
				}
				contentLength = cl
			}
		}
	}

	// --- Phase 2: Validate Content-Length ----------------------------------

	if contentLength == -1 {
		return "", &ResponseError{
			Code:    ParseError,
			Message: "Parse error: no Content-Length header found",
		}
	}

	if contentLength < 0 {
		return "", &ResponseError{
			Code:    ParseError,
			Message: fmt.Sprintf("Parse error: Content-Length must be non-negative, got %d", contentLength),
		}
	}

	// --- Phase 3: Read exactly contentLength bytes of payload -------------
	// We must read EXACTLY this many bytes — not more, not less.
	// Reading more would consume bytes belonging to the next message.

	payload := make([]byte, contentLength)
	n, err := io.ReadFull(mr.r, payload)

	if err == io.ErrUnexpectedEOF || (err == nil && n < contentLength) {
		return "", &ResponseError{
			Code:    ParseError,
			Message: fmt.Sprintf("Parse error: expected %d bytes but got %d", contentLength, n),
		}
	}

	if err != nil {
		return "", &ResponseError{
			Code:    ParseError,
			Message: fmt.Sprintf("Parse error: reading payload: %s", err.Error()),
		}
	}

	// The payload is UTF-8 by spec. Go strings are byte slices, so this
	// conversion is always valid — no encoding check needed here. Invalid
	// UTF-8 would cause json.Unmarshal to fail in the next step.
	return string(payload), nil
}

// ReadMessage reads one framed message and returns a typed Message.
//
// Calls ReadRaw to get the JSON string, then calls ParseMessage to convert
// it to a typed struct.
//
// Returns (nil, io.EOF) on clean end-of-stream.
// Returns (nil, *ResponseError) on framing or parse errors.
func (mr *MessageReader) ReadMessage() (Message, error) {
	raw, err := mr.ReadRaw()
	if err != nil {
		return nil, err
	}
	return ParseMessage(raw)
}
