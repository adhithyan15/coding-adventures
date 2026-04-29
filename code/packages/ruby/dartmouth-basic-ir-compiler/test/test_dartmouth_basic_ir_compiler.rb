# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_dartmouth_basic_ir_compiler"

class DartmouthBasicIrCompilerTest < Minitest::Test
  def test_compiles_to_iir
    result = CodingAdventures::DartmouthBasicIrCompiler::Compiler.compile_source("10 LET A = 40 + 2\n20 END\n")

    assert_equal "dartmouth-basic", result.module.language
    assert_includes result.var_names, "A"
    assert result.module.get_function("main").instructions.any? { |instr| instr.op == "add" }
  end

  def test_runs_print_program_on_lang_vm
    output = CodingAdventures::DartmouthBasicIrCompiler::Runtime.new.run("10 LET A = 40 + 2\n20 PRINT A\n30 END\n")

    assert_equal "42\n", output
  end

  def test_goto_and_if_lower_to_branches
    source = "10 LET A = 1\n20 IF A = 1 THEN 40\n30 PRINT 0\n40 PRINT 1\n50 END\n"
    output = CodingAdventures::DartmouthBasicIrCompiler::Runtime.new.run(source)

    assert_equal "1\n", output
  end

  def test_for_next_loop
    source = "10 FOR I = 1 TO 3\n20 PRINT I\n30 NEXT I\n40 END\n"
    output = CodingAdventures::DartmouthBasicIrCompiler::Runtime.new.run(source)

    assert_equal "1\n2\n3\n", output
  end

  def test_backend_artifact
    artifact = CodingAdventures::DartmouthBasicIrCompiler::Runtime.new.emit("10 PRINT \"HI\"\n20 END\n", target: :clr)

    assert_equal :clr, artifact.target
    assert_includes artifact.body, "language=dartmouth-basic"
  end
end
