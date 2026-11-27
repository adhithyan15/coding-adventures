defmodule CodingAdventures.TestBootstrap.CorpusJson.Clip do
  @moduledoc "Bounded fragments for reports: fixture-derived text must never make a problem line unboundedly large."

  def clip(text, limit) when is_binary(text) do
    if String.length(text) > limit, do: String.slice(text, 0, limit) <> "...", else: text
  end

  def clip(other, limit), do: clip(to_string_safe(other), limit)

  defp to_string_safe(v) when is_binary(v), do: v
  defp to_string_safe(v) when is_integer(v), do: Integer.to_string(v)
  defp to_string_safe(_), do: "<term>"
end
