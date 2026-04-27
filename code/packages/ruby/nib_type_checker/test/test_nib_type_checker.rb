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
end
