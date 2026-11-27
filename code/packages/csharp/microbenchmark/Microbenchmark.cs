using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using CodingAdventures.RankStatistics;

namespace CodingAdventures.Microbenchmark;

/// <summary>Writes complete CLI text and surfaces broken output streams.</summary>
public static class CliOutput
{
    public static void WriteAndFlush(TextWriter writer, string text)
    {
        ArgumentNullException.ThrowIfNull(writer);
        writer.Write(text);
        writer.Flush();
    }
}

/// <summary>Parameters fixed before a paired collection begins.</summary>
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
public sealed record Config
{
    [JsonPropertyName("pairs"), JsonRequired] public int Pairs { get; init; } = 40;
    [JsonPropertyName("confidence_percent"), JsonRequired] public int ConfidencePercent { get; init; } = 95;
    [JsonPropertyName("minimum_factor"), JsonRequired] public double MinimumFactor { get; init; } = 1.05;
    [JsonPropertyName("seed"), JsonRequired] public long Seed { get; init; }
    [JsonPropertyName("target_ns"), JsonRequired] public long TargetNs { get; init; } = 100_000;
    [JsonPropertyName("max_batch"), JsonRequired] public long MaxBatch { get; init; } = 65_536;

    /// <summary>Rejects invalid ranges and non-finite comparison factors.</summary>
    public Config Validate()
    {
        if (Pairs is < 1 or > Microbenchmark.MaximumPairs)
            throw new ArgumentOutOfRangeException(nameof(Pairs), "pairs must be from 1 through 10000");
        if (ConfidencePercent is < 1 or > 99)
            throw new ArgumentOutOfRangeException(nameof(ConfidencePercent), "confidence must be from 1 through 99");
        if (!double.IsFinite(MinimumFactor) || MinimumFactor <= 1.0)
            throw new ArgumentOutOfRangeException(nameof(MinimumFactor), "minimum factor must be finite and greater than 1");
        if (TargetNs <= 0) throw new ArgumentOutOfRangeException(nameof(TargetNs), "target must be positive");
        if (MaxBatch <= 0) throw new ArgumentOutOfRangeException(nameof(MaxBatch), "maximum batch must be positive");
        return this;
    }
}

/// <summary>A single adjacent comparison pair.</summary>
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
public sealed class Pair
{
    [JsonPropertyName("baseline_ns"), JsonRequired] public long BaselineNs { get; init; }
    [JsonPropertyName("candidate_ns"), JsonRequired] public long CandidateNs { get; init; }
    [JsonPropertyName("order"), JsonRequired] public string Order { get; init; } = "";
}

/// <summary>Effect estimate and its exact order-statistic interval.</summary>
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
public sealed class Analysis
{
    [JsonPropertyName("ratio"), JsonRequired] public double Ratio { get; init; }
    [JsonPropertyName("interval"), JsonRequired, JsonIgnore(Condition = JsonIgnoreCondition.Never)] public double[]? Interval { get; init; }
    [JsonPropertyName("verdict"), JsonRequired] public string Verdict { get; init; } = "";
}

