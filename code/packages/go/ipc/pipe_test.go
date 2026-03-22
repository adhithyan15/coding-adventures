package ipc

import (
	"bytes"
	"errors"
	"testing"
)

// ========================================================================
// Basic write/read
// ========================================================================

func TestPipeWriteAndRead(t *testing.T) {
	// Write "hello", read 5 bytes, get "hello" back.
	p := NewPipe(64)
	n, err := p.Write([]byte("hello"))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if n != 5 {
		t.Errorf("expected 5 bytes written, got %d", n)
	}
	data := p.Read(5)
	if !bytes.Equal(data, []byte("hello")) {
		t.Errorf("expected 'hello', got %q", data)
	}
}

func TestPipeFIFOOrdering(t *testing.T) {
	// Data comes out in the same order it went in.
	p := NewPipe(64)
	p.Write([]byte("abc"))
	p.Write([]byte("def"))
	data := p.Read(6)
	if !bytes.Equal(data, []byte("abcdef")) {
		t.Errorf("expected 'abcdef', got %q", data)
	}
}

func TestPipeMultipleReads(t *testing.T) {
	p := NewPipe(64)
	p.Write([]byte("hello world"))
	if d := p.Read(5); !bytes.Equal(d, []byte("hello")) {
		t.Errorf("expected 'hello', got %q", d)
	}
	if d := p.Read(1); !bytes.Equal(d, []byte(" ")) {
		t.Errorf("expected ' ', got %q", d)
	}
	if d := p.Read(5); !bytes.Equal(d, []byte("world")) {
		t.Errorf("expected 'world', got %q", d)
	}
}

// ========================================================================
// Partial reads and writes
// ========================================================================

func TestPipePartialRead(t *testing.T) {
	p := NewPipe(64)
	p.Write([]byte("hello world"))
	data := p.Read(5)
	if !bytes.Equal(data, []byte("hello")) {
		t.Errorf("expected 'hello', got %q", data)
	}
	if p.Available() != 6 {
		t.Errorf("expected 6 available, got %d", p.Available())
	}
}

func TestPipeReadMoreThanAvailable(t *testing.T) {
	p := NewPipe(64)
	p.Write([]byte("hi"))
	data := p.Read(100)
	if !bytes.Equal(data, []byte("hi")) {
		t.Errorf("expected 'hi', got %q", data)
	}
}

func TestPipeWriteMoreThanSpace(t *testing.T) {
	p := NewPipe(8)
	n, _ := p.Write([]byte("0123456789")) // 10 bytes, only 8 fit
	if n != 8 {
		t.Errorf("expected 8 written, got %d", n)
	}
	if !p.IsFull() {
		t.Error("expected pipe to be full")
	}
}

func TestPipeReadEmptyReturnsNil(t *testing.T) {
	p := NewPipe(64)
	data := p.Read(10)
	if data != nil {
		t.Errorf("expected nil, got %q", data)
	}
}

// ========================================================================
// Circular buffer wrapping
// ========================================================================

func TestPipeCircularWrapAround(t *testing.T) {
	// Buffer capacity=8.
	// 1. Write "abcde" (5 bytes)
	// 2. Read 3 bytes ("abc")
	// 3. Write "fghij" (5 bytes, wraps)
	// 4. Read 7 bytes => "defghij"
	p := NewPipe(8)
	p.Write([]byte("abcde"))
	p.Read(3) // discard "abc"

	n, _ := p.Write([]byte("fghij"))
	if n != 5 {
		t.Errorf("expected 5 written, got %d", n)
	}
	if p.Available() != 7 {
		t.Errorf("expected 7 available, got %d", p.Available())
	}

	data := p.Read(7)
	if !bytes.Equal(data, []byte("defghij")) {
		t.Errorf("expected 'defghij', got %q", data)
	}
}

func TestPipeFillAndDrainRepeatedly(t *testing.T) {
	p := NewPipe(4)
	for i := 0; i < 10; i++ {
		p.Write([]byte("abcd"))
		if !p.IsFull() {
			t.Errorf("iteration %d: expected full", i)
		}
		data := p.Read(4)
		if !bytes.Equal(data, []byte("abcd")) {
			t.Errorf("iteration %d: expected 'abcd', got %q", i, data)
		}
		if !p.IsEmpty() {
			t.Errorf("iteration %d: expected empty", i)
		}
	}
}

// ========================================================================
// EOF and BrokenPipe
// ========================================================================

func TestPipeEOFWhenWritersClose(t *testing.T) {
	p := NewPipe(64)
	p.Write([]byte("last data"))
	p.CloseWrite()

	if p.IsEOF() {
		t.Error("should not be EOF while data remains")
	}
	data := p.Read(9)
	if !bytes.Equal(data, []byte("last data")) {
		t.Errorf("expected 'last data', got %q", data)
	}
	if !p.IsEOF() {
		t.Error("expected EOF after draining buffer with no writers")
	}
}

