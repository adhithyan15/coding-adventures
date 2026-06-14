defmodule CodingAdventures.ConduitOtp.MixProject do
  use Mix.Project

  # ---------------------------------------------------------------------------
  # Mix project for `coding_adventures_conduit_otp`
  # ---------------------------------------------------------------------------
  #
  # WEB07 — A pure-OTP Elixir reimplementation of the Conduit web framework.
  # Unlike WEB06 (which uses a Rust NIF for I/O), every byte of this server
  # lives in Elixir/Erlang: gen_tcp sockets, :erlang.decode_packet/3 for HTTP
  # parsing, gen_server-based workers, DynamicSupervisor, and Agent for
  # route storage. No C, no Rust, no external dependencies.
  #
  # The goal is not peak performance — it is to be the most readable OTP web
  # server you'll ever encounter. Every supervisory decision, every
  # gen_server callback, every `:tcp` mode flag is explained inline.

  def project do
    [
      app: :coding_adventures_conduit_otp,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]],
      # Warnings-as-errors OFF: we're writing educational code, and some
      # @spec annotations reference types that only exist at test time.
      elixirc_options: [warnings_as_errors: false]
    ]
  end

  # The OTP `Application` behaviour callback in `otp_application.ex` registers
  # a lightweight top-level supervisor. It starts empty; Server.start_link/2
  # starts the real supervision tree per server instance.
  def application do
    [
      mod: {CodingAdventures.ConduitOtp.OtpApplication, []},
      extra_applications: [:logger, :inets, :ssl]
    ]
  end

  defp deps do
    # Zero external dependencies — pure OTP.
    # :inets provides :httpc (used in tests); :ssl enables TLS (not used yet).
    # Both are bundled with every Erlang/Elixir install.
    []
  end
end
