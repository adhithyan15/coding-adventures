namespace CodingAdventures.Matrix.Tests;

public sealed class MatrixTests
{
    [Fact]
    public void FromScalar_CreatesOneByOneMatrix()
    {
        var matrix = Matrix.FromScalar(5.0);

        Assert.Equal(1, matrix.Rows);
        Assert.Equal(1, matrix.Cols);
        Assert.Equal(5.0, matrix[0, 0]);
    }

    [Fact]
    public void FromArray_CreatesRowVector()
    {
        var matrix = Matrix.FromArray([1.0, 2.0, 3.0]);

        Assert.Equal(1, matrix.Rows);
        Assert.Equal(3, matrix.Cols);
        Assert.Equal(2.0, matrix.Get(0, 1));
    }

    [Fact]
    public void Constructor_DeepCopiesRectangularData()
    {
        var source = new[] { new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 } };
        var matrix = new Matrix(source);
        source[0][0] = 99.0;

        Assert.Equal(2, matrix.Rows);
        Assert.Equal(2, matrix.Cols);
        Assert.Equal(1.0, matrix[0, 0]);
    }

    [Fact]
    public void Zeros_CreatesZeroFilledMatrix()
    {
        var matrix = Matrix.Zeros(3, 2);

        Assert.Equal(3, matrix.Rows);
        Assert.Equal(2, matrix.Cols);
        for (var row = 0; row < matrix.Rows; row++)
        {
            for (var col = 0; col < matrix.Cols; col++)
            {
                Assert.Equal(0.0, matrix[row, col]);
            }
        }
    }

    [Fact]
    public void InvalidConstruction_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Matrix([]));
        Assert.Throws<ArgumentException>(() => new Matrix([[]]));
        Assert.Throws<ArgumentException>(() => new Matrix([new[] { 1.0, 2.0 }, [3.0]]));
        Assert.Throws<ArgumentException>(() => Matrix.FromArray([]));
        Assert.Throws<ArgumentException>(() => Matrix.Zeros(0, 2));
    }

    [Fact]
    public void Add_AddsMatricesElementWise()
    {
        var a = new Matrix([new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 }]);
        var b = new Matrix([new[] { 5.0, 6.0 }, new[] { 7.0, 8.0 }]);

        Assert.Equal(new Matrix([new[] { 6.0, 8.0 }, new[] { 10.0, 12.0 }]), a.Add(b));
    }

    [Fact]
    public void Subtract_SubtractsMatricesElementWise()
    {
        var a = new Matrix([new[] { 5.0, 6.0 }, new[] { 7.0, 8.0 }]);
        var b = new Matrix([new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 }]);

        Assert.Equal(new Matrix([new[] { 4.0, 4.0 }, new[] { 4.0, 4.0 }]), a.Subtract(b));
    }

    [Fact]
    public void ScalarOperations_MapEveryElement()
    {
        var matrix = new Matrix([new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 }]);

        Assert.Equal(new Matrix([new[] { 11.0, 12.0 }, new[] { 13.0, 14.0 }]), matrix.AddScalar(10.0));
        Assert.Equal(new Matrix([new[] { -4.0, -3.0 }, new[] { -2.0, -1.0 }]), matrix.SubtractScalar(5.0));
        Assert.Equal(new Matrix([new[] { 2.0, 4.0 }, new[] { 6.0, 8.0 }]), matrix.Scale(2.0));
        Assert.Equal(Matrix.Zeros(2, 2), matrix.Scale(0.0));
    }

    [Fact]
    public void DimensionMismatch_ThrowsForElementWiseOperations()
    {
        var row = new Matrix([new[] { 1.0, 2.0 }]);
        var column = new Matrix([new[] { 1.0 }, new[] { 2.0 }]);

        Assert.Throws<ArgumentException>(() => row.Add(column));
        Assert.Throws<ArgumentException>(() => row.Subtract(column));
    }

    [Fact]
    public void Transpose_SwapsRowsAndColumns()
    {
        var matrix = new Matrix([new[] { 1.0, 2.0, 3.0 }, new[] { 4.0, 5.0, 6.0 }]);
        var transposed = matrix.Transpose();

        Assert.Equal(3, transposed.Rows);
        Assert.Equal(2, transposed.Cols);
        Assert.Equal(new Matrix([new[] { 1.0, 4.0 }, new[] { 2.0, 5.0 }, new[] { 3.0, 6.0 }]), transposed);
        Assert.Equal(matrix, transposed.Transpose());
    }

    [Fact]
    public void Dot_MultipliesMatrices()
    {
        var a = new Matrix([new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 }]);
        var b = new Matrix([new[] { 5.0, 6.0 }, new[] { 7.0, 8.0 }]);

        Assert.Equal(new Matrix([new[] { 19.0, 22.0 }, new[] { 43.0, 50.0 }]), a.Dot(b));
    }

    [Fact]
    public void Dot_HandlesNonSquareAndIdentityMatrices()
    {
        var row = new Matrix([new[] { 1.0, 2.0, 3.0 }]);
        var column = new Matrix([new[] { 4.0 }, new[] { 5.0 }, new[] { 6.0 }]);
        var identity = new Matrix([new[] { 1.0, 0.0 }, new[] { 0.0, 1.0 }]);
        var matrix = new Matrix([new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 }]);

        Assert.Equal(Matrix.FromScalar(32.0), row.Dot(column));
        Assert.Equal(matrix, matrix.Dot(identity));
        Assert.Equal(matrix, identity.Dot(matrix));
    }

    [Fact]
    public void Dot_RejectsDimensionMismatch()
    {
        var matrix = new Matrix([new[] { 1.0, 2.0 }]);
        Assert.Throws<ArgumentException>(() => matrix.Dot(matrix));
    }

    [Fact]
    public void EqualityHashCodeAndString_UseMatrixValues()
    {
        var a = new Matrix([new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 }]);
        var b = new Matrix([new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 }]);
        var c = new Matrix([new[] { 1.0, 3.0 }, new[] { 3.0, 4.0 }]);

        Assert.Equal(a, b);
        Assert.True(a.Equals((object)b));
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
        Assert.NotEqual(a, c);
        Assert.NotEqual(a, Matrix.FromArray([1.0, 2.0]));
        Assert.Equal("Matrix(2x2)", a.ToString());
    }

    [Fact]
    public void GetData_ReturnsDeepCopy()
    {
        var matrix = new Matrix([new[] { 1.0, 2.0 }]);
        var copy = matrix.GetData();
        copy[0][0] = 999.0;

        Assert.Equal(1.0, matrix[0, 0]);
    }
}
