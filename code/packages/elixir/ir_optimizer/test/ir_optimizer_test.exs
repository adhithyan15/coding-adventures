defmodule CodingAdventures.IrOptimizerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CompilerIr.{IrImmediate, IrInstruction, IrLabel, IrProgram, IrRegister}

  alias CodingAdventures.IrOptimizer

  alias CodingAdventures.IrOptimizer.{
    ConstantFolder,
    DeadCodeEliminator,
    OptimizationResult,
    PeepholeOptimizer
  }

  defmodule IdentityPass do
    @behaviour CodingAdventures.IrOptimizer.Pass

    def name, do: "IdentityPass"
    def run(program), do: program
  end

  defp make_program(instructions, entry \\ "_start") do
    Enum.reduce(instructions, IrProgram.new(entry), &IrProgram.add_instruction(&2, &1))
  end

  defp load_imm(register, value, id) do
    %IrInstruction{
      opcode: :load_imm,
      operands: [%IrRegister{index: register}, %IrImmediate{value: value}],
      id: id
    }
  end

  defp add_imm(dest, src, value, id) do
    %IrInstruction{
      opcode: :add_imm,
      operands: [%IrRegister{index: dest}, %IrRegister{index: src}, %IrImmediate{value: value}],
      id: id
    }
  end

  defp and_imm(dest, src, value, id) do
    %IrInstruction{
      opcode: :and_imm,
      operands: [%IrRegister{index: dest}, %IrRegister{index: src}, %IrImmediate{value: value}],
      id: id
    }
  end

  defp jump(target, id) do
    %IrInstruction{opcode: :jump, operands: [%IrLabel{name: target}], id: id}
  end

  defp label(name), do: %IrInstruction{opcode: :label, operands: [%IrLabel{name: name}], id: -1}
  defp halt(id \\ 0), do: %IrInstruction{opcode: :halt, operands: [], id: id}

  test "OptimizationResult stores eliminated instruction counts" do
    result = %OptimizationResult{
      program: make_program([halt()]),
      passes_run: ["TestPass"],
      instructions_before: 10,
      instructions_after: 7,
      instructions_eliminated: 3
    }

    assert result.passes_run == ["TestPass"]
    assert result.instructions_eliminated == 3
  end

  test "no_op keeps instructions and reports no passes" do
    program = make_program([halt(0), add_imm(1, 1, 1, 1)])

    result = IrOptimizer.optimize(IrOptimizer.no_op(), program)

    assert length(result.program.instructions) == 2
    assert result.passes_run == []
    assert result.instructions_before == 2
    assert result.instructions_after == 2
    assert result.instructions_eliminated == 0
  end

  test "dead code eliminator removes instructions after unconditional branches" do
    program =
      make_program([
        jump("done", 1),
        add_imm(1, 1, 1, 2),
        label("done"),
        halt(3)
      ])

    result = DeadCodeEliminator.run(program)

    assert Enum.map(result.instructions, & &1.id) == [1, -1, 3]
  end

  test "constant folder folds load_imm plus add_imm and and_imm" do
    program =
      make_program([
        load_imm(1, 5, 1),
        add_imm(1, 1, 3, 2),
        and_imm(1, 1, 6, 3),
        halt(4)
      ])

    result = ConstantFolder.run(program)

    assert Enum.map(result.instructions, & &1.opcode) == [:load_imm, :halt]

    assert Enum.at(result.instructions, 0).operands == [
             %IrRegister{index: 1},
             %IrImmediate{value: 0}
           ]
  end

  test "constant folder clears stale pending loads after writes" do
    program =
      make_program([
        load_imm(1, 5, 1),
        %IrInstruction{opcode: :add, operands: [%IrRegister{index: 1}], id: 2},
        add_imm(1, 1, 3, 3)
      ])

    result = ConstantFolder.run(program)

    assert Enum.map(result.instructions, & &1.opcode) == [:load_imm, :add, :add_imm]
  end

  test "peephole optimizer merges add_imm chains" do
    program =
      make_program([
        add_imm(2, 2, 2, 1),
        add_imm(2, 2, 3, 2),
        add_imm(2, 2, 4, 3)
      ])

    result = PeepholeOptimizer.run(program)

    assert length(result.instructions) == 1

    assert hd(result.instructions).operands == [
             %IrRegister{index: 2},
             %IrRegister{index: 2},
             %IrImmediate{value: 9}
           ]
  end

  test "peephole optimizer removes byte-mask no-ops and zero-load adds" do
    program =
      make_program([
        load_imm(1, 7, 1),
        and_imm(1, 1, 255, 2),
        load_imm(2, 0, 3),
        add_imm(2, 2, 5, 4)
      ])

    result = PeepholeOptimizer.run(program)

    assert Enum.map(result.instructions, &(&1.operands |> List.last())) == [
             %IrImmediate{value: 7},
             %IrImmediate{value: 5}
           ]
  end

  test "default pipeline runs passes in order and returns counts" do
    program =
      make_program([
        load_imm(1, 5, 1),
        add_imm(1, 1, 3, 2),
        jump("done", 3),
        add_imm(1, 1, 1, 4),
        label("done"),
        halt(5)
      ])

    result = IrOptimizer.optimize(program)

    assert result.passes_run == ["DeadCodeEliminator", "ConstantFolder", "PeepholeOptimizer"]
    assert result.instructions_before == 6
    assert result.instructions_after == 4
    assert result.instructions_eliminated == 2

    assert Enum.at(result.program.instructions, 0).operands == [
             %IrRegister{index: 1},
             %IrImmediate{value: 8}
           ]
  end

  test "custom pass lists are accepted" do
    program = make_program([halt()])

    result = IrOptimizer.optimize(program, [IdentityPass])

    assert result.passes_run == ["IdentityPass"]
    assert result.instructions_eliminated == 0
  end
end
