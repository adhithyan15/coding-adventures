package riscvsimulator

import (
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/core"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
)

func runPreciseCore(t *testing.T, c *core.Core, maxCycles int) ([]cpupipeline.PipelineSnapshot, []int) {
	t.Helper()
	var snapshots []cpupipeline.PipelineSnapshot
	var retired []int
	for cycle := 0; cycle < maxCycles && !c.IsHalted(); cycle++ {
		snapshot := c.Step()
		snapshots = append(snapshots, snapshot)
		if wb := snapshot.Stages["WB"]; wb != nil && !wb.IsBubble && wb.PreciseFault == nil {
			retired = append(retired, wb.PC)
		}
	}
	if !c.IsHalted() {
		t.Fatalf("precise core did not halt within %d cycles", maxCycles)
	}
	return snapshots, retired
}

func TestPreciseMemoryModePreservesLegacyD05Trace(t *testing.T) {
	words := []uint32{
		EncodeAddi(0, 0, 99), EncodeAddi(1, 0, 0), EncodeAddi(2, 0, 128),
		EncodeSw(1, 2, 0), EncodeAddi(3, 0, 1), EncodeLw(3, 2, 0),
		EncodeBeq(3, 1, 8), EncodeAddi(5, 0, 99), EncodeAddi(4, 3, 1), EncodeEcall(),
	}
	program := Assemble(words)
	legacy, err := NewRiscVCore(core.SimpleConfig(), 256)
	if err != nil {
		t.Fatal(err)
	}
	precise, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
	if err != nil {
		t.Fatal(err)
	}
	legacy.LoadProgram(program, 0)
	precise.LoadProgram(program, 0)
	legacy.MemoryController().WriteWord(128, 42)
	precise.MemoryController().WriteWord(128, 42)

	legacySnapshots, legacyPCs := runPreciseCore(t, legacy, 100)
	preciseSnapshots, precisePCs := runPreciseCore(t, precise, 100)
	if !reflect.DeepEqual(legacySnapshots, preciseSnapshots) {
		t.Fatal("opt-in precise mode changed the nonfault D05 pipeline snapshots")
	}
	if !reflect.DeepEqual(legacyPCs, precisePCs) {
		t.Fatalf("retired PCs differ: legacy=%v precise=%v", legacyPCs, precisePCs)
	}
	if len(preciseSnapshots) != 16 || len(precisePCs) != 9 {
		t.Fatalf("legacy D05 profile changed: cycles=%d retired=%d", len(preciseSnapshots), len(precisePCs))
	}
	if got := precise.Stats().ForwardCount; got != 2 {
		t.Fatalf("forwards=%d, want 2", got)
	}
	if got := precise.Stats().StallCount; got != 1 {
		t.Fatalf("stalls=%d, want 1", got)
	}
	if got := precise.Stats().FlushCount; got != 1 {
		t.Fatalf("redirect flushes=%d, want 1", got)
	}
	if fault, ok := precise.PreciseFaultOutcome(); ok {
		t.Fatalf("nonfault program selected a fault: %+v", fault)
	}
	if got := precise.ReadRegister(4); got != 1 {
		t.Fatalf("target x4=%d, want 1", got)
	}
	if got := precise.MemoryController().ReadWord(128); got != 0 {
		t.Fatalf("final store value=%d, want 0", got)
	}
}

