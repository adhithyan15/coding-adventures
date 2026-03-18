package cpusimulator

import (
    "testing"
)

type mockDecoder struct {}
func (m *mockDecoder) Decode(rawInstruction uint32, pc int) DecodeResult {
    return DecodeResult{
        Mnemonic: "mock",
        Fields: map[string]int{"op": int(rawInstruction & 0xFF)},
        RawInstruction: rawInstruction,
    }
}

type mockExecutor struct {}
func (m *mockExecutor) Execute(decoded DecodeResult, registers *RegisterFile, memory *Memory, pc int) ExecuteResult {
    op := decoded.Fields["op"]
    switch op {
    case 1:
        registers.Write(1, 42)
        return ExecuteResult{Description: "write reg 1", NextPC: pc + 4, Halted: false}
    case 2:
        memory.WriteByte(10, 255)
        return ExecuteResult{Description: "write mem 10", NextPC: pc + 4, Halted: false}
    case 0:
        return ExecuteResult{Description: "halt", NextPC: pc, Halted: true}
    }
    return ExecuteResult{Description: "nop", NextPC: pc + 4, Halted: false}
}

func TestCPU_Pipeline(t *testing.T) {
    decoder := &mockDecoder{}
    executor := &mockExecutor{}
    cpu := NewCPU(decoder, executor, 4, 32, 128)
    
    // Program: OP=1 (writes reg), OP=2 (writes mem), OP=0 (halts)
    program := []byte{
        1, 0, 0, 0,
        2, 0, 0, 0,
        0, 0, 0, 0,
    }
    
    cpu.LoadProgram(program, 0)
    
    traces := cpu.Run(10)
    if len(traces) != 3 {
        t.Fatalf("Expected 3 instructions, got %d", len(traces))
    }
    
    if cpu.Registers.Read(1) != 42 {
        t.Errorf("Register 1 should be 42")
    }
    
    if cpu.Memory.ReadByte(10) != 255 {
        t.Errorf("Memory byte 10 should be 255")
    }
    
    if cpu.PC != 8 {
        t.Errorf("PC should halt at 8")
    }
}

func TestMemory_Endianness(t *testing.T) {
    mem := NewMemory(16)
    mem.WriteWord(0, 0x12345678)
    
    // Little-endian means LSB (78) is at addr 0
    if mem.ReadByte(0) != 0x78 { t.Errorf("expected 0x78") }
    if mem.ReadByte(1) != 0x56 { t.Errorf("expected 0x56") }
    if mem.ReadByte(2) != 0x34 { t.Errorf("expected 0x34") }
    if mem.ReadByte(3) != 0x12 { t.Errorf("expected 0x12") }
    
    if mem.ReadWord(0) != 0x12345678 {
        t.Errorf("read word failed")
    }
}

func TestRegisterFile_Boundaries(t *testing.T) {
    regs := NewRegisterFile(4, 8) // 8-bit registers
    
    // Test write overflow protection
    regs.Write(0, 256) // 256 is 0x100, which overflows 8 bits (max 255)
    if regs.Read(0) != 0 {
        t.Errorf("Expected overflow to be masked to 0, got %d", regs.Read(0))
    }
    
    regs.Write(1, 255)
    if regs.Read(1) != 255 {
        t.Errorf("Expected 255")
    }
}
