defmodule CodingAdventures.JsonRpc.MixProject do
  use Mix.Project

  # ---------------------------------------------------------------------------
  # Project configuration
  # ---------------------------------------------------------------------------
  #
  # This package is deliberately dependency-free. JSON-RPC 2.0 is a transport
  # protocol — it should not pull in any external JSON parsing library. Instead
  # we use Erlang/OTP's built-in `:json` module (available since OTP 27), which
  # provides `encode/1` and `decode/1` functions for JSON text over binaries.
  #
  # If `:json` is not available (OTP < 27), the package falls back to a minimal
  # hand-written encoder/decoder sufficient for JSON-RPC messages.

  def project do
    [
      app: :coding_adventures_json_rpc,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    # No dependencies — stdlib only (no Jason, no Poison, no external JSON lib).
    # JSON encoding/decoding is handled by the internal JsonCodec module which
    # uses OTP 27's :json module when available.
    []
  end
end