func TestPreciseMemoryForwardsAdjacentStoreToLoad(t *testing.T) {
	program := Assemble([]uint32{
		EncodeAddi(10, 0, 128),
		EncodeAddi(1, 0, 55),
		EncodeSw(1, 10, 0),
		EncodeLw(2, 10, 0),
		EncodeEcall(),
	})
	c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
	if err != nil {
		t.Fatal(err)
	}
	c.LoadProgram(program, 0)
	c.MemoryController().WriteWord(128, 7)
	snapshots, retired := runPreciseCore(t, c, 100)
	if got := c.ReadRegister(2); got != 55 {
		t.Fatalf("adjacent load read %d, want 55", got)
	}
	if got := c.MemoryController().ReadWord(128); got != 55 {
		t.Fatalf("committed RAM word=%d, want 55", got)
	}
	if got, want := len(retired), 5; got != want {
		t.Fatalf("retired %d instructions, want %d", got, want)
	}
	receipts := c.PreciseStoreReceipts()
	if len(receipts) != 1 {
		t.Fatalf("store receipts=%+v, want exactly one", receipts)
	}
	if got := receipts[0]; got.PC != 8 || got.Address != 128 || got.Value != 55 {
		t.Fatalf("wrong WB store receipt: %+v", got)
	}
	wbCycle := -1
	for _, snapshot := range snapshots {
		if wb := snapshot.Stages["WB"]; wb != nil && wb.PC == receipts[0].PC && !wb.IsBubble {
			wbCycle = snapshot.Cycle
		}
	}
	if receipts[0].Cycle != wbCycle {
		t.Fatalf("store committed at cycle %d, WB stage cycle is %d", receipts[0].Cycle, wbCycle)
	}
	if len(snapshots) != 9 {
		t.Fatalf("adjacent SW/LW added a wait cycle: got %d cycles, want 9", len(snapshots))
	}

	sequential := NewRiscVSimulator(256)
	sequential.CPU.Memory.WriteWord(128, 7)
	oracleTrace := sequential.Run(program)
	if len(oracleTrace) != len(retired) {
		t.Fatalf("sequential retired %d instructions, precise retired %d", len(oracleTrace), len(retired))
	}
	for index, trace := range oracleTrace {
		if retired[index] != trace.Fetch.PC {
			t.Fatalf("retired pc[%d]=%d, oracle=%d", index, retired[index], trace.Fetch.PC)
		}
	}
	for register := 0; register < 32; register++ {
		if got, want := uint32(c.ReadRegister(register)), sequential.CPU.Registers.Read(register); got != want {
			t.Errorf("x%d=%#x, oracle=%#x", register, got, want)
		}
	}
	for address := 0; address < 256; address += 4 {
		if got, want := uint32(c.MemoryController().ReadWord(address)), sequential.CPU.Memory.ReadWord(address); got != want {
			t.Errorf("RAM[%d]=%#x, oracle=%#x", address, got, want)
		}
	}
}

func TestPreciseMemoryAllowsWordAccessAtExactRAMEnd(t *testing.T) {
	program := Assemble([]uint32{
		EncodeAddi(1, 0, 73),
		EncodeSw(1, 0, 252),
		EncodeLw(2, 0, 252),
		EncodeEcall(),
	})
	c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
	if err != nil {
		t.Fatal(err)
	}
	c.LoadProgram(program, 0)
	_, retired := runPreciseCore(t, c, 100)
	if fault, ok := c.PreciseFaultOutcome(); ok {
		t.Fatalf("exact-end access faulted: %+v", fault)
	}
	if !reflect.DeepEqual(retired, []int{0, 4, 8, 12}) {
		t.Fatalf("retired PCs=%v, want all four instructions", retired)
	}
	if got := c.MemoryController().ReadWord(252); got != 73 {
		t.Fatalf("exact-end store=%d, want 73", got)
	}
	if got := c.ReadRegister(2); got != 73 {
		t.Fatalf("exact-end load=%d, want 73", got)
	}
}

func TestPreciseWordMemoryMustBeEnabledBeforeStepping(t *testing.T) {
	c, err := NewRiscVCore(core.SimpleConfig(), 256)
	if err != nil {
		t.Fatal(err)
	}
	c.Step()
	if err := c.EnablePreciseWordMemory(); err == nil {
		t.Fatal("enabled precise memory after execution began")
	}
}

func TestPreciseWordMemoryRequiresHazardDetection(t *testing.T) {
	config := core.SimpleConfig()
	config.HazardDetection = false
	legacy, err := NewRiscVCore(config, 256)
	if err != nil {
		t.Fatalf("legacy constructor rejected hazard-disabled config: %v", err)
	}
	if legacy.Config().HazardDetection {
		t.Fatal("legacy constructor changed hazard-disabled config")
	}
	if err := legacy.EnablePreciseWordMemory(); err == nil {
		t.Fatal("Core.EnablePreciseWordMemory accepted a config without hazard detection")
	}
	if _, err := NewRiscVCoreWithPreciseMemory(config, 256); err == nil {
		t.Fatal("precise-memory constructor accepted a config without fault-selection hazards")
	}
}

