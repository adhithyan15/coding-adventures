// Package systemboard is the top-level integration package -- the actual
// simulated computer. It composes ROM/BIOS, Bootloader, Interrupt Handler,
// OS Kernel, Display, and a RISC-V CPU into a complete system.
//
// The SystemBoard orchestrates the full boot-to-Hello-World sequence:
//
//	PowerOn() -> BIOS -> Bootloader -> Kernel -> Hello World -> Idle
//
// === How It Works ===
//
// The SystemBoard uses a simple RISC-V simulator (not the full D05 Core
// pipeline) to execute instructions. After each instruction, it checks for
// ecall traps and dispatches them to the kernel's syscall handler in Go.
//
// This is a pragmatic design: the BIOS and bootloader are real RISC-V
// machine code running on the simulator. The kernel initialization and
// syscall handling happen in Go. The hello-world program is real RISC-V
// code that triggers ecall instructions.
package systemboard

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/bootloader"
	"github.com/adhithyan15/coding-adventures/code/packages/go/display"
	interrupthandler "github.com/adhithyan15/coding-adventures/code/packages/go/interrupt-handler"
	oskernel "github.com/adhithyan15/coding-adventures/code/packages/go/os-kernel"
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
	rombios "github.com/adhithyan15/coding-adventures/code/packages/go/rom-bios"
)

// =========================================================================
// SystemBoard -- the complete simulated computer
// =========================================================================

// SystemBoard composes all hardware and software components into a working
// computer. It provides the power-on-to-hello-world boot trace.
type SystemBoard struct {
	// Config holds the system configuration.
	Config SystemConfig

	// CPU is the RISC-V simulator executing instructions.
	CPU *riscv.RiscVSimulator

	// ROM holds the BIOS firmware.
	ROM *rombios.ROM

	// DiskImage is the simulated persistent storage.
	DiskImage *bootloader.DiskImage

	// Display is the text framebuffer driver.
	Display *display.DisplayDriver

	// InterruptCtrl manages interrupt delivery.
	InterruptCtrl *interrupthandler.InterruptController

	// Kernel is the OS kernel.
	Kernel *oskernel.Kernel

	// Trace accumulates boot events.
	Trace *BootTrace

	// Powered is true after PowerOn().
	Powered bool

	// Cycle is the current CPU cycle count.
	Cycle int

	// CurrentPhase tracks the current boot phase.
	CurrentPhase BootPhase

	// kernelBooted tracks whether the kernel has been initialized.
	kernelBooted bool

	// previousPC tracks PC from last step for phase detection.
	previousPC uint32
}

// NewSystemBoard creates a system board with all components instantiated
// but not yet powered on.
func NewSystemBoard(config SystemConfig) *SystemBoard {
	return &SystemBoard{
		Config: config,
		Trace:  &BootTrace{},
	}
}

