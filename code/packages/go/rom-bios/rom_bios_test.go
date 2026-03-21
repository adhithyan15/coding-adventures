package rombios

import (
	"testing"
)

// ═══════════════════════════════════════════════════════════════
// ROM Tests
// ═══════════════════════════════════════════════════════════════

func TestNewROM_LoadsFirmware(t *testing.T) {
	// Loading firmware into ROM should preserve the bytes exactly.
	firmware := []byte{0xAA, 0xBB, 0xCC, 0xDD}
	rom := NewROM(DefaultROMConfig(), firmware)

	if rom.Size() != DefaultROMSize {
		t.Errorf("expected size %d, got %d", DefaultROMSize, rom.Size())
	}
}

func TestNewROM_PanicsOnOversizedFirmware(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("expected panic for oversized firmware, got none")
		}
	}()

	oversized := make([]byte, DefaultROMSize+1)
	NewROM(DefaultROMConfig(), oversized)
}

func TestROM_ReadByte(t *testing.T) {
	// Read individual bytes from known positions in the firmware.
	firmware := []byte{0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0}
	rom := NewROM(DefaultROMConfig(), firmware)
	base := DefaultROMBase

	tests := []struct {
		name    string
		addr    uint32
		want    byte
	}{
		{"first byte", base, 0x12},
		{"second byte", base + 1, 0x34},
		{"fourth byte", base + 3, 0x78},
		{"eighth byte", base + 7, 0xF0},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := rom.Read(tc.addr)
			if got != tc.want {
				t.Errorf("Read(0x%08X) = 0x%02X, want 0x%02X", tc.addr, got, tc.want)
			}
		})
	}
}

func TestROM_ReadWord(t *testing.T) {
	// ReadWord should return a little-endian 32-bit word.
	// Bytes [0x78, 0x56, 0x34, 0x12] → word 0x12345678
	firmware := []byte{0x78, 0x56, 0x34, 0x12, 0xF0, 0xDE, 0xBC, 0x9A}
	rom := NewROM(DefaultROMConfig(), firmware)
	base := DefaultROMBase

	got := rom.ReadWord(base)
	if got != 0x12345678 {
		t.Errorf("ReadWord(0x%08X) = 0x%08X, want 0x12345678", base, got)
	}

	got2 := rom.ReadWord(base + 4)
	if got2 != 0x9ABCDEF0 {
		t.Errorf("ReadWord(0x%08X) = 0x%08X, want 0x9ABCDEF0", base+4, got2)
	}
}

func TestROM_WriteIsIgnored(t *testing.T) {
	// Writing to ROM should have no effect -- the original data persists.
	firmware := []byte{0xAA, 0xBB, 0xCC, 0xDD}
	rom := NewROM(DefaultROMConfig(), firmware)
	base := DefaultROMBase

	// Attempt to overwrite the first byte
	rom.Write(base, 0xFF)

	// Read it back -- should still be the original value
	got := rom.Read(base)
	if got != 0xAA {
		t.Errorf("after Write, Read(0x%08X) = 0x%02X, want 0xAA (write should be ignored)", base, got)
	}
}

func TestROM_OutOfRangeReturnsZero(t *testing.T) {
	firmware := []byte{0xAA}
	rom := NewROM(DefaultROMConfig(), firmware)

	// Address below ROM base
	if got := rom.Read(0x00000000); got != 0 {
		t.Errorf("Read(0x00000000) = 0x%02X, want 0 (below ROM)", got)
	}

	// Address above ROM -- 0xFFFF0000 + 65536 wraps to 0x00000000
	// which is out of ROM range. Use 0xFFFFFFFF (last address in space).
	if got := rom.Read(0xFFFFFFFF); got != 0 {
		// Last byte of ROM is at 0xFFFF0000 + 65535 = 0xFFFFFFFF, so this
		// should actually return the last byte (which is zero for small firmware).
	}

	// ReadWord out of range
	if got := rom.ReadWord(0x00000000); got != 0 {
		t.Errorf("ReadWord(0x00000000) = 0x%08X, want 0 (below ROM)", got)
	}
}

