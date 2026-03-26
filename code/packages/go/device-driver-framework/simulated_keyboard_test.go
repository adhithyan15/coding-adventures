package devicedriverframework

import (
	"bytes"
	"testing"
)

func TestSimulatedKeyboardDefaults(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	if kb.Name != "keyboard0" {
		t.Errorf("Name = %q, want keyboard0", kb.Name)
	}
	if kb.Type != DeviceCharacter {
		t.Errorf("Type = %v, want CHARACTER", kb.Type)
	}
	if kb.Major != MajorKeyboard {
		t.Errorf("Major = %d, want %d", kb.Major, MajorKeyboard)
	}
	if kb.InterruptNumber != IntKeyboard {
		t.Errorf("InterruptNumber = %d, want %d", kb.InterruptNumber, IntKeyboard)
	}
}

func TestSimulatedKeyboardReadEmpty(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	buf := make([]byte, 10)
	n := kb.Read(buf)
	if n != 0 {
		t.Errorf("Read from empty buffer returned %d, want 0", n)
	}
}

func TestSimulatedKeyboardInjectAndRead(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	kb.InjectKeystrokes([]byte("Hello"))
	buf := make([]byte, 5)
	n := kb.Read(buf)
	if n != 5 {
		t.Errorf("Read returned %d, want 5", n)
	}
	if !bytes.Equal(buf[:n], []byte("Hello")) {
		t.Errorf("Read data = %q, want Hello", buf[:n])
	}
}

func TestSimulatedKeyboardReadPartial(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	kb.InjectKeystrokes([]byte("ABCDE"))
	buf := make([]byte, 3)
	n := kb.Read(buf)
	if n != 3 || !bytes.Equal(buf[:n], []byte("ABC")) {
		t.Errorf("First read = %q, want ABC", buf[:n])
	}
	buf2 := make([]byte, 10)
	n = kb.Read(buf2)
	if n != 2 || !bytes.Equal(buf2[:n], []byte("DE")) {
		t.Errorf("Second read = %q, want DE", buf2[:n])
	}
}

func TestSimulatedKeyboardReadMoreThanAvailable(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	kb.InjectKeystrokes([]byte("Hi"))
	buf := make([]byte, 100)
	n := kb.Read(buf)
	if n != 2 {
		t.Errorf("Read returned %d, want 2", n)
	}
}

func TestSimulatedKeyboardWriteReturnsNegative(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	if kb.Write([]byte("test")) != -1 {
		t.Error("Write should return -1 for keyboard")
	}
}

func TestSimulatedKeyboardInit(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	kb.InjectKeystrokes([]byte("leftover"))
	kb.Init()
	if !kb.Initialized {
		t.Error("Should be initialized after Init()")
	}
	buf := make([]byte, 10)
	if kb.Read(buf) != 0 {
		t.Error("Buffer should be empty after Init()")
	}
}

func TestSimulatedKeyboardBufferSize(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	if kb.BufferSize() != 0 {
		t.Errorf("BufferSize = %d, want 0", kb.BufferSize())
	}
	kb.InjectKeystrokes([]byte("ABC"))
	if kb.BufferSize() != 3 {
		t.Errorf("BufferSize = %d, want 3", kb.BufferSize())
	}
	buf := make([]byte, 1)
	kb.Read(buf)
	if kb.BufferSize() != 2 {
		t.Errorf("BufferSize = %d, want 2", kb.BufferSize())
	}
}

func TestSimulatedKeyboardMultipleInjections(t *testing.T) {
	kb := NewSimulatedKeyboard("keyboard0", 0)
	kb.InjectKeystrokes([]byte("AB"))
	kb.InjectKeystrokes([]byte("CD"))
	buf := make([]byte, 4)
	n := kb.Read(buf)
	if n != 4 || !bytes.Equal(buf[:n], []byte("ABCD")) {
		t.Errorf("Read = %q, want ABCD", buf[:n])
	}
}