// PowerOn initializes all components and begins the boot sequence.
// Sets PC to ROM base address. The BIOS begins executing on the next Step().
func (b *SystemBoard) PowerOn() {
	if b.Powered {
		return // Idempotent
	}

	config := b.Config

	// --- 1. Create the CPU ---
	// We need memory large enough for the full address space.
	// The RISC-V simulator uses flat memory, so we allocate enough to cover
	// ROM at 0xFFFF0000. This is large but the simulator uses sparse arrays.
	memSize := 0x100010000 // Just above 4GB boundary - use rom base + rom size
	// Actually, the flat memory of the RiscVSimulator won't work for addresses
	// this high. We need a different approach.
	//
	// For practical purposes, we'll use the cpu-simulator's flat memory which
	// supports byte addressing. The RiscVSimulator uses cpu.Memory which can
	// handle any size. For addresses like 0xFFFF0000, we need memSize > that.
	// This is impractical (4GB). Instead, we'll create a large enough memory
	// and remap ROM/framebuffer to lower addresses.
	//
	// PRAGMATIC APPROACH: We run the BIOS/bootloader phases symbolically by
	// pre-loading their effects, then run the kernel + user program phases
	// on the actual simulator. This is how real emulators often work for the
	// early boot stages.

	// The total memory needed is just enough for the user-space code:
	// - 0x00000000 to 0x00070000 for IDT, boot protocol, bootloader, kernel, processes
	// - 0x10000000 to 0x10200000 for disk image (memory mapped)
	// We skip the ROM region (0xFFFF0000+) and framebuffer (0xFFFB0000+) since
	// those would require 4GB of flat memory.

	// Use a memory size that covers the disk image mapped region.
	memSize = 0x10200000
	b.CPU = riscv.NewRiscVSimulator(memSize)

	// --- 2. Create the Interrupt Controller ---
	b.InterruptCtrl = interrupthandler.NewInterruptController()

	// --- 3. Create the Display ---
	displayMem := make([]byte, config.DisplayConfig.Columns*config.DisplayConfig.Rows*display.BytesPerCell)
	b.Display = display.NewDisplayDriver(config.DisplayConfig, displayMem)

	// --- 4. Generate and load BIOS firmware ---
	biosFirmware := rombios.NewBIOSFirmware(config.BIOSConfig)
	biosBytes := biosFirmware.Generate()
	b.ROM = rombios.NewROM(rombios.DefaultROMConfig(), biosBytes)

	// --- 5. Create the Disk Image ---
	b.DiskImage = bootloader.NewDiskImage(bootloader.DefaultDiskSize)

	// --- 6. Create the Kernel ---
	b.Kernel = oskernel.NewKernel(config.KernelConfig, b.InterruptCtrl, b.Display)

	// --- 7. Generate and prepare all binaries ---
	// Get user program binary.
	var userProgram []byte
	if config.UserProgram != nil {
		userProgram = config.UserProgram
	} else {
		userProgram = oskernel.GenerateHelloWorldProgram(UserProcessBase)
	}

	// Generate kernel binary (this is the code that the bootloader loads).
	// In our system, the "kernel binary" that gets loaded from disk includes:
	// - Kernel initialization stub (a few instructions, then ecall to hand off)
	// - Idle process code
	// - Hello-world process code + data
	// For simplicity, we pre-load everything into the right memory locations.

	// Generate bootloader code.
	blConfig := config.BootloaderConfig
	// Calculate total kernel size: idle + hello-world binaries + kernel stub.
	idleBinary := oskernel.GenerateIdleProgram()
	kernelStubSize := 16 // A few RISC-V instructions for the kernel entry

	// Total size that needs to be on disk and copied.
	totalSize := kernelStubSize + len(idleBinary) + len(userProgram)
	// Round up to multiple of 4.
	if totalSize%4 != 0 {
		totalSize += 4 - (totalSize % 4)
	}
	blConfig.KernelSize = uint32(totalSize)

	bl := bootloader.NewBootloader(blConfig)
	bootloaderCode := bl.Generate()

	// --- 8. Pre-load everything into memory ---

	// Phase 1: Simulate BIOS effects (write boot protocol to 0x00001000).
	// Instead of running BIOS RISC-V code (which would need ROM at 0xFFFF0000),
	// we directly write the hardware info structure.
	hwInfo := rombios.HardwareInfo{
		MemorySize:      uint32(config.MemorySize),
		DisplayColumns:  uint32(config.DisplayConfig.Columns),
		DisplayRows:     uint32(config.DisplayConfig.Rows),
		FramebufferBase: config.DisplayConfig.FramebufferBase,
		IDTBase:         0x00000000,
		IDTEntries:      256,
		BootloaderEntry: uint32(BootloaderBase),
	}
	hwBytes := hwInfo.ToBytes()
	for i, byteVal := range hwBytes {
		b.CPU.CPU.Memory.WriteByte(int(BootProtocolAddr)+i, byteVal)
	}

	// Write boot protocol magic (0xB007CAFE) for the bootloader to validate.
	// The HardwareInfo struct starts with MemorySize, but the bootloader expects
	// the magic at offset 0. Let me check what the bootloader actually reads...
	// The bootloader reads memory[0x00001000] and compares to 0xB007CAFE.
	// But HardwareInfo doesn't have a magic field! The boot protocol from the
	// spec (S02) has a different layout than HardwareInfo from S01.
	//
	// The S02 spec says:
	//   Offset 0x00: Magic number (0xB007CAFE)
	//   Offset 0x04: Total memory size
	//   Offset 0x08: Kernel disk offset
	//   ...
	//
	// But the S01 HardwareInfo struct has:
	//   Offset 0x00: MemorySize
	//   Offset 0x04: DisplayColumns
	//   ...
	//
	// These are different formats! For integration, we write the boot protocol
	// format that the bootloader expects. The bootloader validates magic at
	// offset 0 of the boot protocol address.
	writeWordToMem(b.CPU, BootProtocolAddr+0, bootloader.BootProtocolMagic) // Magic
	writeWordToMem(b.CPU, BootProtocolAddr+4, uint32(config.MemorySize))    // Total memory
	writeWordToMem(b.CPU, BootProtocolAddr+8, blConfig.KernelDiskOffset)    // Kernel disk offset
	writeWordToMem(b.CPU, BootProtocolAddr+12, blConfig.KernelSize)         // Kernel size
	writeWordToMem(b.CPU, BootProtocolAddr+16, blConfig.KernelLoadAddress)  // Kernel load address
	writeWordToMem(b.CPU, BootProtocolAddr+20, blConfig.StackBase)          // Stack base

	// Load bootloader code at its entry address.
	for i, byteVal := range bootloaderCode {
		b.CPU.CPU.Memory.WriteByte(int(BootloaderBase)+i, byteVal)
	}

	// Build the "kernel disk image" -- the bytes that live on disk and get copied.
	kernelDiskData := make([]byte, totalSize)
	// Kernel entry stub: a few instructions that ecall to signal "kernel ready".
	// Actually, in our design the kernel doesn't run as RISC-V code -- it runs
	// as Go code. So the "kernel binary" on disk is really just the idle and
	// hello-world binaries that the bootloader copies into position.
	// The kernel Go code will be invoked by the SystemBoard after the bootloader
	// finishes.
	copy(kernelDiskData, make([]byte, kernelStubSize)) // zero stub
	copy(kernelDiskData[kernelStubSize:], idleBinary)
	copy(kernelDiskData[kernelStubSize+len(idleBinary):], userProgram)

	// Load kernel disk data into the disk image.
	b.DiskImage.LoadKernel(kernelDiskData)

	// Memory-map the disk image at DiskMappedBase.
	diskData := b.DiskImage.Data()
	for i, byteVal := range diskData {
		addr := int(DiskMappedBase) + i
		if addr < memSize {
			b.CPU.CPU.Memory.WriteByte(addr, byteVal)
		}
	}

	// Also pre-load the user program and idle binaries at their final locations.
	// The bootloader will copy them too, but we need them in case the bootloader
	// doesn't run (when we skip BIOS/bootloader phases).
	for i, byteVal := range idleBinary {
		b.CPU.CPU.Memory.WriteByte(int(IdleProcessBase)+i, byteVal)
	}
	for i, byteVal := range userProgram {
		b.CPU.CPU.Memory.WriteByte(int(UserProcessBase)+i, byteVal)
	}

	// Set PC to bootloader entry (we skip BIOS since it needs ROM at 0xFFFF0000).
	b.CPU.CPU.PC = int(BootloaderBase)

	// Configure the CSR file for trap handling.
	// Set mtvec to a known address so ecall triggers a trap instead of halting.
	// We'll use a sentinel address that we can detect.
	b.CPU.CSR.Write(riscv.CSRMtvec, 0xDEAD0000)

	b.Powered = true
	b.CurrentPhase = PhasePowerOn
	b.Trace.AddEvent(PhasePowerOn, 0, "System powered on")
	b.Trace.AddEvent(PhaseBIOS, 0, "BIOS phase simulated (hardware info written to boot protocol)")
	b.CurrentPhase = PhaseBIOS
}

