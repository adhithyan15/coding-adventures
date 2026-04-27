using CodingAdventures.PaintInstructions;

namespace CodingAdventures.BarcodeLayout1D;

public enum Barcode1DRunColor
{
    Bar,
    Space,
}

public enum Barcode1DRunRole
{
    Data,
    Start,
    Stop,
    Guard,
    Check,
    InterCharacterGap,
}

public enum Barcode1DSymbolRole
{
    Data,
    Start,
    Stop,
    Guard,
    Check,
}

public enum Barcode1DLayoutTarget
{
    NativePaintVm,
    CanvasPaintVm,
    DomPaintVm,
}

public sealed record Barcode1DRun(
    Barcode1DRunColor Color,
    uint Modules,
    string SourceLabel,
    int SourceIndex,
    Barcode1DRunRole Role);

public sealed record Barcode1DSymbolLayout(
    string Label,
    uint StartModule,
    uint EndModule,
    int SourceIndex,
    Barcode1DSymbolRole Role);

public sealed record Barcode1DLayout(
    uint LeftQuietZoneModules,
    uint RightQuietZoneModules,
    uint ContentModules,
    uint TotalModules,
    IReadOnlyList<Barcode1DSymbolLayout> SymbolLayouts);

public sealed record Barcode1DSymbolDescriptor(
    string Label,
    uint Modules,
    int SourceIndex,
    Barcode1DSymbolRole Role);

public sealed record Barcode1DRenderConfig
{
    public Barcode1DLayoutTarget LayoutTarget { get; init; } = Barcode1DLayoutTarget.NativePaintVm;
    public double ModuleWidth { get; init; } = 4.0;
    public double BarHeight { get; init; } = 120.0;
    public uint QuietZoneModules { get; init; } = 10;
    public bool IncludeHumanReadableText { get; init; }
    public double TextFontSize { get; init; } = 16.0;
    public double TextMargin { get; init; } = 8.0;
    public string Foreground { get; init; } = "#000000";
    public string Background { get; init; } = "#ffffff";
}

public sealed record PaintBarcode1DOptions
{
    public Barcode1DRenderConfig RenderConfig { get; init; } = new();
    public string? HumanReadableText { get; init; }
    public IReadOnlyDictionary<string, object?> Metadata { get; init; } = new Dictionary<string, object?>();
    public string? Label { get; init; }
    public IReadOnlyList<Barcode1DSymbolDescriptor>? Symbols { get; init; }
}

public sealed record RunsFromBinaryPatternOptions(
    string SourceLabel,
    int SourceIndex,
    Barcode1DRunRole Role);

public sealed record RunsFromWidthPatternOptions(
    string SourceLabel,
    int SourceIndex,
    Barcode1DRunRole Role)
{
    public uint NarrowModules { get; init; } = 1;
    public uint WideModules { get; init; } = 3;
    public char NarrowMarker { get; init; } = 'N';
    public char WideMarker { get; init; } = 'W';
    public Barcode1DRunColor StartingColor { get; init; } = Barcode1DRunColor.Bar;
}

public static class BarcodeLayout1D
{
    public const string VERSION = "0.1.0";

    public static string AsString(this Barcode1DRunColor color) =>
        color switch
        {
            Barcode1DRunColor.Bar => "bar",
            Barcode1DRunColor.Space => "space",
            _ => throw new ArgumentOutOfRangeException(nameof(color), color, null),
        };

    public static string AsString(this Barcode1DRunRole role) =>
        role switch
        {
            Barcode1DRunRole.Data => "data",
            Barcode1DRunRole.Start => "start",
            Barcode1DRunRole.Stop => "stop",
            Barcode1DRunRole.Guard => "guard",
            Barcode1DRunRole.Check => "check",
            Barcode1DRunRole.InterCharacterGap => "inter-character-gap",
            _ => throw new ArgumentOutOfRangeException(nameof(role), role, null),
        };

