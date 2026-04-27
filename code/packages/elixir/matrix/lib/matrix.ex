defmodule Matrix do
  defstruct data: [], rows: 0, cols: 0

  def zeros(rows, cols) do
    data = List.duplicate(List.duplicate(0.0, cols), rows)
    %Matrix{data: data, rows: rows, cols: cols}
  end

  def new(data) when is_number(data) do
    %Matrix{data: [[data / 1]], rows: 1, cols: 1}
  end

  def new(data) when is_list(data) do
    cond do
      length(data) == 0 -> %Matrix{data: [], rows: 0, cols: 0}
      is_number(hd(data)) -> 
        %Matrix{data: [Enum.map(data, &(&1 / 1))], rows: 1, cols: length(data)}
      is_list(hd(data)) ->
        rows = length(data)
        cols = length(hd(data))
        %Matrix{data: data, rows: rows, cols: cols}
    end
  end

  def add(a, b) do
    if a.rows != b.rows or a.cols != b.cols, do: raise "Mismatch addition dimensions"
    data = Enum.zip_with(a.data, b.data, fn r1, r2 ->
      Enum.zip_with(r1, r2, &(&1 + &2))
    end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end
  
  def add_scalar(a, scalar) do
    data = Enum.map(a.data, fn r -> Enum.map(r, &(&1 + scalar)) end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end

  def subtract(a, b) do
    if a.rows != b.rows or a.cols != b.cols, do: raise "Mismatch subtraction dimensions"
    data = Enum.zip_with(a.data, b.data, fn r1, r2 ->
      Enum.zip_with(r1, r2, &(&1 - &2))
    end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end

  def scale(a, scalar) do
    data = Enum.map(a.data, fn r -> Enum.map(r, &(&1 * scalar)) end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end

  def transpose(%Matrix{rows: 0}), do: %Matrix{data: [], rows: 0, cols: 0}
  def transpose(a) do
    data = Enum.zip_with(a.data, &(&1))
    %Matrix{data: data, rows: a.cols, cols: a.rows}
  end

  def dot(a, b) do
    if a.cols != b.rows, do: raise "Mismatch inner dimensions for dot product execution"
    b_t = transpose(b)
    data = Enum.map(a.data, fn row_a ->
      Enum.map(b_t.data, fn col_b ->
        Enum.zip_with(row_a, col_b, &(&1 * &2)) |> Enum.sum()
      end)
    end)
    %Matrix{data: data, rows: a.rows, cols: b.cols}
  end
end
