defmodule CodingAdventures.RankStatisticsTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.RankStatistics, as: Stats

  test "safe median preserves samples" do
    values = [1.7976931348623157e308, -1.7976931348623157e308]
    assert {:ok, midpoint} = Stats.median(values)
    assert midpoint == 0.0
    assert values == [1.7976931348623157e308, -1.7976931348623157e308]
    assert {:ok, 2.5} = Stats.median([1.0, 2.0, 3.0, 4.0])
    assert {:ok, 2.0} = Stats.median([1, 3])
    assert {:ok, singleton} = Stats.median([1])
    assert singleton == 1.0 and is_float(singleton)
    assert {:ok, 9_007_199_254_740_992.0} = Stats.median([9_007_199_254_740_993])
    assert {:ok, {1.0, 6.0}} = Stats.median_interval([1, 2, 3, 4, 5, 6], 95)
  end

  test "rejects integers outside the finite binary64 range" do
    unrepresentable = Integer.pow(2, 1024)

    assert {:error, :nonfinite_sample} = Stats.median([unrepresentable])
    assert {:error, :nonfinite_sample} = Stats.median_interval([1, unrepresentable], 95)
  end

  test "midpoint remains correct at subnormal, normal, and maximum-finite boundaries" do
    <<subnormal::float-64>> = <<1::unsigned-64>>
    <<twice_subnormal::float-64>> = <<2::unsigned-64>>
    assert {:ok, value} = Stats.median([subnormal, twice_subnormal])
    assert value == twice_subnormal
    normal_minimum = 2.2250738585072014e-308
    assert {:ok, normal_midpoint} = Stats.median([normal_minimum, normal_minimum * 2])
    assert normal_midpoint == normal_minimum * 1.5
    max_finite = 1.7976931348623157e308
    assert {:ok, max_midpoint} = Stats.median([max_finite, max_finite])
    assert max_midpoint == max_finite
  end

  test "validates samples and confidence" do
    assert {:error, :empty_sample} = Stats.median([])
    assert {:error, :nonfinite_sample} = Stats.median([:bad])
    assert {:error, :invalid_confidence} = Stats.median_interval([1.0], 0)
  end

  test "small samples match independent exhaustive sign enumeration" do
    for n <- 1..12, confidence <- [1, 25, 50, 75, 95, 99] do
      values = for i <- 0..(n - 1), do: rem(i * 7, 11) / 1
      {:ok, interval} = Stats.median_interval(values, confidence)

      rank =
        Enum.filter(1..(div(n, 2) + 1), &Stats.oracle_rank_qualifies?(n, &1, confidence))
        |> Enum.max(fn -> 0 end)

      if rank == 0 do
        assert interval == nil
      else
        sorted = Enum.sort(values)
        assert interval == {Enum.at(sorted, rank - 1), Enum.at(sorted, n - rank)}
      end
    end
  end

  test "wide sample cutoff is exact" do
    values = Enum.to_list(0..999) |> Enum.map(&(&1 / 1))
    assert {:ok, {468.0, 531.0}} = Stats.median_interval(values, 95)
  end

  test "shared observations match the canonical fixture and frozen wide cutoff" do
    assert File.read!("testdata/microbenchmark-pairs-v1.json") ==
             File.read!("../../../specs/fixtures/microbenchmark-pairs-v1.json")

    fixture = :json.decode(File.read!("testdata/microbenchmark-pairs-v1.json"))

    for item <- fixture["cases"] do
      ratios = Enum.map(item["candidate_ns"], &(&1 / item["baseline_ns"]))
      assert {:ok, ratio} = Stats.median(ratios)
      assert ratio == item["ratio"]
      assert {:ok, interval} = Stats.median_interval(ratios, fixture["confidence_percent"])
      expected = if item["interval"] == :null, do: nil, else: List.to_tuple(item["interval"])
      assert interval == expected
    end

    assert {:ok, {468.0, 531.0}} =
             Stats.median_interval(Enum.to_list(0..999) |> Enum.map(&(&1 / 1)), 95)
  end

  test "rejects non-list inputs and bounded oracle calls" do
    assert {:error, :invalid_sample} = Stats.median(:not_a_list)
    assert {:error, :invalid_confidence} = Stats.median_interval([1.0], 100)
    assert_raise FunctionClauseError, fn -> Stats.oracle_rank_qualifies?(21, 1, 95) end
  end
end