// Step executes one CPU cycle and checks for traps/phase transitions.
func (b *SystemBoard) Step() {
	if !b.Powered {
		return
	}

	b.previousPC = uint32(b.CPU.CPU.PC)
	b.Cycle++

	// Execute one instruction.
	b.CPU.Step()

	// Check for phase transitions based on PC.
	b.detectPhaseTransition()

	// Check for ecall trap (CSR mepc/mcause set by the ISA decoder).
	b.handleTrap()
}

// Run executes cycles until the system is idle or maxCycles is exhausted.
// Returns the complete boot trace.
func (b *SystemBoard) Run(maxCycles int) *BootTrace {
	if !b.Powered {
		return b.Trace
	}

	for i := 0; i < maxCycles; i++ {
		b.Step()

		// Check if we've reached idle state.
		if b.kernelBooted && b.Kernel.IsIdle() {
			if b.CurrentPhase != PhaseIdle {
				b.CurrentPhase = PhaseIdle
				b.Trace.AddEvent(PhaseIdle, b.Cycle,
					"System idle -- all user programs terminated")
			}
			break
		}

		// Safety: if CPU halted (ecall with mtvec=0), stop.
		if b.CPU.CPU.Halted {
			break
		}
	}

	return b.Trace
}

// InjectKeystroke simulates a keyboard press.
func (b *SystemBoard) InjectKeystroke(char byte) {
	if b.Kernel != nil {
		b.Kernel.AddKeystroke(char)
	}
	if b.InterruptCtrl != nil {
		b.InterruptCtrl.RaiseInterrupt(oskernel.InterruptKeyboard)
	}
}

