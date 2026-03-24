package actor

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════════
// Test 1: Create message — verify all fields
// ═══════════════════════════════════════════════════════════════════════════

func TestCreateMessage(t *testing.T) {
	t.Run("all fields are set correctly", func(t *testing.T) {
		meta := map[string]string{"key": "value"}
		msg := NewMessage("sender-1", "text/plain", []byte("hello"), meta)

		if msg.SenderID() != "sender-1" {
			t.Errorf("SenderID: got %q, want %q", msg.SenderID(), "sender-1")
		}
		if msg.ContentType() != "text/plain" {
			t.Errorf("ContentType: got %q, want %q", msg.ContentType(), "text/plain")
		}
		if string(msg.Payload()) != "hello" {
			t.Errorf("Payload: got %q, want %q", string(msg.Payload()), "hello")
		}
		if msg.Metadata()["key"] != "value" {
			t.Errorf("Metadata: got %v, want key=value", msg.Metadata())
		}
		if msg.ID() == "" {
			t.Error("ID should be auto-generated, got empty string")
		}
		if !strings.HasPrefix(msg.ID(), "msg_") {
			t.Errorf("ID should start with 'msg_', got %q", msg.ID())
		}
		if msg.Timestamp() <= 0 {
			t.Errorf("Timestamp should be positive, got %d", msg.Timestamp())
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 2: Immutability — no setter methods, external mutation doesn't affect
// ═══════════════════════════════════════════════════════════════════════════

func TestMessageImmutability(t *testing.T) {
	t.Run("modifying returned payload does not affect message", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		payload := msg.Payload()
		payload[0] = 'X' // mutate the returned copy

		// Original should be unchanged.
		if string(msg.Payload()) != "hello" {
			t.Errorf("Payload was mutated: got %q, want %q", string(msg.Payload()), "hello")
		}
	})

	t.Run("modifying returned metadata does not affect message", func(t *testing.T) {
		meta := map[string]string{"key": "value"}
		msg := NewTextMessage("sender", "hello", meta)
		returned := msg.Metadata()
		returned["key"] = "CHANGED"

		if msg.Metadata()["key"] != "value" {
			t.Errorf("Metadata was mutated: got %q, want %q", msg.Metadata()["key"], "value")
		}
	})

	t.Run("modifying input payload does not affect message", func(t *testing.T) {
		payload := []byte("hello")
		msg := NewMessage("sender", "text/plain", payload, nil)
		payload[0] = 'X' // mutate the input

		if string(msg.Payload()) != "hello" {
			t.Errorf("Payload was mutated via input: got %q, want %q", string(msg.Payload()), "hello")
		}
	})

	t.Run("modifying input metadata does not affect message", func(t *testing.T) {
		meta := map[string]string{"key": "value"}
		msg := NewMessage("sender", "text/plain", []byte("hi"), meta)
		meta["key"] = "CHANGED"

		if msg.Metadata()["key"] != "value" {
			t.Errorf("Metadata was mutated via input: got %q, want %q", msg.Metadata()["key"], "value")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3: Unique IDs — 1000 messages all have distinct IDs
// ═══════════════════════════════════════════════════════════════════════════

func TestUniqueIDs(t *testing.T) {
	t.Run("1000 messages have unique IDs", func(t *testing.T) {
		ids := make(map[string]bool)
		for i := 0; i < 1000; i++ {
			msg := NewTextMessage("sender", "hello", nil)
			if ids[msg.ID()] {
				t.Fatalf("Duplicate ID found: %s", msg.ID())
			}
			ids[msg.ID()] = true
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 4: Timestamp ordering — strictly increasing
// ═══════════════════════════════════════════════════════════════════════════

func TestTimestampOrdering(t *testing.T) {
	t.Run("timestamps are strictly increasing", func(t *testing.T) {
		var prev int64
		for i := 0; i < 100; i++ {
			msg := NewTextMessage("sender", "hello", nil)
			if msg.Timestamp() <= prev {
				t.Fatalf("Timestamp not increasing: prev=%d, current=%d at i=%d",
					prev, msg.Timestamp(), i)
			}
			prev = msg.Timestamp()
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 5: Wire format round-trip (text)
// ═══════════════════════════════════════════════════════════════════════════

func TestWireFormatRoundTripText(t *testing.T) {
	t.Run("text message survives serialization", func(t *testing.T) {
		meta := map[string]string{"trace": "abc123"}
		original := NewTextMessage("agent-a", "Hello, world!", meta)
		data := original.ToBytes()
		restored, err := FromBytes(data)
		if err != nil {
			t.Fatalf("FromBytes failed: %v", err)
		}

		if restored.ID() != original.ID() {
			t.Errorf("ID mismatch: got %q, want %q", restored.ID(), original.ID())
		}
		if restored.Timestamp() != original.Timestamp() {
			t.Errorf("Timestamp mismatch: got %d, want %d", restored.Timestamp(), original.Timestamp())
		}
		if restored.SenderID() != original.SenderID() {
			t.Errorf("SenderID mismatch: got %q, want %q", restored.SenderID(), original.SenderID())
		}
		if restored.ContentType() != original.ContentType() {
			t.Errorf("ContentType mismatch: got %q, want %q", restored.ContentType(), original.ContentType())
		}
		if string(restored.Payload()) != string(original.Payload()) {
			t.Errorf("Payload mismatch: got %q, want %q", string(restored.Payload()), string(original.Payload()))
		}
		if restored.PayloadText() != "Hello, world!" {
			t.Errorf("PayloadText: got %q, want %q", restored.PayloadText(), "Hello, world!")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 6: Wire format round-trip (binary)
// ═══════════════════════════════════════════════════════════════════════════

func TestWireFormatRoundTripBinary(t *testing.T) {
	t.Run("binary message with PNG header survives serialization", func(t *testing.T) {
		// PNG file signature: 8 bytes.
		pngHeader := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		original := NewBinaryMessage("browser", "image/png", pngHeader, nil)

		data := original.ToBytes()
		restored, err := FromBytes(data)
		if err != nil {
			t.Fatalf("FromBytes failed: %v", err)
		}

		if !bytes.Equal(restored.Payload(), original.Payload()) {
			t.Errorf("Binary payload mismatch")
		}
		if restored.ContentType() != "image/png" {
			t.Errorf("ContentType: got %q, want %q", restored.ContentType(), "image/png")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 7: Metadata passthrough
// ═══════════════════════════════════════════════════════════════════════════

func TestMetadataPassthrough(t *testing.T) {
	t.Run("metadata survives serialization round-trip", func(t *testing.T) {
		meta := map[string]string{
			"correlation_id": "req_abc123",
			"priority":       "high",
			"trace_id":       "trace_xyz",
		}
		original := NewTextMessage("sender", "hello", meta)
		data := original.ToBytes()
		restored, err := FromBytes(data)
		if err != nil {
			t.Fatalf("FromBytes failed: %v", err)
		}

		for k, v := range meta {
			if restored.Metadata()[k] != v {
				t.Errorf("Metadata[%q]: got %q, want %q", k, restored.Metadata()[k], v)
			}
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 8: Empty payload
// ═══════════════════════════════════════════════════════════════════════════

func TestEmptyPayload(t *testing.T) {
	t.Run("empty payload serializes and deserializes", func(t *testing.T) {
		msg := NewMessage("sender", "text/plain", []byte{}, nil)
		if len(msg.Payload()) != 0 {
			t.Errorf("Payload should be empty, got %d bytes", len(msg.Payload()))
		}

		data := msg.ToBytes()
		restored, err := FromBytes(data)
		if err != nil {
			t.Fatalf("FromBytes failed: %v", err)
		}
		if len(restored.Payload()) != 0 {
			t.Errorf("Restored payload should be empty, got %d bytes", len(restored.Payload()))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 9: Large payload (1MB)
// ═══════════════════════════════════════════════════════════════════════════

func TestLargePayload(t *testing.T) {
	t.Run("1MB payload serializes and deserializes", func(t *testing.T) {
		// Create a 1MB payload filled with a repeating pattern.
		payload := make([]byte, 1024*1024)
		for i := range payload {
			payload[i] = byte(i % 256)
		}
		msg := NewBinaryMessage("sender", "application/octet-stream", payload, nil)

		data := msg.ToBytes()
		restored, err := FromBytes(data)
		if err != nil {
			t.Fatalf("FromBytes failed: %v", err)
		}
		if !bytes.Equal(restored.Payload(), payload) {
			t.Error("1MB payload mismatch after round-trip")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 10: Content type preserved
// ═══════════════════════════════════════════════════════════════════════════

func TestContentTypePreserved(t *testing.T) {
	types := []string{
		"text/plain",
		"application/json",
		"image/png",
		"video/mp4",
		"application/octet-stream",
	}
	for _, ct := range types {
		t.Run(ct, func(t *testing.T) {
			msg := NewMessage("sender", ct, []byte("data"), nil)
			data := msg.ToBytes()
			restored, err := FromBytes(data)
			if err != nil {
				t.Fatalf("FromBytes failed: %v", err)
			}
			if restored.ContentType() != ct {
				t.Errorf("ContentType: got %q, want %q", restored.ContentType(), ct)
			}
		})
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 11: Convenience constructors
// ═══════════════════════════════════════════════════════════════════════════

func TestConvenienceConstructors(t *testing.T) {
	t.Run("NewTextMessage sets text/plain", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		if msg.ContentType() != "text/plain" {
			t.Errorf("ContentType: got %q, want %q", msg.ContentType(), "text/plain")
		}
		if msg.PayloadText() != "hello" {
			t.Errorf("PayloadText: got %q, want %q", msg.PayloadText(), "hello")
		}
	})

	t.Run("NewJSONMessage sets application/json", func(t *testing.T) {
		data := map[string]string{"key": "value"}
		msg := NewJSONMessage("sender", data, nil)
		if msg.ContentType() != "application/json" {
			t.Errorf("ContentType: got %q, want %q", msg.ContentType(), "application/json")
		}
	})

	t.Run("NewBinaryMessage sets custom content type", func(t *testing.T) {
		msg := NewBinaryMessage("sender", "image/jpeg", []byte{0xFF, 0xD8}, nil)
		if msg.ContentType() != "image/jpeg" {
			t.Errorf("ContentType: got %q, want %q", msg.ContentType(), "image/jpeg")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 12: PayloadText
// ═══════════════════════════════════════════════════════════════════════════

func TestPayloadText(t *testing.T) {
	t.Run("returns decoded UTF-8 string", func(t *testing.T) {
		msg := NewTextMessage("sender", "Hello, 世界!", nil)
		if msg.PayloadText() != "Hello, 世界!" {
			t.Errorf("PayloadText: got %q, want %q", msg.PayloadText(), "Hello, 世界!")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 13: PayloadJSON
// ═══════════════════════════════════════════════════════════════════════════

func TestPayloadJSON(t *testing.T) {
	t.Run("returns parsed JSON object", func(t *testing.T) {
		original := map[string]interface{}{
			"name": "Alice",
			"age":  float64(30),
		}
		msg := NewJSONMessage("sender", original, nil)
		result, err := msg.PayloadJSON()
		if err != nil {
			t.Fatalf("PayloadJSON failed: %v", err)
		}
		m, ok := result.(map[string]interface{})
		if !ok {
			t.Fatal("PayloadJSON did not return a map")
		}
		if m["name"] != "Alice" {
			t.Errorf("name: got %v, want Alice", m["name"])
		}
		if m["age"] != float64(30) {
			t.Errorf("age: got %v, want 30", m["age"])
		}
	})

	t.Run("returns parsed JSON array", func(t *testing.T) {
		original := []string{"a", "b", "c"}
		msg := NewJSONMessage("sender", original, nil)
		result, err := msg.PayloadJSON()
		if err != nil {
			t.Fatalf("PayloadJSON failed: %v", err)
		}
		arr, ok := result.([]interface{})
		if !ok {
			t.Fatal("PayloadJSON did not return a slice")
		}
		if len(arr) != 3 {
			t.Errorf("array length: got %d, want 3", len(arr))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 14: Envelope-only serialization
// ═══════════════════════════════════════════════════════════════════════════

func TestEnvelopeToJSON(t *testing.T) {
	t.Run("produces JSON without payload", func(t *testing.T) {
		msg := NewTextMessage("agent", "secret payload", nil)
		envJSON := msg.EnvelopeToJSON()

		// The envelope JSON should contain metadata fields but NOT the payload.
		var env map[string]interface{}
		if err := json.Unmarshal([]byte(envJSON), &env); err != nil {
			t.Fatalf("EnvelopeToJSON produced invalid JSON: %v", err)
		}
		if env["sender_id"] != "agent" {
			t.Errorf("sender_id: got %v, want 'agent'", env["sender_id"])
		}
		if env["content_type"] != "text/plain" {
			t.Errorf("content_type: got %v, want 'text/plain'", env["content_type"])
		}
		// Payload should NOT be in the envelope.
		if _, exists := env["payload"]; exists {
			t.Error("Envelope JSON should not contain payload")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 15: Wire format magic bytes
// ═══════════════════════════════════════════════════════════════════════════

func TestWireFormatMagic(t *testing.T) {
	t.Run("serialized bytes start with ACTM", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		data := msg.ToBytes()
		if string(data[0:4]) != "ACTM" {
			t.Errorf("Magic bytes: got %q, want %q", string(data[0:4]), "ACTM")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 16: Wire format version byte
// ═══════════════════════════════════════════════════════════════════════════

func TestWireFormatVersion(t *testing.T) {
	t.Run("version byte matches WireVersion", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		data := msg.ToBytes()
		if data[4] != WireVersion {
			t.Errorf("Version byte: got %d, want %d", data[4], WireVersion)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 17: Future version rejection
// ═══════════════════════════════════════════════════════════════════════════

func TestFutureVersionRejection(t *testing.T) {
	t.Run("version > WireVersion returns ErrVersionTooNew", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		data := msg.ToBytes()
		// Tamper with the version byte to simulate a future version.
		data[4] = WireVersion + 1

		_, err := FromBytes(data)
		if err == nil {
			t.Fatal("Expected error for future version, got nil")
		}
		if !strings.Contains(err.Error(), "newer than supported") {
			t.Errorf("Error should mention 'newer than supported', got: %v", err)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 18: Corrupt magic rejection
// ═══════════════════════════════════════════════════════════════════════════

func TestCorruptMagicRejection(t *testing.T) {
	t.Run("wrong magic returns ErrInvalidFormat", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		data := msg.ToBytes()
		// Corrupt the magic bytes.
		data[0] = 'X'
		data[1] = 'Y'

		_, err := FromBytes(data)
		if err == nil {
			t.Fatal("Expected error for corrupt magic, got nil")
		}
		if err != ErrInvalidFormat {
			t.Errorf("Expected ErrInvalidFormat, got: %v", err)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 19: Stream reading
// ═══════════════════════════════════════════════════════════════════════════

func TestStreamReading(t *testing.T) {
	t.Run("reads exactly one message from a multi-message stream", func(t *testing.T) {
		msg1 := NewTextMessage("sender", "first", nil)
		msg2 := NewTextMessage("sender", "second", nil)

		// Concatenate two messages into a single byte stream.
		var buf bytes.Buffer
		buf.Write(msg1.ToBytes())
		buf.Write(msg2.ToBytes())

		reader := bytes.NewReader(buf.Bytes())

		// Read first message.
		restored1, err := FromReader(reader)
		if err != nil {
			t.Fatalf("First FromReader failed: %v", err)
		}
		if restored1.PayloadText() != "first" {
			t.Errorf("First message: got %q, want %q", restored1.PayloadText(), "first")
		}

		// Read second message — reader should be positioned correctly.
		restored2, err := FromReader(reader)
		if err != nil {
			t.Fatalf("Second FromReader failed: %v", err)
		}
		if restored2.PayloadText() != "second" {
			t.Errorf("Second message: got %q, want %q", restored2.PayloadText(), "second")
		}
	})

	t.Run("returns EOF on empty stream", func(t *testing.T) {
		reader := bytes.NewReader([]byte{})
		_, err := FromReader(reader)
		if err == nil {
			t.Fatal("Expected EOF error, got nil")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test: Nil metadata
// ═══════════════════════════════════════════════════════════════════════════

func TestNilMetadata(t *testing.T) {
	t.Run("nil metadata returns nil from getter", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		if msg.Metadata() != nil {
			t.Errorf("Expected nil metadata, got %v", msg.Metadata())
		}
	})

	t.Run("nil metadata survives serialization", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		data := msg.ToBytes()
		restored, err := FromBytes(data)
		if err != nil {
			t.Fatalf("FromBytes failed: %v", err)
		}
		// After deserialization, nil metadata is fine (omitempty in JSON).
		if restored.Metadata() != nil && len(restored.Metadata()) != 0 {
			t.Errorf("Expected nil/empty metadata after round-trip, got %v", restored.Metadata())
		}
	})
}
