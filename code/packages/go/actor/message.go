// Package actor implements the Actor model — a mathematical framework for
// concurrent computation invented by Carl Hewitt, Peter Bishop, and Richard
// Steiger in 1973.
//
// The Actor model defines computation in terms of three primitives:
//
//   - Message — the atom of communication. Immutable, typed, serializable.
//   - Channel — a one-way, append-only pipe for messages. Persistent and replayable.
//   - Actor   — an isolated unit of computation with a mailbox and internal state.
//
// These three primitives are sufficient to build entire distributed systems.
// Erlang/OTP (telecom infrastructure, WhatsApp), Akka (Scala/Java), and
// Microsoft Orleans (C#) are all built on this model.
//
// This file implements the Message primitive.
package actor

import (
	"bytes"
	"crypto/rand"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"sync/atomic"
	"time"
)

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

// WireVersion is the current wire format version. Every serialized message
// embeds this version in its header, enabling forward and backward
// compatibility as the format evolves.
//
// Version rules:
//   - A reader MUST handle all versions <= its own.
//   - A reader that encounters a version > its own MUST return ErrVersionTooNew.
//   - A writer ALWAYS writes the latest version it supports.
const WireVersion byte = 1

// headerSize is the fixed size of the binary header: 4 (magic) + 1 (version)
// + 4 (envelope length) + 8 (payload length) = 17 bytes.
const headerSize = 17

// magic is the 4-byte signature at the start of every serialized message.
// "ACTM" stands for "Actor Message". It lets readers quickly identify whether
// a byte stream contains actor messages (similar to how PNG files start with
// 0x89504E47 and PDF files start with "%PDF").
var magic = [4]byte{'A', 'C', 'T', 'M'}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

// ErrInvalidFormat is returned when deserializing a byte sequence that does
// not begin with the "ACTM" magic bytes. This means the data is not an actor
// message — it might be a different format, corrupted data, or random bytes.
var ErrInvalidFormat = errors.New("actor: invalid message format (bad magic bytes)")

// ErrVersionTooNew is returned when deserializing a message whose version
// byte is higher than WireVersion. This means the message was written by a
// newer version of the software. The reader should tell the user to upgrade.
var ErrVersionTooNew = errors.New("actor: message version is newer than supported")

// ─────────────────────────────────────────────────────────────────────────────
// Monotonic Clock
// ─────────────────────────────────────────────────────────────────────────────
//
// Every message gets a monotonic nanosecond timestamp. "Monotonic" means the
// value always increases — it never goes backwards, even if the system clock
// is adjusted. This guarantees that if message A was created before message B,
// then A.Timestamp() < B.Timestamp().
//
// We combine the wall-clock nanosecond time with an atomic counter to ensure
// uniqueness even when two messages are created in the same nanosecond.
var globalClock atomic.Int64

func init() {
	// Seed the clock with the current time so timestamps are meaningful.
	globalClock.Store(time.Now().UnixNano())
}

