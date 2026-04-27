using CodingAdventures.PaintInstructions;
using CodingAdventures.PaintVm;

namespace CodingAdventures.PaintVmAscii;

public static class PaintVmAsciiPackage
{
    public const string VERSION = "0.1.0";
}

public sealed record AsciiOptions
{
    public double ScaleX { get; init; } = 8.0;

    public double ScaleY { get; init; } = 16.0;
}

public sealed class UnsupportedAsciiFeatureError(string message) : Exception(message)
{
}

internal readonly record struct ClipBounds(int MinCol, int MinRow, int MaxCol, int MaxRow);

[Flags]
internal enum CellFlags
{
    None = 0,
    Up = 1,
    Right = 2,
    Down = 4,
    Left = 8,
    Fill = 16,
    Text = 32,
}

/// <summary>
/// CharBuffer keeps both the visible cell contents and the directional tags
/// that let box-drawing intersections merge together cleanly.
/// </summary>
public sealed class CharBuffer
{
    private static readonly IReadOnlyDictionary<CellFlags, string> BoxCharacters = new Dictionary<CellFlags, string>
    {
        [CellFlags.Left | CellFlags.Right] = "─",
        [CellFlags.Up | CellFlags.Down] = "│",
        [CellFlags.Down | CellFlags.Right] = "┌",
        [CellFlags.Down | CellFlags.Left] = "┐",
        [CellFlags.Up | CellFlags.Right] = "└",
        [CellFlags.Up | CellFlags.Left] = "┘",
        [CellFlags.Left | CellFlags.Right | CellFlags.Down] = "┬",
        [CellFlags.Left | CellFlags.Right | CellFlags.Up] = "┴",
        [CellFlags.Up | CellFlags.Down | CellFlags.Right] = "├",
        [CellFlags.Up | CellFlags.Down | CellFlags.Left] = "┤",
        [CellFlags.Up | CellFlags.Down | CellFlags.Left | CellFlags.Right] = "┼",
        [CellFlags.Right] = "─",
        [CellFlags.Left] = "─",
        [CellFlags.Up] = "│",
        [CellFlags.Down] = "│",
    };

    private readonly string[][] _characters;
    private readonly CellFlags[][] _tags;

    public CharBuffer(int rows, int cols)
    {
        Rows = Math.Max(rows, 0);
        Cols = Math.Max(cols, 0);
        _characters = Enumerable.Range(0, Rows)
            .Select(_ => Enumerable.Repeat(" ", Cols).ToArray())
            .ToArray();
        _tags = Enumerable.Range(0, Rows)
            .Select(_ => Enumerable.Repeat(CellFlags.None, Cols).ToArray())
            .ToArray();
    }

    public int Rows { get; }

    public int Cols { get; }

    internal void WriteTag(int row, int col, CellFlags flags, ClipBounds clip)
    {
        if (!IsInsideClip(row, col, clip) || !IsInsideBuffer(row, col))
        {
            return;
        }

        var existing = _tags[row][col];
        if (existing.HasFlag(CellFlags.Text))
        {
            return;
        }

        var merged = existing | flags;
        _tags[row][col] = merged;
        _characters[row][col] = ResolveCell(merged);
    }

    internal void WriteChar(int row, int col, string value, ClipBounds clip)
    {
        if (!IsInsideClip(row, col, clip) || !IsInsideBuffer(row, col))
        {
            return;
        }

        _characters[row][col] = value;
        _tags[row][col] = CellFlags.Text;
    }

    public override string ToString()
    {
        var lines = _characters
            .Select(row => string.Concat(row).TrimEnd())
            .ToArray();

        var lastContent = Array.FindLastIndex(lines, line => line.Length > 0);
        if (lastContent < 0)
        {
            return string.Empty;
        }

        return string.Join('\n', lines.Take(lastContent + 1));
    }

    private static string ResolveCell(CellFlags flags)
    {
        var directions = flags & (CellFlags.Up | CellFlags.Right | CellFlags.Down | CellFlags.Left);
        if (directions != CellFlags.None && BoxCharacters.TryGetValue(directions, out var box))
        {
            return box;
        }

        if (flags.HasFlag(CellFlags.Fill))
        {
            return "█";
        }

        return "+";
    }

