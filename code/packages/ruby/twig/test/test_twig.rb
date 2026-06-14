# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_twig"

class TwigTest < Minitest::Test
  def test_compiles_to_iir
    mod = CodingAdventures::Twig::Compiler.compile_source("(+ 1 2)")

    assert_equal "twig", mod.language
    assert mod.get_function("main").instructions.any? { |instr| instr.op == "call_builtin" }
  end

  def test_runs_arithmetic
    out, value = CodingAdventures::Twig::Runtime.new.run("(+ 40 2)")

    assert_equal "", out
    assert_equal 42, value
  end

  def test_define_function_and_print
    source = "(define (inc x) (+ x 1)) (print (inc 4)) (inc 41)"
    out, value = CodingAdventures::Twig::Runtime.new.run(source)

    assert_equal "5\n", out
    assert_equal 42, value
  end

  def test_if_uses_lang_vm_branches
    out, value = CodingAdventures::Twig::Runtime.new.run("(if #f 1 42)")

    assert_equal "", out
    assert_equal 42, value
  end

  def test_emits_backend_artifact
    artifact = CodingAdventures::Twig::Runtime.new.emit("(+ 1 2)", target: :wasm)

    assert_equal :wasm, artifact.target
    assert_includes artifact.body, "language=twig"
  end
end
