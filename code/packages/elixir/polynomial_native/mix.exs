defmodule CodingAdventures.PolynomialNative.MixProject do
  use Mix.Project

  # ---------------------------------------------------------------------------
  # Project configuration
  # ---------------------------------------------------------------------------
  #
  # This package wraps the Rust `polynomial` crate as an Erlang NIF.
  # The NIF shared library is built by `cargo build --release` in
  # `native/polynomial_native/`, triggered by the `:make` compiler below.

  def project do
    [
      app: :coding_adventures_polynomial_native,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      # The :make compiler runs our Makefile to build the Rust shared library.
      # Mix's built-in compilers ([:erlang, :elixir, :app]) handle the .ex files.
      compilers: Mix.compilers() ++ [:make],
      make_targets: ["all"],
      make_clean: ["clean"],
      make_cwd: "native/polynomial_native",
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      # elixir_make provides the :make compiler that invokes cargo for us.
      {:elixir_make, "~> 0.7", runtime: false}
    ]
  end
end
