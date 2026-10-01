using System.Buffers;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;
using CodingAdventures.PaintInstructions;

[assembly: InternalsVisibleTo("CodingAdventures.BarcodeLayout1D.Tests")]

namespace CodingAdventures.BarcodeLayout1D;

public sealed class Barcode1DV1Exception(string errorId) : ArgumentException(errorId)
{
    public string ErrorId { get; } = errorId;
}

public sealed record Barcode1DV1Run(
    Barcode1DRunColor Color,
    long Modules,
    string SourceLabel,
    long SourceIndex,
    Barcode1DRunRole Role);

public sealed record Barcode1DV1SymbolDescriptor(
    string Label,
    long Modules,
    long SourceIndex,
    Barcode1DSymbolRole Role);

public sealed record Barcode1DV1SymbolLayout(
    string Label,
    long StartModule,
    long EndModule,
    int SourceIndex,
    Barcode1DSymbolRole Role);

public sealed record Barcode1DV1Layout(
    long LeftQuietZoneModules,
    long RightQuietZoneModules,
    long ContentModules,
    long TotalModules,
    IReadOnlyList<Barcode1DV1SymbolLayout> SymbolLayouts);

public sealed record Barcode1DV1BinaryOptions(
    string SourceLabel,
    long SourceIndex,
    Barcode1DRunRole Role);

public sealed record Barcode1DV1WidthOptions(
    string SourceLabel,
    long SourceIndex,
    Barcode1DRunRole Role)
{
    public string NarrowMarker { get; init; } = "N";
    public string WideMarker { get; init; } = "W";
    public long NarrowModules { get; init; } = 1;
    public long WideModules { get; init; } = 3;
    public Barcode1DRunColor StartingColor { get; init; } = Barcode1DRunColor.Bar;
}

public sealed record Barcode1DV1RenderConfig
{
    public long ModuleWidth { get; init; } = 4;
    public long BarHeight { get; init; } = 120;
    public string Foreground { get; init; } = "#000000";
    public string Background { get; init; } = "#ffffff";
    public bool IncludeHumanReadableText { get; init; }
}

public sealed record Barcode1DV1SceneOptions
{
    public long QuietZoneModules { get; init; } = 10;
    public Barcode1DV1RenderConfig RenderConfig { get; init; } = new();
    public string? HumanReadableText { get; init; }
    public string Label { get; init; } = "1D barcode";
    public IReadOnlyDictionary<string, string> Metadata { get; init; } =
        new Dictionary<string, string>();
    public IReadOnlyList<Barcode1DV1SymbolDescriptor>? Symbols { get; init; }
}

/// <summary>The strict, zero-authority adapter for barcode-layout-1d-v1.</summary>
public static class BarcodeLayout1DV1
{
    private const int MaxPatternScalars = 65_567;
    private const int MaxRuns = 40_979;
    private const long MaxContentModules = 65_567;
    private const long MaxQuietZoneModules = 4_096;
    private const int MaxSymbols = 40_979;
    private const int MaxLabelScalars = 4_096;
    private const int MaxMetadataEntries = 64;
    private const int MaxMetadataKeyScalars = 128;
    private const int MaxMetadataValueScalars = 4_096;
    private const int MaxMetadataBytes = 65_536;
    private const long MaxRenderDimension = 8_192;
    private const int MaxColorScalars = 128;

    public static IReadOnlyList<Barcode1DV1Run> ExpandBinary(
        string pattern,
        Barcode1DV1BinaryOptions options)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        ArgumentNullException.ThrowIfNull(options);
        var scalars = DecodeScalars(pattern, "invalid-binary-token",
            MaxPatternScalars, "pattern-too-long");
        if (scalars.Count == 0) Fail("empty-pattern");
        ValidateSource(options.SourceLabel, options.SourceIndex);
        ValidateRunRole(options.Role);