/// <summary>Paired observations, reproducibility metadata, and computed analysis.</summary>
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
public sealed class Report
{
    [JsonPropertyName("config"), JsonRequired] public Config Config { get; init; } = new();
    [JsonPropertyName("batch_size"), JsonRequired] public long BatchSize { get; init; }
    [JsonPropertyName("pairs"), JsonRequired] public List<Pair> Pairs { get; init; } = [];
    [JsonPropertyName("analysis"), JsonRequired] public Analysis Analysis { get; init; } = new();
    [JsonPropertyName("workload")] public string? Workload { get; init; }
    [JsonPropertyName("invocation")] public string[]? Invocation { get; init; }
    [JsonPropertyName("revision")] public string? Revision { get; init; }
    [JsonPropertyName("runtime")] public string? Runtime { get; init; }
    [JsonPropertyName("os")] public string? Os { get; init; }
    [JsonPropertyName("arch")] public string? Arch { get; init; }
    [JsonPropertyName("profile")] public string? Profile { get; init; }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = false,
        UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        WriteIndented = true,
    };

    /// <summary>Serializes the report with exact signed-64-bit JSON integers.</summary>
    public string ToJson() => JsonSerializer.Serialize(this, JsonOptions);

    /// <summary>Parses, validates, and recomputes a report before returning it.</summary>
    public static Report FromJson(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        if (Encoding.UTF8.GetByteCount(json) > Microbenchmark.MaximumJsonBytes)
            throw new ArgumentException("report exceeds the 4 MiB JSON limit", nameof(json));

        using var document = JsonDocument.Parse(json, new JsonDocumentOptions { MaxDepth = Microbenchmark.MaximumJsonDepth });
        ValidateJsonTree(document.RootElement);
        RejectDuplicateProperties(document.RootElement);
        RejectNullMetadataProperties(document.RootElement);
        var report = JsonSerializer.Deserialize<Report>(json, JsonOptions)
            ?? throw new ArgumentException("report JSON must contain an object", nameof(json));
        report.ValidateReplay();
        return report;
    }

    private void ValidateReplay()
    {
        if (Config is null) throw new ArgumentException("report config is missing");
        Config.Validate();
        if (BatchSize <= 0 || BatchSize > Config.MaxBatch)
            throw new ArgumentException("batch_size must be positive and not exceed max_batch");
        if (Pairs is null || Pairs.Count != Config.Pairs)
            throw new ArgumentException("pair count does not match config");

        var baselineFirst = 0;
        foreach (var pair in Pairs)
        {
            if (pair is null || pair.BaselineNs <= 0 || pair.CandidateNs <= 0)
                throw new ArgumentException("pair durations must be positive signed 64-bit integers");
            if (pair.Order == "baseline-first") baselineFirst++;
            else if (pair.Order != "candidate-first") throw new ArgumentException("invalid pair order");
        }
        if (baselineFirst != (Config.Pairs + 1) / 2)
            throw new ArgumentException("pair order is not balanced");

        ValidateMetadata();
        var computed = Microbenchmark.Analyze(Pairs, Config.ConfidencePercent, Config.MinimumFactor);
        if (Analysis is null || !double.IsFinite(Analysis.Ratio) || Analysis.Ratio != computed.Ratio ||
            Analysis.Verdict != computed.Verdict || !IntervalsEqual(Analysis.Interval, computed.Interval))
            throw new ArgumentException("reported analysis disagrees with raw pairs");
    }

    private void ValidateMetadata()
    {
        foreach (var text in new[] { Workload, Revision, Runtime, Os, Arch, Profile })
        {
            if (text is not null && text.Length == 0) throw new ArgumentException("metadata strings must not be empty");
            if (text is not null && text.Contains('\0')) throw new ArgumentException("metadata strings must not contain NUL");
        }
        if (Workload is not null && !IsValidWorkload(Workload))
            throw new ArgumentException("workload must contain a non-ASCII-whitespace character and no NUL");
        if (Invocation is not null && Invocation.Any(argument => argument is null || argument.Contains('\0')))
            throw new ArgumentException("invocation entries must be strings without NUL");
        var anyStandardMetadata = Workload is not null || Invocation is not null || Revision is not null ||
            Runtime is not null || Os is not null || Arch is not null;
        var hasAllStandardMetadata = Workload is not null && Invocation is not null && Revision is not null &&
            Runtime is not null && Os is not null && Arch is not null;
        if (anyStandardMetadata && !hasAllStandardMetadata)
            throw new ArgumentException("standard metadata fields must be provided together");
        if (Profile is not null && !hasAllStandardMetadata)
            throw new ArgumentException("profile requires all standard metadata fields");
    }

    private static bool IsValidWorkload(string workload) =>
        !workload.Contains('\0') && workload.Any(character =>
            character != ' ' && (character < '\t' || character > '\r'));

    private static void RejectNullMetadataProperties(JsonElement root)
    {
        if (root.ValueKind != JsonValueKind.Object) return;
        foreach (var property in root.EnumerateObject())
        {
            if (property.Name is "workload" or "invocation" or "revision" or "runtime" or "os" or "arch" or "profile" &&
                property.Value.ValueKind == JsonValueKind.Null)
                throw new ArgumentException($"metadata property '{property.Name}' must not be null");
        }
    }

    private static bool IntervalsEqual(double[]? left, double[]? right) =>
        left is null ? right is null : right is not null && left.SequenceEqual(right);

    private static void ValidateJsonTree(JsonElement value, ref int nodes)
    {
        nodes++;
        if (nodes > Microbenchmark.MaximumJsonNodes)
            throw new ArgumentException("report exceeds the 65,536 JSON node limit");
        if (value.ValueKind == JsonValueKind.Object)
            foreach (var property in value.EnumerateObject()) ValidateJsonTree(property.Value, ref nodes);
        else if (value.ValueKind == JsonValueKind.Array)
            foreach (var item in value.EnumerateArray()) ValidateJsonTree(item, ref nodes);
    }

    private static void ValidateJsonTree(JsonElement value)
    {
        var nodes = 0;
        ValidateJsonTree(value, ref nodes);
    }

    private static void RejectDuplicateProperties(JsonElement value)
    {
        if (value.ValueKind == JsonValueKind.Object)
        {
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (var property in value.EnumerateObject())
            {
                if (!names.Add(property.Name)) throw new ArgumentException($"duplicate JSON property '{property.Name}'");
                RejectDuplicateProperties(property.Value);
            }
        }
        else if (value.ValueKind == JsonValueKind.Array)
        {
            foreach (var item in value.EnumerateArray()) RejectDuplicateProperties(item);
        }
    }
}

