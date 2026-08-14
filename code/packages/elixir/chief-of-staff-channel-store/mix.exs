defmodule CodingAdventures.ChiefOfStaffChannelStore.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_chief_of_staff_channel_store,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application, do: [extra_applications: [:logger]]

  defp deps do
    [
      {:coding_adventures_chief_of_staff_channel_crypto,
       path: "../chief-of-staff-channel-crypto"},
      {:coding_adventures_sha256, path: "../sha256"},
      {:coding_adventures_ed25519, path: "../ed25519"},
      {:jason, "~> 1.4"}
    ]
  end
end
