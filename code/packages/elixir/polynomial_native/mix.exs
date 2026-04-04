defmodule CodingAdventures.PolynomialNative.MixProject do
  use Mix.Project

  # ---------------------------------------------------------------------------
  # Project configuration
  # ---------------------------------------------------------------------------
  #
  # This package wraps the Rust `polynomial` crate as an Erlang NIF.
  #
  # The Rust NIF shared library is built EXTERNALLY by the BUILD file, which
  # runs `cargo build --release` and copies the resulting .so/.dylib into
  # `priv/polynomial_native.so`. We do NOT use `elixir_make` here because Mix
  # tries to load Mix.Tasks.Compile.Make at startup — before elixir_make has
  # been compiled — causing a chicken-and-egg "task not found" error in CI.
  #
  # The BUILD file handles the full lifecycle:
  #   1. cargo build --release            (builds the .so / .dylib)
  #   2. cp ... priv/polynomial_native.so (places where :erlang.load_nif expects)
  #   3. mix deps.get && mix compile && mix test

  def project do
    [
      app: :coding_adventures_polynomial_native,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    []
  end
end