/// <summary>Paired collection and VER07 threshold analysis.</summary>
public static class Microbenchmark
{
    public const int MaximumPairs = 10_000;
    internal const int MaximumJsonBytes = 4 * 1024 * 1024;
    internal const int MaximumJsonDepth = 128;
    internal const int MaximumJsonNodes = 65_536;
    public const string BaselineFirst = "baseline-first";
    public const string CandidateFirst = "candidate-first";

    /// <summary>Collects a complete paired run, returning no partial report on failure.</summary>
    public static Report Collect(
        Config config,
        long expectedChecksum,
        Func<long, long> baseline,
        Func<long, long> candidate,
        Func<long> clock)
    {
        ArgumentNullException.ThrowIfNull(config);
        ArgumentNullException.ThrowIfNull(baseline);
        ArgumentNullException.ThrowIfNull(candidate);
        ArgumentNullException.ThrowIfNull(clock);
        config.Validate();
        VerifyOne(baseline, expectedChecksum);
        VerifyOne(candidate, expectedChecksum);

        long? previousTick = null;
        long ReadClock()
        {
            var tick = clock();
            if (tick < 0 || (previousTick.HasValue && tick < previousTick.Value))
                throw new InvalidOperationException("clock ticks must be nonnegative and nonregressing");
            previousTick = tick;
            return tick;
        }

        long Timed(Func<long, long> work, long size, bool calibration)
        {
            var expected = checked(expectedChecksum * size);
            var started = ReadClock();
            var actual = work(size);
            var finished = ReadClock();
            if (actual != expected) throw new InvalidOperationException("batch checksum mismatch");
            var duration = finished - started;
            if (duration < 0 || (!calibration && duration == 0))
                throw new InvalidOperationException("measured duration must be positive");
            return duration;
        }

        long batch = 1;
        while (true)
        {
            var baselineNs = Timed(baseline, batch, calibration: true);
            var candidateNs = Timed(candidate, batch, calibration: true);
            if (baselineNs >= config.TargetNs && candidateNs >= config.TargetNs) break;
            if (batch >= config.MaxBatch) throw new InvalidOperationException("calibration limit exhausted");
            batch = batch > config.MaxBatch / 2 ? config.MaxBatch : batch * 2;
        }

        var pairs = new List<Pair>(config.Pairs);
        foreach (var order in Schedule(config.Pairs, config.Seed))
        {
            long baselineNs;
            long candidateNs;
            if (order == BaselineFirst)
            {
                baselineNs = Timed(baseline, batch, calibration: false);
                candidateNs = Timed(candidate, batch, calibration: false);
            }
            else
            {
                candidateNs = Timed(candidate, batch, calibration: false);
                baselineNs = Timed(baseline, batch, calibration: false);
            }
            pairs.Add(new Pair { BaselineNs = baselineNs, CandidateNs = candidateNs, Order = order });
        }
        return new Report
        {
            Config = config,
            BatchSize = batch,
            Pairs = pairs,
            Analysis = Analyze(pairs, config.ConfidencePercent, config.MinimumFactor),
        };
    }

