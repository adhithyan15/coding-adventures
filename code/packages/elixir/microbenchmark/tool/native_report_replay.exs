path = hd(System.argv())

with {:ok, bytes} <- File.read(path),
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
