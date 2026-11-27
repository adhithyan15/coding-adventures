defmodule CodingAdventures.UnixHistory.FixtureCLI do
  @moduledoc false

  # This command adapter belongs to the shared fixture, not the library.
  # build_elixir.py supplies one canonical command through the environment;
  # replay_cli.py compares the resulting JSON trace.
  def main([command]) do
    command
    |> CodingAdventures.UnixHistory.run_command()
    |> :json.encode()
    |> IO.puts()
  end

  def main(_args), do: raise(ArgumentError, "expected exactly one shell command")
end

CodingAdventures.UnixHistory.FixtureCLI.main([
  System.fetch_env!("HISTORICAL_UNIX_FIXTURE_COMMAND")
])
