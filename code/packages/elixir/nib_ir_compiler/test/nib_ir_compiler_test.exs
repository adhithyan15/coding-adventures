defmodule CodingAdventures.NibIrCompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.NibIrCompiler
  alias CodingAdventures.NibParser
  alias CodingAdventures.NibTypeChecker

  defp compile_source(source) do
    {:ok, ast} = NibParser.parse_nib(source)
    typed = NibTypeChecker.check(ast)
    assert typed.ok
    NibIrCompiler.compile_nib(typed.typed_ast).program
  end

  test "emits entrypoint and halt" do
    program = compile_source("fn main() -> u4 { return 7; }")
    opcodes = Enum.map(program.instructions, & &1.opcode)

    assert :label in opcodes
    assert :call in opcodes
    assert :halt in opcodes
  end

  test "emits call and add shapes" do
    program = compile_source("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }")
    opcodes = Enum.map(program.instructions, & &1.opcode)

    assert :add in opcodes or :add_imm in opcodes
    assert :call in opcodes
  end

  test "emits loop control flow" do
    program =
      compile_source("""
      fn count_to(n: u4) -> u4 {
        let acc: u4 = 0;
        for i: u4 in 0..n {
          acc = acc +% 1;
        }
        return acc;
      }
      """)

    opcodes = Enum.map(program.instructions, & &1.opcode)
    assert :branch_z in opcodes
    assert :jump in opcodes
  end
end
