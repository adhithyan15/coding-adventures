defmodule CodingAdventures.LogicGates.BoomStruct do
  defstruct [:x]

  defimpl Inspect do
    def inspect(_, _), do: raise("inspect must never be called")
  end

  defimpl String.Chars do
    def to_string(_), do: raise("to_string must never be called")
  end
end

defmodule CodingAdventures.LogicGates.GatesExhaustiveTest do
  @moduledoc """
  Exhaustive validation tests for the eleven basic gates (LG02 sections 7, 9).

  Every gate must (1) match its literal truth table, (2) reject every
  non-bit value in every argument position with `InvalidBitError`, naming the
  FIRST invalid parameter by the gate's own parameter name.
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.LogicGates.Gates
  alias CodingAdventures.LogicGates.BoomStruct, as: Boom
  alias CodingAdventures.LogicGates.InvalidBitError

  # {name, arity, fun, truth table as list of {args, expected}}
  @unary [{:not_gate, [{[0], 1}, {[1], 0}]}, {:nand_not, [{[0], 1}, {[1], 0}]}]

  @binary %{
    and_gate: [0, 0, 0, 1],
    or_gate: [0, 1, 1, 1],
    xor_gate: [0, 1, 1, 0],
    nand_gate: [1, 1, 1, 0],
    nor_gate: [1, 0, 0, 0],
    xnor_gate: [1, 0, 0, 1],
    nand_and: [0, 0, 0, 1],
    nand_or: [0, 1, 1, 1],
    nand_xor: [0, 1, 1, 0]
  }

  @inputs [{0, 0}, {0, 1}, {1, 0}, {1, 1}]

  @int_bad Enum.reject(-1024..1024, &(&1 in [0, 1])) ++
             [
               2 ** 31,
               -(2 ** 31),
               2 ** 63,
               -(2 ** 63),
               2 ** 64,
               -(2 ** 64),
               10 ** 100,
               -(10 ** 100)
             ]

  defp type_bad do
    [
      true,
      false,
      nil,
      :a,
      :"1",
      1.0,
      0.0,
      -0.0,
      1.5,
      "1",
      "",
      ~c"1",
      [1],
      [],
      {1},
      {},
      %{},
      %{a: 1},
      %Boom{x: 1},
      %Boom{},
      self(),
      make_ref(),
      &Function.identity/1,
      fn -> 1 end
    ]
  end

  defp bad, do: @int_bad ++ type_bad()

  for {name, table} <- @unary do
    test "#{name} truth table" do
      for {[a], expected} <- unquote(Macro.escape(table)) do
        assert apply(Gates, unquote(name), [a]) === expected
      end
    end

    test "#{name} rejects every non-bit with param a" do
      for bad <- bad() do
        e = assert_raise InvalidBitError, fn -> apply(Gates, unquote(name), [bad]) end
        assert e.param == "a"
        assert String.length(e.value_description) <= 64
        assert Exception.message(e) =~ "a must be the integer 0 or 1"
      end
    end
  end

  for {name, outs} <- @binary do
    test "#{name} truth table" do
      results = for {a, b} <- @inputs, do: apply(Gates, unquote(name), [a, b])
      assert results === unquote(outs)
      assert Enum.all?(results, &(is_integer(&1) and &1 in [0, 1]))
    end

    test "#{name} rejects every non-bit in every position" do
      for bad <- bad(), good <- [0, 1] do
        e1 = assert_raise InvalidBitError, fn -> apply(Gates, unquote(name), [bad, good]) end
        assert e1.param == "a"
        e2 = assert_raise InvalidBitError, fn -> apply(Gates, unquote(name), [good, bad]) end
        assert e2.param == "b"
        assert String.length(e1.value_description) <= 64
      end
    end

    test "#{name} reports the first invalid param when both are bad" do
      for bad <- Enum.take(bad(), 40), bad2 <- [2, true, 1.0] do
        e = assert_raise InvalidBitError, fn -> apply(Gates, unquote(name), [bad, bad2]) end
        assert e.param == "a"
      end
    end
  end

  test "NAND-built gates equal direct gates" do
    for {a, b} <- @inputs do
      assert Gates.nand_and(a, b) === Gates.and_gate(a, b)
      assert Gates.nand_or(a, b) === Gates.or_gate(a, b)
      assert Gates.nand_xor(a, b) === Gates.xor_gate(a, b)
    end

    for a <- [0, 1], do: assert(Gates.nand_not(a) === Gates.not_gate(a))
  end

  test "huge integers give a short message promptly" do
    e = assert_raise InvalidBitError, fn -> Gates.and_gate(Integer.pow(10, 100_000), 0) end
    assert String.length(Exception.message(e)) < 120
    assert e.value_description =~ "bits"
  end

  test "exact-boundary integers print exactly" do
    e = assert_raise InvalidBitError, fn -> Gates.not_gate(2 ** 64) end
    assert e.value_description == "integer 18446744073709551616"
  end

  test "validate_bit!/1 default param and ok result" do
    assert Gates.validate_bit!(1) == :ok
    e = assert_raise InvalidBitError, fn -> Gates.validate_bit!(2) end
    assert e.param == "input"
  end

  test "describe covers remaining types" do
    desc = fn v ->
      assert_raise(InvalidBitError, fn -> Gates.validate_bit!(v, "x") end).value_description
    end

    assert desc.(true) == "boolean true"
    assert desc.(nil) == "nil"
    assert desc.(:a) == "an atom"
    assert desc.(1.0) == "a float"
    assert desc.("1") == "a binary"
    assert desc.(~c"1") == "a list"
    assert desc.({1}) == "a tuple"
    assert desc.(%Boom{}) == "a struct"
    assert desc.(%{}) == "a map"
    assert desc.(self()) == "a pid"
    assert desc.(make_ref()) == "a reference"
    assert desc.(fn -> 1 end) == "a function"
    assert desc.(hd(Port.list())) == "a port"
  end

  test "no valid input raises and results are bits" do
    for {a, b} <- @inputs, name <- Map.keys(@binary) do
      r = apply(Gates, name, [a, b])
      assert is_integer(r) and r in [0, 1]
    end
  end
end
