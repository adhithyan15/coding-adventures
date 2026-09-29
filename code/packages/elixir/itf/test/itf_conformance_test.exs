defmodule CodingAdventures.ItfConformanceTest do
  use ExUnit.Case

  alias CodingAdventures.Itf

  @cases_path Path.expand(
                "../../../../specs/fixtures/barcode-symbologies-v1/cases.json",
                __DIR__
              )

  defp cases do
    @cases_path
    |> File.read!()
    |> Jason.decode!()
    |> Map.fetch!("cases")
    |> Enum.filter(&(&1["symbology"] == "itf"))
  end

  defp input(%{"text" => text}), do: text

  defp input(%{"repeat" => %{"text" => text, "count" => count}}),
    do: String.duplicate(text, count)

  defp modules(data) do
    pairs = Itf.encode_itf(data)
    "1010" <> Enum.map_join(pairs, & &1.binary_pattern) <> "11101"
  end

  defp run_lengths(bits) do
    bits
    |> String.graphemes()
    |> Enum.chunk_by(& &1)
    |> Enum.map(&length/1)
  end

  defp sha256(value), do: :crypto.hash(:sha256, value) |> Base.encode16(case: :lower)

  test "executes all ITF v1 cases" do
    assert length(cases()) == 10

    for test_case <- cases() do
      data = input(test_case["input"])
      expected = test_case["expected"]

      if expected["error"] do
        error = assert_raise ArgumentError, fn -> Itf.normalize_itf(data) end
        assert Itf.error_id(error) == expected["error"]
      else
        normalized = Itf.normalize_itf(data)
        encoded_modules = modules(data)
        runs = run_lengths(encoded_modules)

        if expected["normalized"] do
          assert normalized == expected["normalized"]
          assert encoded_modules == expected["modules"]
          assert runs == expected["run_lengths"]
        else
          assert sha256(normalized) == expected["normalized_sha256"]
          assert String.length(encoded_modules) == expected["module_count"]
          assert sha256(encoded_modules) == expected["module_sha256"]
          assert length(runs) == expected["run_count"]
          assert sha256(Jason.encode!(runs)) == expected["run_lengths_sha256"]
        end
      end
    end
  end
end
