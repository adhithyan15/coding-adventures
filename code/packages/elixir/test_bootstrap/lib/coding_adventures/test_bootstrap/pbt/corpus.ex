defmodule CodingAdventures.TestBootstrap.Pbt.Corpus do
  @moduledoc """
  The production runner for the shared pbt-v1 corpus
  (`code/specs/fixtures/pbt-v1/cases.json`, VER05 section 6).

  It takes the raw JSON, strictly decodes it, evaluates every case with THIS
  package's own public engine functions and returns human-readable problems.  An
  empty list means "conforms".  It does no file I/O (the package is
  capability-free); the test that reads the committed fixture lives in the test
  tree.

  Why a runner that can fail matters: a conformance check that cannot report a
  problem is worse than none.  So the runner is strict:

    * a case must have exactly the fields of its op, with canonical encodings;
    * an unknown op, unknown property name or malformed value is a PROBLEM,
      never a silent skip;
    * every op and all four failure ids must appear, an unregistered id must be
      rejected, and the number of cases must equal 25, so deleting cases is
      itself a problem;
    * every problem about a case starts with that case's id.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.{Prng, Safe, Tfv}
  alias CodingAdventures.TestBootstrap.CorpusJson.{Canon, Clip, StrictJson, Values}
  alias CodingAdventures.TestBootstrap.Pbt.{FailureIds, Gens, Options, Runner, Shrink}
  alias CodingAdventures.TestBootstrap.CorpusJson.Values.CaseError

  @case_count 25
  @max_cases 10_000
  # Bounds the total bytes one gen case may generate and hold (CWE-400).
  @max_gen_bytes 1 <<< 20

  def case_count, do: @case_count

  @doc """
  Strictly decodes a pbt-v1 document into its case list: exactly
  `{"schema_version": 1, "cases": [...]}`.  Cases are returned undecoded.
  """
  def parse(data) do
    root = StrictJson.parse(data)

    unless is_map(root) and map_size(root) == 2 do
      raise StrictJson.JsonError, message: "pbt corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}"
    end

    unless root["schema_version"] == {:number, "1"},
      do: raise(StrictJson.JsonError, message: "pbt corpus: schema_version must be 1")

    unless is_list(root["cases"]),
      do: raise(StrictJson.JsonError, message: "pbt corpus: \"cases\" must be an array")

    root["cases"]
  end

  # Predicates named by the corpus.  `true` means "holds".
  @int_props ~w(int_le_100 int_in_range_neg10_50 int_is_even int_abs_le_10 refinement_x_le_50)
  @list_props ~w(list_no_consecutive_duplicates list_sum_le_20)
  @bytes_props ~w(bytes_len_lt_5 bytes_no_zero_byte)

  defp property("int_le_100"), do: fn x -> x <= 100 end
  defp property("int_in_range_neg10_50"), do: fn x -> x >= -10 and x <= 50 end
  defp property("int_is_even"), do: fn x -> rem(x, 2) == 0 end
  defp property("int_abs_le_10"), do: fn x -> abs(x) <= 10 end
  defp property("refinement_x_le_50"), do: fn x -> x <= 50 end

  defp property("list_no_consecutive_duplicates"),
    do: fn l -> l |> Enum.chunk_every(2, 1, :discard) |> Enum.all?(fn [a, b] -> a != b end) end

  defp property("list_sum_le_20"), do: fn l -> Enum.sum(l) <= 20 end
  defp property("bytes_len_lt_5"), do: fn b -> byte_size(b) < 5 end
  defp property("bytes_no_zero_byte"), do: fn b -> not (0 in :binary.bin_to_list(b)) end

  # The closed field set of each op (besides id and op).
  defp shapes do
    %{
      "gen_primitive" => ~w(type seed count outputs),
      "gen_bounded" => ~w(min max seed count outputs),
      "gen_bytes" => ~w(min_len max_len seed count outputs),
      "shrink_int" => ~w(initial target min max property expected_minimal expected_steps),
      "shrink_refinement" => ~w(initial target min max property expected_minimal expected_steps),
      "shrink_list" => ~w(initial elem_target property expected_minimal expected_steps),
      "shrink_bytes" => ~w(initial property expected_minimal expected_steps),
      "replay_consistency" => ~w(seed property initial_fail expected_minimal),
      "filter_exhausted" => ~w(max_retries verdict failure_id),
      "failure_id_valid" => ~w(input valid),
      "corpus_reject" => ~w(text)
    }
  end

  @doc """
  Evaluates every case; returns `%{evaluated: n, problems: [...]}`.  A nil
  `expected` skips the case-count check (ad-hoc case lists in tests).
  """
  def evaluate(cases_in, expected \\ @case_count) do
    problems = if expected && length(cases_in) != expected, do: ["pbt-v1: expected #{expected} cases, got #{length(cases_in)}"], else: []

    {cases, problems} =
      if length(cases_in) > @max_cases,
        do: {Enum.take(cases_in, @max_cases), problems ++ ["pbt-v1: #{length(cases_in)} cases exceeds the cap of #{@max_cases}"]},
        else: {cases_in, problems}

    problems = if ids_ok?(cases), do: problems, else: problems ++ ["duplicate or missing case id"]

    init = %{problems: Enum.reverse(problems), evaluated: 0, ops: %{}, registered: %{}, rejected_id: false}

    acc = Enum.reduce(cases, init, &evaluate_one/2)

    missing_ops = Enum.sort(Map.keys(shapes()) -- Map.keys(acc.ops))
    missing_ids = FailureIds.all() -- Map.keys(acc.registered)

    extra =
      (if missing_ops == [], do: [], else: ["pbt-v1: operations never exercised: #{inspect(missing_ops)}"]) ++
        (if missing_ids == [], do: [], else: ["pbt-v1: registered failure ids missing: #{inspect(missing_ids)}"]) ++
        (if acc.rejected_id, do: [], else: ["pbt-v1: failure_id_valid needs an unregistered-id rejection"])

    all = Enum.reverse(acc.problems) ++ extra
    %{evaluated: acc.evaluated, problems: Enum.map(all, &Clip.clip(&1, Values.max_problem_len()))}
  end

  defp ids_ok?(cases) do
    ids = Enum.map(cases, fn raw -> if is_map(raw) and is_binary(raw["id"]), do: raw["id"] end)
    Enum.all?(ids, &(&1 not in [nil, ""])) and length(Enum.uniq(ids)) == length(ids)
  end

  defp evaluate_one(raw, acc) do
    cond do
      not is_map(raw) ->
        add(acc, "case is not an object: #{Values.show(raw)}")

      not (is_binary(raw["id"]) and raw["id"] != "") ->
        add(acc, "case missing a string id: #{Values.show(raw)}")

      true ->
        cid = Canon.escape(Clip.clip(raw["id"], 100))
        op = if is_binary(raw["op"]), do: raw["op"], else: ""

        case Map.fetch(shapes(), op) do
          :error ->
            add(acc, "#{cid}: unknown op #{Values.show(raw["op"])}")

          {:ok, fields} ->
            want = Enum.sort(["id", "op"] ++ fields)

            if Enum.sort(Map.keys(raw)) != want do
              add(acc, "#{cid}: fields #{Values.show(Enum.map(Enum.sort(Map.keys(raw)), &{:text, &1}))} != #{inspect(want)}")
            else
              run_case(cid, op, raw, acc)
            end
        end
    end
  end

  defp add(acc, problem), do: %{acc | problems: [problem | acc.problems]}

  defp run_case(cid, op, raw, acc) do
    ev = evaluate_case(cid, op, raw)
    acc = %{acc | problems: Enum.reverse(ev.problems) ++ acc.problems}

    if ev.ran_ok do
      acc =
        cond do
          op != "failure_id_valid" -> acc
          ev.registered -> %{acc | registered: Map.put(acc.registered, ev.candidate, true)}
          true -> %{acc | rejected_id: true}
        end

      %{acc | ops: Map.put(acc.ops, op, true), evaluated: acc.evaluated + 1}
    else
      acc
    end
  end

  # Decodes and checks one case.  Any exception, throw or exit (a malformed case
  # or an engine that failed) is reported as a problem.
  defp evaluate_case(cid, op, raw) do
    case Safe.call(fn -> do_case(cid, op, raw) end) do
      {:ok, result} ->
        result

      {:raised, kind, reason} ->
        %{
          problems: ["#{cid}: #{op} could not be evaluated: #{Clip.clip(describe(kind, reason), Values.max_fragment_len())}"],
          ran_ok: false,
          candidate: "",
          registered: false
        }
    end
  end

  defp describe(:error, %{__exception__: true, message: m}) when is_binary(m), do: Safe.sanitize(m)
  defp describe(kind, reason), do: Safe.local(kind, reason)

  defp do_case(cid, op, raw) do
    v = raw |> Map.drop(["id", "op"]) |> Map.new(fn {k, val} -> {k, Values.decode(val)} end)
    base = %{problems: [], ran_ok: true, candidate: "", registered: false}

    case op do
      o when o in ["gen_primitive", "gen_bounded", "gen_bytes"] ->
        %{base | problems: check_gen(cid, o, v)}

      o when o in ["shrink_int", "shrink_refinement"] ->
        %{base | problems: check_shrink_int(cid, o, v)}

      "shrink_list" ->
        %{base | problems: check_shrink_list(cid, v)}

      "shrink_bytes" ->
        %{base | problems: check_shrink_bytes(cid, v)}

      "replay_consistency" ->
        %{base | problems: check_replay(cid, v)}

      "filter_exhausted" ->
        %{base | problems: check_filter_exhausted(cid, v)}

      "failure_id_valid" ->
        input = case v["input"], do: ({:text, s} -> s; _ -> nil)
        got = input != nil and FailureIds.valid?(input)

        problems =
          if Values.same?(got, v["valid"]),
            do: [],
            else: ["#{cid}: failure id membership disagrees for #{Values.show(v["input"])}"]

        %{base | problems: problems, candidate: input || "", registered: got}

      "corpus_reject" ->
        text = case v["text"], do: ({:text, s} -> s; _ -> case_error("text must be a string"))
        problems = if Tfv.valid?(text), do: ["#{cid}: corpus_reject text was accepted by the strict parser"], else: []
        %{base | problems: problems}
    end
  end

  defp case_error(message), do: raise(CaseError, message: message)

  defp int_of(value, _what) when is_integer(value), do: value
  defp int_of(_value, what), do: case_error("#{what} must be an int")

  # An int that drives a loop or an allocation: 0 <= n <= max_size.
  defp size_of(value, what) do
    n = int_of(value, what)

    unless n >= 0 and n <= Values.max_size(),
      do: case_error("#{what} #{n} outside [0, #{Values.max_size()}]")

    n
  end

  defp text_of({:text, s}, _what), do: s
  defp text_of(_, what), do: case_error("#{what} must be a string")

  defp prop_of(name, allowed, op) do
    case name do
      {:text, s} ->
        if s in allowed, do: property(s), else: case_error("unknown property #{Values.show(name)} for #{op}")

      _ ->
        case_error("unknown property #{Values.show(name)} for #{op}")
    end
  end

  defp check_gen(cid, op, v) do
    seed = band(int_of(v["seed"], "seed"), Prng.mask64())
    count = size_of(v["count"], "count")

    # The expected outputs are in hand already: a case whose list length is not
    # count cannot conform, so refuse it before generating anything.
    unless is_list(v["outputs"]) and length(v["outputs"]) == count,
      do: case_error("outputs must be a list of exactly count (#{count}) values")

    got =
      case op do
        "gen_primitive" ->
          Gens.gen_primitive(seed, text_of(v["type"], "type"), count)

        "gen_bounded" ->
          Gens.gen_bounded(seed, int_of(v["min"], "min"), int_of(v["max"], "max"), count)

        _ ->
          min_len = size_of(v["min_len"], "min_len")
          max_len = size_of(v["max_len"], "max_len")

          if count * max_len > @max_gen_bytes,
            do: case_error("count #{count} x max_len #{max_len} exceeds the generation budget of #{@max_gen_bytes} bytes")

          Gens.gen_bytes(seed, min_len, max_len, count)
      end

    if Values.same?(got, v["outputs"]), do: [], else: ["#{cid}: #{op} outputs mismatch"]
  end

  defp mismatch(cid, op, what, got, corpus),
    do: "#{cid}: #{op} #{what} mismatch: got #{Values.show(got)}, corpus #{Values.show(corpus)}"

  defp check_shrink_int(cid, op, v) do
    prop = prop_of(v["property"], @int_props, op)
    initial = int_of(v["initial"], "initial")
    target = int_of(v["target"], "target")
    lo = int_of(v["min"], "min")
    hi = int_of(v["max"], "max")
    {minimal, steps} = Shrink.shrink_int(initial, target, lo, hi, 1000, prop)

    []
    |> add_if(not Values.same?(minimal, v["expected_minimal"]), mismatch(cid, op, "minimal", minimal, v["expected_minimal"]))
    |> add_if(not Values.same?(steps, v["expected_steps"]), "#{cid}: #{op} step count mismatch: got #{steps}, corpus #{Values.show(v["expected_steps"])}")
    # Engine invariants, independent of the corpus's expected values.
    |> add_if(prop.(minimal) == true, "#{cid}: #{op} minimal #{Values.show(minimal)} does not falsify the property")
    |> add_if(not (minimal >= lo and minimal <= hi), "#{cid}: #{op} minimal #{Values.show(minimal)} left [#{lo}, #{hi}]")
    |> Enum.reverse()
  end

  defp add_if(list, true, problem), do: [problem | list]
  defp add_if(list, false, _problem), do: list

  defp check_shrink_list(cid, v) do
    prop = prop_of(v["property"], @list_props, "shrink_list")
    unless is_list(v["initial"]), do: case_error("initial must be a list of ints")
    initial = Enum.map(v["initial"], &int_of(&1, "element"))
    elem_target = int_of(v["elem_target"], "elem_target")
    {minimal, steps} = Shrink.shrink_list(initial, elem_target, 1000, prop)

    []
    |> add_if(not Values.same?(minimal, v["expected_minimal"]), mismatch(cid, "shrink_list", "minimal", minimal, v["expected_minimal"]))
    |> add_if(not Values.same?(steps, v["expected_steps"]), "#{cid}: shrink_list steps mismatch: got #{steps}, corpus #{Values.show(v["expected_steps"])}")
    |> add_if(prop.(minimal) == true, "#{cid}: shrink_list minimal #{Values.show(minimal)} does not falsify the property")
    |> Enum.reverse()
  end

  defp check_shrink_bytes(cid, v) do
    prop = prop_of(v["property"], @bytes_props, "shrink_bytes")
    unless is_binary(v["initial"]), do: case_error("initial must be bytes")
    {minimal, steps} = Shrink.shrink_bytes(v["initial"], 1000, prop)

    []
    |> add_if(not Values.same?(minimal, v["expected_minimal"]), mismatch(cid, "shrink_bytes", "minimal", minimal, v["expected_minimal"]))
    |> add_if(not Values.same?(steps, v["expected_steps"]), "#{cid}: shrink_bytes steps mismatch: got #{steps}, corpus #{Values.show(v["expected_steps"])}")
    |> add_if(prop.(minimal) == true, "#{cid}: shrink_bytes minimal #{Values.show(minimal)} does not falsify the property")
    |> Enum.reverse()
  end

  defp check_replay(cid, v) do
    seed = band(int_of(v["seed"], "seed"), Prng.mask64())
    prop = prop_of(v["property"], @int_props, "replay_consistency")
    {initial_fail, minimal} = Shrink.simulate_replay(seed, prop)

    []
    |> add_if(not Values.same?(initial_fail, v["initial_fail"]), mismatch(cid, "replay", "initial_fail", initial_fail, v["initial_fail"]))
    |> add_if(not Values.same?(minimal, v["expected_minimal"]), mismatch(cid, "replay", "minimal", minimal, v["expected_minimal"]))
    |> Enum.reverse()
  end

  defp check_filter_exhausted(cid, v) do
    max_retries = size_of(v["max_retries"], "max_retries")
    if max_retries < 1, do: case_error("max_retries must be at least 1")

    calls = :counters.new(1, [])

    gen =
      Gens.filter(Gens.integer(0, 10), max_retries, fn _x ->
        :counters.add(calls, 1, 1)
        false
      end)

    verdict = Runner.check(0, gen, Options.new(num_tests: 1, max_shrink_steps: 1000, ci: :on), fn _ -> true end)
    n = :counters.get(calls, 1)

    []
    |> add_if(v["verdict"] != {:text, "fail"} or verdict.passed, "#{cid}: an unsatisfiable filter must be a fail verdict")
    |> add_if(not Values.same?(wrap(verdict.failure_id), v["failure_id"]), "#{cid}: failure id #{inspect(verdict.failure_id)}, corpus #{Values.show(v["failure_id"])}")
    |> add_if(n != max_retries, "#{cid}: filter sampled #{n} times, expected #{max_retries}")
    |> Enum.reverse()
  end

  defp wrap(nil), do: nil
  defp wrap(s), do: {:text, s}
end
