spec_path = Path.expand("../../../specs/grammar-tools.cli.json", __DIR__)
case CodingAdventures.CliBuilder.Parser.parse(spec_path, ["grammar-tools", "generate"]) do
  {:ok, res} -> IO.inspect(res)
  {:error, err} -> IO.puts("ERROR: #{inspect(err)}")
end
