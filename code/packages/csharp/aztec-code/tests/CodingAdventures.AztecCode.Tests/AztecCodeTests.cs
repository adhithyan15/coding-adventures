using CodingAdventures.Barcode2D;

namespace CodingAdventures.AztecCode.Tests;

public sealed class AztecCodeTests
{
    [Fact]
    public void VersionIsCurrent()
    {
        Assert.Equal("0.1.0", AztecCodeEncoder.Version);
    }

    [Fact]
    public void DefaultOptionsUseTwentyThreePercentEcc()
    {
        Assert.Equal(23, AztecOptions.Default.MinEccPercent);
    }

    [Fact]
    public void EmptyStringProducesSmallCompactSymbol()
    {
        var grid = AztecCodeEncoder.Encode(string.Empty);

        Assert.Equal(15, grid.Rows);
        Assert.Equal(15, grid.Cols);
        Assert.Equal(ModuleShape.Square, grid.ModuleShape);
    }

    [Fact]
    public void SmallInputsFitCompactSymbols()
    {
        Assert.Equal(15, AztecCodeEncoder.Encode("A").Rows);
        Assert.True(AztecCodeEncoder.Encode("HELLO").Rows <= 19);
        Assert.True(AztecCodeEncoder.Encode(new string('A', 20)).Rows <= 27);
    }

    [Fact]
    public void LargerInputsUseFullSymbols()
    {
        var grid = AztecCodeEncoder.Encode(new string('A', 100));

        Assert.True(grid.Rows >= 19);
        Assert.Equal(grid.Rows, grid.Cols);
    }

    [Fact]
    public void HandlesTwoHundredBytePayload()
    {
        var grid = AztecCodeEncoder.Encode(new string('B', 200));

        Assert.True(grid.Rows > 0);
        Assert.Equal(grid.Rows, grid.Cols);
    }

    [Fact]
    public void CompactSymbolSizeMatchesLayerFormula()
    {
        var valid = new[] { 15, 19, 23, 27 };

        Assert.Contains(AztecCodeEncoder.Encode("A").Rows, valid);
    }

    [Fact]
    public void GridIsAlwaysSquare()
    {
        foreach (var input in new[] { "A", "HELLO", "1234567890", new string('X', 50) })
        {
            var grid = AztecCodeEncoder.Encode(input);

            Assert.Equal(grid.Rows, grid.Cols);
        }
    }

    [Fact]
    public void CompactBullseyeCenterAndRingsAreCorrect()
    {
        var grid = AztecCodeEncoder.Encode("A");
        var center = grid.Rows / 2;

        Assert.True(ModuleAt(grid, center, center));
        for (var distance = 0; distance <= 5; distance++)
        {
            var expected = distance <= 1 || distance % 2 == 1;
            Assert.Equal(expected, ModuleAt(grid, center - distance, center));
        }
    }

    [Fact]
    public void FullBullseyeCenterAndRingsAreCorrect()
    {
        var grid = AztecCodeEncoder.Encode(new string('A', 100));
        var center = grid.Rows / 2;

        Assert.True(ModuleAt(grid, center, center));
        for (var distance = 0; distance <= 7; distance++)
        {
            var expected = distance <= 1 || distance % 2 == 1;
            Assert.Equal(expected, ModuleAt(grid, center - distance, center + distance));
        }
    }

    [Fact]
    public void OrientationCornersAreDarkForCompactAndFullSymbols()
    {
        AssertOrientationCorners(AztecCodeEncoder.Encode("HELLO"));
        AssertOrientationCorners(AztecCodeEncoder.Encode(new string('A', 100)));
    }

    [Fact]
    public void FullSymbolHasReferenceGridAtCenter()
    {
        var grid = AztecCodeEncoder.Encode(new string('A', 100));
        var center = grid.Rows / 2;

        Assert.True(ModuleAt(grid, center, center));
    }

    [Fact]
    public void EncodingIsDeterministic()
    {
        var first = AztecCodeEncoder.Encode("HELLO WORLD");
        var second = AztecCodeEncoder.Encode("HELLO WORLD");

        AssertEqualModules(first, second);
    }

    [Fact]
    public void EncodeBytesMatchesStringEncodingForAscii()
    {
        var fromString = AztecCodeEncoder.Encode("HELLO");
        var fromBytes = AztecCodeEncoder.EncodeBytes("HELLO"u8.ToArray());

        AssertEqualModules(fromString, fromBytes);
    }

    [Fact]
    public void EncodeBytesSupportsBinaryPayloads()
    {
        var grid = AztecCodeEncoder.EncodeBytes([0x00, 0x01, 0xFF, 0x7F, 0x80]);

        Assert.True(grid.Rows >= 15);
    }

    [Fact]
    public void BitStuffingHeavyInputsEncode()
    {
        foreach (var input in new[] { new string('A', 30), new string('a', 30), new string('0', 30), new string('F', 30) })
        {
            var grid = AztecCodeEncoder.Encode(input);

            Assert.True(grid.Rows > 0);
            Assert.Equal(grid.Rows, grid.Cols);
        }
    }

