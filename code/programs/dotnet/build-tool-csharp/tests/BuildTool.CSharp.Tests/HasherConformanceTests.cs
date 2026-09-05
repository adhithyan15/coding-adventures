namespace CodingAdventures.BuildTool.CSharp.Tests;

using System.Text;
using System.Text.Json;

public sealed class HasherConformanceTests
{
    private static readonly string RepositoryRoot = FindRepositoryRoot();
    private static readonly string FixtureDirectory = Path.Combine(
        RepositoryRoot,
        "code",
        "specs",
        "fixtures",
        "build-tool-v1");

    [Fact]
    public void ProductionRegistriesExactlyMatchCheckedAuthorities()
    {
        var checkedLanguageJson = File.ReadAllText(Path.Combine(
            FixtureDirectory,
            "language-source-input-registry.json"));
        var checkedBoundaryJson = File.ReadAllText(Path.Combine(
            FixtureDirectory,
            "repository-source-input-boundary.json"));
        using var checkedLanguage = JsonDocument.Parse(checkedLanguageJson);
        using var productionLanguage = JsonDocument.Parse(Hasher.LanguageSourceInputRegistryJson);
        using var checkedBoundary = JsonDocument.Parse(checkedBoundaryJson);
        using var productionBoundary = JsonDocument.Parse(Hasher.RepositorySourceInputBoundaryJson);

        Assert.True(JsonElement.DeepEquals(checkedLanguage.RootElement, productionLanguage.RootElement));
        Assert.True(JsonElement.DeepEquals(checkedBoundary.RootElement, productionBoundary.RootElement));
        Assert.Equal(23, checkedLanguage.RootElement.GetProperty("languages").GetArrayLength());
        Assert.Equal(18, checkedBoundary.RootElement.GetProperty("boundaries").GetArrayLength());
        Assert.Equal(
            "f49bfe8c7c9c0fb9b534ecc9ca4a614f3684abe32bdb0edac82d99bdc806fb70",
            Hasher.LanguageSourceInputRegistryDigest);
        Assert.Equal(
            "963cc4090e165752fd3a62921b699dfff8f0677b49d7236812398a8abed0a25f",
            Hasher.RepositorySourceInputBoundaryDigest);
        Assert.Equal(
            Hasher.LanguageSourceInputRegistryDigest,
            Hasher.CanonicalLanguageSourceInputRegistryDigest(checkedLanguageJson));
        Assert.Equal(
            Hasher.RepositorySourceInputBoundaryDigest,
            Hasher.CanonicalRepositorySourceInputBoundaryDigest(checkedBoundaryJson));
        Assert.Equal(
            Hasher.LanguageSourceInputRegistryDigest,
            checkedBoundary.RootElement.GetProperty("language_source_input_registry_sha256").GetString());
    }

    [Fact]
    public void EveryNeutralSourceCollectionFixtureUsesTheProductionSelector()
    {
        var fixturePaths = Directory
            .GetFiles(Path.Combine(FixtureDirectory, "cases"), "source-collection-*.json")
            .OrderBy(path => path, StringComparer.Ordinal)
            .ToArray();
        Assert.Equal(13, fixturePaths.Length);

        foreach (var fixturePath in fixturePaths)
        {
            using var fixture = JsonDocument.Parse(File.ReadAllText(fixturePath));
            var options = fixture.RootElement.GetProperty("input").GetProperty("options");
            var mode = options.GetProperty("mode").GetString()!;
            var digestProperty = mode == "repository_boundary" ? "boundary_sha256" : "registry_sha256";
            var declaredSources = options.TryGetProperty("declared_srcs", out var declared)
                ? declared.EnumerateArray().Select(value => value.GetString()!).ToArray()
                : [];
            var candidates = options.GetProperty("candidates").EnumerateArray()
                .Select(candidate => new SourceCollectionCandidate(
                    candidate.GetProperty("path").GetString()!,
                    candidate.GetProperty("kind").GetString()!,
                    candidate.TryGetProperty("tracked", out var tracked) && tracked.GetBoolean(),
                    candidate.TryGetProperty("content_hex", out var content)
                        ? Convert.FromHexString(content.GetString()!)
                        : []))
                .ToArray();
            var request = new SourceCollectionRequest(
                options.GetProperty("language").GetString()!,
                options.GetProperty("package_root").GetString()!,
                mode,
                options.GetProperty(digestProperty).GetString()!,
                declaredSources,
                candidates);

            var actual = Hasher.SelectSourceCandidates(request)
                .Select(file => $"{file.Path}\0{file.Digest}")
                .ToArray();
            var expected = fixture.RootElement.GetProperty("expected").GetProperty("result")
                .GetProperty("files").EnumerateArray()
                .Select(file => $"{file.GetProperty("path").GetString()}\0{file.GetProperty("digest").GetString()}")
                .ToArray();

            Assert.Equal(expected, actual);
        }
    }

