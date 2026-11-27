defmodule CodingAdventures.MicrobenchmarkTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.{ArrayWorkload, Microbenchmark}

  test "fixture copy is byte-for-byte canonical" do
    assert File.read!("testdata/microbenchmark-pairs-v1.json") ==
             File.read!("../../../specs/fixtures/microbenchmark-pairs-v1.json")
  end

  test "indexed and iterator callbacks read mutated input" do
    values = ArrayWorkload.make_values()
    assert ArrayWorkload.indexed_batch(values, 0) == 0
    assert ArrayWorkload.indexed_batch(values, 1) == 511_533
    assert ArrayWorkload.iterator_batch(values, 1) == 511_533
    changed = :array.set(0, 700, values)
    assert ArrayWorkload.indexed_batch(changed, 1) == 512_230
    assert ArrayWorkload.iterator_batch(changed, 1) == 512_230
    assert_raise ArgumentError, fn -> ArrayWorkload.indexed_batch(values, -1) end

    assert_raise ArgumentError, fn ->
      ArrayWorkload.indexed_batch(values, 9_223_372_036_854_775_808)
    end

    assert_raise ArgumentError, fn -> ArrayWorkload.indexed_batch(:not_an_array, 1) end
    assert_raise ArgumentError, fn -> ArrayWorkload.indexed_batch(:array.from_list([]), 1) end

    assert_raise ArgumentError, fn ->
      ArrayWorkload.iterator_batch(:array.from_list([9_223_372_036_854_775_807, 1]), 1)
    end

    extreme = :array.from_list([9_223_372_036_854_775_807, 1])
    assert_raise ArgumentError, fn -> ArrayWorkload.indexed_batch(extreme, 1) end
    underflow = :array.from_list([-9_223_372_036_854_775_808, -1])
    assert_raise ArgumentError, fn -> ArrayWorkload.indexed_batch(underflow, 1) end
    assert_raise ArgumentError, fn -> ArrayWorkload.iterator_batch(underflow, 1) end
    cancellation = :array.from_list([9_223_372_036_854_775_807, 1, -1])
    assert_raise ArgumentError, fn -> ArrayWorkload.indexed_batch(cancellation, 1) end
    assert_raise ArgumentError, fn -> ArrayWorkload.iterator_batch(cancellation, 1) end
    safe_one = :array.from_list([4_611_686_018_427_387_903])
    assert_raise ArgumentError, fn -> ArrayWorkload.indexed_batch(safe_one, 3) end
    assert_raise ArgumentError, fn -> ArrayWorkload.iterator_batch(safe_one, 3) end
  end

  test "fake clocks produce balanced reports and strict JSON replay" do
    counter = Agent.start_link(fn -> 0 end) |> elem(1)
    clock = fn -> Agent.get_and_update(counter, &{&1, &1 + 1}) end
    batch = fn n -> 42 * n end

    config = %{
      pairs: 3,
      confidence_percent: 95,
      minimum_factor: 1.05,
      seed: -9_223_372_036_854_775_808,
      target_ns: 1,
      max_batch: 1
    }

    assert {:ok, report} = Microbenchmark.collect(config, 42, batch, batch, clock)
    assert Enum.count(report.pairs, &(&1.order == "baseline-first")) == 2

    metadata = %{
      "workload" => "fake",
      "invocation" => [],
      "revision" => "abc",
      "runtime" => "Elixir",
      "os" => "test",
      "arch" => "test"
    }

    encoded = Microbenchmark.encode_report(Map.merge(report, metadata))
    assert {:ok, replay} = Microbenchmark.decode_report(encoded)
    assert replay.pairs == report.pairs

    max_i64 =
      String.replace(encoded, "\"seed\":-9223372036854775808", "\"seed\":9223372036854775807")

    assert {:ok, max_report} = Microbenchmark.decode_report(max_i64)
    assert max_report.config.seed == 9_223_372_036_854_775_807

    max_duration =
      encoded
      |> String.replace("\"baseline_ns\":1", "\"baseline_ns\":9223372036854775807")
      |> String.replace("\"candidate_ns\":1", "\"candidate_ns\":9223372036854775807")

    assert {:ok, duration_report} = Microbenchmark.decode_report(max_duration)
    assert hd(duration_report.pairs).baseline_ns == 9_223_372_036_854_775_807

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"seed\":-9223372036854775808", "\"seed\":-0")
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(
                 encoded,
                 "\"seed\":-9223372036854775808",
                 "\"seed\":9223372036854775808"
               )
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(
                 encoded,
                 "\"workload\":\"fake\"",
                 "\"workload\":\"fake\",\"workload\":\"evil\""
               )
             )
  end

  test "fixture observations classify all six cases" do
    fixture = :json.decode(File.read!("testdata/microbenchmark-pairs-v1.json"))

    for item <- fixture["cases"] do
      pairs =
        item["candidate_ns"]
        |> Enum.with_index()
        |> Enum.map(fn {candidate, index} ->
          %{
            baseline_ns: item["baseline_ns"],
            candidate_ns: candidate,
            order: if(rem(index, 2) == 0, do: "baseline-first", else: "candidate-first")
          }
        end)

      {:ok, result} =
        Microbenchmark.analyze(pairs, fixture["confidence_percent"], fixture["minimum_factor"])

      assert result.verdict == item["verdict"]
      assert result.ratio == item["ratio"]

      if item["interval"] in [nil, :null],
        do: assert(result.interval == nil),
        else: assert(result.interval == List.to_tuple(item["interval"]))
    end
  end

  test "validates malformed JSON, signed bounds, duplicate names, and schema fields" do
    assert {:error, :invalid_utf8} = Microbenchmark.decode_report(<<0xFF>>)

    assert {:error, :input_too_large} =
             Microbenchmark.decode_report(String.duplicate(" ", 4 * 1024 * 1024 + 1))

    assert {:error, :trailing_json} = Microbenchmark.decode_report("{} false")
    assert {:error, {:invalid_json, _}} = Microbenchmark.decode_report(<<0>>)
    assert {:error, :negative_zero} = Microbenchmark.decode_report("{\"value\":-0}")

    assert {:error, :json_bounds} =
             Microbenchmark.decode_report(
               String.duplicate("[", 129) <> "0" <> String.duplicate("]", 129)
             )

    at_value_limit = "[" <> Enum.join(List.duplicate("0", 65_535), ",") <> "]"
    assert {:error, :invalid_report} = Microbenchmark.decode_report(at_value_limit)

    over_value_limit = "[" <> Enum.join(List.duplicate("0", 65_536), ",") <> "]"
    assert {:error, :json_bounds} = Microbenchmark.decode_report(over_value_limit)

    assert {:error, _} = Microbenchmark.decode_report("{")
    assert {:error, _} = Microbenchmark.decode_report("[]")

    minimal =
      "{\"config\":1,\"batch_size\":1,\"pairs\":[],\"analysis\":[],\"workload\":\"x\",\"invocation\":[],\"revision\":\"x\",\"runtime\":\"x\",\"os\":\"x\",\"arch\":\"x\"}"

    assert {:error, :fields} = Microbenchmark.decode_report(minimal)

    valid_config =
      "{\"config\":{\"pairs\":1,\"confidence_percent\":95,\"minimum_factor\":1.05,\"seed\":0,\"target_ns\":1,\"max_batch\":1},\"batch_size\":1,\"pairs\":[{\"baseline_ns\":1,\"candidate_ns\":1,\"order\":\"baseline-first\"}],\"analysis\":[],\"workload\":\"x\",\"invocation\":[],\"revision\":\"x\",\"runtime\":\"x\",\"os\":\"x\",\"arch\":\"x\"}"

    assert {:error, :analysis_mismatch} = Microbenchmark.decode_report(valid_config)
    assert {:error, :invalid_arguments} = Microbenchmark.collect(%{}, 1, nil, nil, nil)
    config = Microbenchmark.defaults()

    assert {:error, {:callback_or_clock_exception, "boom"}} =
             Microbenchmark.collect(config, 1, fn _ -> raise "boom" end, fn n -> n end, fn ->
               1
             end)

    assert {:error, {:throw, :boom}} =
             Microbenchmark.collect(config, 1, fn _ -> throw(:boom) end, fn n -> n end, fn ->
               1
             end)

    assert {:error, :checksum_mismatch} =
             Microbenchmark.collect(
               %{config | pairs: 1, target_ns: 1, max_batch: 1},
               1,
               fn _ -> 0 end,
               fn n -> n end,
               fn -> 1 end
             )

    assert {:ok, _} =
             Microbenchmark.analyze(
               [%{baseline_ns: 2, candidate_ns: 1, order: "baseline-first"}],
               95,
               2
             )

    counter = Agent.start_link(fn -> 0 end) |> elem(1)
    clock = fn -> Agent.get_and_update(counter, &{&1, &1 + 1}) end

    assert {:error, :checksum_mismatch} =
             Microbenchmark.collect(
               %{config | pairs: 1, target_ns: 2, max_batch: 2},
               1,
               fn n -> if n == 1, do: 1, else: 0 end,
               fn n -> n end,
               clock
             )

    regression = Agent.start_link(fn -> 2 end) |> elem(1)
    regressing_clock = fn -> Agent.get_and_update(regression, &{&1, &1 - 1}) end

    assert {:error, :clock_regressed} =
             Microbenchmark.collect(
               %{config | pairs: 1, target_ns: 1, max_batch: 1},
               1,
               fn n -> n end,
               fn n -> n end,
               regressing_clock
             )

    between_pairs = Agent.start_link(fn -> {0, 0} end) |> elem(1)

    between_pair_regression = fn ->
      Agent.get_and_update(between_pairs, fn {calls, tick} ->
        value = if calls == 8, do: tick - 2, else: tick
        {value, {calls + 1, value + 1}}
      end)
    end

    assert {:error, :clock_regressed} =
             Microbenchmark.collect(
               %{config | pairs: 2, target_ns: 1, max_batch: 1},
               1,
               fn n -> n end,
               fn n -> n end,
               between_pair_regression
             )

    assert {:error, :empty_pairs} = Microbenchmark.analyze([], 95, 1.05)

    maximum_pairs =
      Enum.map(0..9_999, fn index ->
        %{
          baseline_ns: 1,
          candidate_ns: 1,
          order: if(rem(index, 2) == 0, do: "baseline-first", else: "candidate-first")
        }
      end)

    assert {:ok, %{ratio: 1.0}} = Microbenchmark.analyze(maximum_pairs, 95, 1.05)

    assert {:error, :too_many_pairs} =
             Microbenchmark.analyze(maximum_pairs ++ [%{}], :invalid, :invalid)

    assert {:error, :invalid_pair} =
             Microbenchmark.analyze([%{baseline_ns: 1, candidate_ns: 1, order: "bad"}], 95, 1.05)

    assert {:error, :unbalanced_order} =
             Microbenchmark.analyze(
               [
                 %{baseline_ns: 1, candidate_ns: 1, order: "baseline-first"},
                 %{baseline_ns: 1, candidate_ns: 1, order: "baseline-first"}
               ],
               95,
               1.05
             )

    extra_config = Map.put(Microbenchmark.defaults(), :unexpected, 1)

    assert {:error, :config_fields} =
             Microbenchmark.collect(extra_config, 1, fn n -> n end, fn n -> n end, fn -> 0 end)

    counter = Agent.start_link(fn -> 0 end) |> elem(1)
    clock = fn -> Agent.get_and_update(counter, &{&1, &1 + 1}) end

    assert {:error, :calibration_exhausted} =
             Microbenchmark.collect(
               %{config | pairs: 1, target_ns: 3, max_batch: 3},
               1,
               fn n -> n end,
               fn n -> n end,
               clock
             )
  end

  test "replay rejects duplicate decoded aliases and forged or partial reports" do
    counter = Agent.start_link(fn -> 0 end) |> elem(1)
    clock = fn -> Agent.get_and_update(counter, &{&1, &1 + 1}) end
    batch = fn n -> n end

    {:ok, report} =
      Microbenchmark.collect(
        %{
          pairs: 1,
          confidence_percent: 95,
          minimum_factor: 1.05,
          seed: 0,
          target_ns: 1,
          max_batch: 1
        },
        1,
        batch,
        batch,
        clock
      )

    metadata = %{
      "workload" => "fake",
      "invocation" => [],
      "revision" => "r",
      "runtime" => "Elixir",
      "os" => "test",
      "arch" => "test"
    }

    encoded = Microbenchmark.encode_report(Map.merge(report, metadata))
    encode_token = fn value -> value |> :json.encode() |> IO.iodata_to_binary() end

    with_workload = fn token ->
      String.replace(encoded, "\"workload\":\"fake\"", "\"workload\":" <> token, global: false)
    end

    accepted_workloads = [
      {encode_token.("indexed-array-sum-4096"), "indexed-array-sum-4096"},
      {encode_token.("dependent-link-order"), "dependent-link-order"},
      {encode_token.("workload [ { with escaped \" quote"), "workload [ { with escaped \" quote"},
      {~S("\u2003"), <<0xE2, 0x80, 0x83>>},
      {~S("\ud83d\udca1"), <<0xF0, 0x9F, 0x92, 0xA1>>},
      {~S("\ufffd"), <<0xEF, 0xBF, 0xBD>>},
      {~S("\\ud800"), "\\ud800"}
    ]

    for {token, expected} <- accepted_workloads do
      assert {:ok, replayed} = Microbenchmark.decode_report(with_workload.(token))
      assert replayed.metadata["workload"] == expected
    end

    invalid_workloads =
      ["", <<0>>, "bad" <> <<0>>] ++ [" ", <<9>>, <<10>>, <<11>>, <<12>>, <<13>>]

    for workload <- invalid_workloads do
      assert {:error, _} = Microbenchmark.decode_report(with_workload.(encode_token.(workload)))
    end

    for token <- ["null", "17", "true", "[]", "{}", ~S("\ud800"), ~S("\udfff")] do
      assert {:error, _} = Microbenchmark.decode_report(with_workload.(token))
    end

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(
                 encoded,
                 "\"workload\":\"fake\"",
                 "\"workload\":\"fake\",\"\\u0077orkload\":\"evil\""
               )
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"ratio\":1.0", "\"ratio\":0.5")
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"config\":{", "\"config\":{\"surplus\":0,")
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"config\":{", "\"config\":\"bad\",")
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"analysis\":{", "\"analysis\":[]")
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(
                 encoded,
                 "\"workload\":\"fake\"",
                 "\"workload\":\"fake\",\"profile\":1"
               )
             )

    assert {:ok, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"invocation\":[]", "\"invocation\":[\"\"]")
             )

    assert {:ok, _} = Microbenchmark.decode_report(Microbenchmark.encode_report(report))

    assert {:ok, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"revision\":\"r\"", "\"revision\":\" \"", global: false)
             )

    for {field, value} <- [
          {"revision", "r"},
          {"runtime", "Elixir"},
          {"os", "test"},
          {"arch", "test"}
        ] do
      nul =
        String.replace(encoded, "\"#{field}\":\"#{value}\"", "\"#{field}\":\"\\u0000\"",
          global: false
        )

      assert {:error, _} = Microbenchmark.decode_report(nul)
    end

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(
                 encoded,
                 "\"workload\":\"fake\"",
                 "\"workload\":\"fake\",\"profile\":\"\\u0000\"",
                 global: false
               )
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(
                 encoded,
                 "\"workload\":\"fake\"",
                 "\"workload\":\"fake\",\"profile\":\"\"",
                 global: false
               )
             )

    assert {:error, _} =
             Microbenchmark.decode_report(
               String.replace(encoded, "\"invocation\":[]", "\"invocation\":[\"\\u0000\"]",
                 global: false
               )
             )
  end

  test "native command emits 40 positive measured pairs that replay" do
    output = ExUnit.CaptureIO.capture_io(fn -> CodingAdventures.ArrayMicrobenchmark.main([]) end)
    assert {:ok, report} = Microbenchmark.decode_report(String.trim(output))
    assert length(report.pairs) == 40
    assert Enum.all?(report.pairs, &(&1.baseline_ns > 0 and &1.candidate_ns > 0))
  end
end
