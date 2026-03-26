defmodule CodingAdventures.Intel4004SimulatorTest do
  use ExUnit.Case, async: true

  import Bitwise
  alias CodingAdventures.Intel4004Simulator, as: Sim

  # ===================================================================
  # NOP — No operation
  # ===================================================================

  describe "NOP" do
    test "does nothing" do
      {cpu, traces} = Sim.run(<<0x00, 0x01>>)
      assert cpu.accumulator == 0
      assert hd(traces).mnemonic == "NOP"
    end

    test "multiple NOPs" do
      {_cpu, traces} = Sim.run(<<0x00, 0x00, 0x00, 0x01>>)
      assert length(traces) == 4
    end
  end

  # ===================================================================
  # HLT — Halt
  # ===================================================================

  describe "HLT" do
    test "stops execution" do
      {cpu, traces} = Sim.run(<<0x01>>)
      assert cpu.halted == true
      assert length(traces) == 1
      assert hd(traces).mnemonic == "HLT"
    end

    test "mid-program halt" do
      {cpu, traces} = Sim.run(<<0x01, 0xD5>>)
      assert cpu.accumulator == 0
      assert length(traces) == 1
    end

    test "step after halt raises" do
      {cpu, _} = Sim.run(<<0x01>>)
      assert_raise RuntimeError, ~r/halted/, fn -> Sim.step(cpu) end
    end
  end

  # ===================================================================
  # LDM — Load immediate
  # ===================================================================

  describe "LDM" do
    test "loads value into accumulator" do
      {cpu, traces} = Sim.run(<<0xD5, 0x01>>)
      assert cpu.accumulator == 5
      assert hd(traces).mnemonic == "LDM 5"
    end

    test "all values 0-15" do
      for n <- 0..15 do
        {cpu, _} = Sim.run(<<0xD0 ||| n, 0x01>>)
        assert cpu.accumulator == n
      end
    end
  end

  # ===================================================================
  # LD — Load register into accumulator
  # ===================================================================

  describe "LD" do
    test "loads register value" do
      # LDM 7, XCH R0, LD R0
      {cpu, _} = Sim.run(<<0xD7, 0xB0, 0xA0, 0x01>>)
      assert cpu.accumulator == 7
    end
  end

  # ===================================================================
  # XCH — Exchange accumulator and register
  # ===================================================================

  describe "XCH" do
    test "swaps values" do
      # LDM 7, XCH R0
      {cpu, _} = Sim.run(<<0xD7, 0xB0, 0x01>>)
      assert Enum.at(cpu.registers, 0) == 7
      assert cpu.accumulator == 0
    end
  end

  # ===================================================================
  # INC — Increment register
  # ===================================================================

  describe "INC" do
    test "wraps at 15" do
      # LDM 15, XCH R0, INC R0
      {cpu, _} = Sim.run(<<0xDF, 0xB0, 0x60, 0x01>>)
      assert Enum.at(cpu.registers, 0) == 0
    end

    test "does not affect carry" do
      # Set carry, then INC
      {cpu, _} = Sim.run(<<0xDF, 0xB1, 0xDF, 0x81, 0x60, 0x01>>)
      assert cpu.carry == true
    end
  end

  # ===================================================================
  # ADD — Add register to accumulator
  # ===================================================================

  describe "ADD" do
    test "basic addition" do
      # LDM 3, XCH R0, LDM 2, ADD R0
      {cpu, _} = Sim.run(<<0xD3, 0xB0, 0xD2, 0x80, 0x01>>)
      assert cpu.accumulator == 5
      assert cpu.carry == false
    end

    test "overflow sets carry" do
      # LDM 1, XCH R0, LDM 15, ADD R0 → 16, carry=true, A=0
      {cpu, _} = Sim.run(<<0xD1, 0xB0, 0xDF, 0x80, 0x01>>)
      assert cpu.accumulator == 0
      assert cpu.carry == true
    end

    test "carry participates in addition" do
      # 15+15 → carry=1, then 1+1+carry = 3
      {cpu, _} = Sim.run(<<0xDF, 0xB0, 0xDF, 0x80, 0xD1, 0xB1, 0xD1, 0x81, 0x01>>)
      assert cpu.accumulator == 3
    end
  end

  # ===================================================================
  # SUB — Subtract register (complement-add)
  # ===================================================================

  describe "SUB" do
    test "basic subtraction" do
      # LDM 3, XCH R0, LDM 5, SUB R0 → 2, carry=true (no borrow)
      {cpu, _} = Sim.run(<<0xD3, 0xB0, 0xD5, 0x90, 0x01>>)
      assert cpu.accumulator == 2
      assert cpu.carry == true
    end

    test "underflow clears carry" do
      # LDM 1, XCH R0, LDM 0, SUB R0 → 15, carry=false (borrow)
      {cpu, _} = Sim.run(<<0xD1, 0xB0, 0xD0, 0x90, 0x01>>)
      assert cpu.accumulator == 15
      assert cpu.carry == false
    end
  end

  # ===================================================================
  # Accumulator operations
  # ===================================================================

  describe "CLB" do
    test "clears accumulator and carry" do
      {cpu, _} = Sim.run(<<0xDF, 0xB0, 0xDF, 0x80, 0xF0, 0x01>>)
      assert cpu.accumulator == 0
      assert cpu.carry == false
    end
  end

  describe "CLC" do
    test "clears carry" do
      {cpu, _} = Sim.run(<<0xDF, 0xB0, 0xDF, 0x80, 0xF1, 0x01>>)
      assert cpu.carry == false
    end
  end

  describe "IAC" do
    test "increments accumulator" do
      {cpu, _} = Sim.run(<<0xD5, 0xF2, 0x01>>)
      assert cpu.accumulator == 6
    end

    test "overflow sets carry" do
      {cpu, _} = Sim.run(<<0xDF, 0xF2, 0x01>>)
      assert cpu.accumulator == 0
      assert cpu.carry == true
    end
  end

  describe "CMC" do
    test "complements carry" do
      {cpu, _} = Sim.run(<<0xF3, 0x01>>)
      assert cpu.carry == true
    end
  end

  describe "CMA" do
    test "complements accumulator" do
      # CMA of 5 (0101) = 10 (1010)
      {cpu, _} = Sim.run(<<0xD5, 0xF4, 0x01>>)
      assert cpu.accumulator == 10
    end
  end

  describe "RAL" do
    test "rotates left through carry" do
      # LDM 5 (0101), RAL → 1010 = 10, carry=0
      {cpu, _} = Sim.run(<<0xD5, 0xF5, 0x01>>)
      assert cpu.accumulator == 0b1010
    end
  end

  describe "RAR" do
    test "rotates right through carry" do
      # LDM 4 (0100), RAR → 0010 = 2, carry=0
      {cpu, _} = Sim.run(<<0xD4, 0xF6, 0x01>>)
      assert cpu.accumulator == 2
    end
  end

  describe "TCC" do
    test "transfers carry to accumulator" do
      # STC, TCC → A=1, carry=false
      {cpu, _} = Sim.run(<<0xFA, 0xF7, 0x01>>)
      assert cpu.accumulator == 1
      assert cpu.carry == false
    end
  end

  describe "DAC" do
    test "decrements accumulator" do
      {cpu, _} = Sim.run(<<0xD5, 0xF8, 0x01>>)
      assert cpu.accumulator == 4
      assert cpu.carry == true
    end

    test "zero wraps to 15 with borrow" do
      {cpu, _} = Sim.run(<<0xD0, 0xF8, 0x01>>)
      assert cpu.accumulator == 15
      assert cpu.carry == false
    end
  end

  describe "TCS" do
    test "transfer carry subtract" do
      # STC, TCS → A=10, carry=false
      {cpu, _} = Sim.run(<<0xFA, 0xF9, 0x01>>)
      assert cpu.accumulator == 10
    end
  end

  describe "STC" do
    test "sets carry" do
      {cpu, _} = Sim.run(<<0xFA, 0x01>>)
      assert cpu.carry == true
    end
  end

  describe "DAA" do
    test "BCD adjust" do
      # LDM 12, DAA → 12+6=18, keep 2, carry=true
      {cpu, _} = Sim.run(<<0xDC, 0xFB, 0x01>>)
      assert cpu.accumulator == 2
      assert cpu.carry == true
    end
  end

  describe "KBP" do
    test "all values" do
      expected = %{0 => 0, 1 => 1, 2 => 2, 4 => 3, 8 => 4, 3 => 15, 15 => 15}

      for {input, output} <- expected do
        {cpu, _} = Sim.run(<<0xD0 ||| input, 0xFC, 0x01>>)
        assert cpu.accumulator == output, "KBP(#{input})=#{cpu.accumulator}, expected #{output}"
      end
    end
  end

  describe "DCL" do
    test "selects RAM bank" do
      {cpu, _} = Sim.run(<<0xD2, 0xFD, 0x01>>)
      assert cpu.ram_bank == 2
    end
  end

  # ===================================================================
  # Jump instructions
  # ===================================================================

  describe "JUN" do
    test "unconditional jump" do
      # JUN 0x004, LDM 5 (skipped), HLT, NOP, HLT
      {cpu, _} = Sim.run(<<0x40, 0x04, 0xD5, 0x01, 0x01>>)
      assert cpu.accumulator == 0
    end
  end

  describe "JCN" do
    test "jump on zero" do
      # JCN 4,0x04 (test zero, A=0), LDM 5 (skipped), HLT, HLT
      {cpu, _} = Sim.run(<<0x14, 0x04, 0xD5, 0x01, 0x01>>)
      assert cpu.accumulator == 0
    end

    test "no jump when condition false" do
      # LDM 3, JCN 4,0x06 (test zero, but A=3, no jump), LDM 5, HLT, HLT
      {cpu, _} = Sim.run(<<0xD3, 0x14, 0x06, 0xD5, 0x01, 0x01, 0x01>>)
      assert cpu.accumulator == 5
    end

    test "invert condition" do
      # LDM 3, JCN 0xC,0x06 (invert+test_zero: A!=0 → jump), LDM 5, HLT, HLT
      {cpu, _} = Sim.run(<<0xD3, 0x1C, 0x06, 0xD5, 0x01, 0x01, 0x01>>)
      assert cpu.accumulator == 3
    end
  end

  describe "ISZ" do
    test "loops until zero" do
      # LDM 14, XCH R0, ISZ R0,0x02, HLT
      {cpu, _} = Sim.run(<<0xDE, 0xB0, 0x70, 0x02, 0x01>>)
      assert Enum.at(cpu.registers, 0) == 0
    end
  end

  # ===================================================================
  # Subroutines
  # ===================================================================

  describe "JMS/BBL" do
    test "call and return" do
      # JMS 0x004, HLT, NOP, NOP, BBL 5
      {cpu, _} = Sim.run(<<0x50, 0x04, 0x01, 0x00, 0xC5>>)
      assert cpu.accumulator == 5
    end

    test "nested calls" do
      {cpu, _} =
        Sim.run(<<
          0x50, 0x06,  # JMS sub1 (at 0x006)
          0xB0, 0x01,  # XCH R0, HLT
          0x00, 0x00,  # padding
          0x50, 0x0C,  # sub1: JMS sub2 (at 0x00C)
          0xB1,        # XCH R1
          0xD9, 0xC0,  # LDM 9, BBL 0
          0x00,        # padding
          0xC3         # sub2: BBL 3
        >>)

      assert Enum.at(cpu.registers, 1) == 3
    end
  end

  # ===================================================================
  # Register pairs
  # ===================================================================

  describe "FIM" do
    test "loads pair" do
      {cpu, _} = Sim.run(<<0x20, 0xAB, 0x01>>)
      assert Enum.at(cpu.registers, 0) == 0xA
      assert Enum.at(cpu.registers, 1) == 0xB
    end
  end

  describe "SRC + WRM + RDM" do
    test "RAM write and read" do
      {cpu, _} =
        Sim.run(<<
          0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
          0xD7, 0xE0,        # LDM 7, WRM
          0xD0,              # LDM 0
          0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
          0xE9,              # RDM
          0x01
        >>)

      assert cpu.accumulator == 7
    end
  end

  describe "JIN" do
    test "jumps indirect" do
      # FIM P1,0x06; JIN P1; LDM 5; HLT; NOP; HLT
      {cpu, _} = Sim.run(<<0x22, 0x06, 0x33, 0xD5, 0x01, 0x00, 0x01>>)
      assert cpu.accumulator == 0
    end
  end

  # ===================================================================
  # RAM I/O
  # ===================================================================

  describe "RAM status" do
    test "write and read status" do
      {cpu, _} =
        Sim.run(<<
          0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
          0xD3, 0xE4,        # LDM 3, WR0
          0xD0,              # LDM 0
          0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
          0xEC,              # RD0
          0x01
        >>)

      assert cpu.accumulator == 3
    end
  end

  describe "WRR/RDR" do
    test "ROM I/O port" do
      {cpu, _} = Sim.run(<<0xDB, 0xE2, 0xD0, 0xEA, 0x01>>)
      assert cpu.accumulator == 11
    end
  end

  describe "RAM banking" do
    test "banks are independent" do
      {cpu, _} =
        Sim.run(<<
          0xD0, 0xFD,        # DCL bank 0
          0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
          0xD5, 0xE0,        # LDM 5, WRM
          0xD1, 0xFD,        # DCL bank 1
          0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
          0xD9, 0xE0,        # LDM 9, WRM
          0xD0, 0xFD,        # DCL bank 0
          0x20, 0x00, 0x21,  # FIM P0,0x00; SRC P0
          0xE9,              # RDM
          0x01
        >>)

      assert cpu.accumulator == 5
    end
  end

  # ===================================================================
  # End-to-end programs
  # ===================================================================

  describe "end-to-end programs" do
    test "x = 1 + 2" do
      {cpu, _} = Sim.run(<<0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01>>)
      assert Enum.at(cpu.registers, 1) == 3
      assert cpu.halted == true
    end

    test "multiply 3x4" do
      {cpu, _} =
        Sim.run(<<
          0xD3, 0xB0, 0xDC, 0xB1,
          0xD0, 0x80, 0x71, 0x05,
          0xB2, 0x01
        >>)

      assert Enum.at(cpu.registers, 2) == 12
    end

    test "BCD 7+8" do
      {cpu, _} = Sim.run(<<0xD8, 0xB0, 0xD7, 0x80, 0xFB, 0x01>>)
      assert cpu.accumulator == 5
      assert cpu.carry == true
    end

    test "countdown" do
      {cpu, _} = Sim.run(<<0xD5, 0xF8, 0x1C, 0x01, 0x01>>)
      assert cpu.accumulator == 0
    end

    test "max steps" do
      {_cpu, traces} = Sim.run(<<0x40, 0x00>>, 10)
      assert length(traces) == 10
    end
  end

  # ===================================================================
  # Trace structure
  # ===================================================================

  describe "trace" do
    test "captures before/after state" do
      {_cpu, traces} = Sim.run(<<0xD5, 0x01>>)
      trace = hd(traces)
      assert trace.address == 0
      assert trace.raw == 0xD5
      assert trace.accumulator_before == 0
      assert trace.accumulator_after == 5
      assert trace.carry_before == false
      assert trace.carry_after == false
    end

    test "2-byte instruction has raw2" do
      {_cpu, traces} = Sim.run(<<0x40, 0x02, 0x01>>)
      trace = hd(traces)
      assert trace.raw2 == 0x02
    end
  end
end