func TestPipeEOFEmptyNoWriters(t *testing.T) {
	p := NewPipe(64)
	p.CloseWrite()
	if !p.IsEOF() {
		t.Error("expected EOF on empty pipe with no writers")
	}
}

func TestPipeBrokenPipeError(t *testing.T) {
	p := NewPipe(64)
	p.CloseRead()
	_, err := p.Write([]byte("nobody home"))
	if !errors.Is(err, ErrBrokenPipe) {
		t.Errorf("expected ErrBrokenPipe, got %v", err)
	}
}

func TestPipeWriteAfterCloseWrite(t *testing.T) {
	p := NewPipe(64)
	p.CloseWrite()
	_, err := p.Write([]byte("too late"))
	if !errors.Is(err, ErrWriteEndClosed) {
		t.Errorf("expected ErrWriteEndClosed, got %v", err)
	}
}

func TestPipeReadAfterCloseRead(t *testing.T) {
	p := NewPipe(64)
	p.Write([]byte("data"))
	p.CloseRead()
	data := p.Read(4)
	if data != nil {
		t.Errorf("expected nil after close_read, got %q", data)
	}
}

// ========================================================================
// Properties
// ========================================================================

func TestPipeIsEmpty(t *testing.T) {
	p := NewPipe(64)
	if !p.IsEmpty() {
		t.Error("new pipe should be empty")
	}
	p.Write([]byte("x"))
	if p.IsEmpty() {
		t.Error("pipe with data should not be empty")
	}
}

func TestPipeIsFull(t *testing.T) {
	p := NewPipe(4)
	if p.IsFull() {
		t.Error("new pipe should not be full")
	}
	p.Write([]byte("abcd"))
	if !p.IsFull() {
		t.Error("pipe at capacity should be full")
	}
}

func TestPipeAvailable(t *testing.T) {
	p := NewPipe(64)
	if p.Available() != 0 {
		t.Errorf("expected 0, got %d", p.Available())
	}
	p.Write([]byte("hello"))
	if p.Available() != 5 {
		t.Errorf("expected 5, got %d", p.Available())
	}
	p.Read(2)
	if p.Available() != 3 {
		t.Errorf("expected 3, got %d", p.Available())
	}
}

func TestPipeSpace(t *testing.T) {
	p := NewPipe(8)
	if p.Space() != 8 {
		t.Errorf("expected 8, got %d", p.Space())
	}
	p.Write([]byte("abc"))
	if p.Space() != 5 {
		t.Errorf("expected 5, got %d", p.Space())
	}
}

func TestPipeCapacity(t *testing.T) {
	p := NewPipe(1024)
	if p.Capacity() != 1024 {
		t.Errorf("expected 1024, got %d", p.Capacity())
	}
}

func TestPipeIsEOFWithActiveWriters(t *testing.T) {
	p := NewPipe(64)
	if p.IsEOF() {
		t.Error("empty pipe with active writers should not be EOF")
	}
}

// ========================================================================
// Edge cases
// ========================================================================

func TestPipeWriteZeroBytes(t *testing.T) {
	p := NewPipe(64)
	n, err := p.Write([]byte{})
	if err != nil {
		t.Errorf("unexpected error: %v", err)
	}
	if n != 0 {
		t.Errorf("expected 0, got %d", n)
	}
}

func TestPipeReadZeroBytes(t *testing.T) {
	p := NewPipe(64)
	p.Write([]byte("data"))
	data := p.Read(0)
	if data != nil {
		t.Errorf("expected nil, got %q", data)
	}
}

func TestPipeCapacityOne(t *testing.T) {
	p := NewPipe(1)
	n, _ := p.Write([]byte("a"))
	if n != 1 {
		t.Errorf("expected 1, got %d", n)
	}
	if !p.IsFull() {
		t.Error("should be full")
	}
	n2, _ := p.Write([]byte("b"))
	if n2 != 0 {
		t.Errorf("expected 0 (full), got %d", n2)
	}
	data := p.Read(1)
	if !bytes.Equal(data, []byte("a")) {
		t.Errorf("expected 'a', got %q", data)
	}
	if !p.IsEmpty() {
		t.Error("should be empty")
	}
}

func TestPipeWriteFullReturnsZero(t *testing.T) {
	p := NewPipe(4)
	p.Write([]byte("abcd"))
	n, _ := p.Write([]byte("x"))
	if n != 0 {
		t.Errorf("expected 0, got %d", n)
	}
}
