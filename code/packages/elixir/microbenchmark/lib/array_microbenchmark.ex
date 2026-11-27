defmodule CodingAdventures.ArrayMicrobenchmark do
  @moduledoc "Native indexed-array benchmark command."
  alias CodingAdventures.{ArrayWorkload, Microbenchmark}

  def main(args) do
    values = ArrayWorkload.make_values()
    expected = ArrayWorkload.indexed_batch(values, 1)
    clock = native_clock()

    config = Microbenchmark.defaults()

    case Microbenchmark.collect(
           config,
           expected,
           &ArrayWorkload.indexed_batch(values, &1),
           &ArrayWorkload.iterator_batch(values, &1),
           clock
         ) do
      {:ok, report} ->
        revision =
          case System.get_env("GIT_REVISION") do
            nil -> "unknown"
            "" -> "unknown"
            value -> value
          end

        metadata = %{
          "workload" => "indexed-array-sum-4096",
          "invocation" => ["array-microbenchmark" | args],
          "revision" => revision,
          "runtime" => System.version(),
          "os" => :os.type() |> Tuple.to_list() |> Enum.join("-"),
          "arch" => :erlang.system_info(:system_architecture) |> List.to_string()
        }

        output = Microbenchmark.encode_report(Map.merge(report, metadata)) <> "\n"

        case :io.put_chars(:standard_io, output) do
          :ok ->
            :ok

          error ->
            IO.puts(:stderr, "report write failed: #{inspect(error)}")
            System.halt(1)
        end

      {:error, reason} ->
        IO.puts(:stderr, "benchmark failed: #{inspect(reason)}")
        System.halt(1)
    end
  rescue
    error ->
      IO.puts(:stderr, "benchmark failed: #{Exception.message(error)}")
      System.halt(1)
  end

  defp native_clock do
    if match?({:win32, _}, :os.type()) do
      # OTP's Windows monotonic wrapper quantizes QueryPerformanceCounter
      # readings. Its raw performance counter keeps the higher-resolution QPC
      # ticks; the collector checks every reading for regressions and rejects
      # the run if the host counter moves backwards.
      base = :os.perf_counter(:nanosecond)
      fn -> :os.perf_counter(:nanosecond) - base end
    else
      base = System.monotonic_time(:nanosecond)
      fn -> System.monotonic_time(:nanosecond) - base end
    end
  end
end
