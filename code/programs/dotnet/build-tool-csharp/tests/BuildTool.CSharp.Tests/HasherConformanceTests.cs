namespace CodingAdventures.BuildTool.CSharp.Tests;

using System.Security.Cryptography;
using System.Runtime.InteropServices;
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
            var includePaths = fixture.RootElement.GetProperty("input").GetProperty("options")
                .GetProperty("include_paths").EnumerateArray()
                .Select(path => path.GetString()!)
                .ToHashSet(StringComparer.Ordinal);
            var inputs = fixture.RootElement.GetProperty("workspace").GetProperty("files")
                .EnumerateArray()
                .Where(file => includePaths.Contains(file.GetProperty("path").GetString()!))
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

    [Theory]
    [InlineData("csharp", "demo.csproj", "src/main.cs", "tests/Test.csproj", "tests/data.csv", "tools/run-tests.sh")]
    [InlineData("fsharp", "demo.fsproj", "src/main.fs", "tests/Test.fsproj", "tests/data.csv", "tools/run-tests.sh")]
    [InlineData("dotnet", "demo.fsproj", "src/main.fsx", "tests/Test.csproj", null, null)]
    public void SharedDotnetSelectorsUseExactRootRecursiveAndScopedRoles(
        string language,
        string rootProject,
        string source,
        string scopedProject,
        string? scopedResource,
        string? exactTool)
    {
        var candidates = new List<SourceCollectionCandidate>
        {
            new("BUILD", "file", false, [0x61]),
            new("required_capabilities.json", "file", false, [0x61]),
            new("global.json", "file", false, [0x61]),
            new(rootProject, "file", false, [0x61]),
            new(source, "file", false, [0x61]),
            new("src/view.xaml", "file", false, [0x61]),
            new(scopedProject, "file", false, [0x61]),
            new("nested/should-not-be-root.csproj", "file", false, [0x61]),
            new("README.md", "file", false, [0x61]),
        };
        if (scopedResource is not null)
        {
            candidates.Add(new(scopedResource, "file", false, [0x61]));
        }
        if (exactTool is not null)
        {
            candidates.Add(new(exactTool, "file", false, [0x61]));
        }

        var actual = Hasher.SelectSourceCandidates(new SourceCollectionRequest(
            language,
            $"code/packages/{language}/demo",
            "extension",
            Hasher.LanguageSourceInputRegistryDigest,
            [],
            candidates)).Select(file => file.Path).ToArray();

        var expected = new[]
        {
            "BUILD",
            "global.json",
            rootProject,
            "required_capabilities.json",
            source,
            "src/view.xaml",
            scopedProject,
            scopedResource,
            exactTool,
        }.Where(path => path is not null).Cast<string>()
            .OrderBy(path => path, StringComparer.Ordinal)
            .ToArray();
        Assert.Equal(expected, actual);
        Assert.DoesNotContain("README.md", actual, StringComparer.Ordinal);
        Assert.DoesNotContain("nested/should-not-be-root.csproj", actual, StringComparer.Ordinal);
    }

    [Fact]
    public void RepositoryBoundaryDiffSelectsExactSharedDotnetConsumers()
    {
        var packages = new[]
        {
            new PackageSpec(
                "csharp/bitset-native",
                Path.Combine(RepositoryRoot, "code", "packages", "csharp", "bitset-native"),
                [],
                "csharp",
                []),
            new PackageSpec(
                "fsharp/bitset-native",
                Path.Combine(RepositoryRoot, "code", "packages", "fsharp", "bitset-native"),
                [],
                "fsharp",
                []),
        };

        var actual = GitDiff.MapFilesToPackages(
            ["code/packages/rust/Cargo.toml"],
            packages,
            RepositoryRoot);

        Assert.Equal(
            ["csharp/bitset-native", "fsharp/bitset-native"],
            actual.OrderBy(name => name, StringComparer.Ordinal));
    }

    [Fact]
    public void DeclaredSourceGlobsUseDeterministicCharacterClassesAndRejectFilePrefixes()
    {
        var selected = Hasher.SelectSourceCandidates(new SourceCollectionRequest(
            "csharp",
            "code/packages/csharp/demo",
            "declared_sources",
            Hasher.LanguageSourceInputRegistryDigest,
            ["src/*.[ch]"],
            [
                new SourceCollectionCandidate("src/demo.c", "file", false, [0x61]),
                new SourceCollectionCandidate("src/demo.h", "file", false, [0x62]),
                new SourceCollectionCandidate("src/demo.cs", "file", false, [0x63]),
            ]));

        Assert.Equal(["src/demo.c", "src/demo.h"], selected.Select(file => file.Path));

        var prefixError = Assert.Throws<SourceHashException>(() => Hasher.SelectSourceCandidates(
            new SourceCollectionRequest(
                "csharp",
                "code/packages/csharp/demo",
                "extension",
                Hasher.LanguageSourceInputRegistryDigest,
                [],
                [
                    new SourceCollectionCandidate("src", "file", false, [0x61]),
                    new SourceCollectionCandidate("src/demo.cs", "file", false, [0x62]),
                ])));
        Assert.Equal("SOURCE_HASH_PATH_COLLISION", prefixError.Code);
    }

    [Fact]
    public void DiscoveryCarriesStarlarkDeclaredSourcesIntoTheProductionPackage()
    {
        using var repository = TemporaryRepository.Create();
        var packageRoot = repository.PackageRoot("csharp", "demo");
        Directory.CreateDirectory(packageRoot);
        File.WriteAllText(Path.Combine(packageRoot, "BUILD"), """
            load("code/packages/starlark/library-rules/java_library.star", "java_library")

            _targets = [
                java_library(
                    name = "demo",
                    srcs = ["src/**/*.cs", "tests/**/*.cs"],
                    deps = [],
                ),
            ]
            """);

        var package = Assert.Single(Discovery.DiscoverPackages(Path.Combine(repository.Root, "code")));
        Assert.True(package.IsStarlark);
        Assert.Equal(["src/**/*.cs", "tests/**/*.cs"], package.DeclaredSources);

        var concatenated = Discovery.ReadSourceDeclaration("""
            load("code/packages/starlark/library-rules/java_library.star", "java_library")
            _targets = [java_library(name = "demo", srcs = ["src/*.[ch]"] + ["generated/[]].cs"])]
            """);
        Assert.True(concatenated.IsStarlark);
        Assert.Equal(["generated/[]].cs", "src/*.[ch]"], concatenated.DeclaredSources);

        Assert.Throws<InvalidDataException>(() => Discovery.ReadSourceDeclaration("""
            load("code/packages/starlark/library-rules/java_library.star", "java_library")
            _targets = [java_library(name = "demo", srcs = DECLARED_SOURCES)]
            """));

        Assert.Throws<InvalidDataException>(() => Discovery.ReadSourceDeclaration("""
            load("code/packages/starlark/library-rules/java_library.star", "java_library")
            _targets = [java_library(name = "demo", **kwargs)]
            """));

        Assert.Throws<InvalidDataException>(() => Discovery.ReadSourceDeclaration("""
            load("code/packages/starlark/library-rules/java_library.star", "java_library")
            _targets = [java_library(name = "demo", deps = [helper(srcs = ["decoy.cs"])])]
            """));

        Assert.Throws<InvalidDataException>(() => Discovery.ReadSourceDeclaration(
            "\"\"\"\n" +
            "load(\"code/packages/starlark/library-rules/java_library.star\", \"java_library\")\n" +
            "\"\"\"\n" +
            "_targets = [java_library(name = \"demo\", srcs = [\"demo.cs\"])]\n"));

        Assert.Throws<InvalidDataException>(() => Discovery.ReadSourceDeclaration("""
            obj.load("code/packages/starlark/library-rules/java_library.star", "java_library")
            _targets = [java_library(name = "demo", srcs = ["demo.cs"])]
            """));

        Assert.Throws<InvalidDataException>(() => Discovery.ReadSourceDeclaration("""
            load("code/packages/starlark/library-rules/java_library.star", "java_library")
            obj._targets = [java_library(name = "demo", srcs = ["demo.cs"])]
            """));

        Assert.Throws<InvalidDataException>(() => Discovery.ReadSourceDeclaration("""
            load("code/packages/starlark/library-rules/java_library.star", "java_library")
            def declare():
                _targets = [java_library(name = "demo", srcs = ["demo.cs"])]
            """));
    }

    [Fact]
    public void LiveCollectionCountsDirectoriesBeforeSorting()
    {
        using var repository = TemporaryRepository.Create();
        var packageRoot = repository.PackageRoot("csharp", "demo");
        Directory.CreateDirectory(packageRoot);
        Directory.CreateDirectory(Path.Combine(packageRoot, "first"));
        Directory.CreateDirectory(Path.Combine(packageRoot, "second"));

        var error = Assert.Throws<SourceHashException>(() => Hasher.CollectSourceFiles(
            new PackageSpec("csharp/demo", packageRoot, [], "csharp", []),
            new SourceHashLimits(1, 1, 1, 1)));
        Assert.Equal("SOURCE_HASH_LIMIT_EXCEEDED", error.Code);
    }

    [Fact]
    public void NativeMetadataLayoutsUseStableKernelAbis()
    {
        var layout = SecureSourceFileReader.NativeLayoutForTest();
        Assert.Equal(256, layout.LinuxStatxSize);
        Assert.Equal(28, layout.LinuxModeOffset);
        Assert.Equal(40, layout.LinuxSizeOffset);
        Assert.Equal(144, layout.MacStatSize);
    }

    [Fact]
    public void SecureDiscoveryRejectsHardlinkedAndOversizedBuildFiles()
    {
        using var repository = TemporaryRepository.Create();
        var packageRoot = repository.PackageRoot("csharp", "demo");
        Directory.CreateDirectory(packageRoot);
        var outside = Path.Combine(repository.Root, "outside-build");
        File.WriteAllText(outside, "echo outside\n");
        var buildFile = Path.Combine(packageRoot, "BUILD");
        Assert.True(TryCreateHardLink(buildFile, outside));
        Assert.Equal(
            "SOURCE_HASH_LINK_REJECTED",
            Assert.Throws<SourceHashException>(() =>
                Discovery.DiscoverPackages(Path.Combine(repository.Root, "code"))).Code);

        File.Delete(buildFile);
        File.WriteAllBytes(buildFile, new byte[(1024 * 1024) + 1]);
        Assert.Equal(
            "SOURCE_HASH_LIMIT_EXCEEDED",
            Assert.Throws<SourceHashException>(() =>
                Discovery.DiscoverPackages(Path.Combine(repository.Root, "code"))).Code);
    }

    [Fact]
    public void SecureReaderRejectsLinksHardlinksAndSamePathReplacement()
    {
        using var repository = TemporaryRepository.Create();
        var sourceRoot = Path.Combine(repository.Root, "source");
        Directory.CreateDirectory(sourceRoot);
        var target = Path.Combine(sourceRoot, "target.bin");
        File.WriteAllBytes(target, [0x61, 0x62, 0x63]);
        Assert.Equal(
            Convert.ToHexString(SHA256.HashData([0x61, 0x62, 0x63])).ToLowerInvariant(),
            Hasher.HashFile(target));

        var hardlink = Path.Combine(sourceRoot, "hardlink.bin");
        if (TryCreateHardLink(hardlink, target))
        {
            Assert.Equal(
                "SOURCE_HASH_LINK_REJECTED",
                Assert.Throws<SourceHashException>(() => Hasher.HashFile(hardlink)).Code);
            File.Delete(hardlink);
        }

        var link = Path.Combine(sourceRoot, "link.bin");
        try
        {
            File.CreateSymbolicLink(link, target);
            Assert.Equal(
                "SOURCE_HASH_LINK_REJECTED",
                Assert.Throws<SourceHashException>(() => Hasher.HashFile(link)).Code);
        }
        catch (Exception error) when (error is UnauthorizedAccessException or IOException)
        {
            // Windows hosts without developer-mode link permission cannot
            // construct this adversarial fixture; hardlink/state checks remain.
        }

        var replacement = Path.Combine(sourceRoot, "replacement.bin");
        File.WriteAllBytes(replacement, [0x64, 0x65, 0x66]);
        var mutationInvoked = false;
        var mutationError = Record.Exception(() => SecureSourceFileReader.ReadFileForMutationTest(
            target,
            repository.Root,
            () =>
            {
                mutationInvoked = true;
                File.Move(target, target + ".old");
                File.Move(replacement, target);
            }));
        Assert.True(mutationInvoked);
        Assert.NotNull(mutationError);
        Assert.True(mutationError is SourceHashException or IOException or UnauthorizedAccessException);
    }

    [Fact]
    public void RetainedRepositoryScopeRejectsRootRenameSwapRestore()
    {
        using var repository = TemporaryRepository.Create();
        var sourceRoot = Path.Combine(repository.Root, "code", "source");
        Directory.CreateDirectory(sourceRoot);
        File.WriteAllText(Path.Combine(sourceRoot, "original.txt"), "original");
        var movedRoot = repository.Root + ".original";
        var replacementRoot = repository.Root + ".replacement";
        Directory.CreateDirectory(Path.Combine(replacementRoot, "code", "source"));
        File.WriteAllText(Path.Combine(replacementRoot, "code", "source", "replacement.txt"), "replacement");

        using var scope = SecureSourceFileReader.RetainRepositoryRoot(repository.Root);
        _ = scope.EnumerateDirectory(sourceRoot, 10);
        var originalMoved = false;
        var replacementMoved = false;
        try
        {
            var swapError = Record.Exception(() =>
            {
                Directory.Move(repository.Root, movedRoot);
                originalMoved = true;
                Directory.Move(replacementRoot, repository.Root);
                replacementMoved = true;
                scope.Validate();
            });
            Assert.NotNull(swapError);
            Assert.True(swapError is SourceHashException or IOException or UnauthorizedAccessException);
        }
        finally
        {
            if (replacementMoved)
            {
                Directory.Move(repository.Root, replacementRoot);
            }
            if (originalMoved)
            {
                Directory.Move(movedRoot, repository.Root);
            }
            if (Directory.Exists(replacementRoot))
            {
                Directory.Delete(replacementRoot, recursive: true);
            }
        }
    }

    [Fact]
    public void RepositoryScopeKeepsDescriptorUseConstantAcrossManyDirectories()
    {
        using var repository = TemporaryRepository.Create();
        var codeRoot = Path.Combine(repository.Root, "code");
        Directory.CreateDirectory(codeRoot);
        for (var index = 0; index < 512; index++)
        {
            Directory.CreateDirectory(Path.Combine(codeRoot, $"directory-{index:D4}"));
        }

        using var scope = SecureSourceFileReader.RetainRepositoryRoot(repository.Root);
        var children = scope.EnumerateDirectory(codeRoot, 1_000);
        foreach (var child in children.Where(entry => entry.Kind == SecureDirectoryEntryKind.Directory))
        {
            _ = scope.EnumerateDirectory(Path.Combine(codeRoot, child.Name), 1);
            Assert.Equal(1, scope.RetainedNativeObjectCountForTest);
        }
        scope.Validate();
        Assert.Equal(1, scope.RetainedNativeObjectCountForTest);
    }

    [Fact]
    public void PortableGlobClassesMatchTheLanguageNeutralOracle()
    {
        var request = new SourceCollectionRequest(
            "csharp",
            "code/packages/csharp/demo",
            "declared_sources",
            Hasher.LanguageSourceInputRegistryDigest,
            [
                "src/[!a].cs",
                "src/[-a].cs",
                "src/[a-].cs",
                "src/[a-c].cs",
                "src/[^].cs",
                "src/[]].cs",
            ],
            [
                new SourceCollectionCandidate("src/-.cs", "file", false, [0x60]),
                new SourceCollectionCandidate("src/^.cs", "file", false, [0x61]),
                new SourceCollectionCandidate("src/].cs", "file", false, [0x62]),
                new SourceCollectionCandidate("src/a.cs", "file", false, [0x63]),
                new SourceCollectionCandidate("src/b.cs", "file", false, [0x64]),
                new SourceCollectionCandidate("src/c.cs", "file", false, [0x65]),
            ]);
        Assert.Equal(
            ["src/-.cs", "src/].cs", "src/^.cs", "src/a.cs", "src/b.cs", "src/c.cs"],
            Hasher.SelectSourceCandidates(request).Select(file => file.Path));

        Assert.Equal(
            "SOURCE_HASH_GLOB_INVALID",
            Assert.Throws<SourceHashException>(() => Hasher.SelectSourceCandidates(
                request with { DeclaredSources = ["CON"] })).Code);
        Assert.Equal(
            "SOURCE_HASH_GLOB_INVALID",
            Assert.Throws<SourceHashException>(() => Hasher.SelectSourceCandidates(
                request with { DeclaredSources = ["src/[a--!].cs"] })).Code);

        var unmatchedOpening = request with
        {
            DeclaredSources = ["src/[.cs"],
            Candidates = [new SourceCollectionCandidate("src/[.cs", "file", false, [0x61])],
        };
        Assert.Equal(
            ["src/[.cs"],
            Hasher.SelectSourceCandidates(unmatchedOpening).Select(file => file.Path));
    }

    [Fact]
    public void DeclaredSourceGlobWorkIsBoundedAcrossCandidates()
    {
        var patterns = Enumerable.Range(0, 256)
            .Select(index => $"unmatched/{new string('a', 220)}{index:D3}*.cs")
            .ToArray();
        var candidates = Enumerable.Range(0, 100)
            .Select(index => new SourceCollectionCandidate(
                $"src/file{index:D3}.cs",
                "file",
                false,
                [0x61]))
            .ToArray();
        var request = new SourceCollectionRequest(
            "csharp",
            "code/packages/csharp/demo",
            "declared_sources",
            Hasher.LanguageSourceInputRegistryDigest,
            patterns,
            candidates);

        Assert.Equal(
            "SOURCE_HASH_LIMIT_EXCEEDED",
            Assert.Throws<SourceHashException>(() => Hasher.SelectSourceCandidates(request)).Code);
    }

    [Fact]
    public void GitSnapshotIgnoresAmbientIndexAndComparesCompleteEvidence()
    {
        var package = new PackageSpec(
            "csharp/bitset-native",
            Path.Combine(RepositoryRoot, "code", "packages", "csharp", "bitset-native"),
            [],
            "csharp",
            []);
        var priorIndex = Environment.GetEnvironmentVariable("GIT_INDEX_FILE");
        try
        {
            Environment.SetEnvironmentVariable("GIT_INDEX_FILE", Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N")));
            var snapshot = Hasher.CaptureTrackedBoundarySnapshot(RepositoryRoot, [package]);
            Assert.Contains("code/packages/rust/Cargo.toml", snapshot.Keys);
        }
        finally
        {
            Environment.SetEnvironmentVariable("GIT_INDEX_FILE", priorIndex);
        }

        var original = new Dictionary<string, TrackedGitFile>(StringComparer.Ordinal)
        {
            ["code/packages/rust/Cargo.toml"] = new("100644", new string('a', 40), 0, "code/packages/rust/Cargo.toml"),
        };
        var changedObject = new Dictionary<string, TrackedGitFile>(StringComparer.Ordinal)
        {
            ["code/packages/rust/Cargo.toml"] = new("100644", new string('b', 40), 0, "code/packages/rust/Cargo.toml"),
        };
        var changedMode = new Dictionary<string, TrackedGitFile>(StringComparer.Ordinal)
        {
            ["code/packages/rust/Cargo.toml"] = new("100755", new string('a', 40), 0, "code/packages/rust/Cargo.toml"),
        };
        Assert.False(Hasher.TrackedSnapshotsEqual(original, changedObject));
        Assert.False(Hasher.TrackedSnapshotsEqual(original, changedMode));
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

    private static bool TryCreateHardLink(string linkPath, string targetPath) =>
        OperatingSystem.IsWindows()
            ? CreateHardLinkWindows(linkPath, targetPath, nint.Zero)
            : CreateHardLinkPosix(targetPath, linkPath) == 0;

    [DllImport("kernel32.dll", EntryPoint = "CreateHardLinkW", SetLastError = true, CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CreateHardLinkWindows(string linkPath, string targetPath, nint securityAttributes);

    [DllImport("libc", EntryPoint = "link", SetLastError = true)]
    private static extern int CreateHardLinkPosix(string targetPath, string linkPath);

    private sealed class TemporaryRepository : IDisposable
    {
        private TemporaryRepository(string root)
        {
            Root = root;
            Directory.CreateDirectory(Path.Combine(root, "code"));
        }

        internal string Root { get; }

        internal static TemporaryRepository Create() =>
            new(Path.Combine(Path.GetTempPath(), $"build-tool-csharp-source-hashing-{Guid.NewGuid():N}"));

        internal string PackageRoot(string language, string name) =>
            Path.Combine(Root, "code", "packages", language, name);

        public void Dispose()
        {
            if (Directory.Exists(Root))
            {
                Directory.Delete(Root, recursive: true);
            }
        }
    }
}
