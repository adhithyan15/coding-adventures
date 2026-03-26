// Package interrupthandler implements the S03 Interrupt Handler for the
// coding-adventures simulated computer. It manages the full interrupt
// lifecycle: the Interrupt Descriptor Table (IDT), Interrupt Service Routines
// (ISRs), the interrupt controller, and save/restore of CPU context.
//
// # What Is an Interrupt?
//
// An interrupt is a signal that tells the CPU: "stop what you are doing and
// handle this event." Without interrupts, a CPU can only execute instructions
// sequentially. Interrupts are what transform a calculator into a computer --
// they enable responding to external events (keyboard, timer), multitasking,
// and system services.
//
// # Analogy
//
// Interrupts are like a phone ringing while you are cooking. You pause cooking
// (save context -- remember what step you were on), answer the phone (handle
// the interrupt), and when the call ends, resume cooking exactly where you
// left off (restore context).
package interrupthandler

import "encoding/binary"

// =========================================================================
// Interrupt Types and Well-Known Numbers
// =========================================================================

// InterruptType classifies the source of an interrupt. There are three
// categories in real hardware:
//
//	Type           Trigger              Examples
//	------------------------------------------------------------------------
//	Fault          CPU detects error    Division by zero, invalid opcode
//	Timer          Clock tick           Drives the scheduler
//	Keyboard       External keystroke   Host keystroke injected
//	Syscall        ecall instruction    User program requests kernel service
type InterruptType int

const (
	InterruptFault    InterruptType = iota // CPU exception (interrupts 0-31)
	InterruptTimer                         // Clock tick (interrupt 32)
	InterruptKeyboard                      // External keystroke (interrupt 33)
	InterruptSyscall                       // Software ecall (interrupt 128)
)

// Well-known interrupt numbers. These follow x86/RISC-V conventions:
//
//	Number  Name                Source
//	------  ----                ------
//	0       Division by Zero    CPU
//	1       Debug Exception     CPU
//	2       NMI                 Hardware (non-maskable)
//	3       Breakpoint          CPU (ebreak)
//	4       Overflow            CPU
//	5       Invalid Opcode      CPU
//	32      Timer               Timer chip
//	33      Keyboard            Keyboard controller
//	128     System Call          Software (ecall)
const (
	IntDivisionByZero = 0
	IntDebug          = 1
	IntNMI            = 2
	IntBreakpoint     = 3
	IntOverflow       = 4
	IntInvalidOpcode  = 5
	IntTimer          = 32
	IntKeyboard       = 33
	IntSyscall        = 128
)

// =========================================================================
// IDT Entry
// =========================================================================

// IDTEntrySize is the number of bytes each IDT entry occupies in memory.
// The layout is:
//
//	Byte 0-3: ISR address (little-endian uint32)
//	Byte 4:   Present (0x00 or 0x01)
//	Byte 5:   Privilege level (0x00 = kernel)
//	Byte 6-7: Reserved (0x00, 0x00)
const IDTEntrySize = 8

// IDTSize is the total number of bytes the IDT occupies: 256 entries * 8 bytes.
const IDTSize = 256 * IDTEntrySize // 2048 bytes

// IDTBaseAddress is the default memory location of the IDT. The first 2 KB
// of memory (address 0x00000000) are reserved for the interrupt descriptor
// table, matching the convention established in our BIOS spec (S01).
const IDTBaseAddress = 0x00000000

// IDTEntry represents one row in the Interrupt Descriptor Table. It maps
// an interrupt number to the memory address of its handler function (ISR).
//
//	ISR Address:     Where the CPU jumps when this interrupt fires.
//	Present:         true = valid entry. false = unused (triggers double
//	                 fault if this interrupt fires).
//	PrivilegeLevel:  0 = kernel only. User programs cannot invoke directly.
type IDTEntry struct {
	ISRAddress     uint32 // Address of the interrupt service routine
	Present        bool   // True if this entry is valid
	PrivilegeLevel int    // 0 = kernel only
}

// =========================================================================
// Interrupt Descriptor Table
// =========================================================================

// InterruptDescriptorTable is an array of 256 entries stored at address
// 0x00000000 in memory. Each entry maps an interrupt number (0-255) to the
// address of its handler.
//
// Why 256 entries? This matches x86 convention and provides plenty of room:
//
//	0-31:   CPU exceptions (division by zero, invalid opcode, page fault)
//	32-47:  Hardware device interrupts (timer, keyboard)
//	48-127: Available for future use
//	128:    System call (ecall)
//	129-255: Available for future use
//
// Most entries are unused (Present = false). The BIOS populates the IDT
// during boot; the kernel may modify entries later.
type InterruptDescriptorTable struct {
	Entries [256]IDTEntry
}

// NewIDT creates a new Interrupt Descriptor Table with all 256 entries
// initialized to not-present. This is the initial state before the BIOS
// or kernel installs any interrupt handlers.
func NewIDT() *InterruptDescriptorTable {
	return &InterruptDescriptorTable{}
}

// SetEntry installs a handler at the given interrupt number (0-255).
// If the number is out of range, it panics -- this is a programming error,
// not a runtime condition.
func (idt *InterruptDescriptorTable) SetEntry(number int, entry IDTEntry) {
	if number < 0 || number > 255 {
		panic("IDT entry number must be 0-255")
	}
	idt.Entries[number] = entry
}

// GetEntry returns the entry for the given interrupt number (0-255).
func (idt *InterruptDescriptorTable) GetEntry(number int) IDTEntry {
	if number < 0 || number > 255 {
		panic("IDT entry number must be 0-255")
	}
	return idt.Entries[number]
}

// WriteToMemory serializes the entire IDT into a byte slice at the given
// base address. Each entry occupies 8 bytes in little-endian format:
//
//	Offset 0-3: ISR address (uint32, little-endian)
//	Offset 4:   Present bit (0x00 or 0x01)
//	Offset 5:   Privilege level (uint8)
//	Offset 6-7: Reserved (zeroed)
//
// The memory slice must be large enough to hold baseAddress + 2048 bytes.
// RISC-V is little-endian, so we use binary.LittleEndian throughout.
func (idt *InterruptDescriptorTable) WriteToMemory(memory []byte, baseAddress uint32) {
	for i := 0; i < 256; i++ {
		offset := int(baseAddress) + i*IDTEntrySize
		entry := idt.Entries[i]

		// Bytes 0-3: ISR address (little-endian)
		binary.LittleEndian.PutUint32(memory[offset:offset+4], entry.ISRAddress)

		// Byte 4: Present bit
		if entry.Present {
			memory[offset+4] = 0x01
		} else {
			memory[offset+4] = 0x00
		}

		// Byte 5: Privilege level
		memory[offset+5] = byte(entry.PrivilegeLevel)

		// Bytes 6-7: Reserved
		memory[offset+6] = 0x00
		memory[offset+7] = 0x00
	}
}

// LoadFromMemory deserializes the IDT from a byte slice at the given base
// address, reversing the format written by WriteToMemory.
func (idt *InterruptDescriptorTable) LoadFromMemory(memory []byte, baseAddress uint32) {
	for i := 0; i < 256; i++ {
		offset := int(baseAddress) + i*IDTEntrySize

		// Bytes 0-3: ISR address (little-endian)
		idt.Entries[i].ISRAddress = binary.LittleEndian.Uint32(memory[offset : offset+4])

		// Byte 4: Present bit
		idt.Entries[i].Present = memory[offset+4] != 0x00

		// Byte 5: Privilege level
		idt.Entries[i].PrivilegeLevel = int(memory[offset+5])
	}
}
