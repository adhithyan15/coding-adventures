defmodule CorpusConformanceTest do
  use ExUnit.Case, async: true
  @moduletag timeout: 120_000

  alias CodingAdventures.TestBootstrap.{Prng, Pbt.Corpus, Pbt.Hooks, Pbt.Shrink}

  test "the committed corpus replays: 25 evaluated, zero problems" do
    report = Corpus.evaluate(Fx.cases())
    assert report.problems == []
    assert report.evaluated == 25
  end

  test "the corpus has exactly 25 cases, strictly parsed" do
    assert length(Fx.cases()) == Corpus.case_count()
  end

  test "known step counts are exact" do
    assert Shrink.shrink_int(54_321, 0, 0, 100_000, 1000, fn x -> x <= 100 end) == {101, 15}
    assert Shrink.shrink_list([10, 5, 8, 4], 0, 1000, fn l -> Enum.sum(l) <= 20 end) |> elem(0) == [8, 5, 8]
  end

  test "cases.tfv is accepted by the strict TFV validator" do
    assert CodingAdventures.TestBootstrap.Tfv.valid?(File.read!(Fx.fixture_path("cases.tfv")))
  end

  # -- negative controls ------------------------------------------------------------

  defp replace(cases, index, fun), do: List.update_at(cases, index, fun)

  defp bump(%{"kind" => "int", "value" => v}), do: Fx.int(String.to_integer(v) + 1)
  defp bump(%{"kind" => "bytes", "hex" => h}), do: %{"kind" => "bytes", "hex" => h <> "00"}
  defp bump(b) when is_boolean(b), do: not b

  # every corruption keeps the case RUNNABLE, so the only problems are its own
  defp corruptions(%{"op" => op} = c) when op in ["gen_primitive", "gen_bounded", "gen_bytes"] do
    [%{c | "outputs" => List.update_at(c["outputs"], 0, &bump/1)}]
  end

  defp corruptions(%{"op" => op} = c) when op in ["shrink_int", "shrink_refinement", "shrink_list", "shrink_bytes"] do
    minimal =
      case c["expected_minimal"] do
        l when is_list(l) -> List.update_at(l, 0, &bump/1)
        other -> bump(other)
      end

    [%{c | "expected_minimal" => minimal}, %{c | "expected_steps" => bump(c["expected_steps"])}]
  end

  defp corruptions(%{"op" => "replay_consistency"} = c),
    do: [%{c | "initial_fail" => bump(c["initial_fail"])}, %{c | "expected_minimal" => bump(c["expected_minimal"])}]

  defp corruptions(%{"op" => "filter_exhausted"} = c), do: [%{c | "failure_id" => "property-panic"}]
  defp corruptions(%{"op" => "failure_id_valid"} = c), do: [%{c | "valid" => not c["valid"]}]
  defp corruptions(%{"op" => "corpus_reject"} = c), do: [%{c | "text" => "tfv 1\n"}]

  test "per-op corruption of every case fails and names exactly that case" do
    cases = Fx.cases()

    for {c, i} <- Enum.with_index(cases), corrupted <- corruptions(c) do
      report = Corpus.evaluate(replace(cases, i, fn _ -> corrupted end))
      assert report.problems != [], "#{c["id"]} corruption was not detected"

      for p <- report.problems do
        assert String.starts_with?(p, c["id"]), "problem #{inspect(p)} does not start with #{c["id"]}"
      end
    end
  end

  test "dropped and truncated corpora fail" do
    cases = Fx.cases()
    assert Corpus.evaluate(tl(cases)).problems != []
    assert Corpus.evaluate(Enum.take(cases, 10)).problems != []
    assert Corpus.evaluate(cases ++ [hd(cases)]).problems != []
    raw = Fx.raw()
    assert_raise CodingAdventures.TestBootstrap.CorpusJson.StrictJson.JsonError, fn -> Corpus.parse(binary_part(raw, 0, byte_size(raw) - 10)) end
    assert_raise CodingAdventures.TestBootstrap.CorpusJson.StrictJson.JsonError, fn -> Corpus.parse("{}") end
    assert_raise CodingAdventures.TestBootstrap.CorpusJson.StrictJson.JsonError, fn -> Corpus.parse(~s({"schema_version": 2, "cases": []})) end
    assert_raise CodingAdventures.TestBootstrap.CorpusJson.StrictJson.JsonError, fn -> Corpus.parse(~s({"schema_version": 1, "cases": 3})) end
  end

  test "malformed cases are problems, never silent skips" do
    cases = Fx.cases()
    one = hd(cases)

    for bad <- [
          %{one | "op" => "no_such_op"},
          Map.delete(one, "outputs"),
          Map.put(one, "extra", 1),
          %{one | "id" => ""},
          %{one | "count" => Fx.int(-1)},
          %{one | "outputs" => []},
          %{one | "type" => "nope"},
          %{one | "seed" => "text"},
          %{one | "seed" => 3.5},
          %{one | "seed" => %{"kind" => "int", "value" => "007"}},
          "not an object",
          %{one | "count" => %{"kind" => "int", "value" => String.duplicate("9", 2000)}}
        ] do
      report = Corpus.evaluate(replace(cases, 0, fn _ -> bad end))
      assert report.problems != [], "#{inspect(bad)} was accepted"
    end

    dup = Corpus.evaluate([one, one], nil)
    assert Enum.any?(dup.problems, &String.contains?(&1, "duplicate or missing case id"))
    assert Corpus.evaluate([], nil).problems != []

    many = List.duplicate(%{}, 10_001)
    assert Enum.any?(Corpus.evaluate(many, nil).problems, &String.contains?(&1, "exceeds the cap"))
  end

  test "bad property names, bad shapes and unevaluable engines are problems" do
    cases = Fx.cases()

    ints = Enum.find_index(cases, &(&1["op"] == "shrink_int"))
    lists = Enum.find_index(cases, &(&1["op"] == "shrink_list"))
    bytes = Enum.find_index(cases, &(&1["op"] == "shrink_bytes"))
    replay = Enum.find_index(cases, &(&1["op"] == "replay_consistency"))
    filt = Enum.find_index(cases, &(&1["op"] == "filter_exhausted"))
    fid = Enum.find_index(cases, &(&1["op"] == "failure_id_valid"))
    rej = Enum.find_index(cases, &(&1["op"] == "corpus_reject"))

    mutate = fn i, key, val -> replace(cases, i, &Map.put(&1, key, val)) end

    for mutated <- [
          mutate.(ints, "property", "nope"),
          mutate.(ints, "property", 5),
          mutate.(ints, "initial", "x"),
          mutate.(ints, "initial", Fx.int(-5000)),
          mutate.(ints, "initial", Fx.int(1)),
          mutate.(lists, "initial", Fx.int(1)),
          mutate.(lists, "initial", [Fx.int(1), true]),
          mutate.(lists, "initial", [Fx.int(1)]),
          mutate.(bytes, "initial", Fx.int(1)),
          mutate.(bytes, "initial", %{"kind" => "bytes", "hex" => ""}),
          mutate.(replay, "property", "int_le_100_x"),
          mutate.(replay, "property", "int_is_even"),
          mutate.(filt, "max_retries", Fx.int(0)),
          mutate.(filt, "verdict", "pass"),
          mutate.(fid, "input", Fx.int(3)),
          mutate.(rej, "text", Fx.int(3))
        ] do
      assert Corpus.evaluate(mutated).problems != []
    end
  end

  test "gen cases are bounded before generating" do
    cases = Fx.cases()
    i = Enum.find_index(cases, &(&1["op"] == "gen_bytes"))
    big = replace(cases, i, &%{&1 | "count" => Fx.int(10_000), "max_len" => Fx.int(10_000), "outputs" => []})
    assert Enum.any?(Corpus.evaluate(big).problems, &String.contains?(&1, "#{Enum.at(cases, i)["id"]}"))
  end

  # -- engine sabotage hooks are caught -------------------------------------------------------

  test "every sabotage hook makes the corpus run fail" do
    cases = Fx.cases()

    sabotage = [
      bisect_candidate: fn current, _diff -> current end,
      unit_candidate: fn current, _diff -> current end,
      skip_deletion_phase: true,
      new_rng: fn seed -> Prng.new(seed + 1) end
    ]

    for {name, value} <- sabotage do
      report = Hooks.with([{name, value}], fn -> Corpus.evaluate(cases) end)
      assert report.problems != [], "hook #{name} was not caught"
    end

    # and nothing leaked
    assert Corpus.evaluate(cases).problems == []
  end

  test "hooks reject unknown names and restore on raise" do
    assert_raise ArgumentError, fn -> Hooks.with([nope: 1], fn -> :ok end) end

    assert_raise RuntimeError, fn ->
      Hooks.with([skip_deletion_phase: true], fn -> raise "boom" end)
    end

    assert Hooks.get(:skip_deletion_phase) == false
  end
end
