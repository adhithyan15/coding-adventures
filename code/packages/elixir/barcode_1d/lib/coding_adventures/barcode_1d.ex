defmodule CodingAdventures.Barcode1D do
  @moduledoc """
  High-level 1D barcode pipeline for Elixir.

  This package keeps the layers separated:
  - barcode package (`CodingAdventures.Code39`) owns encoding
  - `barcode_layout_1d` owns geometry
  - native Paint VM packages own pixels and image encoding
  """

  alias CodingAdventures.{Code39, PaintCodecPngNative, PaintVmMetalNative}

  @type symbology :: :code39 | String.t()
  @type render_error ::
          :unsupported_symbology
          | :metal_backend_unavailable
          | :metal_requires_apple_silicon
          | :direct2d_not_implemented
          | :cairo_not_implemented
          | :unsupported_os
          | :nif_not_available
          | atom()

  @doc """
  Resolve the current native barcode backend for this host OS.
  """
  @spec current_backend() :: {:ok, :metal} | {:error, render_error()}
  def current_backend do
    current_backend(:os.type(), :erlang.system_info(:system_architecture) |> to_string())
  end

  @doc false
  @spec current_backend(tuple(), String.t()) :: {:ok, :metal} | {:error, render_error()}
  def current_backend(os_type, architecture) do
    case os_type do
      {:unix, :darwin} ->
        if String.contains?(architecture, "arm64") or String.contains?(architecture, "aarch64") do
          {:ok, :metal}
        else
          {:error, :metal_requires_apple_silicon}
        end

      {:win32, _} ->
        {:error, :direct2d_not_implemented}

      {:unix, :linux} ->
        {:error, :cairo_not_implemented}

      _ ->
        {:error, :unsupported_os}
    end
  end

  @doc """
  Build a `PaintScene` for the requested 1D symbology.
  """
  @spec build_scene(String.t(), keyword()) :: {:ok, map()} | {:error, render_error()}
  def build_scene(data, opts \\ [])

  def build_scene(data, opts) when is_binary(data) do
    symbology = Keyword.get(opts, :symbology, :code39)
    layout_config = Keyword.get(opts, :layout_config, Code39.default_layout_config())

    case normalize_symbology(symbology) do
      :code39 -> {:ok, Code39.layout_code39(data, layout_config)}
      :unsupported -> {:error, :unsupported_symbology}
    end
  end

  def build_scene(_data, _opts), do: {:error, :unsupported_symbology}

  @doc """
  Render barcode text to a pixel container through the native Paint VM.
  """
  @spec render_pixels(String.t(), keyword()) ::
          {:ok, CodingAdventures.PixelContainer.t()} | {:error, render_error()}
  def render_pixels(data, opts \\ []) do
    with {:ok, scene} <- build_scene(data, opts),
         {:ok, backend} <- current_backend(),
         {:ok, pixels} <- execute_scene(scene, backend) do
      {:ok, pixels}
    end
  end

  @doc """
  Render barcode text all the way to PNG bytes.
  """
  @spec render_png(String.t(), keyword()) :: {:ok, binary()} | {:error, render_error()}
  def render_png(data, opts \\ []) do
    with {:ok, pixels} <- render_pixels(data, opts),
         {:ok, png} <- PaintCodecPngNative.encode(pixels) do
      {:ok, png}
    end
  end

  defp execute_scene(scene, :metal), do: PaintVmMetalNative.render(scene)

  defp normalize_symbology(:code39), do: :code39
  defp normalize_symbology("code39"), do: :code39
  defp normalize_symbology(_symbology), do: :unsupported
end
