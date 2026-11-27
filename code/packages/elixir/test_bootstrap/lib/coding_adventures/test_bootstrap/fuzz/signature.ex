defmodule CodingAdventures.TestBootstrap.Fuzz.Signature do
  @moduledoc """
  Crash signatures, kind and frame normalisation and the one public failure line
  (VER06 sections 5.2 and 6, VER06n sections 5.2 and 7).

  Nothing in here reads an exception MESSAGE and nothing calls `inspect`,
  `to_string` or any protocol on a value the target supplied: a kind comes from the
  error class and the exception's module; frames come from the stack trace as
  `Module.function/arity`.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.Prng
  alias CodingAdventures.TestBootstrap.Fuzz.{FailureIds, Limits}

  # Frames of these modules are plumbing, never "the target".
  @plumbing ["Elixir.CodingAdventures.TestBootstrap.Fuzz", "Elixir.CodingAdventures.TestBootstrap.Safe"]

  # The closed set of Erlang error tags a kind may carry (VER06n section 5.2): a
  # target cannot mint arbitrary kind text through `:erlang.error(:anything)`.
  @erlang_tags ~w(badarg badarith badmatch function_clause case_clause if_clause try_clause cond_clause undef badfun badarity badkey badmap badrecord noproc system_limit else_clause with_clause)a

  @doc """
  The low 32 bits of `fnv1a64(kind <> ":" <> frames joined by ":")`, 8 lowercase hex.

  Frames are NOT escaped (VER06 section 9.4 item 6): one frame `"x:y"` and two frames
  `"x"`, `"y"` hash alike.  This lane never produces a frame with a colon
  (`normalise_frame/1`), so the ambiguity cannot arise from Elixir targets.
  """
  def crash_signature(kind, frames) when is_binary(kind) and is_list(frames) do
    unless Enum.all?(frames, &is_binary/1),
      do: raise(ArgumentError, "frames must be a list of binaries")

    kind
    |> Kernel.<>(":" <> Enum.join(frames, ":"))
    |> Prng.fnv1a64()
    |> band(0xFFFF_FFFF)
    |> Integer.to_string(16)
    |> String.downcase()
    |> String.pad_leading(8, "0")
  end

  def crash_signature(_kind, _frames),
    do: raise(ArgumentError, "kind must be a binary and frames a list of binaries")

  def duplicate_crash?(kind_a, frames_a, kind_b, frames_b),
    do: crash_signature(kind_a, frames_a) == crash_signature(kind_b, frames_b)

  # -- the public failure line ----------------------------------------------------

  @doc """
  The only public failure line: `not ok N - token [failure-id:hex8]`.  It carries no
  input bytes, seed, message or stack trace (VER00 7.3).

  The target is reduced to its grammar exactly as VER06 section 9.3 and every other
  lane do: truncated to 128 BYTES first, then every BYTE outside `[A-Za-z0-9._/-]`
  (a control character, a space, `#`, a newline, each byte of a multi-byte
  character) becomes `_`.  Nothing can add a TAP line, hide one, or start a `#`
  directive.
  """
  def format_tap_failure(test_num, target, failure_id, sig_hash) do
    Limits.check_range!(test_num, 1, 2_147_483_647, "test_num")
    unless is_binary(target), do: raise(ArgumentError, "target must be a binary")

    unless FailureIds.valid?(failure_id),
      do: raise(ArgumentError, "failure id is not one of the registered fuzz failure ids")

    unless is_binary(sig_hash) and Regex.match?(~r/\A[0-9a-f]{8}\z/, sig_hash),
      do: raise(ArgumentError, "sig hash must be exactly 8 lowercase hex digits")

    "not ok #{test_num} - #{token(target, Limits.max_target_len())} [#{failure_id}:#{sig_hash}]"
  end

  @doc "Truncates to `limit` bytes BEFORE reducing, then maps every byte outside `[A-Za-z0-9._/-]` to `_`."
  def token(text, limit) when is_binary(text) do
    text
    |> binary_part(0, min(byte_size(text), limit))
    |> then(&Regex.replace(~r/[^A-Za-z0-9._\/-]/, &1, "_"))
  end

  @doc """
  A report as public, opaque failure lines (none for a clean one): one per crash
  signature, then one for timeouts and one for ooms, whose hash is the signature of
  the bare kind (they have no stack to hash).
  """
  def tap_failures(report, target, first_test \\ 1) do
    crash_lines =
      report.crashes
      |> Enum.with_index()
      |> Enum.map(fn {rec, i} -> format_tap_failure(first_test + i, target, FailureIds.crash(), rec.signature) end)

    Enum.reduce(
      [{report.timeouts, FailureIds.timeout(), "timeout"}, {report.ooms, FailureIds.oom(), "oom"}],
      crash_lines,
      fn
        {count, id, kind}, lines when count > 0 ->
          lines ++ [format_tap_failure(first_test + length(lines), target, id, crash_signature(kind, []))]

        _, lines ->
          lines
      end
    )
  end

  # -- kinds and frames --------------------------------------------------------------

  @doc "Reduces text to `[A-Za-z0-9._<>$/#-]`; every other BYTE becomes `_` (no `:` ever)."
  def sanitize(text, limit) do
    text
    |> binary_part(0, min(byte_size(text), limit))
    |> then(&Regex.replace(~r/[^A-Za-z0-9._<>$\/#-]/, &1, "_"))
  end

  defp module_name(mod) do
    name = Atom.to_string(mod)
    if String.starts_with?(name, "Elixir."), do: binary_part(name, 7, byte_size(name) - 7), else: name
  end

  @doc """
  The crash kind of a caught `kind, reason`.  Reads the exception's MODULE and
  nothing else: not its message, not any field.
  """
  def kind_of(:error, %{__exception__: true, __struct__: mod}) when is_atom(mod),
    do: "error." <> sanitize(module_name(mod), Limits.max_kind_len())

  def kind_of(:error, reason), do: "error." <> erlang_tag(reason)
  def kind_of(:throw, _reason), do: "throw"
  def kind_of(:exit, _reason), do: "exit"
  def kind_of(_other, _reason), do: "<unreadable>"

  defp erlang_tag(tag) when is_atom(tag), do: if(tag in @erlang_tags, do: Atom.to_string(tag), else: "other")

  defp erlang_tag(reason) when is_tuple(reason) and tuple_size(reason) > 0 do
    case elem(reason, 0) do
      tag when is_atom(tag) -> erlang_tag(tag)
      _ -> "other"
    end
  end

  defp erlang_tag(_), do: "other"

  @doc """
  Frames (innermost first) of a stack trace: `Module.function/arity`, no file, no
  line, no argument value.  Engine and `Safe` frames are dropped, consecutive
  duplicates collapse (a recursive crash hashes alike at any depth), the innermost
  64 survive, and at most `max_walk` entries are inspected.  Total: a malformed
  trace gives `[]`.
  """
  def frames_of(stack) when is_list(stack) do
    stack
    |> Enum.take(Limits.max_walk())
    |> Enum.flat_map(&frame/1)
    |> collapse()
  end

  def frames_of(_), do: []

  defp frame({mod, fun, arity, _loc}) when is_atom(mod) and is_atom(fun) do
    name = Atom.to_string(mod)

    if Enum.any?(@plumbing, &String.starts_with?(name, &1)) do
      []
    else
      [normalise_frame(module_name(mod) <> "." <> Atom.to_string(fun) <> "/" <> arity_text(arity))]
    end
  end

  defp frame(_), do: []

  defp arity_text(a) when is_integer(a), do: Integer.to_string(a)
  defp arity_text(a) when is_list(a), do: Integer.to_string(length(a))
  defp arity_text(_), do: "?"

  @doc "Every character outside `[A-Za-z0-9._<>$/#-]` becomes `_`, so no frame contains `:`."
  def normalise_frame(text), do: sanitize(text, 256)

  @doc "Drops consecutive duplicates and keeps the innermost `max_frames`."
  def collapse(frames) do
    frames
    |> Enum.chunk_by(& &1)
    |> Enum.map(&hd/1)
    |> Enum.take(Limits.max_frames())
  end
end
