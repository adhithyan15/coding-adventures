namespace CodingAdventures.Matrix;

/// <summary>
/// Immutable matrix of double-precision values.
/// </summary>
public sealed class Matrix : IEquatable<Matrix>
{
    private readonly double[][] _data;

    /// <summary>Create a matrix from a rectangular 2D array.</summary>
    public Matrix(double[][] data)
    {
        _data = ValidateAndCopy(data);
        Rows = _data.Length;
        Cols = _data[0].Length;
    }

    /// <summary>Number of rows.</summary>
    public int Rows { get; }

    /// <summary>Number of columns.</summary>
    public int Cols { get; }

    /// <summary>Create a 1x1 matrix from a scalar.</summary>
    public static Matrix FromScalar(double value) => new([[value]]);

    /// <summary>Create a 1xN row vector from a one-dimensional array.</summary>
    public static Matrix FromArray(double[] values)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (values.Length == 0)
        {
            throw new ArgumentException("Array must not be empty", nameof(values));
        }

        return new Matrix([values.ToArray()]);
    }

    /// <summary>Create an MxN zero-filled matrix.</summary>
    public static Matrix Zeros(int rows, int cols)
    {
        if (rows <= 0 || cols <= 0)
        {
            throw new ArgumentException("Dimensions must be positive");
        }

        return new Matrix(Enumerable.Range(0, rows).Select(_ => new double[cols]).ToArray());
    }

    /// <summary>Get an element by row and column.</summary>
    public double this[int row, int col] => _data[row][col];

    /// <summary>Get an element by row and column.</summary>
    public double Get(int row, int col) => this[row, col];

    /// <summary>Return a deep copy of the matrix data.</summary>
    public double[][] GetData() => _data.Select(row => row.ToArray()).ToArray();

    /// <summary>Add another matrix element-wise.</summary>
    public Matrix Add(Matrix other)
    {
        CheckDimensions(other, "add");
        return Map2(other, static (left, right) => left + right);
    }

    /// <summary>Subtract another matrix element-wise.</summary>
    public Matrix Subtract(Matrix other)
    {
        CheckDimensions(other, "subtract");
        return Map2(other, static (left, right) => left - right);
    }

    /// <summary>Add a scalar to every element.</summary>
    public Matrix AddScalar(double scalar) => Map(value => value + scalar);

    /// <summary>Subtract a scalar from every element.</summary>
    public Matrix SubtractScalar(double scalar) => AddScalar(-scalar);

    /// <summary>Multiply every element by a scalar.</summary>
    public Matrix Scale(double scalar) => Map(value => value * scalar);

    /// <summary>Swap rows and columns.</summary>
    public Matrix Transpose()
    {
        var result = new double[Cols][];
        for (var col = 0; col < Cols; col++)
        {
            result[col] = new double[Rows];
            for (var row = 0; row < Rows; row++)
            {
                result[col][row] = _data[row][col];
            }
        }

        return new Matrix(result);
    }

    /// <summary>Multiply this matrix by another matrix.</summary>
    public Matrix Dot(Matrix other)
    {
        ArgumentNullException.ThrowIfNull(other);
        if (Cols != other.Rows)
        {
            throw new ArgumentException(
                $"Dot dimension mismatch: {Rows}x{Cols} dot {other.Rows}x{other.Cols}",
                nameof(other));
        }

        var result = new double[Rows][];
        for (var row = 0; row < Rows; row++)
        {
            result[row] = new double[other.Cols];
            for (var col = 0; col < other.Cols; col++)
            {
                var sum = 0.0;
                for (var k = 0; k < Cols; k++)
                {
                    sum += _data[row][k] * other._data[k][col];
                }

                result[row][col] = sum;
            }
        }

        return new Matrix(result);
    }

    /// <inheritdoc />
    public bool Equals(Matrix? other)
    {
        if (ReferenceEquals(this, other))
        {
            return true;
        }

        if (other is null || Rows != other.Rows || Cols != other.Cols)
        {
            return false;
        }

        for (var row = 0; row < Rows; row++)
        {
            if (!_data[row].SequenceEqual(other._data[row]))
            {
                return false;
            }
        }

        return true;
    }

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is Matrix other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Rows);
        hash.Add(Cols);
        foreach (var row in _data)
        {
            foreach (var value in row)
            {
                hash.Add(value);
            }
        }

        return hash.ToHashCode();
    }

    /// <inheritdoc />
    public override string ToString() => $"Matrix({Rows}x{Cols})";

    private static double[][] ValidateAndCopy(double[][] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length == 0)
        {
            throw new ArgumentException("Matrix must have at least one row and column", nameof(data));
        }

        ArgumentNullException.ThrowIfNull(data[0]);
        if (data[0].Length == 0)
        {
            throw new ArgumentException("Matrix must have at least one row and column", nameof(data));
        }

        var cols = data[0].Length;
        var copy = new double[data.Length][];
        for (var row = 0; row < data.Length; row++)
        {
            ArgumentNullException.ThrowIfNull(data[row]);
            if (data[row].Length != cols)
            {
                throw new ArgumentException(
                    $"Row {row} has {data[row].Length} columns, expected {cols}",
                    nameof(data));
            }

            copy[row] = data[row].ToArray();
        }

        return copy;
    }

    private void CheckDimensions(Matrix? other, string operation)
    {
        ArgumentNullException.ThrowIfNull(other);
        if (Rows != other.Rows || Cols != other.Cols)
        {
            throw new ArgumentException(
                $"{operation} dimension mismatch: {Rows}x{Cols} vs {other.Rows}x{other.Cols}",
                nameof(other));
        }
    }

    private Matrix Map(Func<double, double> mapper)
    {
        var result = new double[Rows][];
        for (var row = 0; row < Rows; row++)
        {
            result[row] = new double[Cols];
            for (var col = 0; col < Cols; col++)
            {
                result[row][col] = mapper(_data[row][col]);
            }
        }

        return new Matrix(result);
    }

    private Matrix Map2(Matrix other, Func<double, double, double> mapper)
    {
        var result = new double[Rows][];
        for (var row = 0; row < Rows; row++)
        {
            result[row] = new double[Cols];
            for (var col = 0; col < Cols; col++)
            {
                result[row][col] = mapper(_data[row][col], other._data[row][col]);
            }
        }

        return new Matrix(result);
    }
}
