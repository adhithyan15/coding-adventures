defmodule CodingAdventures.RiscvSimulator do
  @moduledoc "RISC-V RV32I simulator with M-mode extensions and CSR support."

  defmodule CSR do
    @moduledoc "Control and Status Register file for M-mode."
    @csr_mstatus 0x300
    @csr_mtvec 0x305
    @csr_mscratch 0x340
    @csr_mepc 0x341
    @csr_mcause 0x342
    @mie Bitwise.bsl(1, 3)
    @cause_ecall_mmode 11

    def mstatus, do: @csr_mstatus
    def mtvec, do: @csr_mtvec
    def mepc, do: @csr_mepc
    def mcause, do: @csr_mcause
    def mie_bit, do: @mie
    def cause_ecall_mmode, do: @cause_ecall_mmode

    defstruct regs: %{}
    def new, do: %__MODULE__{}
    def read(%__MODULE__{regs: r}, addr), do: Map.get(r, addr, 0)
    def write(%__MODULE__{regs: r} = c, addr, value), do: %{c | regs: Map.put(r, addr, Bitwise.band(value, 0xFFFFFFFF))}
    def read_write(c, addr, new_val), do: {read(c, addr), write(c, addr, new_val)}
    def read_set(c, addr, mask) do
      import Bitwise
      old = read(c, addr)
      {old, write(c, addr, old ||| mask)}
    end
    def read_clear(c, addr, mask) do
      import Bitwise
      old = read(c, addr)
      {old, write(c, addr, old &&& ~~~mask)}
    end
  end

  defmodule Encoding do
    @moduledoc "Helpers for constructing RISC-V machine code."
    import Bitwise

    @opcode_op_imm 0b0010011
    @opcode_op     0b0110011
    @opcode_load   0b0000011
    @opcode_store  0b0100011
    @opcode_branch 0b1100011
    @opcode_jal    0b1101111
    @opcode_jalr   0b1100111
    @opcode_lui    0b0110111
    @opcode_auipc  0b0010111
    @opcode_system 0b1110011

    defp i_type(rd, rs1, imm, f3, opcode), do: (((imm &&& 0xFFF) <<< 20) ||| (rs1 <<< 15) ||| (f3 <<< 12) ||| (rd <<< 7) ||| opcode) &&& 0xFFFFFFFF
    defp r_type(rd, rs1, rs2, f3, f7), do: ((f7 <<< 25) ||| (rs2 <<< 20) ||| (rs1 <<< 15) ||| (f3 <<< 12) ||| (rd <<< 7) ||| @opcode_op) &&& 0xFFFFFFFF
    defp s_type(rs2, rs1, imm, f3) do
      iv = imm &&& 0xFFF
      il = iv &&& 0x1F
      ih = (iv >>> 5) &&& 0x7F
      ((ih <<< 25) ||| (rs2 <<< 20) ||| (rs1 <<< 15) ||| (f3 <<< 12) ||| (il <<< 7) ||| @opcode_store) &&& 0xFFFFFFFF
    end
    defp b_type(rs1, rs2, offset, f3) do
      imm = offset &&& 0x1FFE
      b12 = (imm >>> 12) &&& 0x1
      b11 = (imm >>> 11) &&& 0x1
      b10_5 = (imm >>> 5) &&& 0x3F
      b4_1 = (imm >>> 1) &&& 0xF
      ((b12 <<< 31) ||| (b10_5 <<< 25) ||| (rs2 <<< 20) ||| (rs1 <<< 15) ||| (f3 <<< 12) ||| (b4_1 <<< 8) ||| (b11 <<< 7) ||| @opcode_branch) &&& 0xFFFFFFFF
    end

    def encode_addi(rd, rs1, imm), do: i_type(rd, rs1, imm, 0, @opcode_op_imm)
    def encode_add(rd, rs1, rs2), do: r_type(rd, rs1, rs2, 0, 0)
    def encode_sub(rd, rs1, rs2), do: r_type(rd, rs1, rs2, 0, 0x20)
    def encode_lw(rd, rs1, imm), do: i_type(rd, rs1, imm, 2, @opcode_load)
    def encode_sw(rs2, rs1, imm), do: s_type(rs2, rs1, imm, 2)
    def encode_sb(rs2, rs1, imm), do: s_type(rs2, rs1, imm, 0)
    def encode_lb(rd, rs1, imm), do: i_type(rd, rs1, imm, 0, @opcode_load)
    def encode_lbu(rd, rs1, imm), do: i_type(rd, rs1, imm, 4, @opcode_load)
    def encode_beq(rs1, rs2, offset), do: b_type(rs1, rs2, offset, 0)
    def encode_bne(rs1, rs2, offset), do: b_type(rs1, rs2, offset, 1)
    def encode_jal(rd, offset) do
      imm = offset &&& 0x1FFFFE
      b20 = (imm >>> 20) &&& 0x1
      b10_1 = (imm >>> 1) &&& 0x3FF
      b11 = (imm >>> 11) &&& 0x1
      b19_12 = (imm >>> 12) &&& 0xFF
      ((b20 <<< 31) ||| (b10_1 <<< 21) ||| (b11 <<< 20) ||| (b19_12 <<< 12) ||| (rd <<< 7) ||| @opcode_jal) &&& 0xFFFFFFFF
    end
    def encode_jalr(rd, rs1, imm), do: i_type(rd, rs1, imm, 0, @opcode_jalr)
    def encode_lui(rd, imm), do: (((imm &&& 0xFFFFF) <<< 12) ||| (rd <<< 7) ||| @opcode_lui) &&& 0xFFFFFFFF
    def encode_ecall, do: @opcode_system
    def encode_mret, do: ((0x18 <<< 25) ||| (0b00010 <<< 20) ||| @opcode_system) &&& 0xFFFFFFFF

    def assemble(instructions) do
      Enum.flat_map(instructions, fn instr ->
        v = instr &&& 0xFFFFFFFF
        [v &&& 0xFF, (v >>> 8) &&& 0xFF, (v >>> 16) &&& 0xFF, (v >>> 24) &&& 0xFF]
      end)
    end
  end
end
