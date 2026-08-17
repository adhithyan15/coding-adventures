# frozen_string_literal: true

require_relative "test_helper"

class NibTypeCheckerTest < Minitest::Test
  def tc(source)
    ast = CodingAdventures::NibParser.parse_nib(source)
    CodingAdventures::NibTypeChecker.check(ast)
  end

  def test_accepts_function_call_and_return_pipeline_shape
    result = tc(<<~NIB)
      fn add(a: u4, b: u4) -> u4 { return a +% b; }
      fn main() -> u4 { return add(3, 4); }
    NIB

    assert result.ok
  end

  def test_accepts_for_loop_subset
    result = tc(<<~NIB)
      fn count_to(n: u4) -> u4 {
        let acc: u4 = 0;
        for i: u4 in 0..n {
          acc = acc +% 1;
        }
        return acc;
      }
    NIB

    assert result.ok
  end

  def test_reports_assignment_type_mismatch
    result = tc("fn main() { let flag: bool = true; flag = 1; }")

    refute result.ok
    assert_includes result.errors.first.message, "assignment"
  end

  def test_reports_call_arity_errors
    result = tc("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(1); }")

    refute result.ok
    assert_includes result.errors.first.message, "expects 2 args"
  end

  # Regression test for #11257: that PR inserted a `shift_expr` precedence
  # level between `add_expr` and `mul_expr` in the shared nib.grammar file
  # (`add_expr -> shift_expr -> mul_expr -> bitwise_expr`) to support `<<`/`>>`,
  # but only updated the Rust consumers. Ruby's nib_parser reads the shared
  # grammar at runtime, so it started wrapping every add_expr operand in a
  # shift_expr node it didn't recognize -- `expression_rule?` filtered
  # shift_expr out, so `expression_children` on the add_expr saw zero
  # operands and check_add_expr inferred no type, even for a plain `a + b`
  # with the PLUS operator (as opposed to the `+%` WRAP_ADD operator used
  # elsewhere in this file, which happens to exercise the same bug already).
  def test_accepts_plain_additive_expression
    result = tc("fn add(a: u4, b: u4) -> u4 { return a + b; } fn main() -> u4 { return add(3, 4); }")

    assert result.ok, result.errors.map(&:message).join("\n")
  end
end
