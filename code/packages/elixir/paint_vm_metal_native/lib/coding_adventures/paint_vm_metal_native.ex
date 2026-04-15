defmodule CodingAdventures.PaintVmMetalNative do
  @moduledoc """
  Native Metal Paint VM bridge for barcode-friendly paint scenes.

  The native boundary stays narrow on purpose:
  - Elixir owns scene construction and barcode layout.
  - Metal owns pixel generation.
  - The bridge returns a `CodingAdventures.PixelContainer`.

  Only rectangle instructions are currently accepted, which matches the
  barcode pipeline on this branch.
  """

  alias CodingAdventures.PixelContainer

  @on_load :load_nif

  @type render_error ::
          :invalid_scene
          | :unsupported_instruction
          | :metal_backend_unavailable
          | :nif_not_available
          | atom()

  @doc false
  def load_nif do
    if supported_runtime?() and nif_file_exists?() do
      case :erlang.load_nif(String.to_charlist(nif_base_path()), 0) do
        :ok -> :ok
        {:error, _reason} -> :ok
      end
    else
      :ok
    end
  end

  @doc """
  Returns true when Metal rendering is available on the current runtime.
  """
  def available? do
    supported_runtime?() and nif_file_exists?()
  end

  @doc """
  Returns true when the host runtime can execute the Metal renderer.
  """
  def supported_runtime? do
    case :os.type() do
      {:unix, :darwin} ->
        architecture = :erlang.system_info(:system_architecture) |> to_string()
        String.contains?(architecture, "arm64") or String.contains?(architecture, "aarch64")

      _ ->
        false
    end
  end

  @doc """
  Execute a paint scene through Metal and return a pixel container.
  """
  @spec render(map()) :: {:ok, PixelContainer.t()} | {:error, render_error()}
  def render(scene) when is_map(scene) do
    cond do
      not supported_runtime?() ->
        {:error, :metal_backend_unavailable}

      not nif_file_exists?() ->
        {:error, :nif_not_available}

      true ->
        with {:ok, {width, height, background, rects}} <- encode_scene(scene) do
          decode_render_result(render_rect_scene_native(width, height, background, rects))
        end
    end
  rescue
    ErlangError ->
      {:error, :nif_not_available}
  end

  def render(_scene), do: {:error, :invalid_scene}

  defp decode_render_result({:ok, {width, height, data}})
       when is_integer(width) and is_integer(height) and is_binary(data) do
    {:ok, %PixelContainer{width: width, height: height, data: data}}
  end

  defp decode_render_result({:error, reason}), do: {:error, reason}
  defp decode_render_result(_), do: {:error, :nif_not_available}

  defp encode_scene(scene) do
    with {:ok, width} <- fetch_number(scene, :width),
         {:ok, height} <- fetch_number(scene, :height),
         {:ok, instructions} <- fetch_value(scene, :instructions),
         {:ok, rects} <- encode_instructions(instructions) do
      background =
        case fetch_value(scene, :background) do
          {:ok, value} -> to_string(value)
          :error -> "#ffffff"
        end

      {:ok, {width, height, background, rects}}
    else
      :error -> {:error, :invalid_scene}
      {:error, _} = error -> error
    end
  end

  defp encode_instructions(instructions) when is_list(instructions) do
    Enum.reduce_while(instructions, {:ok, []}, fn instruction, {:ok, acc} ->
      case encode_instruction(instruction) do
        {:ok, rect} -> {:cont, {:ok, [rect | acc]}}
        {:error, _reason} = error -> {:halt, error}
      end
    end)
    |> case do
      {:ok, rects} -> {:ok, Enum.reverse(rects)}
      {:error, _} = error -> error
    end
  end

  defp encode_instructions(_), do: {:error, :invalid_scene}

  defp encode_instruction(instruction) when is_map(instruction) do
    with {:ok, kind} <- fetch_value(instruction, :kind),
         true <- kind in [:rect, "rect"],
         {:ok, x} <- fetch_number(instruction, :x),
         {:ok, y} <- fetch_number(instruction, :y),
         {:ok, width} <- fetch_number(instruction, :width),
         {:ok, height} <- fetch_number(instruction, :height) do
      fill =
        case fetch_value(instruction, :fill) do
          {:ok, value} -> to_string(value)
          :error -> "#000000"
        end

      {:ok, {x, y, width, height, fill}}
    else
      false -> {:error, :unsupported_instruction}
      :error -> {:error, :invalid_scene}
      {:error, _} = error -> error
    end
  end

  defp encode_instruction(_instruction), do: {:error, :unsupported_instruction}

  defp fetch_number(map, key) do
    with {:ok, value} <- fetch_value(map, key),
         true <- is_integer(value) or is_float(value) do
      {:ok, value * 1.0}
    else
      false -> {:error, :invalid_scene}
      :error -> :error
      {:error, _} = error -> error
    end
  end

  defp fetch_value(map, key) do
    case Map.fetch(map, key) do
      {:ok, value} ->
        {:ok, value}

      :error ->
        Map.fetch(map, Atom.to_string(key))
    end
  end

  defp nif_base_path do
    case :code.priv_dir(:coding_adventures_paint_vm_metal_native) do
      {:error, _reason} -> nil
      priv_dir -> Path.join(to_string(priv_dir), "paint_vm_metal_native")
    end
  end

  defp nif_file_exists? do
    case nif_base_path() do
      nil -> false
      base_path -> File.exists?(base_path <> ".so")
    end
  end

  defp render_rect_scene_native(_width, _height, _background, _rects),
    do: :erlang.nif_error(:not_loaded)
end