    private bool IsInsideBuffer(int row, int col) =>
        row >= 0 && row < Rows && col >= 0 && col < Cols;

    private static bool IsInsideClip(int row, int col, ClipBounds clip) =>
        row >= clip.MinRow &&
        row < clip.MaxRow &&
        col >= clip.MinCol &&
        col < clip.MaxCol;
}

public sealed class AsciiContext
{
    public AsciiContext()
    {
        Buffer = new CharBuffer(0, 0);
        ClipStack = new List<ClipBounds> { new(0, 0, 0, 0) };
    }

    public CharBuffer Buffer { get; internal set; }

    internal List<ClipBounds> ClipStack { get; }
}

/// <summary>
/// paint-vm-ascii is the terminal backend for the paint IR. It keeps the
/// generic PaintVM dispatch model while targeting a character grid.
/// </summary>
public static class PaintVmAscii
{
    public static AsciiContext CreateAsciiContext() => new();

    public static PaintVM<AsciiContext> CreateAsciiVm(AsciiOptions? options = null)
    {
        var scaleX = options?.ScaleX ?? 8.0;
        var scaleY = options?.ScaleY ?? 16.0;

        var vm = new PaintVM<AsciiContext>(
            (context, _, width, height) =>
            {
                var cols = Math.Max(0, (int)Math.Ceiling(width / scaleX));
                var rows = Math.Max(0, (int)Math.Ceiling(height / scaleY));
                context.Buffer = new CharBuffer(rows, cols);
                context.ClipStack.Clear();
                context.ClipStack.Add(FullClip(cols, rows));
            },
            (_, _, _) => throw new ExportNotSupportedError("paint-vm-ascii"));

        vm.Register("rect", (instruction, context, _) =>
        {
            if (instruction is PaintRect rect)
            {
                HandleRect(rect, context, scaleX, scaleY);
            }
        });

        vm.Register("line", (instruction, context, _) =>
        {
            if (instruction is PaintLine line)
            {
                HandleLine(line, context, scaleX, scaleY);
            }
        });

        vm.Register("glyph_run", (instruction, context, _) =>
        {
            if (instruction is PaintGlyphRun glyphRun)
            {
                HandleGlyphRun(glyphRun, context, scaleX, scaleY);
            }
        });

        vm.Register("group", (instruction, context, innerVm) =>
        {
            if (instruction is not PaintGroup group)
            {
                return;
            }

            AssertPlainGroup(group);
            foreach (var child in group.Children)
            {
                innerVm.Dispatch(child, context);
            }
        });

        vm.Register("clip", (instruction, context, innerVm) =>
        {
            if (instruction is PaintClip clip)
            {
                HandleClip(clip, context, innerVm, scaleX, scaleY);
            }
        });

        vm.Register("layer", (instruction, context, innerVm) =>
        {
            if (instruction is not PaintLayer layer)
            {
                return;
            }

            AssertPlainLayer(layer);
            foreach (var child in layer.Children)
            {
                innerVm.Dispatch(child, context);
            }
        });

        return vm;
    }

    public static string RenderToAscii(PaintScene scene, AsciiOptions? options = null)
    {
        var context = CreateAsciiContext();
        CreateAsciiVm(options).Execute(scene, context);
        return context.Buffer.ToString();
    }