    public static string AsString(this Barcode1DSymbolRole role) =>
        role switch
        {
            Barcode1DSymbolRole.Data => "data",
            Barcode1DSymbolRole.Start => "start",
            Barcode1DSymbolRole.Stop => "stop",
            Barcode1DSymbolRole.Guard => "guard",
            Barcode1DSymbolRole.Check => "check",
            _ => throw new ArgumentOutOfRangeException(nameof(role), role, null),
        };

    public static string AsString(this Barcode1DLayoutTarget target) =>
        target switch
        {
            Barcode1DLayoutTarget.NativePaintVm => "native-paint-vm",
            Barcode1DLayoutTarget.CanvasPaintVm => "canvas-paint-vm",
            Barcode1DLayoutTarget.DomPaintVm => "dom-paint-vm",
            _ => throw new ArgumentOutOfRangeException(nameof(target), target, null),
        };

    public static uint TotalModules(IEnumerable<Barcode1DRun> runs)
    {
        ArgumentNullException.ThrowIfNull(runs);
        return runs.Aggregate(0u, (sum, run) => sum + run.Modules);
    }

    public static Barcode1DLayout ComputeBarcode1DLayout(
        IReadOnlyList<Barcode1DRun> runs,
        uint quietZoneModules,
        IReadOnlyList<Barcode1DSymbolDescriptor>? symbols = null)
    {
        ArgumentNullException.ThrowIfNull(runs);
        ValidateRuns(runs);
        if (quietZoneModules == 0)
        {
            throw new ArgumentException("quiet_zone_modules must be greater than zero.", nameof(quietZoneModules));
        }

        var contentModules = TotalModules(runs);
        var symbolLayouts = symbols is null
            ? InferSymbolLayouts(runs)
            : LayoutExplicitSymbols(symbols, contentModules);

        return new Barcode1DLayout(
            quietZoneModules,
            quietZoneModules,
            contentModules,
            quietZoneModules + contentModules + quietZoneModules,
            symbolLayouts);
    }

    public static IReadOnlyList<Barcode1DRun> RunsFromBinaryPattern(
        string pattern,
        RunsFromBinaryPatternOptions options)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        ArgumentNullException.ThrowIfNull(options);
        if (pattern.Length == 0)
        {
            throw new ArgumentException("Binary pattern must not be empty.", nameof(pattern));
        }

        if (pattern.Any(bit => bit is not ('0' or '1')))
        {
            throw new ArgumentException("Binary pattern must contain only 0 or 1.", nameof(pattern));
        }

        var runs = new List<Barcode1DRun>();
        var currentBit = pattern[0];
        var width = 1u;

        for (var index = 1; index < pattern.Length; index++)
        {
            if (pattern[index] == currentBit)
            {
                width++;
                continue;
            }

            runs.Add(MakeRun(currentBit, width, options.SourceLabel, options.SourceIndex, options.Role));
            currentBit = pattern[index];
            width = 1;
        }

