defmodule CodingAdventures.PaintVmAscii do
  @moduledoc """
  Terminal backend for backend-neutral paint scenes.

  The current Elixir paint instruction model is rect-only, so this first
  version renders filled rectangles as block-character output and raises for
  unsupported instruction kinds.
  """

  @type options :: [scale_x: pos_integer(), scale_y: pos_integer()]

  @spec render(map(), options()) :: String.t()
  def render(scene, opts \\ []) do
    sx = Keyword.get(opts, :scale_x, 8)
    sy = Keyword.get(opts, :scale_y, 16)
    cols = ceil(scene.width / sx)
    rows = ceil(scene.height / sy)
    chars = for _row <- 1..rows, do: for(_col <- 1..cols, do: " ")
    buffer = %{rows: rows, cols: cols, chars: chars}

    buffer =
      Enum.reduce(scene.instructions, buffer, fn inst, acc ->
        render_instruction(inst, acc, sx, sy)
      end)

    buffer_to_string(buffer)
  end

  defp render_instruction(%{kind: :rect} = inst, buffer, sx, sy), do: render_rect(inst, buffer, sx, sy)

  defp render_instruction(inst, _buffer, _sx, _sy) do
    raise ArgumentError, "paint_vm_ascii: unsupported paint instruction kind: #{inspect(inst.kind)}"
  end

  defp render_rect(inst, buffer, _sx, _sy)
       when inst.fill in [nil, "", "transparent", "none"],
       do: buffer

  defp render_rect(inst, buffer, sx, sy) do
    c1 = to_col(inst.x, sx)
    r1 = to_row(inst.y, sy)
    c2 = to_col(inst.x + inst.width, sx)
    r2 = to_row(inst.y + inst.height, sy)

    Enum.reduce(r1..r2, buffer, fn row, acc ->
      Enum.reduce(c1..c2, acc, fn col, inner ->
        write_char(inner, row, col, "█")
      end)
    end)
  end

  defp to_col(x, sx), do: round(x / sx)
  defp to_row(y, sy), do: round(y / sy)

  defp write_char(buffer, row, col, _ch)
       when row < 0 or row >= buffer.rows or col < 0 or col >= buffer.cols,
       do: buffer

  defp write_char(buffer, row, col, ch) do
    updated_row =
      buffer.chars
      |> Enum.at(row)
      |> List.replace_at(col, ch)

    %{buffer | chars: List.replace_at(buffer.chars, row, updated_row)}
  end

  defp buffer_to_string(buffer) do
    buffer.chars
    |> Enum.map(&(Enum.join(&1) |> String.trim_trailing()))
    |> Enum.join("\n")
    |> String.trim_trailing()
  end
end
