using System.Security.Cryptography;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.BarcodeLayout1D.Tests;

public sealed class BarcodeLayout1DConformanceTests
{
    private const string CorpusSha = "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388";
    private const int MaxFixtureBytes = 131_072;

    [Fact]
    public void ExecutesEveryV1CorpusCase()
    {
        var bytes = ReadBoundedFixture(FindCorpus());
        Assert.Equal(CorpusSha, Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant());
        ValidateJsonEnvelope(bytes);
        using var document = JsonDocument.Parse(bytes, new JsonDocumentOptions { MaxDepth = 8 });
        var cases = document.RootElement.GetProperty("cases").EnumerateArray().ToArray();
        Assert.Equal(56, cases.Length);
        foreach (var item in cases)
        {
            var id = item.GetProperty("id").GetString()!;
            var expected = item.GetProperty("expected");
            try
            {
                var actual = Dispatch(item);
                Assert.False(expected.TryGetProperty("error", out _), $"{id}: expected an error");
                var expectedNode = JsonNode.Parse(expected.GetRawText());
                Assert.True(JsonNode.DeepEquals(expectedNode, actual),
                    $"{id}\nexpected: {expectedNode}\nactual:   {actual}");
            }
            catch (Barcode1DV1Exception error)
            {
                Assert.True(expected.TryGetProperty("error", out var expectedError),
                    $"{id}: unexpected {error.ErrorId}");
                Assert.Equal(expectedError.GetString(), error.ErrorId);
            }
        }
    }

    [Fact]
    public void TextRequestsFailBeforeNativeResolution()
    {
        var runs = new[] { new Barcode1DV1Run(Barcode1DRunColor.Bar, 1, "A", 0, Barcode1DRunRole.Data) };
        var calls = 0;
        foreach (var options in new[]
        {
            new Barcode1DV1SceneOptions { HumanReadableText = "A" },
            new Barcode1DV1SceneOptions
            {
                RenderConfig = new Barcode1DV1RenderConfig
                {
                    IncludeHumanReadableText = true,
                    ModuleWidth = 0,
                },
            },
        })
        {
            var error = Assert.Throws<Barcode1DV1Exception>(() =>
                BarcodeLayout1DV1.ProjectSceneForTesting(runs, options, () => calls++));
            Assert.Equal("human-readable-text-unsupported", error.ErrorId);
        }
        Assert.Equal(0, calls);
    }

    [Fact]
    public void ResultsAndMetadataAreDeepOwned()
    {
        var caller = new Dictionary<string, string> { ["owner"] = "caller" };
        var runs = BarcodeLayout1DV1.ExpandBinary("101",
            new Barcode1DV1BinaryOptions("A", 0, Barcode1DRunRole.Data));
        var first = BarcodeLayout1DV1.ProjectScene(runs,
            new Barcode1DV1SceneOptions { Metadata = caller });
        caller["owner"] = "changed";
        var firstMetadata = Assert.IsAssignableFrom<IReadOnlyDictionary<string, object?>>(first.Metadata);
        Assert.Equal("caller", firstMetadata["owner"]);
        var second = BarcodeLayout1DV1.ProjectScene(runs,
            new Barcode1DV1SceneOptions { Metadata = new Dictionary<string, string> { ["owner"] = "caller" } });
        Assert.NotSame(first.Instructions, second.Instructions);
        Assert.NotSame(first.Metadata, second.Metadata);
        Assert.NotSame(first.Instructions[0].Metadata, second.Instructions[0].Metadata);
    }

    [Fact]
    public void PatternScannerStopsAtTheNormativeLimit()
    {
        var pattern = new string('1', 65_568) + "\ud800";
        var error = Assert.Throws<Barcode1DV1Exception>(() =>
            BarcodeLayout1DV1.ExpandBinary(pattern,
                new Barcode1DV1BinaryOptions("A", 0, Barcode1DRunRole.Data)));
        Assert.Equal("pattern-too-long", error.ErrorId);
    }

