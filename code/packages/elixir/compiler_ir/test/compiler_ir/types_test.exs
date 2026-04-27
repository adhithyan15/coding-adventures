defmodule CodingAdventures.CompilerIr.TypesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CompilerIr.{
    IrRegister,
    IrImmediate,
    IrLabel,
    IrInstruction,
    IrDataDecl,
    IrProgram,
    IDGenerator
  }

  # ── IrRegister ───────────────────────────────────────────────────────────────

  describe "IrRegister" do
    test "to_string produces vN format" do
      assert IrRegister.to_string(%IrRegister{index: 0}) == "v0"
      assert IrRegister.to_string(%IrRegister{index: 1}) == "v1"
      assert IrRegister.to_string(%IrRegister{index: 5}) == "v5"
      assert IrRegister.to_string(%IrRegister{index: 100}) == "v100"
    end

    test "struct has index field" do
      r = %IrRegister{index: 7}
      assert r.index == 7
    end
  end

  # ── IrImmediate ─────────────────────────────────────────────────────────────

  describe "IrImmediate" do
    test "to_string produces decimal string" do
      assert IrImmediate.to_string(%IrImmediate{value: 0}) == "0"
      assert IrImmediate.to_string(%IrImmediate{value: 42}) == "42"
      assert IrImmediate.to_string(%IrImmediate{value: -1}) == "-1"
      assert IrImmediate.to_string(%IrImmediate{value: 255}) == "255"
    end

    test "struct has value field" do
      i = %IrImmediate{value: 99}
      assert i.value == 99
    end

    test "supports negative values" do
      i = %IrImmediate{value: -42}
      assert i.value == -42
      assert IrImmediate.to_string(i) == "-42"
    end
  end

  # ── IrLabel ──────────────────────────────────────────────────────────────────

  describe "IrLabel" do
    test "to_string returns bare name" do
      assert IrLabel.to_string(%IrLabel{name: "_start"}) == "_start"
      assert IrLabel.to_string(%IrLabel{name: "loop_0_start"}) == "loop_0_start"
      assert IrLabel.to_string(%IrLabel{name: "tape"}) == "tape"
      assert IrLabel.to_string(%IrLabel{name: "__trap_oob"}) == "__trap_oob"
    end

    test "struct has name field" do
      l = %IrLabel{name: "my_label"}
      assert l.name == "my_label"
    end
  end

  # ── IrInstruction ────────────────────────────────────────────────────────────

  describe "IrInstruction" do
    test "default fields" do
      i = %IrInstruction{}
      assert i.opcode == :nop
      assert i.operands == []
      assert i.id == -1
    end

    test "can be constructed with explicit fields" do
      i = %IrInstruction{
        opcode: :add_imm,
        operands: [%IrRegister{index: 1}, %IrRegister{index: 1}, %IrImmediate{value: 1}],
        id: 3
      }

      assert i.opcode == :add_imm
      assert length(i.operands) == 3
      assert i.id == 3
    end

    test "label instructions use id -1" do
      i = %IrInstruction{
        opcode: :label,
        operands: [%IrLabel{name: "_start"}],
        id: -1
      }

      assert i.id == -1
    end
  end

  # ── IrDataDecl ───────────────────────────────────────────────────────────────

  describe "IrDataDecl" do
    test "default init is 0" do
      d = %IrDataDecl{label: "tape", size: 30000}
      assert d.init == 0
    end

    test "struct has label, size, and init fields" do
      d = %IrDataDecl{label: "tape", size: 30000, init: 0}
      assert d.label == "tape"
      assert d.size == 30000
      assert d.init == 0
    end
  end

  # ── IrProgram ────────────────────────────────────────────────────────────────

  describe "IrProgram.new/1" do
    test "creates program with given entry label" do
      p = IrProgram.new("_start")
      assert p.entry_label == "_start"
    end

    test "version is 1" do
      p = IrProgram.new("_start")
      assert p.version == 1
    end

    test "starts with empty instructions and data" do
      p = IrProgram.new("_start")
      assert p.instructions == []
      assert p.data == []
    end
  end

  describe "IrProgram.add_instruction/2" do
    test "appends instruction" do
      p = IrProgram.new("_start")
      instr = %IrInstruction{opcode: :halt, id: 0}
      p2 = IrProgram.add_instruction(p, instr)
      assert length(p2.instructions) == 1
      assert hd(p2.instructions).opcode == :halt
    end

    test "appends multiple instructions in order" do
      p = IrProgram.new("_start")

      p =
        p
        |> IrProgram.add_instruction(%IrInstruction{opcode: :load_imm, id: 0})
        |> IrProgram.add_instruction(%IrInstruction{opcode: :halt, id: 1})

      assert length(p.instructions) == 2
      assert Enum.at(p.instructions, 0).opcode == :load_imm
      assert Enum.at(p.instructions, 1).opcode == :halt
    end

    test "does not mutate original program" do
      p = IrProgram.new("_start")
      _p2 = IrProgram.add_instruction(p, %IrInstruction{opcode: :halt, id: 0})
      assert p.instructions == []
    end
  end

  describe "IrProgram.add_data/2" do
    test "appends data declaration" do
      p = IrProgram.new("_start")
      d = %IrDataDecl{label: "tape", size: 30000, init: 0}
      p2 = IrProgram.add_data(p, d)
      assert length(p2.data) == 1
      assert hd(p2.data).label == "tape"
    end

    test "appends multiple data declarations in order" do
      p = IrProgram.new("_start")

      p =
        p
        |> IrProgram.add_data(%IrDataDecl{label: "tape", size: 30000, init: 0})
        |> IrProgram.add_data(%IrDataDecl{label: "buf", size: 256, init: 0})

      assert length(p.data) == 2
      assert Enum.at(p.data, 0).label == "tape"
      assert Enum.at(p.data, 1).label == "buf"
    end
  end

  # ── IDGenerator ─────────────────────────────────────────────────────────────

  describe "IDGenerator" do
    test "new/0 starts at 0" do
      gen = IDGenerator.new()
      assert IDGenerator.current(gen) == 0
    end

    test "next/1 returns incrementing IDs" do
      gen = IDGenerator.new()
      {id0, gen} = IDGenerator.next(gen)
      {id1, gen} = IDGenerator.next(gen)
      {id2, _gen} = IDGenerator.next(gen)

      assert id0 == 0
      assert id1 == 1
      assert id2 == 2
    end

    test "current/1 returns next ID without incrementing" do
      gen = IDGenerator.new()
      assert IDGenerator.current(gen) == 0
      {_id, gen2} = IDGenerator.next(gen)
      assert IDGenerator.current(gen2) == 1
    end

    test "new_from/1 starts at given value" do
      gen = IDGenerator.new_from(100)
      {id, _} = IDGenerator.next(gen)
      assert id == 100
    end

    test "does not mutate original generator" do
      gen = IDGenerator.new()
      {_id, _gen2} = IDGenerator.next(gen)
      assert IDGenerator.current(gen) == 0
    end

    test "generates unique IDs across many calls" do
      gen = IDGenerator.new()

      {ids, _gen} =
        Enum.reduce(0..99, {[], gen}, fn _, {acc, g} ->
          {id, g2} = IDGenerator.next(g)
          {[id | acc], g2}
        end)

      assert length(Enum.uniq(ids)) == 100
    end
  end
end
