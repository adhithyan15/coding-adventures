# ==========================================================================
# ARM1 Gate-Level Simulator Tests
# ==========================================================================
#
# Tests cover:
#   - Bit conversion helpers
#   - Gate-level ALU (arithmetic + logical)
#   - Gate-level barrel shifter
#   - Cross-validation against the behavioral simulator
#   - CPU execution basics

defmodule CodingAdventures.Arm1GatelevelTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.Arm1Gatelevel, as: GL
  alias CodingAdventures.Arm1Simulator, as: Sim

  # =========================================================================
  # Bit Conversion
  # =========================================================================

  describe "int_to_bits / bits_to_int" do
    test "round-trip for various values" do
      for v <- [0, 1, 42, 0xFF, 0xDEADBEEF, 0xFFFFFFFF] do
        bits = GL.int_to_bits(v, 32)
        assert GL.bits_to_int(bits) == v, "round-trip failed for #{v}"
      end
    end

    test "int_to_bits(5) has correct bit pattern" do
      bits = GL.int_to_bits(5, 32)
      assert Enum.at(bits, 0) == 1  # bit 0
      assert Enum.at(bits, 1) == 0  # bit 1
      assert Enum.at(bits, 2) == 1  # bit 2
    end
  end

  # =========================================================================
  # Gate-Level ALU
  # =========================================================================

  describe "gate ALU ADD" do
    test "1 + 2 = 3" do
      a = GL.int_to_bits(1, 32)
      b = GL.int_to_bits(2, 32)
      r = GL.gate_alu_execute(Sim.op_add(), a, b, 0, 0, 0)
      assert GL.bits_to_int(r.result_bits) == 3
      assert r.n == 0
      assert r.z == 0
      assert r.c == 0
      assert r.v == 0
    end
  end

  describe "gate ALU SUB" do
    test "5 - 5 = 0, Z set, C set" do
      a = GL.int_to_bits(5, 32)
      b = GL.int_to_bits(5, 32)
      r = GL.gate_alu_execute(Sim.op_sub(), a, b, 0, 0, 0)
      assert GL.bits_to_int(r.result_bits) == 0
      assert r.z == 1
      assert r.c == 1
    end
  end

  describe "gate ALU logical" do
    test "AND" do
      a = GL.int_to_bits(0xFF00FF00, 32)
      b = GL.int_to_bits(0x0FF00FF0, 32)
      r = GL.gate_alu_execute(Sim.op_and(), a, b, 0, 0, 0)
      assert GL.bits_to_int(r.result_bits) == 0x0F000F00
    end

    test "EOR" do
      a = GL.int_to_bits(0xFF00FF00, 32)
      b = GL.int_to_bits(0x0FF00FF0, 32)
      r = GL.gate_alu_execute(Sim.op_eor(), a, b, 0, 0, 0)
      assert GL.bits_to_int(r.result_bits) == 0xF0F0F0F0
    end

    test "ORR" do
      a = GL.int_to_bits(0xFF00FF00, 32)
      b = GL.int_to_bits(0x0FF00FF0, 32)
      r = GL.gate_alu_execute(Sim.op_orr(), a, b, 0, 0, 0)
      assert GL.bits_to_int(r.result_bits) == 0xFFF0FFF0
    end
  end

  # =========================================================================
  # Gate-Level Barrel Shifter
  # =========================================================================

  describe "gate barrel shifter LSL" do
    test "LSL #4 of 0xFF" do
      value = GL.int_to_bits(0xFF, 32)
      {result, _} = GL.gate_barrel_shift(value, 0, 4, 0, false)
      assert GL.bits_to_int(result) == 0xFF0
    end
  end

  describe "gate barrel shifter LSR" do
    test "LSR #8 of 0xFF00" do
      value = GL.int_to_bits(0xFF00, 32)
      {result, _} = GL.gate_barrel_shift(value, 1, 8, 0, false)
      assert GL.bits_to_int(result) == 0xFF
    end
  end

  describe "gate barrel shifter ROR" do
    test "ROR #4 of 0xF" do
      value = GL.int_to_bits(0x0000000F, 32)
      {result, _} = GL.gate_barrel_shift(value, 3, 4, 0, false)
      assert GL.bits_to_int(result) == 0xF0000000
    end
  end

  describe "gate barrel shifter RRX" do
    test "RRX of 1 with carry=1" do
      value = GL.int_to_bits(0x00000001, 32)
      {result, carry} = GL.gate_barrel_shift(value, 3, 0, 1, false)
      assert GL.bits_to_int(result) == 0x80000000
      assert carry == 1
    end
  end

  # =========================================================================
  # Gate-Level CPU Basics
  # =========================================================================

  describe "new and reset" do
    test "starts in SVC mode, PC=0" do
      cpu = GL.new(1024)
      assert GL.mode(cpu) == Sim.mode_svc()
      assert GL.pc(cpu) == 0
    end
  end

  describe "halt" do
    test "SWI 0x123456 halts" do
      cpu = GL.new(1024)
      cpu = GL.load_instructions(cpu, [Sim.encode_halt()])
      {cpu, traces} = GL.run(cpu, 10)
      assert GL.halted?(cpu)
      assert length(traces) == 1
    end
  end

  describe "gate ops tracking" do
    test "gate ops non-zero after execution" do
      cpu = GL.new(1024)
      cpu = GL.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 42),
        Sim.encode_halt()
      ])
      {cpu, _} = GL.run(cpu, 10)
      assert GL.gate_ops(cpu) > 0
    end
  end

  # =========================================================================
  # Cross-validation: Gate-Level vs Behavioral
  # =========================================================================
  #
  # The ultimate correctness guarantee. We run the same program on both
  # simulators and verify they produce identical results.

  defp cross_validate(name, instructions) do
    behavioral = Sim.new(4096)
    gate_level = GL.new(4096)

    behavioral = Sim.load_instructions(behavioral, instructions)
    gate_level = GL.load_instructions(gate_level, instructions)

    {_b_cpu, b_traces} = Sim.run(behavioral, 200)
    {g_cpu, g_traces} = GL.run(gate_level, 200)

    assert length(b_traces) == length(g_traces),
      "#{name}: trace count mismatch: behavioral=#{length(b_traces)} gate-level=#{length(g_traces)}"

    Enum.zip(b_traces, g_traces)
    |> Enum.with_index()
    |> Enum.each(fn {{bt, gt}, i} ->
      assert bt.address == gt.address,
        "#{name} step #{i}: address mismatch: B=0x#{Integer.to_string(bt.address, 16)} G=0x#{Integer.to_string(gt.address, 16)}"

      assert bt.condition_met == gt.condition_met,
        "#{name} step #{i}: condition mismatch"

      for r <- 0..15 do
        assert bt.regs_after[r] == gt.regs_after[r],
          "#{name} step #{i}: R#{r} mismatch: B=0x#{Integer.to_string(bt.regs_after[r], 16)} G=0x#{Integer.to_string(gt.regs_after[r], 16)}"
      end

      assert bt.flags_after == gt.flags_after,
        "#{name} step #{i}: flags mismatch: B=#{inspect(bt.flags_after)} G=#{inspect(gt.flags_after)}"
    end)

    GL.gate_ops(g_cpu)
  end

  describe "cross-validation" do
    test "1 + 2" do
      gate_ops = cross_validate("1+2", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 1),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 2),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 2, 0, 1),
        Sim.encode_halt()
      ])
      assert gate_ops > 0
    end

    test "SUBS with flags" do
      cross_validate("SUBS", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 5),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 5),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_sub(), 1, 2, 0, 1),
        Sim.encode_halt()
      ])
    end

    test "conditional execution" do
      cross_validate("conditional", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 5),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 5),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_sub(), 1, 2, 0, 1),
        Sim.encode_mov_imm(Sim.cond_ne(), 3, 99),
        Sim.encode_mov_imm(Sim.cond_eq(), 4, 42),
        Sim.encode_halt()
      ])
    end

    test "barrel shifter (multiply by 5)" do
      add_with_shift = (Sim.cond_al() <<< 28) |||
        (Sim.op_add() <<< 21) |||
        (0 <<< 16) |||
        (1 <<< 12) |||
        (2 <<< 7) |||
        (Sim.shift_lsl() <<< 5) |||
        0

      cross_validate("barrel_shifter", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 7),
        add_with_shift,
        Sim.encode_halt()
      ])
    end

    test "loop: sum 1 to 10" do
      cross_validate("loop_sum_1_to_10", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 10),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 0, 0, 1),
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_sub(), 1, 1, 1, (1 <<< 25) ||| 1),
        Sim.encode_branch(Sim.cond_ne(), false, -16),
        Sim.encode_halt()
      ])
    end

    test "LDR/STR" do
      cross_validate("ldr_str", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 42),
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 0, 0, 1, (1 <<< 25) ||| (12 <<< 8) ||| 1),
        Sim.encode_str(Sim.cond_al(), 0, 1, 0, true),
        Sim.encode_mov_imm(Sim.cond_al(), 0, 0),
        Sim.encode_ldr(Sim.cond_al(), 0, 1, 0, true),
        Sim.encode_halt()
      ])
    end

    test "STM/LDM" do
      cross_validate("stm_ldm", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 10),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 20),
        Sim.encode_mov_imm(Sim.cond_al(), 2, 30),
        Sim.encode_mov_imm(Sim.cond_al(), 3, 40),
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 0, 0, 5, (1 <<< 25) ||| (12 <<< 8) ||| 1),
        Sim.encode_stm(Sim.cond_al(), 5, 0x000F, true, "IA"),
        Sim.encode_mov_imm(Sim.cond_al(), 0, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 2, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 3, 0),
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 0, 0, 5, (1 <<< 25) ||| (12 <<< 8) ||| 1),
        Sim.encode_ldm(Sim.cond_al(), 5, 0x000F, true, "IA"),
        Sim.encode_halt()
      ])
    end

    test "branch and link" do
      cross_validate("branch_and_link", [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 7),
        Sim.encode_branch(Sim.cond_al(), true, 4),
        Sim.encode_halt(),
        0,
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 0, 0, 0),
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 1, 0, 15, 14)
      ])
    end
  end
end
