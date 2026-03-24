# ==========================================================================
# ARM1 Behavioral Simulator Tests
# ==========================================================================
#
# Comprehensive tests covering every subsystem:
#   - Constants and type helpers
#   - Condition code evaluation
#   - Barrel shifter (all 4 shift types + RRX)
#   - ALU (all 16 operations with flag behavior)
#   - Instruction decoder
#   - CPU execution (MOV, ADD, SUB, branch, load/store, block transfer)
#   - End-to-end programs (Fibonacci, loops)

defmodule CodingAdventures.Arm1SimulatorTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.Arm1Simulator, as: Sim
  alias CodingAdventures.Arm1Simulator.Flags

  # =========================================================================
  # Types and Constants
  # =========================================================================

  test "mode_string returns correct names" do
    assert Sim.mode_string(Sim.mode_usr()) == "USR"
    assert Sim.mode_string(Sim.mode_fiq()) == "FIQ"
    assert Sim.mode_string(Sim.mode_irq()) == "IRQ"
    assert Sim.mode_string(Sim.mode_svc()) == "SVC"
    assert Sim.mode_string(99) == "???"
  end

  test "op_string returns correct mnemonics" do
    assert Sim.op_string(Sim.op_add()) == "ADD"
    assert Sim.op_string(Sim.op_mov()) == "MOV"
    assert Sim.op_string(Sim.op_sub()) == "SUB"
    assert Sim.op_string(99) == "???"
  end

  test "test_op? identifies test-only operations" do
    assert Sim.test_op?(Sim.op_tst())
    assert Sim.test_op?(Sim.op_teq())
    assert Sim.test_op?(Sim.op_cmp())
    assert Sim.test_op?(Sim.op_cmn())
    refute Sim.test_op?(Sim.op_add())
    refute Sim.test_op?(Sim.op_mov())
  end

  test "logical_op? identifies logical operations" do
    assert Sim.logical_op?(Sim.op_and())
    assert Sim.logical_op?(Sim.op_mov())
    assert Sim.logical_op?(Sim.op_eor())
    refute Sim.logical_op?(Sim.op_add())
    refute Sim.logical_op?(Sim.op_sub())
  end

  test "shift_string returns correct mnemonics" do
    assert Sim.shift_string(Sim.shift_lsl()) == "LSL"
    assert Sim.shift_string(Sim.shift_lsr()) == "LSR"
    assert Sim.shift_string(Sim.shift_asr()) == "ASR"
    assert Sim.shift_string(Sim.shift_ror()) == "ROR"
    assert Sim.shift_string(99) == "???"
  end

  test "constant accessor functions return correct values" do
    assert Sim.flag_n() == 1 <<< 31
    assert Sim.flag_z() == 1 <<< 30
    assert Sim.flag_c() == 1 <<< 29
    assert Sim.flag_v() == 1 <<< 28
    assert Sim.pc_mask() == 0x03FFFFFC
    assert Sim.mode_mask() == 0x3
    assert Sim.halt_swi() == 0x123456
  end

  # =========================================================================
  # Condition Evaluator
  # =========================================================================

  describe "evaluate_condition" do
    test "EQ when Z set" do
      assert Sim.evaluate_condition(Sim.cond_eq(), %Flags{z: true})
    end

    test "EQ when Z clear" do
      refute Sim.evaluate_condition(Sim.cond_eq(), %Flags{})
    end

    test "NE when Z clear" do
      assert Sim.evaluate_condition(Sim.cond_ne(), %Flags{})
    end

    test "NE when Z set" do
      refute Sim.evaluate_condition(Sim.cond_ne(), %Flags{z: true})
    end

    test "CS when C set" do
      assert Sim.evaluate_condition(Sim.cond_cs(), %Flags{c: true})
    end

    test "CC when C clear" do
      assert Sim.evaluate_condition(Sim.cond_cc(), %Flags{})
    end

    test "MI when N set" do
      assert Sim.evaluate_condition(Sim.cond_mi(), %Flags{n: true})
    end

    test "PL when N clear" do
      assert Sim.evaluate_condition(Sim.cond_pl(), %Flags{})
    end

    test "VS when V set" do
      assert Sim.evaluate_condition(Sim.cond_vs(), %Flags{v: true})
    end

    test "VC when V clear" do
      assert Sim.evaluate_condition(Sim.cond_vc(), %Flags{})
    end

    test "HI when C=1,Z=0" do
      assert Sim.evaluate_condition(Sim.cond_hi(), %Flags{c: true})
    end

    test "HI fails when C=1,Z=1" do
      refute Sim.evaluate_condition(Sim.cond_hi(), %Flags{c: true, z: true})
    end

    test "LS when C=0" do
      assert Sim.evaluate_condition(Sim.cond_ls(), %Flags{})
    end

    test "LS when Z=1" do
      assert Sim.evaluate_condition(Sim.cond_ls(), %Flags{c: true, z: true})
    end

    test "GE when N=V=0" do
      assert Sim.evaluate_condition(Sim.cond_ge(), %Flags{})
    end

    test "GE when N=V=1" do
      assert Sim.evaluate_condition(Sim.cond_ge(), %Flags{n: true, v: true})
    end

    test "GE fails when N!=V" do
      refute Sim.evaluate_condition(Sim.cond_ge(), %Flags{n: true})
    end

    test "LT when N!=V" do
      assert Sim.evaluate_condition(Sim.cond_lt(), %Flags{n: true})
    end

    test "LT fails when N=V" do
      refute Sim.evaluate_condition(Sim.cond_lt(), %Flags{})
    end

    test "GT when Z=0,N=V" do
      assert Sim.evaluate_condition(Sim.cond_gt(), %Flags{})
    end

    test "GT fails when Z=1" do
      refute Sim.evaluate_condition(Sim.cond_gt(), %Flags{z: true})
    end

    test "LE when Z=1" do
      assert Sim.evaluate_condition(Sim.cond_le(), %Flags{z: true})
    end

    test "LE when N!=V" do
      assert Sim.evaluate_condition(Sim.cond_le(), %Flags{n: true})
    end

    test "AL always" do
      assert Sim.evaluate_condition(Sim.cond_al(), %Flags{})
    end

    test "NV never" do
      refute Sim.evaluate_condition(Sim.cond_nv(), %Flags{})
    end
  end

  # =========================================================================
  # Barrel Shifter
  # =========================================================================

  describe "barrel_shift LSL" do
    test "LSL #0 (no shift)" do
      assert {0xFF, false} = Sim.barrel_shift(0xFF, Sim.shift_lsl(), 0, false, false)
    end

    test "LSL #1" do
      assert {0x1FE, false} = Sim.barrel_shift(0xFF, Sim.shift_lsl(), 1, false, false)
    end

    test "LSL #4" do
      assert {0xFF0, false} = Sim.barrel_shift(0xFF, Sim.shift_lsl(), 4, false, false)
    end

    test "LSL #31" do
      assert {0x80000000, false} = Sim.barrel_shift(1, Sim.shift_lsl(), 31, false, false)
    end

    test "LSL #32 sets carry from bit 0" do
      assert {0, true} = Sim.barrel_shift(1, Sim.shift_lsl(), 32, false, false)
    end

    test "LSL #33 clears carry" do
      assert {0, false} = Sim.barrel_shift(1, Sim.shift_lsl(), 33, false, false)
    end
  end

  describe "barrel_shift LSR" do
    test "LSR #1" do
      {val, carry} = Sim.barrel_shift(0xFF, Sim.shift_lsr(), 1, false, false)
      assert val == 0x7F
      assert carry == true
    end

    test "LSR #8" do
      {val, carry} = Sim.barrel_shift(0xFF00, Sim.shift_lsr(), 8, false, false)
      assert val == 0xFF
      assert carry == false
    end

    test "LSR #0 (encodes #32)" do
      {val, carry} = Sim.barrel_shift(0x80000000, Sim.shift_lsr(), 0, false, false)
      assert val == 0
      assert carry == true
    end

    test "LSR #32 by register" do
      {val, carry} = Sim.barrel_shift(0x80000000, Sim.shift_lsr(), 32, false, true)
      assert val == 0
      assert carry == true
    end
  end

  describe "barrel_shift ASR" do
    test "ASR #1 positive" do
      {val, carry} = Sim.barrel_shift(0x7FFFFFFE, Sim.shift_asr(), 1, false, false)
      assert val == 0x3FFFFFFF
      assert carry == false
    end

    test "ASR #1 negative" do
      {val, carry} = Sim.barrel_shift(0x80000000, Sim.shift_asr(), 1, false, false)
      assert val == 0xC0000000
      assert carry == false
    end

    test "ASR #0 (encodes #32) negative" do
      {val, carry} = Sim.barrel_shift(0x80000000, Sim.shift_asr(), 0, false, false)
      assert val == 0xFFFFFFFF
      assert carry == true
    end

    test "ASR #0 (encodes #32) positive" do
      {val, carry} = Sim.barrel_shift(0x7FFFFFFF, Sim.shift_asr(), 0, false, false)
      assert val == 0
      assert carry == false
    end
  end

  describe "barrel_shift ROR" do
    test "ROR #4" do
      {val, carry} = Sim.barrel_shift(0x0000000F, Sim.shift_ror(), 4, false, false)
      assert val == 0xF0000000
      assert carry == true
    end

    test "ROR #8" do
      {val, carry} = Sim.barrel_shift(0x000000FF, Sim.shift_ror(), 8, false, false)
      assert val == 0xFF000000
      assert carry == true
    end

    test "ROR #16" do
      {val, carry} = Sim.barrel_shift(0x0000FFFF, Sim.shift_ror(), 16, false, false)
      assert val == 0xFFFF0000
      assert carry == true
    end
  end

  describe "barrel_shift RRX" do
    test "RRX with carry=1 and bit0=1" do
      {val, carry} = Sim.barrel_shift(0x00000001, Sim.shift_ror(), 0, true, false)
      assert val == 0x80000000
      assert carry == true
    end

    test "RRX with carry=1 and bit0=0" do
      {val, carry} = Sim.barrel_shift(0x00000000, Sim.shift_ror(), 0, true, false)
      assert val == 0x80000000
      assert carry == false
    end
  end

  describe "barrel_shift by register with amount=0" do
    test "value passes through unchanged" do
      {val, carry} = Sim.barrel_shift(0xDEADBEEF, Sim.shift_lsl(), 0, true, true)
      assert val == 0xDEADBEEF
      assert carry == true
    end
  end

  describe "decode_immediate" do
    test "no rotation" do
      assert {0xFF, false} = Sim.decode_immediate(0xFF, 0)
    end

    test "rotate 1 (ROR 2)" do
      {val, _} = Sim.decode_immediate(0x01, 1)
      assert val == 0x40000000
    end

    test "rotate 4 (ROR 8)" do
      {val, _} = Sim.decode_immediate(0xFF, 4)
      assert val == 0xFF000000
    end
  end

  # =========================================================================
  # ALU
  # =========================================================================

  describe "ALU ADD" do
    test "1 + 2 = 3 with no flags" do
      r = Sim.alu_execute(Sim.op_add(), 1, 2, false, false, false)
      assert r.result == 3
      refute r.n
      refute r.z
      refute r.c
      refute r.v
    end

    test "signed overflow: 0x7FFFFFFF + 1" do
      r = Sim.alu_execute(Sim.op_add(), 0x7FFFFFFF, 1, false, false, false)
      assert r.result == 0x80000000
      assert r.n
      assert r.v
    end

    test "unsigned overflow: 0xFFFFFFFF + 1" do
      r = Sim.alu_execute(Sim.op_add(), 0xFFFFFFFF, 1, false, false, false)
      assert r.result == 0
      assert r.c
      assert r.z
    end
  end

  describe "ALU SUB" do
    test "5 - 3 = 2, carry set (no borrow)" do
      r = Sim.alu_execute(Sim.op_sub(), 5, 3, false, false, false)
      assert r.result == 2
      assert r.c
    end

    test "3 - 5 = -2 (borrow, carry clear)" do
      r = Sim.alu_execute(Sim.op_sub(), 3, 5, false, false, false)
      assert r.result == 0xFFFFFFFE
      refute r.c
      assert r.n
    end
  end

  describe "ALU RSB" do
    test "RSB: op2 - rn = 5 - 3 = 2" do
      r = Sim.alu_execute(Sim.op_rsb(), 3, 5, false, false, false)
      assert r.result == 2
    end
  end

  describe "ALU ADC" do
    test "1 + 2 + carry = 4" do
      r = Sim.alu_execute(Sim.op_adc(), 1, 2, true, false, false)
      assert r.result == 4
    end
  end

  describe "ALU SBC" do
    test "5 - 3 - 0 = 2 when C=1" do
      r = Sim.alu_execute(Sim.op_sbc(), 5, 3, true, false, false)
      assert r.result == 2
    end
  end

  describe "ALU logical operations" do
    test "AND" do
      r = Sim.alu_execute(Sim.op_and(), 0xFF00FF00, 0x0FF00FF0, false, false, false)
      assert r.result == 0x0F000F00
    end

    test "EOR" do
      r = Sim.alu_execute(Sim.op_eor(), 0xFF00FF00, 0x0FF00FF0, false, false, false)
      assert r.result == 0xF0F0F0F0
    end

    test "ORR" do
      r = Sim.alu_execute(Sim.op_orr(), 0xFF00FF00, 0x0FF00FF0, false, false, false)
      assert r.result == 0xFFF0FFF0
    end

    test "BIC" do
      r = Sim.alu_execute(Sim.op_bic(), 0xFFFFFFFF, 0x0000FF00, false, false, false)
      assert r.result == 0xFFFF00FF
    end

    test "MOV" do
      r = Sim.alu_execute(Sim.op_mov(), 0, 42, false, false, false)
      assert r.result == 42
    end

    test "MVN" do
      r = Sim.alu_execute(Sim.op_mvn(), 0, 0, false, false, false)
      assert r.result == 0xFFFFFFFF
    end
  end

  describe "ALU test ops" do
    test "TST does not write result" do
      r = Sim.alu_execute(Sim.op_tst(), 0xFF, 0x00, false, false, false)
      refute r.write_result
      assert r.z
    end

    test "CMP 5-5 sets Z and C" do
      r = Sim.alu_execute(Sim.op_cmp(), 5, 5, false, false, false)
      refute r.write_result
      assert r.z
      assert r.c
    end
  end

  # =========================================================================
  # Decoder
  # =========================================================================

  describe "decode" do
    test "data processing: ADD R2, R0, R1 (0xE0802001)" do
      d = Sim.decode(0xE0802001)
      assert d.type == Sim.inst_data_processing()
      assert d.condition == Sim.cond_al()
      assert d.opcode == Sim.op_add()
      refute d.s
      assert d.rn == 0
      assert d.rd == 2
      assert d.rm == 1
    end

    test "MOV R0, #42 (0xE3A0002A)" do
      d = Sim.decode(0xE3A0002A)
      assert d.type == Sim.inst_data_processing()
      assert d.opcode == Sim.op_mov()
      assert d.immediate
      assert d.rd == 0
      assert d.imm8 == 42
    end

    test "branch B +8 (0xEA000002)" do
      d = Sim.decode(0xEA000002)
      assert d.type == Sim.inst_branch()
      refute d.link
      assert d.branch_offset == 8
    end

    test "branch BL -8 (0xEBFFFFFE)" do
      d = Sim.decode(0xEBFFFFFE)
      assert d.type == Sim.inst_branch()
      assert d.link
      assert d.branch_offset == -8
    end

    test "SWI 0x123456 (0xEF123456)" do
      d = Sim.decode(0xEF123456)
      assert d.type == Sim.inst_swi()
      assert d.swi_comment == 0x123456
    end
  end

  describe "disassemble" do
    test "MOV R0, #42" do
      assert Sim.disassemble(Sim.decode(0xE3A0002A)) == "MOV R0, #42"
    end

    test "ADD R2, R0, R1" do
      assert Sim.disassemble(Sim.decode(0xE0802001)) == "ADD R2, R0, R1"
    end

    test "ADDS R2, R1, R1" do
      assert Sim.disassemble(Sim.decode(0xE0912001)) == "ADDS R2, R1, R1"
    end

    test "ADDNE R2, R0, R1" do
      assert Sim.disassemble(Sim.decode(0x10802001)) == "ADDNE R2, R0, R1"
    end

    test "HLT (SWI 0x123456)" do
      assert Sim.disassemble(Sim.decode(0xEF123456)) == "HLT"
    end
  end

  # =========================================================================
  # CPU — Power-on State
  # =========================================================================

  describe "new CPU" do
    test "starts in SVC mode with IRQ/FIQ disabled" do
      cpu = Sim.new(1024)
      assert Sim.mode(cpu) == Sim.mode_svc()
      assert Sim.pc(cpu) == 0
      f = Sim.flags(cpu)
      refute f.n
      refute f.z
      refute f.c
      refute f.v
    end

    test "reset clears state" do
      cpu = Sim.new(1024)
      cpu = Sim.write_register(cpu, 0, 42)
      cpu = Sim.reset(cpu)
      assert Sim.read_register(cpu, 0) == 0
      assert Sim.mode(cpu) == Sim.mode_svc()
    end
  end

  # =========================================================================
  # CPU — Memory Operations
  # =========================================================================

  describe "memory operations" do
    test "read/write word round-trip" do
      cpu = Sim.new(1024)
      cpu = Sim.write_word(cpu, 0, 0xDEADBEEF)
      assert Sim.read_word(cpu, 0) == 0xDEADBEEF
    end

    test "read/write byte round-trip" do
      cpu = Sim.new(1024)
      cpu = Sim.write_byte(cpu, 0, 0xAB)
      assert Sim.read_byte(cpu, 0) == 0xAB
    end

    test "out-of-bounds read returns 0" do
      cpu = Sim.new(16)
      assert Sim.read_word(cpu, 100) == 0
    end
  end

  # =========================================================================
  # CPU — Basic Programs
  # =========================================================================

  describe "MOV immediate" do
    test "MOV R0, #42" do
      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 42),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 10)
      assert Sim.read_register(cpu, 0) == 42
    end
  end

  describe "1 + 2 program" do
    test "R0=1, R1=2, R2=R0+R1=3" do
      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 1),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 2),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 2, 0, 1),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 10)
      assert Sim.read_register(cpu, 0) == 1
      assert Sim.read_register(cpu, 1) == 2
      assert Sim.read_register(cpu, 2) == 3
    end
  end

  describe "SUBS with flags" do
    test "5 - 5 sets Z and C flags" do
      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 5),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 5),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_sub(), 1, 2, 0, 1),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 10)
      assert Sim.read_register(cpu, 2) == 0
      f = Sim.flags(cpu)
      assert f.z
      assert f.c
    end
  end

  describe "conditional execution" do
    test "MOVNE skipped, MOVEQ executed when Z set" do
      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 5),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 5),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_sub(), 1, 2, 0, 1),
        Sim.encode_mov_imm(Sim.cond_ne(), 3, 99),
        Sim.encode_mov_imm(Sim.cond_eq(), 4, 42),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 20)
      assert Sim.read_register(cpu, 3) == 0
      assert Sim.read_register(cpu, 4) == 42
    end
  end

  describe "barrel shifter in instruction" do
    test "ADD R1, R0, R0, LSL #2 (multiply by 5)" do
      cpu = Sim.new(1024)


      # Encode: ADD R1, R0, R0, LSL #2
      add_with_shift = (Sim.cond_al() <<< 28) |||
        (Sim.op_add() <<< 21) |||
        (0 <<< 16) |||
        (1 <<< 12) |||
        (2 <<< 7) |||
        (Sim.shift_lsl() <<< 5) |||
        0

      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 7),
        add_with_shift,
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 10)
      assert Sim.read_register(cpu, 1) == 35
    end
  end

  describe "loop: sum 1 to 10" do
    test "result is 55" do

      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 10),
        # loop:
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 0, 0, 1),
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_sub(), 1, 1, 1, (1 <<< 25) ||| 1),
        Sim.encode_branch(Sim.cond_ne(), false, -16),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 100)
      assert Sim.read_register(cpu, 0) == 55
      assert Sim.read_register(cpu, 1) == 0
    end
  end

  # =========================================================================
  # CPU — Load/Store
  # =========================================================================

  describe "LDR/STR" do
    test "store and reload value" do

      cpu = Sim.new(4096)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 42),
        # MOV R1, #256 (imm8=1, rotate=12 -> 1 ROR 24 = 256)
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 0, 0, 1, (1 <<< 25) ||| (12 <<< 8) ||| 1),
        Sim.encode_str(Sim.cond_al(), 0, 1, 0, true),
        Sim.encode_mov_imm(Sim.cond_al(), 0, 0),
        Sim.encode_ldr(Sim.cond_al(), 0, 1, 0, true),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 20)
      assert Sim.read_register(cpu, 0) == 42
    end
  end

  # =========================================================================
  # CPU — Block Transfer (LDM/STM)
  # =========================================================================

  describe "STM/LDM" do
    test "store and reload R0-R3" do

      cpu = Sim.new(4096)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 10),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 20),
        Sim.encode_mov_imm(Sim.cond_al(), 2, 30),
        Sim.encode_mov_imm(Sim.cond_al(), 3, 40),
        # MOV R5, #256
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 0, 0, 5, (1 <<< 25) ||| (12 <<< 8) ||| 1),
        Sim.encode_stm(Sim.cond_al(), 5, 0x000F, true, "IA"),
        Sim.encode_mov_imm(Sim.cond_al(), 0, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 2, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 3, 0),
        # MOV R5, #256
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 0, 0, 5, (1 <<< 25) ||| (12 <<< 8) ||| 1),
        Sim.encode_ldm(Sim.cond_al(), 5, 0x000F, true, "IA"),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 50)
      assert Sim.read_register(cpu, 0) == 10
      assert Sim.read_register(cpu, 1) == 20
      assert Sim.read_register(cpu, 2) == 30
      assert Sim.read_register(cpu, 3) == 40
    end
  end

  # =========================================================================
  # CPU — Branch and Link
  # =========================================================================

  describe "branch and link" do
    test "BL to subroutine that doubles R0" do
      cpu = Sim.new(4096)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 7),
        Sim.encode_branch(Sim.cond_al(), true, 4),
        Sim.encode_halt(),
        0,
        # double subroutine at 0x10:
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 0, 0, 0),
        # MOVS PC, LR
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_mov(), 1, 0, 15, 14)
      ])
      {cpu, _traces} = Sim.run(cpu, 20)
      assert Sim.read_register(cpu, 0) == 14
    end
  end

  # =========================================================================
  # CPU — Fibonacci
  # =========================================================================

  describe "fibonacci" do
    test "compute fib(10) iterations" do

      cpu = Sim.new(4096)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 0),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 1),
        Sim.encode_mov_imm(Sim.cond_al(), 2, 10),
        # loop:
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_add(), 0, 3, 0, 1),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_mov(), 0, 0, 0, 1),
        Sim.encode_alu_reg(Sim.cond_al(), Sim.op_mov(), 0, 1, 0, 3),
        Sim.encode_data_processing(Sim.cond_al(), Sim.op_sub(), 1, 2, 2, (1 <<< 25) ||| 1),
        Sim.encode_branch(Sim.cond_ne(), false, -24),
        Sim.encode_halt()
      ])
      {cpu, _traces} = Sim.run(cpu, 200)
      assert Sim.read_register(cpu, 1) == 89
    end
  end

  # =========================================================================
  # CPU — Halt detection
  # =========================================================================

  describe "halt" do
    test "SWI 0x123456 halts the CPU" do
      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [Sim.encode_halt()])
      {cpu, traces} = Sim.run(cpu, 10)
      assert Sim.halted?(cpu)
      assert length(traces) == 1
    end
  end

  # =========================================================================
  # CPU — Trace recording
  # =========================================================================

  describe "trace recording" do
    test "step returns correct trace" do
      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [Sim.encode_mov_imm(Sim.cond_al(), 0, 42)])
      {_cpu, trace} = Sim.step(cpu)
      assert trace.address == 0
      assert trace.condition_met == true
      assert trace.regs_after[0] == 42
    end

    test "run collects all traces" do
      cpu = Sim.new(1024)
      cpu = Sim.load_instructions(cpu, [
        Sim.encode_mov_imm(Sim.cond_al(), 0, 1),
        Sim.encode_mov_imm(Sim.cond_al(), 1, 2),
        Sim.encode_halt()
      ])
      {_cpu, traces} = Sim.run(cpu, 10)
      assert length(traces) == 3
    end
  end

  # =========================================================================
  # Encoding helpers
  # =========================================================================

  describe "encoding helpers" do
    test "encode_halt produces SWI 0x123456" do
      inst = Sim.encode_halt()
      d = Sim.decode(inst)
      assert d.type == Sim.inst_swi()
      assert d.swi_comment == 0x123456
    end

    test "encode_mov_imm round-trips" do
      inst = Sim.encode_mov_imm(Sim.cond_al(), 3, 99)
      d = Sim.decode(inst)
      assert d.opcode == Sim.op_mov()
      assert d.rd == 3
      assert d.imm8 == 99
    end

    test "encode_branch round-trips" do
      inst = Sim.encode_branch(Sim.cond_al(), false, 8)
      d = Sim.decode(inst)
      assert d.type == Sim.inst_branch()
      refute d.link
      assert d.branch_offset == 8
    end

    test "encode_branch with link" do
      inst = Sim.encode_branch(Sim.cond_al(), true, -8)
      d = Sim.decode(inst)
      assert d.link
      assert d.branch_offset == -8
    end
  end
end