    [Fact]
    public void FixtureLoaderRejectsDuplicateKeysAndInvalidRepeatCounts()
    {
        Assert.Throws<InvalidDataException>(() => ValidateJsonEnvelope("{\"a\":1,\"a\":2}"u8));
        using var negative = JsonDocument.Parse("{\"repeat\":{\"token\":\"1\",\"count\":-1}}");
        Assert.Throws<InvalidDataException>(() => Pattern(negative.RootElement));
        using var tooManyRuns = JsonDocument.Parse("{\"repeatRuns\":{\"count\":40980}}");
        var runError = Assert.Throws<Barcode1DV1Exception>(() => Runs(tooManyRuns.RootElement));
        Assert.Equal("too-many-runs", runError.ErrorId);
        using var tooManySymbols = JsonDocument.Parse("{\"repeatSymbols\":{\"count\":40980}}");
        var symbolError = Assert.Throws<Barcode1DV1Exception>(() => Symbols(tooManySymbols.RootElement));
        Assert.Equal("too-many-symbols", symbolError.ErrorId);
    }

    private static JsonObject Dispatch(JsonElement item)
    {
        var operation = item.GetProperty("operation").GetString();
        var input = item.GetProperty("input");
        return operation switch
        {
            "expand-binary" => RunResult(BarcodeLayout1DV1.ExpandBinary(
                Pattern(input), new Barcode1DV1BinaryOptions(
                    RequiredString(input, "sourceLabel"), RequiredInt(input, "sourceIndex"),
                    RunRole(RequiredString(input, "role"))))),
            "expand-width" => RunResult(BarcodeLayout1DV1.ExpandWidth(
                Pattern(input), WidthOptions(input))),
            "compute-layout" => new JsonObject { ["layout"] = LayoutNode(
                BarcodeLayout1DV1.ComputeLayout(Runs(input), RequiredInt(input, "quietZoneModules"), Symbols(input))) },
            "project-scene" => new JsonObject { ["scene"] = SceneNode(
                BarcodeLayout1DV1.ProjectScene(Runs(input), SceneOptions(input))) },
            _ => throw new InvalidOperationException(operation),
        };
    }

    private static JsonObject RunResult(IReadOnlyList<Barcode1DV1Run> runs)
    {
        if (runs.Count <= 1_000)
            return new JsonObject { ["runs"] = new JsonArray(runs.Select(RunNode).ToArray()) };
        var bytes = JsonSerializer.SerializeToUtf8Bytes(runs.Select(CanonicalRun).ToArray(),
            new JsonSerializerOptions { Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping });
        return new JsonObject
        {
            ["runDigest"] = new JsonObject
            {
                ["runCount"] = runs.Count,
                ["contentModules"] = runs.Sum(run => run.Modules),
                ["firstRun"] = RunNode(runs[0]),
                ["lastRun"] = RunNode(runs[^1]),
                ["runsSha256"] = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant(),
            },
        };
    }

    private static SortedDictionary<string, object> CanonicalRun(Barcode1DV1Run run) => new(StringComparer.Ordinal)
    {
        ["color"] = run.Color.AsString(), ["modules"] = run.Modules,
        ["role"] = run.Role.AsString(), ["sourceIndex"] = run.SourceIndex,
        ["sourceLabel"] = run.SourceLabel,
    };

    private static JsonObject RunNode(Barcode1DV1Run run) => new()
    {
        ["color"] = run.Color.AsString(), ["modules"] = run.Modules,
        ["sourceLabel"] = run.SourceLabel, ["sourceIndex"] = run.SourceIndex,
        ["role"] = run.Role.AsString(),
    };

