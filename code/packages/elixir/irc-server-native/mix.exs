defmodule CodingAdventures.IrcServerNative.MixProject do
  use Mix.Project

  # Wraps the Rust `irc_server_native` cdylib as an Erlang NIF. The shared
  # library is built EXTERNALLY by BUILD (cargo build + copy into priv/), not by
  # elixir_make (which causes a chicken-and-egg compile-task error in CI — see
  # lessons.md).

  def project do
    [
      app: :coding_adventures_irc_server_native,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      # The Native module is pure NIF stubs whose bodies are replaced by the Rust
      # cdylib at load time, so they never execute under coverage instrumentation
      # (analogous to excluding the .so itself). Exclude it; the real logic lives
      # in Server, which the end-to-end tests cover.
      test_coverage: [
        summary: [threshold: 80],
        ignore_modules: [CodingAdventures.IrcServerNative.Native]
      ]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps, do: []
end
