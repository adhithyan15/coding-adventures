defmodule CodingAdventures.CommonmarkParser.Scanner do
  @moduledoc """
  String Scanner — cursor-based string reader for parsing.

  A cursor-based scanner over a binary string. Used by both the block parser
  (to scan individual lines) and the inline parser (to scan inline content
  character by character).

  ## Design

  The scanner is a struct with a `source` binary and a `pos` integer. All read
  operations return `{result, new_scanner}` — the scanner is immutable. To
  "backtrack", simply keep the original scanner value and discard the new one.

      # Pattern: try-then-backtrack
      saved = scanner
      case try_something(scanner) do
        {:ok, result, scanner2} -> {result, scanner2}
        :error -> {nil, saved}   # backtrack — restore saved pos
      end

  ## Character classification

  GFM cares about several Unicode character categories:
    - ASCII punctuation: !"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~
    - Unicode punctuation (for emphasis rules)
    - ASCII whitespace: space, tab, CR, LF, FF
    - Unicode whitespace

  These classification functions are co-located with the scanner because
  they are used throughout the inline parser.
  """

  defstruct [:source, :pos]

  @type t :: %__MODULE__{source: String.t(), pos: non_neg_integer()}

  @doc """
  Create a new scanner for the given source string, starting at `start` (default 0).
  """
  @spec new(String.t(), non_neg_integer()) :: t()
  def new(source, start \\ 0) do
    %__MODULE__{source: source, pos: start}
  end

  @doc """
  True if the scanner has consumed all input.
  """
  @spec done?(t()) :: boolean()
  def done?(%__MODULE__{source: s, pos: p}), do: p >= byte_size(s)

  @doc """
  Number of bytes remaining.
  """
  @spec remaining(t()) :: non_neg_integer()
  def remaining(%__MODULE__{source: s, pos: p}), do: max(0, byte_size(s) - p)

  @doc """
  Peek at `offset` codepoints ahead without advancing.
  Returns `""` at end of string.
  """
  @spec peek(t(), non_neg_integer()) :: String.t()
  def peek(%__MODULE__{source: s, pos: p}, offset \\ 0) do
    target_pos = advance_by_codepoints(s, p, offset)

    case String.next_codepoint(binary_part_safe(s, target_pos)) do
      {ch, _} -> ch
      nil -> ""
    end
  end

  @doc """
  Peek at the next `n` bytes as a string without advancing.
  """
  @spec peek_slice(t(), non_neg_integer()) :: String.t()
  def peek_slice(%__MODULE__{source: s, pos: p}, n) do
    len = min(n, byte_size(s) - p)
    if len <= 0, do: "", else: binary_part(s, p, len)
  end

  @doc """
  Advance by one codepoint and return `{char, new_scanner}`.
  Returns `{"", scanner}` at end of string.
  """
  @spec advance(t()) :: {String.t(), t()}
  def advance(%__MODULE__{source: s, pos: p} = scanner) do
    case String.next_codepoint(binary_part_safe(s, p)) do
      {ch, _} -> {ch, %{scanner | pos: p + byte_size(ch)}}
      nil -> {"", scanner}
    end
  end

  @doc """
  Advance by `n` bytes (not codepoints) and return the new scanner.
  Used for ASCII-only contexts where byte_size == char count.
  """
  @spec skip(t(), non_neg_integer()) :: t()
  def skip(%__MODULE__{source: s, pos: p} = scanner, n) do
    %{scanner | pos: min(p + n, byte_size(s))}
  end

  @doc """
  If the next bytes exactly match `str`, advance past them and return
  `{true, new_scanner}`. Otherwise return `{false, scanner}` unchanged.
  """
  @spec match(t(), String.t()) :: {boolean(), t()}
  def match(%__MODULE__{source: s, pos: p} = scanner, str) do
    len = byte_size(str)

    if p + len <= byte_size(s) and binary_part(s, p, len) == str do
      {true, %{scanner | pos: p + len}}
    else
      {false, scanner}
    end
  end

  @doc """
  Consume characters while the predicate returns true.
  Returns `{consumed_string, new_scanner}`.
  """
  @spec consume_while(t(), (String.t() -> boolean())) :: {String.t(), t()}
  def consume_while(%__MODULE__{source: s, pos: p} = scanner, pred) do
    consume_while_loop(s, p, p, pred, scanner)
  end

  defp consume_while_loop(s, start, pos, pred, scanner) do
    case String.next_codepoint(binary_part_safe(s, pos)) do
      {ch, _} when true ->
        if pred.(ch) do
          consume_while_loop(s, start, pos + byte_size(ch), pred, scanner)
        else
          {binary_part(s, start, pos - start), %{scanner | pos: pos}}
        end

      _ ->
        {binary_part(s, start, pos - start), %{scanner | pos: pos}}
    end
  end

  @doc """
  Consume the rest of the line (up to but not including the newline).
  Returns `{line_string, new_scanner}`.
  """
  @spec consume_line(t()) :: {String.t(), t()}
  def consume_line(%__MODULE__{source: s, pos: p} = scanner) do
    case :binary.match(s, "\n", scope: {p, byte_size(s) - p}) do
      {nl_pos, _} ->
        {binary_part(s, p, nl_pos - p), %{scanner | pos: nl_pos}}

      :nomatch ->
        {binary_part(s, p, byte_size(s) - p), %{scanner | pos: byte_size(s)}}
    end
  end

  @doc """
  Return the rest of the input from current pos without advancing.
  """
  @spec rest(t()) :: String.t()
  def rest(%__MODULE__{source: s, pos: p}) do
    binary_part_safe(s, p)
  end

  @doc """
  Return a slice of source from `start` to current pos.
  """
  @spec slice_from(t(), non_neg_integer()) :: String.t()
  def slice_from(%__MODULE__{source: s, pos: p}, start) do
    if start >= p, do: "", else: binary_part(s, start, p - start)
  end

  @doc """
  Skip ASCII spaces and tabs. Returns `{count_skipped, new_scanner}`.
  """
  @spec skip_spaces(t()) :: {non_neg_integer(), t()}
  def skip_spaces(%__MODULE__{source: s, pos: p} = scanner) do
    new_pos = skip_spaces_pos(s, p)
    {new_pos - p, %{scanner | pos: new_pos}}
  end

  defp skip_spaces_pos(s, pos) do
    case binary_part_safe(s, pos) do
      <<ch, _::binary>> when ch == ?\s or ch == ?\t ->
        skip_spaces_pos(s, pos + 1)

      _ ->
        pos
    end
  end

  @doc """
  Count leading virtual spaces (expanding tabs to the next 4-column tab stop).

  `base_col` is the virtual column of `source[pos]` in the original document —
  necessary after partial-tab stripping. Returns the number of virtual
  indentation spaces (relative to `base_col`).

  Example: `indentOf("  \\tbar", 2)` — the two spaces are at cols 2-3, the tab
  starts at col 4 and expands to col 8 (adds 4 virtual spaces). Returns 6.
  """
  @spec count_indent(t(), non_neg_integer()) :: non_neg_integer()
  def count_indent(%__MODULE__{source: s, pos: p}, base_col \\ 0) do
    count_indent_loop(s, p, base_col, base_col) - base_col
  end

  defp count_indent_loop(s, pos, col, base_col) do
    case binary_part_safe(s, pos) do
      <<?\s, _::binary>> -> count_indent_loop(s, pos + 1, col + 1, base_col)
      <<?\t, _::binary>> -> count_indent_loop(s, pos + 1, col + (4 - rem(col, 4)), base_col)
      _ -> col
    end
  end

  # ── Character classification ─────────────────────────────────────────────────

  # ASCII punctuation set per GFM spec
  @ascii_punct_chars ~c"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
  @ascii_punct_set MapSet.new(@ascii_punct_chars |> Enum.map(&<<&1::utf8>>))

  @doc """
  True if `ch` is an ASCII punctuation character (GFM definition).
  These are: ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \\ ] ^ _ ` { | } ~
  """
  @spec ascii_punctuation?(String.t()) :: boolean()
  def ascii_punctuation?(ch), do: MapSet.member?(@ascii_punct_set, ch)

  @doc """
  True if `ch` is a Unicode punctuation character for GFM flanking.

  GFM defines this as any ASCII punctuation character OR any character
  in Unicode categories: Pc, Pd, Pe, Pf, Pi, Po, Ps (punctuation) or
  Sm, Sc, Sk, So (symbols).

  Symbol categories are included because cmark treats them as punctuation
  for delimiter flanking (e.g. £ U+00A3 Sc, € U+20AC Sc).
  """
  @spec unicode_punctuation?(String.t()) :: boolean()
  def unicode_punctuation?(""), do: false

  def unicode_punctuation?(ch) do
    if MapSet.member?(@ascii_punct_set, ch) do
      true
    else
      # Check Unicode punctuation/symbol categories via regex
      Regex.match?(~r/^\p{P}$/u, ch) or Regex.match?(~r/^\p{S}$/u, ch)
    end
  end

  @doc """
  True if `ch` is ASCII whitespace: space (0x20), tab (0x09),
  newline (0x0A), form feed (0x0C), carriage return (0x0D).
  """
  @spec ascii_whitespace?(String.t()) :: boolean()
  def ascii_whitespace?(ch),
    do: ch == " " or ch == "\t" or ch == "\n" or ch == "\r" or ch == "\f"

  @doc """
  True if `ch` is Unicode whitespace (any code point with Unicode
  property White_Space=yes).
  """
  @spec unicode_whitespace?(String.t()) :: boolean()
  def unicode_whitespace?(""), do: false

  def unicode_whitespace?(ch) do
    # Use regex for Unicode whitespace detection (includes non-ASCII spaces)
    Regex.match?(~r/^\s$/u, ch) or ch in [
      "\u00A0",
      "\u1680",
      "\u202F",
      "\u205F",
      "\u3000"
    ] or
      (ch >= "\u2000" and ch <= "\u200A")
  end

  @doc """
  True if `ch` is an ASCII digit (0-9).
  """
  @spec digit?(String.t()) :: boolean()
  def digit?(ch), do: ch >= "0" and ch <= "9"

  # ── Link label normalization ──────────────────────────────────────────────────

  @doc """
  Normalize a link label per GFM:
    - Strip leading and trailing whitespace
    - Collapse internal whitespace runs to a single space
    - Fold to lowercase

  Two labels are equivalent if their normalized forms are equal.

  Note: `ß` (U+00DF) and `ẞ` (U+1E9E) both fold to "ss" in Unicode Full
  Case Folding. Elixir's `String.downcase/1` handles this via Unicode ICU.
  """
  @spec normalize_link_label(String.t()) :: String.t()
  def normalize_link_label(label) do
    label
    |> String.trim()
    |> String.replace(~r/\s+/, " ")
    |> String.downcase()
    |> String.replace("ß", "ss")
  end

  @doc """
  Normalize a URL: percent-encode spaces and characters that should not
  appear unencoded in HTML href/src attributes.
  """
  @spec normalize_url(String.t()) :: String.t()
  def normalize_url(url) do
    # Percent-encode characters that are not in the "safe" ASCII URL set.
    # We use a character-by-character approach rather than a regex because
    # the `\w` Unicode regex class would incorrectly match non-ASCII letters
    # (e.g. `ö` matches `\w` with the `u` flag) and skip encoding them.
    # The safe set is: ASCII alphanumerics + `-._~:/?#@!$&'()*+,;=%`
    # Everything else — including non-ASCII Unicode — must be percent-encoded.
    url
    |> String.graphemes()
    |> Enum.map_join(fn ch ->
      cond do
        # Already-percent-encoded sequences stay as-is
        Regex.match?(~r/^[A-Za-z0-9\-._~:\/?#@!$&'()*+,;=%]$/, ch) -> ch
        true -> percent_encode(ch)
      end
    end)
  end

  defp percent_encode(ch) do
    ch
    |> :binary.bin_to_list()
    |> Enum.map_join(fn byte -> "%" <> String.upcase(:io_lib.format("~2.16.0B", [byte]) |> IO.iodata_to_binary()) end)
  end

  # ── Private helpers ──────────────────────────────────────────────────────────

  # Safely get a sub-binary from position `pos` to end of string
  defp binary_part_safe(s, pos) when pos >= byte_size(s), do: ""
  defp binary_part_safe(s, pos), do: binary_part(s, pos, byte_size(s) - pos)

  # Advance `n` codepoints from `start_pos`, returning the resulting byte offset
  defp advance_by_codepoints(_, pos, 0), do: pos

  defp advance_by_codepoints(s, pos, n) do
    case String.next_codepoint(binary_part_safe(s, pos)) do
      {ch, _} -> advance_by_codepoints(s, pos + byte_size(ch), n - 1)
      nil -> pos
    end
  end
end
