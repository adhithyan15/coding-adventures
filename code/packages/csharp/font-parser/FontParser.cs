using System.Text;

namespace CodingAdventures.FontParser;

/// <summary>Identifies why a font could not be parsed.</summary>
public enum FontErrorKind
{
    /// <summary>A read would extend past the supplied bytes.</summary>
    BufferTooShort,
    /// <summary>The sfnt version is neither TrueType nor OpenType.</summary>
    InvalidMagic,
    /// <summary>The required head table sentinel is invalid.</summary>
    InvalidHeadMagic,
    /// <summary>A required metrics table is missing.</summary>
    TableNotFound,
}

/// <summary>Raised when an OpenType or TrueType binary is malformed.</summary>
public sealed class FontParseException(FontErrorKind kind, string message) : Exception(message)
{
    /// <summary>The stable category for this parse failure.</summary>
    public FontErrorKind Kind { get; } = kind;
}

/// <summary>Global typographic metrics in font design units.</summary>
public sealed record FontMetrics(
    ushort UnitsPerEm,
    short Ascender,
    short Descender,
    short LineGap,
    short? XHeight,
    short? CapHeight,
    ushort NumGlyphs,
    string FamilyName,
    string SubfamilyName);

/// <summary>Horizontal metrics for a single glyph.</summary>
public sealed record GlyphMetrics(ushort AdvanceWidth, short LeftSideBearing);

internal readonly record struct TableRecord(int Offset, int Length);

/// <summary>An immutable handle to a parsed font binary.</summary>
public sealed class FontFile
{
    internal FontFile(byte[] data, IReadOnlyDictionary<string, TableRecord> tables)
    {
        Data = data;
        Tables = tables;
    }

    internal byte[] Data { get; }
    internal IReadOnlyDictionary<string, TableRecord> Tables { get; }
}

/// <summary>Metrics-only OpenType and TrueType font parser.</summary>
public static class FontParser
{
    private const uint TrueTypeMagic = 0x0001_0000;
    private const uint OpenTypeMagic = 0x4F54_544F;
    private const uint HeadMagic = 0x5F0F_3CF5;

    /// <summary>Parse a complete font binary and retain an immutable copy.</summary>
    public static FontFile Load(byte[] bytes)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        if (bytes.Length < 12)
        {
            Throw(FontErrorKind.BufferTooShort, "font buffer is too small to contain an sfnt header");
        }

        var data = (byte[])bytes.Clone();
        var magic = ReadU32(data, 0);
        if (magic is not TrueTypeMagic and not OpenTypeMagic)
        {
            Throw(FontErrorKind.InvalidMagic, $"invalid sfnt version 0x{magic:X8}");
        }

        var numTables = ReadU16(data, 4);
        var tables = new Dictionary<string, TableRecord>(StringComparer.Ordinal);
        for (var i = 0; i < numTables; i++)
        {
            var recordOffset = checked(12 + (i * 16));
            EnsureRange(data, recordOffset, 16);
            var tag = Encoding.ASCII.GetString(data, recordOffset, 4);
            var offset = CheckedInt(ReadU32(data, recordOffset + 8), "table offset");
            var length = CheckedInt(ReadU32(data, recordOffset + 12), "table length");
            EnsureRange(data, offset, length);
            tables[tag] = new TableRecord(offset, length);
        }

        foreach (var tag in new[] { "head", "hhea", "maxp", "cmap", "hmtx" })
        {
            RequireTable(tables, tag);
        }

        var head = RequireTable(tables, "head");
        if (ReadU32(data, head.Offset + 12) != HeadMagic)
        {
            Throw(FontErrorKind.InvalidHeadMagic, "invalid head.magicNumber");
        }

