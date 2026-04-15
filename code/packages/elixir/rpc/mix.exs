defmodule CodingAdventures.Rpc.MixProject do
  use Mix.Project

  # ---------------------------------------------------------------------------
  # Project configuration
  # ---------------------------------------------------------------------------
  #
  # The `rpc` package is the abstract RPC primitive layer. It sits below any
  # concrete codec (JSON, MessagePack, Protobuf) and below any framing scheme
  # (Content-Length, length-prefix, newline-delimited). Its only job is to
  # define the RPC *concepts*: what a message looks like, how to dispatch method
  # calls, how to correlate request ids, and how to handle errors.
  #
  # No Hex dependencies are needed — stdlib and OTP alone are sufficient.

  def project do
    [
      app: :coding_adventures_rpc,
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
    # No external dependencies — stdlib/OTP only.
    # The RPC primitive must not pull in JSON, Protobuf, or any codec library.
    # Concrete codec packages (json_rpc, msgpack_rpc, …) depend on rpc, not
    # the other way around.
    []
  end
end