    private static JsonObject LayoutNode(Barcode1DV1Layout layout) => new()
    {
        ["leftQuietZoneModules"] = layout.LeftQuietZoneModules,
        ["rightQuietZoneModules"] = layout.RightQuietZoneModules,
        ["contentModules"] = layout.ContentModules, ["totalModules"] = layout.TotalModules,
        ["symbolLayouts"] = new JsonArray(layout.SymbolLayouts.Select(symbol => new JsonObject
        {
            ["label"] = symbol.Label, ["startModule"] = symbol.StartModule,
            ["endModule"] = symbol.EndModule, ["sourceIndex"] = symbol.SourceIndex,
            ["role"] = symbol.Role.AsString(),
        }).ToArray()),
    };

    private static JsonObject SceneNode(PaintScene scene)
    {
        var rectangles = new JsonArray();
        foreach (var instruction in scene.Instructions)
        {
            var rect = Assert.IsType<PaintRect>(instruction);
            rectangles.Add(new JsonObject
            {
                ["x"] = checked((long)rect.X), ["y"] = checked((long)rect.Y),
                ["width"] = checked((long)rect.Width), ["height"] = checked((long)rect.Height),
                ["fill"] = rect.Fill,
                ["metadata"] = MetadataNode(rect.Metadata!),
            });
        }
        return new JsonObject
        {
            ["width"] = checked((long)scene.Width), ["height"] = checked((long)scene.Height),
            ["background"] = scene.Background, ["rectangles"] = rectangles,
            ["metadata"] = MetadataNode(scene.Metadata!),
        };
    }

    private static JsonObject MetadataNode(IReadOnlyDictionary<string, object?> metadata)
    {
        var result = new JsonObject();
        foreach (var pair in metadata.OrderBy(pair => pair.Key, StringComparer.Ordinal))
            result[pair.Key] = Assert.IsType<string>(pair.Value);
        return result;
    }

    private static Barcode1DV1WidthOptions WidthOptions(JsonElement input) =>
        new(RequiredString(input, "sourceLabel"), RequiredInt(input, "sourceIndex"),
            RunRole(RequiredString(input, "role")))
        {
            NarrowMarker = OptionalString(input, "narrowMarker", "N"),
            WideMarker = OptionalString(input, "wideMarker", "W"),
            NarrowModules = OptionalInt(input, "narrowModules", 1),
            WideModules = OptionalInt(input, "wideModules", 3),
            StartingColor = Color(OptionalString(input, "startingColor", "bar")),
        };

    private static Barcode1DV1SceneOptions SceneOptions(JsonElement input)
    {
        var render = input.TryGetProperty("renderConfig", out var value) ? value : default;
        var metadata = input.TryGetProperty("metadata", out var map)
            ? map.EnumerateObject().ToDictionary(p => p.Name, p => p.Value.GetString()!)
            : new Dictionary<string, string>();
        return new Barcode1DV1SceneOptions
        {
            QuietZoneModules = RequiredInt(input, "quietZoneModules"),
            Label = OptionalString(input, "label", "1D barcode"), Metadata = metadata,
            HumanReadableText = input.TryGetProperty("humanReadableText", out var text) && text.ValueKind != JsonValueKind.Null
                ? text.GetString() : null,
            Symbols = Symbols(input),
            RenderConfig = new Barcode1DV1RenderConfig
            {
                ModuleWidth = render.ValueKind == JsonValueKind.Object ? OptionalInt(render, "moduleWidth", 4) : 4,
                BarHeight = render.ValueKind == JsonValueKind.Object ? OptionalInt(render, "barHeight", 120) : 120,
                Foreground = render.ValueKind == JsonValueKind.Object ? OptionalString(render, "foreground", "#000000") : "#000000",
                Background = render.ValueKind == JsonValueKind.Object ? OptionalString(render, "background", "#ffffff") : "#ffffff",
                IncludeHumanReadableText = render.ValueKind == JsonValueKind.Object &&
                    render.TryGetProperty("includeHumanReadableText", out var enabled) && enabled.GetBoolean(),
            },
        };
    }

