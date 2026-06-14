# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_tetrad_runtime"

class TetradRuntimeTest < Minitest::Test
  def test_compiles_function_to_iir
    mod = CodingAdventures::TetradRuntime::Compiler.compile_source("fn main() { return 40 + 2 }")

    assert_equal "tetrad", mod.language
    assert_equal "main", mod.entry_point
    assert mod.get_function("main").instructions.any? { |instr| instr.op == "add" }
  end

  def test_runs_on_lang_vm
    rt = CodingAdventures::TetradRuntime::Runtime.new

    assert_equal 42, rt.run("fn main() { let x = 40 return x + 2 }")
  end

  def test_wraps_u8_values
    rt = CodingAdventures::TetradRuntime::Runtime.new

    assert_equal 4, rt.run("fn main() { return 250 + 10 }")
  end

  def test_emits_backend_artifacts
    rt = CodingAdventures::TetradRuntime::Runtime.new
    artifact = rt.emit("fn main() { return 1 }", target: :jvm)

    assert_equal :jvm, artifact.target
    assert_includes artifact.body, "language=tetrad"
  end
end
