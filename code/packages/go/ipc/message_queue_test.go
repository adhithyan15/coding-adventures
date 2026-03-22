package ipc

import (
	"bytes"
	"testing"
)

// ========================================================================
// Basic send/receive
// ========================================================================

func TestMessageQueueSendAndReceive(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	if !mq.Send(1, []byte("hello")) {
		t.Fatal("send should succeed")
	}
	mt, data, ok := mq.Receive(0)
	if !ok {
		t.Fatal("receive should succeed")
	}
	if mt != 1 || !bytes.Equal(data, []byte("hello")) {
		t.Errorf("got (%d, %q), want (1, hello)", mt, data)
	}
}

func TestMessageQueueFIFOOrder(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	mq.Send(1, []byte("first"))
	mq.Send(1, []byte("second"))
	mq.Send(1, []byte("third"))

	_, d1, _ := mq.Receive(0)
	_, d2, _ := mq.Receive(0)
	_, d3, _ := mq.Receive(0)
	if !bytes.Equal(d1, []byte("first")) || !bytes.Equal(d2, []byte("second")) || !bytes.Equal(d3, []byte("third")) {
		t.Error("messages not in FIFO order")
	}
}

func TestMessageQueueReceiveEmptyReturnsNotOK(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	_, _, ok := mq.Receive(0)
	if ok {
		t.Error("receive from empty queue should return false")
	}
}

// ========================================================================
// Typed receive
// ========================================================================

func TestMessageQueueTypedReceive(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	mq.Send(1, []byte("req1"))
	mq.Send(2, []byte("status"))
	mq.Send(1, []byte("req2"))

	mt, data, ok := mq.Receive(2)
	if !ok || mt != 2 || !bytes.Equal(data, []byte("status")) {
		t.Errorf("expected (2, status, true), got (%d, %q, %v)", mt, data, ok)
	}
	if mq.MessageCount() != 2 {
		t.Errorf("expected 2 remaining, got %d", mq.MessageCount())
	}
}

func TestMessageQueueReceiveTypeZeroMeansAny(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	mq.Send(3, []byte("three"))
	mq.Send(1, []byte("one"))
	mt, _, _ := mq.Receive(0)
	if mt != 3 {
		t.Errorf("expected type 3 (oldest), got %d", mt)
	}
}

func TestMessageQueueReceiveNonexistentType(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	mq.Send(1, []byte("data"))
	_, _, ok := mq.Receive(99)
	if ok {
		t.Error("should not find type 99")
	}
	if mq.MessageCount() != 1 {
		t.Error("original message should be untouched")
	}
}

func TestMessageQueueTypedReceivePreservesOrder(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	mq.Send(1, []byte("a"))
	mq.Send(2, []byte("b"))
	mq.Send(1, []byte("c"))
	mq.Send(3, []byte("d"))

	mq.Receive(2) // remove type 2

	_, d1, _ := mq.Receive(0)
	_, d2, _ := mq.Receive(0)
	_, d3, _ := mq.Receive(0)
	if !bytes.Equal(d1, []byte("a")) || !bytes.Equal(d2, []byte("c")) || !bytes.Equal(d3, []byte("d")) {
		t.Error("remaining messages not in expected order")
	}
}

func TestMessageQueueReceiveOldestOfMatchingType(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	mq.Send(1, []byte("old"))
	mq.Send(1, []byte("new"))
	_, data, _ := mq.Receive(1)
	if !bytes.Equal(data, []byte("old")) {
		t.Errorf("expected 'old', got %q", data)
	}
}

// ========================================================================
// Limits
// ========================================================================

func TestMessageQueueFullRejectsSend(t *testing.T) {
	mq := NewMessageQueue(3, 4096)
	mq.Send(1, []byte("a"))
	mq.Send(1, []byte("b"))
	mq.Send(1, []byte("c"))
	if mq.Send(1, []byte("d")) {
		t.Error("should reject when full")
	}
	if mq.MessageCount() != 3 {
		t.Errorf("expected 3, got %d", mq.MessageCount())
	}
}

func TestMessageQueueOversizedRejected(t *testing.T) {
	mq := NewMessageQueue(256, 8)
	if mq.Send(1, []byte("this is too long")) {
		t.Error("should reject oversized message")
	}
}

func TestMessageQueueExactlyMaxSizeAccepted(t *testing.T) {
	mq := NewMessageQueue(256, 5)
	if !mq.Send(1, []byte("exact")) {
		t.Error("exactly max size should be accepted")
	}
}

func TestMessageQueueInvalidTypeZero(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	if mq.Send(0, []byte("data")) {
		t.Error("type 0 should be rejected")
	}
}

func TestMessageQueueInvalidTypeNegative(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	if mq.Send(-1, []byte("data")) {
		t.Error("negative type should be rejected")
	}
}

// ========================================================================
// Properties
// ========================================================================

func TestMessageQueueMessageCount(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	if mq.MessageCount() != 0 {
		t.Error("new queue should have 0 messages")
	}
	mq.Send(1, []byte("a"))
	if mq.MessageCount() != 1 {
		t.Errorf("expected 1, got %d", mq.MessageCount())
	}
	mq.Send(2, []byte("b"))
	if mq.MessageCount() != 2 {
		t.Errorf("expected 2, got %d", mq.MessageCount())
	}
	mq.Receive(0)
	if mq.MessageCount() != 1 {
		t.Errorf("expected 1, got %d", mq.MessageCount())
	}
}

func TestMessageQueueIsEmpty(t *testing.T) {
	mq := NewMessageQueue(256, 4096)
	if !mq.IsEmpty() {
		t.Error("new queue should be empty")
	}
	mq.Send(1, []byte("x"))
	if mq.IsEmpty() {
		t.Error("queue with message should not be empty")
	}
}

func TestMessageQueueIsFull(t *testing.T) {
	mq := NewMessageQueue(2, 4096)
	if mq.IsFull() {
		t.Error("new queue should not be full")
	}
	mq.Send(1, []byte("a"))
	if mq.IsFull() {
		t.Error("1/2 should not be full")
	}
	mq.Send(1, []byte("b"))
	if !mq.IsFull() {
		t.Error("2/2 should be full")
	}
}