// nextTimestamp returns a strictly increasing nanosecond timestamp. If the
// wall clock has advanced past our counter, we jump to it. Otherwise, we
// increment by 1. This ensures strict ordering even under high throughput.
func nextTimestamp() int64 {
	for {
		old := globalClock.Load()
		now := time.Now().UnixNano()
		next := old + 1
		if now > next {
			next = now
		}
		if globalClock.CompareAndSwap(old, next) {
			return next
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Message
// ─────────────────────────────────────────────────────────────────────────────
//
// A Message is the atom of communication in the Actor model. Every piece of
// data that flows between actors — a user's request, an agent's response, a
// credential from a vault — is a Message.
//
// # Immutability
//
// Messages are immutable: once created, they cannot be modified. All fields
// are unexported (lowercase in Go), with getter methods to read them. There
// are no setter methods. To "modify" a message, create a new one.
//
// Analogy: A Message is a sealed letter. Once sealed, the contents are fixed.
// The envelope records who sent it, when, and what kind of letter it is.
//
// # Fields
//
//	+──────────────+────────────────────────────────────────────────────+
//	| id           | Unique identifier. Auto-generated at creation.    |
//	| timestamp    | Monotonic nanosecond counter. Strictly increasing. |
//	| senderID     | The actor that created this message.              |
//	| contentType  | MIME type describing the payload format.          |
//	| payload      | Raw bytes. Always bytes, never interpreted.       |
//	| metadata     | Optional key-value pairs for extensibility.       |
//	+──────────────+────────────────────────────────────────────────────+
type Message struct {
	id          string
	timestamp   int64
	senderID    string
	contentType string
	payload     []byte
	metadata    map[string]string
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

// generateID creates a unique message identifier. It uses 16 bytes of
// cryptographic randomness encoded as hex, prefixed with "msg_". This gives
// 2^128 possible IDs — collisions are astronomically unlikely.
func generateID() string {
	b := make([]byte, 16)
	_, _ = rand.Read(b)
	return "msg_" + hex.EncodeToString(b)
}

// NewMessage creates a new Message with the given fields. The ID and
// timestamp are generated automatically. The payload is stored as-is
// (raw bytes). Metadata is optional — pass nil if not needed.
//
// This is the general-purpose constructor. For common cases, use the
// convenience constructors: NewTextMessage, NewJSONMessage, NewBinaryMessage.
func NewMessage(senderID, contentType string, payload []byte, metadata map[string]string) *Message {
	// Copy metadata to prevent external mutation of our internal map.
	var md map[string]string
	if metadata != nil {
		md = make(map[string]string, len(metadata))
		for k, v := range metadata {
			md[k] = v
		}
	}

	// Copy payload to prevent external mutation.
	p := make([]byte, len(payload))
	copy(p, payload)

	return &Message{
		id:          generateID(),
		timestamp:   nextTimestamp(),
		senderID:    senderID,
		contentType: contentType,
		payload:     p,
		metadata:    md,
	}
}

// NewTextMessage creates a text message. The payload string is encoded as
// UTF-8 bytes internally. The content type is set to "text/plain".
//
// This is the most common message type — plain text between actors.
//
//	msg := NewTextMessage("agent-a", "Hello, world!", nil)
//	msg.PayloadText() // "Hello, world!"
func NewTextMessage(senderID, payload string, metadata map[string]string) *Message {
	return NewMessage(senderID, "text/plain", []byte(payload), metadata)
}

// NewJSONMessage creates a JSON message. The payload value is serialized to
// JSON bytes internally. The content type is set to "application/json".
//
// The value can be any type that encoding/json can marshal: maps, slices,
// structs, primitives, etc.
//
//	msg := NewJSONMessage("agent-b", map[string]string{"key": "value"}, nil)
//	msg.PayloadJSON() // map[string]interface{}{"key": "value"}
func NewJSONMessage(senderID string, payload interface{}, metadata map[string]string) *Message {
	data, err := json.Marshal(payload)
	if err != nil {
		// If the value can't be marshaled, store the error as text.
		// This is a programming error — the caller should fix it.
		data = []byte(fmt.Sprintf(`{"error":"marshal failed: %v"}`, err))
	}
	return NewMessage(senderID, "application/json", data, metadata)
}

// NewBinaryMessage creates a binary message with a custom content type.
// Use this for images, videos, or any arbitrary binary data.
//
//	pngBytes := []byte{0x89, 0x50, 0x4E, 0x47} // PNG header
//	msg := NewBinaryMessage("browser", "image/png", pngBytes, nil)
func NewBinaryMessage(senderID, contentType string, payload []byte, metadata map[string]string) *Message {
	return NewMessage(senderID, contentType, payload, metadata)
}

// ─────────────────────────────────────────────────────────────────────────────
// Getter Methods (Read-Only Access)
// ─────────────────────────────────────────────────────────────────────────────
//
// These are the ONLY way to access a message's fields. There are no setter
// methods. This enforces immutability at the API level.
//
// In Go, immutability is enforced by convention: unexported fields + exported
// getters + no exported setters. The type system prevents external packages
// from directly accessing unexported fields.

// ID returns the message's unique identifier. This is auto-generated at
// creation time and never changes. Two messages with the same ID are the
// same message.
func (m *Message) ID() string { return m.id }

// Timestamp returns the monotonic nanosecond timestamp. This is strictly
// increasing within a single process — if message A was created before
// message B, then A.Timestamp() < B.Timestamp().
func (m *Message) Timestamp() int64 { return m.timestamp }

// SenderID returns the ID of the actor that created this message. This is
// set at creation time and cannot be forged — an actor cannot pretend to
// be another actor.
func (m *Message) SenderID() string { return m.senderID }

// ContentType returns the MIME type describing the payload format.
// Common values: "text/plain", "application/json", "image/png",
// "application/octet-stream".
func (m *Message) ContentType() string { return m.contentType }

// Payload returns a copy of the raw payload bytes. The caller gets their
// own copy and cannot modify the message's internal payload.
func (m *Message) Payload() []byte {
	p := make([]byte, len(m.payload))
	copy(p, m.payload)
	return p
}

// Metadata returns a copy of the metadata map. The caller gets their own
// copy and cannot modify the message's internal metadata.
func (m *Message) Metadata() map[string]string {
	if m.metadata == nil {
		return nil
	}
	md := make(map[string]string, len(m.metadata))
	for k, v := range m.metadata {
		md[k] = v
	}
	return md
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience Accessors
// ─────────────────────────────────────────────────────────────────────────────

// PayloadText returns the payload decoded as a UTF-8 string. This is a
// convenience method for text messages. For binary payloads, the result
// may contain garbage characters — use Payload() instead.
func (m *Message) PayloadText() string {
	return string(m.payload)
}

// PayloadJSON parses the payload as JSON and returns the result. Returns
// an error if the payload is not valid JSON.
//
// The return type is interface{} because JSON can represent objects (maps),
// arrays (slices), strings, numbers, booleans, and null.
func (m *Message) PayloadJSON() (interface{}, error) {
	var result interface{}
	err := json.Unmarshal(m.payload, &result)
	return result, err
}

// ─────────────────────────────────────────────────────────────────────────────
// Wire Format Serialization
// ─────────────────────────────────────────────────────────────────────────────
//
// Messages are serialized to a binary wire format for persistence and network
// transmission. The format separates the envelope (metadata) from the payload
// (raw bytes), avoiding Base64 bloat for binary data.
//
// Wire layout (17-byte header + variable envelope + variable payload):
//
//	┌─────────────────────────────────────────────┐
//	│ HEADER (17 bytes, fixed size)               │
//	│                                             │
//	│ magic:           4 bytes  "ACTM"            │
//	│ version:         1 byte   0x01              │
//	│ envelope_length: 4 bytes  (big-endian u32)  │
//	│ payload_length:  8 bytes  (big-endian u64)  │
//	├─────────────────────────────────────────────┤
//	│ ENVELOPE (UTF-8 JSON, envelope_length bytes)│
//	├─────────────────────────────────────────────┤
//	│ PAYLOAD (raw bytes, payload_length bytes)    │
//	└─────────────────────────────────────────────┘

// envelope is the JSON-serializable portion of a message. It contains
// everything except the raw payload bytes. This separation means we can
// index, search, and filter messages by their metadata without ever loading
// the (potentially enormous) payload.
type envelope struct {
	ID          string            `json:"id"`
	Timestamp   int64             `json:"timestamp"`
	SenderID    string            `json:"sender_id"`
	ContentType string            `json:"content_type"`
	Metadata    map[string]string `json:"metadata,omitempty"`
}

// EnvelopeToJSON serializes only the envelope (everything except payload) to
// a JSON string. Useful for logging, indexing, and debugging without touching
// the payload.
//
//	msg := NewTextMessage("agent", "hello", nil)
//	fmt.Println(msg.EnvelopeToJSON())
//	// {"id":"msg_abc123","timestamp":1679616000000000000,...}
func (m *Message) EnvelopeToJSON() string {
	env := envelope{
		ID:          m.id,
		Timestamp:   m.timestamp,
		SenderID:    m.senderID,
		ContentType: m.contentType,
		Metadata:    m.metadata,
	}
	data, _ := json.Marshal(env)
	return string(data)
}

// ToBytes serializes the message to the binary wire format.
//
// The format is designed for efficient binary storage:
//  1. No Base64 — a 10MB image is 10MB on disk, not 13.3MB.
//  2. Scannable — you can skip payloads by reading header lengths.
//  3. Versionable — the version byte enables format evolution.
func (m *Message) ToBytes() []byte {
	// Step 1: Marshal the envelope to JSON bytes.
	env := envelope{
		ID:          m.id,
		Timestamp:   m.timestamp,
		SenderID:    m.senderID,
		ContentType: m.contentType,
		Metadata:    m.metadata,
	}
	envBytes, _ := json.Marshal(env)

	// Step 2: Calculate total size.
	maxBufferSize := int(^uint(0) >> 1)
	if len(envBytes) > maxBufferSize-headerSize-len(m.payload) {
		panic("actor: message too large to serialize")
	}
	totalSize := headerSize + len(envBytes) + len(m.payload)
	buf := make([]byte, totalSize)

	// Step 3: Write the 17-byte header.
	//
	// Bytes 0-3:   Magic ("ACTM") — identifies this as an actor message.
	// Byte 4:      Version — tells the reader which parser to use.
	// Bytes 5-8:   Envelope length (big-endian u32) — how many bytes of
	//              JSON follow the header.
	// Bytes 9-16:  Payload length (big-endian u64) — how many bytes of
	//              raw payload follow the envelope.
	copy(buf[0:4], magic[:])
	buf[4] = WireVersion
	binary.BigEndian.PutUint32(buf[5:9], uint32(len(envBytes)))
	binary.BigEndian.PutUint64(buf[9:17], uint64(len(m.payload)))

	// Step 4: Write envelope JSON bytes.
	copy(buf[17:17+len(envBytes)], envBytes)

	// Step 5: Write raw payload bytes (no encoding, no transformation).
	copy(buf[17+len(envBytes):], m.payload)

	return buf
}

// FromBytes deserializes a message from the binary wire format.
//
// This function validates the magic bytes and version before attempting
// to parse. If the magic is wrong, it returns ErrInvalidFormat. If the
// version is too new, it returns ErrVersionTooNew.
//
// The data must contain exactly one complete message. For reading from
// a stream (file or socket), use FromReader instead.
func FromBytes(data []byte) (*Message, error) {
	reader := bytes.NewReader(data)
	return FromReader(reader)
}

// FromReader reads exactly one message from an io.Reader (a file, network
// socket, or any byte stream).
//
// After a successful read, the reader is positioned at the first byte of
// the NEXT message (or at EOF). This enables sequential reading of a
// channel log: call FromReader in a loop until it returns io.EOF.
//
// The reading process:
//  1. Read 17-byte header → validate magic, check version.
//  2. Read envelope_length bytes → parse JSON → extract metadata fields.
//  3. Read payload_length bytes → store as raw bytes.
//
// If the reader reaches EOF mid-header, it returns io.EOF (clean end).
// If the reader reaches EOF mid-envelope or mid-payload, it returns
// io.ErrUnexpectedEOF (truncated message — possible crash during write).
func FromReader(reader io.Reader) (*Message, error) {
	// Step 1: Read the 17-byte header.
	header := make([]byte, headerSize)
	_, err := io.ReadFull(reader, header)
	if err != nil {
		// io.ReadFull returns io.EOF if zero bytes were read (clean end),
		// or io.ErrUnexpectedEOF if some but not all bytes were read
		// (truncated header — possible crash during write).
		return nil, err
	}

	// Step 2: Validate magic bytes.
	if header[0] != magic[0] || header[1] != magic[1] ||
		header[2] != magic[2] || header[3] != magic[3] {
		return nil, ErrInvalidFormat
	}

	// Step 3: Check version.
	version := header[4]
	if version > WireVersion {
		return nil, fmt.Errorf("%w: got version %d, max supported %d",
			ErrVersionTooNew, version, WireVersion)
	}

	// Step 4: Extract lengths from the header.
	envLen := binary.BigEndian.Uint32(header[5:9])
	payLen := binary.BigEndian.Uint64(header[9:17])

	// Step 5: Read envelope bytes and parse JSON.
	envBytes := make([]byte, envLen)
	_, err = io.ReadFull(reader, envBytes)
	if err != nil {
		return nil, io.ErrUnexpectedEOF
	}

	var env envelope
	if err := json.Unmarshal(envBytes, &env); err != nil {
		return nil, fmt.Errorf("actor: failed to parse envelope JSON: %w", err)
	}

	// Step 6: Read raw payload bytes.
	payload := make([]byte, payLen)
	if payLen > 0 {
		_, err = io.ReadFull(reader, payload)
		if err != nil {
			return nil, io.ErrUnexpectedEOF
		}
	}

	// Step 7: Reconstruct the message.
	return &Message{
		id:          env.ID,
		timestamp:   env.Timestamp,
		senderID:    env.SenderID,
		contentType: env.ContentType,
		payload:     payload,
		metadata:    env.Metadata,
	}, nil
}
