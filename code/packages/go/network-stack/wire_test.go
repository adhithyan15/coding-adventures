package networkstack

import (
	"bytes"
	"testing"
)

func TestWireSendAReceiveB(t *testing.T) {
	w := NewNetworkWire()
	w.SendA([]byte("Hello from A"))
	if !w.HasDataForB() {
		t.Error("should have data for B")
	}
	got := w.ReceiveB()
	if !bytes.Equal(got, []byte("Hello from A")) {
		t.Errorf("got %q", string(got))
	}
}

func TestWireSendBReceiveA(t *testing.T) {
	w := NewNetworkWire()
	w.SendB([]byte("Hello from B"))
	if !w.HasDataForA() {
		t.Error("should have data for A")
	}
	got := w.ReceiveA()
	if !bytes.Equal(got, []byte("Hello from B")) {
		t.Errorf("got %q", string(got))
	}
}

func TestWireEmptyReceive(t *testing.T) {
	w := NewNetworkWire()
	if w.ReceiveA() != nil {
		t.Error("expected nil")
	}
	if w.ReceiveB() != nil {
		t.Error("expected nil")
	}
}

func TestWireInitiallyEmpty(t *testing.T) {
	w := NewNetworkWire()
	if w.HasDataForA() || w.HasDataForB() {
		t.Error("should be empty initially")
	}
}

func TestWireFIFO(t *testing.T) {
	w := NewNetworkWire()
	w.SendA([]byte("first"))
	w.SendA([]byte("second"))
	w.SendA([]byte("third"))

	if string(w.ReceiveB()) != "first" {
		t.Error("FIFO order wrong")
	}
	if string(w.ReceiveB()) != "second" {
		t.Error("FIFO order wrong")
	}
	if string(w.ReceiveB()) != "third" {
		t.Error("FIFO order wrong")
	}
	if w.ReceiveB() != nil {
		t.Error("should be empty")
	}
}

func TestWireFullDuplex(t *testing.T) {
	w := NewNetworkWire()
	w.SendA([]byte("from A"))
	w.SendB([]byte("from B"))

	if string(w.ReceiveB()) != "from A" {
		t.Error("A->B failed")
	}
	if string(w.ReceiveA()) != "from B" {
		t.Error("B->A failed")
	}
}

func TestWireIndependentQueues(t *testing.T) {
	w := NewNetworkWire()
	w.SendA([]byte("data"))

	if !w.HasDataForB() {
		t.Error("B should have data")
	}
	if w.HasDataForA() {
		t.Error("A should not have data")
	}
	if w.ReceiveA() != nil {
		t.Error("A receive should be nil")
	}
	if !w.HasDataForB() {
		t.Error("B should still have data")
	}
}

func TestWireLargeFrame(t *testing.T) {
	w := NewNetworkWire()
	large := make([]byte, 10000)
	for i := range large {
		large[i] = 0xAA
	}
	w.SendA(large)
	got := w.ReceiveB()
	if !bytes.Equal(got, large) {
		t.Error("large frame corrupted")
	}
}
