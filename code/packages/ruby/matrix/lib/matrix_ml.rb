class Matrix
  attr_reader :data, :rows, :cols

  def self.zeros(rows, cols)
    Matrix.new(Array.new(rows) { Array.new(cols, 0.0) })
  end

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
end