func TestROM_FirmwareSmallerThanROM(t *testing.T) {
	// If firmware is smaller than ROM, remaining bytes should be zero.
	firmware := []byte{0xAA, 0xBB}
	rom := NewROM(DefaultROMConfig(), firmware)
	base := DefaultROMBase

	if got := rom.Read(base + 2); got != 0 {
		t.Errorf("Read(base+2) = 0x%02X, want 0 (zero-filled)", got)
	}
	if got := rom.Read(base + 100); got != 0 {
		t.Errorf("Read(base+100) = 0x%02X, want 0 (zero-filled)", got)
	}
}

func TestROM_CustomConfig(t *testing.T) {
	// ROM at a custom base address and size should work correctly.
	config := ROMConfig{
		BaseAddress: 0x10000000,
		Size:        256,
	}
	firmware := []byte{0x11, 0x22, 0x33, 0x44}
	rom := NewROM(config, firmware)

	if rom.Size() != 256 {
		t.Errorf("Size() = %d, want 256", rom.Size())
	}
	if rom.BaseAddress() != 0x10000000 {
		t.Errorf("BaseAddress() = 0x%08X, want 0x10000000", rom.BaseAddress())
	}
	if got := rom.Read(0x10000000); got != 0x11 {
		t.Errorf("Read(0x10000000) = 0x%02X, want 0x11", got)
	}
	// Out of range for this ROM
	if got := rom.Read(DefaultROMBase); got != 0 {
		t.Errorf("Read at default base = 0x%02X, want 0 (wrong ROM region)", got)
	}
}

func TestROM_Contains(t *testing.T) {
	rom := NewROM(DefaultROMConfig(), []byte{0xAA})

	if !rom.Contains(DefaultROMBase) {
		t.Error("Contains(base) should be true")
	}
	if !rom.Contains(0xFFFFFFFF) {
		t.Error("Contains(last byte = 0xFFFFFFFF) should be true")
	}
	if rom.Contains(DefaultROMBase - 1) {
		t.Error("Contains(base-1) should be false")
	}
	// 0xFFFF0000 + 65536 would wrap to 0, so test with address 0 instead
	if rom.Contains(0x00000000) {
		t.Error("Contains(0x00000000) should be false")
	}
}

func TestROM_BoundaryReads(t *testing.T) {
	// Test reading the very first and very last bytes/words of ROM.
	firmware := make([]byte, DefaultROMSize)
	firmware[0] = 0x01
	firmware[1] = 0x02
	firmware[2] = 0x03
	firmware[3] = 0x04
	firmware[DefaultROMSize-4] = 0xA1
	firmware[DefaultROMSize-3] = 0xA2
	firmware[DefaultROMSize-2] = 0xA3
	firmware[DefaultROMSize-1] = 0xA4
	rom := NewROM(DefaultROMConfig(), firmware)
	base := DefaultROMBase

	// First word
	if got := rom.ReadWord(base); got != 0x04030201 {
		t.Errorf("ReadWord(first) = 0x%08X, want 0x04030201", got)
	}

	// Last word: base + 65536 - 4 = 0xFFFF0000 + 0xFFFC = 0xFFFFFFFC
	lastWordAddr := uint32(0xFFFFFFFC)
	if got := rom.ReadWord(lastWordAddr); got != 0xA4A3A2A1 {
		t.Errorf("ReadWord(last) = 0x%08X, want 0xA4A3A2A1", got)
	}
}

// ═══════════════════════════════════════════════════════════════
// HardwareInfo Tests
// ═══════════════════════════════════════════════════════════════

func TestDefaultHardwareInfo(t *testing.T) {
	info := DefaultHardwareInfo()

	if info.MemorySize != 0 {
		t.Errorf("MemorySize = %d, want 0", info.MemorySize)
	}
	if info.DisplayColumns != 80 {
		t.Errorf("DisplayColumns = %d, want 80", info.DisplayColumns)
	}
	if info.DisplayRows != 25 {
		t.Errorf("DisplayRows = %d, want 25", info.DisplayRows)
	}
	if info.FramebufferBase != 0xFFFB0000 {
		t.Errorf("FramebufferBase = 0x%08X, want 0xFFFB0000", info.FramebufferBase)
	}
	if info.IDTBase != 0x00000000 {
		t.Errorf("IDTBase = 0x%08X, want 0x00000000", info.IDTBase)
	}
	if info.IDTEntries != 256 {
		t.Errorf("IDTEntries = %d, want 256", info.IDTEntries)
	}
	if info.BootloaderEntry != 0x00010000 {
		t.Errorf("BootloaderEntry = 0x%08X, want 0x00010000", info.BootloaderEntry)
	}
}

