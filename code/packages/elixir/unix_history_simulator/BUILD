# needs-toolchain: python
mix format --check-formatted
mix compile
mix test
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py mix run --no-compile -r ../../../specs/fixtures/historical-unix-v1/test-adapters/elixir/fixture_cli.exs -e 'CodingAdventures.UnixHistory.FixtureCLI.main(System.argv())' --
