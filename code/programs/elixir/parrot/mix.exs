defmodule Parrot.MixProject do
  use Mix.Project

  # mix.exs — the build manifest for the Parrot program.
  #
  # Mix is Elixir's build tool, dependency manager, and task runner.
  # This file tells Mix everything it needs to know about this project:
  # its name, version, Elixir compatibility, and how to build it into
  # a runnable binary (escript).
  #
  # The escript target compiles the project into a self-contained .escript
  # file that can be executed with `elixir parrot` or `mix escript.build`.
  # All Elixir code is bundled in; only the Erlang runtime needs to be
  # installed on the target machine.

  def project do
    [
      app: :parrot,
      version: "1.0.0",
      elixir: "~> 1.14",
      # start_permanent: in production (:prod), if the application supervisor
      # crashes, the whole VM shuts down. In :dev/:test it just logs the error.
      start_permanent: Mix.env() == :prod,
      # escript: configure the runnable binary. main_module is the module whose
      # main/1 function serves as the entry point (like `main` in C/Java).
      escript: [main_module: Parrot.Main],
      deps: deps(),
      # Require at least 80% test coverage before tests pass.
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  # application/0 — describes the OTP application.
  #
  # extra_applications: we pull in :logger so that the Elixir logging
  # infrastructure is available. Most programs want this even if they don't
  # call Logger directly, because dependencies often do.
  def application do
    [extra_applications: [:logger]]
  end

  # deps/0 — the list of libraries this project depends on.
  #
  # We use a path dependency so that the local `repl` package is used
  # directly from the monorepo checkout. In a published package this would
  # be a version dependency on Hex (the Elixir package registry).
  #
  # Path layout:  code/programs/elixir/parrot/  (this file)
  #                               ↑ three levels up
  #               code/packages/elixir/repl/
  defp deps do
    [
      {:coding_adventures_repl, path: "../../../packages/elixir/repl"}
    ]
  end
end
