defmodule CodingAdventures.Conduit.MixProject do
  use Mix.Project

  # ---------------------------------------------------------------------------
  # Mix project for `coding_adventures_conduit`
  # ---------------------------------------------------------------------------
  #
  # This package wraps the Rust `conduit_native` cdylib as an Erlang NIF.
  #
  # The Rust shared library is built EXTERNALLY by the BUILD file: it runs
  # `cargo build --release` in `native/conduit_native/` and copies the
  # resulting .so/.dylib into `priv/conduit_native.so`. We do NOT use
  # `elixir_make` here because Mix tries to load `Mix.Tasks.Compile.Make`
  # at startup — before `elixir_make` itself has been compiled — causing
  # a chicken-and-egg "task not found" error in CI.
  #
  # See `lessons.md` (2026-04-04 entry) for the full story.

  def project do
    [
      app: :coding_adventures_conduit,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger, :inets, :ssl]]
  end

  defp deps do
    []
  end
end