func TestPreciseFaultPreservesOlderEffectsAndDoesNotRetireFaultToken(t *testing.T) {
	program := Assemble([]uint32{
		EncodeAddi(3, 0, 42),
		EncodeAddi(10, 0, 128),
		EncodeAddi(2, 0, 11),
		EncodeSw(2, 10, 0),
		EncodeAddi(4, 0, 7),
		EncodeLw(3, 0, 256),   // selected fault at PC 20
		EncodeAddi(31, 0, 99), // younger effect must be squashed
	})
	c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
	if err != nil {
		t.Fatal(err)
	}
	c.LoadProgram(program, 0)
	snapshots, retired := runPreciseCore(t, c, 100)
	wantRetired := []int{0, 4, 8, 12, 16}
	if !reflect.DeepEqual(retired, wantRetired) {
		t.Fatalf("retired PCs=%v, want older instructions only %v", retired, wantRetired)
	}
	if got := c.Pipeline().Stats().InstructionsCompleted; got != len(wantRetired) {
		t.Fatalf("completed count=%d, want %d (fault token is not retired)", got, len(wantRetired))
	}
	if got := c.ReadRegister(3); got != 42 {
		t.Fatalf("faulting LW changed x3 to %d, want 42", got)
	}
	if got := c.ReadRegister(4); got != 7 {
		t.Fatalf("older GPR effect missing: x4=%d", got)
	}
	if got := c.ReadRegister(31); got != 0 {
		t.Fatalf("younger x31 effect escaped: %d", got)
	}
	if got := c.MemoryController().ReadWord(128); got != 11 {
		t.Fatalf("older store did not commit: RAM[128]=%d", got)
	}
	if receipts := c.PreciseStoreReceipts(); len(receipts) != 1 || receipts[0].PC != 12 {
		t.Fatalf("older store receipt missing or duplicated: %+v", receipts)
	}
	fault, ok := c.PreciseFaultOutcome()
	if !ok || fault.PC != 20 || fault.Address != 256 || fault.Access != "load" || fault.Width != 4 || fault.Cause != "load_access_fault" {
		t.Fatalf("wrong selected fault: ok=%t fault=%+v", ok, fault)
	}
	last := snapshots[len(snapshots)-1]
	if wb := last.Stages["WB"]; wb == nil || wb.PreciseFault == nil || wb.PC != fault.PC {
		t.Fatalf("fault-selection WB token missing: %+v", wb)
	}
	cycle := c.Cycle()
	if got := c.Step(); got.Cycle != cycle || c.Cycle() != cycle {
		t.Fatalf("post-terminal Step advanced cycle from %d to %d", cycle, c.Cycle())
	}
	if again, ok := c.PreciseFaultOutcome(); !ok || again != fault {
		t.Fatalf("terminal fault outcome changed or disappeared: %+v", again)
	}
}

func TestPreciseFaultingStoresAreAtomic(t *testing.T) {
	tests := []struct {
		name    string
		address int
		cause   string
	}{
		{"misaligned", 254, "store_address_misaligned"},
		{"out-of-range", 256, "store_access_fault"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			program := Assemble([]uint32{EncodeAddi(2, 0, 99), EncodeSw(2, 0, tc.address)})
			c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
			if err != nil {
				t.Fatal(err)
			}
			c.MemoryController().WriteWord(252, 0x11223344)
			c.LoadProgram(program, 0)
			_, retired := runPreciseCore(t, c, 100)
			if got := c.MemoryController().ReadWord(252); got != 0x11223344 {
				t.Fatalf("faulting store partially changed boundary word: %#x", got)
			}
			if got := c.PreciseStoreReceipts(); len(got) != 0 {
				t.Fatalf("faulting store emitted commit receipt: %+v", got)
			}
			fault, ok := c.PreciseFaultOutcome()
			if !ok || fault.PC != 4 || fault.Address != uint32(tc.address) || fault.Cause != tc.cause {
				t.Fatalf("fault=%+v ok=%t", fault, ok)
			}
			if !reflect.DeepEqual(retired, []int{0}) {
				t.Fatalf("faulting SW retired: %v", retired)
			}
		})
	}
}

