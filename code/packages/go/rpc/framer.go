package rpc

import "io"

// framer.go — RpcFramer: the byte-stream boundary interface
//
// # What is a framer?
//
// A byte stream (like TCP or stdin/stdout) is continuous — it has no concept of
// message boundaries. One write of 100 bytes and two writes of 50 bytes each
// produce the same stream from the reader's perspective. The framer's job is to
// add and interpret boundaries so that each Read returns exactly one complete
// message payload.
//
// Think of the framer like a mail sorter at a post office. The postal truck
// delivers an undifferentiated stream of packages (bytes). The sorter reads the
// label on each package, figures out where one package ends and the next begins,
// and hands each complete package to the recipient. The codec (translator) then
// reads what's inside the package. The sorter doesn't care what language the
// letters are written in; the translator doesn't care how the packages were
// sorted.
//
// # Framing strategies
//
// Different framing strategies are suited to different transports:
//
//   - ContentLengthFramer: prepends "Content-Length: N\r\n\r\n"; used by LSP.
//     Robust because the receiver can skip ahead exactly N bytes without scanning.
//
//   - LengthPrefixFramer: prepends a fixed-width (4 or 8 byte) big-endian
//     integer length. Compact and efficient for binary protocols (gRPC, msgpack-rpc).
//
//   - NewlineFramer: appends '\n'; used by NDJSON (Newline-Delimited JSON)
//     streaming. Simple but unsuitable for binary payloads that contain newlines.
//
//   - WebSocketFramer: wraps each payload in a WebSocket data frame. Used when
//     the transport is a WebSocket connection (browser clients).
//
//   - PassthroughFramer: no framing; each WriteFrame call is one complete
//     stream (useful when the transport—e.g., HTTP—handles boundaries externally).
//
// # Contract
//
//   - ReadFrame returns io.EOF on clean EOF (peer closed the connection normally).
//   - ReadFrame returns another error on framing failures (truncated length prefix,
//     malformed header, etc.).
//   - WriteFrame is called with exactly the payload bytes — no framing envelope
//     should be pre-applied. The framer adds the envelope itself.
//   - Each ReadFrame call returns exactly one complete payload (the bytes between
//     two envelope boundaries). Back-to-back reads must not cross-contaminate.

// RpcFramer reads and writes discrete byte chunks from/to a raw byte stream.
//
// A framer instance is stateful — it maintains a read position in the underlying
// stream. Do not share a single RpcFramer instance across concurrent goroutines
// without external synchronization.
//
// The framer knows nothing about the content of the chunks it reads and writes.
// It is only responsible for identifying where one chunk ends and the next begins.
//
// Typical usage:
//
//	for {
//	    data, err := framer.ReadFrame()
//	    if err == io.EOF {
//	        break // clean close
//	    }
//	    if err != nil {
//	        // framing error — send error response, then continue or break
//	    }
//	    msg, err := codec.Decode(data)
//	    // ...
//	}
type RpcFramer interface {
	// ReadFrame reads the next complete payload from the stream.
	//
	// Returns (payload, nil) when a complete frame is available.
	// Returns (nil, io.EOF) on a clean close — the peer disconnected gracefully.
	// Returns (nil, err) on any framing error (truncated header, invalid length,
	// etc.). The server should send a ParseError response and may choose to
	// continue reading or shut down.
	//
	// The returned byte slice is owned by the caller — the framer must not
	// reuse its backing array.
	ReadFrame() ([]byte, error)

	// WriteFrame sends a payload to the stream, wrapping it in whatever
	// framing envelope this framer implements.
	//
	// The caller provides exactly the payload bytes. WriteFrame prepends,
	// appends, or wraps them as needed (e.g., adding a Content-Length header
	// or a 4-byte length prefix).
	//
	// Returns nil on success, or an error if the underlying write failed.
	WriteFrame(data []byte) error
}

// ioEOF re-exports io.EOF so callers of this package who only import "rpc"
// can still compare ReadFrame errors without importing "io" themselves.
//
// Usage:
//
//	data, err := framer.ReadFrame()
//	if err == rpc.EOF {
//	    // clean close
//	}
//
// This is a convenience alias; the underlying value is identical to io.EOF.
var EOF = io.EOF
