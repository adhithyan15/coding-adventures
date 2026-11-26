defmodule CodingAdventures.TestBootstrap.Safe do
  @moduledoc """
  Classification of whatever user code does (VER05m 3).

  A property is user code and on the BEAM it can fail three ways: `raise`
  (kind `:error`), `throw` and `exit`.  `call/1` catches all three WITHOUT
  inspecting the value to decide: every kind is data the engine reports
  (`property-panic`), never a crash of the run.  Not catchable, by design: a
  brutal kill or a heap limit ends the process.
  """
  alias CodingAdventures.TestBootstrap.CorpusJson.{Canon, Clip}

  @doc "Runs `fun`; `{:ok, value}` or `{:raised, kind, reason}` for error, throw and exit."
  def call(fun) do
    {:ok, fun.()}
  catch
    kind, reason -> {:raised, kind, reason}
  end

  @doc """
  Like `call/1`, and also returns the stack trace of the failure (a bounded list
  of `{module, function, arity_or_args, location}` entries, never rendered here).
  The fuzz engine derives crash signatures from it (VER06n section 5.2).
  """
  def call_stacked(fun) do
    {:ok, fun.()}
  catch
    kind, reason -> {:raised, kind, reason, __STACKTRACE__}
  end

  @doc """
  The kind (and, for an exception struct, its module) for redacted (CI)
  output.  Reads no user code and never renders the value.
  """
  def redacted(kind, reason) do
    case {kind, reason} do
      {:error, %{__struct__: mod}} when is_atom(mod) ->
        "error " <> sanitize(String.replace_prefix(Atom.to_string(mod), "Elixir.", ""))

      {k, _} ->
        Atom.to_string(k)
    end
  end

  @doc "Kind plus the value rendered structurally (never `inspect`): total, bounded, escaped."
  def local(kind, reason),
    do: Clip.clip(Atom.to_string(kind) <> ": " <> Canon.render(reason), 400)

  def sanitize(text), do: text |> Clip.clip(200) |> Canon.escape()
end
