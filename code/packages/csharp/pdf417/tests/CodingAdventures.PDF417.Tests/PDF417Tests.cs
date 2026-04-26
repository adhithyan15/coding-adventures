using CodingAdventures.PDF417;
using CodingAdventures.Barcode2D;

// =============================================================================
// PDF417Tests.cs — Unit and integration tests for the PDF417 encoder
// =============================================================================
//
// Test strategy (mirrors the spec's Test Strategy section):
//
//   1. GF(929) arithmetic — field operations must be correct before RS works.
//   2. RS ECC encoding — generator polynomial and shift-register encoder.
//   3. Byte compaction — 6-bytes→5-codewords and single-byte remainder.
//   4. Row indicator computation — LRI/RRI values from the spec example.
//   5. Symbol dimensions — dimension selection heuristic.
//   6. Integration — full encode pipeline: module grid dimensions, start/stop.
//   7. Error handling — invalid options raise correct exceptions.
//
// =============================================================================

namespace CodingAdventures.PDF417.Tests;

// ─────────────────────────────────────────────────────────────────────────────
// 1. GF(929) arithmetic
// ─────────────────────────────────────────────────────────────────────────────

public class Gf929ArithmeticTests
{
    // ── Addition ─────────────────────────────────────────────────────────────

    [Fact]
    public void Add_NormalCase_SumsModulo929()
    {
        // (100 + 900) mod 929 = 1000 mod 929 = 71
        Assert.Equal(71, PDF417Encoder.GfAdd(100, 900));
    }

    [Fact]
    public void Add_NoWrap_ReturnsSum()
    {
        Assert.Equal(10, PDF417Encoder.GfAdd(3, 7));
    }

    [Fact]
    public void Add_WrapAround_IsZero()
    {
        // 929 mod 929 = 0
        Assert.Equal(0, PDF417Encoder.GfAdd(929 - 1, 1));
    }

    // ── Multiplication ───────────────────────────────────────────────────────

    [Fact]
    public void Mul_Zero_ReturnsZero()
    {
        Assert.Equal(0, PDF417Encoder.GfMul(0, 5));
        Assert.Equal(0, PDF417Encoder.GfMul(5, 0));
    }

    [Fact]
    public void Mul_Alpha3_Is27()
    {
        // α = 3, so α^3 = 27
        Assert.Equal(27, PDF417Encoder.GfMul(3, 9)); // 3 × 3^2 = 3^3 = 27
    }

    [Fact]
    public void Mul_400x400_CorrectModular()
    {
        // 400 × 400 = 160000; 160000 mod 929 = ?
        // 929 × 172 = 159788; 160000 - 159788 = 212
        int expected = (400 * 400) % 929;
        Assert.Equal(expected, PDF417Encoder.GfMul(400, 400));
    }

    [Fact]
    public void Mul_Commutativity()
    {
        Assert.Equal(PDF417Encoder.GfMul(3, 7), PDF417Encoder.GfMul(7, 3));
        Assert.Equal(PDF417Encoder.GfMul(100, 200), PDF417Encoder.GfMul(200, 100));
    }

    // ── Exp/Log tables ───────────────────────────────────────────────────────

    [Fact]
    public void ExpTable_Index0_Is1()
    {
        // α^0 = 1
        Assert.Equal(1, PDF417Encoder.GfExp[0]);
    }

    [Fact]
    public void ExpTable_Index1_IsAlpha()
    {
        // α^1 = 3 (primitive root)
        Assert.Equal(3, PDF417Encoder.GfExp[1]);
    }

    [Fact]
    public void ExpTable_RoundTrip_LogOfExp()
    {
        // exp[log[v]] == v for all v in 1..928
        for (int v = 1; v < 929; v++)
        {
            int idx = PDF417Encoder.GfLog[v];
            Assert.Equal(v, PDF417Encoder.GfExp[idx]);
        }
    }

    [Fact]
    public void ExpTable_Index928_Equals_Index0()
    {
        // The wrap-around entry: GfExp[928] = GfExp[0] = 1
        Assert.Equal(PDF417Encoder.GfExp[0], PDF417Encoder.GfExp[928]);
    }

