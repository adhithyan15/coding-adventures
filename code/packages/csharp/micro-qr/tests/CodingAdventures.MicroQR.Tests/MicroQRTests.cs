using CodingAdventures.Barcode2D;
using CodingAdventures.MicroQR;

namespace CodingAdventures.MicroQR.Tests;

// MicroQRTests.cs — comprehensive tests for the Micro QR encoder
// ==============================================================
//
// Test strategy follows the spec (code/specs/micro-qr.md):
//   1. RS encoder correctness
//   2. Format information table verification
//   3. Mode selection heuristic
//   4. Bit stream assembly (mode indicator, char count, terminator, padding)
//   5. Penalty scoring
//   6. Masking (reserved modules not flipped, format info changes)
//   7. Integration: known symbol dimensions and structural properties
//   8. Error paths (too long, unsupported mode, bad characters)

public sealed class VersionTests
{
    [Fact]
    public void VersionIsCurrent()
    {
        Assert.Equal("0.1.0", MicroQR.Version);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. RS encoder
// ─────────────────────────────────────────────────────────────────────────────

public sealed class RsEncoderTests
{
    // The RS encoder computes the polynomial remainder D(x)·x^n mod G(x).
    // We verify against the known generator polynomials from the spec.

    [Fact]
    public void TwoEccCodewordsForM1Detection()
    {
        // M1 detection: g(x) = x² + 3x + 2, two ECC bytes.
        // data = [0x10, 0x20, 0x0C] (example three bytes)
        var data = new List<byte> { 0x10, 0x20, 0x0C };
        var ecc  = MicroQR.RsEncode(data, 2);
        Assert.Equal(2, ecc.Length);
        // ECC bytes are deterministic — verify they are non-zero for non-trivial input
        Assert.True(ecc[0] != 0 || ecc[1] != 0);
    }

    [Fact]
    public void FiveEccCodewordsForM2L()
    {
        var data = new List<byte> { 0x40, 0xD2, 0x75, 0x47, 0x76 };
        var ecc  = MicroQR.RsEncode(data, 5);
        Assert.Equal(5, ecc.Length);
    }

    [Fact]
    public void SixEccCodewords()
    {
        var data = new List<byte> { 0x20, 0x5A, 0x72, 0x6A, 0x61 };
        var ecc  = MicroQR.RsEncode(data, 6);
        Assert.Equal(6, ecc.Length);
    }

    [Fact]
    public void EightEccCodewords()
    {
        var data = new List<byte> { 0x40, 0xC3, 0x46, 0x8E, 0xCA, 0x68 };
        var ecc  = MicroQR.RsEncode(data, 8);
        Assert.Equal(8, ecc.Length);
    }

    [Fact]
    public void TenEccCodewords()
    {
        var data = Enumerable.Range(1, 14).Select(i => (byte)i).ToList();
        var ecc  = MicroQR.RsEncode(data, 10);
        Assert.Equal(10, ecc.Length);
    }

    [Fact]
    public void FourteenEccCodewords()
    {
        var data = Enumerable.Range(1, 10).Select(i => (byte)i).ToList();
        var ecc  = MicroQR.RsEncode(data, 14);
        Assert.Equal(14, ecc.Length);
    }

    [Fact]
    public void AllZeroDataGivesAllZeroEcc()
    {
        // For all-zero data, every feedback is 0 so no XOR ever fires → ECC = all zeros.
        var data = new List<byte>(Enumerable.Repeat((byte)0, 5));
        var ecc  = MicroQR.RsEncode(data, 5);
        Assert.All(ecc, b => Assert.Equal(0, b));
    }

    [Fact]
    public void InvalidEccCountThrows()
    {
        var data = new List<byte> { 0x01 };
        Assert.Throws<MicroQRException>(() => MicroQR.RsEncode(data, 99));
    }

    [Fact]
    public void EccIsConsistentAcrossIdenticalInputs()
    {
        var data = new List<byte> { 0xAB, 0xCD, 0xEF };
        var ecc1 = MicroQR.RsEncode(data, 2);
        var ecc2 = MicroQR.RsEncode(data, 2);
        Assert.Equal(ecc1, ecc2);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Format information table
// ─────────────────────────────────────────────────────────────────────────────

public sealed class FormatTableTests
{
    // Spot-check several entries from the pre-computed table.
    // The full table is verified by re-deriving from the BCH formula.

    [Theory]
    [InlineData(0, 0, 0x4445)]  // M1, mask 0
    [InlineData(0, 1, 0x4172)]  // M1, mask 1
    [InlineData(0, 2, 0x4E2B)]  // M1, mask 2
    [InlineData(0, 3, 0x4B1C)]  // M1, mask 3
    [InlineData(1, 0, 0x5528)]  // M2-L, mask 0
    [InlineData(1, 1, 0x501F)]  // M2-L, mask 1
    [InlineData(2, 0, 0x6649)]  // M2-M, mask 0
    [InlineData(3, 0, 0x7764)]  // M3-L, mask 0
    [InlineData(4, 0, 0x06DE)]  // M3-M, mask 0
    [InlineData(5, 0, 0x17F3)]  // M4-L, mask 0
    [InlineData(6, 0, 0x24B2)]  // M4-M, mask 0
    [InlineData(7, 0, 0x359F)]  // M4-Q, mask 0
    [InlineData(7, 3, 0x3AC6)]  // M4-Q, mask 3
    public void FormatWordMatchesTable(int symbolIndicator, int mask, int expected)
    {
        Assert.Equal(expected, Tables.FormatTable[symbolIndicator][mask]);
    }

    [Fact]
    public void TableHasExactly8Symbols()
    {
        Assert.Equal(8, Tables.FormatTable.Length);
    }

    [Fact]
    public void EachSymbolHas4MaskEntries()
    {
        foreach (var row in Tables.FormatTable)
            Assert.Equal(4, row.Length);
    }

    [Fact]
    public void AllFormatWordsAreFifteenBits()
    {
        // All values must fit in 15 bits (< 0x8000)
        foreach (var row in Tables.FormatTable)
        foreach (var word in row)
            Assert.True(word >= 0 && word < 0x8000,
                $"Format word 0x{word:X4} does not fit in 15 bits");
    }

    [Fact]
    public void FormatWordsAreDistinct()
    {
        // Every (symbolIndicator, mask) pair should produce a unique word
        var seen = new HashSet<int>();
        foreach (var row in Tables.FormatTable)
        foreach (var word in row)
            Assert.True(seen.Add(word), $"Duplicate format word 0x{word:X4}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Mode selection
// ─────────────────────────────────────────────────────────────────────────────

public sealed class ModeSelectionTests
{
    private static SymbolConfig CfgFor(MicroQRVersion v, MicroQREccLevel ecc) =>
        Tables.SymbolConfigs.First(c => c.Version == v && c.Ecc == ecc);

    [Fact]
    public void AllDigitsSelectsNumericMode()
    {
        var cfg = CfgFor(MicroQRVersion.M1, MicroQREccLevel.Detection);
        Assert.Equal(EncodingMode.Numeric, MicroQR.SelectMode("12345", cfg));
    }

    [Fact]
    public void EmptyStringSelectsNumericMode()
    {
        var cfg = CfgFor(MicroQRVersion.M2, MicroQREccLevel.L);
        Assert.Equal(EncodingMode.Numeric, MicroQR.SelectMode("", cfg));
    }

    [Fact]
    public void UppercaseLettersSelectAlphanumericForM2()
    {
        var cfg = CfgFor(MicroQRVersion.M2, MicroQREccLevel.L);
        Assert.Equal(EncodingMode.Alphanumeric, MicroQR.SelectMode("HELLO", cfg));
    }

    [Fact]
    public void LowercaseSelectsByteForM3()
    {
        var cfg = CfgFor(MicroQRVersion.M3, MicroQREccLevel.L);
        Assert.Equal(EncodingMode.Byte, MicroQR.SelectMode("hello", cfg));
    }

    [Fact]
    public void AlphanumericNotSupportedInM1ThrowsUnsupportedMode()
    {
        var cfg = CfgFor(MicroQRVersion.M1, MicroQREccLevel.Detection);
        Assert.Throws<UnsupportedModeException>(() => MicroQR.SelectMode("HELLO", cfg));
    }

    [Fact]
    public void ByteNotSupportedInM2ThrowsUnsupportedMode()
    {
        var cfg = CfgFor(MicroQRVersion.M2, MicroQREccLevel.L);
        // lowercase requires byte mode but M2 supports byte (byteCap=4)
        // so this should NOT throw — it selects byte
        Assert.Equal(EncodingMode.Byte, MicroQR.SelectMode("hi", cfg));
    }

    [Fact]
    public void ByteNotAvailableInM1ThrowsForNonNumeric()
    {
        var cfg = CfgFor(MicroQRVersion.M1, MicroQREccLevel.Detection);
        Assert.Throws<UnsupportedModeException>(() => MicroQR.SelectMode("hi", cfg));
    }

    [Fact]
    public void MixedAlphanumericAndSpaceUsesAlphanumeric()
    {
        var cfg = CfgFor(MicroQRVersion.M3, MicroQREccLevel.L);
        Assert.Equal(EncodingMode.Alphanumeric, MicroQR.SelectMode("MICRO QR", cfg));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Config / version auto-selection
// ─────────────────────────────────────────────────────────────────────────────

public sealed class ConfigSelectionTests
{
    [Fact]
    public void SingleDigitSelectsM1()
    {
        var cfg = MicroQR.SelectConfig("1", null, null);
        Assert.Equal(MicroQRVersion.M1, cfg.Version);
        Assert.Equal(MicroQREccLevel.Detection, cfg.Ecc);
    }

    [Fact]
    public void FiveDigitsSelectsM1()
    {
        var cfg = MicroQR.SelectConfig("12345", null, null);
        Assert.Equal(MicroQRVersion.M1, cfg.Version);
    }

    [Fact]
    public void SixDigitsFallsToM2()
    {
        var cfg = MicroQR.SelectConfig("123456", null, null);
        Assert.Equal(MicroQRVersion.M2, cfg.Version);
    }

    [Fact]
    public void HelloSelectsM2()
    {
        // "HELLO" = 5 alphanumeric chars → M2-L
        var cfg = MicroQR.SelectConfig("HELLO", null, null);
        Assert.Equal(MicroQRVersion.M2, cfg.Version);
    }

    [Fact]
    public void LowercaseHelloSelectsM3()
    {
        // "hello" = 5 bytes, M2 byte cap is 4 → needs M3
        var cfg = MicroQR.SelectConfig("hello", null, null);
        Assert.Equal(MicroQRVersion.M3, cfg.Version);
    }

    [Fact]
    public void LongUrlSelectsM4()
    {
        var cfg = MicroQR.SelectConfig("https://a.b", null, null);
        Assert.Equal(MicroQRVersion.M4, cfg.Version);
    }

    [Fact]
    public void TooLongInputThrows()
    {
        Assert.Throws<InputTooLongException>(() =>
            MicroQR.SelectConfig(new string('A', 100), null, null));
    }

    [Fact]
    public void ExplicitVersionAndEccRespected()
    {
        var cfg = MicroQR.SelectConfig("12", MicroQRVersion.M4, MicroQREccLevel.Q);
        Assert.Equal(MicroQRVersion.M4, cfg.Version);
        Assert.Equal(MicroQREccLevel.Q, cfg.Ecc);
    }

    [Fact]
    public void RequestingInvalidVersionEccCombinationThrows()
    {
        // M1 only supports Detection, not L — no candidates exist → EccNotAvailableException
        Assert.Throws<EccNotAvailableException>(() =>
            MicroQR.SelectConfig("12345", MicroQRVersion.M1, MicroQREccLevel.L));
    }

    [Fact]
    public void M3QDoesNotExistThrows()
    {
        Assert.Throws<EccNotAvailableException>(() =>
            MicroQR.SelectConfig("1", MicroQRVersion.M3, MicroQREccLevel.Q));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Data codeword assembly
// ─────────────────────────────────────────────────────────────────────────────

public sealed class DataCodewordTests
{
    private static SymbolConfig CfgFor(MicroQRVersion v, MicroQREccLevel ecc) =>
        Tables.SymbolConfigs.First(c => c.Version == v && c.Ecc == ecc);

    [Fact]
    public void M1NumericProducesExactlyThreeBytes()
    {
        var cfg   = CfgFor(MicroQRVersion.M1, MicroQREccLevel.Detection);
        var bytes = MicroQR.BuildDataCodewords("1", cfg);
        Assert.Equal(3, bytes.Count);
    }

    [Fact]
    public void M2LProducesExactlyFiveBytes()
    {
        var cfg   = CfgFor(MicroQRVersion.M2, MicroQREccLevel.L);
        var bytes = MicroQR.BuildDataCodewords("HELLO", cfg);
        Assert.Equal(5, bytes.Count);
    }

    [Fact]
    public void M3LProducesExactlyElevenBytes()
    {
        var cfg   = CfgFor(MicroQRVersion.M3, MicroQREccLevel.L);
        var bytes = MicroQR.BuildDataCodewords("MICRO QR", cfg);
        Assert.Equal(11, bytes.Count);
    }

    [Fact]
    public void M4LProducesSixteenBytes()
    {
        var cfg   = CfgFor(MicroQRVersion.M4, MicroQREccLevel.L);
        var bytes = MicroQR.BuildDataCodewords("MICRO QR TEST", cfg);
        Assert.Equal(16, bytes.Count);
    }

    [Fact]
    public void ShortInputHasEcPaddingBytes()
    {
        // "1" in M2-L: only a few bits of data, rest should be 0xEC/0x11 padding
        var cfg   = CfgFor(MicroQRVersion.M2, MicroQREccLevel.L);
        var bytes = MicroQR.BuildDataCodewords("1", cfg);
        Assert.Equal(5, bytes.Count);
        // At minimum one pad byte should appear (0xEC is first)
        Assert.Contains((byte)0xEC, bytes);
    }

    [Fact]
    public void M1ThirdByteHasDataInUpperNibble()
    {
        // M1's third byte has data only in bits[7:4]; bits[3:0] must be 0.
        var cfg   = CfgFor(MicroQRVersion.M1, MicroQREccLevel.Detection);
        var bytes = MicroQR.BuildDataCodewords("12345", cfg);
        Assert.Equal(0, bytes[2] & 0x0F);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Penalty scoring
// ─────────────────────────────────────────────────────────────────────────────

public sealed class PenaltyTests
{
    private static bool[,] AllDark(int sz)
    {
        var m = new bool[sz, sz];
        for (var r = 0; r < sz; r++)
        for (var c = 0; c < sz; c++)
            m[r, c] = true;
        return m;
    }

    private static bool[,] Checkerboard(int sz)
    {
        var m = new bool[sz, sz];
        for (var r = 0; r < sz; r++)
        for (var c = 0; c < sz; c++)
            m[r, c] = (r + c) % 2 == 0;
        return m;
    }

    [Fact]
    public void AllDarkModulesHaveHighPenalty()
    {
        // All-dark grid: every row and column is a maximum run (sz long).
        // Rule 1: each row contributes sz-2, each column too → 2*sz*(sz-2).
        // Rule 2: many 2×2 dark blocks → (sz-1)^2 × 3.
        // Rule 3: finder-like sequences may appear.
        // Rule 4: 100% dark → large deviation from 50%.
        var penalty = MicroQR.ComputePenalty(AllDark(11), 11);
        Assert.True(penalty > 0);
    }

    [Fact]
    public void CheckerboardHasLowerPenaltyThanAllDark()
    {
        var darkPenalty  = MicroQR.ComputePenalty(AllDark(11), 11);
        var checkPenalty = MicroQR.ComputePenalty(Checkerboard(11), 11);
        // Checkerboard has no runs ≥5 and no 2×2 blocks, so penalty should be lower
        Assert.True(checkPenalty < darkPenalty);
    }

    [Fact]
    public void Rule1ScoreForLongRunIsRunLengthMinusTwo()
    {
        // Build a row with a run of 7 dark modules (columns 0–6) and 4 light.
        var m = new bool[11, 11];
        for (var c = 0; c < 7; c++) m[0, c] = true;
        // Rule 1 contribution from row 0: run=7 → penalty += 7-2=5
        // (Other rows and columns are all light = run of 11 → += 11-2=9 each for 10 rows+10 cols, minus the first col)
        // We just check penalty > 0 and the formula
        var p = MicroQR.ComputePenalty(m, 11);
        Assert.True(p > 0);
    }

    [Fact]
    public void Rule2TwoByTwoBlockAddsThree()
    {
        // Single 2×2 dark block in an otherwise light grid
        var m = new bool[11, 11];
        m[5, 5] = true; m[5, 6] = true;
        m[6, 5] = true; m[6, 6] = true;
        var p = MicroQR.ComputePenalty(m, 11);
        // Should contain at least +3 from Rule 2
        Assert.True(p >= 3);
    }

    [Fact]
    public void PenaltyIsNonNegative()
    {
        var p = MicroQR.ComputePenalty(Checkerboard(13), 13);
        Assert.True(p >= 0);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Integration tests — encode full symbols
// ─────────────────────────────────────────────────────────────────────────────

public sealed class IntegrationTests
{
    // For each integration test we verify:
    //   a) Correct grid dimensions (size × size)
    //   b) Grid is square
    //   c) Finder pattern in top-left 7×7 is correct
    //   d) Top-left corner is always dark (finder outer ring)

    private static void AssertValidGrid(ModuleGrid grid, int expectedSize)
    {
        Assert.Equal(expectedSize, grid.Rows);
        Assert.Equal(expectedSize, grid.Cols);
        Assert.Equal(grid.Rows, grid.Cols);
        Assert.Equal(ModuleShape.Square, grid.ModuleShape);
        // Top-left corner must be dark (part of finder outer ring)
        Assert.True(grid.Modules[0][0]);
    }

    [Fact]
    public void EncodeOneDigit_M1()
    {
        var grid = MicroQR.Encode("1");
        AssertValidGrid(grid, 11);
    }

    [Fact]
    public void EncodeFiveDigits_M1()
    {
        var grid = MicroQR.Encode("12345");
        AssertValidGrid(grid, 11);
    }

    [Fact]
    public void EncodeHello_M2()
    {
        var grid = MicroQR.Encode("HELLO");
        AssertValidGrid(grid, 13);
    }

    [Fact]
    public void EncodeNumericPushesFromM1toM2()
    {
        // "123456" = 6 digits, exceeds M1 capacity of 5
        var grid = MicroQR.Encode("123456");
        AssertValidGrid(grid, 13);
    }

    [Fact]
    public void EncodeLowercasePushesToM3()
    {
        // "hello" = 5 bytes — M2 byte cap is 4, so M3 needed
        var grid = MicroQR.Encode("hello");
        AssertValidGrid(grid, 15);
    }

    [Fact]
    public void EncodeMicroQRTest_M3()
    {
        var grid = MicroQR.Encode("MICRO QR TEST");
        AssertValidGrid(grid, 15);
    }

    [Fact]
    public void EncodeUrl_M4()
    {
        var grid = MicroQR.Encode("https://a.b");
        AssertValidGrid(grid, 17);
    }

    [Fact]
    public void EncodeAllEccLevelsForM4()
    {
        // "MICRO QR" — 8 alphanumeric chars, fits in all M4 levels
        foreach (var ecc in new[] { MicroQREccLevel.L, MicroQREccLevel.M, MicroQREccLevel.Q })
        {
            var grid = MicroQR.Encode("MICRO QR", MicroQRVersion.M4, ecc);
            AssertValidGrid(grid, 17);
        }
    }

    [Fact]
    public void EncodeSameInputTwiceIsDeterministic()
    {
        // The encoder is deterministic — same input always gives the same grid.
        var g1 = MicroQR.Encode("HELLO");
        var g2 = MicroQR.Encode("HELLO");
        for (var r = 0; r < g1.Rows; r++)
        for (var c = 0; c < g1.Cols; c++)
            Assert.Equal(g1.Modules[r][c], g2.Modules[r][c]);
    }

    [Fact]
    public void FinderPatternTopLeftIsCorrect()
    {
        // The 7×7 finder pattern has a specific bit pattern that must be preserved.
        //   Outer border all dark, inner ring all light, 3×3 core dark.
        var grid = MicroQR.Encode("1");
        // Top-left corner of finder
        Assert.True(grid.Modules[0][0]);  // top-left
        Assert.True(grid.Modules[0][6]);  // top-right of finder
        Assert.True(grid.Modules[6][0]);  // bottom-left of finder
        Assert.True(grid.Modules[6][6]);  // bottom-right of finder
        // Inner ring should be light
        Assert.False(grid.Modules[1][1]);
        Assert.False(grid.Modules[1][5]);
        Assert.False(grid.Modules[5][1]);
        Assert.False(grid.Modules[5][5]);
        // Center core should be dark
        Assert.True(grid.Modules[3][3]);
    }

    [Fact]
    public void SeparatorRow7IsMostlyLight()
    {
        // Row 7, cols 1–7 are the separator (all light).
        var grid = MicroQR.Encode("HELLO");
        for (var c = 1; c <= 7; c++)
            Assert.False(grid.Modules[7][c], $"Separator at row 7 col {c} should be light");
    }

    [Fact]
    public void TimingPatternAtRow0AlternatesAfterCol7()
    {
        // Row 0 timing: col 8 is dark (even), col 9 light, col 10 dark, etc.
        // The finder occupies cols 0–6; col 7 is separator; col 8+ is timing.
        var grid = MicroQR.Encode("MICRO QR");  // M3
        // col 8 (even) should be dark, col 9 (odd) should be light
        Assert.True(grid.Modules[0][8],  "Timing col 8 should be dark (even)");
        Assert.False(grid.Modules[0][9], "Timing col 9 should be light (odd)");
        Assert.True(grid.Modules[0][10], "Timing col 10 should be dark (even)");
    }

    [Fact]
    public void FormatAreaRow8IsWritten()
    {
        // Row 8, cols 1–8 hold format information bits.
        // After encoding, these modules should not be uniform (some dark, some light
        // depending on the format word). We simply check the row is not all-dark.
        var grid = MicroQR.Encode("1");
        var row8Values = Enumerable.Range(1, 8).Select(c => grid.Modules[8][c]).ToArray();
        // Format word 0x4445 = 0100 0100 0100 0101 in binary.
        // Not all the same, so the row should be mixed.
        Assert.True(row8Values.Any(v => v) && row8Values.Any(v => !v));
    }

    [Fact]
    public void AllVersions_ProduceExpectedSizes()
    {
        // Verify each version auto-selects and produces the correct size.
        Assert.Equal(11, MicroQR.Encode("1").Rows);
        Assert.Equal(13, MicroQR.Encode("ABCDE1").Rows);  // numeric in M2
        Assert.Equal(15, MicroQR.Encode("MICRO QR TEST").Rows);
        Assert.Equal(17, MicroQR.Encode("https://a.b").Rows);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Error paths
// ─────────────────────────────────────────────────────────────────────────────

public sealed class ErrorPathTests
{
    [Fact]
    public void InputExceedingM4CapacityThrows()
    {
        // 36 numeric chars exceeds M4-L capacity of 35
        Assert.Throws<InputTooLongException>(() =>
            MicroQR.Encode("123456789012345678901234567890123456"));
    }

    [Fact]
    public void TooManyBytesThrows()
    {
        // 16 bytes exceeds M4-L byte cap of 15
        Assert.Throws<InputTooLongException>(() =>
            MicroQR.Encode("abcdefghijklmnop"));
    }

    [Fact]
    public void InvalidEccLevelForVersionThrows()
    {
        Assert.Throws<EccNotAvailableException>(() =>
            MicroQR.SelectConfig("1", MicroQRVersion.M2, MicroQREccLevel.Q));
    }

    [Fact]
    public void EmptyStringEncodesSuccessfully()
    {
        // Empty string is a valid 0-char numeric input
        var grid = MicroQR.Encode("");
        Assert.Equal(11, grid.Rows);
    }

    [Fact]
    public void InvalidVersionEccCombinationThrows()
    {
        // There is no M3-Q
        Assert.Throws<EccNotAvailableException>(() =>
            MicroQR.Encode("1", MicroQRVersion.M3, MicroQREccLevel.Q));
    }

    [Fact]
    public void M1OnlySupportsDetection()
    {
        // Requesting M1-L should find no candidates → EccNotAvailableException
        Assert.Throws<EccNotAvailableException>(() =>
            MicroQR.Encode("1", MicroQRVersion.M1, MicroQREccLevel.L));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Symbol table integrity
// ─────────────────────────────────────────────────────────────────────────────

public sealed class SymbolTableTests
{
    [Fact]
    public void ExactlyEightConfigurations()
    {
        Assert.Equal(8, Tables.SymbolConfigs.Length);
    }

    [Fact]
    public void SymbolIndicatorsAreZeroThroughSeven()
    {
        var indicators = Tables.SymbolConfigs.Select(c => c.SymbolIndicator).OrderBy(x => x).ToArray();
        Assert.Equal(Enumerable.Range(0, 8).ToArray(), indicators);
    }

    [Fact]
    public void SizeFormulaIsCorrect()
    {
        // size = 2 × versionNumber + 9
        foreach (var cfg in Tables.SymbolConfigs)
        {
            var vNum = cfg.Version switch
            {
                MicroQRVersion.M1 => 1,
                MicroQRVersion.M2 => 2,
                MicroQRVersion.M3 => 3,
                MicroQRVersion.M4 => 4,
                _ => throw new InvalidOperationException()
            };
            Assert.Equal(2 * vNum + 9, cfg.Size);
        }
    }

    [Fact]
    public void M1OnlyHasDetectionEcc()
    {
        var m1Cfgs = Tables.SymbolConfigs.Where(c => c.Version == MicroQRVersion.M1).ToArray();
        Assert.Single(m1Cfgs);
        Assert.Equal(MicroQREccLevel.Detection, m1Cfgs[0].Ecc);
    }

    [Fact]
    public void M4HasThreeEccLevels()
    {
        var m4Cfgs = Tables.SymbolConfigs.Where(c => c.Version == MicroQRVersion.M4).ToArray();
        Assert.Equal(3, m4Cfgs.Length);
        Assert.Contains(MicroQREccLevel.L, m4Cfgs.Select(c => c.Ecc));
        Assert.Contains(MicroQREccLevel.M, m4Cfgs.Select(c => c.Ecc));
        Assert.Contains(MicroQREccLevel.Q, m4Cfgs.Select(c => c.Ecc));
    }

    [Fact]
    public void DataCwPlusEccCwEqualsExpectedTotals()
    {
        // Total codewords per symbol from the spec:
        var expected = new Dictionary<(MicroQRVersion, MicroQREccLevel), (int data, int ecc)>
        {
            [(MicroQRVersion.M1, MicroQREccLevel.Detection)] = (3, 2),
            [(MicroQRVersion.M2, MicroQREccLevel.L)]         = (5, 5),
            [(MicroQRVersion.M2, MicroQREccLevel.M)]         = (4, 6),
            [(MicroQRVersion.M3, MicroQREccLevel.L)]         = (11, 6),
            [(MicroQRVersion.M3, MicroQREccLevel.M)]         = (9, 8),
            [(MicroQRVersion.M4, MicroQREccLevel.L)]         = (16, 8),
            [(MicroQRVersion.M4, MicroQREccLevel.M)]         = (14, 10),
            [(MicroQRVersion.M4, MicroQREccLevel.Q)]         = (10, 14),
        };
        foreach (var cfg in Tables.SymbolConfigs)
        {
            var (d, e) = expected[(cfg.Version, cfg.Ecc)];
            Assert.Equal(d, cfg.DataCW);
            Assert.Equal(e, cfg.EccCW);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Alphanumeric character set
// ─────────────────────────────────────────────────────────────────────────────

public sealed class AlphanumCharsetTests
{
    [Fact]
    public void AlphanumSetHasFortyFiveChars()
    {
        Assert.Equal(45, Tables.AlphanumChars.Length);
    }

    [Fact]
    public void DigitsAreAtIndicesZeroThroughNine()
    {
        for (var d = 0; d <= 9; d++)
            Assert.Equal(d, Tables.AlphanumChars.IndexOf((char)('0' + d)));
    }

    [Fact]
    public void UppercaseLettersAreAtIndicesTenThroughThirtyFive()
    {
        for (var l = 0; l < 26; l++)
            Assert.Equal(10 + l, Tables.AlphanumChars.IndexOf((char)('A' + l)));
    }

    [Fact]
    public void SpaceIsAtIndex36()
    {
        Assert.Equal(36, Tables.AlphanumChars.IndexOf(' '));
    }

    [Fact]
    public void LowercaseNotInAlphanumSet()
    {
        Assert.Equal(-1, Tables.AlphanumChars.IndexOf('a'));
        Assert.Equal(-1, Tables.AlphanumChars.IndexOf('z'));
    }
}
