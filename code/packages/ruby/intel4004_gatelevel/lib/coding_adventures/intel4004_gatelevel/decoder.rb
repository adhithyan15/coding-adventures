# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Instruction decoder -- combinational logic that maps opcodes to control signals.
# ---------------------------------------------------------------------------
#
# === How instruction decoding works in hardware ===
#
# The decoder takes an 8-bit instruction byte and produces control signals
# that tell the rest of the CPU what to do. In the real 4004, this was a
# combinational logic network -- a forest of AND, OR, and NOT gates that
# pattern-match the opcode bits.
#
# For example, to detect LDM (0xD_):
#     is_ldm = AND(bit7, bit6, NOT(bit5), bit4)  -> bits 7654 = 1101
#
# The decoder does not use sequential logic -- it is purely combinational.
# Given the same input bits, it always produces the same output signals.
#
# === Control signals ===
#
# The decoder outputs tell the control unit what to do:
#     - is_ldm:       Load immediate into accumulator
#     - is_add:       Route through ALU add
#     - is_sub:       Route through ALU subtract
#     - is_jun:       Unconditional jump
#     - is_jms:       Jump to subroutine (push return address)
#     - is_bbl:       Return from subroutine (pop return address)
#     - is_two_byte:  Instruction is 2 bytes
#     - reg_index:    Which register (lower nibble)
#     - pair_index:   Which register pair
#     - immediate:    Immediate value from instruction
# ---------------------------------------------------------------------------

require "coding_adventures_logic_gates"

