# ─────────────────────────────────────────────────────────────────────
# Matrix — A Pure Elixir Matrix Library
#
# This module provides a 2D matrix type with arithmetic, reductions,
# element-wise math, shape manipulation, and comparison operations.
#
# ## Design Principles
#
# 1. **Immutable by default.** All functions return a *new* Matrix struct.
#    In Elixir, data is immutable, so this is the natural style.
#
# 2. **No external dependencies.** Only `:math` from Erlang's standard
#    library for sqrt, pow, and abs.
#
# 3. **Functional style.** Instead of methods on an object, we use
#    module functions: `Matrix.sum(m)` rather than `m.sum()`.
#
# ## Internal Representation
#
#   %Matrix{
#     data: [[float]],   -- list of lists, outer = rows, inner = columns
#     rows: non_neg_integer,
#     cols: non_neg_integer
#   }
# ─────────────────────────────────────────────────────────────────────

defmodule Matrix do
  defstruct data: [], rows: 0, cols: 0

  # ─── Constructors ─────────────────────────────────────────────────

  @doc """
  Create a matrix filled with zeros.

  This is the workhorse factory — used internally by many operations
  that need a blank canvas to fill in.
  """
  def zeros(rows, cols) do
    data = List.duplicate(List.duplicate(0.0, cols), rows)
    %Matrix{data: data, rows: rows, cols: cols}
  end

  @doc """
  Create a Matrix from a scalar, a 1D list, or a 2D list of lists.

  Examples:
    Matrix.new(5)              -> 1x1 matrix [[5.0]]
    Matrix.new([1, 2, 3])      -> 1x3 matrix [[1.0, 2.0, 3.0]]
    Matrix.new([[1,2],[3,4]])   -> 2x2 matrix
  """
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

  # ─── Factory Methods ──────────────────────────────────────────────

  @doc """
  Create an n x n identity matrix.

  The identity matrix has 1.0 on the main diagonal and 0.0 everywhere
  else. It is the multiplicative identity for matrix dot products:

    identity(n) |> dot(m) == m   (for any n x m matrix m)

  This is analogous to multiplying a number by 1 — it changes nothing.
  """
  def identity(n) do
    data = for i <- 0..(n - 1)//1 do
      for j <- 0..(n - 1)//1 do
        if i == j, do: 1.0, else: 0.0
      end
    end
    %Matrix{data: data, rows: n, cols: n}
  end

  @doc """
  Create a diagonal matrix from a list of values.

  The resulting matrix is n x n where n = length(values).
  Only the main diagonal is populated; off-diagonal entries are 0.

    from_diagonal([2, 3]) -> [[2.0, 0.0], [0.0, 3.0]]
  """
  def from_diagonal(values) do
    n = length(values)
    indexed = Enum.with_index(values)
    data = for {val, i} <- indexed do
      for j <- 0..(n - 1)//1 do
        if i == j, do: val / 1, else: 0.0
      end
    end
    %Matrix{data: data, rows: n, cols: n}
  end

  # ─── Basic Arithmetic ────────────────────────────────────────────

  @doc """
  Element-wise matrix addition. Both matrices must have the same shape.
  """
  def add(a, b) do
    if a.rows != b.rows or a.cols != b.cols, do: raise "Mismatch addition dimensions"
    data = Enum.zip_with(a.data, b.data, fn r1, r2 ->
      Enum.zip_with(r1, r2, &(&1 + &2))
    end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end

  @doc """
  Add a scalar to every element (broadcast addition).
  """
  def add_scalar(a, scalar) do
    data = Enum.map(a.data, fn r -> Enum.map(r, &(&1 + scalar)) end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end

  @doc """
  Element-wise matrix subtraction.
  """
  def subtract(a, b) do
    if a.rows != b.rows or a.cols != b.cols, do: raise "Mismatch subtraction dimensions"
    data = Enum.zip_with(a.data, b.data, fn r1, r2 ->
      Enum.zip_with(r1, r2, &(&1 - &2))
    end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end

  @doc """
  Multiply every element by a scalar.
  """
  def scale(a, scalar) do
    data = Enum.map(a.data, fn r -> Enum.map(r, &(&1 * scalar)) end)
    %Matrix{data: data, rows: a.rows, cols: a.cols}
  end

  @doc """
  Transpose: swap rows and columns. M^T[j][i] = M[i][j].
  """
  def transpose(%Matrix{rows: 0}), do: %Matrix{data: [], rows: 0, cols: 0}
  def transpose(a) do
    data = Enum.zip_with(a.data, &(&1))
    %Matrix{data: data, rows: a.cols, cols: a.rows}
  end

  @doc """
  Matrix multiplication (dot product).

  For an m x k matrix A and a k x n matrix B, the result is m x n
  where C[i][j] = sum over k of A[i][k] * B[k][j].
  """
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

  # ─── Element Access ───────────────────────────────────────────────

  @doc """
  Get the element at (row, col). Zero-based indices.

  Raises if the index is out of bounds.
  """
  def get(m, row, col) do
    if row < 0 or row >= m.rows or col < 0 or col >= m.cols do
      raise "Index (#{row}, #{col}) out of bounds for #{m.rows}x#{m.cols} matrix"
    end
    m.data |> Enum.at(row) |> Enum.at(col)
  end

  @doc """
  Return a new matrix with the element at (row, col) replaced by value.

  The original matrix is not modified — Elixir data is always immutable.
  """
  def set(m, row, col, value) do
    if row < 0 or row >= m.rows or col < 0 or col >= m.cols do
      raise "Index (#{row}, #{col}) out of bounds for #{m.rows}x#{m.cols} matrix"
    end
    data = List.update_at(m.data, row, fn r ->
      List.replace_at(r, col, value)
    end)
    %Matrix{data: data, rows: m.rows, cols: m.cols}
  end

  # ─── Reductions ──────────────────────────────────────────────────

  @doc """
  Sum of all elements.

  For [[1,2],[3,4]]: 1 + 2 + 3 + 4 = 10.0

  This is a "full reduction" — the entire matrix collapses to one scalar.
  """
  def sum(m) do
    Enum.reduce(m.data, 0, fn row, acc ->
      acc + Enum.sum(row)
    end)
  end

  @doc """
  Sum each row, returning an n x 1 column vector.

  For [[1,2],[3,4]]: rows -> [[3],[7]]
  """
  def sum_rows(m) do
    data = Enum.map(m.data, fn row -> [Enum.sum(row)] end)
    %Matrix{data: data, rows: m.rows, cols: 1}
  end

  @doc """
  Sum each column, returning a 1 x m row vector.

  For [[1,2],[3,4]]: cols -> [[4,6]]
  """
  def sum_cols(m) do
    sums = Enum.reduce(m.data, List.duplicate(0, m.cols), fn row, acc ->
      Enum.zip_with(row, acc, &(&1 + &2))
    end)
    %Matrix{data: [sums], rows: 1, cols: m.cols}
  end

  @doc """
  Arithmetic mean of all elements: sum / count.
  """
  def mean(m) do
    sum(m) / (m.rows * m.cols)
  end

  @doc """
  Minimum element value.
  """
  def min_val(m) do
    m.data |> List.flatten() |> Enum.min()
  end

  @doc """
  Maximum element value.
  """
  def max_val(m) do
    m.data |> List.flatten() |> Enum.max()
  end

  @doc """
  {row, col} of the minimum element. First occurrence in row-major order.
  """
  def argmin(m) do
    find_extreme(m, :min)
  end

  @doc """
  {row, col} of the maximum element. First occurrence in row-major order.

    argmax(Matrix.new([[1,2],[3,4]])) -> {1, 1}
  """
  def argmax(m) do
    find_extreme(m, :max)
  end

  # Helper: scan row-major order to find first min or max position.
  defp find_extreme(m, mode) do
    flat = m.data
      |> Enum.with_index()
      |> Enum.flat_map(fn {row, i} ->
        Enum.with_index(row) |> Enum.map(fn {val, j} -> {val, i, j} end)
      end)

    target = case mode do
      :min -> flat |> Enum.min_by(fn {val, _i, _j} -> val end)
      :max -> flat |> Enum.max_by(fn {val, _i, _j} -> val end)
    end

    {_val, row, col} = target
    {row, col}
  end

  # ─── Element-wise Math ──────────────────────────────────────────

  @doc """
  Apply a function to every element, returning a new matrix.

  This is the most general element-wise operation. `matrix_sqrt`,
  `matrix_abs`, and `matrix_pow` are all special cases of `map_elements`.
  """
  def map_elements(m, fun) do
    data = Enum.map(m.data, fn row ->
      Enum.map(row, fun)
    end)
    %Matrix{data: data, rows: m.rows, cols: m.cols}
  end

  @doc """
  Element-wise square root.
  """
  def matrix_sqrt(m) do
    map_elements(m, &:math.sqrt/1)
  end

  @doc """
  Element-wise absolute value.
  """
  def matrix_abs(m) do
    map_elements(m, &abs/1)
  end

  @doc """
  Element-wise exponentiation: each element raised to exp.
  """
  def matrix_pow(m, exp) do
    map_elements(m, fn val -> :math.pow(val, exp) end)
  end

  # ─── Shape Operations ─────────────────────────────────────────────

  @doc """
  Flatten into a 1 x n row vector (n = rows * cols).

  Elements are read in row-major order.
  """
  def flatten(m) do
    flat = List.flatten(m.data)
    %Matrix{data: [flat], rows: 1, cols: m.rows * m.cols}
  end

  @doc """
  Reshape into a matrix with the given dimensions.

  rows * cols must equal m.rows * m.cols.
  """
  def reshape(m, rows, cols) do
    if rows * cols != m.rows * m.cols do
      raise "Cannot reshape #{m.rows}x#{m.cols} to #{rows}x#{cols}"
    end
    flat = List.flatten(m.data)
    data = Enum.chunk_every(flat, cols)
    %Matrix{data: data, rows: rows, cols: cols}
  end

  @doc """
  Extract row i as a 1 x cols matrix. Zero-based.
  """
  def get_row(m, i) do
    if i < 0 or i >= m.rows do
      raise "Row index #{i} out of bounds for #{m.rows} rows"
    end
    %Matrix{data: [Enum.at(m.data, i)], rows: 1, cols: m.cols}
  end

  @doc """
  Extract column j as a rows x 1 matrix. Zero-based.
  """
  def get_col(m, j) do
    if j < 0 or j >= m.cols do
      raise "Column index #{j} out of bounds for #{m.cols} cols"
    end
    data = Enum.map(m.data, fn row -> [Enum.at(row, j)] end)
    %Matrix{data: data, rows: m.rows, cols: 1}
  end

  @doc """
  Extract a sub-matrix from rows [r0..r1) and columns [c0..c1).

  The range is half-open: r1 and c1 are exclusive.
  """
  def matrix_slice(m, r0, r1, c0, c1) do
    if r0 < 0 or r1 > m.rows or c0 < 0 or c1 > m.cols or r0 >= r1 or c0 >= c1 do
      raise "Invalid slice [#{r0}:#{r1}, #{c0}:#{c1}] for #{m.rows}x#{m.cols} matrix"
    end
    data = m.data
      |> Enum.slice(r0..(r1 - 1)//1)
      |> Enum.map(fn row -> Enum.slice(row, c0..(c1 - 1)//1) end)
    %Matrix{data: data, rows: r1 - r0, cols: c1 - c0}
  end

  # ─── Equality and Comparison ──────────────────────────────────────

  @doc """
  Exact element-wise equality.
  """
  def equals(a, b) do
    a.rows == b.rows and a.cols == b.cols and a.data == b.data
  end

  @doc """
  Approximate equality within a tolerance.

  Returns true iff |a - b| <= tolerance for every element pair.
  Default tolerance is 1.0e-9.
  """
  def close(a, b, tolerance \\ 1.0e-9) do
    if a.rows != b.rows or a.cols != b.cols do
      false
    else
      Enum.zip(List.flatten(a.data), List.flatten(b.data))
      |> Enum.all?(fn {x, y} -> abs(x - y) <= tolerance end)
    end
  end
end
