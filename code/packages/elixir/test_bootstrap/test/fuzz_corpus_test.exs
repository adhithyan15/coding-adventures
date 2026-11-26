defmodule FuzzCorpusTest do
  use ExUnit.Case, async: true
  @moduletag timeout: 120_000

  alias CodingAdventures.TestBootstrap.Fuzz.{Corpus, Hooks, Mutate}
  alias CodingAdventures.TestBootstrap.CorpusJson.StrictJson

  test "the committed fuzz-v1 corpus replays: 25 evaluated, zero problems" do
    report = Corpus.evaluate(Fx.fuzz_cases())
    assert report.problems == []
    assert report.evaluated == 25
    assert length(Fx.fuzz_cases()) == Corpus.case_count()
  end

  test "cases.tfv is accepted by the strict TFV validator" do
    assert CodingAdventures.TestBootstrap.Tfv.valid?(File.read!(Fx.fuzz_path("cases.tfv")))
  end

  # -- Go's 820 oracle vectors, parsed from the committed table (no copy) ---------------

  @row ~r/\A\t\{"([a-z_]+)", \[\]string\{(.*)\}, "([0-9a-f]*)"\},\z/

  defp vectors do
    lines = File.read!(Fx.go_vectors_path()) |> String.split("\n")
    start = Enum.find_index(lines, &(&1 == "var referenceVectors = []refVector{"))
    assert start, "the vector table moved"

    lines
    |> Enum.drop(start + 1)
    |> Enum.take_while(&(&1 != "}"))
    |> Enum.with_index()
    |> Enum.map(fn {line, i} ->
      [_, op, args, out] = Regex.run(@row, line) || flunk("unparseable vector row #{i}: #{line}")
      {op, Regex.scan(~r/"([^"]*)"/, args) |> Enum.map(&List.last/1), out}
    end)
  end

  test "all 820 reference vectors match" do
    vs = vectors()
    assert length(vs) == 820

    for {{op, a, out}, i} <- Enum.with_index(vs) do
      i2 = &String.to_integer/1
      h = &Fx.hex/1

      got =
        case op do
          "bit_flip" -> Mutate.bit_flip(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), i2.(Enum.at(a, 2)))
          "byte_flip" -> Mutate.byte_flip(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), i2.(Enum.at(a, 2)))
          "arithmetic" -> Mutate.byte_arithmetic(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), i2.(Enum.at(a, 2)), i2.(Enum.at(a, 3)), Enum.at(a, 4))
          "boundary" -> Mutate.boundary_insert(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), i2.(Enum.at(a, 2)), i2.(Enum.at(a, 3)), Enum.at(a, 4))
          "block_delete" -> Mutate.block_delete(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), i2.(Enum.at(a, 2)))
          "block_insert" -> Mutate.block_insert(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), h.(Enum.at(a, 2)))
          "block_duplicate" -> Mutate.block_duplicate(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), i2.(Enum.at(a, 2)), i2.(Enum.at(a, 3)))
          "splice" -> Mutate.splice(h.(Enum.at(a, 0)), h.(Enum.at(a, 1)), i2.(Enum.at(a, 2)), i2.(Enum.at(a, 3)))
          "sequence" -> Mutate.mutate_sequence(h.(Enum.at(a, 0)), i2.(Enum.at(a, 1)), i2.(Enum.at(a, 2)), i2.(Enum.at(a, 3)))
          other -> flunk("unknown vector op #{other}")
        end

      assert Fx.unhex(got) == out, "vector #{i} #{op} #{inspect(a)}"
    end

    counts = Enum.frequencies_by(vs, &elem(&1, 0))
    for op <- ~w(bit_flip byte_flip arithmetic boundary block_delete block_insert block_duplicate splice sequence) do
      assert counts[op] >= 50, "only #{counts[op]} vectors for #{op}"
    end
  end

  # -- negative controls: the runner must FAIL on mutated fixtures ----------------------

  defp replace(cases, index, fun), do: List.update_at(cases, index, fun)

  defp bump(%{"kind" => "int", "value" => v}), do: Fx.int(String.to_integer(v) + 1)
  defp bump(%{"kind" => "bytes", "hex" => h}), do: %{"kind" => "bytes", "hex" => h <> "00"}
  defp bump(b) when is_boolean(b), do: not b

  @mutators ~w(mutate_bit_flip mutate_byte_flip mutate_arithmetic mutate_boundary mutate_block_delete mutate_block_insert mutate_block_duplicate mutate_splice mutate_sequence)

  # every corruption keeps the case RUNNABLE, so the only problems are its own
  defp corruptions(%{"op" => op} = c) when op in @mutators, do: [%{c | "output" => bump(c["output"])}]
  defp corruptions(%{"op" => "corpus_admit"} = c), do: [%{c | "new_edges" => bump(c["new_edges"])}, %{c | "admitted" => bump(c["admitted"])}]
  defp corruptions(%{"op" => "crash_dedup"} = c), do: [%{c | "is_duplicate" => bump(c["is_duplicate"])}]
  defp corruptions(%{"op" => "failure_id_valid"} = c), do: [%{c | "valid" => bump(c["valid"])}]

  defp corruptions(%{"op" => "private_reporting"} = c),
    do: [%{c | "tap_line" => c["tap_line"] <> "x"}, %{c | "contains_raw_payload" => bump(c["contains_raw_payload"])}]

  defp corruptions(%{"op" => "corpus_reject"} = c), do: [%{c | "text" => "tfv 1\n"}]

  test "a wrong expected crash, coverage count, output or verdict fails, naming exactly that case" do
    cases = Fx.fuzz_cases()
    assert Enum.all?(cases, &(corruptions(&1) != []))

    for {c, i} <- Enum.with_index(cases), corrupted <- corruptions(c) do
      report = Corpus.evaluate(replace(cases, i, fn _ -> corrupted end))
      assert report.problems != [], "#{c["id"]} corruption was not detected"

      for p <- report.problems do
        assert String.starts_with?(p, c["id"]), "problem #{inspect(p)} does not start with #{c["id"]}"
      end
    end
  end

  test "every op is covered by the corruption table" do
    ops = Fx.fuzz_cases() |> Enum.map(& &1["op"]) |> Enum.uniq() |> Enum.sort()
    assert length(ops) == 14
  end

  test "dropped, duplicated and truncated corpora fail" do
    cases = Fx.fuzz_cases()
    assert Corpus.evaluate(tl(cases)).problems != []
    assert Corpus.evaluate(Enum.take(cases, 10)).problems != []
    assert Corpus.evaluate(cases ++ [hd(cases)]).problems != []
    raw = Fx.fuzz_raw()
    assert_raise StrictJson.JsonError, fn -> Corpus.parse(binary_part(raw, 0, byte_size(raw) - 10)) end
    assert_raise StrictJson.JsonError, fn -> Corpus.parse("{}") end
    assert_raise StrictJson.JsonError, fn -> Corpus.parse(~s({"schema_version": 2, "cases": []})) end
    assert_raise StrictJson.JsonError, fn -> Corpus.parse(~s({"schema_version": 1, "cases": 3})) end
  end

  test "removing every case of one op fails (each op must be exercised)" do
    cases = Fx.fuzz_cases()

    for op <- cases |> Enum.map(& &1["op"]) |> Enum.uniq() do
      kept = Enum.reject(cases, &(&1["op"] == op))
      report = Corpus.evaluate(kept, nil)
      assert Enum.any?(report.problems, &String.contains?(&1, "never exercised")), "dropping #{op} was not noticed"
    end
  end

  test "malformed cases are problems, never silent skips" do
    cases = Fx.fuzz_cases()
    one = hd(cases)

    for bad <- [
          %{one | "op" => "no_such_op"},
          Map.delete(one, "output"),
          Map.put(one, "extra", 1),
          %{one | "id" => ""},
          %{one | "count" => "text"},
          %{one | "input" => Fx.int(1)},
          %{one | "output" => true},
          %{one | "bit_offset" => 3.5},
          %{one | "count" => %{"kind" => "int", "value" => "007"}},
          %{one | "input" => %{"kind" => "bytes", "hex" => "ABC"}},
          %{one | "count" => %{"kind" => "int", "value" => String.duplicate("9", 2000)}},
          "not an object",
          %{}
        ] do
      report = Corpus.evaluate(replace(cases, 0, fn _ -> bad end))
      assert report.problems != [], "#{inspect(bad)} was accepted"
    end

    dup = Corpus.evaluate([one, one], nil)
    assert Enum.any?(dup.problems, &String.contains?(&1, "duplicate or missing case id"))
    assert Corpus.evaluate([], nil).problems != []
    assert Enum.any?(Corpus.evaluate(List.duplicate(%{}, 10_001), nil).problems, &String.contains?(&1, "exceeds the cap"))
  end

  defp find(cases, op), do: Enum.find_index(cases, &(&1["op"] == op))

  test "bad shapes of every non-mutator op are problems" do
    cases = Fx.fuzz_cases()
    mutate = fn op, key, val -> replace(cases, find(cases, op), &Map.put(&1, key, val)) end

    for mutated <- [
          mutate.("corpus_admit", "existing_edges", Fx.int(1)),
          mutate.("corpus_admit", "existing_edges", [Fx.int(-1)]),
          mutate.("corpus_admit", "trace_edges", [%{"kind" => "int", "value" => "18446744073709551616"}]),
          mutate.("corpus_admit", "trace_edges", ["x"]),
          mutate.("corpus_admit", "admitted", Fx.int(1)),
          mutate.("corpus_admit", "new_edges", true),
          mutate.("crash_dedup", "crash_a_kind", Fx.int(1)),
          mutate.("crash_dedup", "crash_a_frames", ["ok", Fx.int(1)]),
          mutate.("crash_dedup", "crash_a_frames", "eval"),
          mutate.("crash_dedup", "is_duplicate", "yes"),
          mutate.("failure_id_valid", "input", Fx.int(3)),
          mutate.("failure_id_valid", "valid", Fx.int(1)),
          mutate.("private_reporting", "sig_hash", "ZZZZZZZZ"),
          mutate.("private_reporting", "sig_hash", "3a8f19b"),
          mutate.("private_reporting", "failure_id", "fuzz-bogus"),
          mutate.("private_reporting", "target", Fx.int(1)),
          mutate.("private_reporting", "raw_payload", "text"),
          mutate.("private_reporting", "contains_raw_payload", "no"),
          mutate.("corpus_reject", "text", Fx.int(3)),
          mutate.("mutate_arithmetic", "endian", "middle"),
          mutate.("mutate_arithmetic", "endian", Fx.int(1)),
          mutate.("mutate_arithmetic", "width", true),
          mutate.("mutate_sequence", "steps", Fx.int(-1)),
          mutate.("mutate_sequence", "steps", Fx.int(10_001)),
          mutate.("mutate_sequence", "seed", "x"),
          mutate.("mutate_splice", "target", "x"),
          mutate.("mutate_block_insert", "insert_bytes", Fx.int(1))
        ] do
      assert Corpus.evaluate(mutated).problems != []
    end
  end

  test "a fixture id with control and bidi characters is escaped in every problem" do
    cases = Fx.fuzz_cases()
    evil = "fuzz/x\n" <> <<0x202E::utf8, 0x85::utf8>> <> " forged"
    corrupted = Map.put(hd(cases), "id", evil) |> Map.put("output", bump(hd(cases)["output"]))
    problems = Corpus.evaluate(replace(cases, 0, fn _ -> corrupted end)).problems
    assert problems != []
    joined = Enum.join(problems, "\n")
    refute String.contains?(joined, <<0x202E::utf8>>)
    refute String.contains?(joined, <<0x85::utf8>>)
    refute String.contains?(joined, "\u0085")
    assert String.contains?(joined, "\\u202e")
    # one line per problem: no raw newline from the id
    assert Enum.all?(problems, &(not String.contains?(&1, "\n")))
  end

  # -- the aggregate mutate_sequence budget (CWE-400) -------------------------------------

  defp seq_case(id, input, seed, steps),
    do: %{
      "id" => id,
      "op" => "mutate_sequence",
      "input" => Fx.bytes_node(input),
      "seed" => Fx.int(seed),
      "steps" => Fx.int(steps),
      "output" => Fx.bytes_node("")
    }

  test "the aggregate work is charged for failed calls and a breach zeroes the allowance" do
    parent = self()

    # an engine whose first call runs into the cap
    counting = fn _data, _seed, _steps, _max_len, cap ->
      send(parent, {:seq_call, cap})
      {:work_exhausted, 999}
    end

    cases = for i <- 1..4, do: seq_case("seq/#{i}", <<7, 7, 7>>, i, 10)

    report = Hooks.with([mutate_sequence: counting], fn -> Corpus.evaluate(cases, nil) end)

    # the first call ran with the whole allowance and breached it; the allowance is
    # zeroed, so the other three fail WITHOUT running the engine
    assert_received {:seq_call, 16_777_216}
    refute_received {:seq_call, _}
    exhausted = Enum.filter(report.problems, &String.contains?(&1, "work budget"))
    assert length(exhausted) == 4
    assert String.starts_with?(hd(exhausted), "seq/1")
  end

  test "a successful call is charged its real work (the next call gets what is left)" do
    parent = self()

    counting = fn data, seed, steps, max_len, cap ->
      send(parent, {:seq_call, cap})
      Mutate.mutate_sequence_counted(data, seed, steps, max_len, cap)
    end

    cases = [seq_case("seq/a", <<1, 2, 3, 4>>, 1, 10), seq_case("seq/b", <<1, 2, 3, 4>>, 2, 10)]
    Hooks.with([mutate_sequence: counting], fn -> Corpus.evaluate(cases, nil) end)
    assert_received {:seq_call, first}
    assert_received {:seq_call, second}
    assert first == 16_777_216
    assert second < first
    # at least the initial copy (4) plus one buffer per step was charged
    assert first - second >= 4 + 10
  end

  # -- sabotage hooks ------------------------------------------------------------------------

  test "every sabotage hook makes the corpus run fail, and nothing leaks" do
    cases = Fx.fuzz_cases()

    sabotage = [
      bit_flip: fn d, _o, _c -> d end,
      byte_flip: fn d, _o, _c -> d end,
      byte_arithmetic: fn d, _o, _dl, _w, _e -> d end,
      boundary_insert: fn d, _o, _w, _v, _e -> d end,
      block_delete: fn d, _o, _l -> d end,
      block_insert: fn d, _o, _i -> d end,
      block_duplicate: fn d, _s, _l, _t -> d end,
      splice: fn t, _d, _a, _b -> t end,
      mutate_sequence: fn d, _s, _n, _m, _c -> {:ok, d, 0} end,
      corpus_admit: fn _e, _t -> {false, 0} end,
      duplicate_crash: fn _a, _b, _c, _d -> true end,
      failure_id: fn _id -> true end,
      format_tap: fn _n, _t, _f, _s -> "not ok 1 - x" end,
      tfv_valid: fn _t -> true end
    ]

    assert Enum.sort(Keyword.keys(sabotage)) == Enum.sort(Hooks.names())

    for {name, value} <- sabotage do
      report = Hooks.with([{name, value}], fn -> Corpus.evaluate(cases) end)
      assert report.problems != [], "hook #{name} was not caught"
    end

    assert Corpus.evaluate(cases).problems == []
  end

  test "hooks that return the wrong type or raise are problems" do
    cases = Fx.fuzz_cases()

    for overrides <- [
          [bit_flip: fn _d, _o, _c -> :not_bytes end],
          [bit_flip: fn _d, _o, _c -> raise "boom" end],
          [corpus_admit: fn _e, _t -> throw(:x) end],
          [format_tap: fn _n, _t, _f, _s -> :nope end],
          [format_tap: fn _n, _t, _f, _s -> raise ArgumentError, "refused" end],
          [mutate_sequence: fn _d, _s, _n, _m, _c -> exit(:gone) end]
        ] do
      assert Hooks.with(overrides, fn -> Corpus.evaluate(cases) end).problems != []
    end
  end

  test "a TAP line that leaks the payload or spans two lines is a problem" do
    cases = Fx.fuzz_cases()
    i = find(cases, "private_reporting")
    c = Enum.at(cases, i)

    # a hook that prints the payload hex in the line (the expected tap_line is then also wrong)
    leak = fn _n, _t, _f, _s -> "not ok 1 - t [fuzz-crash:" <> Base.encode16(Fx.hex("deadbeef"), case: :lower) <> "]" end
    assert Enum.any?(Hooks.with([format_tap: leak], fn -> Corpus.evaluate(cases) end).problems, &String.contains?(&1, "leaked"))

    # a hook that returns a two-line string, with a corpus whose expected line is the same
    two = "not ok 1 - a\nok 2 - b"
    forged = replace(cases, i, fn _ -> %{c | "tap_line" => two} end)
    report = Hooks.with([format_tap: fn _n, _t, _f, _s -> two end], fn -> Corpus.evaluate(forged) end)
    assert Enum.any?(report.problems, &String.contains?(&1, "more than one line"))
  end

  test "hooks reject unknown names and restore on raise" do
    assert_raise ArgumentError, fn -> Hooks.with([nope: 1], fn -> :ok end) end

    assert_raise RuntimeError, fn -> Hooks.with([tfv_valid: fn _ -> true end], fn -> raise "boom" end) end
    assert Hooks.get(:tfv_valid).("tfv 1\n") in [true, false]
    refute Hooks.get(:tfv_valid).("bad header\n")
  end

  test "the engine reports unregistered failure ids as a problem when the hook rejects them" do
    report = Hooks.with([failure_id: fn _ -> false end], fn -> Corpus.evaluate(Fx.fuzz_cases()) end)
    assert Enum.any?(report.problems, &String.contains?(&1, "engine rejects registered ids"))
    assert Enum.any?(report.problems, &String.contains?(&1, "registered-id acceptance"))
  end
end
