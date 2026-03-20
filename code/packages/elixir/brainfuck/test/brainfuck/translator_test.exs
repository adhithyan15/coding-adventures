defmodule CodingAdventures.Brainfuck.TranslatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Brainfuck.Translator
  alias CodingAdventures.Brainfuck.Translator.TranslationError
  alias CodingAdventures.Brainfuck.Opcodes

  # =========================================================================
  # Empty and minimal programs
  # =========================================================================

  describe "empty and minimal programs" do
    test "empty source produces just HALT" do
      code = Translator.translate("")
      assert length(code.instructions) == 1
      assert hd(code.instructions).opcode == Opcodes.halt()
    end

    test "single increment" do
      code = Translator.translate("+")
      assert length(code.instructions) == 2
      assert Enum.at(code.instructions, 0).opcode == Opcodes.inc()
      assert Enum.at(code.instructions, 1).opcode == Opcodes.halt()
    end

    test "all simple commands" do
      code = Translator.translate("><+-.,")
      assert length(code.instructions) == 7  # 6 commands + HALT

      opcodes = Enum.map(code.instructions, & &1.opcode)
      assert opcodes == [
        Opcodes.right(),
        Opcodes.left(),
        Opcodes.inc(),
        Opcodes.dec(),
        Opcodes.output_op(),
        Opcodes.input_op(),
        Opcodes.halt()
      ]
    end
  end

  # =========================================================================
  # Simple commands (no operands)
  # =========================================================================

  describe "simple commands" do
    test "three increments and output: +++." do
      code = Translator.translate("+++.")
      assert length(code.instructions) == 5  # 3 INCs + OUTPUT + HALT
    end

    test "simple commands have nil operand" do
      code = Translator.translate("+->")
      # All non-loop commands should have nil operand
      non_halt = Enum.slice(code.instructions, 0, 3)
      assert Enum.all?(non_halt, fn instr -> instr.operand == nil end)
    end
  end

  # =========================================================================
  # Bracket matching
  # =========================================================================

  describe "bracket matching" do
    test "simple loop: [+]" do
      code = Translator.translate("[+]")
      # Instructions: LOOP_START(3), INC, LOOP_END(0), HALT
      assert length(code.instructions) == 4

      loop_start = Enum.at(code.instructions, 0)
      assert loop_start.opcode == Opcodes.loop_start()
      assert loop_start.operand == 3  # jump past LOOP_END (index 2) to HALT (index 3)

      loop_end = Enum.at(code.instructions, 2)
      assert loop_end.opcode == Opcodes.loop_end()
      assert loop_end.operand == 0  # jump back to LOOP_START
    end

    test "nested brackets: [[+]]" do
      code = Translator.translate("[[+]]")
      # Instructions: LOOP_START(5), LOOP_START(4), INC, LOOP_END(1), LOOP_END(0), HALT
      assert length(code.instructions) == 6

      outer_start = Enum.at(code.instructions, 0)
      assert outer_start.operand == 5  # jump past outer LOOP_END

      inner_start = Enum.at(code.instructions, 1)
      assert inner_start.operand == 4  # jump past inner LOOP_END

      inner_end = Enum.at(code.instructions, 3)
      assert inner_end.operand == 1  # jump to inner LOOP_START

      outer_end = Enum.at(code.instructions, 4)
      assert outer_end.operand == 0  # jump to outer LOOP_START
    end

    test "sequential loops: [+][+]" do
      code = Translator.translate("[+][+]")
      # First loop: LOOP_START(3), INC, LOOP_END(0)
      # Second loop: LOOP_START(6), INC, LOOP_END(3)
      # HALT
      assert length(code.instructions) == 7

      first_start = Enum.at(code.instructions, 0)
      assert first_start.operand == 3

      second_start = Enum.at(code.instructions, 3)
      assert second_start.operand == 6
    end

    test "loop with body: ++[>+<-]" do
      code = Translator.translate("++[>+<-]")
      # INC, INC, LOOP_START(8), RIGHT, INC, LEFT, DEC, LOOP_END(2), HALT
      assert length(code.instructions) == 9

      loop_start = Enum.at(code.instructions, 2)
      assert loop_start.opcode == Opcodes.loop_start()
      assert loop_start.operand == 8  # jump past LOOP_END to HALT

      loop_end = Enum.at(code.instructions, 7)
      assert loop_end.opcode == Opcodes.loop_end()
      assert loop_end.operand == 2  # jump back to LOOP_START
    end
  end

  # =========================================================================
  # Error cases
  # =========================================================================

  describe "bracket errors" do
    test "unmatched ] raises TranslationError" do
      assert_raise TranslationError, ~r/Unmatched '\]'/, fn ->
        Translator.translate("]")
      end
    end

    test "unmatched [ raises TranslationError" do
      assert_raise TranslationError, ~r/Unmatched '\['/, fn ->
        Translator.translate("[+")
      end
    end

    test "multiple unmatched [ reports count" do
      assert_raise TranslationError, ~r/2 unclosed bracket/, fn ->
        Translator.translate("[[+")
      end
    end

    test "] before [ raises error" do
      assert_raise TranslationError, fn ->
        Translator.translate("]+[")
      end
    end
  end

  # =========================================================================
  # Comments and constants/names
  # =========================================================================

  describe "comments and metadata" do
    test "comments are ignored" do
      code = Translator.translate("Hello World! +")
      # Only the + and HALT should produce instructions
      # (note: "," and "." are BF commands, so we avoid them in comments)
      assert length(code.instructions) == 2
      assert Enum.at(code.instructions, 0).opcode == Opcodes.inc()
    end

    test "all non-command characters are comments" do
      code = Translator.translate("abc123!@#$%^&*()_={}|\\:;\"'?/~`\n\t ")
      # None of these are commands, only HALT
      assert length(code.instructions) == 1
    end

    test "constants pool is empty" do
      code = Translator.translate("+++")
      assert code.constants == []
    end

    test "names pool is empty" do
      code = Translator.translate("+++")
      assert code.names == []
    end
  end
end