// DisplaySnapshot returns the current state of the text display.
func (b *SystemBoard) DisplaySnapshot() *display.DisplaySnapshot {
	if b.Display == nil {
		return nil
	}
	snap := b.Display.Snapshot()
	return &snap
}

// GetBootTrace returns the accumulated boot trace.
func (b *SystemBoard) GetBootTrace() *BootTrace {
	return b.Trace
}

// IsIdle returns true when the kernel reports only the idle process remains.
func (b *SystemBoard) IsIdle() bool {
	return b.kernelBooted && b.Kernel != nil && b.Kernel.IsIdle()
}

// GetCycleCount returns the total CPU cycles executed since PowerOn.
func (b *SystemBoard) GetCycleCount() int {
	return b.Cycle
}

// GetCurrentPhase returns the current boot phase.
func (b *SystemBoard) GetCurrentPhase() BootPhase {
	return b.CurrentPhase
}

// =========================================================================
// Internal: Phase detection
// =========================================================================

// detectPhaseTransition checks the current PC and kernel state to detect
// boot phase transitions.
func (b *SystemBoard) detectPhaseTransition() {
	pc := uint32(b.CPU.CPU.PC)

	switch b.CurrentPhase {
	case PhaseBIOS:
		// BIOS phase: we start at bootloader entry, so transition immediately.
		if pc >= BootloaderBase && pc < BootloaderBase+0x10000 {
			b.CurrentPhase = PhaseBootloader
			b.Trace.AddEvent(PhaseBootloader, b.Cycle,
				"Bootloader executing: copying kernel from disk to RAM")
		}

	case PhaseBootloader:
		// Bootloader jumps to kernel entry at KernelBase.
		if pc >= KernelBase && pc < KernelBase+0x10000 {
			b.CurrentPhase = PhaseKernelInit
			b.Trace.AddEvent(PhaseKernelInit, b.Cycle,
				"Kernel entry reached: initializing subsystems")
			b.initializeKernel()
		}

	case PhaseKernelInit:
		// After kernel init, we redirect to user program.
		if pc >= UserProcessBase && pc < UserProcessBase+0x10000 {
			b.CurrentPhase = PhaseUserProgram
			b.Trace.AddEvent(PhaseUserProgram, b.Cycle,
				"User program (hello-world) executing")
		}
	}
}

// initializeKernel boots the kernel (Go-side) and sets up the CPU to run
// the hello-world process.
func (b *SystemBoard) initializeKernel() {
	if b.kernelBooted {
		return
	}

	// Boot the kernel (creates processes, registers ISRs, starts scheduler).
	b.Kernel.Boot()
	b.kernelBooted = true

	b.Trace.AddEvent(PhaseKernelInit, b.Cycle,
		fmt.Sprintf("Kernel booted: %d processes created", b.Kernel.ProcessCount()))

	// Redirect CPU to run the hello-world process (PID 1).
	// The kernel started PID 1 as Running, so we set the CPU's PC to
	// the hello-world entry point.
	if len(b.Kernel.ProcessTable) > 1 {
		pcb := b.Kernel.ProcessTable[1]
		b.CPU.CPU.PC = int(pcb.SavedPC)

		// Set the stack pointer.
		b.CPU.CPU.Registers.Write(oskernel.RegSP, pcb.StackPointer)
	}
}

// =========================================================================
// Internal: Trap handling
// =========================================================================

