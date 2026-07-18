using Parser = CodingAdventures.FontParser.FontParser;

namespace CodingAdventures.FontParser.Tests;

public sealed class FontParserTests
{
    private static byte[] InterBytes() =>
        File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", "Inter-Regular.ttf"));

    [Fact]
    public void LoadRejectsNullAndShortBuffers()
    {
        Assert.Throws<ArgumentNullException>(() => Parser.Load(null!));
        var error = Assert.Throws<FontParseException>(() => Parser.Load([]));
        Assert.Equal(FontErrorKind.BufferTooShort, error.Kind);
    }

    [Fact]
    public void LoadRejectsInvalidMagicAndMissingTables()
    {
        var invalid = new byte[12];
        invalid[0] = 0xDE;
        var magicError = Assert.Throws<FontParseException>(() => Parser.Load(invalid));
        Assert.Equal(FontErrorKind.InvalidMagic, magicError.Kind);

        var emptyDirectory = new byte[12];
        WriteU32(emptyDirectory, 0, 0x00010000);
        var tableError = Assert.Throws<FontParseException>(() => Parser.Load(emptyDirectory));
        Assert.Equal(FontErrorKind.TableNotFound, tableError.Kind);
    }

    [Fact]
    public void LoadRejectsTruncatedDirectoriesAndBadHeadMagic()
    {
        var truncated = new byte[12];
        WriteU32(truncated, 0, 0x00010000);
        WriteU16(truncated, 4, 1);
        Assert.Equal(
            FontErrorKind.BufferTooShort,
            Assert.Throws<FontParseException>(() => Parser.Load(truncated)).Kind);

        var badHead = BuildSyntheticFont();
        var head = FindTable(badHead, "head");
        WriteU32(badHead, head + 12, 0);
        Assert.Equal(
            FontErrorKind.InvalidHeadMagic,
            Assert.Throws<FontParseException>(() => Parser.Load(badHead)).Kind);
    }

    [Fact]
    public void LoadOwnsAnImmutableCopy()
    {
        var bytes = BuildSyntheticFont();
        var font = Parser.Load(bytes);
        bytes[0] = 0xFF;
        Assert.Equal((ushort)1000, Parser.GetFontMetrics(font).UnitsPerEm);
    }

    [Fact]
    public void InterExposesGlobalMetricsAndNames()
    {
        var metrics = Parser.GetFontMetrics(Parser.Load(InterBytes()));
        Assert.Equal((ushort)2048, metrics.UnitsPerEm);
        Assert.Equal("Inter", metrics.FamilyName);
        Assert.Equal("Regular", metrics.SubfamilyName);
        Assert.True(metrics.Ascender > 0);
        Assert.True(metrics.Descender <= 0);
        Assert.True(metrics.NumGlyphs > 100);
        Assert.True(metrics.XHeight > 0);
        Assert.True(metrics.CapHeight > 0);
    }

    [Fact]
    public void SyntheticFontUsesHheaFallbackAndUnknownNames()
    {
        var metrics = Parser.GetFontMetrics(Parser.Load(BuildSyntheticFont()));
        Assert.Equal((ushort)1000, metrics.UnitsPerEm);
        Assert.Equal((short)800, metrics.Ascender);
        Assert.Equal((short)-200, metrics.Descender);
        Assert.Equal((short)10, metrics.LineGap);
        Assert.Null(metrics.XHeight);
        Assert.Null(metrics.CapHeight);
        Assert.Equal("(unknown)", metrics.FamilyName);
        Assert.Equal("(unknown)", metrics.SubfamilyName);
    }

    [Fact]
    public void GlyphLookupHandlesMappedUnmappedAndOutOfBmpValues()
    {
        var font = Parser.Load(InterBytes());
        var a = Parser.GetGlyphId(font, 'A');
        var v = Parser.GetGlyphId(font, 'V');
        Assert.NotNull(a);
        Assert.NotNull(v);
        Assert.NotEqual(a, v);
        Assert.NotNull(Parser.GetGlyphId(font, ' '));
        Assert.Null(Parser.GetGlyphId(font, -1));
        Assert.Null(Parser.GetGlyphId(font, 0x10000));
        Assert.Null(Parser.GetGlyphId(Parser.Load(BuildSyntheticFont()), 0xFFFF));
    }

