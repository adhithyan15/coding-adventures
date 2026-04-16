package ge225simulator

import "testing"

func ins(t *testing.T, opcode, address, modifier int) int {
	t.Helper()
	word, err := EncodeInstruction(opcode, modifier, address)
	if err != nil { t.Fatalf("encode failed: %v", err) }
	return word
}

func fixed(t *testing.T, mnemonic string) int {
	t.Helper()
	word, err := AssembleFixed(mnemonic)
	if err != nil { t.Fatalf("assemble fixed failed: %v", err) }
	return word
}

func shift(t *testing.T, mnemonic string, count int) int {
	t.Helper()
	word, err := AssembleShift(mnemonic, count)
	if err != nil { t.Fatalf("assemble shift failed: %v", err) }
	return word
}

func TestEncodeDecodeRoundTrip(t *testing.T) {
	word := ins(t, 0o01, 0x1234&0x1fff, 0o2)
	opcode, modifier, address := DecodeInstruction(word)
	if opcode != 0o01 || modifier != 0o2 || address != (0x1234&0x1fff) {
		t.Fatalf("unexpected decode: %o %o %o", opcode, modifier, address)
	}
	packed := PackWords([]int{word, fixed(t, "NOP")})
	unpacked, err := UnpackWords(packed)
	if err != nil { t.Fatalf("unpack failed: %v", err) }
	if len(unpacked) != 2 || unpacked[0] != word || unpacked[1] != fixed(t, "NOP") {
		t.Fatalf("unexpected unpacked words: %#v", unpacked)
	}
}

func TestLDAAddSTAProgram(t *testing.T) {
	sim := New(4096)
	if err := sim.LoadWords([]int{ins(t, 0o00, 10, 0), ins(t, 0o01, 11, 0), ins(t, 0o03, 12, 0), fixed(t, "NOP"), 0, 0, 0, 0, 0, 0, 1, 2, 0}, 0); err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if _, err := sim.Run(4); err != nil { t.Fatalf("run failed: %v", err) }
	state := sim.GetState()
	if state.A != 3 || state.Memory[12] != 3 { t.Fatalf("unexpected state: %+v", state) }
}

func TestSPBStoresP(t *testing.T) {
	sim := New(4096)
	if err := sim.LoadWords([]int{ins(t, 0o07, 4, 2), fixed(t, "NOP"), fixed(t, "NOP"), fixed(t, "NOP"), ins(t, 0o00, 10, 0), fixed(t, "NOP"), 0, 0, 0, 0, 0x12345}, 0); err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if _, err := sim.Run(3); err != nil { t.Fatalf("run failed: %v", err) }
	state := sim.GetState()
	if state.XWords[2] != 0 || state.A != 0x12345 { t.Fatalf("unexpected state: %+v", state) }
}

func TestOddAddressDoubleOps(t *testing.T) {
	sim := New(4096)
	_ = sim.WriteWord(11, 0x13579)
	if err := sim.LoadWords([]int{ins(t, 0o10, 11, 0), ins(t, 0o13, 13, 0), fixed(t, "NOP")}, 0); err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if _, err := sim.Run(3); err != nil { t.Fatalf("run failed: %v", err) }
	state := sim.GetState()
	if state.A != 0x13579 || state.Q != 0x13579 || state.Memory[13] != 0x13579 {
		t.Fatalf("unexpected state: %+v", state)
	}
}

func TestMOYMovesBlocks(t *testing.T) {
	sim := New(4096)
	_ = sim.WriteWord(20, 0x11111)
	_ = sim.WriteWord(21, 0x22222)
	_ = sim.WriteWord(30, 40)
	_ = sim.WriteWord(31, (1<<20)-2)
	if err := sim.LoadWords([]int{ins(t, 0o00, 30, 0), fixed(t, "LQA"), ins(t, 0o00, 31, 0), fixed(t, "XAQ"), ins(t, 0o24, 20, 0), fixed(t, "NOP")}, 0); err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if _, err := sim.Run(6); err != nil { t.Fatalf("run failed: %v", err) }
	state := sim.GetState()
	if state.A != 0 || state.Memory[40] != 0x11111 || state.Memory[41] != 0x22222 {
		t.Fatalf("unexpected state: %+v", state)
	}
}

func TestConsoleTypewriterPath(t *testing.T) {
	sim := New(4096)
	sim.SetControlSwitches(0o1633)
	if err := sim.LoadWords([]int{fixed(t, "RCS"), fixed(t, "TON"), shift(t, "SAN", 6), fixed(t, "TYP"), fixed(t, "NOP")}, 0); err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if _, err := sim.Run(5); err != nil { t.Fatalf("run failed: %v", err) }
	if sim.GetTypewriterOutput() != "-" { t.Fatalf("unexpected output: %q", sim.GetTypewriterOutput()) }
	if !sim.GetState().TypewriterPower { t.Fatalf("typewriter should be on") }
}

func TestRCDLoadsQueuedRecord(t *testing.T) {
	sim := New(4096)
	sim.QueueCardReaderRecord([]int{0x11111, 0x22222})
	if err := sim.LoadWords([]int{ins(t, 0o25, 10, 0), fixed(t, "NOP")}, 0); err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if _, err := sim.Run(2); err != nil { t.Fatalf("run failed: %v", err) }
	state := sim.GetState()
	if state.Memory[10] != 0x11111 || state.Memory[11] != 0x22222 { t.Fatalf("unexpected memory: %#v %#v", state.Memory[10], state.Memory[11]) }
}
