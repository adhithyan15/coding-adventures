defmodule CodingAdventures.StarlarkInterpreter.InterpreterTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkInterpreter.Interpreter

  # ===========================================================================
  # Chapter 1: Basic Execution — Arithmetic & Variables
  # ===========================================================================
  # These tests verify the full pipeline: source → tokens → AST → bytecode → VM.

  describe "basic arithmetic" do
    test "integer addition" do
      result = Interpreter.interpret("x = 1 + 2\n")
      assert result.variables["x"] == 3
    end

    test "integer subtraction" do
      result = Interpreter.interpret("x = 10 - 3\n")
      assert result.variables["x"] == 7
    end

    test "integer multiplication" do
      result = Interpreter.interpret("x = 4 * 5\n")
      assert result.variables["x"] == 20
    end

    test "integer division (always float)" do
      result = Interpreter.interpret("x = 10 / 3\n")
      assert_in_delta result.variables["x"], 3.333, 0.01
    end

    test "floor division" do
      result = Interpreter.interpret("x = 10 // 3\n")
      assert result.variables["x"] == 3
    end

    test "modulo" do
      result = Interpreter.interpret("x = 10 % 3\n")
      assert result.variables["x"] == 1
    end

    test "exponentiation" do
      result = Interpreter.interpret("x = 2 ** 10\n")
      assert result.variables["x"] == 1024
    end

    test "negation" do
      result = Interpreter.interpret("x = -42\n")
      assert result.variables["x"] == -42
    end

    test "complex expression" do
      result = Interpreter.interpret("x = (2 + 3) * 4 - 1\n")
      assert result.variables["x"] == 19
    end

    test "multiple assignments" do
      result = Interpreter.interpret("x = 10\ny = 20\nz = x + y\n")
      assert result.variables["x"] == 10
      assert result.variables["y"] == 20
      assert result.variables["z"] == 30
    end
  end

  # ===========================================================================
  # Chapter 2: Data Types
  # ===========================================================================

  describe "data types" do
    test "strings" do
      result = Interpreter.interpret("x = \"hello\" + \" \" + \"world\"\n")
      assert result.variables["x"] == "hello world"
    end

    test "string repetition" do
      result = Interpreter.interpret("x = \"ab\" * 3\n")
      assert result.variables["x"] == "ababab"
    end

    test "booleans" do
      result = Interpreter.interpret("x = True\ny = False\n")
      assert result.variables["x"] == true
      assert result.variables["y"] == false
    end

    test "None" do
      result = Interpreter.interpret("x = None\n")
      assert result.variables["x"] == nil
    end

    test "float literals" do
      result = Interpreter.interpret("x = 3.14\n")
      assert_in_delta result.variables["x"], 3.14, 0.001
    end

    test "lists" do
      result = Interpreter.interpret("x = [1, 2, 3]\n")
      assert result.variables["x"] == [1, 2, 3]
    end

    test "list concatenation" do
      result = Interpreter.interpret("x = [1, 2] + [3, 4]\n")
      assert result.variables["x"] == [1, 2, 3, 4]
    end

    test "empty dict" do
      result = Interpreter.interpret("x = {}\n")
      assert result.variables["x"] == %{}
    end

    test "dict with entries" do
      result = Interpreter.interpret("x = {\"a\": 1, \"b\": 2}\n")
      assert result.variables["x"] == %{"a" => 1, "b" => 2}
    end

    test "tuple" do
      result = Interpreter.interpret("x = (1, 2, 3)\n")
      assert result.variables["x"] == {1, 2, 3}
    end
  end

  # ===========================================================================
  # Chapter 3: Comparisons & Boolean Logic
  # ===========================================================================

  describe "comparisons" do
    test "equality" do
      result = Interpreter.interpret("x = 1 == 1\ny = 1 == 2\n")
      assert result.variables["x"] == true
      assert result.variables["y"] == false
    end

    test "inequality" do
      result = Interpreter.interpret("x = 1 != 2\n")
      assert result.variables["x"] == true
    end

    test "less than" do
      result = Interpreter.interpret("x = 1 < 2\ny = 2 < 1\n")
      assert result.variables["x"] == true
      assert result.variables["y"] == false
    end

    test "greater than" do
      result = Interpreter.interpret("x = 5 > 3\n")
      assert result.variables["x"] == true
    end

    test "boolean not" do
      result = Interpreter.interpret("x = not True\ny = not False\n")
      assert result.variables["x"] == false
      assert result.variables["y"] == true
    end
  end

  # ===========================================================================
  # Chapter 4: Control Flow
  # ===========================================================================

  describe "if statements" do
    test "if true branch" do
      source = """
      x = 0
      if True:
          x = 1
      """
      result = Interpreter.interpret(source)
      assert result.variables["x"] == 1
    end

    test "if false branch with else" do
      source = """
      if False:
          x = 1
      else:
          x = 2
      """
      result = Interpreter.interpret(source)
      assert result.variables["x"] == 2
    end

    test "if-elif-else" do
      source = """
      val = 15
      if val > 20:
          x = "big"
      elif val > 10:
          x = "medium"
      else:
          x = "small"
      """
      result = Interpreter.interpret(source)
      assert result.variables["x"] == "medium"
    end
  end

  describe "for loops" do
    test "simple for loop" do
      source = """
      total = 0
      for i in [1, 2, 3, 4, 5]:
          total = total + i
      """
      result = Interpreter.interpret(source)
      assert result.variables["total"] == 15
    end

    test "for loop with range" do
      source = """
      total = 0
      for i in range(5):
          total = total + i
      """
      result = Interpreter.interpret(source)
      assert result.variables["total"] == 10
    end

    test "nested for loops" do
      source = """
      total = 0
      for i in [1, 2, 3]:
          for j in [10, 20]:
              total = total + i * j
      """
      result = Interpreter.interpret(source)
      # 1*10 + 1*20 + 2*10 + 2*20 + 3*10 + 3*20 = 10+20+20+40+30+60 = 180
      assert result.variables["total"] == 180
    end
  end

  # ===========================================================================
  # Chapter 5: Functions
  # ===========================================================================

  describe "functions" do
    test "simple function definition and call" do
      source = """
      def add(a, b):
          return a + b
      result = add(3, 4)
      """
      result = Interpreter.interpret(source)
      assert result.variables["result"] == 7
    end

    test "function with default arguments" do
      source = """
      def greet(name, greeting = "Hello"):
          return greeting + " " + name
      x = greet("World")
      y = greet("World", "Hi")
      """
      result = Interpreter.interpret(source)
      assert result.variables["x"] == "Hello World"
      assert result.variables["y"] == "Hi World"
    end

    test "recursive function" do
      source = """
      def factorial(n):
          if n <= 1:
              return 1
          return n * factorial(n - 1)
      result = factorial(5)
      """
      result = Interpreter.interpret(source)
      assert result.variables["result"] == 120
    end

    test "function returning a list" do
      source = """
      def make_list(n):
          result = []
          for i in range(n):
              result = result + [i]
          return result
      x = make_list(4)
      """
      result = Interpreter.interpret(source)
      assert result.variables["x"] == [0, 1, 2, 3]
    end
  end

  # ===========================================================================
  # Chapter 6: Built-in Functions
  # ===========================================================================

  describe "built-in functions" do
    test "len()" do
      result = Interpreter.interpret("x = len([1, 2, 3])\n")
      assert result.variables["x"] == 3
    end

    test "range()" do
      result = Interpreter.interpret("x = range(5)\n")
      assert result.variables["x"] == [0, 1, 2, 3, 4]
    end

    test "type()" do
      result = Interpreter.interpret("x = type(42)\ny = type(\"hello\")\n")
      assert result.variables["x"] == "int"
      assert result.variables["y"] == "string"
    end

    test "min() and max()" do
      result = Interpreter.interpret("x = min([3, 1, 2])\ny = max([3, 1, 2])\n")
      assert result.variables["x"] == 1
      assert result.variables["y"] == 3
    end

    test "sorted()" do
      result = Interpreter.interpret("x = sorted([3, 1, 4, 1, 5])\n")
      assert result.variables["x"] == [1, 1, 3, 4, 5]
    end

    test "int() conversion" do
      result = Interpreter.interpret("x = int(\"42\")\n")
      assert result.variables["x"] == 42
    end

    test "str() conversion" do
      result = Interpreter.interpret("x = str(42)\n")
      assert result.variables["x"] == "42"
    end

    test "bool() conversion" do
      result = Interpreter.interpret("x = bool(1)\ny = bool(0)\n")
      assert result.variables["x"] == true
      assert result.variables["y"] == false
    end

    test "abs()" do
      result = Interpreter.interpret("x = abs(-5)\n")
      assert result.variables["x"] == 5
    end

    test "reversed()" do
      result = Interpreter.interpret("x = reversed([1, 2, 3])\n")
      assert result.variables["x"] == [3, 2, 1]
    end
  end

  # ===========================================================================
  # Chapter 7: Print & Output Capture
  # ===========================================================================

  describe "print and output" do
    test "print captures output" do
      result = Interpreter.interpret("print(42)\n")
      assert result.output == ["42"]
    end

    test "multiple print statements" do
      result = Interpreter.interpret("print(1)\nprint(2)\nprint(3)\n")
      assert result.output == ["1", "2", "3"]
    end

    test "print with string" do
      result = Interpreter.interpret("print(\"hello\")\n")
      assert result.output == ["hello"]
    end
  end

  # ===========================================================================
  # Chapter 8: Subscript & Attribute Access
  # ===========================================================================

  describe "subscript access" do
    test "list indexing" do
      result = Interpreter.interpret("x = [10, 20, 30]\ny = x[1]\n")
      assert result.variables["y"] == 20
    end

    test "dict access" do
      result = Interpreter.interpret("x = {\"a\": 1}\ny = x[\"a\"]\n")
      assert result.variables["y"] == 1
    end

    test "negative indexing" do
      result = Interpreter.interpret("x = [10, 20, 30]\ny = x[-1]\n")
      assert result.variables["y"] == 30
    end
  end

  # ===========================================================================
  # Chapter 9: The load() Function — Module Loading
  # ===========================================================================

  describe "load() — module loading" do
    test "load a simple variable" do
      files = %{
        "//lib.star" => "x = 42\n"
      }

      source = """
      load("//lib.star", "x")
      result = x
      """

      result = Interpreter.interpret(source, file_resolver: files)
      assert result.variables["result"] == 42
    end

    test "load a function and call it" do
      files = %{
        "//rules/math.star" => "def double(n):\n    return n * 2\n"
      }

      source = """
      load("//rules/math.star", "double")
      result = double(21)
      """

      result = Interpreter.interpret(source, file_resolver: files)
      assert result.variables["result"] == 42
    end

    test "load multiple symbols from one file" do
      files = %{
        "//lib.star" => "x = 1\ny = 2\nz = 3\n"
      }

      source = """
      load("//lib.star", "x", "y")
      result = x + y
      """

      result = Interpreter.interpret(source, file_resolver: files)
      assert result.variables["result"] == 3
    end

    test "load from multiple files" do
      files = %{
        "//a.star" => "a_val = 10\n",
        "//b.star" => "b_val = 20\n"
      }

      source = """
      load("//a.star", "a_val")
      load("//b.star", "b_val")
      result = a_val + b_val
      """

      result = Interpreter.interpret(source, file_resolver: files)
      assert result.variables["result"] == 30
    end

    test "load caches files (evaluated once)" do
      # Use a counter to track evaluations via a function resolver
      call_count = :counters.new(1, [:atomics])

      resolver = fn label ->
        :counters.add(call_count, 1, 1)
        case label do
          "//lib.star" -> "val = 42\n"
          other -> raise "Unknown file: #{other}"
        end
      end

      source = """
      load("//lib.star", "val")
      result = val
      """

      _result = Interpreter.interpret(source, file_resolver: resolver)
      assert :counters.get(call_count, 1) == 1
    end

    test "load with no resolver raises error" do
      source = """
      load("//missing.star", "x")
      """

      assert_raise RuntimeError, ~r/no file_resolver configured/, fn ->
        Interpreter.interpret(source)
      end
    end

    test "load with missing file raises error" do
      files = %{"//exists.star" => "x = 1\n"}

      source = """
      load("//missing.star", "x")
      """

      assert_raise RuntimeError, ~r/file not found/, fn ->
        Interpreter.interpret(source, file_resolver: files)
      end
    end

    test "load with function resolver" do
      resolver = fn label ->
        case label do
          "//lib.star" -> "val = 99\n"
          other -> raise "Unknown: #{other}"
        end
      end

      source = """
      load("//lib.star", "val")
      result = val
      """

      result = Interpreter.interpret(source, file_resolver: resolver)
      assert result.variables["result"] == 99
    end
  end

  # ===========================================================================
  # Chapter 10: interpret_file/2
  # ===========================================================================

  describe "interpret_file/2" do
    test "executes a file from the filesystem" do
      # Write a temporary file
      tmp_path = Path.join(System.tmp_dir!(), "starlark_test_#{:rand.uniform(999999)}.star")

      try do
        File.write!(tmp_path, "x = 1 + 2\nprint(x)\n")
        result = Interpreter.interpret_file(tmp_path)
        assert result.variables["x"] == 3
        assert result.output == ["3"]
      after
        File.rm(tmp_path)
      end
    end

    test "adds trailing newline if missing" do
      tmp_path = Path.join(System.tmp_dir!(), "starlark_test_#{:rand.uniform(999999)}.star")

      try do
        File.write!(tmp_path, "x = 42")
        result = Interpreter.interpret_file(tmp_path)
        assert result.variables["x"] == 42
      after
        File.rm(tmp_path)
      end
    end

    test "raises on nonexistent file" do
      assert_raise File.Error, fn ->
        Interpreter.interpret_file("/nonexistent/path/program.star")
      end
    end
  end

  # ===========================================================================
  # Chapter 11: Integration — Complex Programs
  # ===========================================================================

  describe "complex programs" do
    test "FizzBuzz" do
      source = """
      results = []
      for i in range(1, 16):
          if i % 15 == 0:
              results = results + ["FizzBuzz"]
          elif i % 3 == 0:
              results = results + ["Fizz"]
          elif i % 5 == 0:
              results = results + ["Buzz"]
          else:
              results = results + [i]
      """
      result = Interpreter.interpret(source)
      expected = [1, 2, "Fizz", 4, "Buzz", "Fizz", 7, 8, "Fizz", "Buzz",
                  11, "Fizz", 13, 14, "FizzBuzz"]
      assert result.variables["results"] == expected
    end

    test "map-like function over a list" do
      source = """
      def square(n):
          return n * n
      result = []
      for val in [1, 2, 3, 4, 5]:
          result = result + [square(val)]
      """
      result = Interpreter.interpret(source)
      assert result.variables["result"] == [1, 4, 9, 16, 25]
    end

    test "Fibonacci sequence" do
      source = """
      def fib(n):
          if n <= 1:
              return n
          return fib(n - 1) + fib(n - 2)
      result = []
      for i in range(8):
          result = result + [fib(i)]
      """
      result = Interpreter.interpret(source)
      assert result.variables["result"] == [0, 1, 1, 2, 3, 5, 8, 13]
    end

    test "BUILD file simulation" do
      files = %{
        "//rules/python.star" => """
        def py_library(name, srcs = [], deps = []):
            return {"name": name, "srcs": srcs, "deps": deps, "kind": "py_library"}
        """
      }

      source = """
      load("//rules/python.star", "py_library")
      target = py_library(
          name = "mylib",
          srcs = ["main.py", "util.py"],
          deps = ["//other:lib"],
      )
      """

      result = Interpreter.interpret(source, file_resolver: files)
      target = result.variables["target"]
      assert target["name"] == "mylib"
      assert target["srcs"] == ["main.py", "util.py"]
      assert target["deps"] == ["//other:lib"]
      assert target["kind"] == "py_library"
    end

    test "augmented assignment" do
      source = """
      x = 10
      x += 5
      x -= 3
      x *= 2
      """
      result = Interpreter.interpret(source)
      assert result.variables["x"] == 24
    end

    test "string methods via attribute access" do
      source = """
      x = "hello world"
      upper = x.upper()
      """
      result = Interpreter.interpret(source)
      assert result.variables["upper"] == "HELLO WORLD"
    end

    test "membership test" do
      source = """
      x = 2 in [1, 2, 3]
      y = 4 in [1, 2, 3]
      """
      result = Interpreter.interpret(source)
      assert result.variables["x"] == true
      assert result.variables["y"] == false
    end
  end

  # ===========================================================================
  # Chapter 12: Result Structure
  # ===========================================================================

  describe "result structure" do
    test "result has variables, output, and traces" do
      result = Interpreter.interpret("x = 1\nprint(x)\n")
      assert is_map(result.variables)
      assert is_list(result.output)
      assert is_list(result.traces)
    end

    test "empty program produces empty result" do
      result = Interpreter.interpret("\n")
      assert result.variables == %{} or map_size(result.variables) == 0
    end
  end

  # ===========================================================================
  # Chapter 13: Options
  # ===========================================================================

  describe "options" do
    test "max_recursion_depth option" do
      # This should work with default depth
      source = """
      def count(n):
          if n <= 0:
              return 0
          return 1 + count(n - 1)
      result = count(10)
      """
      result = Interpreter.interpret(source, max_recursion_depth: 200)
      assert result.variables["result"] == 10
    end
  end
end