func TestPreciseFaultingLoadsDoNotWriteDestination(t *testing.T) {
	tests := []struct {
		name    string
		address int
		cause   string
	}{
		{"misaligned", 2, "load_address_misaligned"},
		{"out-of-range", 256, "load_access_fault"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			program := Assemble([]uint32{
				EncodeAddi(2, 0, 42),
				EncodeLw(2, 0, tc.address),
			})
			c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
			if err != nil {
				t.Fatal(err)
			}
			c.LoadProgram(program, 0)
			_, retired := runPreciseCore(t, c, 100)
			if got := c.ReadRegister(2); got != 42 {
				t.Fatalf("faulting LW changed x2 to %d", got)
			}
			fault, ok := c.PreciseFaultOutcome()
			if !ok || fault.PC != 4 || fault.Address != uint32(tc.address) || fault.Cause != tc.cause {
				t.Fatalf("fault=%+v ok=%t", fault, ok)
			}
			if !reflect.DeepEqual(retired, []int{0}) {
				t.Fatalf("faulting LW retired: %v", retired)
			}
		})
	}
}

func TestPreciseMemoryUsesWrappedRV32EffectiveAddresses(t *testing.T) {
	t.Run("low wrapped address succeeds", func(t *testing.T) {
		program := Assemble([]uint32{
			EncodeAddi(1, 0, -1),
			EncodeLw(2, 1, 1), // 0xffffffff + 1 wraps to address zero
			EncodeEcall(),
		})
		c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
		if err != nil {
			t.Fatal(err)
		}
		c.LoadProgram(program, 128)
		c.MemoryController().WriteWord(0, 0x12345678)
		for cycle := 0; cycle < 100 && !c.IsHalted(); cycle++ {
			c.Step()
		}
		if _, faulted := c.PreciseFaultOutcome(); !c.IsHalted() || faulted {
			t.Fatal("wrapped low address did not complete successfully")
		}
		if got := uint32(c.ReadRegister(2)); got != 0x12345678 {
			t.Fatalf("wrapped low read=%#x, want %#x", got, uint32(0x12345678))
		}
		oracle := NewRiscVSimulator(256)
		oracle.CPU.LoadProgram(program, 128)
		oracle.CPU.Memory.WriteWord(0, 0x12345678)
		oracle.CPU.PC = 128
		oracle.CPU.Run(100)
		if got, want := uint32(c.ReadRegister(2)), oracle.CPU.Registers.Read(2); got != want {
			t.Fatalf("wrapped result=%#x, sequential=%#x", got, want)
		}
	})
	t.Run("aligned high address faults", func(t *testing.T) {
		program := Assemble([]uint32{EncodeAddi(1, 0, -4), EncodeLw(2, 1, 0)})
		c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
		if err != nil {
			t.Fatal(err)
		}
		c.LoadProgram(program, 0)
		_, _ = runPreciseCore(t, c, 100)
		fault, ok := c.PreciseFaultOutcome()
		if !ok || fault.Address != 0xfffffffc || fault.Cause != "load_access_fault" {
			t.Fatalf("high-address fault=%+v ok=%t", fault, ok)
		}
	})
}

func TestPreciseModeDoesNotSelectWrongPathLoadFault(t *testing.T) {
	program := Assemble([]uint32{
		EncodeAddi(1, 0, 1),
		EncodeBeq(1, 1, 8),
		EncodeLw(2, 0, 256), // wrong-path fault must be flushed before EX
		EncodeAddi(3, 0, 9),
		EncodeEcall(),
	})
	c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
	if err != nil {
		t.Fatal(err)
	}
	c.LoadProgram(program, 0)
	_, retired := runPreciseCore(t, c, 100)
	if fault, ok := c.PreciseFaultOutcome(); ok {
		t.Fatalf("wrong-path fault was selected: %+v", fault)
	}
	if !reflect.DeepEqual(retired, []int{0, 4, 12, 16}) {
		t.Fatalf("wrong-path path retired PCs=%v", retired)
	}
	if got := c.ReadRegister(3); got != 9 {
		t.Fatalf("correct target did not retire: x3=%d", got)
	}
}