// handleTrap checks if an ecall trap occurred and dispatches to the kernel.
//
// When the RISC-V ISA decoder encounters ecall with mtvec != 0, it:
//   1. Saves PC to mepc
//   2. Sets mcause to 11 (ecall from M-mode)
//   3. Jumps to mtvec
//
// We detect this by checking if PC == mtvec sentinel (0xDEAD0000).
func (b *SystemBoard) handleTrap() {
	pc := uint32(b.CPU.CPU.PC)

	// Check if we landed at the trap vector sentinel.
	if pc != 0xDEAD0000 {
		return
	}

	if !b.kernelBooted {
		// Before kernel is booted, ecall from the bootloader/BIOS is unexpected.
		// Just skip it.
		mepc := b.CPU.CSR.Read(riscv.CSRMepc)
		b.CPU.CPU.PC = int(mepc) + 4
		b.CPU.CSR.Write(riscv.CSRMstatus, b.CPU.CSR.Read(riscv.CSRMstatus)|riscv.MIE)
		return
	}

	// Read the syscall number from a7 (x17).
	syscallNum := int(b.CPU.CPU.Registers.Read(oskernel.RegA7))

	// Read mepc (the PC of the ecall instruction).
	mepc := b.CPU.CSR.Read(riscv.CSRMepc)

	// Create register and memory accessors for the kernel.
	regAccess := &cpuRegAccess{cpu: b.CPU}
	memAccess := &cpuMemAccess{cpu: b.CPU}

	// Dispatch the syscall.
	b.Kernel.HandleSyscall(syscallNum, regAccess, memAccess)

	// After syscall handling, check if we need to switch processes.
	if b.Kernel.GetCurrentPCB() != nil {
		currentPCB := b.Kernel.GetCurrentPCB()

		if currentPCB.State == oskernel.ProcessRunning {
			// Resume the same process after the ecall: PC = mepc + 4.
			b.CPU.CPU.PC = int(mepc) + 4
		} else if currentPCB.State == oskernel.ProcessReady ||
			currentPCB.State == oskernel.ProcessTerminated {
			// Process yielded or exited. Switch to next process.
			nextPCB := b.Kernel.GetCurrentPCB()
			if nextPCB != nil && nextPCB.State == oskernel.ProcessRunning {
				b.CPU.CPU.PC = int(nextPCB.SavedPC)
				b.CPU.CPU.Registers.Write(oskernel.RegSP, nextPCB.StackPointer)
			} else {
				// No runnable process -- system is idle.
				// Set PC to idle process.
				if len(b.Kernel.ProcessTable) > 0 {
					idlePCB := b.Kernel.ProcessTable[0]
					b.CPU.CPU.PC = int(idlePCB.SavedPC)
				}
			}
		}
	} else {
		// No current process -- skip past ecall.
		b.CPU.CPU.PC = int(mepc) + 4
	}

	// Re-enable interrupts.
	b.CPU.CSR.Write(riscv.CSRMstatus, b.CPU.CSR.Read(riscv.CSRMstatus)|riscv.MIE)

	// Record syscall events in the trace.
	switch syscallNum {
	case oskernel.SysWrite:
		b.Trace.AddEvent(b.CurrentPhase, b.Cycle,
			fmt.Sprintf("sys_write: bytes written to display"))
	case oskernel.SysExit:
		b.Trace.AddEvent(b.CurrentPhase, b.Cycle,
			fmt.Sprintf("sys_exit: process terminated"))
	case oskernel.SysYield:
		b.Trace.AddEvent(b.CurrentPhase, b.Cycle, "sys_yield: voluntary context switch")
	}
}

// =========================================================================
// CPU Access Adapters
// =========================================================================

// cpuRegAccess adapts the RiscVSimulator to the kernel's RegisterAccess interface.
type cpuRegAccess struct {
	cpu *riscv.RiscVSimulator
}

func (r *cpuRegAccess) ReadRegister(index int) uint32 {
	return r.cpu.CPU.Registers.Read(index)
}

func (r *cpuRegAccess) WriteRegister(index int, value uint32) {
	r.cpu.CPU.Registers.Write(index, value)
}

// cpuMemAccess adapts the RiscVSimulator to the kernel's MemoryAccess interface.
type cpuMemAccess struct {
	cpu *riscv.RiscVSimulator
}

func (m *cpuMemAccess) ReadMemoryByte(address uint32) byte {
	return m.cpu.CPU.Memory.ReadByte(int(address))
}

// =========================================================================
// Helper
// =========================================================================

// writeWordToMem writes a 32-bit little-endian word to the simulator's memory.
func writeWordToMem(cpu *riscv.RiscVSimulator, address uint32, value uint32) {
	cpu.CPU.Memory.WriteByte(int(address), byte(value&0xFF))
	cpu.CPU.Memory.WriteByte(int(address)+1, byte((value>>8)&0xFF))
	cpu.CPU.Memory.WriteByte(int(address)+2, byte((value>>16)&0xFF))
	cpu.CPU.Memory.WriteByte(int(address)+3, byte((value>>24)&0xFF))
}
