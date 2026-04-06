defmodule CodingAdventures.ImageCodec do
  @moduledoc """
  Behaviour for image codecs.

  A codec (coder/decoder) is a module that knows how to convert between a
  `CodingAdventures.PixelContainer` and a particular on-disk binary format
  (BMP, PPM, QOI, PNG, etc.).

  Any module that `@behaviour CodingAdventures.ImageCodec` must implement
  three callbacks:

  - `mime_type/0` — the IANA media type string, e.g. `"image/bmp"`
  - `encode/1`    — serialize a pixel container into a binary
  - `decode/1`    — parse a binary back into a pixel container

  ## Why a behaviour?

  Elixir behaviours are like interfaces in Java or protocols in Swift. They
  let you write code that works with *any* codec without knowing which one
  you're using at compile time. For example:

      def save(container, codec) do
        data = codec.encode(container)
        File.write!("output.img", data)
      end

  Pass `CodingAdventures.ImageCodecBmp` or `CodingAdventures.ImageCodecPpm`
  and it works the same way.
  """

  @doc "Returns the IANA MIME type string for this format, e.g. \"image/bmp\"."
  @callback mime_type() :: String.t()

  @doc """
  Encodes the pixel container into a binary blob in this codec's format.

  Returns a `binary()` suitable for writing directly to a file.
  """
  @callback encode(CodingAdventures.PixelContainer.t()) :: binary()

  @doc """
  Decodes a binary blob into a pixel container.

  Returns `{:ok, container}` on success, or `{:error, reason}` if the data
  is malformed, truncated, or otherwise invalid.
  """
  @callback decode(binary()) ::
              {:ok, CodingAdventures.PixelContainer.t()} | {:error, String.t()}
end