func TestPreciseFaultsMatchIndependentSequentialPrefix(t *testing.T) {
	tests := []struct {
		name      string
		access    string
		address   int
		wantCause string
	}{
		{name: "aligned out-of-range load", access: "load", address: 256, wantCause: "load_access_fault"},
		{name: "misaligned load", access: "load", address: 1, wantCause: "load_address_misaligned"},
		{name: "aligned out-of-range store", access: "store", address: 256, wantCause: "store_access_fault"},
		{name: "misaligned store", access: "store", address: 1, wantCause: "store_address_misaligned"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			prefix := []uint32{
				EncodeAddi(10, 0, 128), // older store address
				EncodeAddi(1, 0, 42),   // older store data
				EncodeSw(1, 10, 0),     // must commit before selected fault
				EncodeAddi(3, 0, 7),    // older GPR write and faulting-LW sentinel
			}
			var faulting uint32
			if tc.access == "load" {
				faulting = EncodeLw(3, 0, tc.address)
			} else {
				faulting = EncodeSw(1, 0, tc.address)
			}
			guest := append(append([]uint32(nil), prefix...),
				faulting,
				EncodeSw(1, 10, 4),    // younger store must not commit
				EncodeAddi(31, 0, 99), // younger GPR effect must not retire
			)

			c, err := NewRiscVCoreWithPreciseMemory(core.SimpleConfig(), 256)
			if err != nil {
				t.Fatal(err)
			}
			c.LoadProgram(Assemble(guest), 0)
			c.MemoryController().WriteWord(132, 88)
			snapshots, retired := runPreciseCore(t, c, 100)

			// Load the same complete instruction image, but step the independent
			// engine only through the successful prefix before the selected access.
			sequential := NewRiscVSimulator(256)
			sequential.CPU.Memory.WriteWord(132, 88)
			sequential.CPU.LoadProgram(Assemble(guest), 0)
			wantRetired := make([]int, 0, len(prefix))
			for range prefix {
				step := sequential.Step()
				wantRetired = append(wantRetired, step.Fetch.PC)
			}
			if !reflect.DeepEqual(retired, wantRetired) {
				t.Fatalf("successful retirements=%v, sequential prefix=%v", retired, wantRetired)
			}
			if got := c.Pipeline().Stats().InstructionsCompleted; got != len(prefix) {
				t.Fatalf("completed count=%d, want successful prefix count %d", got, len(prefix))
			}
			fault, ok := c.PreciseFaultOutcome()
			if !ok || fault.PC != 16 || fault.Address != uint32(tc.address) || fault.Width != 4 || fault.Access != tc.access || fault.Cause != tc.wantCause {
				t.Fatalf("selected fault=%+v, ok=%t", fault, ok)
			}
			last := snapshots[len(snapshots)-1]
			if wb := last.Stages["WB"]; wb == nil || wb.PreciseFault == nil || wb.PC != fault.PC {
				t.Fatalf("fault selection token missing at WB: %+v", wb)
			}
			if got := c.ReadRegister(31); got != 0 {
				t.Fatalf("younger x31 effect escaped: %d", got)
			}
			if got := c.ReadRegister(3); got != 7 {
				t.Fatalf("faulting load changed x3 or older write was lost: %d", got)
			}
			receipts := c.PreciseStoreReceipts()
			if len(receipts) != 1 || receipts[0].PC != 8 || receipts[0].Address != 128 || receipts[0].Value != 42 {
				t.Fatalf("committed store receipts=%+v, want only older PC 8 store", receipts)
			}
			for register := 0; register < 32; register++ {
				if got, want := uint32(c.ReadRegister(register)), sequential.CPU.Registers.Read(register); got != want {
					t.Errorf("x%d staged=%#x, successful-prefix oracle=%#x", register, got, want)
				}
			}
			for address := 0; address < 256; address += 4 {
				if got, want := uint32(c.MemoryController().ReadWord(address)), sequential.CPU.Memory.ReadWord(address); got != want {
					t.Errorf("RAM[%d] staged=%#x, successful-prefix oracle=%#x", address, got, want)
				}
			}
		})
	}
}
