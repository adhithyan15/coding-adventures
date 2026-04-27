defmodule CodingAdventures.Barcode1D do
  @moduledoc """
  High-level 1D barcode pipeline for Elixir.

  This package keeps the layers separated:
  - barcode packages own encoding
  - `barcode_layout_1d` owns geometry
  - native Paint VM packages own pixels and image encoding
  """

  alias CodingAdventures.{
    Codabar,
    Code128,
    Code39,
    Ean13,
    Itf,
    PaintCodecPngNative,
    PaintVmMetalNative,
    UpcA
  }

  @type symbology :: :codabar | :code128 | :code39 | :ean13 | :itf | :upca | String.t()
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
      :codabar -> {:ok, Codabar.layout_codabar(data, layout_config)}
      :code128 -> {:ok, Code128.layout_code128(data, layout_config)}
      :code39 -> {:ok, Code39.layout_code39(data, layout_config)}
      :ean13 -> {:ok, Ean13.layout_ean_13(data, layout_config)}
      :itf -> {:ok, Itf.layout_itf(data, layout_config)}
      :upca -> {:ok, UpcA.layout_upc_a(data, layout_config)}
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
    backend_result = Keyword.get_lazy(opts, :backend_result, &current_backend/0)
    executor = Keyword.get(opts, :scene_executor, &execute_scene/2)

    with {:ok, scene} <- build_scene(data, opts),
         {:ok, backend} <- backend_result,
         {:ok, pixels} <- executor.(scene, backend) do
      {:ok, pixels}
    end
  end

  @doc """
  Render barcode text all the way to PNG bytes.
  """
  @spec render_png(String.t(), keyword()) :: {:ok, binary()} | {:error, render_error()}
  def render_png(data, opts \\ []) do
    encoder = Keyword.get(opts, :png_encoder, &PaintCodecPngNative.encode/1)

    with {:ok, pixels} <- render_pixels(data, opts),
         {:ok, png} <- encoder.(pixels) do
      {:ok, png}
    end
  end

  defp execute_scene(scene, :metal), do: PaintVmMetalNative.render(scene)

  defp normalize_symbology(:codabar), do: :codabar
  defp normalize_symbology(:code128), do: :code128
  defp normalize_symbology(:code39), do: :code39
  defp normalize_symbology(:ean13), do: :ean13
  defp normalize_symbology(:itf), do: :itf
  defp normalize_symbology(:upca), do: :upca
  defp normalize_symbology("codabar"), do: :codabar
  defp normalize_symbology("code128"), do: :code128
  defp normalize_symbology("code39"), do: :code39
  defp normalize_symbology("ean13"), do: :ean13
  defp normalize_symbology("ean-13"), do: :ean13
  defp normalize_symbology("itf"), do: :itf
  defp normalize_symbology("upca"), do: :upca
  defp normalize_symbology("upc-a"), do: :upca
  defp normalize_symbology(_symbology), do: :unsupported
end