    private static IReadOnlyList<Barcode1DV1Run> Runs(JsonElement input)
    {
        if (input.TryGetProperty("runs", out var values))
        {
            if (values.ValueKind != JsonValueKind.Array) throw new InvalidDataException("fixture-invalid-type");
            if (values.GetArrayLength() > 40_979) throw new Barcode1DV1Exception("too-many-runs");
            return values.EnumerateArray().Select(Run).ToArray();
        }
        var repeat = input.GetProperty("repeatRuns");
        var count = FixtureCount(repeat, "count", 40_979, "too-many-runs");
        var modules = RequiredInt(repeat, "modules");
        var label = RequiredString(repeat, "sourceLabel");
        var index = RequiredInt(repeat, "sourceIndex");
        var role = RunRole(RequiredString(repeat, "role"));
        var first = Color(RequiredString(repeat, "firstColor"));
        return Enumerable.Range(0, count).Select(i => new Barcode1DV1Run(
            i % 2 == 0 ? first : Toggle(first), modules, label, index, role)).ToArray();
    }

    private static Barcode1DV1Run Run(JsonElement run) => new(
        Color(RequiredString(run, "color")), RequiredInt(run, "modules"),
        RequiredString(run, "sourceLabel"), RequiredInt(run, "sourceIndex"),
        RunRole(RequiredString(run, "role")));

    private static IReadOnlyList<Barcode1DV1SymbolDescriptor>? Symbols(JsonElement input)
    {
        if (input.TryGetProperty("symbols", out var values))
        {
            if (values.ValueKind != JsonValueKind.Array) throw new InvalidDataException("fixture-invalid-type");
            if (values.GetArrayLength() > 40_979) throw new Barcode1DV1Exception("too-many-symbols");
            return values.EnumerateArray().Select(Symbol).ToArray();
        }
        if (!input.TryGetProperty("repeatSymbols", out var repeat)) return null;
        var count = FixtureCount(repeat, "count", 40_979, "too-many-symbols");
        return Enumerable.Range(0, count).Select(index => new Barcode1DV1SymbolDescriptor(
            RequiredString(repeat, "label"), RequiredInt(repeat, "modules"), index,
            SymbolRole(RequiredString(repeat, "role")))).ToArray();
    }

    private static Barcode1DV1SymbolDescriptor Symbol(JsonElement symbol) => new(
        RequiredString(symbol, "label"), RequiredInt(symbol, "modules"),
        RequiredInt(symbol, "sourceIndex"), SymbolRole(RequiredString(symbol, "role")));

    private static string Pattern(JsonElement input)
    {
        if (input.TryGetProperty("pattern", out var pattern)) return pattern.GetString()!;
        var repeat = input.GetProperty("repeat");
        var token = RequiredString(repeat, "token");
        var suffix = OptionalString(repeat, "suffix", "");
        var count = FixtureCount(repeat, "count", 65_568, "pattern-too-long");
        var scalarCount = checked((long)token.EnumerateRunes().Count() * count + suffix.EnumerateRunes().Count());
        if (scalarCount > 65_567) throw new Barcode1DV1Exception("pattern-too-long");
        return string.Concat(Enumerable.Repeat(token, count)) + suffix;
    }

    private static int FixtureCount(JsonElement value, string name, int limit, string errorId)
    {
        var property = value.GetProperty(name);
        if (property.ValueKind != JsonValueKind.Number || !property.TryGetInt64(out var count) || count < 0)
            throw new InvalidDataException("fixture-invalid-count");
        if (count > limit) throw new Barcode1DV1Exception(errorId);
        return checked((int)count);
    }