    private static void HandleRect(PaintRect instruction, AsciiContext context, double scaleX, double scaleY)
    {
        var clip = TopClip(context);
        var c1 = ToCol(instruction.X, scaleX);
        var r1 = ToRow(instruction.Y, scaleY);
        var c2 = ToCol(instruction.X + instruction.Width, scaleX);
        var r2 = ToRow(instruction.Y + instruction.Height, scaleY);

        var hasFill = !string.IsNullOrWhiteSpace(instruction.Fill) &&
            instruction.Fill is not "transparent" and not "none";
        var hasStroke = !string.IsNullOrWhiteSpace(instruction.Stroke);

        if (hasFill)
        {
            for (var row = r1; row <= r2; row++)
            {
                for (var col = c1; col <= c2; col++)
                {
                    context.Buffer.WriteTag(row, col, CellFlags.Fill, clip);
                }
            }
        }

        if (!hasStroke)
        {
            return;
        }

        context.Buffer.WriteTag(r1, c1, CellFlags.Down | CellFlags.Right, clip);
        context.Buffer.WriteTag(r1, c2, CellFlags.Down | CellFlags.Left, clip);
        context.Buffer.WriteTag(r2, c1, CellFlags.Up | CellFlags.Right, clip);
        context.Buffer.WriteTag(r2, c2, CellFlags.Up | CellFlags.Left, clip);

        for (var col = c1 + 1; col < c2; col++)
        {
            context.Buffer.WriteTag(r1, col, CellFlags.Left | CellFlags.Right, clip);
            context.Buffer.WriteTag(r2, col, CellFlags.Left | CellFlags.Right, clip);
        }

        for (var row = r1 + 1; row < r2; row++)
        {
            context.Buffer.WriteTag(row, c1, CellFlags.Up | CellFlags.Down, clip);
            context.Buffer.WriteTag(row, c2, CellFlags.Up | CellFlags.Down, clip);
        }
    }

    private static void HandleLine(PaintLine instruction, AsciiContext context, double scaleX, double scaleY)
    {
        var clip = TopClip(context);
        var c1 = ToCol(instruction.X1, scaleX);
        var r1 = ToRow(instruction.Y1, scaleY);
        var c2 = ToCol(instruction.X2, scaleX);
        var r2 = ToRow(instruction.Y2, scaleY);

        if (r1 == r2)
        {
            var minCol = Math.Min(c1, c2);
            var maxCol = Math.Max(c1, c2);
            for (var col = minCol; col <= maxCol; col++)
            {
                var flags = CellFlags.None;
                if (col > minCol)
                {
                    flags |= CellFlags.Left;
                }

                if (col < maxCol)
                {
                    flags |= CellFlags.Right;
                }

                if (col == minCol && col == maxCol)
                {
                    flags = CellFlags.Left | CellFlags.Right;
                }

                context.Buffer.WriteTag(r1, col, flags, clip);
            }

            return;
        }

        if (c1 == c2)
        {
            var minRow = Math.Min(r1, r2);
            var maxRow = Math.Max(r1, r2);
            for (var row = minRow; row <= maxRow; row++)
            {
                var flags = CellFlags.None;
                if (row > minRow)
                {
                    flags |= CellFlags.Up;
                }

                if (row < maxRow)
                {
                    flags |= CellFlags.Down;
                }

                if (row == minRow && row == maxRow)
                {
                    flags = CellFlags.Up | CellFlags.Down;
                }

                context.Buffer.WriteTag(row, c1, flags, clip);
            }

            return;
        }

        var deltaRow = Math.Abs(r2 - r1);
        var deltaCol = Math.Abs(c2 - c1);
        var stepRow = r1 < r2 ? 1 : -1;
        var stepCol = c1 < c2 ? 1 : -1;
        var error = deltaCol - deltaRow;
        var rowCursor = r1;
        var colCursor = c1;

        while (true)
        {
            var flags = deltaCol > deltaRow
                ? CellFlags.Left | CellFlags.Right
                : CellFlags.Up | CellFlags.Down;
            context.Buffer.WriteTag(rowCursor, colCursor, flags, clip);

            if (rowCursor == r2 && colCursor == c2)
            {
                break;
            }

            var doubled = 2 * error;
            if (doubled > -deltaRow)
            {
                error -= deltaRow;
                colCursor += stepCol;
            }

            if (doubled < deltaCol)
            {
                error += deltaCol;
                rowCursor += stepRow;
            }
        }
    }

