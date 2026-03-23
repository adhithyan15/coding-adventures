package actor

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════════
// Test 20: Create channel
// ═══════════════════════════════════════════════════════════════════════════

func TestCreateChannel(t *testing.T) {
	t.Run("id and name are set correctly", func(t *testing.T) {
		ch := NewChannel("ch_001", "email-summaries")
		if ch.ID() != "ch_001" {
			t.Errorf("ID: got %q, want %q", ch.ID(), "ch_001")
		}
		if ch.Name() != "email-summaries" {
			t.Errorf("Name: got %q, want %q", ch.Name(), "email-summaries")
		}
		if ch.CreatedAt() <= 0 {
			t.Errorf("CreatedAt should be positive, got %d", ch.CreatedAt())
		}
		if ch.Length() != 0 {
			t.Errorf("Length should be 0, got %d", ch.Length())
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 21: Append and length
// ═══════════════════════════════════════════════════════════════════════════

func TestAppendAndLength(t *testing.T) {
	t.Run("appending 3 messages gives length 3", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 3; i++ {
			ch.Append(NewTextMessage("sender", "hello", nil))
		}
		if ch.Length() != 3 {
			t.Errorf("Length: got %d, want 3", ch.Length())
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 22: Append returns sequence number
// ═══════════════════════════════════════════════════════════════════════════

func TestAppendReturnsSequenceNumber(t *testing.T) {
	t.Run("sequence numbers are 0, 1, 2", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 3; i++ {
			seq := ch.Append(NewTextMessage("sender", "hello", nil))
			if seq != i {
				t.Errorf("Sequence number: got %d, want %d", seq, i)
			}
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 23: Read from beginning
// ═══════════════════════════════════════════════════════════════════════════

func TestReadFromBeginning(t *testing.T) {
	t.Run("read all 5 messages from offset 0", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 5; i++ {
			ch.Append(NewTextMessage("sender", "msg", nil))
		}
		msgs := ch.Read(0, 5)
		if len(msgs) != 5 {
			t.Errorf("Read returned %d messages, want 5", len(msgs))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 24: Read with offset
// ═══════════════════════════════════════════════════════════════════════════

func TestReadWithOffset(t *testing.T) {
	t.Run("read messages 2, 3, 4 from offset 2", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 5; i++ {
			ch.Append(NewTextMessage("sender", string(rune('A'+i)), nil))
		}
		msgs := ch.Read(2, 3)
		if len(msgs) != 3 {
			t.Fatalf("Read returned %d messages, want 3", len(msgs))
		}
		if msgs[0].PayloadText() != "C" {
			t.Errorf("First message: got %q, want %q", msgs[0].PayloadText(), "C")
		}
		if msgs[2].PayloadText() != "E" {
			t.Errorf("Last message: got %q, want %q", msgs[2].PayloadText(), "E")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 25: Read past end
// ═══════════════════════════════════════════════════════════════════════════

func TestReadPastEnd(t *testing.T) {
	t.Run("reading past end returns empty slice", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 3; i++ {
			ch.Append(NewTextMessage("sender", "msg", nil))
		}
		msgs := ch.Read(5, 10)
		if len(msgs) != 0 {
			t.Errorf("Read should return empty slice, got %d messages", len(msgs))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 26: Read with limit
// ═══════════════════════════════════════════════════════════════════════════

func TestReadWithLimit(t *testing.T) {
	t.Run("limit caps the number of returned messages", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 10; i++ {
			ch.Append(NewTextMessage("sender", "msg", nil))
		}
		msgs := ch.Read(0, 3)
		if len(msgs) != 3 {
			t.Errorf("Read returned %d messages, want 3", len(msgs))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 27: Slice
// ═══════════════════════════════════════════════════════════════════════════

func TestSlice(t *testing.T) {
	t.Run("slice(1, 4) returns messages 1, 2, 3", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 5; i++ {
			ch.Append(NewTextMessage("sender", string(rune('A'+i)), nil))
		}
		msgs := ch.Slice(1, 4)
		if len(msgs) != 3 {
			t.Fatalf("Slice returned %d messages, want 3", len(msgs))
		}
		if msgs[0].PayloadText() != "B" {
			t.Errorf("First: got %q, want %q", msgs[0].PayloadText(), "B")
		}
		if msgs[2].PayloadText() != "D" {
			t.Errorf("Last: got %q, want %q", msgs[2].PayloadText(), "D")
		}
	})

	t.Run("slice with start >= end returns empty", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		ch.Append(NewTextMessage("sender", "msg", nil))
		msgs := ch.Slice(3, 1)
		if len(msgs) != 0 {
			t.Errorf("Expected empty slice, got %d messages", len(msgs))
		}
	})

	t.Run("slice with end beyond length clamps", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		ch.Append(NewTextMessage("sender", "msg", nil))
		msgs := ch.Slice(0, 100)
		if len(msgs) != 1 {
			t.Errorf("Expected 1 message, got %d", len(msgs))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 28: Independent readers
// ═══════════════════════════════════════════════════════════════════════════

func TestIndependentReaders(t *testing.T) {
	t.Run("two consumers read the same channel independently", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 5; i++ {
			ch.Append(NewTextMessage("sender", string(rune('A'+i)), nil))
		}

		// Consumer A reads from offset 0.
		msgsA := ch.Read(0, 2)
		if len(msgsA) != 2 {
			t.Fatalf("Consumer A: got %d, want 2", len(msgsA))
		}
		if msgsA[0].PayloadText() != "A" {
			t.Errorf("Consumer A first: got %q, want A", msgsA[0].PayloadText())
		}

		// Consumer B reads from offset 3.
		msgsB := ch.Read(3, 2)
		if len(msgsB) != 2 {
			t.Fatalf("Consumer B: got %d, want 2", len(msgsB))
		}
		if msgsB[0].PayloadText() != "D" {
			t.Errorf("Consumer B first: got %q, want D", msgsB[0].PayloadText())
		}

		// Consumer A's read does not affect Consumer B and vice versa.
		if ch.Length() != 5 {
			t.Errorf("Channel length changed: got %d, want 5", ch.Length())
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 29: Append-only — no delete or modify methods
// ═══════════════════════════════════════════════════════════════════════════
//
// This test is structural: we verify that Channel only has Append for writes.
// In Go, this is enforced by the type system — there simply are no Delete or
// Update methods on Channel. We test that the log grows monotonically.

func TestAppendOnly(t *testing.T) {
	t.Run("log only grows, never shrinks", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		for i := 0; i < 5; i++ {
			ch.Append(NewTextMessage("sender", "msg", nil))
			if ch.Length() != i+1 {
				t.Errorf("After append %d: length=%d, want %d", i, ch.Length(), i+1)
			}
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 30: Binary persistence — file starts with ACTM
// ═══════════════════════════════════════════════════════════════════════════

func TestBinaryPersistence(t *testing.T) {
	t.Run("persisted file starts with ACTM magic", func(t *testing.T) {
		dir := t.TempDir()
		ch := NewChannel("ch_001", "test-channel")
		ch.Append(NewTextMessage("sender", "hello", nil))
		ch.Append(NewBinaryMessage("sender", "image/png",
			[]byte{0x89, 0x50, 0x4E, 0x47}, nil))

		err := ch.Persist(dir)
		if err != nil {
			t.Fatalf("Persist failed: %v", err)
		}

		path := filepath.Join(dir, "test-channel.log")
		data, err := os.ReadFile(path)
		if err != nil {
			t.Fatalf("Failed to read persisted file: %v", err)
		}
		if string(data[0:4]) != "ACTM" {
			t.Errorf("File should start with ACTM, got %q", string(data[0:4]))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 31: Recovery — persist and recover all messages
// ═══════════════════════════════════════════════════════════════════════════

func TestRecovery(t *testing.T) {
	t.Run("all messages are restored after persist and recover", func(t *testing.T) {
		dir := t.TempDir()
		ch := NewChannel("ch_001", "test-recover")

		ch.Append(NewTextMessage("sender", "message one", nil))
		ch.Append(NewTextMessage("sender", "message two", nil))
		ch.Append(NewBinaryMessage("sender", "image/png",
			[]byte{0x89, 0x50, 0x4E, 0x47}, nil))

		err := ch.Persist(dir)
		if err != nil {
			t.Fatalf("Persist failed: %v", err)
		}

		recovered, err := Recover(dir, "test-recover")
		if err != nil {
			t.Fatalf("Recover failed: %v", err)
		}
		if recovered.Length() != 3 {
			t.Fatalf("Recovered %d messages, want 3", recovered.Length())
		}

		msgs := recovered.Read(0, 3)
		if msgs[0].PayloadText() != "message one" {
			t.Errorf("Message 0: got %q", msgs[0].PayloadText())
		}
		if msgs[1].PayloadText() != "message two" {
			t.Errorf("Message 1: got %q", msgs[1].PayloadText())
		}
		if !bytes.Equal(msgs[2].Payload(), []byte{0x89, 0x50, 0x4E, 0x47}) {
			t.Errorf("Message 2 binary payload mismatch")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 32: Recovery preserves order
// ═══════════════════════════════════════════════════════════════════════════

func TestRecoveryPreservesOrder(t *testing.T) {
	t.Run("100 messages are recovered in order", func(t *testing.T) {
		dir := t.TempDir()
		ch := NewChannel("ch_001", "order-test")

		for i := 0; i < 100; i++ {
			ch.Append(NewTextMessage("sender",
				string(rune('A'+(i%26))), nil))
		}

		err := ch.Persist(dir)
		if err != nil {
			t.Fatalf("Persist failed: %v", err)
		}

		recovered, err := Recover(dir, "order-test")
		if err != nil {
			t.Fatalf("Recover failed: %v", err)
		}
		if recovered.Length() != 100 {
			t.Fatalf("Recovered %d messages, want 100", recovered.Length())
		}

		msgs := recovered.Read(0, 100)
		for i, msg := range msgs {
			expected := string(rune('A' + (i % 26)))
			if msg.PayloadText() != expected {
				t.Errorf("Message %d: got %q, want %q", i, msg.PayloadText(), expected)
			}
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 33: Empty channel recovery
// ═══════════════════════════════════════════════════════════════════════════

func TestEmptyChannelRecovery(t *testing.T) {
	t.Run("recovering non-existent file returns empty channel", func(t *testing.T) {
		dir := t.TempDir()
		ch, err := Recover(dir, "nonexistent")
		if err != nil {
			t.Fatalf("Recover failed: %v", err)
		}
		if ch.Length() != 0 {
			t.Errorf("Expected empty channel, got %d messages", ch.Length())
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 34: Mixed content recovery
// ═══════════════════════════════════════════════════════════════════════════

func TestMixedContentRecovery(t *testing.T) {
	t.Run("text, JSON, and binary messages all recover correctly", func(t *testing.T) {
		dir := t.TempDir()
		ch := NewChannel("ch_001", "mixed")

		ch.Append(NewTextMessage("sender", "plain text", nil))
		ch.Append(NewJSONMessage("sender", map[string]string{"key": "val"}, nil))
		ch.Append(NewBinaryMessage("sender", "image/png",
			[]byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}, nil))

		if err := ch.Persist(dir); err != nil {
			t.Fatalf("Persist failed: %v", err)
		}

		recovered, err := Recover(dir, "mixed")
		if err != nil {
			t.Fatalf("Recover failed: %v", err)
		}
		if recovered.Length() != 3 {
			t.Fatalf("Recovered %d messages, want 3", recovered.Length())
		}

		msgs := recovered.Read(0, 3)

		// Text message.
		if msgs[0].ContentType() != "text/plain" {
			t.Errorf("Msg 0 content type: %q", msgs[0].ContentType())
		}
		if msgs[0].PayloadText() != "plain text" {
			t.Errorf("Msg 0 payload: %q", msgs[0].PayloadText())
		}

		// JSON message.
		if msgs[1].ContentType() != "application/json" {
			t.Errorf("Msg 1 content type: %q", msgs[1].ContentType())
		}
		parsed, err := msgs[1].PayloadJSON()
		if err != nil {
			t.Fatalf("Msg 1 PayloadJSON: %v", err)
		}
		m := parsed.(map[string]interface{})
		if m["key"] != "val" {
			t.Errorf("Msg 1 JSON key: got %v", m["key"])
		}

		// Binary message.
		if msgs[2].ContentType() != "image/png" {
			t.Errorf("Msg 2 content type: %q", msgs[2].ContentType())
		}
		expectedPNG := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		if !bytes.Equal(msgs[2].Payload(), expectedPNG) {
			t.Errorf("Msg 2 PNG payload mismatch")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 35: Truncated write recovery
// ═══════════════════════════════════════════════════════════════════════════

func TestTruncatedWriteRecovery(t *testing.T) {
	t.Run("truncated file recovers complete messages only", func(t *testing.T) {
		dir := t.TempDir()
		ch := NewChannel("ch_001", "truncated")

		ch.Append(NewTextMessage("sender", "complete message 1", nil))
		ch.Append(NewTextMessage("sender", "complete message 2", nil))
		ch.Append(NewTextMessage("sender", "this will be truncated", nil))

		if err := ch.Persist(dir); err != nil {
			t.Fatalf("Persist failed: %v", err)
		}

		// Simulate a crash by truncating the file mid-message.
		path := filepath.Join(dir, "truncated.log")
		data, _ := os.ReadFile(path)

		// Remove the last 10 bytes to corrupt the third message.
		truncated := data[:len(data)-10]
		if err := os.WriteFile(path, truncated, 0o644); err != nil {
			t.Fatalf("Failed to write truncated file: %v", err)
		}

		recovered, err := Recover(dir, "truncated")
		if err != nil {
			t.Fatalf("Recover failed: %v", err)
		}

		// Should have recovered the first 2 complete messages.
		if recovered.Length() != 2 {
			t.Fatalf("Recovered %d messages, want 2", recovered.Length())
		}
		msgs := recovered.Read(0, 2)
		if msgs[0].PayloadText() != "complete message 1" {
			t.Errorf("Msg 0: got %q", msgs[0].PayloadText())
		}
		if msgs[1].PayloadText() != "complete message 2" {
			t.Errorf("Msg 1: got %q", msgs[1].PayloadText())
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 36: Read returns copies (not references to internal log)
// ═══════════════════════════════════════════════════════════════════════════

func TestReadReturnsCopies(t *testing.T) {
	t.Run("modifying returned slice does not affect channel", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		ch.Append(NewTextMessage("sender", "hello", nil))
		ch.Append(NewTextMessage("sender", "world", nil))

		msgs := ch.Read(0, 2)
		// Set an element to nil — should not affect the internal log.
		msgs[0] = nil

		msgs2 := ch.Read(0, 2)
		if msgs2[0] == nil {
			t.Error("Internal log was corrupted by modifying returned slice")
		}
	})
}