    private static long RequiredInt(JsonElement value, string name) => value.GetProperty(name).GetInt64();
    private static long OptionalInt(JsonElement value, string name, long fallback) =>
        value.TryGetProperty(name, out var found) ? found.GetInt64() : fallback;
    private static string RequiredString(JsonElement value, string name) => value.GetProperty(name).GetString()!;
    private static string OptionalString(JsonElement value, string name, string fallback) =>
        value.TryGetProperty(name, out var found) ? found.GetString()! : fallback;
    private static Barcode1DRunColor Color(string value) => value == "bar" ? Barcode1DRunColor.Bar : Barcode1DRunColor.Space;
    private static Barcode1DRunColor Toggle(Barcode1DRunColor value) => value == Barcode1DRunColor.Bar ? Barcode1DRunColor.Space : Barcode1DRunColor.Bar;
    private static Barcode1DRunRole RunRole(string value) => value switch
    {
        "data" => Barcode1DRunRole.Data, "start" => Barcode1DRunRole.Start,
        "stop" => Barcode1DRunRole.Stop, "guard" => Barcode1DRunRole.Guard,
        "check" => Barcode1DRunRole.Check, _ => Barcode1DRunRole.InterCharacterGap,
    };
    private static Barcode1DSymbolRole SymbolRole(string value) => value switch
    {
        "data" => Barcode1DSymbolRole.Data, "start" => Barcode1DSymbolRole.Start,
        "stop" => Barcode1DSymbolRole.Stop, "guard" => Barcode1DSymbolRole.Guard,
        _ => Barcode1DSymbolRole.Check,
    };

    private static string FindCorpus()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            var candidate = Path.Combine(directory.FullName, "code", "specs", "fixtures",
                "barcode-layout-1d-v1", "cases.json");
            if (File.Exists(candidate)) return candidate;
        }
        throw new FileNotFoundException("barcode-layout-1d-v1/cases.json");
    }

    private static byte[] ReadBoundedFixture(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        if (stream.Length is < 1 or > MaxFixtureBytes) throw new InvalidDataException("fixture-size-limit");
        var bytes = new byte[checked((int)stream.Length)];
        stream.ReadExactly(bytes);
        return bytes;
    }

    private static void ValidateJsonEnvelope(ReadOnlySpan<byte> bytes)
    {
        var reader = new Utf8JsonReader(bytes, new JsonReaderOptions { MaxDepth = 8 });
        if (!reader.Read()) throw new InvalidDataException("fixture-invalid-json");
        ScanJsonValue(ref reader, 0);
        if (reader.Read()) throw new InvalidDataException("fixture-trailing-data");
    }

    private static void ScanJsonValue(ref Utf8JsonReader reader, int depth)
    {
        if (depth > 8) throw new InvalidDataException("fixture-depth-limit");
        if (reader.TokenType == JsonTokenType.String)
        {
            ValidateFixtureString(reader.GetString()!);
            return;
        }
        if (reader.TokenType == JsonTokenType.Number && !reader.TryGetInt64(out _))
            throw new InvalidDataException("fixture-invalid-number");
        if (reader.TokenType == JsonTokenType.StartArray)
        {
            while (reader.Read() && reader.TokenType != JsonTokenType.EndArray)
                ScanJsonValue(ref reader, depth + 1);
            if (reader.TokenType != JsonTokenType.EndArray) throw new InvalidDataException("fixture-invalid-json");
            return;
        }
        if (reader.TokenType != JsonTokenType.StartObject) return;
        var names = new HashSet<string>(StringComparer.Ordinal);
        while (reader.Read() && reader.TokenType != JsonTokenType.EndObject)
        {
            if (reader.TokenType != JsonTokenType.PropertyName) throw new InvalidDataException("fixture-invalid-json");
            var name = reader.GetString()!;
            ValidateFixtureString(name);
            if (!names.Add(name)) throw new InvalidDataException("fixture-duplicate-key");
            if (!reader.Read()) throw new InvalidDataException("fixture-invalid-json");
            ScanJsonValue(ref reader, depth + 1);
        }
        if (reader.TokenType != JsonTokenType.EndObject) throw new InvalidDataException("fixture-invalid-json");
    }

    private static void ValidateFixtureString(string value)
    {
        var remaining = value.AsSpan();
        while (!remaining.IsEmpty)
        {
            var status = Rune.DecodeFromUtf16(remaining, out _, out var consumed);
            if (status != System.Buffers.OperationStatus.Done) throw new InvalidDataException("fixture-invalid-scalar");
            remaining = remaining[consumed..];
        }
    }
}
