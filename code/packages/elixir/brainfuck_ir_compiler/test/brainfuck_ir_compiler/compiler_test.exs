defmodule CodingAdventures.BrainfuckIrCompiler.CompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BrainfuckIrCompiler
  alias CodingAdventures.BrainfuckIrCompiler.BuildConfig
  alias CodingAdventures.CompilerIr.{Printer, Parser}
  alias CodingAdventures.Parser.ASTNode

  # ── Test helpers ─────────────────────────────────────────────────────────────

  # Parse and compile Brainfuck source.
  defp compile_source(source, config) do
    case CodingAdventures.Brainfuck.parse(source) do
      {:ok, ast} -> BrainfuckIrCompiler.compile(ast, "test.bf", config)
      {:error, _} = err -> err
    end
  end

  # Compile and crash on error — for tests that should always succeed.
  defp must_compile(source, config \\ BuildConfig.release_config()) do
    {:ok, result} = compile_source(source, config)
    result
  end

  # Count instructions with a given opcode in the program.
  defp count_opcode(program, opcode) do
    Enum.count(program.instructions, fn i -> i.opcode == opcode end)
  end

  # Check if the program contains a label with the given name.
  defp has_label?(program, name) do
    Enum.any?(program.instructions, fn i ->
      i.opcode == :label and
        Enum.any?(i.operands, fn op ->
          match?(%{name: ^name}, op)
        end)
    end)
  end

  # ── Empty program ─────────────────────────────────────────────────────────────

  describe "empty program" do
    test "has _start label" do
      result = must_compile("")
      assert has_label?(result.program, "_start")
    end

    test "has exactly one HALT" do
      result = must_compile("")
      assert count_opcode(result.program, :halt) == 1
    end

    test "version is 1" do
      result = must_compile("")
      assert result.program.version == 1
    end

    test "entry_label is _start" do
      result = must_compile("")
      assert result.program.entry_label == "_start"
    end

    test "has tape data declaration" do
      result = must_compile("")
      assert length(result.program.data) == 1
      [d] = result.program.data
      assert d.label == "tape"
      assert d.size == 30_000
      assert d.init == 0
    end
  end

  # ── INC (+) ──────────────────────────────────────────────────────────────────

  describe "INC command (+)" do
    test "produces at least one LOAD_BYTE" do
      result = must_compile("+")
      assert count_opcode(result.program, :load_byte) >= 1
    end

    test "produces at least one STORE_BYTE" do
      result = must_compile("+")
      assert count_opcode(result.program, :store_byte) >= 1
    end

    test "produces AND_IMM for byte masking (release)" do
      result = must_compile("+")
      assert count_opcode(result.program, :and_imm) >= 1
    end

    test "no AND_IMM when masking disabled" do
      config = %{BuildConfig.release_config() | mask_byte_arithmetic: false}
      result = must_compile("+", config)
      assert count_opcode(result.program, :and_imm) == 0
    end
  end

  # ── DEC (-) ──────────────────────────────────────────────────────────────────

  describe "DEC command (-)" do
    test "produces ADD_IMM with value -1" do
      result = must_compile("-")

      found =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :add_imm and
            length(i.operands) >= 3 and
            match?(%{value: -1}, Enum.at(i.operands, 2))
        end)

      assert found, "Expected ADD_IMM with -1 for DEC"
    end
  end

  # ── RIGHT (>) ────────────────────────────────────────────────────────────────

  describe "RIGHT command (>)" do
    test "produces ADD_IMM v1, v1, 1" do
      result = must_compile(">")

      found =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :add_imm and
            length(i.operands) >= 3 and
            match?(%{index: 1}, hd(i.operands)) and
            match?(%{value: 1}, Enum.at(i.operands, 2))
        end)

      assert found, "Expected ADD_IMM v1, v1, 1"
    end
  end

  # ── LEFT (<) ─────────────────────────────────────────────────────────────────

  describe "LEFT command (<)" do
    test "produces ADD_IMM v1, v1, -1" do
      result = must_compile("<")

      found =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :add_imm and
            length(i.operands) >= 3 and
            match?(%{index: 1}, hd(i.operands)) and
            match?(%{value: -1}, Enum.at(i.operands, 2))
        end)

      assert found, "Expected ADD_IMM v1, v1, -1"
    end
  end

  # ── OUTPUT (.) ───────────────────────────────────────────────────────────────

  describe "OUTPUT command (.)" do
    test "produces SYSCALL" do
      result = must_compile(".")
      assert count_opcode(result.program, :syscall) >= 1
    end

    test "SYSCALL number is 1 (write)" do
      result = must_compile(".")

      found =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :syscall and
            length(i.operands) > 0 and
            match?(%{value: 1}, hd(i.operands))
        end)

      assert found, "Expected SYSCALL 1 for OUTPUT"
    end

    test "copies the byte into v4 with ADD_IMM 0 in release mode" do
      result = must_compile(".")

      found =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :add_imm and
            length(i.operands) >= 3 and
            match?(%{index: 4}, hd(i.operands)) and
            match?(%{index: 2}, Enum.at(i.operands, 1)) and
            match?(%{value: 0}, Enum.at(i.operands, 2))
        end)

      assert found, "Expected ADD_IMM v4, v2, 0 for OUTPUT"
    end
  end

  # ── INPUT (,) ────────────────────────────────────────────────────────────────

  describe "INPUT command (,)" do
    test "produces SYSCALL 2 (read)" do
      result = must_compile(",")

      found =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :syscall and
            length(i.operands) > 0 and
            match?(%{value: 2}, hd(i.operands))
        end)

      assert found, "Expected SYSCALL 2 for INPUT"
    end
  end

  # ── Loop ─────────────────────────────────────────────────────────────────────

  describe "loop [body]" do
    test "has loop_0_start label" do
      result = must_compile("[-]")
      assert has_label?(result.program, "loop_0_start")
    end

    test "has loop_0_end label" do
      result = must_compile("[-]")
      assert has_label?(result.program, "loop_0_end")
    end

    test "has BRANCH_Z for loop entry" do
      result = must_compile("[-]")
      assert count_opcode(result.program, :branch_z) >= 1
    end

    test "has JUMP for loop back-edge" do
      result = must_compile("[-]")
      assert count_opcode(result.program, :jump) >= 1
    end

    test "empty loop still has labels" do
      result = must_compile("[]")
      assert has_label?(result.program, "loop_0_start")
      assert has_label?(result.program, "loop_0_end")
    end

    test "nested loops have unique labels" do
      result = must_compile("[>[+<-]]")
      assert has_label?(result.program, "loop_0_start")
      assert has_label?(result.program, "loop_1_start")
    end
  end

  # ── Debug mode: bounds checking ───────────────────────────────────────────────

  describe "debug mode bounds checking" do
    test "RIGHT in debug mode adds CMP_GT" do
      result = must_compile(">", BuildConfig.debug_config())
      assert count_opcode(result.program, :cmp_gt) >= 1
    end

    test "RIGHT in debug mode adds BRANCH_NZ" do
      result = must_compile(">", BuildConfig.debug_config())
      assert count_opcode(result.program, :branch_nz) >= 1
    end

    test "RIGHT in debug mode has __trap_oob label" do
      result = must_compile(">", BuildConfig.debug_config())
      assert has_label?(result.program, "__trap_oob")
    end

    test "LEFT in debug mode adds CMP_LT" do
      result = must_compile("<", BuildConfig.debug_config())
      assert count_opcode(result.program, :cmp_lt) >= 1
    end

    test "no CMP_GT in release mode" do
      result = must_compile("><")
      assert count_opcode(result.program, :cmp_gt) == 0
    end

    test "no CMP_LT in release mode" do
      result = must_compile("><")
      assert count_opcode(result.program, :cmp_lt) == 0
    end

    test "no __trap_oob in release mode" do
      result = must_compile("><")
      refute has_label?(result.program, "__trap_oob")
    end
  end

  # ── Source map ────────────────────────────────────────────────────────────────

  describe "source map" do
    test "+. produces 2 SourceToAst entries" do
      result = must_compile("+.")
      assert length(result.source_map.source_to_ast.entries) == 2
    end

    test "first entry for + is at column 1" do
      result = must_compile("+.")
      entry = hd(result.source_map.source_to_ast.entries)
      assert entry.pos.column == 1
    end

    test "second entry for . is at column 2" do
      result = must_compile("+.")
      entry = Enum.at(result.source_map.source_to_ast.entries, 1)
      assert entry.pos.column == 2
    end

    test "file name is recorded in source positions" do
      result = must_compile("+")

      for entry <- result.source_map.source_to_ast.entries do
        assert entry.pos.file == "test.bf"
      end
    end

    test "+ produces 4 IR IDs (LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE)" do
      result = must_compile("+")
      assert length(result.source_map.ast_to_ir.entries) == 1
      [entry] = result.source_map.ast_to_ir.entries
      assert length(entry.ir_ids) == 4
    end

    test "[-] has at least 2 SourceToAst entries (loop + command)" do
      result = must_compile("[-]")
      assert length(result.source_map.source_to_ast.entries) >= 2
    end
  end

  # ── IR text output ────────────────────────────────────────────────────────────

  describe "IR text output" do
    test "printed IR contains .version 1" do
      result = must_compile("+.")
      text = Printer.print(result.program)
      assert String.contains?(text, ".version 1")
    end

    test "printed IR contains .data tape 30000 0" do
      result = must_compile("+.")
      text = Printer.print(result.program)
      assert String.contains?(text, ".data tape 30000 0")
    end

    test "printed IR contains .entry _start" do
      result = must_compile("+.")
      text = Printer.print(result.program)
      assert String.contains?(text, ".entry _start")
    end

    test "printed IR contains LOAD_BYTE" do
      result = must_compile("+.")
      text = Printer.print(result.program)
      assert String.contains?(text, "LOAD_BYTE")
    end

    test "printed IR contains HALT" do
      result = must_compile("+.")
      text = Printer.print(result.program)
      assert String.contains?(text, "HALT")
    end
  end

  # ── Roundtrip: print → parse ──────────────────────────────────────────────────

  describe "IR roundtrip" do
    test "parsed instruction count matches original" do
      result = must_compile("++[-].")
      text = Printer.print(result.program)
      {:ok, parsed} = Parser.parse(text)
      assert length(parsed.instructions) == length(result.program.instructions)
    end

    test "simple program roundtrips" do
      result = must_compile(">+<-")
      text = Printer.print(result.program)
      assert {:ok, _} = Parser.parse(text)
    end
  end

  # ── Custom tape size ──────────────────────────────────────────────────────────

  describe "custom tape size" do
    test "tape data size matches config" do
      config = %{BuildConfig.release_config() | tape_size: 1000}
      result = must_compile("", config)
      [d] = result.program.data
      assert d.size == 1000
    end
  end

  # ── Instruction ID uniqueness ─────────────────────────────────────────────────

  describe "instruction ID uniqueness" do
    test "all instruction IDs are unique (labels excluded)" do
      result = must_compile("++[>+<-].")

      ids =
        result.program.instructions
        |> Enum.reject(fn i -> i.id == -1 end)
        |> Enum.map(fn i -> i.id end)

      assert length(ids) == length(Enum.uniq(ids))
    end
  end

  # ── Complex programs ──────────────────────────────────────────────────────────

  describe "complex programs" do
    test "Hello World subset has a loop and output syscall" do
      # Simplified: 8 increments, then a loop, then output
      source = "++++++++[>+++++++++<-]>."
      result = must_compile(source)

      assert has_label?(result.program, "loop_0_start")

      found_write =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :syscall and
            length(i.operands) > 0 and
            match?(%{value: 1}, hd(i.operands))
        end)

      assert found_write
    end

    test "cat program ,[.,] has read and write syscalls" do
      result = must_compile(",[.,]")

      found_read =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :syscall and
            length(i.operands) > 0 and
            match?(%{value: 2}, hd(i.operands))
        end)

      found_write =
        Enum.any?(result.program.instructions, fn i ->
          i.opcode == :syscall and
            length(i.operands) > 0 and
            match?(%{value: 1}, hd(i.operands))
        end)

      assert found_read, "expected SYSCALL 2 (read) in cat program"
      assert found_write, "expected SYSCALL 1 (write) in cat program"
    end
  end

  # ── Error cases ───────────────────────────────────────────────────────────────

  describe "error cases" do
    test "non-program AST root returns error" do
      ast = %ASTNode{rule_name: "not_a_program", children: []}
      assert {:error, msg} = BrainfuckIrCompiler.compile(ast, "t.bf", BuildConfig.release_config())
      assert String.contains?(msg, "program")
    end

    test "zero tape size returns error" do
      {:ok, ast} = CodingAdventures.Brainfuck.parse("")
      config = %{BuildConfig.release_config() | tape_size: 0}
      assert {:error, msg} = BrainfuckIrCompiler.compile(ast, "t.bf", config)
      assert String.contains?(msg, "tape_size")
    end

    test "negative tape size returns error" do
      {:ok, ast} = CodingAdventures.Brainfuck.parse("")
      config = %{BuildConfig.release_config() | tape_size: -1}
      assert {:error, msg} = BrainfuckIrCompiler.compile(ast, "t.bf", config)
      assert String.contains?(msg, "tape_size")
    end
  end
end
