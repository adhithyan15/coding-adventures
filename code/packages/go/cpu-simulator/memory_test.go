package cpusimulator

import (
	"reflect"
	"testing"
)

func TestNewMemoryPanic(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("NewMemory(0) should panic")
		}
	}()
	NewMemory(0)
}

func TestMemoryBoundsPanic(t *testing.T) {
	mem := NewMemory(10)
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("ReadByte(10) should panic")
		}
	}()
	mem.ReadByte(10)
}

func TestMemoryReadWrite(t *testing.T) {
	mem := NewMemory(16)
	mem.WriteByte(5, 255)
	if mem.ReadByte(5) != 255 {
		t.Errorf("ReadByte failed")
	}
	
	mem.LoadBytes(0, []byte{1,2,3})
	if mem.ReadByte(1) != 2 {
		t.Errorf("LoadBytes failed")
	}

	dump := mem.Dump(0, 3)
	if !reflect.DeepEqual(dump, []byte{1,2,3}) {
		t.Errorf("Dump failed")
	}
}