        return new FontFile(data, tables);
    }

    /// <summary>Return global font metrics, preferring OS/2 typographic values.</summary>
    public static FontMetrics GetFontMetrics(FontFile font)
    {
        ArgumentNullException.ThrowIfNull(font);
        var data = font.Data;
        var head = RequireTable(font.Tables, "head");
        var hhea = RequireTable(font.Tables, "hhea");
        var maxp = RequireTable(font.Tables, "maxp");

        var ascender = ReadI16(data, hhea.Offset + 4);
        var descender = ReadI16(data, hhea.Offset + 6);
        var lineGap = ReadI16(data, hhea.Offset + 8);
        short? xHeight = null;
        short? capHeight = null;

        if (font.Tables.TryGetValue("OS/2", out var os2))
        {
            var version = ReadU16(data, os2.Offset);
            if (os2.Length >= 74)
            {
                ascender = ReadI16(data, os2.Offset + 68);
                descender = ReadI16(data, os2.Offset + 70);
                lineGap = ReadI16(data, os2.Offset + 72);
            }

            if (version >= 2 && os2.Length >= 90)
            {
                xHeight = ReadI16(data, os2.Offset + 86);
                capHeight = ReadI16(data, os2.Offset + 88);
            }
        }

        return new FontMetrics(
            ReadU16(data, head.Offset + 18),
            ascender,
            descender,
            lineGap,
            xHeight,
            capHeight,
            ReadU16(data, maxp.Offset + 4),
            ReadName(font, 1) ?? "(unknown)",
            ReadName(font, 2) ?? "(unknown)");
    }

    /// <summary>Map a BMP Unicode codepoint to a glyph identifier.</summary>
    public static ushort? GetGlyphId(FontFile font, int codepoint)
    {
        ArgumentNullException.ThrowIfNull(font);
        if (codepoint is < 0 or > 0xFFFF)
        {
            return null;
        }

        var data = font.Data;
        var cmap = RequireTable(font.Tables, "cmap");
        var numSubtables = ReadU16(data, cmap.Offset + 2);
        int? selected = null;

        for (var i = 0; i < numSubtables; i++)
        {
            var record = cmap.Offset + 4 + (i * 8);
            var platform = ReadU16(data, record);
            var encoding = ReadU16(data, record + 2);
            var relative = CheckedInt(ReadU32(data, record + 4), "cmap subtable offset");
            var absolute = checked(cmap.Offset + relative);
            if (platform == 3 && encoding == 1)
            {
                selected = absolute;
                break;
            }

            if (platform == 0 && selected is null)
            {
                selected = absolute;
            }
        }

        if (selected is null || ReadU16(data, selected.Value) != 4)
        {
            return null;
        }

        var subtable = selected.Value;
        var segmentCount = ReadU16(data, subtable + 6) / 2;
        var endCodes = subtable + 14;
        var startCodes = subtable + 16 + (segmentCount * 2);
        var deltas = subtable + 16 + (segmentCount * 4);
        var rangeOffsets = subtable + 16 + (segmentCount * 6);

        var lo = 0;
        var hi = segmentCount;
        while (lo < hi)
        {
            var mid = lo + ((hi - lo) / 2);
            if (ReadU16(data, endCodes + (mid * 2)) < codepoint)
            {
                lo = mid + 1;
            }
            else
            {
                hi = mid;
            }
        }

        if (lo >= segmentCount)
        {
            return null;
        }

        var start = ReadU16(data, startCodes + (lo * 2));
        var end = ReadU16(data, endCodes + (lo * 2));
        if (codepoint < start || codepoint > end)
        {
            return null;
        }

        var delta = ReadI16(data, deltas + (lo * 2));
        var rangeOffsetAddress = rangeOffsets + (lo * 2);
        var rangeOffset = ReadU16(data, rangeOffsetAddress);
        ushort glyph;
        if (rangeOffset == 0)
        {
            glyph = unchecked((ushort)(codepoint + delta));
        }
        else
        {
            var glyphAddress = checked(rangeOffsetAddress + rangeOffset + ((codepoint - start) * 2));
            glyph = ReadU16(data, glyphAddress);
            if (glyph != 0)
            {
                glyph = unchecked((ushort)(glyph + delta));
            }
        }

        return glyph == 0 ? null : glyph;
    }

    /// <summary>Return horizontal metrics for a glyph, or null when out of range.</summary>
    public static GlyphMetrics? GetGlyphMetrics(FontFile font, int glyphId)
    {
        ArgumentNullException.ThrowIfNull(font);
        var data = font.Data;
        var maxp = RequireTable(font.Tables, "maxp");
        var hhea = RequireTable(font.Tables, "hhea");
        var hmtx = RequireTable(font.Tables, "hmtx");
        var numGlyphs = ReadU16(data, maxp.Offset + 4);
        var numHorizontalMetrics = ReadU16(data, hhea.Offset + 34);

        if (glyphId < 0 || glyphId >= numGlyphs || numHorizontalMetrics == 0)
        {
            return null;
        }

        if (glyphId < numHorizontalMetrics)
        {
            var offset = hmtx.Offset + (glyphId * 4);
            return new GlyphMetrics(ReadU16(data, offset), ReadI16(data, offset + 2));
        }

        var lastMetric = hmtx.Offset + ((numHorizontalMetrics - 1) * 4);
        var bearing = hmtx.Offset + (numHorizontalMetrics * 4) + ((glyphId - numHorizontalMetrics) * 2);
        return new GlyphMetrics(ReadU16(data, lastMetric), ReadI16(data, bearing));
    }

    /// <summary>Return a legacy kern format 0 adjustment, or zero when absent.</summary>
    public static short GetKerning(FontFile font, int leftGlyphId, int rightGlyphId)
    {
        ArgumentNullException.ThrowIfNull(font);
        if (leftGlyphId is < 0 or > 0xFFFF || rightGlyphId is < 0 or > 0xFFFF ||
            !font.Tables.TryGetValue("kern", out var kern))
        {
            return 0;
        }

        var data = font.Data;
        var tableEnd = checked(kern.Offset + kern.Length);
        var subtableCount = ReadU16(data, kern.Offset + 2);
        var position = kern.Offset + 4;
        var target = ((uint)leftGlyphId << 16) | (uint)rightGlyphId;

        for (var table = 0; table < subtableCount && position + 6 <= tableEnd; table++)
        {
            var length = ReadU16(data, position + 2);
            if (length < 6 || position + length > tableEnd)
            {
                break;
            }

            var format = ReadU16(data, position + 4) >> 8;
            if (format == 0 && length >= 14)
            {
                var pairCount = ReadU16(data, position + 6);
                var pairs = position + 14;
                var lo = 0;
                var hi = (int)pairCount;
                while (lo < hi)
                {
                    var mid = lo + ((hi - lo) / 2);
                    var pair = pairs + (mid * 6);
                    if (pair + 6 > position + length)
                    {
                        break;
                    }

                    var key = ((uint)ReadU16(data, pair) << 16) | ReadU16(data, pair + 2);
                    if (key == target)
                    {
                        return ReadI16(data, pair + 4);
                    }

                    if (key < target)
                    {
                        lo = mid + 1;
                    }
                    else
                    {
                        hi = mid;
                    }
                }
            }

            position += length;
        }

        return 0;
    }

    private static string? ReadName(FontFile font, ushort nameId)
    {
        if (!font.Tables.TryGetValue("name", out var name))
        {
            return null;
        }

        var data = font.Data;
        var count = ReadU16(data, name.Offset + 2);
        var stringBase = name.Offset + ReadU16(data, name.Offset + 4);
        (int Start, int Length)? fallback = null;

        for (var i = 0; i < count; i++)
        {
            var record = name.Offset + 6 + (i * 12);
            var platform = ReadU16(data, record);
            var encoding = ReadU16(data, record + 2);
            if (ReadU16(data, record + 6) != nameId)
            {
                continue;
            }

            var length = ReadU16(data, record + 8);
            var start = stringBase + ReadU16(data, record + 10);
            EnsureRange(data, start, length);
            if (platform == 3 && encoding is 1 or 10)
            {
                return Encoding.BigEndianUnicode.GetString(data, start, length);
            }

            if (platform == 0 && fallback is null)
            {
                fallback = (start, length);
            }
        }

        return fallback is { } value
            ? Encoding.BigEndianUnicode.GetString(data, value.Start, value.Length)
            : null;
    }

    private static TableRecord RequireTable(IReadOnlyDictionary<string, TableRecord> tables, string tag)
    {
        if (!tables.TryGetValue(tag, out var table))
        {
            Throw(FontErrorKind.TableNotFound, $"required table '{tag}' was not found");
        }

        return table;
    }

    private static ushort ReadU16(byte[] data, int offset)
    {
        EnsureRange(data, offset, 2);
        return (ushort)((data[offset] << 8) | data[offset + 1]);
    }

    private static short ReadI16(byte[] data, int offset) => unchecked((short)ReadU16(data, offset));

    private static uint ReadU32(byte[] data, int offset)
    {
        EnsureRange(data, offset, 4);
        return ((uint)data[offset] << 24) |
               ((uint)data[offset + 1] << 16) |
               ((uint)data[offset + 2] << 8) |
               data[offset + 3];
    }

    private static int CheckedInt(uint value, string label)
    {
        if (value > int.MaxValue)
        {
            Throw(FontErrorKind.BufferTooShort, $"{label} exceeds the supported buffer size");
        }

        return (int)value;
    }

    private static void EnsureRange(byte[] data, int offset, int length)
    {
        if (offset < 0 || length < 0 || offset > data.Length - length)
        {
            Throw(FontErrorKind.BufferTooShort, $"read at offset {offset} with length {length} exceeds the font buffer");
        }
    }

    private static void Throw(FontErrorKind kind, string message) => throw new FontParseException(kind, message);
}
