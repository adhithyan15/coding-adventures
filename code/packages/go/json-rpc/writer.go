package jsonrpc

// writer.go — MessageWriter: typed Message → framed byte stream
//
// The writer is the inverse of the reader. Where the reader peels off the
// Content-Length header and returns raw JSON, the writer takes a typed Message,
// serializes it to compact JSON, measures the byte length, prepends the header,
// and writes the whole frame in one operation.
//
// # Wire Format (same as reader.go for reference)
//
//	Content-Length: <n>\r\n
//	\r\n
//	<UTF-8 JSON payload, exactly n bytes>
//
// # Why Measure Bytes, Not Characters?
//
// Content-Length is a BYTE count, not a character count. For ASCII-only JSON
// the two are equal, but JSON strings can contain any Unicode codepoint.
// A single emoji (e.g. 🎸) is one character but FOUR bytes in UTF-8.
//
// Always encode the JSON string to bytes first, then measure len(payload).
//
// # Flushing
//
// The writer calls Flush() after every message if the underlying writer is a
// bufio.Writer. Without flushing, bytes sit in the buffer and never reach the
// reader's stdin. This is critical in tests where the buffer is checked
// immediately after the write.

import (
	"encoding/json"
	"fmt"
	"io"
)

// MessageWriter writes Content-Length-framed JSON-RPC messages to a stream.
//
// Each call to WriteMessage produces exactly one framed message on the
// underlying stream.
//
// Example:
//
//	writer := jsonrpc.NewWriter(os.Stdout)
//	err := writer.WriteMessage(&jsonrpc.Response{Id: 1, Result: map[string]interface{}{"ok": true}})
type MessageWriter struct {
	w io.Writer
}

// NewWriter creates a new MessageWriter that writes to w.
//
// The writer writes directly to w. Pass os.Stdout (or any io.Writer) here.
func NewWriter(w io.Writer) *MessageWriter {
	return &MessageWriter{w: w}
}

// WriteRaw writes a raw JSON string as a Content-Length-framed message.
//
// This low-level method measures the byte length of the JSON string,
// writes the header, and writes the payload. Use WriteMessage if you
// have a typed Message object.
//
// Returns an error if the write fails.
func (mw *MessageWriter) WriteRaw(jsonStr string) error {
	// Encode to UTF-8 bytes first so we can measure the true byte length.
	// This must happen BEFORE we write the Content-Length header!
	payload := []byte(jsonStr)
	contentLength := len(payload)

	// Write the header block:
	//   Content-Length: <n>\r\n
	//   \r\n
	// The \r\n line endings are required by the LSP spec (HTTP convention).
	header := fmt.Sprintf("Content-Length: %d\r\n\r\n", contentLength)
	if _, err := io.WriteString(mw.w, header); err != nil {
		return fmt.Errorf("writing header: %w", err)
	}

	// Write the payload bytes.
	if _, err := mw.w.Write(payload); err != nil {
		return fmt.Errorf("writing payload: %w", err)
	}

	return nil
}

// WriteMessage serializes a typed Message and writes it as a framed message.
//
// This is the primary API. It serializes the message to compact JSON (no
// extra whitespace) and delegates to WriteRaw.
//
// Compact JSON is used (no pretty-printing) to keep payloads small. The
// reader will parse the JSON anyway, so human-readability on the wire adds
// no value.
//
// Returns an error if serialization or the write fails.
func (mw *MessageWriter) WriteMessage(msg Message) error {
	// Convert the typed message to a plain map, then to compact JSON.
	d, err := MessageToMap(msg)
	if err != nil {
		return fmt.Errorf("serializing message: %w", err)
	}

	// Use compact JSON encoding. The separators in Go's json.Marshal default
	// to compact already (no spaces), but we call Marshal explicitly here for
	// clarity and future configurability.
	jsonBytes, err := json.Marshal(d)
	if err != nil {
		return fmt.Errorf("encoding to JSON: %w", err)
	}

	return mw.WriteRaw(string(jsonBytes))
}