module CodingAdventures
  module Intel4004Gatelevel
    # Control signals produced by the instruction decoder.
    #
    # Every field represents a wire carrying a 0 or 1 signal, or a
    # multi-bit value extracted from the instruction.
    DecodedInstruction = Data.define(
      :raw, :raw2,
      :upper, :lower,
      :is_nop, :is_hlt, :is_ldm, :is_ld, :is_xch, :is_inc,
      :is_add, :is_sub, :is_jun, :is_jcn, :is_isz, :is_jms,
      :is_bbl, :is_fim, :is_src, :is_fin, :is_jin,
      :is_io, :is_accum,
      :is_two_byte,
      :reg_index, :pair_index, :immediate, :condition,
      :addr12, :addr8
    )

    module Decoder
      # Decode an instruction byte into control signals using gates.
      #
      # In real hardware, this is a combinational circuit -- no clock needed.
      # The input bits propagate through AND/OR/NOT gate trees to produce
      # the output control signals.
      #
      # @param raw [Integer] first instruction byte (0x00-0xFF)
      # @param raw2 [Integer, nil] second byte for 2-byte instructions, or nil
      # @return [DecodedInstruction] with all control signals set
      def self.decode(raw, raw2 = nil)
        # Extract individual bits using AND gates (masking)
        b7 = (raw >> 7) & 1
        b6 = (raw >> 6) & 1
        b5 = (raw >> 5) & 1
        b4 = (raw >> 4) & 1
        b3 = (raw >> 3) & 1
        b2 = (raw >> 2) & 1
        b1 = (raw >> 1) & 1
        b0 = raw & 1

        upper = (raw >> 4) & 0xF
        lower = raw & 0xF

        # --- Instruction family detection ---
        # Each family is detected by AND-ing the upper nibble bits.
        # Using NOT for inverted bits.

        # NOP = 0x00: all bits zero
        is_nop = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), LogicGates.not_gate(b6)),
          LogicGates.and_gate(
            LogicGates.and_gate(LogicGates.not_gate(b5), LogicGates.not_gate(b4)),
            LogicGates.and_gate(LogicGates.not_gate(b3), LogicGates.not_gate(b2))
          )
        )
        is_nop = LogicGates.and_gate(is_nop, LogicGates.and_gate(LogicGates.not_gate(b1), LogicGates.not_gate(b0)))

        # HLT = 0x01: only b0 is 1
        is_hlt = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), LogicGates.not_gate(b6)),
          LogicGates.and_gate(
            LogicGates.and_gate(LogicGates.not_gate(b5), LogicGates.not_gate(b4)),
            LogicGates.and_gate(LogicGates.not_gate(b3), LogicGates.not_gate(b2))
          )
        )
        is_hlt = LogicGates.and_gate(is_hlt, LogicGates.and_gate(LogicGates.not_gate(b1), b0))

        # Upper nibble patterns (using gate logic):
        # 0x1_ = 0001 : JCN
        is_jcn_family = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), LogicGates.not_gate(b6)),
          LogicGates.and_gate(LogicGates.not_gate(b5), b4)
        )

        # 0x2_ = 0010 : FIM (even b0) or SRC (odd b0)
        is_2x = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), LogicGates.not_gate(b6)),
          LogicGates.and_gate(b5, LogicGates.not_gate(b4))
        )
        is_fim = LogicGates.and_gate(is_2x, LogicGates.not_gate(b0))
        is_src = LogicGates.and_gate(is_2x, b0)

        # 0x3_ = 0011 : FIN (even b0) or JIN (odd b0)
        is_3x = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), LogicGates.not_gate(b6)),
          LogicGates.and_gate(b5, b4)
        )
        is_fin = LogicGates.and_gate(is_3x, LogicGates.not_gate(b0))
        is_jin = LogicGates.and_gate(is_3x, b0)

        # 0x4_ = 0100 : JUN
        is_jun_family = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), b6),
          LogicGates.and_gate(LogicGates.not_gate(b5), LogicGates.not_gate(b4))
        )

        # 0x5_ = 0101 : JMS
        is_jms_family = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), b6),
          LogicGates.and_gate(LogicGates.not_gate(b5), b4)
        )

        # 0x6_ = 0110 : INC
        is_inc_family = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), b6),
          LogicGates.and_gate(b5, LogicGates.not_gate(b4))
        )

        # 0x7_ = 0111 : ISZ
        is_isz_family = LogicGates.and_gate(
          LogicGates.and_gate(LogicGates.not_gate(b7), b6),
          LogicGates.and_gate(b5, b4)
        )

        # 0x8_ = 1000 : ADD
        is_add_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, LogicGates.not_gate(b6)),
          LogicGates.and_gate(LogicGates.not_gate(b5), LogicGates.not_gate(b4))
        )

        # 0x9_ = 1001 : SUB
        is_sub_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, LogicGates.not_gate(b6)),
          LogicGates.and_gate(LogicGates.not_gate(b5), b4)
        )

        # 0xA_ = 1010 : LD
        is_ld_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, LogicGates.not_gate(b6)),
          LogicGates.and_gate(b5, LogicGates.not_gate(b4))
        )

        # 0xB_ = 1011 : XCH
        is_xch_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, LogicGates.not_gate(b6)),
          LogicGates.and_gate(b5, b4)
        )

        # 0xC_ = 1100 : BBL
        is_bbl_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, b6),
          LogicGates.and_gate(LogicGates.not_gate(b5), LogicGates.not_gate(b4))
        )

        # 0xD_ = 1101 : LDM
        is_ldm_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, b6),
          LogicGates.and_gate(LogicGates.not_gate(b5), b4)
        )

        # 0xE_ = 1110 : I/O operations
        is_io_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, b6),
          LogicGates.and_gate(b5, LogicGates.not_gate(b4))
        )

        # 0xF_ = 1111 : accumulator operations
        is_accum_family = LogicGates.and_gate(
          LogicGates.and_gate(b7, b6),
          LogicGates.and_gate(b5, b4)
        )

        # Two-byte detection
        is_two_byte = LogicGates.or_gate(
          LogicGates.or_gate(is_jcn_family, is_jun_family),
          LogicGates.or_gate(
            LogicGates.or_gate(is_jms_family, is_isz_family),
            is_fim
          )
        )

        # Operand extraction
        reg_index = lower
        pair_index = lower >> 1
        immediate = lower
        condition = lower

        # 12-bit address for JUN/JMS
        second = raw2.nil? ? 0 : raw2
        addr12 = (lower << 8) | second
        addr8 = second

        DecodedInstruction.new(
          raw: raw,
          raw2: raw2,
          upper: upper,
          lower: lower,
          is_nop: is_nop,
          is_hlt: is_hlt,
          is_ldm: is_ldm_family,
          is_ld: is_ld_family,
          is_xch: is_xch_family,
          is_inc: is_inc_family,
          is_add: is_add_family,
          is_sub: is_sub_family,
          is_jun: is_jun_family,
          is_jcn: is_jcn_family,
          is_isz: is_isz_family,
          is_jms: is_jms_family,
          is_bbl: is_bbl_family,
          is_fim: is_fim,
          is_src: is_src,
          is_fin: is_fin,
          is_jin: is_jin,
          is_io: is_io_family,
          is_accum: is_accum_family,
          is_two_byte: is_two_byte,
          reg_index: reg_index,
          pair_index: pair_index,
          immediate: immediate,
          condition: condition,
          addr12: addr12,
          addr8: addr8
        )
      end
    end
  end
end
