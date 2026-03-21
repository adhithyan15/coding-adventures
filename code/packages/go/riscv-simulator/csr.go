// Package riscvsimulator — Control and Status Register (CSR) file for M-mode.
//
// === What are CSRs? ===
//
// Control and Status Registers are special-purpose registers that control
// CPU behavior at a level above normal computation. While the general-purpose
// registers (x0-x31) hold data your program works with, CSRs control things
// like:
//
//   - Whether interrupts are enabled (mstatus)
//   - Where to jump when a trap occurs (mtvec)
//   - What caused the most recent trap (mcause)
//   - Where to return after handling a trap (mepc)
//
// === What is M-mode? ===
//
// RISC-V defines privilege levels. Machine mode (M-mode) is the highest
// privilege level — it has full access to all hardware. When a RISC-V
// CPU powers on, it starts in M-mode. An operating system kernel typically
// runs in a lower privilege level (S-mode), while M-mode firmware handles
// the most sensitive operations.
//
// The "m" prefix on CSR names (mstatus, mtvec, mepc, mcause, mscratch)
// indicates these are Machine-mode CSRs.
//
// === CSR addresses ===
//
// Each CSR has a 12-bit address. The RISC-V spec assigns specific addresses:
//
//   0x300 = mstatus    (machine status — interrupt enable bits)
//   0x305 = mtvec      (machine trap vector — where to jump on trap)
//   0x340 = mscratch   (machine scratch — temp storage for trap handler)
//   0x341 = mepc       (machine exception PC — saved PC on trap entry)
//   0x342 = mcause     (machine cause — why the trap happened)
//
// === How traps work ===
//
// When an exception occurs (like ecall), the CPU performs these steps
// atomically (all at once, before executing the next instruction):
//
//   1. Save current PC to mepc (so we can return later)
//   2. Save the cause code to mcause
//   3. Disable interrupts (clear MIE bit in mstatus)
//   4. Jump to the address in mtvec (the trap handler)
//
// The trap handler does its work, then executes "mret" to:
//   1. Restore PC from mepc
//   2. Re-enable interrupts (restore MIE bit in mstatus)
package riscvsimulator

// CSR address constants — these are defined by the RISC-V privileged spec.
const (
	CSRMstatus  = 0x300 // Machine status register
	CSRMtvec    = 0x305 // Machine trap-handler base address
	CSRMscratch = 0x340 // Machine scratch register for trap handlers
	CSRMepc     = 0x341 // Machine exception program counter
	CSRMcause   = 0x342 // Machine trap cause
)

// MIE is the Machine Interrupt Enable bit within mstatus (bit 3).
// When this bit is 0, all machine-level interrupts are disabled.
// When it is 1, interrupts that are individually enabled will fire.
const MIE = 1 << 3

// Trap cause codes — these identify why a trap was taken.
const (
	CauseEcallMMode = 11 // Environment call from Machine mode
)

// CSRFile holds the machine-mode Control and Status Registers.
//
// We use a simple map from CSR address to value. A real CPU would have
// dedicated hardware registers, but a map gives us flexibility to add
// new CSRs without changing the structure.
type CSRFile struct {
	regs map[uint32]uint32
}

// NewCSRFile creates a fresh CSR file with all registers initialized to 0.
func NewCSRFile() *CSRFile {
	return &CSRFile{
		regs: make(map[uint32]uint32),
	}
}

// Read returns the value of a CSR. Uninitialized CSRs read as 0.
func (c *CSRFile) Read(addr uint32) uint32 {
	return c.regs[addr]
}

// Write sets the value of a CSR.
func (c *CSRFile) Write(addr uint32, value uint32) {
	c.regs[addr] = value
}

// ReadWrite atomically reads the old value and writes a new value.
// This implements the CSRRW semantic: "swap rs1 value into CSR, old CSR into rd."
func (c *CSRFile) ReadWrite(addr uint32, newValue uint32) uint32 {
	old := c.regs[addr]
	c.regs[addr] = newValue
	return old
}

// ReadSet atomically reads the old value and sets bits specified by mask.
// This implements CSRRS: "read CSR, then OR in the mask bits."
//
// Example: if CSR = 0b0100 and mask = 0b0011, result CSR = 0b0111.
func (c *CSRFile) ReadSet(addr uint32, mask uint32) uint32 {
	old := c.regs[addr]
	c.regs[addr] = old | mask
	return old
}

// ReadClear atomically reads the old value and clears bits specified by mask.
// This implements CSRRC: "read CSR, then AND NOT the mask bits."
//
// Example: if CSR = 0b0111 and mask = 0b0011, result CSR = 0b0100.
func (c *CSRFile) ReadClear(addr uint32, mask uint32) uint32 {
	old := c.regs[addr]
	c.regs[addr] = old &^ mask
	return old
}
