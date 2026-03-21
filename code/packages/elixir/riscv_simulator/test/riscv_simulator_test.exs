defmodule CodingAdventures.RiscvSimulatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.RiscvSimulator.{CSR, Encoding}

  describe "CSR" do
    test "read/write" do
      c = CSR.new() |> CSR.write(CSR.mstatus(), 0x42)
      assert CSR.read(c, CSR.mstatus()) == 0x42
    end

    test "uninitialized reads as 0" do
      assert CSR.read(CSR.new(), CSR.mtvec()) == 0
    end

    test "read_write swaps values" do
      c = CSR.new() |> CSR.write(CSR.mstatus(), 0x42)
      {old, c2} = CSR.read_write(c, CSR.mstatus(), 0x99)
      assert old == 0x42
      assert CSR.read(c2, CSR.mstatus()) == 0x99
    end

    test "read_set ORs bits" do
      c = CSR.new() |> CSR.write(CSR.mstatus(), 0b0100)
      {old, c2} = CSR.read_set(c, CSR.mstatus(), 0b0011)
      assert old == 0b0100
      assert CSR.read(c2, CSR.mstatus()) == 0b0111
    end

    test "read_clear ANDs NOT bits" do
      c = CSR.new() |> CSR.write(CSR.mstatus(), 0b0111)
      {old, c2} = CSR.read_clear(c, CSR.mstatus(), 0b0011)
      assert old == 0b0111
      assert CSR.read(c2, CSR.mstatus()) == 0b0100
    end
  end

  describe "Encoding" do
    test "assemble produces little-endian bytes" do
      assert Encoding.assemble([0x12345678]) == [0x78, 0x56, 0x34, 0x12]
    end

    test "encode_addi produces valid instruction" do
      # addi x1, x0, 42 should be a valid 32-bit instruction
      instr = Encoding.encode_addi(1, 0, 42)
      assert is_integer(instr)
      assert instr > 0
      # opcode should be 0b0010011
      assert Bitwise.band(instr, 0x7F) == 0b0010011
    end

    test "encode_add produces valid instruction" do
      instr = Encoding.encode_add(3, 1, 2)
      assert Bitwise.band(instr, 0x7F) == 0b0110011
    end

    test "encode_ecall produces system opcode" do
      assert Encoding.encode_ecall() == 0b1110011
    end

    test "encode_lui loads upper 20 bits" do
      instr = Encoding.encode_lui(1, 0x12345)
      assert Bitwise.band(instr, 0x7F) == 0b0110111
    end

    test "encode_jal produces valid jump" do
      instr = Encoding.encode_jal(1, 8)
      assert Bitwise.band(instr, 0x7F) == 0b1101111
    end

    test "encode_beq produces valid branch" do
      instr = Encoding.encode_beq(1, 2, 8)
      assert Bitwise.band(instr, 0x7F) == 0b1100011
    end

    test "encode_sw produces valid store" do
      instr = Encoding.encode_sw(1, 0, 0x100)
      assert Bitwise.band(instr, 0x7F) == 0b0100011
    end

    test "encode_lw produces valid load" do
      instr = Encoding.encode_lw(2, 0, 0x100)
      assert Bitwise.band(instr, 0x7F) == 0b0000011
    end
  end
end
