defmodule CodingAdventures.HyperLogLogTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HyperLogLog

  test "count stays small for duplicates" do
    hll = Enum.reduce(1..100, HyperLogLog.new(), fn _, acc -> HyperLogLog.add(acc, "same") end)
    assert HyperLogLog.count(hll) < 10
  end

  test "merge combines sketches" do
    left = Enum.reduce(1..100, HyperLogLog.new(precision: 10), fn i, acc -> HyperLogLog.add(acc, i) end)
    right = Enum.reduce(101..200, HyperLogLog.new(precision: 10), fn i, acc -> HyperLogLog.add(acc, i) end)

    merged = HyperLogLog.merge(left, right)
    assert HyperLogLog.count(merged) > 150
  end

  test "metadata helpers and mismatch errors are covered" do
    hll = HyperLogLog.new(precision: 12)
    assert HyperLogLog.precision(hll) == 12
    assert HyperLogLog.num_registers(hll) == 4096
    assert HyperLogLog.memory_bytes(12) > 0
    assert HyperLogLog.error_rate(hll) > 0.0
    assert HyperLogLog.registers(hll) |> Enum.all?(&(&1 == 0))
    assert HyperLogLog.len(hll) == 0
    assert HyperLogLog.from_values([1, 2, 3], precision: 12) |> HyperLogLog.count() >= 3
    assert {:error, :precision_mismatch} = HyperLogLog.try_merge(HyperLogLog.new(precision: 10), HyperLogLog.new(precision: 11))
  end
end
