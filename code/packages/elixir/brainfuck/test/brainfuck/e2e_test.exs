defmodule CodingAdventures.Brainfuck.E2ETest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Brainfuck.VM
  alias CodingAdventures.Brainfuck.VM.BrainfuckResult

  # =========================================================================
  # Empty and minimal programs
  # =========================================================================

  describe "empty and minimal programs" do
    test "empty program" do
      result = VM.execute_brainfuck("")
      assert result.output == ""
      assert result.dp == 0
      assert result.steps > 0  # at least HALT executes
    end

    test "single increment" do
      result = VM.execute_brainfuck("+")
      assert Enum.at(result.tape, 0) == 1
      assert result.dp == 0
    end

    test "multiple increments" do
      result = VM.execute_brainfuck("+++")
      assert Enum.at(result.tape, 0) == 3
    end
  end

  # =========================================================================
  # Arithmetic
  # =========================================================================

  describe "arithmetic" do
    test "addition: 2 + 5 = 7 via ++>+++++[<+>-]" do
      result = VM.execute_brainfuck("++>+++++[<+>-]")
      # Cell 0 should be 7 (2+5), cell 1 should be 0 (decremented to 0)
      assert Enum.at(result.tape, 0) == 7
      assert Enum.at(result.tape, 1) == 0
    end

    test "subtraction via decrement" do
      result = VM.execute_brainfuck("+++++---")
      assert Enum.at(result.tape, 0) == 2
    end

    test "multiplication: 3 * 4 = 12 via +++[>++++<-]" do
      result = VM.execute_brainfuck("+++[>++++<-]")
      assert Enum.at(result.tape, 1) == 12
    end
  end

  # =========================================================================
  # Cell wrapping
  # =========================================================================

  describe "cell wrapping" do
    test "overflow: 255 + 1 = 0" do
      # Set cell to 255 with a loop: generate 255 via 17*15
      # Simpler: just use 255 increments (slow but correct)
      source = String.duplicate("+", 256)
      result = VM.execute_brainfuck(source)
      assert Enum.at(result.tape, 0) == 0
    end

    test "underflow: 0 - 1 = 255" do
      result = VM.execute_brainfuck("-")
      assert Enum.at(result.tape, 0) == 255
    end

    test "double wrap: 255 + 2 = 1" do
      source = String.duplicate("+", 257)
      result = VM.execute_brainfuck(source)
      assert Enum.at(result.tape, 0) == 1
    end
  end

  # =========================================================================
  # Loops
  # =========================================================================

  describe "loops" do
    test "loop skip when cell is 0" do
      # Cell starts at 0, so [+] should be skipped entirely
      result = VM.execute_brainfuck("[+]")
      assert Enum.at(result.tape, 0) == 0
    end

    test "simple loop: count down from 3" do
      result = VM.execute_brainfuck("+++[-]")
      assert Enum.at(result.tape, 0) == 0
    end

    test "nested loops" do
      # 2 * (3 * 4) = 24 via nested multiplication
      result = VM.execute_brainfuck("++[>+++[>++++<-]<-]")
      # Cell 2 should have 24 (2 * 3 * 4), but actually it accumulates:
      # Outer loop runs 2 times, inner loop adds 4 to cell 2 three times = 12 per outer iteration
      # But cell 1 is reset to 0 each time inner loop finishes, so outer loop
      # actually sees cell 1 at 0 after first iteration... let me rethink.
      # Actually: outer decrements cell 0, inner adds 4 to cell 2 three times.
      # Outer runs 2 times, so cell 2 gets 2 * 12 = 24
      assert Enum.at(result.tape, 2) == 24
    end
  end

  # =========================================================================
  # Output
  # =========================================================================

  describe "output" do
    test "simple output: ASCII 65 = 'A'" do
      # 65 = 5*13: +++++[>+++++++++++++<-]>.
      result = VM.execute_brainfuck("+++++[>+++++++++++++<-]>.")
      assert result.output == "A"
    end

    test "multiple outputs" do
      # Output two characters: 65 (A) and 66 (B)
      result = VM.execute_brainfuck("+++++[>+++++++++++++<-]>.+.")
      assert result.output == "AB"
    end

    test "Hello World" do
      hello_world = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
      result = VM.execute_brainfuck(hello_world)
      assert result.output == "Hello World!\n"
    end
  end

  # =========================================================================
  # Input
  # =========================================================================

  describe "input" do
    test "input echo: ,." do
      result = VM.execute_brainfuck(",.", "A")
      assert result.output == "A"
      assert Enum.at(result.tape, 0) == ?A
    end

    test "cat program: ,[.,] echoes all input" do
      result = VM.execute_brainfuck(",[.,]", "Hi")
      assert result.output == "Hi"
    end

    test "input with no data gives 0 (EOF)" do
      result = VM.execute_brainfuck(",")
      assert Enum.at(result.tape, 0) == 0
    end

    test "input reads successive bytes" do
      result = VM.execute_brainfuck(",>,", "AB")
      assert Enum.at(result.tape, 0) == ?A
      assert Enum.at(result.tape, 1) == ?B
    end
  end

  # =========================================================================
  # Comments
  # =========================================================================

  describe "comments" do
    test "comments are ignored" do
      result = VM.execute_brainfuck("This is a comment + and only the plus counts")
      assert Enum.at(result.tape, 0) == 1
    end
  end

  # =========================================================================
  # Result struct
  # =========================================================================

  describe "result struct" do
    test "returns a BrainfuckResult" do
      result = VM.execute_brainfuck("+")
      assert %BrainfuckResult{} = result
    end

    test "output field is a string" do
      result = VM.execute_brainfuck("")
      assert is_binary(result.output)
    end

    test "tape field is a list" do
      result = VM.execute_brainfuck("")
      assert is_list(result.tape)
      assert length(result.tape) == 30_000
    end

    test "dp field tracks data pointer" do
      result = VM.execute_brainfuck(">>>")
      assert result.dp == 3
    end

    test "traces field is a list" do
      result = VM.execute_brainfuck("+")
      assert is_list(result.traces)
    end

    test "steps field counts instructions" do
      result = VM.execute_brainfuck("+++")
      # 3 INCs + 1 HALT = 4 steps
      assert result.steps == 4
    end
  end

  # =========================================================================
  # Data pointer tracking
  # =========================================================================

  describe "data pointer" do
    test "moves right correctly" do
      result = VM.execute_brainfuck(">>>")
      assert result.dp == 3
    end

    test "moves left correctly" do
      result = VM.execute_brainfuck(">>><<")
      assert result.dp == 1
    end

    test "writes to correct cell after movement" do
      result = VM.execute_brainfuck(">+++>++")
      assert Enum.at(result.tape, 0) == 0
      assert Enum.at(result.tape, 1) == 3
      assert Enum.at(result.tape, 2) == 2
    end
  end

  # =========================================================================
  # Convenience API
  # =========================================================================

  describe "convenience API" do
    test "create_brainfuck_vm with default input" do
      vm = VM.create_brainfuck_vm()
      assert vm.halted == false
      assert vm.pc == 0
    end

    test "create_brainfuck_vm with custom input" do
      vm = VM.create_brainfuck_vm("test input")
      input = CodingAdventures.VirtualMachine.GenericVM.get_extra(vm, :input_buffer)
      assert input == "test input"
    end
  end

  # =========================================================================
  # Edge cases
  # =========================================================================

  describe "edge cases" do
    test "program with only comments" do
      result = VM.execute_brainfuck("hello world")
      assert result.output == ""
      assert result.steps == 1  # just HALT
    end

    test "empty loop at start: []" do
      result = VM.execute_brainfuck("[]")
      # Cell starts at 0, loop is skipped
      assert Enum.at(result.tape, 0) == 0
    end

    test "move pointer to cell 1 and back" do
      result = VM.execute_brainfuck("+>++<")
      assert result.dp == 0
      assert Enum.at(result.tape, 0) == 1
      assert Enum.at(result.tape, 1) == 2
    end
  end
end