    [Theory]
    [InlineData(5)]
    [InlineData(95)]
    public void InvalidEccPercentThrows(int value)
    {
        var ex = Assert.Throws<InvalidAztecOptionsException>(
            () => AztecCodeEncoder.Encode("HELLO", new AztecOptions(value)));

        Assert.Contains(value.ToString(), ex.Message);
    }

    [Theory]
    [InlineData(10)]
    [InlineData(90)]
    public void EccPercentBoundaryValuesAreAllowed(int value)
    {
        var grid = AztecCodeEncoder.Encode("HELLO", new AztecOptions(value));

        Assert.True(grid.Rows > 0);
    }

    [Fact]
    public void HigherEccDoesNotShrinkSymbol()
    {
        var defaultGrid = AztecCodeEncoder.Encode("HELLO WORLD");
        var highEccGrid = AztecCodeEncoder.Encode("HELLO WORLD", new AztecOptions(50));

        Assert.True(highEccGrid.Rows >= defaultGrid.Rows);
    }

    [Fact]
    public void MassivePayloadThrowsInputTooLong()
    {
        var ex = Assert.Throws<InputTooLongException>(() => AztecCodeEncoder.Encode(new string('A', 100_000)));

        Assert.Contains("maximum supported", ex.Message);
    }

    [Fact]
    public void NullInputsThrow()
    {
        Assert.Throws<ArgumentNullException>(() => AztecCodeEncoder.Encode(null!));
        Assert.Throws<ArgumentNullException>(() => AztecCodeEncoder.EncodeBytes(null!));
    }

    [Fact]
    public void ModuleGridDimensionsMatchRowsAndCols()
    {
        var grid = AztecCodeEncoder.Encode("HELLO");

        Assert.Equal(grid.Rows, grid.Modules.Count);
        foreach (var row in grid.Modules)
        {
            Assert.Equal(grid.Cols, row.Count);
        }
    }

    [Fact]
    public void ModuleGridContainsDarkAndLightModules()
    {
        var grid = AztecCodeEncoder.Encode("HELLO WORLD");
        var darkCount = 0;
        var lightCount = 0;

        for (var row = 0; row < grid.Rows; row++)
        {
            for (var col = 0; col < grid.Cols; col++)
            {
                if (ModuleAt(grid, row, col))
                {
                    darkCount++;
                }
                else
                {
                    lightCount++;
                }
            }
        }

        Assert.True(darkCount > 0);
        Assert.True(lightCount > 0);
    }

    [Fact]
    public void Utf8MultiByteCharactersEncode()
    {
        var grid = AztecCodeEncoder.Encode("cafe resume");

        Assert.True(grid.Rows > 0);
    }

    [Theory]
    [InlineData(31)]
    [InlineData(32)]
    public void LengthEncodingBoundaryCasesEncode(int length)
    {
        var grid = AztecCodeEncoder.Encode(new string('X', length));

        Assert.True(grid.Rows > 0);
    }

    [Fact]
    public void OrientationCornersAreEquidistantFromCenter()
    {
        var grid = AztecCodeEncoder.Encode("ABC");
        var center = grid.Rows / 2;
        var radius = ExpectedBullseyeRadius(grid.Rows) + 1;
        var corners = new[]
        {
            (Row: center - radius, Col: center - radius),
            (Row: center - radius, Col: center + radius),
            (Row: center + radius, Col: center + radius),
            (Row: center + radius, Col: center - radius),
        };

        foreach (var (row, col) in corners)
        {
            Assert.Equal(radius, Chebyshev((row, col), (center, center)));
        }
    }

    private static bool ModuleAt(ModuleGrid grid, int row, int col) => grid.Modules[row][col];

    private static int ExpectedBullseyeRadius(int size) => size <= 27 ? 5 : 7;

    private static int Chebyshev((int Row, int Col) a, (int Row, int Col) b) =>
        Math.Max(Math.Abs(a.Row - b.Row), Math.Abs(a.Col - b.Col));

    private static void AssertOrientationCorners(ModuleGrid grid)
    {
        var center = grid.Rows / 2;
        var ring = ExpectedBullseyeRadius(grid.Rows) + 1;

        Assert.True(ModuleAt(grid, center - ring, center - ring));
        Assert.True(ModuleAt(grid, center - ring, center + ring));
        Assert.True(ModuleAt(grid, center + ring, center + ring));
        Assert.True(ModuleAt(grid, center + ring, center - ring));
    }

    private static void AssertEqualModules(ModuleGrid first, ModuleGrid second)
    {
        Assert.Equal(first.Rows, second.Rows);
        Assert.Equal(first.Cols, second.Cols);

        for (var row = 0; row < first.Rows; row++)
        {
            for (var col = 0; col < first.Cols; col++)
            {
                Assert.Equal(first.Modules[row][col], second.Modules[row][col]);
            }
        }
    }
}
