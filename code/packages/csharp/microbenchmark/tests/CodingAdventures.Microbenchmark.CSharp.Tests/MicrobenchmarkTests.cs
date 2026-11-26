using System.Text.Json;
using System.Text.RegularExpressions;
using CodingAdventures.Microbenchmark;

namespace CodingAdventures.Microbenchmark.CSharp.Tests;

public sealed class MicrobenchmarkTests
{
    [Fact]
    public void ArrayStrategiesReadTheSuppliedMutableArray()
    {
        var values = new[] { 91, -7, 204, 0 };
        Assert.Equal(288L, ArrayWorkload.IndexedBatch(values, 1));
        Assert.Equal(288L, ArrayWorkload.IterationBatch(values, 1));
        Assert.Equal(0L, ArrayWorkload.IndexedBatch(values, 0));
        Assert.Equal(0L, ArrayWorkload.IterationBatch(values, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => ArrayWorkload.IndexedBatch(values, -1));
        Assert.Throws<ArgumentOutOfRangeException>(() => ArrayWorkload.IterationBatch(values, -1));
        values[0] = 2;
        Assert.Equal(199L, ArrayWorkload.IndexedBatch(values, 1));
        Assert.Equal(199L, ArrayWorkload.IterationBatch(values, 1));
    }

    [Fact]
    public void CliOutputPropagatesBrokenWriterFailures()
    {
        using var writer = new ThrowingWriter();
        Assert.Throws<IOException>(() => CliOutput.WriteAndFlush(writer, "complete report"));
    }

    [Fact]
    public void FixedArrayRunProducesPortableMetadataAndRealClockTicks()
    {
        long tick = 0;
        var report = ArrayWorkload.Run(
            new Config { Pairs = 1, TargetNs = 1, MaxBatch = 1 },
            () => tick++, ["array-microbenchmark", "--pairs", "1"], "test-revision", "debug");
        Assert.Equal(ArrayWorkload.Checksum, ArrayWorkload.IndexedBatch(ArrayWorkload.CreateArray(), 1));
        Assert.Equal(ArrayWorkload.Checksum, ArrayWorkload.IterationBatch(ArrayWorkload.CreateArray(), 1));
        Assert.Equal(ArrayWorkload.WorkloadId, report.Workload);
        Assert.Equal("test-revision", report.Revision);
        var clock = ArrayWorkload.CreateClock();
        Assert.True(clock() >= 0);
        Assert.True(clock() >= 0);
    }

    [Fact]
    public void CliFallbackRevisionProducesReplayableReportAndBlankOverrideFails()
    {
        var oldGitRevision = Environment.GetEnvironmentVariable("GIT_REVISION");
        var oldGithubSha = Environment.GetEnvironmentVariable("GITHUB_SHA");
        var oldOut = Console.Out;
        var oldError = Console.Error;
        try
        {
            Environment.SetEnvironmentVariable("GIT_REVISION", " ");
            Environment.SetEnvironmentVariable("GITHUB_SHA", "ci-revision");
            var output = new StringWriter();
            Console.SetOut(output);
            Console.SetError(new StringWriter());
            var status = ProgramEntry.Run(["--pairs", "1", "--confidence", "95", "--factor", "1.05", "--seed", "-3", "--target-ns", "1", "--max-batch", "1"]);
            Assert.Equal(0, status);
            var report = Report.FromJson(output.ToString());
            Assert.Equal("ci-revision", report.Revision);
            Assert.Equal(ArrayWorkload.WorkloadId, report.Workload);

            var explicitOutput = new StringWriter();
            Console.SetOut(explicitOutput);
            Assert.Equal(0, ProgramEntry.Run(["--pairs", "1", "--target-ns", "1", "--max-batch", "1", "--revision", "explicit-revision"]));
            Assert.Equal("explicit-revision", Report.FromJson(explicitOutput.ToString()).Revision);

            var blankOutput = new StringWriter();
            Console.SetOut(blankOutput);
            Assert.Equal(1, ProgramEntry.Run(["--pairs", "1", "--target-ns", "1", "--max-batch", "1", "--revision", " "]));
            Assert.Empty(blankOutput.ToString());

            var helpOutput = new StringWriter();
            Console.SetOut(helpOutput);
            Assert.Equal(0, ProgramEntry.Run(["--help"]));
            Assert.Contains("Usage:", helpOutput.ToString());

            var invalidOutput = new StringWriter();
            Console.SetOut(invalidOutput);
            Assert.Equal(1, ProgramEntry.Run(["--unknown", "value"]));
            Assert.Empty(invalidOutput.ToString());

            using var brokenWriter = new ThrowingWriter();
            Console.SetOut(brokenWriter);
            Assert.Equal(1, ProgramEntry.Run(["--help"]));
        }
        finally
        {
            Console.SetOut(oldOut);
            Console.SetError(oldError);
            Environment.SetEnvironmentVariable("GIT_REVISION", oldGitRevision);
            Environment.SetEnvironmentVariable("GITHUB_SHA", oldGithubSha);
        }
    }

    private sealed class ThrowingWriter : StringWriter
    {
        public override void Flush() => throw new IOException("closed output");
    }

    [Fact]
    public void CollectorWarmsCalibratesPairsAndReplaysDeterministically()
    {
        var config = new Config { Pairs = 7, TargetNs = 1, MaxBatch = 1, Seed = -71 };
        var events = new List<string>();
        long baseline(long count) { events.Add($"b{count}"); return 11 * count; }
        long candidate(long count) { events.Add($"c{count}"); return 11 * count; }
        long tick = 0;
        var report = Microbenchmark.Collect(config, 11, baseline, candidate, () => tick++);

        Assert.Equal(7, report.Pairs.Count);
        Assert.Equal(9, events.Count(x => x[0] == 'b'));
        Assert.Equal(9, events.Count(x => x[0] == 'c'));
        Assert.Equal(4, report.Pairs.Count(x => x.Order == Microbenchmark.BaselineFirst));
        Assert.Equal(Microbenchmark.Analyze(report.Pairs, 95, 1.05).Verdict, report.Analysis.Verdict);

        for (var i = 0; i < report.Pairs.Count; i++)
        {
            var first = events[4 + 2 * i][0] == 'b' ? Microbenchmark.BaselineFirst : Microbenchmark.CandidateFirst;
            Assert.Equal(first, report.Pairs[i].Order);
        }

        long replayTick = 0;
        var replay = Microbenchmark.Collect(config, 11, baseline, candidate, () => replayTick++);
        Assert.Equal(report.Pairs.Select(x => x.Order), replay.Pairs.Select(x => x.Order));
        Assert.Equal(1L, report.BatchSize);
    }

    [Fact]
    public void CalibrationDoublesToCommonUsableSize()
    {
        var config = new Config { Pairs = 1, TargetNs = 4, MaxBatch = 4 };
        static long Identity(long count) => count;
        var ticks = new Queue<long>([0, 1, 2, 3, 4, 6, 7, 9, 10, 14, 15, 19, 20, 24, 25, 29]);
        var report = Microbenchmark.Collect(config, 1, Identity, Identity, () => ticks.Dequeue());
        Assert.Equal(4L, report.BatchSize);
    }

    [Fact]
    public void CollectorRejectsChecksumClockZeroDurationOverflowAndExhaustion()
    {
        var config = new Config { Pairs = 1, TargetNs = 1, MaxBatch = 1 };
        static long Identity(long count) => count;
        long tick = 0;
        Assert.Throws<InvalidOperationException>(() => Microbenchmark.Collect(config, 1, _ => 0, Identity, () => tick++));
        Assert.Throws<InvalidOperationException>(() => Microbenchmark.Collect(config, 1, _ => throw new InvalidOperationException("callback"), Identity, () => tick++));
        Assert.Throws<InvalidOperationException>(() => Microbenchmark.Collect(config, 1, Identity, Identity, () => 5));
        var zeroMeasured = new Queue<long>([0, 1, 2, 3, 3, 3]);
        Assert.Throws<InvalidOperationException>(() => Microbenchmark.Collect(config, 1, Identity, Identity, () => zeroMeasured.Dequeue()));
        var overflowConfig = config with { MaxBatch = 2, TargetNs = 2 };
        static long Huge(long count) => count == 1 ? long.MaxValue : 0;
        Assert.Throws<OverflowException>(() => Microbenchmark.Collect(overflowConfig, long.MaxValue, Huge, Huge, () => tick++));
        Assert.Throws<InvalidOperationException>(() => Microbenchmark.Collect(config with { TargetNs = 2 }, 1, Identity, Identity, () => 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => Microbenchmark.Collect(config with { Pairs = 0 }, 1, Identity, Identity, () => tick++));
    }

    [Fact]
    public void AnalyzerUsesExactVerdictBoundariesAndRejectsInvalidPairs()
    {
        Pair Make(long baseline, long candidate) => new() { BaselineNs = baseline, CandidateNs = candidate, Order = Microbenchmark.BaselineFirst };
        Assert.Equal("faster", Microbenchmark.Analyze(Enumerable.Repeat(Make(100, 80), 6).ToArray(), 95, 1.0625).Verdict);
        Assert.Equal("slower", Microbenchmark.Analyze(Enumerable.Repeat(Make(100, 120), 6).ToArray(), 95, 1.0625).Verdict);
        Assert.Equal("inconclusive", Microbenchmark.Analyze([Make(long.MaxValue, long.MaxValue)], 95, 1.05).Verdict);
        Assert.Throws<ArgumentException>(() => Microbenchmark.Analyze(Array.Empty<Pair>()));
        Assert.Throws<ArgumentException>(() => Microbenchmark.Analyze([Make(0, 1)]));
        Assert.Throws<ArgumentException>(() => Microbenchmark.Analyze([new Pair { BaselineNs = 1, CandidateNs = 1, Order = "x" }]));
    }

    [Fact]
    public void JsonReplayPreservesI64RejectsUnknownDuplicateUnbalancedAndForgedReports()
    {
        var config = new Config { Pairs = 3, TargetNs = 1, MaxBatch = 1 };
        long tick = 0;
        var report = Microbenchmark.Collect(config, 1, x => x, x => x, () => tick++);
        report = new Report
        {
            Config = report.Config,
            BatchSize = report.BatchSize,
            Pairs = report.Pairs,
            Analysis = report.Analysis,
            Workload = "indexed-array-sum-4096",
            Invocation = ["array-microbenchmark", "--pairs", "3"],
            Revision = "abc123",
            Runtime = ".NET 9",
            Os = "windows",
            Arch = "x64",
            Profile = "release",
        };
        var json = report.ToJson();
        Assert.Equal(3, Report.FromJson(json).Pairs.Count);
        Assert.Contains("\"profile\": \"release\"", json);
        string WithWorkload(string value) => json.Replace(
            "\"workload\": \"indexed-array-sum-4096\"",
            "\"workload\": " + JsonSerializer.Serialize(value),
            StringComparison.Ordinal);
        Assert.Equal("dependent-link-order", Report.FromJson(WithWorkload("dependent-link-order")).Workload);
        Assert.Equal("\u2003", Report.FromJson(WithWorkload("\u2003")).Workload);
        Assert.Equal("\U0001F680", Report.FromJson(WithWorkload("\U0001F680")).Workload);
        Assert.Equal("\uFFFD", Report.FromJson(WithWorkload("\uFFFD")).Workload);
        Assert.Equal("literal\\uD800", Report.FromJson(WithWorkload("literal\\uD800")).Workload);
        foreach (var invalidWorkload in new[] { "", " \t\n\r\f" + (char)11, "\0", "dependent\0link" })
            Assert.Throws<ArgumentException>(() => Report.FromJson(WithWorkload(invalidWorkload)));
        foreach (var surrogate in new[] { "\\uD800", "\\uDFFF" })
            Assert.Throws<JsonException>(() => Report.FromJson(json.Replace("indexed-array-sum-4096", surrogate, StringComparison.Ordinal)));
        Assert.Throws<ArgumentException>(() => Report.FromJson(json.Replace(
            "\"workload\": \"indexed-array-sum-4096\"", "\"workload\": null", StringComparison.Ordinal)));
        Assert.Throws<JsonException>(() => Report.FromJson(json.Replace(
            "\"workload\": \"indexed-array-sum-4096\"", "\"workload\": 7", StringComparison.Ordinal)));

        var partialMetadata = new Report
        {
            Config = report.Config,
            BatchSize = report.BatchSize,
            Pairs = report.Pairs,
            Analysis = report.Analysis,
            Workload = ArrayWorkload.WorkloadId,
        };
        Assert.Throws<ArgumentException>(() => Report.FromJson(partialMetadata.ToJson()));
        var profileOnly = new Report
        {
            Config = report.Config,
            BatchSize = report.BatchSize,
            Pairs = report.Pairs,
            Analysis = report.Analysis,
            Profile = "release",
        };
        Assert.Throws<ArgumentException>(() => Report.FromJson(profileOnly.ToJson()));

        var maxReport = """
            {"config":{"pairs":1,"confidence_percent":95,"minimum_factor":1.05,"seed":0,"target_ns":1,"max_batch":9223372036854775807},"batch_size":1,"pairs":[{"baseline_ns":9223372036854775807,"candidate_ns":9223372036854775807,"order":"baseline-first"}],"analysis":{"ratio":1,"interval":null,"verdict":"inconclusive"}}
            """;
        var parsedMax = Report.FromJson(maxReport);
        Assert.Equal(long.MaxValue, parsedMax.Pairs[0].BaselineNs);
        Assert.Contains("9223372036854775807", parsedMax.ToJson());

        Assert.Throws<ArgumentException>(() => Report.FromJson("{\"a\":1,\"a\":2}"));
        Assert.Throws<JsonException>(() => Report.FromJson(json.Replace("\"analysis\":", "\"unknown\":true,\"analysis\":", StringComparison.Ordinal)));
        Assert.Throws<ArgumentException>(() => Report.FromJson(json.Replace("baseline-first", "candidate-first", StringComparison.Ordinal)));
        Assert.Throws<ArgumentException>(() => Report.FromJson(json.Replace("\"ratio\": 1", "\"ratio\": 2", StringComparison.Ordinal)));
        Assert.Throws<JsonException>(() => Report.FromJson(maxReport.Replace("9223372036854775807", "9223372036854775808", StringComparison.Ordinal)));

        // Metadata is absent as a complete set or present with non-null values;
        // nullable DTO properties must not turn explicit JSON nulls into absence.
        var metadata = new[] { "workload", "invocation", "revision", "runtime", "os", "arch" };
        foreach (var field in metadata)
        {
            var explicitNull = field == "invocation"
                ? Regex.Replace(json, "\\\"invocation\\\"\\s*:\\s*\\[[^\\]]*\\]", "\"invocation\": null")
                : field switch
                {
                    "workload" => json.Replace("\"workload\": \"indexed-array-sum-4096\"", "\"workload\": null", StringComparison.Ordinal),
                    "revision" => json.Replace("\"revision\": \"abc123\"", "\"revision\": null", StringComparison.Ordinal),
                    "runtime" => json.Replace("\"runtime\": \".NET 9\"", "\"runtime\": null", StringComparison.Ordinal),
                    "os" => json.Replace("\"os\": \"windows\"", "\"os\": null", StringComparison.Ordinal),
                    "arch" => json.Replace("\"arch\": \"x64\"", "\"arch\": null", StringComparison.Ordinal),
                    _ => throw new InvalidOperationException(field),
                };
            Assert.NotEqual(json, explicitNull);
            Assert.Throws<ArgumentException>(() => Report.FromJson(explicitNull));
        }
        Assert.Throws<ArgumentException>(() => Report.FromJson(json.Replace("\"profile\": \"release\"", "\"profile\": null", StringComparison.Ordinal)));
        Assert.Throws<ArgumentException>(() => Report.FromJson(json.Replace("\"analysis\":", "\"invocation\": [null],\n  \"analysis\":", StringComparison.Ordinal)));
        Assert.DoesNotContain("\"workload\": null", Report.FromJson(maxReport).ToJson());
        Assert.Null(Report.FromJson(maxReport).Analysis.Interval);
    }

    [Fact]
    public void SharedFixtureCopyMatchesCanonicalAndAnalysis()
    {
        var localPath = Path.Combine(AppContext.BaseDirectory, "testdata", "microbenchmark-pairs-v1.json");
        var local = File.ReadAllBytes(localPath);
        string? canonical = null;
        for (var current = new DirectoryInfo(AppContext.BaseDirectory); current is not null; current = current.Parent)
        {
            var candidate = Path.Combine(current.FullName, "code", "specs", "fixtures", "microbenchmark-pairs-v1.json");
            if (File.Exists(candidate)) { canonical = candidate; break; }
        }
        if (canonical is not null)
            Assert.Equal(File.ReadAllBytes(canonical), local);
        using var doc = JsonDocument.Parse(local);
        var root = doc.RootElement;
        var confidence = root.GetProperty("confidence_percent").GetInt32();
        var factor = root.GetProperty("minimum_factor").GetDouble();
        foreach (var item in root.GetProperty("cases").EnumerateArray())
        {
            var baseline = item.GetProperty("baseline_ns").GetInt64();
            var pairs = item.GetProperty("candidate_ns").EnumerateArray().Select((value, i) => new Pair
            {
                BaselineNs = baseline,
                CandidateNs = value.GetInt64(),
                Order = i % 2 == 0 ? Microbenchmark.BaselineFirst : Microbenchmark.CandidateFirst,
            }).ToArray();
            var result = Microbenchmark.Analyze(pairs, confidence, factor);
            Assert.Equal(item.GetProperty("ratio").GetDouble(), result.Ratio);
            Assert.Equal(item.GetProperty("verdict").GetString(), result.Verdict);
        }
    }

    [Fact]
    public void GeneratedObservationsKeepIntervalsInsideSamples()
    {
        var random = new Random(0x4d454449);
        for (var attempt = 0; attempt < 512; attempt++)
        {
            var pairs = Enumerable.Range(0, random.Next(1, 40)).Select(i => new Pair
            {
                BaselineNs = 100_000 + random.Next(1, 100_000),
                CandidateNs = 100_000 + random.Next(1, 100_000),
                Order = i % 2 == 0 ? Microbenchmark.BaselineFirst : Microbenchmark.CandidateFirst,
            }).ToArray();
            var result = Microbenchmark.Analyze(pairs, random.Next(1, 100), 1.05);
            if (result.Interval is not null)
            {
                var ratios = pairs.Select(x => (double)x.CandidateNs / x.BaselineNs).ToArray();
                Assert.Contains(result.Interval[0], ratios);
                Assert.Contains(result.Interval[1], ratios);
            }
        }
    }

    [Fact]
    public void ReplaysMaximumReportAndDeterministicJsonMutationsSafely()
    {
        var config = new Config { Pairs = 10_000, TargetNs = 1, MaxBatch = 1 };
        var pairs = Enumerable.Range(0, config.Pairs).Select(i => new Pair
        {
            BaselineNs = 100 + i % 7,
            CandidateNs = 99 + i % 9,
            Order = i % 2 == 0 ? Microbenchmark.BaselineFirst : Microbenchmark.CandidateFirst,
        }).ToList();
        var report = new Report
        {
            Config = config,
            BatchSize = 1,
            Pairs = pairs,
            Analysis = Microbenchmark.Analyze(pairs, config.ConfidencePercent, config.MinimumFactor),
        };
        var json = report.ToJson();
        Assert.Equal(10_000, Report.FromJson(json).Pairs.Count);

        var random = new Random(0x4a534f4e);
        for (var attempt = 0; attempt < 256; attempt++)
        {
            var chars = json.ToCharArray();
            var position = random.Next(chars.Length);
            chars[position] = chars[position] switch { '0' => '1', '1' => '0', '{' => '[', '[' => '{', _ => 'x' };
            try
            {
                var parsed = Report.FromJson(new string(chars));
                Assert.Equal(parsed.ToJson(), Report.FromJson(parsed.ToJson()).ToJson());
            }
            catch (Exception error) when (error is ArgumentException or JsonException or InvalidOperationException or OverflowException) { }
        }
    }
}
