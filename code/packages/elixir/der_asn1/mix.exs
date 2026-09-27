defmodule CodingAdventures.DerAsn1.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_der_asn1,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 95]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_der_tlv, path: "../der_tlv"},
      {:jason, "~> 1.4", only: :test}
    ]
  end
end
