package cpusimulator

import (
	"testing"
)

func TestRegisterReadWrite(t *testing.T) {
	regs := NewRegisterFile(4, 16) // 16-bit
	regs.Write(0, 0xFFFF)
	if regs.Read(0) != 0xFFFF {
		t.Errorf("Read failed")
	}
	
	// test wrap
	regs.Write(1, 0xFF0000)
	if regs.Read(1) != 0 {
		t.Errorf("Wrap failed, got %x", regs.Read(1))
	}
}

func TestRegisterDump(t *testing.T) {
	regs := NewRegisterFile(2, 8)
	regs.Write(1, 42)
	dump := regs.Dump()
	if dump["R1"] != 42 {
		t.Errorf("Dump failed")
	}
}

func TestRegisterBoundsRead(t *testing.T) {
	regs := NewRegisterFile(2, 8)
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Read(2) should panic")
		}
	}()
	regs.Read(2)
}

func TestRegisterBoundsWrite(t *testing.T) {
	regs := NewRegisterFile(2, 8)
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Write(2) should panic")
		}
	}()
	regs.Write(2, 1)
}