    /// <summary>Analyzes previously collected positive signed-64-bit pairs.</summary>
    public static Analysis Analyze(IReadOnlyList<Pair> pairs, int confidencePercent = 95, double minimumFactor = 1.05)
    {
        ArgumentNullException.ThrowIfNull(pairs);
        if (pairs.Count == 0) throw new ArgumentException("pairs must not be empty", nameof(pairs));
        if (pairs.Count > MaximumPairs)
            throw new ArgumentOutOfRangeException(nameof(pairs), "pairs must be from 1 through 10000");
        if (confidencePercent is < 1 or > 99) throw new ArgumentOutOfRangeException(nameof(confidencePercent));
        if (!double.IsFinite(minimumFactor) || minimumFactor <= 1.0)
            throw new ArgumentOutOfRangeException(nameof(minimumFactor));

        var ratios = new double[pairs.Count];
        for (var i = 0; i < pairs.Count; i++)
        {
            var pair = pairs[i] ?? throw new ArgumentException($"pair {i} is null", nameof(pairs));
            if (pair.BaselineNs <= 0 || pair.CandidateNs <= 0)
                throw new ArgumentException($"pair {i} durations must be positive", nameof(pairs));
            if (pair.Order is not (BaselineFirst or CandidateFirst))
                throw new ArgumentException($"pair {i} order is invalid", nameof(pairs));
            ratios[i] = (double)pair.CandidateNs / (double)pair.BaselineNs;
        }

        var ratio = CodingAdventures.RankStatistics.RankStatistics.Median(ratios);
        var interval = CodingAdventures.RankStatistics.RankStatistics.MedianInterval(ratios, confidencePercent);
        var verdict = "inconclusive";
        if (interval is not null)
        {
            var fastBoundary = 1.0 / minimumFactor;
            verdict = interval.Value.High < fastBoundary ? "faster"
                : interval.Value.Low > minimumFactor ? "slower"
                : interval.Value.Low >= fastBoundary && interval.Value.High <= minimumFactor ? "equivalent"
                : "inconclusive";
        }
        return new Analysis
        {
            Ratio = ratio,
            Interval = interval is null ? null : [interval.Value.Low, interval.Value.High],
            Verdict = verdict,
        };
    }

    private static void VerifyOne(Func<long, long> work, long expected)
    {
        if (work(1) != expected) throw new InvalidOperationException("one-operation checksum mismatch");
    }

    private static IReadOnlyList<string> Schedule(int pairs, long seed)
    {
        var orders = Enumerable.Repeat(BaselineFirst, (pairs + 1) / 2)
            .Concat(Enumerable.Repeat(CandidateFirst, pairs / 2)).ToArray();
        var random = new SplitMix64(unchecked((ulong)seed));
        for (var i = orders.Length - 1; i > 0; i--)
        {
            var j = (int)random.Bounded((ulong)i + 1);
            (orders[i], orders[j]) = (orders[j], orders[i]);
        }
        return orders;
    }

    private sealed class SplitMix64(ulong seed)
    {
        private ulong _state = seed;
        public ulong Next()
        {
            _state = unchecked(_state + 0x9E3779B97F4A7C15UL);
            var value = _state;
            value = unchecked((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9UL);
            value = unchecked((value ^ (value >> 27)) * 0x94D049BB133111EBUL);
            return value ^ (value >> 31);
        }
        public ulong Bounded(ulong bound)
        {
            var threshold = unchecked(0UL - bound) % bound;
            while (true)
            {
                var value = Next();
                if (value >= threshold) return value % bound;
            }
        }
    }
}
