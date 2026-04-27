using CodingAdventures.Barcode2D;

namespace CodingAdventures.QRCode.Tests;

public sealed class QRCodeTests
{
    [Fact]
    public void VersionIsCurrent()
    {
        Assert.Equal("0.1.0", QRCodeEncoder.Version);
    }

    [Fact]
    public void VersionOneGridIsTwentyOneByTwentyOne()
    {
        var grid = QRCodeEncoder.Encode("1", EccLevel.L);

        Assert.Equal(21, grid.Rows);
        Assert.Equal(21, grid.Cols);
        Assert.Equal(ModuleShape.Square, grid.ModuleShape);
    }

    [Fact]
    public void VersionTwoGridIsTwentyFiveByTwentyFive()
    {
        var grid = QRCodeEncoder.Encode(new string('1', 42), EccLevel.L);

        Assert.Equal(25, grid.Rows);
        Assert.Equal(25, grid.Cols);
    }

    [Fact]
    public void VersionFiveGridIsThirtySevenByThirtySeven()
    {
        var grid = QRCodeEncoder.Encode(new string('1', 188), EccLevel.L);

        Assert.Equal(37, grid.Rows);
        Assert.Equal(37, grid.Cols);
    }

    [Fact]
    public void LargeNumericInputRequiresVersionTenOrHigher()
    {
        var grid = QRCodeEncoder.Encode(new string('1', 600), EccLevel.L);

        Assert.True(grid.Rows >= 57);
    }

    [Fact]
    public void FinderPatternsArePresent()
    {
        var grid = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);
        var size = grid.Rows;