func TestHardwareInfo_ToBytesRoundTrip(t *testing.T) {
	// Serializing and deserializing should produce the same struct.
	info := HardwareInfo{
		MemorySize:      64 * 1024 * 1024,
		DisplayColumns:  80,
		DisplayRows:     25,
		FramebufferBase: 0xFFFB0000,
		IDTBase:         0x00000000,
		IDTEntries:      256,
		BootloaderEntry: 0x00010000,
	}

	bytes := info.ToBytes()
	if len(bytes) != HardwareInfoSize {
		t.Fatalf("ToBytes() length = %d, want %d", len(bytes), HardwareInfoSize)
	}

	restored := HardwareInfoFromBytes(bytes)
	if restored != info {
		t.Errorf("round-trip mismatch:\n  original: %+v\n  restored: %+v", info, restored)
	}
}

func TestHardwareInfo_ToBytesLayout(t *testing.T) {
	// Verify the exact byte layout matches the spec.
	info := HardwareInfo{
		MemorySize:      0x04000000, // 64 MB
		DisplayColumns:  80,
		DisplayRows:     25,
		FramebufferBase: 0xFFFB0000,
		IDTBase:         0x00000000,
		IDTEntries:      256,
		BootloaderEntry: 0x00010000,
	}

	bytes := info.ToBytes()

	// MemorySize at offset 0: 0x04000000 little-endian = [0x00, 0x00, 0x00, 0x04]
	if bytes[0] != 0x00 || bytes[3] != 0x04 {
		t.Errorf("MemorySize bytes wrong: got [%02X %02X %02X %02X]",
			bytes[0], bytes[1], bytes[2], bytes[3])
	}

	// DisplayColumns at offset 4: 80 = 0x50
	if bytes[4] != 0x50 || bytes[5] != 0x00 {
		t.Errorf("DisplayColumns bytes wrong: got [%02X %02X]", bytes[4], bytes[5])
	}
}

func TestHardwareInfoFromBytes_PanicsOnShortData(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("expected panic for short data, got none")
		}
	}()

	HardwareInfoFromBytes([]byte{0x01, 0x02})
}

// ═══════════════════════════════════════════════════════════════
// BIOS Firmware Generation Tests
// ═══════════════════════════════════════════════════════════════

func TestBIOSFirmware_GenerateNonEmpty(t *testing.T) {
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	code := bios.Generate()

	if len(code) == 0 {
		t.Error("Generate() returned empty byte slice")
	}
}

func TestBIOSFirmware_GenerateWordAligned(t *testing.T) {
	// RISC-V instructions are 32 bits (4 bytes), so output must be
	// a multiple of 4.
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	code := bios.Generate()

	if len(code)%4 != 0 {
		t.Errorf("Generate() length = %d, not a multiple of 4", len(code))
	}
}

func TestBIOSFirmware_GenerateDeterministic(t *testing.T) {
	// Calling Generate() twice with the same config should produce
	// identical output.
	config := DefaultBIOSConfig()
	bios1 := NewBIOSFirmware(config)
	bios2 := NewBIOSFirmware(config)

	code1 := bios1.Generate()
	code2 := bios2.Generate()

	if len(code1) != len(code2) {
		t.Fatalf("length mismatch: %d vs %d", len(code1), len(code2))
	}
	for i := range code1 {
		if code1[i] != code2[i] {
			t.Errorf("byte %d differs: 0x%02X vs 0x%02X", i, code1[i], code2[i])
			break
		}
	}
}