        runs.Add(MakeRun(currentBit, width, options.SourceLabel, options.SourceIndex, options.Role));
        return runs;
    }

    public static IReadOnlyList<Barcode1DRun> RunsFromWidthPattern(
        string pattern,
        RunsFromWidthPatternOptions options)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        ArgumentNullException.ThrowIfNull(options);
        if (pattern.Length == 0)
        {
            throw new ArgumentException("Width pattern must not be empty.", nameof(pattern));
        }

        if (options.NarrowModules == 0 || options.WideModules == 0)
        {
            throw new ArgumentException("Narrow and wide module counts must be greater than zero.", nameof(options));
        }

        var runs = new List<Barcode1DRun>();
        var color = options.StartingColor;
        foreach (var marker in pattern)
        {
            var modules = marker == options.NarrowMarker
                ? options.NarrowModules
                : marker == options.WideMarker
                    ? options.WideModules
                    : throw new ArgumentException($"Unknown width marker '{marker}'.", nameof(pattern));

            runs.Add(new Barcode1DRun(color, modules, options.SourceLabel, options.SourceIndex, options.Role));
            color = color == Barcode1DRunColor.Bar ? Barcode1DRunColor.Space : Barcode1DRunColor.Bar;
        }

        return runs;
    }

    public static PaintScene LayoutBarcode1D(
        IReadOnlyList<Barcode1DRun> runs,
        PaintBarcode1DOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(runs);
        options ??= new PaintBarcode1DOptions();
        ValidateRenderConfig(options.RenderConfig);

        if (options.RenderConfig.IncludeHumanReadableText)
        {
            throw new NotSupportedException("Human-readable text shaping is not wired for dotnet barcode-layout-1d yet.");
        }

        var layout = ComputeBarcode1DLayout(runs, options.RenderConfig.QuietZoneModules, options.Symbols);
        var instructions = new List<PaintInstructionBase>();
        var moduleCursor = layout.LeftQuietZoneModules;

        foreach (var run in runs)
        {
            var x = moduleCursor * options.RenderConfig.ModuleWidth;
            var width = run.Modules * options.RenderConfig.ModuleWidth;
            if (run.Color == Barcode1DRunColor.Bar)
            {
                instructions.Add(CodingAdventures.PaintInstructions.PaintInstructions.PaintRect(
                    x,
                    0,
                    width,
                    options.RenderConfig.BarHeight,
                    new PaintRectOptions
                    {
                        Fill = options.RenderConfig.Foreground,
                        Metadata = new Dictionary<string, object?>
                        {
                            ["sourceLabel"] = run.SourceLabel,
                            ["sourceIndex"] = run.SourceIndex,
                            ["role"] = run.Role.AsString(),
                            ["moduleStart"] = moduleCursor,
                            ["moduleEnd"] = moduleCursor + run.Modules,
                        },
                    }));
            }

            moduleCursor += run.Modules;
        }

        var sceneWidth = layout.TotalModules * options.RenderConfig.ModuleWidth;
        var sceneHeight = options.RenderConfig.BarHeight;
        var metadata = new Dictionary<string, object?>(options.Metadata)
        {
            ["label"] = options.Label ?? "1D barcode",
            ["leftQuietZoneModules"] = layout.LeftQuietZoneModules,
            ["rightQuietZoneModules"] = layout.RightQuietZoneModules,
            ["contentModules"] = layout.ContentModules,
            ["totalModules"] = layout.TotalModules,
            ["moduleWidthPx"] = options.RenderConfig.ModuleWidth,
            ["barHeightPx"] = options.RenderConfig.BarHeight,
            ["sceneWidthPx"] = sceneWidth,
            ["sceneHeightPx"] = sceneHeight,
            ["symbolCount"] = layout.SymbolLayouts.Count,
            ["layoutTarget"] = options.RenderConfig.LayoutTarget.AsString(),
        };

        if (options.HumanReadableText is not null)
        {
            metadata["humanReadableText"] = options.HumanReadableText;
        }

        return CodingAdventures.PaintInstructions.PaintInstructions.PaintScene(
            sceneWidth,
            sceneHeight,
            options.RenderConfig.Background,
            instructions,
            new SceneOptions { Metadata = metadata });
    }

    private static Barcode1DRun MakeRun(char bit, uint modules, string sourceLabel, int sourceIndex, Barcode1DRunRole role) =>
        new(bit == '1' ? Barcode1DRunColor.Bar : Barcode1DRunColor.Space, modules, sourceLabel, sourceIndex, role);

    private static IReadOnlyList<Barcode1DSymbolLayout> LayoutExplicitSymbols(
        IReadOnlyList<Barcode1DSymbolDescriptor> symbols,
        uint contentModules)
    {
        var layouts = new List<Barcode1DSymbolLayout>();
        var cursor = 0u;
        foreach (var symbol in symbols)
        {
            if (symbol.Modules == 0)
            {
                throw new ArgumentException($"Symbol '{symbol.Label}' modules must be greater than zero.", nameof(symbols));
            }

            layouts.Add(new Barcode1DSymbolLayout(symbol.Label, cursor, cursor + symbol.Modules, symbol.SourceIndex, symbol.Role));
            cursor += symbol.Modules;
        }

        if (cursor != contentModules)
        {
            throw new ArgumentException("Symbol descriptors must add up to the same total width as the run stream.", nameof(symbols));
        }

        return layouts;
    }

    private static IReadOnlyList<Barcode1DSymbolLayout> InferSymbolLayouts(IReadOnlyList<Barcode1DRun> runs)
    {
        var layouts = new List<Barcode1DSymbolLayout>();
        var cursor = 0u;
        uint currentStart = 0;
        string? currentLabel = null;
        var currentSourceIndex = 0;
        Barcode1DSymbolRole? currentRole = null;

        foreach (var run in runs)
        {
            var symbolRole = ToSymbolRole(run.Role);
            if (symbolRole is not null)
            {
                var sameSymbol = currentLabel == run.SourceLabel
                    && currentSourceIndex == run.SourceIndex
                    && currentRole == symbolRole;
                if (!sameSymbol)
                {
                    Flush();
                    currentStart = cursor;
                    currentLabel = run.SourceLabel;
                    currentSourceIndex = run.SourceIndex;
                    currentRole = symbolRole;
                }
            }

            cursor += run.Modules;
        }

        Flush();
        return layouts;

        void Flush()
        {
            if (currentLabel is not null && currentRole is not null)
            {
                layouts.Add(new Barcode1DSymbolLayout(currentLabel, currentStart, cursor, currentSourceIndex, currentRole.Value));
            }
        }
    }

    private static Barcode1DSymbolRole? ToSymbolRole(Barcode1DRunRole role) =>
        role switch
        {
            Barcode1DRunRole.Data => Barcode1DSymbolRole.Data,
            Barcode1DRunRole.Start => Barcode1DSymbolRole.Start,
            Barcode1DRunRole.Stop => Barcode1DSymbolRole.Stop,
            Barcode1DRunRole.Guard => Barcode1DSymbolRole.Guard,
            Barcode1DRunRole.Check => Barcode1DSymbolRole.Check,
            Barcode1DRunRole.InterCharacterGap => null,
            _ => throw new ArgumentOutOfRangeException(nameof(role), role, null),
        };

    private static void ValidateRuns(IReadOnlyList<Barcode1DRun> runs)
    {
        for (var index = 0; index < runs.Count; index++)
        {
            if (runs[index].Modules == 0)
            {
                throw new ArgumentException($"runs[{index}].modules must be greater than zero.", nameof(runs));
            }

            if (index > 0 && runs[index - 1].Color == runs[index].Color)
            {
                throw new ArgumentException("Runs must alternate between bars and spaces.", nameof(runs));
            }
        }
    }

    private static void ValidateRenderConfig(Barcode1DRenderConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        ValidatePositive(config.ModuleWidth, nameof(config.ModuleWidth));
        ValidatePositive(config.BarHeight, nameof(config.BarHeight));
        ValidatePositive(config.TextFontSize, nameof(config.TextFontSize));

        if (config.QuietZoneModules == 0)
        {
            throw new ArgumentException("Quiet zone modules must be greater than zero.", nameof(config));
        }

        if (!double.IsFinite(config.TextMargin) || config.TextMargin < 0)
        {
            throw new ArgumentException("Text margin must be zero or greater.", nameof(config));
        }
    }

    private static void ValidatePositive(double value, string name)
    {
        if (!double.IsFinite(value) || value <= 0)
        {
            throw new ArgumentException($"{name} must be a positive number.");
        }
    }
}