    [Fact]
    public void ExpTable_Fermat_AlphaPow928_IsOne()
    {
        // By Fermat's little theorem: α^{p-1} ≡ 1 (mod p) for prime p=929.
        // α^928 mod 929 = 1.
        int val = 1;
        for (int i = 0; i < 928; i++)
            val = (val * 3) % 929;
        Assert.Equal(1, val);
    }

    // ── Inverse via mul ──────────────────────────────────────────────────────

    [Fact]
    public void Mul_3TimesIts_Inverse_Is1()
    {
        // inv(3) in GF(929): 3 × x ≡ 1 (mod 929) → x = 310
        // Verify: 3 × 310 = 930 ≡ 1 (mod 929) ✓
        Assert.Equal(1, PDF417Encoder.GfMul(3, 310));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. RS ECC encoding
// ─────────────────────────────────────────────────────────────────────────────

public class RsEccTests
{
    // ── Generator polynomial ─────────────────────────────────────────────────

    [Fact]
    public void BuildGenerator_Level0_HasDegree2()
    {
        // ECC level 0: k = 2, generator has k+1 = 3 coefficients.
        int[] g = PDF417Encoder.BuildGenerator(0);
        Assert.Equal(3, g.Length);
    }

    [Fact]
    public void BuildGenerator_Level0_LeadingCoefficientIs1()
    {
        int[] g = PDF417Encoder.BuildGenerator(0);
        Assert.Equal(1, g[0]);
    }

    [Fact]
    public void BuildGenerator_Level0_Matches_KnownValues()
    {
        // For ECC level 0, k=2, roots α^3=27 and α^4=81.
        // g(x) = (x - 27)(x - 81) = x^2 - 108x + 2187
        // In GF(929): 2187 mod 929 = 329, -108 mod 929 = 821.
        // So g = [1, 821, 329].
        int[] g = PDF417Encoder.BuildGenerator(0);
        Assert.Equal(1, g[0]);
        Assert.Equal(821, g[1]);
        Assert.Equal(329, g[2]);
    }

    [Fact]
    public void BuildGenerator_Level1_HasDegree4()
    {
        // ECC level 1: k = 4, generator has 5 coefficients.
        int[] g = PDF417Encoder.BuildGenerator(1);
        Assert.Equal(5, g.Length);
    }

    [Fact]
    public void BuildGenerator_Level2_HasDegree8()
    {
        // ECC level 2: k = 8.
        int[] g = PDF417Encoder.BuildGenerator(2);
        Assert.Equal(9, g.Length);
    }

    // ── Shift-register encoder ───────────────────────────────────────────────

    [Fact]
    public void RsEncode_Level0_Returns2EccCwords()
    {
        var data = new List<int> { 10, 20, 30 };
        var ecc = PDF417Encoder.RsEncode(data, 0);
        Assert.Equal(2, ecc.Count);
    }

    [Fact]
    public void RsEncode_Level2_Returns8EccCwords()
    {
        var data = new List<int> { 1, 2, 3, 4, 5 };
        var ecc = PDF417Encoder.RsEncode(data, 2);
        Assert.Equal(8, ecc.Count);
    }

    [Fact]
    public void RsEncode_AllEccCwordsInRange()
    {
        // All ECC codeword values must be in 0..928.
        var data = new List<int> { 100, 200, 300, 400, 500 };
        var ecc = PDF417Encoder.RsEncode(data, 2);
        foreach (int cw in ecc)
            Assert.InRange(cw, 0, 928);
    }

    [Fact]
    public void RsEncode_EmptyData_AllZeroEcc()
    {
        // Encoding zeros in GF(929) should yield zero ECC (g(0) feedback is 0).
        var data = new List<int> { 0, 0, 0 };
        var ecc = PDF417Encoder.RsEncode(data, 0);
        Assert.All(ecc, cw => Assert.Equal(0, cw));
    }

    [Fact]
    public void RsEncode_Deterministic_SameInputSameOutput()
    {
        var data = new List<int> { 50, 100, 150, 200 };
        var ecc1 = PDF417Encoder.RsEncode(data, 2);
        var ecc2 = PDF417Encoder.RsEncode(data, 2);
        Assert.Equal(ecc1, ecc2);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Byte compaction
// ─────────────────────────────────────────────────────────────────────────────

public class ByteCompactionTests
{
    [Fact]
    public void ByteCompact_StartsWithLatch924()
    {
        var result = PDF417Encoder.ByteCompact([0x41]);
        Assert.Equal(924, result[0]); // latch to byte mode
    }

    [Fact]
    public void ByteCompact_SingleByte_DirectMapping()
    {
        // A single byte 0xFF → codeword 255 (direct mapping after the latch).
        var result = PDF417Encoder.ByteCompact([0xFF]);
        Assert.Equal(2, result.Count);   // [924, 255]
        Assert.Equal(255, result[1]);
    }

    [Fact]
    public void ByteCompact_SixBytes_FiveCodewords()
    {
        // 6 bytes → 1 latch + 5 compacted codewords = 6 total.
        var result = PDF417Encoder.ByteCompact([0x41, 0x42, 0x43, 0x44, 0x45, 0x46]);
        Assert.Equal(6, result.Count);
    }

    [Fact]
    public void ByteCompact_SixBytes_AllCwordsInRange()
    {
        // Each compacted codeword must be 0..899 (base-900 digit).
        var result = PDF417Encoder.ByteCompact([0x41, 0x42, 0x43, 0x44, 0x45, 0x46]);
        // Skip the latch (index 0); indices 1..5 are the compacted codewords.
        for (int i = 1; i < result.Count; i++)
            Assert.InRange(result[i], 0, 899);
    }

    [Fact]
    public void ByteCompact_SevenBytes_SixCwordsAfterLatch()
    {
        // 7 bytes = 1 group of 6 (→5 codewords) + 1 byte (→1 codeword) = 6 + latch = 7.
        var result = PDF417Encoder.ByteCompact([0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47]);
        Assert.Equal(7, result.Count);
        // Last codeword is the raw byte value of 0x47 = 71.
        Assert.Equal(71, result[6]);
    }

    [Fact]
    public void ByteCompact_KnownSixBytes_RoundTrip()
    {
        // Verify the 6-byte group by re-expanding the 5 codewords back to 6 bytes.
        byte[] input = [0x41, 0x42, 0x43, 0x44, 0x45, 0x46];
        var result = PDF417Encoder.ByteCompact(input);

        // Reconstruct the original 6 bytes from the 5 codewords.
        ulong n = 0;
        for (int i = 1; i <= 5; i++)
            n = n * 900 + (ulong)result[i];

        byte[] recovered = new byte[6];
        for (int j = 5; j >= 0; j--)
        {
            recovered[j] = (byte)(n % 256);
            n /= 256;
        }

        Assert.Equal(input, recovered);
    }

    [Fact]
    public void ByteCompact_EmptyInput_OnlyLatch()
    {
        var result = PDF417Encoder.ByteCompact([]);
        Assert.Single(result);
        Assert.Equal(924, result[0]);
    }

    [Fact]
    public void ByteCompact_FiveBytes_FiveRemainderCwords()
    {
        // 5 bytes do not form a full 6-byte group → 5 direct codewords + latch.
        byte[] input = [1, 2, 3, 4, 5];
        var result = PDF417Encoder.ByteCompact(input);
        Assert.Equal(6, result.Count); // latch + 5 bytes
        for (int i = 0; i < 5; i++)
            Assert.Equal(input[i], result[i + 1]);
    }

    [Fact]
    public void ByteCompact_TwelveBytes_TwoGroups()
    {
        // 12 bytes = 2 groups of 6 → 2 × 5 = 10 codewords + latch = 11.
        var result = PDF417Encoder.ByteCompact(new byte[12]);
        Assert.Equal(11, result.Count);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Row indicator computation
// ─────────────────────────────────────────────────────────────────────────────

public class RowIndicatorTests
{
    // Spec example: R=10, C=3, ECC level L=2.
    //   R_info = (10-1)/3 = 3
    //   C_info = 3-1 = 2
    //   L_info = 3×2 + (10-1) mod 3 = 6+0 = 6
    //
    //   Row 0 (cluster 0): LRI = 30×0 + 3 = 3, RRI = 30×0 + 2 = 2

    private const int R = 10, C = 3, L = 2;

    [Fact]
    public void Lri_Row0_Cluster0_IsRInfo()
    {
        // Cluster 0: LRI = 30*(r/3) + R_info = 0 + 3 = 3
        Assert.Equal(3, PDF417Encoder.ComputeLri(0, R, C, L));
    }

    [Fact]
    public void Rri_Row0_Cluster0_IsCInfo()
    {
        // Cluster 0: RRI = 30*(r/3) + C_info = 0 + 2 = 2
        Assert.Equal(2, PDF417Encoder.ComputeRri(0, R, C, L));
    }

    [Fact]
    public void Lri_Row1_Cluster1_IsLInfo()
    {
        // Cluster 1: LRI = 30*(r/3) + L_info = 0 + 6 = 6
        Assert.Equal(6, PDF417Encoder.ComputeLri(1, R, C, L));
    }

    [Fact]
    public void Rri_Row1_Cluster1_IsRInfo()
    {
        // Cluster 1: RRI = 30*(r/3) + R_info = 0 + 3 = 3
        Assert.Equal(3, PDF417Encoder.ComputeRri(1, R, C, L));
    }

    [Fact]
    public void Lri_Row2_Cluster2_IsCInfo()
    {
        // Cluster 2: LRI = 30*(r/3) + C_info = 0 + 2 = 2
        Assert.Equal(2, PDF417Encoder.ComputeLri(2, R, C, L));
    }

    [Fact]
    public void Rri_Row2_Cluster2_IsLInfo()
    {
        // Cluster 2: RRI = 30*(r/3) + L_info = 0 + 6 = 6
        Assert.Equal(6, PDF417Encoder.ComputeRri(2, R, C, L));
    }

    [Fact]
    public void Lri_Row3_Cluster0_IncludesRowGroup()
    {
        // Row 3, cluster 0: LRI = 30*(3/3) + R_info = 30 + 3 = 33
        Assert.Equal(33, PDF417Encoder.ComputeLri(3, R, C, L));
    }

    [Fact]
    public void Rri_Row3_Cluster0_IncludesRowGroup()
    {
        // Row 3, cluster 0: RRI = 30*(3/3) + C_info = 30 + 2 = 32
        Assert.Equal(32, PDF417Encoder.ComputeRri(3, R, C, L));
    }

    [Fact]
    public void RowIndicators_ValuesInValidRange()
    {
        // For the full 10-row symbol, all indicator values must be 0..928.
        for (int r = 0; r < R; r++)
        {
            int lri = PDF417Encoder.ComputeLri(r, R, C, L);
            int rri = PDF417Encoder.ComputeRri(r, R, C, L);
            Assert.InRange(lri, 0, 928);
            Assert.InRange(rri, 0, 928);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Dimension selection
// ─────────────────────────────────────────────────────────────────────────────

public class DimensionSelectionTests
{
    [Fact]
    public void ChooseDimensions_SmallTotal_ProducesValidGrid()
    {
        var (cols, rows) = PDF417Encoder.ChooseDimensions(19);
        Assert.InRange(cols, 1, 30);
        Assert.InRange(rows, 3, 90);
        Assert.True(cols * rows >= 19);
    }

    [Fact]
    public void ChooseDimensions_1Codeword_MinimumDimensions()
    {
        // Even 1 codeword must produce a valid 3-row, 1-column symbol.
        var (cols, rows) = PDF417Encoder.ChooseDimensions(1);
        Assert.InRange(cols, 1, 30);
        Assert.InRange(rows, 3, 90);
        Assert.True(cols * rows >= 1);
    }

    [Fact]
    public void ChooseDimensions_LargeTotal_SatisfiesCapacity()
    {
        var (cols, rows) = PDF417Encoder.ChooseDimensions(200);
        Assert.True(cols * rows >= 200);
    }

    [Fact]
    public void ChooseDimensions_MaxCapacity_StaysWithinLimits()
    {
        // Maximum grid = 90 rows × 30 cols = 2700 slots.
        var (cols, rows) = PDF417Encoder.ChooseDimensions(2700);
        Assert.InRange(cols, 1, 30);
        Assert.InRange(rows, 3, 90);
        Assert.True(cols * rows >= 2700);
    }

    [Fact]
    public void AutoEccLevel_SmallData_Level2()
    {
        Assert.Equal(2, PDF417Encoder.AutoEccLevel(10));
        Assert.Equal(2, PDF417Encoder.AutoEccLevel(40));
    }

    [Fact]
    public void AutoEccLevel_MediumData_Level3()
    {
        Assert.Equal(3, PDF417Encoder.AutoEccLevel(41));
        Assert.Equal(3, PDF417Encoder.AutoEccLevel(160));
    }

    [Fact]
    public void AutoEccLevel_LargerData_Level4()
    {
        Assert.Equal(4, PDF417Encoder.AutoEccLevel(161));
        Assert.Equal(4, PDF417Encoder.AutoEccLevel(320));
    }

    [Fact]
    public void AutoEccLevel_LargeData_Level6()
    {
        Assert.Equal(6, PDF417Encoder.AutoEccLevel(864));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Integration — full encode pipeline
// ─────────────────────────────────────────────────────────────────────────────

public class IntegrationTests
{
    // ── Module grid dimensions ───────────────────────────────────────────────

    [Fact]
    public void Encode_MinimalInput_GridDimensionsCorrect()
    {
        // Encode 1 byte "A".
        // Byte compaction: 1 latch + 1 byte = 2 data cwords.
        // Length descriptor: 1 + 2 + 8 (ECC level 2) = 11.
        // Auto dimensions for total=11: c=ceil(sqrt(11/3))≈2, r=ceil(11/2)=6 → r=max(3,6)=6.
        // Actual result may vary by implementation — test the invariants instead.
        var grid = PDF417Encoder.Encode("A"u8.ToArray());
        int moduleWidth = grid.Cols;
        int moduleHeight = grid.Rows;
        Assert.True(moduleWidth >= 69 + 17); // at least 1 data column
        Assert.True(moduleHeight >= 3);       // at least 3 rows × 1 row-height
        // Width must be 69 + 17*cols (formula)
        Assert.Equal(0, (moduleWidth - 69) % 17);
    }

    [Fact]
    public void Encode_HelloWorld_WidthFormulaCorrect()
    {
        var grid = PDF417Encoder.Encode("HELLO WORLD"u8.ToArray());
        // Width = 69 + 17 * cols; must be congruent to 69 mod 17 = 69 - 4*17 = 69-68=1.
        Assert.Equal(0, (grid.Cols - 69) % 17);
    }

    [Fact]
    public void Encode_StringOverload_MatchesBytesOverload()
    {
        byte[] bytes = System.Text.Encoding.UTF8.GetBytes("TEST");
        var g1 = PDF417Encoder.Encode(bytes);
        var g2 = PDF417Encoder.Encode("TEST");
        Assert.Equal(g1.Rows, g2.Rows);
        Assert.Equal(g1.Cols, g2.Cols);
        // Compare module contents.
        for (int r = 0; r < g1.Rows; r++)
            for (int c = 0; c < g1.Cols; c++)
                Assert.Equal(g1.Modules[r][c], g2.Modules[r][c]);
    }

    // ── Start pattern in every row ───────────────────────────────────────────
    //
    // Start pattern: 11111111010101000 (17 modules)
    // As dark/light: T T T T T T T T F T F T F T F F F
    // (true = dark, false = light)

    private static bool[] StartPattern { get; } =
        [true, true, true, true, true, true, true, true,
         false, true, false, true, false, true, false, false, false];

    [Fact]
    public void Encode_MinimalInput_StartPatternPresentEveryRow()
    {
        var grid = PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { RowHeight = 1 });
        int rows = grid.Rows;
        int cols = grid.Cols;
        for (int r = 0; r < rows; r++)
        {
            for (int m = 0; m < 17; m++)
                Assert.Equal(StartPattern[m], grid.Modules[r][m]);
        }
    }

    // ── Stop pattern in every row ─────────────────────────────────────────────
    //
    // Stop pattern: 111111101000101001 (18 modules)
    // As dark/light: T T T T T T T F T F F F T F T F F T

    private static bool[] StopPattern { get; } =
        [true, true, true, true, true, true, true, false,
         true, false, false, false, true, false, true, false, false, true];

    [Fact]
    public void Encode_MinimalInput_StopPatternPresentEveryRow()
    {
        var grid = PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { RowHeight = 1 });
        int rows = grid.Rows;
        int cols = grid.Cols;
        for (int r = 0; r < rows; r++)
        {
            for (int m = 0; m < 18; m++)
                Assert.Equal(StopPattern[m], grid.Modules[r][cols - 18 + m]);
        }
    }

    // ── Row height ───────────────────────────────────────────────────────────

    [Fact]
    public void Encode_RowHeight3_GridHeightIsMultipleOf3()
    {
        var grid = PDF417Encoder.Encode("TEST"u8.ToArray(), new PDF417Options { RowHeight = 3 });
        Assert.Equal(0, grid.Rows % 3);
    }

    [Fact]
    public void Encode_RowHeight5_GridHeightIsMultipleOf5()
    {
        var grid = PDF417Encoder.Encode("TEST"u8.ToArray(), new PDF417Options { RowHeight = 5 });
        Assert.Equal(0, grid.Rows % 5);
    }

    [Fact]
    public void Encode_RowHeight3_RowsAreIdentical()
    {
        // With rowHeight=3, each logical row is repeated 3 times vertically.
        var grid = PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { RowHeight = 3 });
        // Every group of 3 physical rows should be identical.
        for (int logicalRow = 0; logicalRow < grid.Rows / 3; logicalRow++)
        {
            int r0 = logicalRow * 3;
            for (int h = 1; h < 3; h++)
            {
                for (int c = 0; c < grid.Cols; c++)
                    Assert.Equal(grid.Modules[r0][c], grid.Modules[r0 + h][c]);
            }
        }
    }

    // ── Determinism ──────────────────────────────────────────────────────────

    [Fact]
    public void Encode_SameInput_SameOutput()
    {
        byte[] data = System.Text.Encoding.UTF8.GetBytes("HELLO WORLD");
        var g1 = PDF417Encoder.Encode(data);
        var g2 = PDF417Encoder.Encode(data);
        Assert.Equal(g1.Rows, g2.Rows);
        Assert.Equal(g1.Cols, g2.Cols);
        for (int r = 0; r < g1.Rows; r++)
            for (int c = 0; c < g1.Cols; c++)
                Assert.Equal(g1.Modules[r][c], g2.Modules[r][c]);
    }

    // ── Custom columns ────────────────────────────────────────────────────────

    [Fact]
    public void Encode_CustomColumns_WidthMatchesFormula()
    {
        var grid = PDF417Encoder.Encode("TEST"u8.ToArray(),
            new PDF417Options { Columns = 5, RowHeight = 1 });
        Assert.Equal(69 + 17 * 5, grid.Cols);
    }

    // ── Custom ECC level ──────────────────────────────────────────────────────

    [Fact]
    public void Encode_EccLevel0_ProducesSmallSymbol()
    {
        // ECC level 0 = 2 ECC codewords — smallest possible.
        var gridL0 = PDF417Encoder.Encode("A"u8.ToArray(),
            new PDF417Options { EccLevel = 0, RowHeight = 1 });
        var gridL4 = PDF417Encoder.Encode("A"u8.ToArray(),
            new PDF417Options { EccLevel = 4, RowHeight = 1 });
        // Higher ECC level → more total codewords → potentially more rows.
        Assert.True(gridL0.Rows * gridL0.Cols <= gridL4.Rows * gridL4.Cols);
    }

    // ── Binary data ──────────────────────────────────────────────────────────

    [Fact]
    public void Encode_AllByteValues_Succeeds()
    {
        // Encode all 256 byte values [0..255] — must not throw.
        byte[] allBytes = new byte[256];
        for (int i = 0; i < 256; i++) allBytes[i] = (byte)i;
        var grid = PDF417Encoder.Encode(allBytes);
        Assert.True(grid.Rows >= 3);
        Assert.True(grid.Cols >= 86); // at least 1 column → 69 + 17 = 86
    }

    // ── Empty input ────────────────────────────────────────────────────────────

    [Fact]
    public void Encode_EmptyInput_Succeeds()
    {
        // Empty input → only the latch codeword in byte mode → still valid.
        var grid = PDF417Encoder.Encode(Array.Empty<byte>());
        Assert.True(grid.Rows >= 3);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Error handling
// ─────────────────────────────────────────────────────────────────────────────

public class ErrorHandlingTests
{
    [Fact]
    public void Encode_InvalidEccLevel_Negative_Throws()
    {
        Assert.Throws<InvalidECCLevelException>(() =>
            PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { EccLevel = -1 }));
    }

    [Fact]
    public void Encode_InvalidEccLevel_TooHigh_Throws()
    {
        Assert.Throws<InvalidECCLevelException>(() =>
            PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { EccLevel = 9 }));
    }

    [Fact]
    public void Encode_InvalidColumns_Zero_Throws()
    {
        Assert.Throws<InvalidDimensionsException>(() =>
            PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { Columns = 0 }));
    }

    [Fact]
    public void Encode_InvalidColumns_TooMany_Throws()
    {
        Assert.Throws<InvalidDimensionsException>(() =>
            PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { Columns = 31 }));
    }

    [Fact]
    public void Encode_ValidEccLevel0_Succeeds()
    {
        // ECC level 0 is valid.
        var grid = PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { EccLevel = 0 });
        Assert.True(grid.Rows >= 3);
    }

    [Fact]
    public void Encode_ValidEccLevel8_Succeeds()
    {
        // ECC level 8 = 512 ECC codewords — still valid with enough data.
        var data = new byte[200];
        var grid = PDF417Encoder.Encode(data, new PDF417Options { EccLevel = 8 });
        Assert.True(grid.Rows >= 3);
    }

    [Fact]
    public void Encode_ValidColumns_1_Succeeds()
    {
        var grid = PDF417Encoder.Encode("A"u8.ToArray(), new PDF417Options { Columns = 1 });
        Assert.Equal(86, grid.Cols); // 69 + 17*1
    }

    [Fact]
    public void Encode_ValidColumns_30_Succeeds()
    {
        // 30 columns is the maximum; encode enough data to need it.
        var data = new byte[50];
        var grid = PDF417Encoder.Encode(data, new PDF417Options { Columns = 30, EccLevel = 2 });
        Assert.Equal(69 + 17 * 30, grid.Cols);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Cluster table sanity checks
// ─────────────────────────────────────────────────────────────────────────────

public class ClusterTableTests
{
    [Fact]
    public void ClusterTables_Has3Clusters()
    {
        Assert.Equal(3, ClusterTables.Tables.Length);
    }

    [Fact]
    public void ClusterTables_Each929Entries()
    {
        foreach (var table in ClusterTables.Tables)
            Assert.Equal(929, table.Length);
    }

    [Fact]
    public void ClusterTables_AllPatternsSumTo17Modules()
    {
        // Each packed pattern must decode to 8 widths that sum to exactly 17.
        foreach (var table in ClusterTables.Tables)
        {
            foreach (uint packed in table)
            {
                int b1 = (int)((packed >> 28) & 0xF);
                int s1 = (int)((packed >> 24) & 0xF);
                int b2 = (int)((packed >> 20) & 0xF);
                int s2 = (int)((packed >> 16) & 0xF);
                int b3 = (int)((packed >> 12) & 0xF);
                int s3 = (int)((packed >>  8) & 0xF);
                int b4 = (int)((packed >>  4) & 0xF);
                int s4 = (int) (packed        & 0xF);
                int total = b1 + s1 + b2 + s2 + b3 + s3 + b4 + s4;
                Assert.Equal(17, total);
            }
        }
    }

    [Fact]
    public void ClusterTables_AllWidthsPositive()
    {
        // No element width may be 0.
        foreach (var table in ClusterTables.Tables)
        {
            foreach (uint packed in table)
            {
                int b1 = (int)((packed >> 28) & 0xF);
                int s1 = (int)((packed >> 24) & 0xF);
                int b2 = (int)((packed >> 20) & 0xF);
                int s2 = (int)((packed >> 16) & 0xF);
                int b3 = (int)((packed >> 12) & 0xF);
                int s3 = (int)((packed >>  8) & 0xF);
                int b4 = (int)((packed >>  4) & 0xF);
                int s4 = (int) (packed        & 0xF);
                Assert.True(b1 >= 1 && s1 >= 1 && b2 >= 1 && s2 >= 1);
                Assert.True(b3 >= 1 && s3 >= 1 && b4 >= 1 && s4 >= 1);
            }
        }
    }
}
