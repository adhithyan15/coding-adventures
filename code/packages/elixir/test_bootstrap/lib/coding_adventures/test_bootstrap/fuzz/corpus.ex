defmodule CodingAdventures.TestBootstrap.Fuzz.Corpus do
  @moduledoc """
  The production runner for the shared fuzz-v1 corpus
  (`code/specs/fixtures/fuzz-v1/cases.json`, VER06 section 8, VER06n section 8).

  It takes the raw JSON, strictly decodes it, evaluates every case with THIS
  package's own public engine functions (through `Fuzz.Hooks`, so a test can sabotage
  one) and returns human-readable problems.  An empty list means "conforms".  It
  does no file I/O; the tests read the committed fixture.

  A runner that cannot report a problem is worse than none, so this one is strict:

    * a case has exactly the fields of its op, in canonical encodings;
    * an unknown op or a malformed value is a PROBLEM, never a silent skip;
    * every op appears, a failure id is accepted and one rejected, and the case count
      is 25, so deleting cases is itself a problem;
    * fixture-controlled sizes are capped (CWE-400): a hostile `steps = 10^9` is a
      reported problem, not a hang, and the aggregate `mutate_sequence` work of the
      whole corpus is capped at 2^24 bytes ACTUALLY copied;
    * every problem about a case starts with that case's id, ESCAPED with `Canon`.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.Safe
  alias CodingAdventures.TestBootstrap.CorpusJson.{Canon, Clip, StrictJson, Values}
  alias CodingAdventures.TestBootstrap.CorpusJson.Values.CaseError
  alias CodingAdventures.TestBootstrap.Fuzz.{FailureIds, Hooks, Limits}

  @case_count 25
  @max_cases 10_000
  @two64 1 <<< 64

  def case_count, do: @case_count

  @doc "Strictly decodes a fuzz-v1 document into its case list: exactly `{\"schema_version\": 1, \"cases\": [...]}`."
  def parse(data) do
    root = StrictJson.parse(data)

    unless is_map(root) and map_size(root) == 2,
      do: raise(StrictJson.JsonError, message: "fuzz corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}")

    unless root["schema_version"] == {:number, "1"},
      do: raise(StrictJson.JsonError, message: "fuzz corpus: schema_version must be 1")

    unless is_list(root["cases"]),
      do: raise(StrictJson.JsonError, message: "fuzz corpus: \"cases\" must be an array")

    root["cases"]
  end

  # The closed field set of each op (besides id and op).
  defp shapes do
    %{
      "mutate_bit_flip" => ~w(input bit_offset count output),
      "mutate_byte_flip" => ~w(input byte_offset count output),
      "mutate_arithmetic" => ~w(input byte_offset delta width endian output),
      "mutate_boundary" => ~w(input byte_offset width val endian output),
      "mutate_block_delete" => ~w(input offset length output),
      "mutate_block_insert" => ~w(input offset insert_bytes output),
      "mutate_block_duplicate" => ~w(input src_offset length dst_offset output),
      "mutate_splice" => ~w(target donor target_split donor_split output),
      "mutate_sequence" => ~w(input seed steps output),
      "corpus_admit" => ~w(existing_edges trace_edges admitted new_edges),
      "crash_dedup" => ~w(crash_a_kind crash_a_frames crash_b_kind crash_b_frames is_duplicate),
      "failure_id_valid" => ~w(input valid),
      "private_reporting" => ~w(target failure_id raw_payload sig_hash tap_line contains_raw_payload),
      "corpus_reject" => ~w(text)
    }
  end

  @doc """
  Evaluates every case; returns `%{evaluated: n, problems: [...]}`.  A nil
  `expected` skips the case-count check (ad-hoc case lists in tests).
  """
  def evaluate(cases_in, expected \\ @case_count) do
    problems =
      if expected && length(cases_in) != expected,
        do: ["fuzz-v1: expected #{expected} cases, got #{length(cases_in)}"],
        else: []

    {cases, problems} =
      if length(cases_in) > @max_cases,
        do: {Enum.take(cases_in, @max_cases), problems ++ ["fuzz-v1: #{length(cases_in)} cases exceeds the cap of #{@max_cases}"]},
        else: {cases_in, problems}

    problems = if ids_ok?(cases), do: problems, else: problems ++ ["duplicate or missing case id"]

    # The aggregate work cell: ONE mutable cell for the whole corpus, as VER06n section 3 says.
    work = :atomics.new(1, signed: true)
    :atomics.put(work, 1, Limits.corpus_sequence_work())

    init = %{problems: Enum.reverse(problems), evaluated: 0, ops: %{}, accepted: false, rejected: false, work: work}
    acc = Enum.reduce(cases, init, &evaluate_one/2)

    missing_ops = Enum.sort(Map.keys(shapes()) -- Map.keys(acc.ops))
    unregistered = Enum.reject(FailureIds.all(), &Hooks.get(:failure_id).(&1))

    extra =
      (if missing_ops == [], do: [], else: ["fuzz-v1: operations never exercised: #{inspect(missing_ops)}"]) ++
        (if acc.accepted, do: [], else: ["fuzz-v1: failure_id_valid needs a registered-id acceptance"]) ++
        (if acc.rejected, do: [], else: ["fuzz-v1: failure_id_valid needs an unregistered-id rejection"]) ++
        (if unregistered == [], do: [], else: ["fuzz-v1: engine rejects registered ids #{inspect(unregistered)}"])

    all = Enum.reverse(acc.problems) ++ extra
    %{evaluated: acc.evaluated, problems: Enum.map(all, &Clip.clip(&1, Values.max_problem_len()))}
  end

  defp ids_ok?(cases) do
    ids = Enum.map(cases, fn raw -> if is_map(raw) and is_binary(raw["id"]), do: raw["id"] end)
    Enum.all?(ids, &(&1 not in [nil, ""])) and length(Enum.uniq(ids)) == length(ids)
  end

  defp add(acc, problem), do: %{acc | problems: [problem | acc.problems]}

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

  defp run_case(cid, op, raw, acc) do
    ev = evaluate_case(cid, op, raw, acc.work)
    acc = %{acc | problems: Enum.reverse(ev.problems) ++ acc.problems}

    if ev.ran_ok do
      acc =
        if op == "failure_id_valid",
          do: if(ev.accepted, do: %{acc | accepted: true}, else: %{acc | rejected: true}),
          else: acc

      %{acc | ops: Map.put(acc.ops, op, true), evaluated: acc.evaluated + 1}
    else
      acc
    end
  end

  # Decodes and checks one case.  Any raise, throw or exit (a malformed case or an
  # engine that failed) is reported as a problem.
  defp evaluate_case(cid, op, raw, work) do
    case Safe.call(fn -> do_case(cid, op, raw, work) end) do
      {:ok, result} ->
        result

      {:raised, kind, reason} ->
        %{
          problems: ["#{cid}: #{op} could not be evaluated: #{Clip.clip(describe(kind, reason), Values.max_fragment_len())}"],
          ran_ok: false,
          accepted: false
        }
    end
  end

  defp describe(:error, %{__exception__: true, message: m}) when is_binary(m), do: Safe.sanitize(m)
  defp describe(kind, reason), do: Safe.local(kind, reason)

  defp do_case(cid, op, raw, work) do
    v = raw |> Map.drop(["id", "op"]) |> Map.new(fn {k, val} -> {k, Values.decode(val)} end)
    base = %{problems: [], ran_ok: true, accepted: false}

    case op do
      "corpus_admit" ->
        %{base | problems: check_corpus_admit(cid, v)}

      "crash_dedup" ->
        %{base | problems: check_crash_dedup(cid, v)}

      "failure_id_valid" ->
        input = case v["input"], do: ({:text, s} -> s; _ -> nil)
        accepted = input != nil and Hooks.get(:failure_id).(input) == true

        problems =
          if Values.same?(accepted, v["valid"]),
            do: [],
            else: ["#{cid}: failure id membership disagrees for #{Values.show(v["input"])}"]

        %{base | problems: problems, accepted: accepted}

      "private_reporting" ->
        %{base | problems: check_private_reporting(cid, v)}

      "corpus_reject" ->
        text = case v["text"], do: ({:text, s} -> s; _ -> case_error("text must be a string"))

        problems =
          if Hooks.get(:tfv_valid).(text), do: ["#{cid}: corpus_reject text was accepted by the strict parser"], else: []

        %{base | problems: problems}

      _mutator ->
        %{base | problems: check_mutator(cid, op, v, work)}
    end
  end

  # -- typed field access ---------------------------------------------------------

  defp case_error(message), do: raise(CaseError, message: message)

  defp bytes_of(v, _what) when is_binary(v), do: v
  defp bytes_of(_v, what), do: case_error("#{what} must be bytes")

  defp text_of({:text, s}, _what), do: s
  defp text_of(_v, what), do: case_error("#{what} must be a string")

  defp bool_of(v, _what) when is_boolean(v), do: v
  defp bool_of(_v, what), do: case_error("#{what} must be a bool")

  defp int_of(v, _what) when is_integer(v), do: v
  defp int_of(_v, what), do: case_error("#{what} must be an int")

  # An int that drives a loop or an allocation: 0 <= n <= max_size.
  defp size_of(v, what) do
    n = int_of(v, what)
    unless n >= 0 and n <= Values.max_size(), do: case_error("#{what} #{n} outside [0, #{Values.max_size()}]")
    n
  end

  defp texts_of(v, what) do
    unless is_list(v) and Enum.all?(v, &match?({:text, _}, &1)),
      do: case_error("#{what} must be a list of strings")

    Enum.map(v, fn {:text, s} -> s end)
  end

  # A list of edge ids, each within [0, 2^64).
  defp edges_of(v, what) do
    unless is_list(v) and Enum.all?(v, &is_integer/1), do: case_error("#{what} must be a list of ints")

    Enum.each(v, fn e ->
      unless e >= 0 and e < @two64, do: case_error("#{what} element #{Values.show(e)} outside [0, 2^64)")
    end)

    v
  end

  # -- per-op checks --------------------------------------------------------------

  defp check_mutator(cid, op, v, work) do
    got =
      case op do
        "mutate_bit_flip" ->
          Hooks.get(:bit_flip).(bytes_of(v["input"], "input"), int_of(v["bit_offset"], "bit_offset"), int_of(v["count"], "count"))

        "mutate_byte_flip" ->
          Hooks.get(:byte_flip).(bytes_of(v["input"], "input"), int_of(v["byte_offset"], "byte_offset"), int_of(v["count"], "count"))

        "mutate_arithmetic" ->
          Hooks.get(:byte_arithmetic).(
            bytes_of(v["input"], "input"),
            int_of(v["byte_offset"], "byte_offset"),
            int_of(v["delta"], "delta"),
            int_of(v["width"], "width"),
            text_of(v["endian"], "endian")
          )

        "mutate_boundary" ->
          Hooks.get(:boundary_insert).(
            bytes_of(v["input"], "input"),
            int_of(v["byte_offset"], "byte_offset"),
            int_of(v["width"], "width"),
            int_of(v["val"], "val"),
            text_of(v["endian"], "endian")
          )

        "mutate_block_delete" ->
          Hooks.get(:block_delete).(bytes_of(v["input"], "input"), int_of(v["offset"], "offset"), int_of(v["length"], "length"))

        "mutate_block_insert" ->
          Hooks.get(:block_insert).(bytes_of(v["input"], "input"), int_of(v["offset"], "offset"), bytes_of(v["insert_bytes"], "insert_bytes"))

        "mutate_block_duplicate" ->
          Hooks.get(:block_duplicate).(
            bytes_of(v["input"], "input"),
            int_of(v["src_offset"], "src_offset"),
            int_of(v["length"], "length"),
            int_of(v["dst_offset"], "dst_offset")
          )

        "mutate_splice" ->
          Hooks.get(:splice).(
            bytes_of(v["target"], "target"),
            bytes_of(v["donor"], "donor"),
            int_of(v["target_split"], "target_split"),
            int_of(v["donor_split"], "donor_split")
          )

        "mutate_sequence" ->
          sequence_result(v, work)
      end

    problems = compare(cid, op, got, v)

    cond do
      not is_binary(got) ->
        problems

      op in ["mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary"] ->
        add_if(problems, byte_size(got) != byte_size(v["input"]), "#{cid}: #{op} changed the length of its input")

      op == "mutate_splice" ->
        tgt = v["target"]
        don = v["donor"]
        want = clamp(v["target_split"], byte_size(tgt)) + byte_size(don) - clamp(v["donor_split"], byte_size(don))
        add_if(problems, byte_size(got) != want, "#{cid}: splice length is not prefix + suffix")

      true ->
        problems
    end
  end

  defp clamp(n, size), do: n |> max(0) |> min(size)

  defp add_if(list, true, problem), do: list ++ [problem]
  defp add_if(list, false, _problem), do: list

  # Charges the REAL work: the call is handed what is left of the aggregate cap and
  # what it actually copied (inserts and duplicates grow the buffer, so
  # steps x initial length would undercount) is deducted afterwards.  A call that hits
  # its cap is charged too, the aggregate is then ZEROED, and an exhausted aggregate
  # fails every later case at once without running it.
  defp sequence_result(v, work) do
    input = bytes_of(v["input"], "input")
    seed = int_of(v["seed"], "seed")
    steps = size_of(v["steps"], "steps")
    left = :atomics.get(work, 1)
    exhausted = "mutate_sequence work budget of #{Limits.corpus_sequence_work()} byte-steps exhausted"
    if left <= 0, do: case_error(exhausted)

    case Hooks.get(:mutate_sequence).(input, seed, steps, Limits.max_input_bytes(), left) do
      {:ok, data, done} ->
        :atomics.put(work, 1, max(0, left - done))
        data

      {:work_exhausted, _done} ->
        :atomics.put(work, 1, 0)
        case_error(exhausted)
    end
  end

  defp compare(cid, op, got, v) do
    want = v["output"]

    cond do
      not is_binary(want) -> ["#{cid}: #{op} output must be bytes"]
      got == want -> []
      is_binary(got) -> ["#{cid}: #{op} expected #{hex(want)}, got #{hex(got)}"]
      true -> ["#{cid}: #{op} did not return bytes"]
    end
  end

  defp hex(bytes), do: Clip.clip(Base.encode16(bytes, case: :lower), Values.max_fragment_len())

  defp check_corpus_admit(cid, v) do
    existing = edges_of(v["existing_edges"], "existing_edges")
    trace = edges_of(v["trace_edges"], "trace_edges")
    admitted = bool_of(v["admitted"], "admitted")
    new_edges = int_of(v["new_edges"], "new_edges")
    {got_admitted, got_new} = Hooks.get(:corpus_admit).(existing, trace)
    have = MapSet.new(existing)

    []
    |> add_if(got_admitted != admitted, "#{cid}: corpus_admit admitted #{inspect(got_admitted)}, corpus #{admitted}")
    |> add_if(got_new != new_edges, "#{cid}: corpus_admit new_edges #{inspect(got_new)}, corpus #{Values.show(new_edges)}")
    # Invariant: admission is exactly "some trace edge is unseen".
    |> add_if(got_admitted != Enum.any?(trace, &(not MapSet.member?(have, &1))), "#{cid}: corpus_admit disagrees with set difference")
  end

  defp check_crash_dedup(cid, v) do
    ka = text_of(v["crash_a_kind"], "crash_a_kind")
    fa = texts_of(v["crash_a_frames"], "crash_a_frames")
    kb = text_of(v["crash_b_kind"], "crash_b_kind")
    fb = texts_of(v["crash_b_frames"], "crash_b_frames")
    want = bool_of(v["is_duplicate"], "is_duplicate")
    got = Hooks.get(:duplicate_crash).(ka, fa, kb, fb)
    if got == want, do: [], else: ["#{cid}: crash_dedup is_duplicate #{inspect(got)}, corpus #{want}"]
  end

  defp check_private_reporting(cid, v) do
    raw = bytes_of(v["raw_payload"], "raw_payload")
    target = text_of(v["target"], "target")
    fid = text_of(v["failure_id"], "failure_id")
    sig = text_of(v["sig_hash"], "sig_hash")
    tap = text_of(v["tap_line"], "tap_line")
    want_leak = bool_of(v["contains_raw_payload"], "contains_raw_payload")

    line =
      case Safe.call(fn -> Hooks.get(:format_tap).(1, target, fid, sig) end) do
        {:ok, line} when is_binary(line) -> line
        {:ok, _} -> case_error("format_tap_failure did not return text")
        {:raised, kind, reason} -> case_error("format_tap_failure refused: #{Clip.clip(describe(kind, reason), Values.max_fragment_len())}")
      end

    leaked = raw != <<>> and String.contains?(line, Base.encode16(raw, case: :lower))

    []
    |> add_if(
      line != tap,
      "#{cid}: tap_line expected #{Clip.clip(Canon.escape(tap), Values.max_fragment_len())}, got #{Clip.clip(Canon.escape(line), Values.max_fragment_len())}"
    )
    |> add_if(leaked != want_leak, "#{cid}: privacy verdict wrong, payload leaked: #{leaked}")
    |> add_if(Regex.match?(~r/[\r\n]/, line), "#{cid}: failure line spans more than one line")
  end
end
