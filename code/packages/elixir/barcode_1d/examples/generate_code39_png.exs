alias CodingAdventures.Barcode1D

output_path = System.get_env("BARCODE_1D_OUTPUT") || "/tmp/elixir-metal-code39.png"

case Barcode1D.render_png("ELIXIR-METAL-123", symbology: :code39) do
  {:ok, png} ->
    File.write!(output_path, png)
    IO.puts(output_path)

  {:error, reason} ->
    IO.puts(:stderr, "barcode render failed: #{inspect(reason)}")
    System.halt(1)
end