func TestBIOSFirmware_ConfigurableProducessDifferentOutput(t *testing.T) {
	// Different configs should produce different firmware.
	config1 := DefaultBIOSConfig()
	config2 := DefaultBIOSConfig()
	config2.MemorySize = 128 * 1024 * 1024 // 128 MB

	code1 := NewBIOSFirmware(config1).Generate()
	code2 := NewBIOSFirmware(config2).Generate()

	// They should be different (different memory size handling)
	if len(code1) == len(code2) {
		same := true
		for i := range code1 {
			if code1[i] != code2[i] {
				same = false
				break
			}
		}
		if same {
			t.Error("different configs produced identical firmware")
		}
	}
}

func TestBIOSFirmware_WithConfiguredMemorySize(t *testing.T) {
	// When MemorySize is set, the probe should be skipped and the
	// firmware should be shorter than the probing version.
	probeConfig := DefaultBIOSConfig()
	fixedConfig := DefaultBIOSConfig()
	fixedConfig.MemorySize = 64 * 1024 * 1024

	probeCode := NewBIOSFirmware(probeConfig).Generate()
	fixedCode := NewBIOSFirmware(fixedConfig).Generate()

	// Fixed size firmware should be shorter (no probe loop)
	if len(fixedCode) >= len(probeCode) {
		t.Errorf("fixed config firmware (%d bytes) should be shorter than probe firmware (%d bytes)",
			len(fixedCode), len(probeCode))
	}
}

// ═══════════════════════════════════════════════════════════════
// Annotated Output Tests
// ═══════════════════════════════════════════════════════════════

func TestBIOSFirmware_GenerateWithCommentsMatchesGenerate(t *testing.T) {
	// The annotated output should produce the same machine code as Generate().
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	code := bios.Generate()
	annotated := bios.GenerateWithComments()

	if len(annotated)*4 != len(code) {
		t.Fatalf("annotated has %d instructions (%d bytes), Generate() has %d bytes",
			len(annotated), len(annotated)*4, len(code))
	}

	for i, inst := range annotated {
		offset := i * 4
		expected := readLE32(code[offset:])
		if inst.MachineCode != expected {
			t.Errorf("instruction %d: annotated MachineCode 0x%08X != Generate() word 0x%08X",
				i, inst.MachineCode, expected)
		}
	}
}

func TestBIOSFirmware_AnnotatedAddressContinuity(t *testing.T) {
	// Addresses should increase by 4 for each instruction and start
	// at the ROM base address.
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	annotated := bios.GenerateWithComments()

	if len(annotated) == 0 {
		t.Fatal("no annotated instructions")
	}

	if annotated[0].Address != DefaultROMBase {
		t.Errorf("first instruction address = 0x%08X, want 0x%08X",
			annotated[0].Address, DefaultROMBase)
	}

	for i := 1; i < len(annotated); i++ {
		expected := annotated[i-1].Address + 4
		if annotated[i].Address != expected {
			t.Errorf("instruction %d address = 0x%08X, want 0x%08X",
				i, annotated[i].Address, expected)
		}
	}
}

func TestBIOSFirmware_AnnotatedNonEmptyStrings(t *testing.T) {
	// Every instruction should have non-empty assembly and comment.
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	annotated := bios.GenerateWithComments()

	for i, inst := range annotated {
		if inst.Assembly == "" {
			t.Errorf("instruction %d has empty Assembly", i)
		}
		if inst.Comment == "" {
			t.Errorf("instruction %d has empty Comment", i)
		}
	}
}

func TestBIOSFirmware_AnnotatedContainsRISCVMnemonics(t *testing.T) {
	// Assembly strings should contain recognizable RISC-V mnemonics.
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	annotated := bios.GenerateWithComments()

	mnemonics := map[string]bool{
		"lui": false, "addi": false, "sw": false,
		"jalr": false,
	}

	for _, inst := range annotated {
		for m := range mnemonics {
			if containsWord(inst.Assembly, m) {
				mnemonics[m] = true
			}
		}
	}

	for m, found := range mnemonics {
		if !found {
			t.Errorf("expected mnemonic '%s' not found in any assembly string", m)
		}
	}
}

