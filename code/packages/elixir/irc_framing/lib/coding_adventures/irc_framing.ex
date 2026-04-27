defmodule CodingAdventures.IrcFraming do
  @moduledoc """
  IRC line framing — reassembles a TCP byte stream into complete IRC lines.

  This package sits between the raw TCP socket layer and the IRC protocol
  parser. Its job is to buffer incoming bytes and deliver complete CRLF-
  terminated lines.

  ## Responsibility

  IRC uses CRLF-terminated text lines. TCP delivers an arbitrary byte stream.
  This package bridges the gap with a purely functional `Framer` struct.

  ## API

  All functions delegate to `CodingAdventures.IrcFraming.Framer`:

  - `new/0`          -- create a fresh Framer
  - `feed/2`         -- append raw bytes to the buffer
  - `frames/1`       -- extract complete lines; returns `{framer, lines}`
  - `reset/1`        -- clear the buffer
  - `buffer_size/1`  -- current buffer size in bytes
  """

  alias CodingAdventures.IrcFraming.Framer

  @doc "Create a new, empty Framer."
  defdelegate new(), to: Framer

  @doc "Append *bytes* to the framer's buffer."
  defdelegate feed(framer, bytes), to: Framer

  @doc "Extract all complete lines. Returns `{updated_framer, lines}`."
  defdelegate frames(framer), to: Framer

  @doc "Clear the framer's buffer."
  defdelegate reset(framer), to: Framer

  @doc "Return the current buffer size in bytes."
  defdelegate buffer_size(framer), to: Framer
end
