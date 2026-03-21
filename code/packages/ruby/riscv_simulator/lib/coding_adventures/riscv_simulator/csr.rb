# frozen_string_literal: true

# === Control and Status Register (CSR) file for M-mode ===
#
# CSRs control CPU behavior at a level above normal computation:
#   mstatus  (0x300) -- interrupt enable bits
#   mtvec    (0x305) -- trap handler address
#   mscratch (0x340) -- scratch register for trap handlers
#   mepc     (0x341) -- saved PC on trap entry
#   mcause   (0x342) -- why the trap happened

module CodingAdventures
  module RiscvSimulator
    # CSR address constants
    CSR_MSTATUS  = 0x300
    CSR_MTVEC    = 0x305
    CSR_MSCRATCH = 0x340
    CSR_MEPC     = 0x341
    CSR_MCAUSE   = 0x342

    # MIE = Machine Interrupt Enable bit (bit 3 of mstatus)
    MIE = 1 << 3

    # Trap cause codes
    CAUSE_ECALL_M_MODE = 11

    MASK32 = 0xFFFFFFFF

    class CSRFile
      def initialize
        @regs = {}
      end

      def read(addr)
        @regs.fetch(addr, 0)
      end

      def write(addr, value)
        @regs[addr] = value & MASK32
      end

      # CSRRW: atomically read old, write new
      def read_write(addr, new_value)
        old = @regs.fetch(addr, 0)
        @regs[addr] = new_value & MASK32
        old
      end

      # CSRRS: read old, set bits
      def read_set(addr, mask)
        old = @regs.fetch(addr, 0)
        @regs[addr] = (old | mask) & MASK32
        old
      end

      # CSRRC: read old, clear bits
      def read_clear(addr, mask)
        old = @regs.fetch(addr, 0)
        @regs[addr] = (old & ~mask) & MASK32
        old
      end
    end
  end
end
