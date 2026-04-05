# frozen_string_literal: true

# Matrix -- a pure-Ruby 2D matrix for linear algebra.
#
# A matrix is a rectangular grid of numbers arranged in rows and columns.
# Think of it like a spreadsheet: each cell sits at the intersection of a
# row number and a column number.  Matrices are the fundamental building
# block for linear algebra, which powers everything from graphics to
# machine learning.
#
# Design principles
# -----------------
# 1. **Immutable by default.**  Every method returns a *new* Matrix; the
#    original is never mutated.
# 2. **No external dependencies.**  Only Ruby's built-in Math module.
# 3. **Consistent error handling.**  Out-of-bounds indices, shape
#    mismatches, and invalid reshape dimensions all raise descriptive
#    ArgumentError or IndexError exceptions.
class Matrix
  attr_reader :data, :rows, :cols

  # ------------------------------------------------------------------
  # Construction
  # ------------------------------------------------------------------
  # You can build a Matrix from:
  #   - A single number   -> 1x1 matrix
  #   - A flat array       -> 1-row matrix (row vector)
  #   - An array of arrays -> full 2D matrix

  def initialize(data)
    if data.is_a?(Numeric)
      @data = [[data.to_f]]
    elsif data.is_a?(Array) && data.length > 0 && data[0].is_a?(Numeric)
      @data = [data.map(&:to_f)]
    elsif data.is_a?(Array)
      @data = data
    else
      @data = []
    end
    @rows = @data.length
    @cols = @rows > 0 ? @data[0].length : 0
  end

  # Create an rows x cols matrix filled with zeros.
  def self.zeros(rows, cols)
    Matrix.new(Array.new(rows) { Array.new(cols, 0.0) })
  end

  # ------------------------------------------------------------------
  # Factory methods
  # ------------------------------------------------------------------

  # Create an n x n identity matrix.
  #
  # The identity matrix is the matrix equivalent of the number 1:
  # multiplying any matrix by the identity leaves it unchanged.
  # It has 1s on the main diagonal and 0s everywhere else.
  def self.identity(n)
    data = Array.new(n) { |i| Array.new(n) { |j| i == j ? 1.0 : 0.0 } }
    Matrix.new(data)
  end

  # Create a square diagonal matrix from an array of values.
  #
  # A diagonal matrix has non-zero entries only on the main diagonal.
  # This is useful for creating scaling transforms.
  def self.from_diagonal(values)
    n = values.length
    data = Array.new(n) { |i| Array.new(n) { |j| i == j ? values[i].to_f : 0.0 } }
    Matrix.new(data)
  end

  # ------------------------------------------------------------------
  # Arithmetic operators (existing)
  # ------------------------------------------------------------------

  def +(other)
    if other.is_a?(Numeric)
      Matrix.new(@data.map { |r| r.map { |v| v + other } })
    else
      raise ArgumentError, "Addition dimension mismatch" if @rows != other.rows || @cols != other.cols
      Matrix.new(@data.map.with_index { |row, i|
        row.map.with_index { |val, j| val + other.data[i][j] }
      })
    end
  end

  def -(other)
    if other.is_a?(Numeric)
      Matrix.new(@data.map { |r| r.map { |v| v - other } })
    else
      raise ArgumentError, "Subtraction dimension mismatch" if @rows != other.rows || @cols != other.cols
      Matrix.new(@data.map.with_index { |row, i|
        row.map.with_index { |val, j| val - other.data[i][j] }
      })
    end
  end

  def *(scalar)
    Matrix.new(@data.map { |r| r.map { |v| v * scalar } })
  end

  def transpose
    return Matrix.new([]) if @rows == 0
    Matrix.new(@data[0].map.with_index { |_, col_idx|
      @data.map { |row| row[col_idx] }
    })
  end

  def dot(other)
    raise ArgumentError, "Dot product inner dimension mismatch" if @cols != other.rows
    c = Matrix.zeros(@rows, other.cols)
    @rows.times do |i|
      other.cols.times do |j|
        @cols.times do |k|
          c.data[i][j] += @data[i][k] * other.data[k][j]
        end
      end
    end
    c
  end

  # ------------------------------------------------------------------
  # Element access
  # ------------------------------------------------------------------
  # get reads a single cell.  set returns a *new* matrix with one cell
  # changed (the original is never mutated).

  # Return the element at (row, col).
  # Raises IndexError if indices are out of bounds.
  def get(row, col)
    if row < 0 || row >= @rows || col < 0 || col >= @cols
      raise IndexError, "Index (#{row}, #{col}) out of bounds for #{@rows}x#{@cols} matrix"
    end
    @data[row][col].to_f
  end

  # Return a new matrix with the element at (row, col) replaced.
  # The original matrix is unchanged (immutable-by-default pattern).
  def set(row, col, value)
    if row < 0 || row >= @rows || col < 0 || col >= @cols
      raise IndexError, "Index (#{row}, #{col}) out of bounds for #{@rows}x#{@cols} matrix"
    end
    new_data = @data.map(&:dup)
    new_data[row][col] = value.to_f
    Matrix.new(new_data)
  end

  # ------------------------------------------------------------------
  # Reductions
  # ------------------------------------------------------------------
  # Reductions collapse a matrix (or parts of it) down to a single
  # number or a smaller matrix.

  # Sum of every element in the matrix.
  def sum
    total = 0.0
    @data.each { |row| row.each { |v| total += v } }
    total
  end

  # Sum each row, returning an (rows x 1) column vector.
  # Imagine collapsing every row into a single number.
  def sum_rows
    Matrix.new(@data.map { |row| [row.sum.to_f] })
  end

  # Sum each column, returning a (1 x cols) row vector.
  # Imagine collapsing every column downward.
  def sum_cols
    sums = Array.new(@cols, 0.0)
    @data.each do |row|
      row.each_with_index { |v, j| sums[j] += v }
    end
    Matrix.new([sums])
  end

  # Arithmetic mean of every element (sum / count).
  def mean
    n = @rows * @cols
    raise ArgumentError, "Cannot compute mean of an empty matrix" if n == 0
    sum / n.to_f
  end

  # Smallest element in the matrix.
  def min
    raise ArgumentError, "Cannot compute min of an empty matrix" if @rows == 0 || @cols == 0
    best = @data[0][0]
    @data.each { |row| row.each { |v| best = v if v < best } }
    best.to_f
  end

  # Largest element in the matrix.
  def max
    raise ArgumentError, "Cannot compute max of an empty matrix" if @rows == 0 || @cols == 0
    best = @data[0][0]
    @data.each { |row| row.each { |v| best = v if v > best } }
    best.to_f
  end

  # Position [row, col] of the smallest element.
  # First occurrence wins on ties (scanning left-to-right, top-to-bottom).
  def argmin
    raise ArgumentError, "Cannot compute argmin of an empty matrix" if @rows == 0 || @cols == 0
    best_val = @data[0][0]
    best_r = 0
    best_c = 0
    @data.each_with_index do |row, i|
      row.each_with_index do |v, j|
        if v < best_val
          best_val = v
          best_r = i
          best_c = j
        end
      end
    end
    [best_r, best_c]
  end

  # Position [row, col] of the largest element.
  # First occurrence wins on ties.
  def argmax
    raise ArgumentError, "Cannot compute argmax of an empty matrix" if @rows == 0 || @cols == 0
    best_val = @data[0][0]
    best_r = 0
    best_c = 0
    @data.each_with_index do |row, i|
      row.each_with_index do |v, j|
        if v > best_val
          best_val = v
          best_r = i
          best_c = j
        end
      end
    end
    [best_r, best_c]
  end

  # ------------------------------------------------------------------
  # Element-wise math
  # ------------------------------------------------------------------
  # These methods apply a function to every element independently.
  # The shape stays the same; only the values change.

  # Apply a block to every element, returning a new matrix.
  # This is the most general element-wise operation.
  def map_elements(&block)
    Matrix.new(@data.map { |row| row.map { |v| block.call(v) } })
  end

  # Element-wise square root.
  def sqrt
    map_elements { |v| Math.sqrt(v) }
  end

  # Element-wise absolute value.
  def abs
    map_elements { |v| v.abs.to_f }
  end

  # Raise every element to the power exp.
  def pow(exp)
    map_elements { |v| v**exp }
  end

  # ------------------------------------------------------------------
  # Shape operations
  # ------------------------------------------------------------------
  # Shape operations rearrange elements without altering their values.

  # Flatten to a 1 x n row vector (row-major order).
  def flatten
    flat = @data.flat_map { |row| row.map(&:to_f) }
    Matrix.new([flat])
  end

  # Reshape to rows x cols.  Total element count must stay the same.
  def reshape(new_rows, new_cols)
    total = @rows * @cols
    if new_rows * new_cols != total
      raise ArgumentError, "Cannot reshape #{@rows}x#{@cols} (#{total} elements) into #{new_rows}x#{new_cols} (#{new_rows * new_cols} elements)"
    end
    flat = flatten.data[0]
    new_data = Array.new(new_rows) { |i| flat[i * new_cols, new_cols] }
    Matrix.new(new_data)
  end

  # Extract row i as a 1 x cols matrix.
  def row(i)
    raise IndexError, "Row #{i} out of bounds for #{@rows}-row matrix" if i < 0 || i >= @rows
    Matrix.new([@data[i].dup])
  end

  # Extract column j as a rows x 1 matrix.
  def col(j)
    raise IndexError, "Column #{j} out of bounds for #{@cols}-column matrix" if j < 0 || j >= @cols
    Matrix.new(@data.map { |r| [r[j]] })
  end

  # Extract sub-matrix for rows [r0, r1) and cols [c0, c1).
  # Half-open intervals, like Ruby ranges with exclusive end.
  def slice(r0, r1, c0, c1)
    if r0 < 0 || r1 > @rows || c0 < 0 || c1 > @cols
      raise IndexError, "Slice [#{r0}:#{r1}, #{c0}:#{c1}] out of bounds for #{@rows}x#{@cols} matrix"
    end
    if r0 >= r1 || c0 >= c1
      raise ArgumentError, "Slice dimensions must be positive (r0 < r1, c0 < c1)"
    end
    Matrix.new((r0...r1).map { |i| (c0...c1).map { |j| @data[i][j].to_f } })
  end

  # ------------------------------------------------------------------
  # Equality and comparison
  # ------------------------------------------------------------------

  # Exact element-wise equality.
  def equals(other)
    return false if @rows != other.rows || @cols != other.cols
    @data.each_with_index do |row, i|
      row.each_with_index do |v, j|
        return false if v != other.data[i][j]
      end
    end
    true
  end

  # Check whether two matrices are element-wise within tolerance.
  # Useful for comparing floating-point results.
  def close(other, tolerance = 1e-9)
    return false if @rows != other.rows || @cols != other.cols
    @data.each_with_index do |row, i|
      row.each_with_index do |v, j|
        return false if (v - other.data[i][j]).abs > tolerance
      end
    end
    true
  end
end