        Assert.True(HasFinder(grid, 0, 0));
        Assert.True(HasFinder(grid, 0, size - 7));
        Assert.True(HasFinder(grid, size - 7, 0));
    }

    [Fact]
    public void VersionFiveFinderPatternsArePresent()
    {
        var grid = QRCodeEncoder.Encode(new string('1', 188), EccLevel.L);
        var size = grid.Rows;

        Assert.True(HasFinder(grid, 0, 0));
        Assert.True(HasFinder(grid, 0, size - 7));
        Assert.True(HasFinder(grid, size - 7, 0));
    }

    [Theory]
    [InlineData("HELLO WORLD", EccLevel.M, 0b00u)]
    [InlineData("12345", EccLevel.L, 0b01u)]
    [InlineData("TEST", EccLevel.Q, 0b11u)]
    [InlineData("ABC", EccLevel.H, 0b10u)]
    public void FormatInfoBchIsValidAndEccMatches(string input, EccLevel ecc, uint expectedIndicator)
    {
        var grid = QRCodeEncoder.Encode(input, ecc);
        var format = ReadFormatInfo(grid);

        Assert.NotNull(format);
        Assert.Equal(expectedIndicator, format.Value.Indicator);
    }

    [Fact]
    public void DarkModuleIsPresentForVersionOneAndFive()
    {
        var versionOne = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);
        var versionFive = QRCodeEncoder.Encode(new string('1', 188), EccLevel.L);

        Assert.True(versionOne.Modules[13][8]);
        Assert.True(versionFive.Modules[29][8]);
    }

    [Fact]
    public void NumericAndAlphanumericInputsStayInVersionOneAtEccM()
    {
        Assert.Equal(21, QRCodeEncoder.Encode("01234567", EccLevel.M).Rows);
        Assert.Equal(21, QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M).Rows);
    }

    [Fact]
    public void LowercaseInputUsesByteModeAndStillEncodes()
    {
        var grid = QRCodeEncoder.Encode("hello", EccLevel.M);

        Assert.Equal(21, grid.Rows);
    }

    [Fact]
    public void MediumInputSelectsHigherVersion()
    {
        var grid = QRCodeEncoder.Encode(new string('3', 300), EccLevel.L);

        Assert.True(grid.Rows > 21);
    }

    [Fact]
    public void AllEccLevelsEncode()
    {
        foreach (var ecc in new[] { EccLevel.L, EccLevel.M, EccLevel.Q, EccLevel.H })
        {
            var grid = QRCodeEncoder.Encode("HELLO", ecc);

            Assert.True(grid.Rows >= 21);
        }
    }

    [Fact]
    public void EmptyStringEncodes()
    {
        var grid = QRCodeEncoder.Encode(string.Empty, EccLevel.M);

        Assert.Equal(21, grid.Rows);
    }

    [Fact]
    public void OverlyLongInputThrows()
    {
        Assert.Throws<InputTooLongException>(() => QRCodeEncoder.Encode(new string('A', 8000), EccLevel.H));
    }

    [Fact]
    public void ExactByteModeOverflowThrows()
    {
        Assert.Throws<InputTooLongException>(() => QRCodeEncoder.Encode(new string('a', 7089), EccLevel.L));
    }

    [Fact]
    public void NullInputThrows()
    {
        Assert.Throws<ArgumentNullException>(() => QRCodeEncoder.Encode(null!, EccLevel.M));
    }

    [Fact]
    public void ModuleGridDimensionsMatchRowsAndCols()
    {
        var grid = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);

        Assert.Equal(grid.Rows, grid.Modules.Count);
        foreach (var row in grid.Modules)
        {
            Assert.Equal(grid.Cols, row.Count);
        }
    }

    [Fact]
    public void TimingPatternsAlternate()
    {
        var grid = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);
        var size = grid.Rows;

        for (var col = 8; col <= size - 9; col++)
        {
            Assert.Equal(col % 2 == 0, grid.Modules[6][col]);
        }

        for (var row = 8; row <= size - 9; row++)
        {
            Assert.Equal(row % 2 == 0, grid.Modules[row][6]);
        }
    }

    [Fact]
    public void VersionSevenOrHigherEncodes()
    {
        var grid = QRCodeEncoder.Encode(new string('a', 155), EccLevel.L);

        Assert.True(grid.Rows >= 45);
    }

    [Fact]
    public void HighVersionFormatInfoIsValid()
    {
        var grid = QRCodeEncoder.Encode(new string('1', 280), EccLevel.L);

        Assert.NotNull(ReadFormatInfo(grid));
    }

    [Fact]
    public void FullAlphanumericCharsetEncodes()
    {
        var grid = QRCodeEncoder.Encode("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:", EccLevel.L);

        Assert.True(grid.Rows >= 21);
    }

    [Theory]
    [InlineData("0")]
    [InlineData("42")]
    [InlineData("123")]
    [InlineData("000")]
    public void NumericEdgeCasesEncode(string input)
    {
        Assert.Equal(21, QRCodeEncoder.Encode(input, EccLevel.L).Rows);
    }

    [Fact]
    public void UrlAndMixedCaseInputsEncode()
    {
        Assert.True(QRCodeEncoder.Encode("https://example.com", EccLevel.M).Rows >= 21);
        Assert.True(QRCodeEncoder.Encode("Hello World", EccLevel.M).Rows >= 21);
    }

    [Fact]
    public void Utf8ByteInputEncodes()
    {
        var grid = QRCodeEncoder.Encode("caf\u00e9", EccLevel.M);

        Assert.True(grid.Rows >= 21);
    }

    [Fact]
    public void SeparatorRowsAreLightAroundTopLeftFinder()
    {
        var grid = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);

        for (var col = 0; col <= 7; col++)
        {
            Assert.False(grid.Modules[7][col]);
        }

        for (var row = 0; row <= 7; row++)
        {
            Assert.False(grid.Modules[row][7]);
        }
    }

    [Fact]
    public void GridContainsBothDarkAndLightModules()
    {
        var grid = QRCodeEncoder.Encode("TEST", EccLevel.H);
        var dark = 0;
        var light = 0;

        for (var row = 0; row < grid.Rows; row++)
        {
            for (var col = 0; col < grid.Cols; col++)
            {
                if (grid.Modules[row][col])
                {
                    dark++;
                }
                else
                {
                    light++;
                }
            }
        }

        Assert.True(dark > 0);
        Assert.True(light > 0);
    }

    [Fact]
    public void EccIndicatorsAreValid()
    {
        Assert.Equal(0b11u, ReadFormatInfo(QRCodeEncoder.Encode("HELLO WORLD", EccLevel.Q))!.Value.Indicator);
        Assert.Equal(0b10u, ReadFormatInfo(QRCodeEncoder.Encode("HELLO WORLD", EccLevel.H))!.Value.Indicator);
        Assert.Equal(0b01u, ReadFormatInfo(QRCodeEncoder.Encode("HELLO WORLD", EccLevel.L))!.Value.Indicator);
    }

    [Fact]
    public void EncodingIsDeterministic()
    {
        var first = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);
        var second = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);

        Assert.Equal(first.Rows, second.Rows);
        for (var row = 0; row < first.Rows; row++)
        {
            for (var col = 0; col < first.Cols; col++)
            {
                Assert.Equal(first.Modules[row][col], second.Modules[row][col]);
            }
        }
    }

    private static bool HasFinder(ModuleGrid grid, int top, int left)
    {
        for (var dr = 0; dr <= 6; dr++)
        {
            for (var dc = 0; dc <= 6; dc++)
            {
                var onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                var inCore = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
                if (grid.Modules[top + dr][left + dc] != (onBorder || inCore))
                {
                    return false;
                }
            }
        }

        return true;
    }

    private static (uint Indicator, uint Mask)? ReadFormatInfo(ModuleGrid grid)
    {
        (int Row, int Col)[] positions =
        [
            (8, 0), (8, 1), (8, 2), (8, 3), (8, 4), (8, 5), (8, 7), (8, 8),
            (7, 8), (5, 8), (4, 8), (3, 8), (2, 8), (1, 8), (0, 8),
        ];

        var raw = 0u;
        for (var i = 0; i < positions.Length; i++)
        {
            var (row, col) = positions[i];
            if (grid.Modules[row][col])
            {
                raw |= 1u << (14 - i);
            }
        }

        var format = raw ^ 0x5412u;
        var rem = (format >> 10) << 10;
        for (var i = 14; i >= 10; i--)
        {
            if (((rem >> i) & 1u) == 1u)
            {
                rem ^= 0x537u << (i - 10);
            }
        }

        if ((rem & 0x3FFu) != (format & 0x3FFu))
        {
            return null;
        }

        return ((format >> 13) & 0x3u, (format >> 10) & 0x7u);
    }
}
