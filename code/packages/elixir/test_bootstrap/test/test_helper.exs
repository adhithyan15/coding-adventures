ExUnit.start(timeout: 120_000)

defmodule Fx do
  @moduledoc "Shared helpers: the COMMITTED pbt-v1 fixture is read by walking up, never copied."

  alias CodingAdventures.TestBootstrap.Pbt.Corpus

  def repo_root do
    walk(Path.expand(__DIR__))
  end

  defp walk(dir) do
    cond do
      File.regular?(Path.join(dir, "code/specs/fixtures/pbt-v1/cases.json")) -> dir
      Path.dirname(dir) == dir -> raise "could not find code/specs/fixtures/pbt-v1/cases.json above #{__DIR__}"
      true -> walk(Path.dirname(dir))
    end
  end

  def fixture_path(name \\ "cases.json"), do: Path.join([repo_root(), "code/specs/fixtures/pbt-v1", name])
  def raw, do: File.read!(fixture_path())
  # A fresh, independent parse of the committed corpus.
  def cases, do: Corpus.parse(raw())

  def int(n), do: %{"kind" => "int", "value" => Integer.to_string(n)}
end
