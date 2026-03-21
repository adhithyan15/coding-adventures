# frozen_string_literal: true

# ==========================================================================
# End-to-End Tests — Real Brainfuck Programs Translated and Executed
# ==========================================================================

require_relative "test_helper"

class TestSimplePrograms < Minitest::Test
  def execute(source, input_data: "")
    CodingAdventures::Brainfuck.execute_brainfuck(source, input_data: input_data)
  end

  # Small programs that test fundamental behavior.

  def test_empty_program
    result = execute("")
    assert_equal "", result.output
    assert_equal 0, result.tape[0]
  end

  def test_single_inc
    result = execute("+")
    assert_equal 1, result.tape[0]
  end

  def test_addition
    # 2 + 5 = 7 — classic BF addition pattern.
    # Put 2 in cell 0, 5 in cell 1.
    # Loop: decrement cell 1, increment cell 0.
    # Result: 7 in cell 0, 0 in cell 1.
    result = execute("++>+++++[<+>-]")
    assert_equal 7, result.tape[0]
    assert_equal 0, result.tape[1]
  end

  def test_move_value
    # Move value from cell 0 to cell 1.
    # Set cell 0 to 5, then [>+<-] moves it to cell 1.
    result = execute("+++++[>+<-]")
    assert_equal 0, result.tape[0]
    assert_equal 5, result.tape[1]
  end

  def test_cell_wrapping_overflow
    # 255 + 1 = 0
    source = "+" * 256
    result = execute(source)
    assert_equal 0, result.tape[0]
  end

  def test_cell_wrapping_underflow
    # 0 - 1 = 255
    result = execute("-")
    assert_equal 255, result.tape[0]
  end

  def test_skip_empty_loop
    # [] is skipped when cell is 0 (which it starts as).
    result = execute("[]+++")
    assert_equal 3, result.tape[0]
  end
end

class TestOutput < Minitest::Test
  def execute(source, input_data: "")
    CodingAdventures::Brainfuck.execute_brainfuck(source, input_data: input_data)
  end

  def test_output_h
    # Output 'H' (ASCII 72). 9 * 8 = 72.
    result = execute("+++++++++[>++++++++<-]>.")
    assert_equal "H", result.output
  end

  def test_output_multiple_chars
    # Output 'AB' by setting cell to 65 and incrementing.
    source = "+" * 65 + ".+."
    result = execute(source)
    assert_equal "AB", result.output
  end

  def test_hello_world
    # The classic Brainfuck Hello World program.
    hello_world = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]" \
                  ">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
    result = execute(hello_world)
    assert_equal "Hello World!\n", result.output
  end
end

class TestInput < Minitest::Test
  def execute(source, input_data: "")
    CodingAdventures::Brainfuck.execute_brainfuck(source, input_data: input_data)
  end

  def test_echo_single_char
    result = execute(",.", input_data: "X")
    assert_equal "X", result.output
  end

  def test_cat_program
    # ,[.,] — echo input until EOF.
    result = execute(",[.,]", input_data: "Hi")
    assert_equal "Hi", result.output
  end

  def test_input_to_cell
    result = execute(",", input_data: "A")
    assert_equal 65, result.tape[0]
  end

  def test_eof_is_zero
    result = execute(",")
    assert_equal 0, result.tape[0]
  end
end

class TestNestedLoops < Minitest::Test
  def execute(source, input_data: "")
    CodingAdventures::Brainfuck.execute_brainfuck(source, input_data: input_data)
  end

  def test_nested_multiplication
    # 2 * 3 = 6 using nested loops.
    source = "++>+++<[>[>+>+<<-]>>[<<+>>-]<<<-]"
    result = execute(source)
    assert_equal 6, result.tape[2]
  end

  def test_deeply_nested
    # ++[>++[>+<-]<-] — nested decrement loops.
    result = execute("++[>++[>+<-]<-]")
    assert_equal 4, result.tape[2]
    assert_equal 0, result.tape[1]
    assert_equal 0, result.tape[0]
  end
end

class TestBrainfuckResult < Minitest::Test
  def execute(source, input_data: "")
    CodingAdventures::Brainfuck.execute_brainfuck(source, input_data: input_data)
  end

  def test_result_fields
    result = execute("+++.")
    assert_instance_of String, result.output
    assert_instance_of Array, result.tape
    assert_instance_of Integer, result.dp
    assert_instance_of Array, result.traces
    assert_instance_of Integer, result.steps
  end

  def test_step_count
    result = execute("+++")
    # 3 INCs + 1 HALT = 4 steps
    assert_equal 4, result.steps
  end

  def test_final_dp
    result = execute(">>>")
    assert_equal 3, result.dp
  end

  def test_traces_populated
    result = execute("+")
    assert_equal 2, result.traces.length  # INC + HALT
  end
end

class TestComments < Minitest::Test
  def execute(source, input_data: "")
    CodingAdventures::Brainfuck.execute_brainfuck(source, input_data: input_data)
  end

  def test_comments_in_code
    result = execute("This is + a + program + .")
    assert_equal 3, result.tape[0]
  end

  def test_numbers_ignored
    result = execute("123+456")
    assert_equal 1, result.tape[0]
  end

  def test_newlines_ignored
    result = execute("+\n+\n+")
    assert_equal 3, result.tape[0]
  end
end