    [Fact]
    public void HashingCacheFixturesUseTheHashingV1PackageDigest()
    {
        var fixturePaths = Directory
            .GetFiles(Path.Combine(FixtureDirectory, "cases"), "hashing-cache-*.json")
            .OrderBy(path => path, StringComparer.Ordinal)
            .ToArray();
        Assert.Equal(3, fixturePaths.Length);

        foreach (var fixturePath in fixturePaths)
        {
            using var fixture = JsonDocument.Parse(File.ReadAllText(fixturePath));
            var inputs = fixture.RootElement.GetProperty("workspace").GetProperty("files")
                .EnumerateArray()
                .Select(file => new PackageHashInput(
                    file.GetProperty("path").GetString()!,
                    file.TryGetProperty("content_hex", out var contentHex)
                        ? Convert.FromHexString(contentHex.GetString()!)
                        : Encoding.UTF8.GetBytes(file.GetProperty("content_utf8").GetString()!)))
                .ToArray();
            var expected = fixture.RootElement.GetProperty("expected").GetProperty("result")
                .GetProperty("package_digest").GetString();

            Assert.Equal(expected, Hasher.HashPackageInputs(inputs));
        }
    }

    [Fact]
    public void HashingV1IncludesPathsAndExactRawBytes()
    {
        var original = Hasher.HashPackageInputs(
            [new PackageHashInput("code/packages/csharp/demo/src/data.bin", [0x00, 0xff, 0x0d, 0x0a])]);
        var renamed = Hasher.HashPackageInputs(
            [new PackageHashInput("code/packages/csharp/demo/src/renamed.bin", [0x00, 0xff, 0x0d, 0x0a])]);
        var textNormalized = Hasher.HashPackageInputs(
            [new PackageHashInput("code/packages/csharp/demo/src/data.bin", [0x00, 0xff, 0x0a])]);

        Assert.NotEqual(original, renamed);
        Assert.NotEqual(original, textNormalized);
    }

    [Fact]
    public void PublicSnapshotHelpersFailClosedOnHostileLimitsAndPaths()
    {
        var limitError = Assert.Throws<SourceHashException>(() => Hasher.SelectSourceCandidates(
            new SourceCollectionRequest(
                "csharp",
                "code/packages/csharp/demo",
                "extension",
                Hasher.LanguageSourceInputRegistryDigest,
                [],
                [
                    new SourceCollectionCandidate("a.cs", "file", false, [0x61]),
                    new SourceCollectionCandidate("b.cs", "file", false, [0x62]),
                ]),
            new SourceHashLimits(1, 1, 1, 2)));
        Assert.Equal("SOURCE_HASH_LIMIT_EXCEEDED", limitError.Code);

        var pathError = Assert.Throws<SourceHashException>(() => Hasher.SelectSourceCandidates(
            new SourceCollectionRequest(
                "csharp",
                "code/packages/csharp/demo",
                "extension",
                Hasher.LanguageSourceInputRegistryDigest,
                [],
                [new SourceCollectionCandidate("src/e\u0301.cs", "file", false, [0x61])])));
        Assert.Equal("SOURCE_HASH_PATH_INVALID", pathError.Code);
        Assert.DoesNotContain("e\u0301.cs", pathError.Message, StringComparison.Ordinal);

        var languageError = Assert.Throws<SourceHashException>(() => Hasher.SelectSourceCandidates(
            new SourceCollectionRequest(
                "unknown",
                "code/packages/unknown/demo",
                "extension",
                Hasher.LanguageSourceInputRegistryDigest,
                [],
                [])));
        Assert.Equal("SOURCE_HASH_LANGUAGE_UNKNOWN", languageError.Code);
    }

    private static string FindRepositoryRoot()
    {
        foreach (var start in new[]
        {
            new DirectoryInfo(AppContext.BaseDirectory),
            new DirectoryInfo(Directory.GetCurrentDirectory()),
        })
        {
            for (var current = start; current is not null; current = current.Parent)
            {
                if (File.Exists(Path.Combine(
                    current.FullName,
                    "code",
                    "specs",
                    "fixtures",
                    "build-tool-v1",
                    "pure-domains.schema.json")))
                {
                    return current.FullName;
                }
            }
        }

        throw new DirectoryNotFoundException("Could not locate the coding-adventures repository root.");
    }
}
