defmodule CodingAdventures.CompilerIr.PrinterTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CompilerIr.{
    IrProgram,
    IrInstruction,
    IrDataDecl,
    IrRegister,
    IrImmediate,
    IrLabel,
    Printer
  }

  # ── Helper to build programs quickly ────────────────────────────────────────

  defp build_program(instructions, data \\ [], entry \\ "_start") do
    base = %IrProgram{entry_label: entry, version: 1}

    base =
      Enum.reduce(data, base, fn d, p ->
        IrProgram.add_data(p, d)
      end)

    Enum.reduce(instructions, base, fn i, p ->
      IrProgram.add_instruction(p, i)
    end)
  end

  # ── Version directive ────────────────────────────────────────────────────────

  describe "version directive" do
    test "always emits .version N as first line" do
      p = IrProgram.new("_start")
      text = Printer.print(p)
      assert String.starts_with?(text, ".version 1\n")
    end

    test "respects version field" do
      p = %IrProgram{entry_label: "_start", version: 2}
      text = Printer.print(p)
      assert String.starts_with?(text, ".version 2\n")
    end
  end

  # ── Entry directive ──────────────────────────────────────────────────────────

  describe "entry directive" do
    test "emits .entry with entry_label" do
      p = IrProgram.new("_start")
      text = Printer.print(p)
      assert String.contains?(text, ".entry _start")
    end

    test "entry label is used as-is" do
      p = IrProgram.new("my_entry")
      text = Printer.print(p)
      assert String.contains?(text, ".entry my_entry")
    end
  end

  # ── Data declarations ────────────────────────────────────────────────────────

  describe "data declarations" do
    test "emits .data label size init" do
      p = build_program([], [%IrDataDecl{label: "tape", size: 30000, init: 0}])
      text = Printer.print(p)
      assert String.contains?(text, ".data tape 30000 0")
    end

    test "emits multiple data declarations" do
      p =
        build_program([], [
          %IrDataDecl{label: "tape", size: 30000, init: 0},
          %IrDataDecl{label: "buf", size: 256, init: 0}
        ])

      text = Printer.print(p)
      assert String.contains?(text, ".data tape 30000 0")
      assert String.contains?(text, ".data buf 256 0")
    end

    test "no .data lines when no data declarations" do
      p = IrProgram.new("_start")
      text = Printer.print(p)
      refute String.contains?(text, ".data")
    end
  end

  # ── Label instructions ───────────────────────────────────────────────────────

  describe "label instructions" do
    test "emits label on its own unindented line with colon" do
      p =
        build_program([
          %IrInstruction{opcode: :label, operands: [%IrLabel{name: "_start"}], id: -1}
        ])

      text = Printer.print(p)
      assert String.contains?(text, "_start:")
      # Label should NOT be indented
      refute String.contains?(text, "  _start:")
    end

    test "label has no ID comment" do
      p =
        build_program([
          %IrInstruction{opcode: :label, operands: [%IrLabel{name: "_start"}], id: -1}
        ])

      text = Printer.print(p)
      lines = String.split(text, "\n")
      label_line = Enum.find(lines, &String.ends_with?(&1, "_start:"))
      assert label_line != nil
      refute String.contains?(label_line, "; #")
    end
  end

  # ── Comment instructions ──────────────────────────────────────────────────────

  describe "comment instructions" do
    test "emits comment as ; text" do
      p =
        build_program([
          %IrInstruction{
            opcode: :comment,
            operands: [%IrLabel{name: "load tape base"}],
            id: -1
          }
        ])

      text = Printer.print(p)
      assert String.contains?(text, "; load tape base")
    end

    test "comment has no ID" do
      p =
        build_program([
          %IrInstruction{
            opcode: :comment,
            operands: [%IrLabel{name: "setup"}],
            id: -1
          }
        ])

      text = Printer.print(p)
      lines = String.split(text, "\n")
      comment_line = Enum.find(lines, &String.contains?(&1, "; setup"))
      assert comment_line != nil
      refute String.contains?(comment_line, "; #")
    end
  end

  # ── Regular instructions ──────────────────────────────────────────────────────

  describe "regular instructions" do
    test "emits opcode and operands with ID comment" do
      p =
        build_program([
          %IrInstruction{
            opcode: :load_imm,
            operands: [%IrRegister{index: 1}, %IrImmediate{value: 0}],
            id: 5
          }
        ])

      text = Printer.print(p)
      assert String.contains?(text, "LOAD_IMM")
      assert String.contains?(text, "v1")
      assert String.contains?(text, "; #5")
    end

    test "operands are comma-separated" do
      p =
        build_program([
          %IrInstruction{
            opcode: :add_imm,
            operands: [
              %IrRegister{index: 1},
              %IrRegister{index: 1},
              %IrImmediate{value: 1}
            ],
            id: 3
          }
        ])

      text = Printer.print(p)
      assert String.contains?(text, "v1, v1, 1")
    end

    test "instructions are indented with two spaces" do
      p =
        build_program([
          %IrInstruction{opcode: :halt, operands: [], id: 0}
        ])

      text = Printer.print(p)
      lines = String.split(text, "\n")
      halt_line = Enum.find(lines, &String.contains?(&1, "HALT"))
      assert halt_line != nil
      assert String.starts_with?(halt_line, "  ")
    end

    test "HALT with no operands" do
      p = build_program([%IrInstruction{opcode: :halt, operands: [], id: 2}])
      text = Printer.print(p)
      assert String.contains?(text, "HALT")
      assert String.contains?(text, "; #2")
    end

    test "negative immediates" do
      p =
        build_program([
          %IrInstruction{
            opcode: :add_imm,
            operands: [
              %IrRegister{index: 1},
              %IrRegister{index: 1},
              %IrImmediate{value: -1}
            ],
            id: 0
          }
        ])

      text = Printer.print(p)
      assert String.contains?(text, "-1")
    end
  end

  # ── Full program output ───────────────────────────────────────────────────────

  describe "full program" do
    test "produces correct sections in order" do
      program =
        build_program(
          [
            %IrInstruction{opcode: :label, operands: [%IrLabel{name: "_start"}], id: -1},
            %IrInstruction{
              opcode: :load_addr,
              operands: [%IrRegister{index: 0}, %IrLabel{name: "tape"}],
              id: 0
            },
            %IrInstruction{opcode: :halt, operands: [], id: 1}
          ],
          [%IrDataDecl{label: "tape", size: 30000, init: 0}]
        )

      text = Printer.print(program)

      # Find positions of each section
      ver_pos = :binary.match(text, ".version") |> elem(0)
      data_pos = :binary.match(text, ".data") |> elem(0)
      entry_pos = :binary.match(text, ".entry") |> elem(0)

      assert ver_pos < data_pos
      assert data_pos < entry_pos
    end
  end
end
