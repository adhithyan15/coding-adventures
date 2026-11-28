path = hd(System.argv())
max_report_bytes = 4 * 1024 * 1024

with {:ok, bytes} <-
       File.open(path, [:read, :binary], fn io -> IO.binread(io, max_report_bytes + 1) end),
     true <- is_binary(bytes) and byte_size(bytes) <= max_report_bytes,
     {:ok, report} <- CodingAdventures.Microbenchmark.decode_report(bytes),
     true <- length(report.pairs) == 40,
     true <-
       Enum.all?(report.pairs, fn pair ->
         pair.baseline_ns > 0 and pair.candidate_ns > 0
       end) do
  IO.puts("native report replay passed: 40 positive pairs")
else
  error ->
    IO.puts(:stderr, "native report replay failed: #{inspect(error)}")
    System.halt(1)
end