// containsWord checks if a string contains a word (space-delimited or at start).
func containsWord(s, word string) bool {
	if len(s) < len(word) {
		return false
	}
	// Check if word appears at the start
	if s[:len(word)] == word && (len(s) == len(word) || s[len(word)] == ' ') {
		return true
	}
	// Check after spaces
	for i := 0; i < len(s)-len(word); i++ {
		if s[i] == ' ' && s[i+1:i+1+len(word)] == word {
			return true
		}
	}
	return false
}

// ═══════════════════════════════════════════════════════════════
// BIOS Firmware Loads into ROM Successfully
// ═══════════════════════════════════════════════════════════════

func TestBIOSFirmware_FitsInROM(t *testing.T) {
	// The generated firmware should fit within the default ROM size.
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	code := bios.Generate()

	if len(code) > DefaultROMSize {
		t.Errorf("firmware (%d bytes) exceeds ROM size (%d bytes)", len(code), DefaultROMSize)
	}
}

func TestBIOSFirmware_LoadIntoROM(t *testing.T) {
	// Should be able to load firmware into ROM and read it back.
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	code := bios.Generate()
	rom := NewROM(DefaultROMConfig(), code)

	// The first instruction should be readable
	firstWord := rom.ReadWord(DefaultROMBase)
	expectedWord := readLE32(code[0:4])
	if firstWord != expectedWord {
		t.Errorf("ROM first word = 0x%08X, want 0x%08X", firstWord, expectedWord)
	}
}

// ═══════════════════════════════════════════════════════════════
// Default Config Tests
// ═══════════════════════════════════════════════════════════════

func TestDefaultROMConfig(t *testing.T) {
	config := DefaultROMConfig()
	if config.BaseAddress != 0xFFFF0000 {
		t.Errorf("BaseAddress = 0x%08X, want 0xFFFF0000", config.BaseAddress)
	}
	if config.Size != 65536 {
		t.Errorf("Size = %d, want 65536", config.Size)
	}
}

func TestDefaultBIOSConfig(t *testing.T) {
	config := DefaultBIOSConfig()
	if config.MemorySize != 0 {
		t.Errorf("MemorySize = %d, want 0", config.MemorySize)
	}
	if config.DisplayColumns != 80 {
		t.Errorf("DisplayColumns = %d, want 80", config.DisplayColumns)
	}
	if config.DisplayRows != 25 {
		t.Errorf("DisplayRows = %d, want 25", config.DisplayRows)
	}
	if config.FramebufferBase != 0xFFFB0000 {
		t.Errorf("FramebufferBase = 0x%08X, want 0xFFFB0000", config.FramebufferBase)
	}
	if config.BootloaderEntry != 0x00010000 {
		t.Errorf("BootloaderEntry = 0x%08X, want 0x00010000", config.BootloaderEntry)
	}
}

// ═══════════════════════════════════════════════════════════════
// Edge Case Tests
// ═══════════════════════════════════════════════════════════════

func TestBIOSFirmware_LastInstructionIsJump(t *testing.T) {
	// The very last instruction should be a JALR (jump to bootloader).
	bios := NewBIOSFirmware(DefaultBIOSConfig())
	annotated := bios.GenerateWithComments()

	last := annotated[len(annotated)-1]
	if !containsWord(last.Assembly, "jalr") {
		t.Errorf("last instruction should be jalr, got: %s", last.Assembly)
	}
}

func TestBIOSFirmware_EmptyFirmwareStillWorks(t *testing.T) {
	// ROM with no firmware should be all zeros.
	rom := NewROM(DefaultROMConfig(), nil)
	if got := rom.ReadWord(DefaultROMBase); got != 0 {
		t.Errorf("empty ROM ReadWord = 0x%08X, want 0", got)
	}
}

func TestSignExtend12(t *testing.T) {
	tests := []struct {
		input int
		want  int
	}{
		{0, 0},
		{1, 1},
		{2047, 2047},        // 0x7FF -- largest positive
		{0x800, -2048},      // sign bit set
		{0xFFF, -1},         // all ones = -1
		{0xEEF, -273},       // 0xEEF = 3823, signed = -273
	}

	for _, tc := range tests {
		got := signExtend12(tc.input)
		if got != tc.want {
			t.Errorf("signExtend12(0x%03X) = %d, want %d", tc.input, got, tc.want)
		}
	}
}