        var runs = new List<Barcode1DV1Run>();
        var current = scalars[0];
        long width = 1;
        for (var index = 1; index < scalars.Count; index++)
        {
            if (scalars[index] == current)
            {
                width = checked(width + 1);
                continue;
            }
            AppendBinary(runs, current, width, options);
            current = scalars[index];
            width = 1;
        }
        AppendBinary(runs, current, width, options);
        return runs;
    }

    public static IReadOnlyList<Barcode1DV1Run> ExpandWidth(
        string pattern,
        Barcode1DV1WidthOptions options)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        ArgumentNullException.ThrowIfNull(options);
        var scalars = DecodeScalars(pattern, "invalid-width-token",
            MaxPatternScalars, "pattern-too-long");
        if (scalars.Count == 0) Fail("empty-pattern");
        var narrow = SingleScalar(options.NarrowMarker);
        var wide = SingleScalar(options.WideMarker);
        if (narrow is null || wide is null || narrow == wide) Fail("invalid-marker-configuration");
        if (options.NarrowModules <= 0 || options.WideModules <= 0) Fail("invalid-module-count");
        ValidateSource(options.SourceLabel, options.SourceIndex);
        ValidateRunRole(options.Role);
        ValidateColor(options.StartingColor);

        var runs = new List<Barcode1DV1Run>();
        var color = options.StartingColor;
        long content = 0;
        foreach (var marker in scalars)
        {
            var modules = marker == narrow ? options.NarrowModules
                : marker == wide ? options.WideModules
                : throw new Barcode1DV1Exception("invalid-width-token");
            content = CheckedAdd(content, modules, "content-too-wide");
            if (content > MaxContentModules) Fail("content-too-wide");
            if (runs.Count == MaxRuns) Fail("too-many-runs");
            runs.Add(new Barcode1DV1Run(color, modules, options.SourceLabel,
                options.SourceIndex, options.Role));
            color = color == Barcode1DRunColor.Bar
                ? Barcode1DRunColor.Space : Barcode1DRunColor.Bar;
        }
        return runs;
    }

    public static Barcode1DV1Layout ComputeLayout(
        IReadOnlyList<Barcode1DV1Run> runs,
        long quietZoneModules,
        IReadOnlyList<Barcode1DV1SymbolDescriptor>? symbols = null)
    {
        ArgumentNullException.ThrowIfNull(runs);
        var content = ValidateRuns(runs);
        if (quietZoneModules is < 1 or > MaxQuietZoneModules) Fail("invalid-quiet-zone");
        var total = CheckedAdd(CheckedAdd(content, quietZoneModules, "content-too-wide"),
            quietZoneModules, "content-too-wide");
        var layouts = symbols is null
            ? InferSymbols(runs)
            : LayoutSymbols(symbols, content);
        return new Barcode1DV1Layout(quietZoneModules, quietZoneModules, content,
            total, layouts);
    }

    public static PaintScene ProjectScene(
        IReadOnlyList<Barcode1DV1Run> runs,
        Barcode1DV1SceneOptions? options = null) =>
        ProjectSceneForTesting(runs, options, static () =>
            throw new InvalidOperationException("native text resolution is forbidden"));

    internal static PaintScene ProjectSceneForTesting(
        IReadOnlyList<Barcode1DV1Run> runs,
        Barcode1DV1SceneOptions? options,
        Action forbiddenNativeTextResolver)
    {
        ArgumentNullException.ThrowIfNull(runs);
        ArgumentNullException.ThrowIfNull(forbiddenNativeTextResolver);
        options ??= new Barcode1DV1SceneOptions();
        ArgumentNullException.ThrowIfNull(options.RenderConfig);
        if (options.RenderConfig.IncludeHumanReadableText || options.HumanReadableText is not null)
            Fail("human-readable-text-unsupported");

        var config = options.RenderConfig;
        if (config.ModuleWidth is < 1 or > MaxRenderDimension ||
            config.BarHeight is < 1 or > MaxRenderDimension)
            Fail("invalid-render-config");
        ScalarCount(config.Foreground, "invalid-render-config", MaxColorScalars,
            "invalid-render-config");
        ScalarCount(config.Background, "invalid-render-config", MaxColorScalars,
            "invalid-render-config");

        var layout = ComputeLayout(runs, options.QuietZoneModules, options.Symbols);
        ValidateMetadata(options.Metadata, options.Label);
        var rectangles = new List<PaintInstructionBase>();
        long cursor = layout.LeftQuietZoneModules;
        foreach (var run in runs)
        {
            var end = CheckedAdd(cursor, run.Modules, "content-too-wide");
            if (run.Color == Barcode1DRunColor.Bar)
            {
                var x = CheckedMultiply(cursor, config.ModuleWidth, "invalid-render-config");
                var width = CheckedMultiply(run.Modules, config.ModuleWidth, "invalid-render-config");
                rectangles.Add(CodingAdventures.PaintInstructions.PaintInstructions.PaintRect(
                    x, 0, width, config.BarHeight,
                    new PaintRectOptions
                    {
                        Fill = config.Foreground,
                        Metadata = new Dictionary<string, object?>
                        {
                            ["sourceLabel"] = run.SourceLabel,
                            ["sourceIndex"] = Decimal(run.SourceIndex),
                            ["role"] = run.Role.AsString(),
                            ["moduleStart"] = Decimal(cursor),
                            ["moduleEnd"] = Decimal(end),
                        },
                    }));
            }
            cursor = end;
        }

        var sceneWidth = CheckedMultiply(layout.TotalModules, config.ModuleWidth,
            "invalid-render-config");
        var metadata = options.Metadata.ToDictionary(pair => pair.Key,
            pair => (object?)pair.Value, StringComparer.Ordinal);
        metadata["label"] = options.Label;
        metadata["leftQuietZoneModules"] = Decimal(layout.LeftQuietZoneModules);
        metadata["rightQuietZoneModules"] = Decimal(layout.RightQuietZoneModules);
        metadata["contentModules"] = Decimal(layout.ContentModules);
        metadata["totalModules"] = Decimal(layout.TotalModules);
        metadata["moduleWidthPx"] = Decimal(config.ModuleWidth);
        metadata["barHeightPx"] = Decimal(config.BarHeight);
        metadata["sceneWidthPx"] = Decimal(sceneWidth);
        metadata["sceneHeightPx"] = Decimal(config.BarHeight);
        metadata["symbolCount"] = Decimal(layout.SymbolLayouts.Count);
        return CodingAdventures.PaintInstructions.PaintInstructions.PaintScene(
            sceneWidth, config.BarHeight, config.Background, rectangles,
            new SceneOptions { Metadata = metadata });
    }

    private static long ValidateRuns(IReadOnlyList<Barcode1DV1Run> runs)
    {
        if (runs.Count > MaxRuns) Fail("too-many-runs");
        long content = 0;
        for (var index = 0; index < runs.Count; index++)
        {
            var run = runs[index];
            ValidateSource(run.SourceLabel, run.SourceIndex);
            if (run.Modules <= 0) Fail("invalid-module-count");
            ValidateColor(run.Color);
            ValidateRunRole(run.Role);
            if (index > 0 && runs[index - 1].Color == run.Color) Fail("non-alternating-runs");
            content = CheckedAdd(content, run.Modules, "content-too-wide");
            if (content > MaxContentModules) Fail("content-too-wide");
        }
        return content;
    }

    private static IReadOnlyList<Barcode1DV1SymbolLayout> InferSymbols(
        IReadOnlyList<Barcode1DV1Run> runs)
    {
        var result = new List<Barcode1DV1SymbolLayout>();
        long cursor = 0;
        (string Label, int Index, Barcode1DSymbolRole Role, long Start)? current = null;
        foreach (var run in runs)
        {
            var role = ToSymbolRole(run.Role);
            if (role is not null)
            {
                var index = checked((int)run.SourceIndex);
                if (current is null || current.Value.Label != run.SourceLabel ||
                    current.Value.Index != index || current.Value.Role != role)
                {
                    if (current is not null)
                        AddSymbol(result, current.Value, cursor);
                    current = (run.SourceLabel, index, role.Value, cursor);
                }
            }
            cursor = checked(cursor + run.Modules);
        }
        if (current is not null) AddSymbol(result, current.Value, cursor);
        return result;
    }

    private static IReadOnlyList<Barcode1DV1SymbolLayout> LayoutSymbols(
        IReadOnlyList<Barcode1DV1SymbolDescriptor> symbols, long content)
    {
        if (symbols.Count > MaxSymbols) Fail("too-many-symbols");
        var result = new List<Barcode1DV1SymbolLayout>();
        long cursor = 0;
        foreach (var symbol in symbols)
        {
            if (symbol.Modules <= 0) Fail("invalid-module-count");
            ValidateSource(symbol.Label, symbol.SourceIndex);
            if (!Enum.IsDefined(symbol.Role)) Fail("invalid-source-attribution");
            var end = CheckedAdd(cursor, symbol.Modules, "symbol-width-mismatch");
            result.Add(new Barcode1DV1SymbolLayout(symbol.Label, cursor, end,
                checked((int)symbol.SourceIndex), symbol.Role));
            cursor = end;
        }
        if (cursor != content) Fail("symbol-width-mismatch");
        return result;
    }

    private static void AddSymbol(List<Barcode1DV1SymbolLayout> result,
        (string Label, int Index, Barcode1DSymbolRole Role, long Start) symbol,
        long end)
    {
        result.Add(new Barcode1DV1SymbolLayout(symbol.Label, symbol.Start, end,
            symbol.Index, symbol.Role));
        if (result.Count > MaxSymbols) Fail("too-many-symbols");
    }

    private static void AppendBinary(List<Barcode1DV1Run> runs, Rune token,
        long width, Barcode1DV1BinaryOptions options)
    {
        Barcode1DRunColor color;
        if (token.Value == '1') color = Barcode1DRunColor.Bar;
        else if (token.Value == '0') color = Barcode1DRunColor.Space;
        else throw new Barcode1DV1Exception("invalid-binary-token");
        if (runs.Count == MaxRuns) Fail("too-many-runs");
        runs.Add(new Barcode1DV1Run(color, width, options.SourceLabel,
            options.SourceIndex, options.Role));
    }

    private static void ValidateSource(string label, long sourceIndex)
    {
        ScalarCount(label, "invalid-source-attribution", MaxLabelScalars,
            "invalid-source-attribution");
        if (sourceIndex is < int.MinValue or > int.MaxValue)
            Fail("invalid-source-attribution");
    }

    private static void ValidateMetadata(IReadOnlyDictionary<string, string> metadata,
        string label)
    {
        ArgumentNullException.ThrowIfNull(metadata);
        if (metadata.Count > MaxMetadataEntries) Fail("metadata-too-large");
        ScalarCount(label, "metadata-too-large", MaxLabelScalars, "metadata-too-large");
        long bytes = 0;
        foreach (var pair in metadata)
        {
            ScalarCount(pair.Key, "metadata-too-large", MaxMetadataKeyScalars,
                "metadata-too-large");
            ScalarCount(pair.Value, "metadata-too-large", MaxMetadataValueScalars,
                "metadata-too-large");
            bytes = checked(bytes + Encoding.UTF8.GetByteCount(pair.Key));
            bytes = checked(bytes + Encoding.UTF8.GetByteCount(pair.Value));
            if (bytes > MaxMetadataBytes) Fail("metadata-too-large");
        }
    }

    private static List<Rune> DecodeScalars(string value, string error,
        int limit = int.MaxValue, string? limitError = null)
    {
        if (value is null) throw new Barcode1DV1Exception(error);
        var result = new List<Rune>();
        var remaining = value.AsSpan();
        while (!remaining.IsEmpty)
        {
            var status = Rune.DecodeFromUtf16(remaining, out var rune, out var consumed);
            if (status != OperationStatus.Done) throw new Barcode1DV1Exception(error);
            result.Add(rune);
            if (result.Count > limit) throw new Barcode1DV1Exception(limitError ?? error);
            remaining = remaining[consumed..];
        }
        return result;
    }

    private static int ScalarCount(string value, string error, int limit,
        string limitError) => DecodeScalars(value, error, limit, limitError).Count;

    private static Rune? SingleScalar(string value)
    {
        try
        {
            var scalars = DecodeScalars(value, "invalid-marker-configuration", 1,
                "invalid-marker-configuration");
            return scalars.Count == 1 ? scalars[0] : null;
        }
        catch (Barcode1DV1Exception) { return null; }
    }

    private static void ValidateColor(Barcode1DRunColor color)
    {
        if (color is not Barcode1DRunColor.Bar and not Barcode1DRunColor.Space)
            Fail("invalid-source-attribution");
    }

    private static void ValidateRunRole(Barcode1DRunRole role)
    {
        if (!Enum.IsDefined(role)) Fail("invalid-source-attribution");
    }

    private static Barcode1DSymbolRole? ToSymbolRole(Barcode1DRunRole role) => role switch
    {
        Barcode1DRunRole.Data => Barcode1DSymbolRole.Data,
        Barcode1DRunRole.Start => Barcode1DSymbolRole.Start,
        Barcode1DRunRole.Stop => Barcode1DSymbolRole.Stop,
        Barcode1DRunRole.Guard => Barcode1DSymbolRole.Guard,
        Barcode1DRunRole.Check => Barcode1DSymbolRole.Check,
        Barcode1DRunRole.InterCharacterGap => null,
        _ => throw new Barcode1DV1Exception("invalid-source-attribution"),
    };

    private static long CheckedAdd(long left, long right, string error)
    {
        try { return checked(left + right); }
        catch (OverflowException) { throw new Barcode1DV1Exception(error); }
    }

    private static long CheckedMultiply(long left, long right, string error)
    {
        try { return checked(left * right); }
        catch (OverflowException) { throw new Barcode1DV1Exception(error); }
    }

    private static string Decimal(long value) => value.ToString(CultureInfo.InvariantCulture);
    private static void Fail(string id) => throw new Barcode1DV1Exception(id);
}