    [Fact]
    public void GlyphLookupIgnoresUnsupportedCmapFormats()
    {
        var bytes = BuildSyntheticFont();
        var cmap = FindTable(bytes, "cmap");
        WriteU16(bytes, cmap + 12, 12);
        Assert.Null(Parser.GetGlyphId(Parser.Load(bytes), 'A'));
    }

    [Fact]
    public void GlyphMetricsSupportFullAndSharedAdvanceRecords()
    {
        var font = Parser.Load(BuildSyntheticFont());
        Assert.Equal(new GlyphMetrics(600, 10), Parser.GetGlyphMetrics(font, 0));
        Assert.Equal(new GlyphMetrics(700, 20), Parser.GetGlyphMetrics(font, 1));
        Assert.Equal(new GlyphMetrics(700, 40), Parser.GetGlyphMetrics(font, 3));
        Assert.Null(Parser.GetGlyphMetrics(font, -1));
        Assert.Null(Parser.GetGlyphMetrics(font, 5));

        var inter = Parser.Load(InterBytes());
        var glyph = Parser.GetGlyphId(inter, 'A');
        Assert.True(Parser.GetGlyphMetrics(inter, glyph!.Value)!.AdvanceWidth > 0);
    }

    [Fact]
    public void KerningReadsSortedFormatZeroPairs()
    {
        var font = Parser.Load(BuildSyntheticFont([(1, 2, -140), (3, 4, 80)]));
        Assert.Equal((short)-140, Parser.GetKerning(font, 1, 2));
        Assert.Equal((short)80, Parser.GetKerning(font, 3, 4));
        Assert.Equal((short)0, Parser.GetKerning(font, 1, 4));
        Assert.Equal((short)0, Parser.GetKerning(font, 2, 1));
        Assert.Equal((short)0, Parser.GetKerning(font, -1, 2));
    }

    [Fact]
    public void KerningDefaultsToZeroWhenTableIsAbsent()
    {
        var font = Parser.Load(InterBytes());
        var a = Parser.GetGlyphId(font, 'A')!.Value;
        var v = Parser.GetGlyphId(font, 'V')!.Value;
        Assert.Equal((short)0, Parser.GetKerning(font, a, v));
    }

