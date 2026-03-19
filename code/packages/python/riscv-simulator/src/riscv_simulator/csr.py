"""Control and Status Register (CSR) file for M-mode.

=== What are CSRs? ===

Control and Status Registers are special-purpose registers that control
CPU behavior at a level above normal computation. While the general-purpose
registers (x0-x31) hold data your program works with, CSRs control things
like:

  - Whether interrupts are enabled (mstatus)
  - Where to jump when a trap occurs (mtvec)
  - What caused the most recent trap (mcause)
  - Where to return after handling a trap (mepc)

=== What is M-mode? ===

RISC-V defines privilege levels. Machine mode (M-mode) is the highest
privilege level -- it has full access to all hardware. The "m" prefix on
CSR names (mstatus, mtvec, mepc, mcause, mscratch) indicates these are
Machine-mode CSRs.

=== CSR addresses ===

  0x300 = mstatus    (machine status -- interrupt enable bits)
  0x305 = mtvec      (machine trap vector -- where to jump on trap)
  0x340 = mscratch   (machine scratch -- temp storage for trap handler)
  0x341 = mepc       (machine exception PC -- saved PC on trap entry)
  0x342 = mcause     (machine cause -- why the trap happened)

=== How traps work ===

When an exception occurs (like ecall), the CPU:
  1. Save current PC to mepc
  2. Save the cause code to mcause
  3. Disable interrupts (clear MIE bit in mstatus)
  4. Jump to the address in mtvec (the trap handler)

The trap handler does its work, then executes "mret" to:
  1. Restore PC from mepc
  2. Re-enable interrupts (restore MIE bit in mstatus)
"""

# CSR address constants -- defined by the RISC-V privileged spec.
CSR_MSTATUS = 0x300
CSR_MTVEC = 0x305
CSR_MSCRATCH = 0x340
CSR_MEPC = 0x341
CSR_MCAUSE = 0x342

# MIE is the Machine Interrupt Enable bit within mstatus (bit 3).
MIE = 1 << 3

# Trap cause codes
CAUSE_ECALL_M_MODE = 11  # Environment call from Machine mode


class CSRFile:
    """Holds the machine-mode Control and Status Registers.

    We use a simple dict from CSR address to value. A real CPU would have
    dedicated hardware registers, but a dict gives us flexibility.
    """

    def __init__(self) -> None:
        """Create a fresh CSR file with all registers initialized to 0."""
        self._regs: dict[int, int] = {}

    def read(self, addr: int) -> int:
        """Read a CSR. Uninitialized CSRs read as 0."""
        return self._regs.get(addr, 0)

    def write(self, addr: int, value: int) -> None:
        """Write a CSR."""
        self._regs[addr] = value & 0xFFFFFFFF

    def read_write(self, addr: int, new_value: int) -> int:
        """Atomically read old value and write new value (CSRRW semantic)."""
        old = self._regs.get(addr, 0)
        self._regs[addr] = new_value & 0xFFFFFFFF
        return old

    def read_set(self, addr: int, mask: int) -> int:
        """Read old value, then set bits specified by mask (CSRRS semantic)."""
        old = self._regs.get(addr, 0)
        self._regs[addr] = (old | mask) & 0xFFFFFFFF
        return old

    def read_clear(self, addr: int, mask: int) -> int:
        """Read old value, then clear bits specified by mask (CSRRC semantic)."""
        old = self._regs.get(addr, 0)
        self._regs[addr] = (old & ~mask) & 0xFFFFFFFF
        return old
