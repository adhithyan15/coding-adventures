defmodule Ls00.DocumentManager do
  @moduledoc """
  Tracks all files currently open in the editor.

  ## The Document Manager's Job

  When the user opens a file in VS Code, the editor sends a textDocument/didOpen
  notification with the full file content. From that point on, the editor does
  NOT re-send the entire file on every keystroke. Instead, it sends incremental
  changes: what changed, and where. The DocumentManager applies these changes to
  maintain the current text of each open file.

      Editor opens file:   didOpen   -> DocumentManager stores text at version 1
      User types "X":      didChange -> DocumentManager applies delta -> version 2
      User saves:          didSave   -> (optional: trigger format)
      User closes:         didClose  -> DocumentManager removes entry

  ## Why Version Numbers?

  The editor increments the version number with every change. The ParseCache
  uses (uri, version) as its cache key -- if the version matches, the cached
  parse result is still valid. This avoids re-parsing the file on every
  keystroke when the user is just moving the cursor.

  ## UTF-16: The Tricky Part

  LSP specifies that character offsets are measured in UTF-16 CODE UNITS.
  This is a historical accident: VS Code is built on TypeScript, which uses
  UTF-16 strings internally (like Java and C#). Since LSP was designed for
  VS Code, it inherited this convention.

  Elixir strings are UTF-8. A single Unicode codepoint can occupy:
    - 1 byte in UTF-8 (ASCII, e.g. 'A')
    - 2 bytes in UTF-8 (e.g. 'e', U+00E9)
    - 3 bytes in UTF-8 (e.g. the Chinese character for 'middle', U+4E2D)
    - 4 bytes in UTF-8 (e.g. guitar emoji, U+1F3B8)

  In UTF-16:
    - Codepoints in the Basic Multilingual Plane (U+0000-U+FFFF) -> 1 code unit
    - Codepoints above U+FFFF (emojis, rare CJK) -> 2 code units (a "surrogate pair")

  The guitar emoji (U+1F3B8) is above U+FFFF:
    UTF-8:  4 bytes  (0xF0 0x9F 0x8E 0xB8)
    UTF-16: 2 code units (surrogate pair)

  So if the LSP client says character=8 (UTF-16), we cannot simply slice 8 bytes
  into the UTF-8 Elixir string. We must walk the UTF-8 bytes, converting each
  codepoint to its UTF-16 length, accumulating until we reach code unit 8.
  """

  alias Ls00.Types.{Document, TextChange}

  # ---------------------------------------------------------------------------
  # Data structure
  # ---------------------------------------------------------------------------
  #
  # We store documents as a map of uri -> Document struct. This is NOT a
  # GenServer -- it is a plain data structure. The LspServer owns it and passes
  # it through function calls. This keeps the design simple and testable.

  @type t :: %{String.t() => Document.t()}

  @doc """
  Create an empty DocumentManager (just an empty map).
  """
  @spec new() :: t()
  def new, do: %{}

  @doc """
  Record a newly opened file.

  Called when the editor sends textDocument/didOpen. Stores the initial text
  and version number (typically 1 for a freshly opened file).
  """
  @spec open(t(), String.t(), String.t(), integer()) :: t()
  def open(docs, uri, text, version) do
    Map.put(docs, uri, %Document{uri: uri, text: text, version: version})
  end

  @doc """
  Apply a list of incremental changes to an open document.

  Changes are applied in order. If a range is nil, the change replaces the
  entire document. After all changes, the document's version is updated.

  Returns `{:ok, updated_docs}` or `{:error, reason}`.
  """
  @spec apply_changes(t(), String.t(), [TextChange.t()], integer()) ::
          {:ok, t()} | {:error, String.t()}
  def apply_changes(docs, uri, changes, version) do
    case Map.fetch(docs, uri) do
      :error ->
        {:error, "document not open: #{uri}"}

      {:ok, %Document{} = doc} ->
        case apply_changes_to_text(doc.text, changes) do
          {:ok, new_text} ->
            updated = %Document{doc | text: new_text, version: version}
            {:ok, Map.put(docs, uri, updated)}

          {:error, _} = err ->
            err
        end
    end
  end

  @doc """
  Get the document for a URI.

  Returns `{:ok, document}` or `:error` if the document is not open.
  """
  @spec get(t(), String.t()) :: {:ok, Document.t()} | :error
  def get(docs, uri) do
    Map.fetch(docs, uri)
  end

  @doc """
  Remove a document from the manager.

  Called when the editor sends textDocument/didClose.
  """
  @spec close(t(), String.t()) :: t()
  def close(docs, uri) do
    Map.delete(docs, uri)
  end

  # ---------------------------------------------------------------------------
  # UTF-16 offset conversion
  # ---------------------------------------------------------------------------

  @doc """
  Convert a 0-based (line, UTF-16 character) position to a byte offset in a
  UTF-8 Elixir string.

  This is the most critical function in the entire package. If this function is
  wrong, every feature that depends on cursor position will be wrong: hover,
  go-to-definition, references, completion, rename, signature help.

  ## Why UTF-16?

  LSP character offsets are UTF-16 code units because VS Code's internal string
  representation is UTF-16 (as is JavaScript's String type). This function
  bridges the gap to Elixir's UTF-8 strings.

  ## Algorithm

  1. Walk line-by-line to find the byte offset of the start of the target line.
  2. From that offset, walk UTF-8 codepoints, converting each to its UTF-16
     length, until we reach the target UTF-16 character offset.

  ## Example

      iex> text = "hello \\xF0\\x9F\\x8E\\xB8 world"
      iex> # guitar emoji (U+1F3B8) is 4 UTF-8 bytes but 2 UTF-16 code units
      iex> convert_utf16_offset_to_byte_offset(text, 0, 8)
      11
  """
  @spec convert_utf16_offset_to_byte_offset(String.t(), non_neg_integer(), non_neg_integer()) ::
          non_neg_integer()
  def convert_utf16_offset_to_byte_offset(text, line, char) do
    # Phase 1: find the byte offset of the start of the target line.
    line_start = find_line_start(text, line, 0, 0)

    # Phase 2: from line_start, advance `char` UTF-16 code units.
    advance_utf16_units(text, line_start, char, 0)
  end

  # Find the byte offset where line `target_line` begins.
  # We scan through the binary looking for newline characters.
  defp find_line_start(_text, 0, _current_line, byte_offset), do: byte_offset

  defp find_line_start(text, target_line, current_line, byte_offset) do
    remaining = binary_part_safe(text, byte_offset)

    case remaining do
      "" ->
        # Past end of text -- clamp to end.
        byte_size(text)

      <<?\n, _rest::binary>> ->
        # Found a newline. Advance to the next line.
        find_line_start(text, target_line - 1, current_line + 1, byte_offset + 1)

      _ ->
        # Not a newline -- advance one byte in the binary. But we need to
        # advance by the full codepoint width to not split multi-byte characters.
        <<_codepoint::utf8, _rest::binary>> = remaining
        cp_bytes = codepoint_byte_size(remaining)
        find_line_start(text, target_line, current_line, byte_offset + cp_bytes)
    end
  end

  # Advance `target_units` UTF-16 code units from `byte_offset`.
  # Returns the final byte offset.
  defp advance_utf16_units(_text, byte_offset, target_units, accumulated)
       when accumulated >= target_units,
       do: byte_offset

  defp advance_utf16_units(text, byte_offset, target_units, accumulated) do
    remaining = binary_part_safe(text, byte_offset)

    case remaining do
      "" ->
        # Past end of text -- clamp.
        byte_size(text)

      <<?\n, _rest::binary>> ->
        # Don't advance past the newline -- the position is beyond the line end.
        byte_offset

      _ ->
        <<codepoint::utf8, _rest::binary>> = remaining
        cp_bytes = codepoint_byte_size(remaining)
        utf16_len = utf16_unit_length(codepoint)

        # Check if adding this codepoint would overshoot the target.
        if accumulated + utf16_len > target_units do
          # Would overshoot (e.g., in the middle of a surrogate pair).
          byte_offset
        else
          advance_utf16_units(text, byte_offset + cp_bytes, target_units, accumulated + utf16_len)
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Apply a list of changes to a text string, in order.
  defp apply_changes_to_text(text, []), do: {:ok, text}

  defp apply_changes_to_text(_text, [%TextChange{range: nil, new_text: new_text} | rest]) do
    # Full document replacement -- simplest case.
    apply_changes_to_text(new_text, rest)
  end

  defp apply_changes_to_text(text, [%TextChange{range: range, new_text: new_text} | rest]) do
    start_byte = convert_utf16_offset_to_byte_offset(text, range.start.line, range.start.character)
    end_byte = convert_utf16_offset_to_byte_offset(text, range.end_pos.line, range.end_pos.character)

    # Clamp end_byte to text length.
    end_byte = min(end_byte, byte_size(text))

    if start_byte > end_byte do
      {:error, "start offset #{start_byte} > end offset #{end_byte}"}
    else
      prefix = binary_part(text, 0, start_byte)
      suffix = binary_part(text, end_byte, byte_size(text) - end_byte)
      new_text_full = prefix <> new_text <> suffix
      apply_changes_to_text(new_text_full, rest)
    end
  end

  # How many UTF-16 code units does this Unicode codepoint require?
  #
  # - Codepoints in the BMP (U+0000-U+FFFF): 1 code unit
  # - Codepoints above U+FFFF (emoji, etc.): 2 code units (surrogate pair)
  @doc false
  def utf16_unit_length(codepoint) when codepoint > 0xFFFF, do: 2
  def utf16_unit_length(_codepoint), do: 1

  # Get the byte size of the first codepoint in a binary.
  defp codepoint_byte_size(<<cp::utf8, rest::binary>>) do
    total = byte_size(<<cp::utf8, rest::binary>>)
    total - byte_size(rest)
  end

  # Safely get a sub-binary from byte_offset to end.
  defp binary_part_safe(text, offset) when offset >= byte_size(text), do: ""
  defp binary_part_safe(text, offset), do: binary_part(text, offset, byte_size(text) - offset)
end
