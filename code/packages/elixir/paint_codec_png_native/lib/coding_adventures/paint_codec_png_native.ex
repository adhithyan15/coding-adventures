defmodule CodingAdventures.PaintCodecPngNative do
  @moduledoc """
  Native PNG codec bridge for `CodingAdventures.PixelContainer`.
  """

  alias CodingAdventures.PixelContainer

  @on_load :load_nif

  @doc false
  def load_nif do
    if nif_file_exists?() do
      case :erlang.load_nif(String.to_charlist(nif_base_path()), 0) do
        :ok -> :ok
        {:error, _reason} -> :ok
      end
    else
      :ok
    end
  end

  @doc """
  Returns true when the PNG codec NIF is present for this package.
  """
  def available? do
    nif_file_exists?()
  end

  @doc """
  Encode a pixel container as PNG bytes.
  """
  @spec encode(PixelContainer.t()) :: {:ok, binary()} | {:error, atom()}
  def encode(%PixelContainer{width: width, height: height, data: data})
      when is_integer(width) and is_integer(height) and is_binary(data) do
    cond do
      not nif_file_exists?() ->
        {:error, :nif_not_available}

      true ->
        encode_rgba8_native(width, height, data)
    end
  rescue
    ErlangError ->
      {:error, :nif_not_available}
  end

  def encode(_pixels), do: {:error, :invalid_pixel_container}

  defp nif_base_path do
    case :code.priv_dir(:coding_adventures_paint_codec_png_native) do
      {:error, _reason} -> nil
      priv_dir -> Path.join(to_string(priv_dir), "paint_codec_png_native")
    end
  end

  defp nif_file_exists? do
    case nif_base_path() do
      nil -> false
      base_path -> File.exists?(base_path <> ".so")
    end
  end

  defp encode_rgba8_native(_width, _height, _data), do: :erlang.nif_error(:not_loaded)
end
