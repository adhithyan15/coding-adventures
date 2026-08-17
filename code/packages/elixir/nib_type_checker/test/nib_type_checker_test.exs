defmodule CodingAdventures.NibTypeCheckerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.NibParser
  alias CodingAdventures.NibTypeChecker

  defp tc(source) do
    {:ok, ast} = NibParser.parse_nib(source)
    NibTypeChecker.check(ast)
  end

  test "accepts function calls and returns" do
    result = tc("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }")
    assert result.ok
  end

  test "accepts the loop subset" do
    result =
      tc("""
      fn count_to(n: u4) -> u4 {
        let acc: u4 = 0;
        for i: u4 in 0..n {
          acc = acc +% 1;
        }
        return acc;
      }
      """)

    assert result.ok
  end

  test "reports assignment mismatches" do
    result = tc("fn main() { let flag: bool = true; flag = 1; }")
    refute result.ok
    assert Enum.any?(result.errors, &String.contains?(&1.message, "assignment"))
  end

  test "reports arity mismatches" do
    result = tc("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(1); }")
    refute result.ok
    assert Enum.any?(result.errors, &String.contains?(&1.message, "expects 2 args"))
  end

  test "accepts multiplicative expressions (regression: mul_expr level)" do
    result = tc("fn double(n: u4) -> u4 { return n * 2; }")
    assert result.ok
  end

  test "reports multiplicative type mismatches" do
    result = tc("fn bad() -> u4 { let b: bool = true; let n: u4 = 1; return b * n; }")
    refute result.ok
    assert Enum.any?(result.errors, &String.contains?(&1.message, "type mismatch"))
  end

  test "accepts plain additive expressions (regression: shift_expr level)" do
    result = tc("fn add(a: u4, b: u4) -> u4 { return a + b; }")
    assert result.ok
  end
end
