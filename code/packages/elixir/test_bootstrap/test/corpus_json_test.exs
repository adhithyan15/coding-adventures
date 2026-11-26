defmodule HostileInspect do
  defstruct [:a]
  defimpl Inspect do
    def inspect(_, _), do: raise("hostile inspect must never run")
  end
  defimpl String.Chars do
    def to_string(_), do: raise("hostile to_string must never run")
  end
end

defmodule HostileError do
  defexception [:message]
  def message(_), do: raise("hostile message/1 must never run")
end

defmodule CorpusJsonTest do
  use ExUnit.Case, async: true
  @moduletag timeout: 60_000

  import Bitwise
  alias CodingAdventures.TestBootstrap.{Safe, Tfv}
  alias CodingAdventures.TestBootstrap.CorpusJson.{Canon, Clip, StrictJson, Values}
  alias CodingAdventures.TestBootstrap.CorpusJson.StrictJson.JsonError
  alias CodingAdventures.TestBootstrap.CorpusJson.Values.CaseError

  # -- StrictJson -----------------------------------------------------------------------------

  defp parse!(s), do: StrictJson.parse(s)
  defp rejects(s), do: assert_raise(JsonError, fn -> StrictJson.parse(s) end)

  test "accepts ordinary documents and keeps numbers as text" do
    assert parse!(~s({"a": [1, -0, 1.5e+10, 123456789012345678901234567890], "b": null, "c": true, "d": false, "e": "x"})) ==
             %{
               "a" => [{:number, "1"}, {:number, "-0"}, {:number, "1.5e+10"}, {:number, "123456789012345678901234567890"}],
               "b" => nil,
               "c" => true,
               "d" => false,
               "e" => "x"
             }

    assert parse!(" \t\r\n[ ] ") == []
    assert parse!("{}") == %{}
    assert parse!(~s("\\u0041\\ud83d\\ude00\\n\\/")) == "A\u{1F600}\n/"
    assert parse!(~s(["é", "日本"])) == ["é", "日本"]
    assert parse!("1E5") == {:number, "1E5"}
    assert parse!("0.0") == {:number, "0.0"}
  end

  test "rejects duplicate keys, compared as decoded bytes" do
    rejects(~s({"a": 1, "a": 2}))
    rejects(~s({"a": 1, "\\u0061": 2}))
    assert parse!(~s({"a": 1, "A": 2})) |> map_size() == 2
  end

  test "rejects invalid UTF-8, a BOM and raw control characters" do
    rejects(<<?", 0xFF, ?">>)
    rejects(<<0xEF, 0xBB, 0xBF, ?1>>)
    rejects(<<?", 0xC0, 0xAF, ?">>)
    rejects(<<?", 0xED, 0xA0, 0x80, ?">>)
    rejects("\"a\nb\"")
    rejects("\"a\tb\"")
    rejects(<<?", 0, ?">>)
  end

  test "rejects bad escapes and surrogates" do
    for bad <- [
          "\"\\x\"",
          "\"\\u12\"",
          "\"\\uZZZZ\"",
          "\"\\ud83d\"",
          "\"\\ud83dx\"",
          "\"\\ude00\"",
          "\"\\ud83d\\u0041\"",
          "\"abc\\",
          "\"abc",
          "\"\\u",
          "\"\\ud83d\\u12"
        ] do
      rejects(bad)
    end
  end

  test "rejects malformed numbers and literals" do
    for bad <- ["01", "-01", "+1", "1.", ".5", "-", "1e", "1e+", "NaN", "Infinity", "0x10", "tru", "nul", "fals", "--1", "1.e5", "-a"] do
      rejects(bad)
    end
  end

  test "rejects trailing data, commas, comments and structure errors" do
    for bad <- ["1 2", "[1,]", "[1 2]", "{\"a\":1,}", "{\"a\" 1}", "{a: 1}", "{1: 1}", "// c\n1", "[1] /* c */", "[", "{", "", "   ", "[1}", "{\"a\":1]", "\"a\" x", "{\"a\":}", "@", "[1,,2]"] do
      rejects(bad)
    end
  end

  test "nesting: 64 containers are fine, the 65th is refused" do
    ok = String.duplicate("[", 64) <> String.duplicate("]", 64)
    assert parse!(ok) != nil
    rejects(String.duplicate("[", 65) <> String.duplicate("]", 65))
    okobj = String.duplicate("{\"a\":", 64) <> "1" <> String.duplicate("}", 64)
    assert parse!(okobj) != nil
    rejects(String.duplicate("{\"a\":", 65) <> "1" <> String.duplicate("}", 65))
    # a hostile million-deep document is refused before it can recurse
    rejects(String.duplicate("[", 1_000_000))
  end

  test "size caps: bytes and nodes" do
    rejects(String.duplicate(" ", StrictJson.max_bytes() + 1) <> "1")
    assert parse!("[" <> String.duplicate("0,", StrictJson.max_nodes() - 2) <> "0]") |> length() == StrictJson.max_nodes() - 1
    rejects("[" <> String.duplicate("0,", StrictJson.max_nodes()) <> "0]")
    rejects("{" <> Enum.map_join(1..(div(StrictJson.max_nodes(), 2) + 1), ",", &"\"#{&1}\":0") <> "}")
    assert_raise JsonError, fn -> StrictJson.parse(:nope) end
  end

  test "the committed pbt-v1 fixture parses strictly" do
    assert %{"schema_version" => {:number, "1"}, "cases" => cases} = parse!(Fx.raw())
    assert length(cases) == 25
  end

  # -- Values ------------------------------------------------------------------------------------

  test "decode strictly and distinguish kinds" do
    assert Values.decode(%{"kind" => "int", "value" => "-12"}) == -12
    assert Values.decode(%{"kind" => "bytes", "hex" => "00ff"}) == <<0, 255>>
    assert Values.decode(true) == true
    assert Values.decode("t") == {:text, "t"}
    assert Values.decode([true, %{"kind" => "int", "value" => "0"}]) == [true, 0]

    for bad <- [
          %{"kind" => "int", "value" => "01"},
          %{"kind" => "int", "value" => "-0"},
          %{"kind" => "int", "value" => 5},
          %{"kind" => "int", "value" => String.duplicate("9", 1001)},
          %{"kind" => "bytes", "hex" => "0"},
          %{"kind" => "bytes", "hex" => "FF"},
          %{"kind" => "bytes", "hex" => String.duplicate("00", 10_001)},
          %{"kind" => "int", "value" => "1", "x" => 1},
          %{"kind" => "other"},
          nil,
          {:number, "1"},
          Enum.map(1..10_001, fn _ -> true end)
        ] do
      assert_raise CaseError, fn -> Values.decode(bad) end
    end

    deep = Enum.reduce(1..70, true, fn _, acc -> [acc] end)
    assert_raise CaseError, fn -> Values.decode(deep) end
  end

  test "same? does not conflate true with 1, bytes with text, or lists with anything" do
    assert Values.same?(1, 1)
    refute Values.same?(true, 1)
    refute Values.same?(1, true)
    refute Values.same?(<<"a">>, {:text, "a"})
    assert Values.same?({:text, "a"}, {:text, "a"})
    assert Values.same?([1, <<2>>], [1, <<2>>])
    refute Values.same?([1], [1, 2])
    refute Values.same?([1], 1)
    refute Values.same?(false, nil)
    refute Values.same?(:a, :b)
    deep = Enum.reduce(1..70, 1, fn _, acc -> [acc] end)
    refute Values.same?(deep, deep)
  end

  test "show is bounded" do
    assert String.length(Values.show(String.duplicate("a", 1000))) <= Values.max_fragment_len() + 3
    assert Clip.clip("abc", 10) == "abc"
    assert Clip.clip("abcdef", 3) == "abc..."
    assert Clip.clip(12, 5) == "12"
    assert Clip.clip(:atom, 10) == "<term>"
  end

  # -- Canon -------------------------------------------------------------------------------------------

  test "renders every term kind structurally" do
    assert Canon.render(nil) == "nil"
    assert Canon.render(true) == "true"
    assert Canon.render(-12) == "-12"
    assert Canon.render(:abc) == ":abc"
    assert Canon.render(1.5) == "1.5"
    assert Canon.render("hi") == "\"hi\""
    assert Canon.render(<<255, 1>>) == "0xff01"
    assert Canon.render(<<1::size(3)>>) == "<<bits 3>>"
    assert Canon.render([1, [2]]) == "[1,[2]]"
    assert Canon.render({1, :a}) == "{1,:a}"
    assert Canon.render([1 | 2]) == "[1|2]"
    assert Canon.render(%{b: 1, a: 2}) == "%{:a=>2,:b=>1}"
    assert Canon.render(MapSet.new([3, 1, 2])) == "#MapSet[1,2,3]"
    assert Canon.render(%{__struct__: Foo, z: 1}) == "%Foo{:z=>1}"
    assert Canon.render(self()) == "#PID"
    assert Canon.render(make_ref()) == "#Reference"
    assert Canon.render(fn -> 1 end) == "#Function"
    assert Canon.render(hd(Port.list())) == "#Port"
    assert Canon.render([]) == "[]"
    assert Canon.render({}) == "{}"
    assert Canon.render(%{}) == "%{}"
    assert Canon.render(MapSet.new()) == "#MapSet[]"
    assert Canon.render(%{__struct__: "not an atom"}) =~ "%{"
  end

  test "maps and sets are sorted by bytes, independent of insertion and iteration order" do
    big = for i <- 1..200, into: %{}, do: {"k#{i}", i}
    again = for i <- 200..1//-1, into: %{}, do: {"k#{i}", i}
    assert Canon.render(big) == Canon.render(again)
    assert Canon.render(MapSet.new(1..100)) == Canon.render(MapSet.new(100..1//-1))
    assert Canon.render(%{"b" => 1, "a" => 2}) == "%{\"a\"=>2,\"b\"=>1}"
  end

  test "escapes control, C1 and bidirectional characters" do
    for c <- [0, 1, 0x1F, 0x7F, 0x80, 0x9F, 0x2028, 0x2029, 0x061C, 0x200E, 0x200F, 0x202A, 0x202E, 0x2066, 0x2069] do
      out = Canon.render(<<c::utf8>>)
      assert out =~ "\\u", "U+#{Integer.to_string(c, 16)} was not escaped: #{inspect(out)}"
      refute String.contains?(out, <<c::utf8>>)
    end

    assert Canon.render("a\nb\r\t\"\\") == ~S("a\nb\r\t\"\\")
    assert Canon.render("é日") == "\"é日\""
    assert Canon.render(:"a\nb") == ":a\\nb"
    assert Canon.escape("a\nb") == "a\\nb"
    assert Canon.escape("a\"b") == "a\"b"
    assert Canon.escape("a\"b", quote: true) == "a\\\"b"
    assert Canon.escape(<<255>>) == "0xff"
    assert Canon.render(%{<<0x202E::utf8>> => 1}) =~ "\\u202e"
  end

  test "huge integers are summarised" do
    assert Canon.render(1 <<< 100_001) == "<bigint 100002 bits>"
    assert Canon.render(-(1 <<< 100_001)) == "<bigint 100002 bits>"
    assert Canon.render(1 <<< 99_999) |> byte_size() > 1000
  end

  test "depth beyond 16 renders ..." do
    deep = Enum.reduce(1..40, 1, fn _, acc -> [acc] end)
    assert Canon.render(deep) == String.duplicate("[", 16) <> "..." <> String.duplicate("]", 16)
    assert Canon.render(Enum.reduce(1..40, 1, fn _, acc -> {acc} end)) =~ "..."
    assert Canon.render(Enum.reduce(1..40, 1, fn _, acc -> %{a: acc} end)) =~ "..."
    assert Canon.render(Enum.reduce(1..40, 1, fn _, acc -> MapSet.new([{acc}]) end)) =~ "..."
    assert Canon.render(Enum.reduce(1..40, 1, fn _, acc -> %{__struct__: Foo, a: acc} end)) =~ "..."
  end

  test "more than 10,000 nodes truncates" do
    out = Canon.render(Enum.to_list(1..20_000))
    assert String.ends_with?(out, "...")
    assert byte_size(out) < 100_000
    assert Canon.render(List.to_tuple(Enum.to_list(1..20_000))) |> String.ends_with?("...")
    assert Canon.render(for(i <- 1..20_000, into: %{}, do: {i, i})) |> String.ends_with?("...")
    assert Canon.render(MapSet.new(1..20_000)) |> String.ends_with?("...")
    assert Canon.render(Enum.to_list(1..9_000) ++ [[1 | 2]]) |> is_binary()
  end

  test "more than 1 MiB of output truncates on a character boundary" do
    s = String.duplicate("é", 400_000)
    out = Canon.render([s, s, s])
    assert String.valid?(out)
    assert String.ends_with?(out, "...")
    assert byte_size(out) <= Canon.max_output() + 3
    huge = Canon.render(String.duplicate("a", 3 <<< 20))
    assert byte_size(huge) <= Canon.max_output() + 3
    many = Canon.render(List.duplicate(String.duplicate("x", 5000), 400))
    assert byte_size(many) <= Canon.max_output() + 3
    assert String.ends_with?(many, "...")
    assert Canon.render(%{String.duplicate("k", 600_000) => String.duplicate("v", 600_000)}) |> String.ends_with?("...")
  end

  test "invisible formatting characters are escaped" do
    for c <- [0xAD, 0x34F, 0x180E, 0x200B, 0x200C, 0x200D, 0x2060, 0x2064, 0xFEFF, 0xFFF9, 0xFFFB, 0xE0000, 0xE0041, 0xE007F] do
      out = Canon.render(<<c::utf8>>)
      assert out =~ "\\u", "U+#{Integer.to_string(c, 16)} was not escaped"
      refute String.contains?(out, <<c::utf8>>)
    end

    assert Canon.render(<<0xE0041::utf8>>) == ~S("\u{e0041}")
    assert Canon.render("a\u00ADb") == ~S("a\u00adb")
  end

  test "caps are applied before allocation: a 50 MB binary and a 1M-entry map stay capped and fast" do
    {t1, out1} = :timer.tc(fn -> Canon.render(:binary.copy("a", 50_000_000)) end)
    assert byte_size(out1) <= Canon.max_output() + 3
    assert String.ends_with?(out1, "...")
    assert t1 < 2_000_000

    {t2, out2} = :timer.tc(fn -> Canon.render(:binary.copy(<<255>>, 50_000_000)) end)
    assert byte_size(out2) <= Canon.max_output() + 3
    assert t2 < 2_000_000

    {t3, out3} = :timer.tc(fn -> Canon.render(:binary.copy("é", 25_000_000)) end)
    assert String.valid?(out3)
    assert t3 < 2_000_000

    big = Map.new(1..1_000_000, &{&1, &1})
    {t4, out4} = :timer.tc(fn -> Canon.render(big) end)
    assert byte_size(out4) <= Canon.max_output() + 3
    assert String.ends_with?(out4, "...")
    assert t4 < 2_000_000

    {t5, out5} = :timer.tc(fn -> Canon.render(MapSet.new(1..1_000_000)) end)
    assert String.ends_with?(out5, "...")
    assert t5 < 2_000_000

    struct_big = Map.put(big, :__struct__, Foo)
    assert Canon.render(struct_big) |> String.ends_with?("...")
  end

  test "never runs user inspect, to_string or message callbacks" do
    assert Canon.render(%HostileInspect{a: 1}) == "%HostileInspect{:a=>1}"
    assert Canon.render(%HostileError{message: "m"}) =~ "HostileError"
    assert Canon.render([%HostileInspect{a: %HostileInspect{}}]) =~ "HostileInspect"
    assert Canon.unrenderable() == "<unrenderable>"
  end

  # -- Safe ------------------------------------------------------------------------------------------------

  test "Safe.call classifies raise, throw and exit without looking at the value" do
    assert Safe.call(fn -> 1 end) == {:ok, 1}
    assert {:raised, :error, %RuntimeError{}} = Safe.call(fn -> raise "x" end)
    assert {:raised, :throw, {:any, 1}} = Safe.call(fn -> throw({:any, 1}) end)
    assert {:raised, :exit, :bye} = Safe.call(fn -> exit(:bye) end)
  end

  test "Safe messages: redacted names the kind, local renders structurally" do
    assert Safe.redacted(:error, %HostileError{message: "secret"}) == "error HostileError"
    assert Safe.redacted(:error, :badarg) == "error"
    assert Safe.redacted(:throw, :secret) == "throw"
    assert Safe.redacted(:exit, {:secret, 1}) == "exit"
    assert Safe.local(:throw, :v) == "throw: :v"
    assert String.length(Safe.local(:throw, String.duplicate("x", 5000))) <= 403
    assert Safe.sanitize("a\nb") == "a\\nb"
    assert Safe.local(:error, %HostileError{message: "m"}) =~ "HostileError"
  end

  # -- TFV ---------------------------------------------------------------------------------------------------

  defp tfv_rejects(text), do: refute(Tfv.valid?(text), "accepted: #{inspect(text)}")

  test "TFV accepts well-formed documents" do
    assert Tfv.valid?("tfv 1\n")
    assert Tfv.valid?("tfv 1")
    assert Tfv.valid?("tfv 1\n# comment\n\nr/a.b-c R2 Kx I-5 Ky L2 T N\n")
    assert Tfv.valid?("tfv 1\nr R3 Ka D3ff0000000000000 Kb X00ff Kc S68690a\n")
    assert Tfv.valid?("tfv 1\nr R1 Km M2 I1 T I2 F\n")
    assert Tfv.valid?("tfv 1\nr R1 Km M0\n")
    assert Tfv.valid?("tfv 1\nr R1 Kl L0\n")
    assert Tfv.valid?("tfv 1\na R0\nb R0\n")
    assert Tfv.valid?("tfv 1\nr R1 Kv D7ff8000000000000\n")
    assert Tfv.valid?(File.read!(Fx.fixture_path("cases.tfv")))
  end

  test "TFV rejects every documented corruption" do
    for bad <- [
          "bad header\n",
          "",
          "\n",
          "tfv 2\n",
          "tfv 1\nr R1 Kop S706274 Kdata XFF\n",
          "tfv 1\nr R1 Kx X0\n",
          "tfv 1\nr R1 Kx I-0\n",
          "tfv 1\nr R1 Kx I007\n",
          "tfv 1\nr R1 Kx I\n",
          "tfv 1\nr R1 Kx I-\n",
          "tfv 1\nr R1 Kx Iabc\n",
          "tfv 1\nr R2 Kx I1\n",
          "tfv 1\nr R1 Kx I1 Ky I2\n",
          "tfv 1\nr R1 Kx I1  Ky I2\n",
          "tfv 1\nr R1 Kx I1 \n",
          "tfv 1\nr R2 Kx I1 Kx I2\n",
          "tfv 1\nr R0\nr R0\n",
          "tfv 1\nr R1 Kx Sff\n",
          "tfv 1\nr R1 Kx S6\n",
          "tfv 1\nr R1 Kx S4A\n",
          "tfv 1\nr R1 Kx I1\r\n",
          "tfv 1\nr R1 Kx Q1\n",
          "tfv 1\nr R1 Kx N1\n",
          "tfv 1\nr R1 Kx Dzz\n",
          "tfv 1\nr R1 Kx D00\n",
          "tfv 1\nr R1 Kx L2 T\n",
          "tfv 1\nr R1 Kx L01 T\n",
          "tfv 1\nr R1 Kx L1234567890 T\n",
          "tfv 1\nr R1 Kx Lx T\n",
          "tfv 1\nr R1 Kx L\n",
          "tfv 1\nr R1 Km M2 I1 T I1 F\n",
          "tfv 1\nr R1 Km M2 D7ff8000000000000 T D7ff8000000000001 F\n",
          "tfv 1\nr R1 KX I1\n",
          "tfv 1\nr R1 K I1\n",
          "tfv 1\nr Rx\n",
          "tfv 1\nr R\n",
          "tfv 1\nr\n",
          "tfv 1\nr !x R0\n",
          "tfv 1\nr Q1\n",
          "tfv 1\n r R0\n",
          "tfv 1\nr R1 Kx I" <> String.duplicate("9", 1001) <> "\n",
          "tfv 1\nr R1 Kx " <> String.duplicate("L1 ", 65) <> "T\n",
          "tfv 1\nr R1 Kx R1\n",
          <<"tfv 1\nr R1 Kx I1\n", 0xFF>>,
          String.duplicate("a", 1 <<< 20) <> "x"
        ] do
      tfv_rejects(bad)
    end

    assert_raise Tfv.InvalidTfvError, fn -> Tfv.validate(5) end
    assert_raise Tfv.InvalidTfvError, ~r/line 2/, fn -> Tfv.validate("tfv 1\nr R1\n") end
    # 64 nested lists are fine, 65 are not
    nested = fn n -> "tfv 1\nr R1 Kx " <> String.duplicate("L1 ", n) <> "T\n" end
    assert Tfv.valid?(nested.(64))
    tfv_rejects(nested.(65))
    assert Tfv.valid?("tfv 1\nr R1 Kx I" <> String.duplicate("9", 1000) <> "\n")
  end
end