    private static byte[] BuildSyntheticFont((ushort Left, ushort Right, short Value)[]? pairs = null)
    {
        pairs ??= [];
        pairs = pairs.OrderBy(pair => ((uint)pair.Left << 16) | pair.Right).ToArray();
        const int tableCount = 6;
        const int headLength = 54;
        const int hheaLength = 36;
        const int maxpLength = 6;
        const int cmapLength = 36;
        const int hmtxLength = 14;
        var kernLength = 18 + pairs.Length * 6;
        var directoryLength = 12 + tableCount * 16;
        var head = directoryLength;
        var hhea = head + headLength;
        var maxp = hhea + hheaLength;
        var cmap = maxp + maxpLength;
        var hmtx = cmap + cmapLength;
        var kern = hmtx + hmtxLength;
        var bytes = new byte[kern + kernLength];

        WriteU32(bytes, 0, 0x00010000);
        WriteU16(bytes, 4, tableCount);
        WriteTable(bytes, 0, "cmap", cmap, cmapLength);
        WriteTable(bytes, 1, "head", head, headLength);
        WriteTable(bytes, 2, "hhea", hhea, hheaLength);
        WriteTable(bytes, 3, "hmtx", hmtx, hmtxLength);
        WriteTable(bytes, 4, "kern", kern, kernLength);
        WriteTable(bytes, 5, "maxp", maxp, maxpLength);

        WriteU32(bytes, head, 0x00010000);
        WriteU32(bytes, head + 12, 0x5F0F3CF5);
        WriteU16(bytes, head + 18, 1000);

        WriteU32(bytes, hhea, 0x00010000);
        WriteI16(bytes, hhea + 4, 800);
        WriteI16(bytes, hhea + 6, -200);
        WriteI16(bytes, hhea + 8, 10);
        WriteU16(bytes, hhea + 34, 2);

        WriteU32(bytes, maxp, 0x00005000);
        WriteU16(bytes, maxp + 4, 5);

        WriteU16(bytes, cmap + 2, 1);
        WriteU16(bytes, cmap + 4, 3);
        WriteU16(bytes, cmap + 6, 1);
        WriteU32(bytes, cmap + 8, 12);
        WriteU16(bytes, cmap + 12, 4);
        WriteU16(bytes, cmap + 14, 24);
        WriteU16(bytes, cmap + 18, 2);
        WriteU16(bytes, cmap + 20, 2);
        WriteU16(bytes, cmap + 26, 0xFFFF);
        WriteU16(bytes, cmap + 28, 0);
        WriteU16(bytes, cmap + 30, 0xFFFF);
        WriteI16(bytes, cmap + 32, 1);
        WriteU16(bytes, cmap + 34, 0);

        WriteU16(bytes, hmtx, 600);
        WriteI16(bytes, hmtx + 2, 10);
        WriteU16(bytes, hmtx + 4, 700);
        WriteI16(bytes, hmtx + 6, 20);
        WriteI16(bytes, hmtx + 8, 30);
        WriteI16(bytes, hmtx + 10, 40);
        WriteI16(bytes, hmtx + 12, 50);

        WriteU16(bytes, kern, 0);
        WriteU16(bytes, kern + 2, 1);
        WriteU16(bytes, kern + 4, 0);
        WriteU16(bytes, kern + 6, kernLength - 4);
        WriteU16(bytes, kern + 8, 1);
        WriteU16(bytes, kern + 10, pairs.Length);
        for (var index = 0; index < pairs.Length; index++)
        {
            var offset = kern + 18 + index * 6;
            WriteU16(bytes, offset, pairs[index].Left);
            WriteU16(bytes, offset + 2, pairs[index].Right);
            WriteI16(bytes, offset + 4, pairs[index].Value);
        }

        return bytes;
    }

    private static int FindTable(byte[] bytes, string tag)
    {
        var count = ReadU16(bytes, 4);
        for (var index = 0; index < count; index++)
        {
            var record = 12 + index * 16;
            if (System.Text.Encoding.ASCII.GetString(bytes, record, 4) == tag)
            {
                return (int)ReadU32(bytes, record + 8);
            }
        }

        throw new InvalidOperationException($"Missing table {tag}");
    }

    private static void WriteTable(byte[] bytes, int index, string tag, int offset, int length)
    {
        var record = 12 + index * 16;
        System.Text.Encoding.ASCII.GetBytes(tag).CopyTo(bytes, record);
        WriteU32(bytes, record + 8, (uint)offset);
        WriteU32(bytes, record + 12, (uint)length);
    }

    private static ushort ReadU16(byte[] bytes, int offset) =>
        (ushort)((bytes[offset] << 8) | bytes[offset + 1]);

    private static uint ReadU32(byte[] bytes, int offset) =>
        ((uint)bytes[offset] << 24) |
        ((uint)bytes[offset + 1] << 16) |
        ((uint)bytes[offset + 2] << 8) |
        bytes[offset + 3];

    private static void WriteU16(byte[] bytes, int offset, int value)
    {
        bytes[offset] = (byte)(value >> 8);
        bytes[offset + 1] = (byte)value;
    }

    private static void WriteI16(byte[] bytes, int offset, short value) =>
        WriteU16(bytes, offset, unchecked((ushort)value));

    private static void WriteU32(byte[] bytes, int offset, uint value)
    {
        bytes[offset] = (byte)(value >> 24);
        bytes[offset + 1] = (byte)(value >> 16);
        bytes[offset + 2] = (byte)(value >> 8);
        bytes[offset + 3] = (byte)value;
    }
}
