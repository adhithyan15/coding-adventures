defmodule CodingAdventures.CompilerIr.ParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CompilerIr.{
    IrProgram,
    IrInstruction,
    IrDataDecl,
    IrRegister,
    IrImmediate,
    IrLabel,
    Printer,
    Parser
  }

  # ── Version directive ────────────────────────────────────────────────────────

  describe "version directive" do
    test "parses .version 1" do
      {:ok, p} = Parser.parse(".version 1\n.entry _start\n")
      assert p.version == 1
    end

    test "parses .version 2" do
      {:ok, p} = Parser.parse(".version 2\n.entry _start\n")
      assert p.version == 2
    end

    test "invalid version returns error" do
      assert {:error, msg} = Parser.parse(".version bad\n.entry _start\n")
      assert String.contains?(msg, "version")
    end

    test "extra fields in version returns error" do
      assert {:error, _} = Parser.parse(".version 1 extra\n.entry _start\n")
    end
  end

  # ── Data directive ───────────────────────────────────────────────────────────

  describe "data directive" do
    test "parses .data label size init" do
      {:ok, p} = Parser.parse(".version 1\n.data tape 30000 0\n.entry _start\n")
      assert length(p.data) == 1
      [d] = p.data
      assert d.label == "tape"
      assert d.size == 30000
      assert d.init == 0
    end

    test "parses multiple data declarations" do
      text = ".version 1\n.data tape 30000 0\n.data buf 256 255\n.entry _start\n"
      {:ok, p} = Parser.parse(text)
      assert length(p.data) == 2
    end

    test "invalid data size returns error" do
      assert {:error, msg} =
               Parser.parse(".version 1\n.data tape bad 0\n.entry _start\n")

      assert String.contains?(msg, "data")
    end
  end

  # ── Entry directive ──────────────────────────────────────────────────────────

  describe "entry directive" do
    test "parses .entry label" do
      {:ok, p} = Parser.parse(".version 1\n.entry _start\n")
      assert p.entry_label == "_start"
    end

    test "parses .entry with any label" do
      {:ok, p} = Parser.parse(".version 1\n.entry my_func\n")
      assert p.entry_label == "my_func"
    end
  end

  # ── Label instructions ───────────────────────────────────────────────────────

  describe "label lines" do
    test "parses label line" do
      {:ok, p} = Parser.parse(".version 1\n.entry _start\n\n_start:\n")
      labels =
        Enum.filter(p.instructions, fn i -> i.opcode == :label end)

      assert length(labels) == 1
      [l] = labels
      assert l.operands == [%IrLabel{name: "_start"}]
      assert l.id == -1
    end

    test "parses multiple labels" do
      text = """
      .version 1
      .entry _start

      _start:
      loop_0_start:
      """

      {:ok, p} = Parser.parse(text)
      label_names =
        p.instructions
        |> Enum.filter(&(&1.opcode == :label))
        |> Enum.map(fn i -> hd(i.operands).name end)

      assert "_start" in label_names
      assert "loop_0_start" in label_names
    end
  end

  # ── Comment lines ────────────────────────────────────────────────────────────

  describe "comment lines" do
    test "parses standalone comment as COMMENT instruction" do
      {:ok, p} = Parser.parse(".version 1\n.entry _start\n\n  ; hello world\n")
      comments = Enum.filter(p.instructions, fn i -> i.opcode == :comment end)
      assert length(comments) == 1
    end

    test "ID comment lines like '; #3' are NOT added as COMMENT instructions" do
      # ID comments appear after regular instructions; standalone "; #N" lines
      # should not generate COMMENT instructions
      text = ".version 1\n.entry _start\n  ; #3\n"
      {:ok, p} = Parser.parse(text)
      # No COMMENT instruction should be produced
      comments = Enum.filter(p.instructions, fn i -> i.opcode == :comment end)
      assert comments == []
    end
  end

  # ── Regular instructions ──────────────────────────────────────────────────────

  describe "instruction parsing" do
    test "parses HALT with no operands" do
      {:ok, p} = Parser.parse(".version 1\n.entry _start\n  HALT  ; #0\n")
      instrs = Enum.filter(p.instructions, fn i -> i.opcode == :halt end)
      assert length(instrs) == 1
      [h] = instrs
      assert h.id == 0
    end

    test "parses LOAD_IMM with register and immediate" do
      {:ok, p} =
        Parser.parse(".version 1\n.entry _start\n  LOAD_IMM v1, 0  ; #1\n")

      [instr] = Enum.filter(p.instructions, fn i -> i.opcode == :load_imm end)
      assert instr.operands == [%IrRegister{index: 1}, %IrImmediate{value: 0}]
      assert instr.id == 1
    end

    test "parses ADD_IMM with three operands" do
      {:ok, p} =
        Parser.parse(".version 1\n.entry _start\n  ADD_IMM v1, v1, 1  ; #3\n")

      [instr] = Enum.filter(p.instructions, fn i -> i.opcode == :add_imm end)

      assert instr.operands == [
               %IrRegister{index: 1},
               %IrRegister{index: 1},
               %IrImmediate{value: 1}
             ]
    end

    test "parses BRANCH_Z with register and label" do
      {:ok, p} =
        Parser.parse(
          ".version 1\n.entry _start\n  BRANCH_Z v2, loop_0_end  ; #7\n"
        )

      [instr] = Enum.filter(p.instructions, fn i -> i.opcode == :branch_z end)
      assert instr.operands == [%IrRegister{index: 2}, %IrLabel{name: "loop_0_end"}]
    end

    test "parses negative immediate" do
      {:ok, p} =
        Parser.parse(".version 1\n.entry _start\n  ADD_IMM v1, v1, -1  ; #5\n")

      [instr] = Enum.filter(p.instructions, fn i -> i.opcode == :add_imm end)
      assert List.last(instr.operands) == %IrImmediate{value: -1}
    end

    test "parses LOAD_ADDR with register and label" do
      {:ok, p} =
        Parser.parse(".version 1\n.entry _start\n  LOAD_ADDR v0, tape  ; #0\n")

      [instr] = Enum.filter(p.instructions, fn i -> i.opcode == :load_addr end)
      assert instr.operands == [%IrRegister{index: 0}, %IrLabel{name: "tape"}]
    end

    test "unknown opcode returns error" do
      assert {:error, msg} =
               Parser.parse(".version 1\n.entry _start\n  BOGUS_OP  ; #0\n")

      assert String.contains?(msg, "opcode")
    end
  end

  # ── Operand parsing ───────────────────────────────────────────────────────────

  describe "operand type inference" do
    test "vN is a register" do
      {:ok, p} =
        Parser.parse(
          ".version 1\n.entry _start\n  LOAD_IMM v0, 0  ; #0\n"
        )

      [i] = p.instructions
      assert hd(i.operands) == %IrRegister{index: 0}
    end

    test "integer is an immediate" do
      {:ok, p} =
        Parser.parse(
          ".version 1\n.entry _start\n  LOAD_IMM v0, 42  ; #0\n"
        )

      [i] = p.instructions
      assert List.last(i.operands) == %IrImmediate{value: 42}
    end

    test "non-register non-integer is a label" do
      {:ok, p} =
        Parser.parse(".version 1\n.entry _start\n  JUMP loop_start  ; #0\n")

      [i] = p.instructions
      assert hd(i.operands) == %IrLabel{name: "loop_start"}
    end
  end

  # ── Roundtrip ─────────────────────────────────────────────────────────────────

  describe "roundtrip print→parse" do
    test "empty program roundtrips" do
      p = IrProgram.new("_start")
      {:ok, p2} = p |> Printer.print() |> Parser.parse()
      assert p2.entry_label == p.entry_label
      assert p2.version == p.version
      assert p2.instructions == []
    end

    test "program with data and instructions roundtrips" do
      p =
        IrProgram.new("_start")
        |> IrProgram.add_data(%IrDataDecl{label: "tape", size: 30000, init: 0})
        |> IrProgram.add_instruction(%IrInstruction{
          opcode: :label,
          operands: [%IrLabel{name: "_start"}],
          id: -1
        })
        |> IrProgram.add_instruction(%IrInstruction{
          opcode: :load_addr,
          operands: [%IrRegister{index: 0}, %IrLabel{name: "tape"}],
          id: 0
        })
        |> IrProgram.add_instruction(%IrInstruction{
          opcode: :halt,
          operands: [],
          id: 1
        })

      text = Printer.print(p)
      {:ok, p2} = Parser.parse(text)

      assert length(p2.instructions) == length(p.instructions)
      assert length(p2.data) == length(p.data)
    end

    test "instruction count preserved across roundtrip" do
      p =
        IrProgram.new("_start")
        |> IrProgram.add_instruction(%IrInstruction{
          opcode: :load_imm,
          operands: [%IrRegister{index: 1}, %IrImmediate{value: 0}],
          id: 0
        })
        |> IrProgram.add_instruction(%IrInstruction{
          opcode: :add_imm,
          operands: [
            %IrRegister{index: 1},
            %IrRegister{index: 1},
            %IrImmediate{value: 1}
          ],
          id: 1
        })
        |> IrProgram.add_instruction(%IrInstruction{opcode: :halt, operands: [], id: 2})

      text = Printer.print(p)
      {:ok, p2} = Parser.parse(text)
      assert length(p2.instructions) == 3
    end
  end

  # ── Edge cases ────────────────────────────────────────────────────────────────

  describe "edge cases" do
    test "blank lines are skipped" do
      text = ".version 1\n\n\n.entry _start\n\n  HALT  ; #0\n\n"
      {:ok, p} = Parser.parse(text)
      assert length(Enum.filter(p.instructions, fn i -> i.opcode == :halt end)) == 1
    end

    test "empty input returns default program" do
      {:ok, p} = Parser.parse("")
      assert p.version == 1
    end

    test "missing ID is treated as -1" do
      {:ok, p} = Parser.parse(".version 1\n.entry _start\n  HALT\n")
      [halt] = Enum.filter(p.instructions, fn i -> i.opcode == :halt end)
      assert halt.id == -1
    end
  end
end