    private static void HandleGlyphRun(PaintGlyphRun instruction, AsciiContext context, double scaleX, double scaleY)
    {
        var clip = TopClip(context);
        foreach (var glyph in instruction.Glyphs)
        {
            context.Buffer.WriteChar(
                ToRow(glyph.Y, scaleY),
                ToCol(glyph.X, scaleX),
                ToSafeTerminalGlyph(glyph.GlyphId),
                clip);
        }
    }

    private static void HandleClip(PaintClip instruction, AsciiContext context, PaintVM<AsciiContext> vm, double scaleX, double scaleY)
    {
        var parent = TopClip(context);
        var next = new ClipBounds(
            Math.Max(parent.MinCol, ToCol(instruction.X, scaleX)),
            Math.Max(parent.MinRow, ToRow(instruction.Y, scaleY)),
            Math.Min(parent.MaxCol, ToCol(instruction.X + instruction.Width, scaleX)),
            Math.Min(parent.MaxRow, ToRow(instruction.Y + instruction.Height, scaleY)));

        context.ClipStack.Add(next);
        try
        {
            foreach (var child in instruction.Children)
            {
                vm.Dispatch(child, context);
            }
        }
        finally
        {
            context.ClipStack.RemoveAt(context.ClipStack.Count - 1);
        }
    }

    private static void AssertPlainGroup(PaintGroup group)
    {
        if (!IsIdentityTransform(group.Transform))
        {
            throw new UnsupportedAsciiFeatureError("paint-vm-ascii does not support transformed groups");
        }

        if (group.Opacity is double opacity && opacity != 1.0)
        {
            throw new UnsupportedAsciiFeatureError("paint-vm-ascii does not support group opacity");
        }
    }

    private static void AssertPlainLayer(PaintLayer layer)
    {
        if (!IsIdentityTransform(layer.Transform))
        {
            throw new UnsupportedAsciiFeatureError("paint-vm-ascii does not support transformed layers");
        }

        if (layer.Opacity is double opacity && opacity != 1.0)
        {
            throw new UnsupportedAsciiFeatureError("paint-vm-ascii does not support layer opacity");
        }

        if (layer.Filters is { Count: > 0 })
        {
            throw new UnsupportedAsciiFeatureError("paint-vm-ascii does not support layer filters");
        }

        if (layer.BlendMode is CodingAdventures.PaintInstructions.BlendMode blendMode &&
            blendMode != CodingAdventures.PaintInstructions.BlendMode.Normal)
        {
            throw new UnsupportedAsciiFeatureError("paint-vm-ascii does not support layer blend modes");
        }
    }

    private static string ToSafeTerminalGlyph(int codePoint)
    {
        try
        {
            return IsSafeTerminalCodePoint(codePoint)
                ? char.ConvertFromUtf32(codePoint)
                : "?";
        }
        catch (ArgumentOutOfRangeException)
        {
            return "?";
        }
    }

    private static bool IsSafeTerminalCodePoint(int codePoint)
    {
        if (codePoint < 0x20)
        {
            return false;
        }

        if (codePoint >= 0x7f && codePoint <= 0x9f)
        {
            return false;
        }

        if (codePoint is 0x200e or 0x200f or 0x061c)
        {
            return false;
        }

        if (codePoint >= 0x202a && codePoint <= 0x202e)
        {
            return false;
        }

        return codePoint is < 0x2066 or > 0x2069;
    }

    private static bool IsIdentityTransform(Transform2D? transform) =>
        transform is null ||
        (transform.Value.A == 1.0 &&
         transform.Value.B == 0.0 &&
         transform.Value.C == 0.0 &&
         transform.Value.D == 1.0 &&
         transform.Value.E == 0.0 &&
         transform.Value.F == 0.0);

    private static int ToCol(double x, double scaleX) =>
        (int)Math.Round(x / scaleX, MidpointRounding.AwayFromZero);

    private static int ToRow(double y, double scaleY) =>
        (int)Math.Round(y / scaleY, MidpointRounding.AwayFromZero);

    private static ClipBounds TopClip(AsciiContext context) => context.ClipStack[^1];

    private static ClipBounds FullClip(int cols, int rows) => new(0, 0, cols, rows);
}
