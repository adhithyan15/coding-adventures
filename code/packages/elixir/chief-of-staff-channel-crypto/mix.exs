defmodule CodingAdventures.ChiefOfStaffChannelCrypto.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_chief_of_staff_channel_crypto,
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
    [
      {:coding_adventures_chacha20_poly1305, path: "../chacha20-poly1305"},
      {:coding_adventures_ed25519, path: "../ed25519"},
      {:coding_adventures_sha256, path: "../sha256"},
      {:jason, "~> 1.4"}
    ]
  end
end
