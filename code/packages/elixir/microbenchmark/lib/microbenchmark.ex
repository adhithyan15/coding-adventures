defmodule CodingAdventures.Microbenchmark do
  import Bitwise

  @moduledoc """
  Paired batch collection, analysis, exact JSON replay, and signed-64 checks.
  Failures return `{:error, reason}` and never expose a partial report.
  """
  alias CodingAdventures.RankStatistics, as: Stats
  @min_i64 -(1 <<< 63)
  @max_i64 (1 <<< 63) - 1
  @max_pairs 10_000
  def defaults,
    do: %{
      pairs: 40,
      confidence_percent: 95,
      minimum_factor: 1.05,
      seed: 0,
      target_ns: 100_000,
      max_batch: 65_536
    }

  def analyze(pairs, confidence, factor) when is_list(pairs) do
    with :ok <- validate_pair_count(pairs),
         :ok <- validate_analysis_parameters(confidence, factor),
         :ok <- validate_pairs(pairs),
         :ok <- validate_balance(pairs),
         ratios <- Enum.map(pairs, fn p -> p.candidate_ns / p.baseline_ns end),
         {:ok, ratio} <- Stats.median(ratios),
         {:ok, interval} <- Stats.median_interval(ratios, confidence) do
      verdict = verdict(interval, factor)
      {:ok, %{ratio: ratio, interval: interval, verdict: verdict}}
    end
  end

  def analyze(_, _, _), do: {:error, :empty_pairs}

  def collect(config, expected_checksum, baseline, candidate, clock)
      when is_map(config) and is_function(baseline, 1) and is_function(candidate, 1) and
             is_function(clock, 0) do
    try do
      with :ok <- validate_config(config),
           :ok <- verify(baseline, expected_checksum, 1),
           :ok <- verify(candidate, expected_checksum, 1),
           {:ok, size, clock_state} <-
             calibrate(config, expected_checksum, baseline, candidate, clock),
           {:ok, pairs, _state} <-
             measured(config, expected_checksum, size, baseline, candidate, clock, clock_state),
           {:ok, analysis} <- analyze(pairs, config.confidence_percent, config.minimum_factor) do
        {:ok, %{config: config, batch_size: size, pairs: pairs, analysis: analysis}}
      end
    rescue
      error -> {:error, {:callback_or_clock_exception, Exception.message(error)}}
    catch
      kind, reason -> {:error, {kind, reason}}
    end
  end

  def collect(_, _, _, _, _), do: {:error, :invalid_arguments}

  def encode_report(report), do: report |> json_ready() |> :json.encode() |> IO.iodata_to_binary()

  def decode_report(binary) when is_binary(binary) and byte_size(binary) <= 4 * 1024 * 1024 do
    case :unicode.characters_to_binary(binary, :utf8, :utf8) do
      ^binary -> decode_valid_utf8(binary)
      _ -> {:error, :invalid_utf8}
    end
  end

  def decode_report(_), do: {:error, :input_too_large}

  defp decode_valid_utf8(binary) do
    with :ok <- preflight_json(binary) do
      push = fn key, value, object ->
        if Map.has_key?(object, key), do: throw({:duplicate_json_key, key})
        Map.put(object, key, value)
      end

      decoders = %{
        object_start: fn _ -> %{} end,
        object_push: push,
        object_finish: fn object, old -> {object, old} end
      }

      try do
        case :json.decode(binary, nil, decoders) do
          {decoded, nil, <<>>} -> validate_report(decoded)
          _ -> {:error, :trailing_json}
        end
      rescue
        error -> {:error, {:invalid_json, Exception.message(error)}}
      catch
        :throw, reason -> {:error, reason}
      end
    end
  end

  defp validate_report(report) when is_map(report) do
    required = ~w(config batch_size pairs analysis)
    allowed = required ++ ~w(workload invocation revision runtime os arch profile)

    if Enum.any?(Map.keys(report), &(not is_binary(&1) or &1 not in allowed)) do
      {:error, :unknown_field}
    else
      with true <- Enum.all?(required, &Map.has_key?(report, &1)) || {:error, :missing_field},
           :ok <-
             exact_fields(
               report["config"],
               ~w(pairs confidence_percent minimum_factor seed target_ns max_batch)
             ),
           config <- normalize_config(report["config"]),
           :ok <- validate_config(config),
           true <-
             (is_list(report["pairs"]) and length(report["pairs"]) == config.pairs) ||
               {:error, :pair_count},
           true <-
             Enum.all?(
               report["pairs"],
               &(is_map(&1) and map_size(&1) == 3 and
                   Enum.all?(~w(baseline_ns candidate_ns order), fn key ->
                     Map.has_key?(&1, key)
                   end))
             ) || {:error, :pair_fields},
           pairs <- Enum.map(report["pairs"], &normalize_pair/1),
           {:ok, analysis} <- analyze(pairs, config.confidence_percent, config.minimum_factor),
           :ok <- compare_analysis(report["analysis"], analysis),
           :ok <- validate_metadata(report),
           :ok <- exact_fields(report["analysis"], ~w(ratio interval verdict)),
           batch_size <- report["batch_size"],
           true <-
             (i64?(batch_size) and batch_size > 0 and batch_size <= config.max_batch) ||
               {:error, :batch_size} do
        {:ok,
         %{
           config: config,
           batch_size: batch_size,
           pairs: pairs,
           analysis: analysis,
           metadata: Map.take(report, allowed -- ["config", "batch_size", "pairs", "analysis"])
         }}
      else
        {:error, _} = error -> error
      end
    end
  rescue
    _ -> {:error, :invalid_report}
  end

  defp validate_report(_), do: {:error, :invalid_report}

  defp exact_fields(value, fields) when is_map(value),
    do:
      if(map_size(value) == length(fields) and Enum.all?(fields, &Map.has_key?(value, &1)),
        do: :ok,
        else: {:error, :fields}
      )

  defp exact_fields(_, _), do: {:error, :fields}

  defp preflight_json(binary), do: scan_json_values(binary, 0, 0, false)

  # Count semantic JSON values before :json.decode allocates a parsed tree.
  # Object keys are strings too, but do not consume the value-node budget.
  defp scan_json_values(<<>>, 0, count, negative_zero) do
    cond do
      negative_zero -> {:error, :negative_zero}
      count > 65_536 -> {:error, :json_bounds}
      true -> :ok
    end
  end

  defp scan_json_values(<<>>, _depth, _count, _negative_zero), do: {:error, :json_bounds}

  defp scan_json_values(<<byte, rest::binary>>, depth, count, negative_zero)
       when byte in [9, 10, 13, 32, 44, 58],
       do: scan_json_values(rest, depth, count, negative_zero)

  defp scan_json_values(<<34, rest::binary>>, depth, count, negative_zero) do
    case scan_json_string(rest) do
      {:ok, remaining} ->
        after_string = skip_json_whitespace(remaining)
        value_count = if match?(<<58, _::binary>>, after_string), do: count, else: count + 1
        scan_json_values(after_string, depth, value_count, negative_zero)

      :incomplete ->
        :ok
    end
  end

  defp scan_json_values(<<opener, rest::binary>>, depth, count, negative_zero)
       when opener in [123, 91] do
    next_depth = depth + 1
    next_count = count + 1

    if next_depth > 128 or next_count > 65_536,
      do: {:error, :json_bounds},
      else: scan_json_values(rest, next_depth, next_count, negative_zero)
  end

  defp scan_json_values(<<closer, rest::binary>>, depth, count, negative_zero)
       when closer in [125, 93] do
    if depth == 0,
      do: {:error, :json_bounds},
      else: scan_json_values(rest, depth - 1, count, negative_zero)
  end

  defp scan_json_values(binary, depth, count, negative_zero) do
    {token, rest} = take_json_scalar(binary, [])
    next_count = count + 1

    if next_count > 65_536,
      do: {:error, :json_bounds},
      else: scan_json_values(rest, depth, next_count, negative_zero or token == "-0")
  end

  defp scan_json_string(<<>>), do: :incomplete
  defp scan_json_string(<<92, _escaped, rest::binary>>), do: scan_json_string(rest)
  defp scan_json_string(<<34, rest::binary>>), do: {:ok, rest}
  defp scan_json_string(<<_byte, rest::binary>>), do: scan_json_string(rest)

  defp skip_json_whitespace(<<byte, rest::binary>>) when byte in [9, 10, 13, 32],
    do: skip_json_whitespace(rest)

  defp skip_json_whitespace(binary), do: binary

  defp take_json_scalar(<<byte, _::binary>> = binary, acc)
       when byte in [9, 10, 13, 32, 44, 58, 123, 125, 91, 93, 34],
       do: {acc |> Enum.reverse() |> :erlang.list_to_binary(), binary}

  defp take_json_scalar(<<byte, rest::binary>>, acc), do: take_json_scalar(rest, [byte | acc])

  defp take_json_scalar(<<>>, acc),
    do: {acc |> Enum.reverse() |> :erlang.list_to_binary(), <<>>}

  defp normalize_config(c) when is_map(c) do
    %{
      pairs: c["pairs"],
      confidence_percent: c["confidence_percent"],
      minimum_factor: c["minimum_factor"],
      seed: c["seed"],
      target_ns: c["target_ns"],
      max_batch: c["max_batch"]
    }
  end

  defp normalize_pair(p) when is_map(p),
    do: %{baseline_ns: p["baseline_ns"], candidate_ns: p["candidate_ns"], order: p["order"]}

  defp compare_analysis(actual, expected) when is_map(actual) do
    wanted_interval =
      if expected.interval == nil, do: :null, else: Tuple.to_list(expected.interval)

    if map_size(actual) == 3 and actual["ratio"] == expected.ratio and
         actual["verdict"] == expected.verdict and actual["interval"] == wanted_interval,
       do: :ok,
       else: {:error, :analysis_mismatch}
  end

  defp compare_analysis(_, _), do: {:error, :analysis_mismatch}

  defp validate_metadata(m) do
    keys = ~w(workload invocation revision runtime os arch)
    present = Enum.filter(keys, &Map.has_key?(m, &1))
    has_profile = Map.has_key?(m, "profile")

    cond do
      present == [] and not has_profile ->
        :ok

      length(present) != length(keys) ->
        {:error, :metadata_group}

      not valid_workload?(m["workload"]) ->
        {:error, :metadata}

      not Enum.all?(~w(revision runtime os arch), &valid_nonempty_string?(m[&1])) ->
        {:error, :metadata}

      not is_list(m["invocation"]) or
          not Enum.all?(m["invocation"], &(is_binary(&1) and not String.contains?(&1, <<0>>))) ->
        {:error, :metadata}

      has_profile and not valid_nonempty_string?(m["profile"]) ->
        {:error, :metadata}

      true ->
        :ok
    end
  end

  defp valid_nonempty_string?(value),
    do: is_binary(value) and value != "" and not String.contains?(value, <<0>>)

  defp valid_workload?(value) when is_binary(value) and byte_size(value) > 0 do
    not String.contains?(value, <<0>>) and
      not Enum.all?(String.to_charlist(value), fn codepoint ->
        codepoint == 0x20 or codepoint in 0x09..0x0D
      end)
  end

  defp valid_workload?(_), do: false

  defp validate_config(c) do
    with :ok <-
           exact_atom_fields(c, [
             :pairs,
             :confidence_percent,
             :minimum_factor,
             :seed,
             :target_ns,
             :max_batch
           ]) do
      cond do
        not is_integer(c[:pairs]) or c.pairs not in 1..10_000 ->
          {:error, :pairs}

        not is_integer(c[:confidence_percent]) or c.confidence_percent not in 1..99 ->
          {:error, :confidence}

        not is_number(c[:minimum_factor]) or not finite?(c.minimum_factor) or
            c.minimum_factor <= 1 ->
          {:error, :factor}

        not Enum.all?([c[:seed], c[:target_ns], c[:max_batch]], &i64?/1) ->
          {:error, :signed_64}

        c.target_ns <= 0 or c.max_batch <= 0 ->
          {:error, :positive_limits}

        true ->
          :ok
      end
    end
  end

  defp exact_atom_fields(value, keys) when is_map(value),
    do:
      if(map_size(value) == length(keys) and Enum.all?(keys, &Map.has_key?(value, &1)),
        do: :ok,
        else: {:error, :config_fields}
      )

  defp validate_analysis_parameters(c, f),
    do:
      if(is_integer(c) and c in 1..99 and is_number(f) and finite?(f) and f > 1,
        do: :ok,
        else: {:error, :analysis_parameters}
      )

  defp validate_pair_count([]), do: {:error, :empty_pairs}

  defp validate_pair_count(pairs) do
    if at_most_pair_limit?(pairs, @max_pairs),
      do: :ok,
      else: {:error, :too_many_pairs}
  end

  defp at_most_pair_limit?([], _remaining), do: true
  defp at_most_pair_limit?([_ | _], 0), do: false

  defp at_most_pair_limit?([_ | rest], remaining),
    do: at_most_pair_limit?(rest, remaining - 1)

  defp validate_pairs(pairs) do
    if Enum.all?(pairs, fn p ->
         is_map(p) and i64?(p[:baseline_ns]) and i64?(p[:candidate_ns]) and p.baseline_ns > 0 and
           p.candidate_ns > 0 and p[:order] in ["baseline-first", "candidate-first"]
       end),
       do: :ok,
       else: {:error, :invalid_pair}
  end

  defp validate_balance(pairs) do
    count = Enum.count(pairs, &(&1.order == "baseline-first"))
    if count == div(length(pairs) + 1, 2), do: :ok, else: {:error, :unbalanced_order}
  end

  defp verdict(nil, _factor), do: "inconclusive"

  defp verdict({lo, hi}, factor) do
    cond do
      hi < 1 / factor -> "faster"
      lo > factor -> "slower"
      lo >= 1 / factor and hi <= factor -> "equivalent"
      true -> "inconclusive"
    end
  end

  defp i64?(n), do: is_integer(n) and n >= @min_i64 and n <= @max_i64
  defp finite?(n) when is_integer(n), do: true
  defp finite?(n) when is_float(n), do: n == n and abs(n) <= 1.7976931348623157e308
  defp json_ready(nil), do: :null

  defp json_ready(value) when is_map(value) do
    Map.new(value, fn {key, item} ->
      {if(is_atom(key), do: Atom.to_string(key), else: key), json_ready(item)}
    end)
  end

  defp json_ready(value) when is_list(value), do: Enum.map(value, &json_ready/1)

  defp json_ready(value) when is_tuple(value),
    do: value |> Tuple.to_list() |> Enum.map(&json_ready/1)

  defp json_ready(value), do: value

  defp verify(fun, expected, size) do
    with true <- i64?(expected) || {:error, :checksum_range},
         true <- i64?(expected * size) || {:error, :checksum_overflow},
         ^expected <- fun.(size) do
      :ok
    else
      {:error, _} = error -> error
      _ -> {:error, :checksum_mismatch}
    end
  end

  defp calibrate(c, expected, base, cand, clock),
    do: calibrate(c, expected, base, cand, clock, 1, nil)

  defp calibrate(c, expected, base, cand, clock, size, state) do
    with {:ok, b, state} <- measure(base, expected, size, clock, state, true),
         {:ok, a, state} <- measure(cand, expected, size, clock, state, true) do
      cond do
        b >= c.target_ns and a >= c.target_ns -> {:ok, size, state}
        size >= c.max_batch -> {:error, :calibration_exhausted}
        true -> calibrate(c, expected, base, cand, clock, min(size * 2, c.max_batch), state)
      end
    end
  end

  defp measure(fun, expected, size, clock, state, calibration) do
    with {:ok, start, state} <- read_clock(clock, state),
         result <- fun.(size),
         {:ok, stop, state} <- read_clock(clock, state),
         true <- result == expected * size || {:error, :checksum_mismatch},
         true <- i64?(result) || {:error, :checksum_overflow},
         elapsed <- stop - start,
         true <- (elapsed >= 0 and (calibration or elapsed > 0)) || {:error, :invalid_duration} do
      {:ok, elapsed, state}
    else
      {:error, _} = error -> error
    end
  end

  defp read_clock(clock, last) do
    value = clock.()

    if i64?(value) and value >= 0 and (last == nil or value >= last),
      do: {:ok, value, value},
      else: {:error, :clock_regressed}
  end

  defp measured(c, expected, size, base, cand, clock, state) do
    orders = Enum.map(0..(c.pairs - 1), &(rem(&1, 2) == 0)) |> shuffle(c.seed)

    Enum.reduce_while(orders, {:ok, [], state}, fn first, {:ok, pairs, old_state} ->
      {f1, f2, label} =
        if first, do: {base, cand, "baseline-first"}, else: {cand, base, "candidate-first"}

      with {:ok, d1, new_state} <- measure(f1, expected, size, clock, old_state, false),
           {:ok, d2, new_state} <- measure(f2, expected, size, clock, new_state, false) do
        pair =
          if first,
            do: %{baseline_ns: d1, candidate_ns: d2, order: label},
            else: %{baseline_ns: d2, candidate_ns: d1, order: label}

        {:cont, {:ok, [pair | pairs], new_state}}
      else
        error -> {:halt, error}
      end
    end)
    |> case do
      {:ok, pairs, new_state} -> {:ok, Enum.reverse(pairs), new_state}
      error -> error
    end
  end

  defp shuffle(values, seed) do
    s =
      :rand.seed_s(
        :exsss,
        {band(seed, 0x3FFFFFFF) + 1, band(seed >>> 30, 0x3FFFFFFF) + 1,
         band(seed >>> 60, 0x3FFFFFFF) + 1}
      )

    {shuffled, _state} =
      Enum.reduce((length(values) - 1)..1//-1, {values, s}, fn i, {items, state} ->
        {j0, next} = :rand.uniform_s(i + 1, state)
        j = j0 - 1
        a = Enum.at(items, i)
        b = Enum.at(items, j)
        {items |> List.replace_at(i, b) |> List.replace_at(j, a), next}
      end)

    shuffled
  end
end
