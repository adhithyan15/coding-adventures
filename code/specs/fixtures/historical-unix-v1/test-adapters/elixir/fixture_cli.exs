defmodule CodingAdventures.UnixHistory.FixtureCLI do
  @moduledoc false

  # This command adapter belongs to the shared fixture, not the library.
  # replay_cli.py supplies one canonical command and compares this JSON trace.
  def main([command]) do
    command
    |> CodingAdventures.UnixHistory.run_command()
    |> :json.encode()
    |> IO.puts()
  end

  def main(_args), do: raise(ArgumentError, "expected exactly one shell command")
end
