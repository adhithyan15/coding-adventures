namespace CodingAdventures.BuildTool.CSharp.Tests;

using System.Text.Json;

public sealed class BuildToolTests : IDisposable
{
    private static readonly string RepositoryRoot = FindRepositoryRoot();
    private readonly string _tempRoot = Path.Combine(Path.GetTempPath(), $"build-tool-csharp-{Guid.NewGuid():N}");

    public BuildToolTests()
    {
        Directory.CreateDirectory(_tempRoot);
    }

    [Fact]
    public void DiscoveryFindsPackagesAndPrograms()
    {
        WriteFile("code/packages/csharp/hash-map/BUILD", "dotnet test\n");
        WriteFile("code/programs/dotnet/build-tool-csharp/BUILD", "dotnet test\n");

        var packages = Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code"));

        Assert.Contains(packages, package => package.Name == "csharp/hash-map");
        Assert.Contains(packages, package => package.Name == "dotnet/programs/build-tool-csharp");
    }

    [Fact]
    public void ResolverReadsDotnetProjectReferences()
    {
        WriteFile("code/packages/csharp/hash-map/BUILD", "dotnet test\n");
        WriteFile("code/packages/csharp/hash-map/CodingAdventures.HashMap.csproj", "<Project />\n");
        WriteFile("code/packages/csharp/hash-set/BUILD", "dotnet test\n");
        WriteFile(
            "code/packages/csharp/hash-set/CodingAdventures.HashSet.csproj",
            """
            <Project Sdk="Microsoft.NET.Sdk">
              <ItemGroup>
                <ProjectReference Include="../hash-map/CodingAdventures.HashMap.csproj" />
              </ItemGroup>
            </Project>
            """);

        var packages = Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code"));
        var graph = Resolver.ResolveDependencies(packages);

        Assert.Contains("csharp/hash-set", graph.Successors("csharp/hash-map"));
    }

    [Fact]
    public void GitDiffMapsFilesToContainingPackage()
    {
        WriteFile("code/packages/fsharp/md5/BUILD", "dotnet test\n");
        var packages = Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code"));

        var changed = GitDiff.MapFilesToPackages(
            ["code/packages/fsharp/md5/Md5.fs"],
            packages,
            _tempRoot);

        Assert.Equal(["fsharp/md5"], changed.OrderBy(value => value));
    }

    [Fact]
    public void HasherChangesWhenSourceFileChanges()
    {
        WriteFile("code/packages/csharp/bitset/BUILD", "dotnet test\n");
        WriteFile("code/packages/csharp/bitset/Bitset.cs", "class Bitset { }\n");

        var package = Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code")).Single();
        var firstHash = Hasher.HashPackage(package);

        WriteFile("code/packages/csharp/bitset/Bitset.cs", "class Bitset { public int Count => 1; }\n");
        var secondHash = Hasher.HashPackage(package);

        Assert.NotEqual(firstHash, secondHash);
    }

    [Fact]
    public void CacheRoundTrips()
    {
        var cache = new BuildCache();
        cache.Record("csharp/hash-map", "pkg", "deps", "success");

        var cacheFile = Path.Combine(_tempRoot, ".build-cache.json");
        cache.Save(cacheFile);

        var loaded = new BuildCache();
        loaded.Load(cacheFile);

        Assert.False(loaded.NeedsBuild("csharp/hash-map", "pkg", "deps"));
        Assert.True(loaded.NeedsBuild("csharp/hash-map", "pkg-2", "deps"));
    }

    [Fact]
    public async Task ExecutorMarksDependentsAsDepSkipped()
    {
        WriteFile("code/packages/csharp/hash-map/BUILD", "dotnet --definitely-not-a-real-flag\n");
        WriteFile("code/packages/csharp/hash-map/CodingAdventures.HashMap.csproj", "<Project />\n");
        WriteFile("code/packages/csharp/hash-set/BUILD", "dotnet --version\n");
        WriteFile(
            "code/packages/csharp/hash-set/CodingAdventures.HashSet.csproj",
            """
            <Project Sdk="Microsoft.NET.Sdk">
              <ItemGroup>
                <ProjectReference Include="../hash-map/CodingAdventures.HashMap.csproj" />
              </ItemGroup>
            </Project>
            """);

        var packages = Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code"));
        var graph = Resolver.ResolveDependencies(packages);
        var packageHashes = packages.ToDictionary(package => package.Name, Hasher.HashPackage);
        var dependencyHashes = packages.ToDictionary(
            package => package.Name,
            package => Hasher.HashDependencies(package.Name, graph, packageHashes));

        var results = await Executor.ExecuteBuildsAsync(new ExecuteBuildsOptions
        {
            Packages = packages,
            Graph = graph,
            Cache = new BuildCache(),
            PackageHashes = packageHashes,
            DependencyHashes = dependencyHashes,
            Force = true,
        });

        Assert.Equal("failed", results["csharp/hash-map"].Status);
        Assert.Equal("dep-skipped", results["csharp/hash-set"].Status);
    }

    [Fact]
    public void PlanFileRoundTrips()
    {
        var plan = new BuildPlan
        {
            SchemaVersion = PlanFile.CurrentSchemaVersion,
            DiffBase = "origin/main",
            Force = false,
            AffectedPackages = ["csharp/hash-map"],
            Packages =
            [
                new BuildPlanPackageEntry
                {
                    Name = "csharp/hash-map",
                    RelativePath = "code/packages/csharp/hash-map",
                    Language = "csharp",
                    BuildCommands = ["dotnet test"],
                },
            ],
            DependencyEdges = [new List<string> { "csharp/hash-map", "csharp/hash-set" }],
            LanguagesNeeded = new Dictionary<string, bool> { ["dotnet"] = true },
        };

        var planPath = Path.Combine(_tempRoot, "build-plan.json");
        PlanFile.Write(plan, planPath);
        var loaded = PlanFile.Read(planPath);

        Assert.Equal("origin/main", loaded.DiffBase);
        Assert.Equal("csharp/hash-map", loaded.Packages.Single().Name);
    }

    [Fact]
    public async Task AppEmitsPlanAndDetectsLanguages()
    {
        WriteFile("code/packages/csharp/hash-map/BUILD", "dotnet --version\n");
        WriteFile("code/packages/csharp/hash-map/CodingAdventures.HashMap.csproj", "<Project />\n");

        var exitCode = await BuildToolApp.RunAsync(
        [
            "--root", _tempRoot,
            "--force",
            "--emit-plan",
            "--plan-file", "build-plan.json",
        ]);

        Assert.Equal(0, exitCode);
        Assert.True(File.Exists(Path.Combine(_tempRoot, "build-plan.json")));

        using var document = JsonDocument.Parse(File.ReadAllText(Path.Combine(_tempRoot, "build-plan.json")));
        Assert.Equal(PlanFile.CurrentSchemaVersion, document.RootElement.GetProperty("schema_version").GetInt32());
    }

    [Theory]
    [InlineData("validation-orphan-crates-clean.json")]
    [InlineData("validation-orphan-crates-unlisted.json")]
    [InlineData("validation-orphan-exemptions-invalid.json")]
    [InlineData("validation-orphan-exemptions-stale.json")]
    public void OrphanCrateValidationMatchesSharedConformanceFixtures(string fixtureName)
    {
        var fixturePath = Path.Combine(
            RepositoryRoot,
            "code",
            "specs",
            "fixtures",
            "build-tool-v1",
            "cases",
            fixtureName);
        using var fixture = JsonDocument.Parse(File.ReadAllText(fixturePath));
        var snapshot = fixture.RootElement
            .GetProperty("input")
            .GetProperty("options")
            .GetProperty("orphan_snapshot");
        var directories = snapshot
            .GetProperty("directories")
            .EnumerateArray()
            .Select(path => path.GetString()!)
            .ToArray();
        var manifests = snapshot
            .GetProperty("manifests")
            .EnumerateArray()
            .Select(manifest => new OrphanManifest(
                manifest.GetProperty("path").GetString()!,
                manifest.GetProperty("kind").GetString()!))
            .ToArray();
        var buildFiles = snapshot
            .GetProperty("build_files")
            .EnumerateArray()
            .Select(buildFile => new OrphanBuildFile(
                buildFile.GetProperty("path").GetString()!,
                buildFile.GetProperty("state").GetString()!))
            .ToArray();
        var exemptions = snapshot
            .GetProperty("exemptions")
            .EnumerateArray()
            .Select(exemption => new OrphanExemption(
                exemption.GetProperty("line").GetInt32(),
                exemption.GetProperty("kind").GetString()!,
                exemption.GetProperty("path").GetString()!,
                exemption.GetProperty("reason").GetString()!))
            .ToArray();

        var actual = Validator.ValidateOrphanCrateSnapshot(
            new OrphanCrateSnapshot(directories, manifests, buildFiles, exemptions));
        var actualDiagnostics = JsonSerializer.SerializeToElement(actual.Diagnostics);
        var expected = fixture.RootElement.GetProperty("expected");
        var expectedDiagnostics = expected.GetProperty("diagnostics");
        var expectedPendingCount = expected
            .GetProperty("result")
            .GetProperty("pending_exemption_count")
            .GetInt32();

        Assert.True(
            JsonElement.DeepEquals(expectedDiagnostics, actualDiagnostics),
            $"Expected {expectedDiagnostics.GetRawText()}, but received {actualDiagnostics.GetRawText()}.");
        Assert.Equal(expectedPendingCount, actual.PendingExemptionCount);
    }

    public static TheoryData<string> InvalidOrphanExemptionPaths => new()
    {
        string.Empty,
        new string('a', 513),
        "/absolute/secret-project",
        "C:/host/secret-project",
        "code/packages/rust/bad<name>",
        "code/packages/rust/trailing.",
        "code/packages/rust/CON",
    };

    [Theory]
    [MemberData(nameof(InvalidOrphanExemptionPaths))]
    public void OrphanCrateValidationDoesNotEchoUnsafeExemptionPaths(string unsafePath)
    {
        var result = Validator.ValidateOrphanCrateSnapshot(new OrphanCrateSnapshot(
            ["code/packages/rust/demo"],
            [new OrphanManifest("code/packages/rust/demo", "package")],
            [],
            [new OrphanExemption(7, "PENDING", unsafePath, "not allowed")]));

        var invalid = Assert.Single(result.Diagnostics, diagnostic =>
            diagnostic.Code == "ORPHAN_EXEMPTION_INVALID");
        Assert.Equal("code/BUILD-EXEMPTIONS", invalid.Path);
        Assert.Equal("PATH_UNSAFE", invalid.Details.Problem);
        if (unsafePath.Length > 0)
        {
            Assert.DoesNotContain(unsafePath, JsonSerializer.Serialize(result), StringComparison.Ordinal);
        }
    }

    [Fact]
    public void OrphanCrateValidationUsesPythonWhitespaceForReasons()
    {
        var result = Validator.ValidateOrphanCrateSnapshot(new OrphanCrateSnapshot(
            ["code/packages/rust/demo"],
            [new OrphanManifest("code/packages/rust/demo", "package")],
            [],
            [new OrphanExemption(7, "PENDING", "code/packages/rust/demo", "\u001c")]));

        Assert.Equal(0, result.PendingExemptionCount);
        Assert.Equal(
            ["ORPHAN_CRATE_UNLISTED", "ORPHAN_EXEMPTION_INVALID"],
            result.Diagnostics.Select(diagnostic => diagnostic.Code));
        Assert.Equal("REASON_MISSING", result.Diagnostics[1].Details.Problem);
    }

    [Fact]
    public void OrphanCrateValidationChoosesClosestEmptyBuildThenFixedNameOrder()
    {
        var result = Validator.ValidateOrphanCrateSnapshot(new OrphanCrateSnapshot(
            ["code/packages/rust/demo/child"],
            [new OrphanManifest("code/packages/rust/demo/child", "package")],
            [
                new OrphanBuildFile("code/packages/rust/BUILD", "empty"),
                new OrphanBuildFile("code/packages/rust/demo/BUILD_linux", "empty"),
                new OrphanBuildFile("code/packages/rust/demo/BUILD", "empty"),
            ],
            []));

        var diagnostic = Assert.Single(result.Diagnostics);
        Assert.Equal("ORPHAN_CRATE_EMPTY_BUILD", diagnostic.Code);
        Assert.Equal("code/packages/rust/demo/BUILD", diagnostic.Details.BuildPath);
    }

    [Theory]
    [InlineData("validation-tracked-artifacts-clean.json")]
    [InlineData("validation-tracked-artifacts-forbidden.json")]
    [InlineData("validation-tracked-artifacts-aliases.json")]
    [InlineData("validation-tracked-artifacts-invalid.json")]
    [InlineData("validation-tracked-artifacts-unicode-boundaries.json")]
    public void TrackedArtifactValidationMatchesSharedConformanceFixtures(string fixtureName)
    {
        var fixturePath = Path.Combine(
            RepositoryRoot,
            "code",
            "specs",
            "fixtures",
            "build-tool-v1",
            "cases",
            fixtureName);
        using var fixture = JsonDocument.Parse(File.ReadAllText(fixturePath));
        var snapshot = fixture.RootElement
            .GetProperty("input")
            .GetProperty("options")
            .GetProperty("tracked_artifact_snapshot");
        var unicodeVersion = snapshot.GetProperty("unicode_version").GetString()!;
        var entries = snapshot
            .GetProperty("entries")
            .EnumerateArray()
            .Select(entry => new TrackedArtifactEntry(
                entry.GetProperty("ordinal").GetInt32(),
                entry.GetProperty("path").GetString()!,
                entry.GetProperty("entry_kind").GetString()!))
            .ToArray();

        var actual = JsonSerializer.SerializeToElement(
            Validator.ValidateTrackedArtifactSnapshot(unicodeVersion, entries));
        var expected = fixture.RootElement.GetProperty("expected").GetProperty("diagnostics");

        Assert.True(
            JsonElement.DeepEquals(expected, actual),
            $"Expected {expected.GetRawText()}, but received {actual.GetRawText()}.");
    }

    [Fact]
    public void TrackedArtifactValidationRejectsUnicodeVersionDrift()
    {
        Assert.Equal("17.0.0", Validator.TrackedArtifactUnicodeVersion);
        var error = Assert.Throws<ArgumentException>(() =>
            Validator.ValidateTrackedArtifactSnapshot("15.1.0", []));
        Assert.Contains("Unicode version must be 17.0.0", error.Message, StringComparison.Ordinal);
    }

    public static TheoryData<string, string> InvalidTrackedArtifactPaths => new()
    {
        { string.Empty, "EMPTY" },
        { new string('a', 513), "TOO_LONG" },
        { "code/packages/e\u0301/file.cs", "NON_NFC" },
        { "/absolute/file.cs", "ABSOLUTE" },
        { "C:\\repo\\file.cs", "DRIVE_QUALIFIED" },
        { "code//file.cs", "EMPTY_SEGMENT" },
        { "code/trailing/", "EMPTY_SEGMENT" },
        { "code\\trailing\\", "EMPTY_SEGMENT" },
        { "code/<unsafe>/file.cs", "UNSAFE_CHARACTER" },
        { "code/\u001f/file.cs", "UNSAFE_CHARACTER" },
        { "code/../file.cs", "DOT_SEGMENT" },
        { "code/trailing./file.cs", "TRAILING_DOT_OR_SPACE" },
        { "code/CON.txt/file.cs", "RESERVED_BASENAME" },
    };

    [Theory]
    [MemberData(nameof(InvalidTrackedArtifactPaths))]
    public void TrackedArtifactValidationRejectsEveryUnsafePathClassWithoutEchoingInput(
        string unsafePath,
        string expectedProblem)
    {
        var diagnostic = Assert.Single(Validator.ValidateTrackedArtifactSnapshot(
        [
            new TrackedArtifactEntry(7, unsafePath, "regular"),
        ]));

        Assert.Equal("TRACKED_ARTIFACT_PATH_INVALID", diagnostic.Code);
        Assert.Equal("repository", diagnostic.Path);
        Assert.Equal(expectedProblem, diagnostic.Details.Problem);
        if (unsafePath.Length > 0)
        {
            Assert.DoesNotContain(unsafePath, JsonSerializer.Serialize(diagnostic), StringComparison.Ordinal);
        }
    }

    [Fact]
    public void TrackedArtifactLengthLimitCountsUnicodeScalarsRatherThanUtf16Units()
    {
        var allowed = string.Concat(Enumerable.Repeat("\U0001f600", 512));
        var tooLong = string.Concat(Enumerable.Repeat("\U0001f600", 513));

        Assert.Empty(Validator.ValidateTrackedArtifactSnapshot(
        [
            new TrackedArtifactEntry(1, allowed, "regular"),
        ]));
        var diagnostic = Assert.Single(Validator.ValidateTrackedArtifactSnapshot(
        [
            new TrackedArtifactEntry(1, tooLong, "regular"),
        ]));
        Assert.Equal("TOO_LONG", diagnostic.Details.Problem);
    }

    [Fact]
    public void TrackedArtifactDiagnosticsSortPathsByUnicodeScalarValue()
    {
        var diagnostics = Validator.ValidateTrackedArtifactSnapshot(
        [
            new TrackedArtifactEntry(1, "\U00010000/node_modules/a", "regular"),
            new TrackedArtifactEntry(2, "\ue000/node_modules/b", "regular"),
        ]);

        Assert.Equal("\ue000/node_modules/b", diagnostics[0].Path);
        Assert.Equal("\U00010000/node_modules/a", diagnostics[1].Path);
    }

    [Fact]
    public void TrackedArtifactValidationUsesFullUppercaseForReservedBasenames()
    {
        var diagnostic = Assert.Single(Validator.ValidateTrackedArtifactSnapshot(
        [
            new TrackedArtifactEntry(1, "code/con\u0131n$.txt/file.cs", "regular"),
        ]));

        Assert.Equal("TRACKED_ARTIFACT_PATH_INVALID", diagnostic.Code);
        Assert.Equal("RESERVED_BASENAME", diagnostic.Details.Problem);
        Assert.Equal("repository", diagnostic.Path);
    }

    [Fact]
    public void ToolchainDetectionConsumesEveryNeutralFixture()
    {
        var fixtureDirectory = Path.Combine(
            RepositoryRoot,
            "code",
            "specs",
            "fixtures",
            "build-tool-v1",
            "cases");
        var fixturePaths = Directory
            .GetFiles(fixtureDirectory, "toolchain-detection-*.json")
            .OrderBy(path => path, StringComparer.Ordinal)
            .ToArray();
        Assert.True(fixturePaths.Length >= 10, $"Expected at least 10 fixtures, found {fixturePaths.Length}.");

        foreach (var fixturePath in fixturePaths)
        {
            using var fixture = JsonDocument.Parse(File.ReadAllText(fixturePath));
            var options = fixture.RootElement.GetProperty("input").GetProperty("options");
            var packages = options
                .GetProperty("packages")
                .EnumerateArray()
                .Select(package => new ToolchainPackageSnapshot(
                    package.GetProperty("name").GetString()!,
                    package.GetProperty("language").GetString()!,
                    package
                        .GetProperty("build_files")
                        .EnumerateObject()
                        .ToDictionary(
                            property => property.Name,
                            property => property.Value.GetString()!,
                            StringComparer.Ordinal)))
                .ToArray();
            var scheduledElement = options.GetProperty("scheduled_packages");
            IReadOnlyList<string>? scheduledPackages = scheduledElement.ValueKind == JsonValueKind.Null
                ? null
                : scheduledElement.EnumerateArray().Select(value => value.GetString()!).ToArray();
            var forcedToolchains = options
                .GetProperty("forced_toolchains")
                .EnumerateArray()
                .Select(value => value.GetString()!)
                .ToArray();

            var actual = ToolchainDetection.EvaluateSnapshot(
                options.GetProperty("platform").GetString()!,
                options.GetProperty("force_full").GetBoolean(),
                packages,
                scheduledPackages,
                forcedToolchains);
            var expected = fixture.RootElement.GetProperty("expected");
            var expectedOutcome = expected.GetProperty("outcome").GetString()!;

            Assert.True(
                string.Equals(expectedOutcome, actual.Outcome, StringComparison.Ordinal),
                $"{Path.GetFileName(fixturePath)} expected {expectedOutcome}, got {actual.Outcome}.");
            if (expectedOutcome == "ok")
            {
                var expectedToolchains = expected
                    .GetProperty("result")
                    .GetProperty("toolchains")
                    .EnumerateObject()
                    .ToDictionary(
                        property => property.Name,
                        property => property.Value.GetBoolean(),
                        StringComparer.Ordinal);
                Assert.Equal(expectedToolchains.Count, actual.Toolchains.Count);
                foreach (var (toolchain, needed) in expectedToolchains)
                {
                    Assert.True(actual.Toolchains.TryGetValue(toolchain, out var actualNeeded));
                    Assert.Equal(needed, actualNeeded);
                }
                Assert.Empty(actual.Diagnostics);
            }
            else
            {
                Assert.Empty(actual.Toolchains);
                Assert.Empty(expected.GetProperty("result").EnumerateObject());
                var expectedDiagnostic = Assert.Single(expected.GetProperty("diagnostics").EnumerateArray());
                var actualDiagnostic = Assert.Single(actual.Diagnostics);
                Assert.Equal(expectedDiagnostic.GetProperty("code").GetString(), actualDiagnostic.Code);
                Assert.Equal(expectedDiagnostic.GetProperty("severity").GetString(), actualDiagnostic.Severity);
                Assert.Equal(expectedDiagnostic.GetProperty("package").GetString(), actualDiagnostic.Package);
            }
        }
    }

    [Fact]
    public void DiscoveryReadsDeclarationsFromTheSelectedPlatformFront()
    {
        WriteFile(
            "code/packages/rust/java-to-semantic-ir/BUILD",
            "# needs-toolchain: java\ncargo test\n");
        WriteFile(
            "code/packages/rust/java-to-semantic-ir/BUILD_windows",
            "# needs-toolchain: python\ncargo test\n");

        var package = Assert.Single(Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code"), "windows"));

        Assert.Equal(["python"], package.ExtraToolchains);
        Assert.Equal(["cargo test"], package.BuildCommands);
    }

    [Fact]
    public void ToolchainSnapshotRejectsPerFileAndAggregateLimitOverruns()
    {
        var exactByteLimit = "# needs-toolchain: python\n";
        exactByteLimit += new string('a', ToolchainDetection.MaxBuildBytes - exactByteLimit.Length);
        Assert.Equal(["python"], ToolchainDetection.ParseExtraToolchains(exactByteLimit));

        var exactLineLimit = "# needs-toolchain: python" + new string('\n', ToolchainDetection.MaxBuildLines - 1);
        Assert.Equal(["python"], ToolchainDetection.ParseExtraToolchains(exactLineLimit));

        var oversizedFile = new string('a', ToolchainDetection.MaxBuildBytes + 1);
        Assert.Throws<ArgumentException>(() => ToolchainDetection.EvaluateSnapshot(
            "linux",
            false,
            [new ToolchainPackageSnapshot("rust/app", "rust", new Dictionary<string, string> { ["BUILD"] = oversizedFile })],
            null,
            []));

        var excessiveLines = exactLineLimit + "\n# needs-toolchain: java";
        Assert.Throws<ArgumentException>(() => ToolchainDetection.EvaluateSnapshot(
            "linux",
            false,
            [new ToolchainPackageSnapshot("rust/app", "rust", new Dictionary<string, string> { ["BUILD"] = excessiveLines })],
            null,
            []));

        var exactAggregatePackages = Enumerable
            .Range(0, 16)
            .Select(index => new ToolchainPackageSnapshot(
                $"rust/exact-{index}",
                "rust",
                new Dictionary<string, string> { ["BUILD"] = new string('a', ToolchainDetection.MaxBuildBytes) }))
            .ToArray();
        Assert.Equal(
            "ok",
            ToolchainDetection.EvaluateSnapshot("linux", false, exactAggregatePackages, [], []).Outcome);

        var aggregatePackages = Enumerable
            .Range(0, 17)
            .Select(index => new ToolchainPackageSnapshot(
                $"rust/app-{index}",
                "rust",
                new Dictionary<string, string> { ["BUILD"] = new string('a', 65_000) }))
            .ToArray();
        Assert.Throws<ArgumentException>(() => ToolchainDetection.EvaluateSnapshot(
            "linux",
            false,
            aggregatePackages,
            null,
            []));
    }

    [Fact]
    public void ToolchainDeclarationsRejectALoneCarriageReturnButAcceptCrlf()
    {
        Assert.Empty(ToolchainDetection.ParseExtraToolchains("# needs-toolchain: python\r"));
        Assert.Equal(
            ["python"],
            ToolchainDetection.ParseExtraToolchains("# needs-toolchain: python\r\n"));
    }

    [Fact]
    public void ProductionToolchainSchedulingConsultsOnlyAffectedPackageDeclarations()
    {
        var packages = new PackageSpec[]
        {
            new("rust/selected", string.Empty, [], "rust", ["python"]),
            new("go/unselected", string.Empty, [], "go", ["java"]),
        };

        var result = ToolchainDetection.EvaluatePackages(
            packages,
            ["rust/selected"],
            false,
            []);

        Assert.Equal("ok", result.Outcome);
        Assert.True(result.Toolchains["rust"]);
        Assert.True(result.Toolchains["python"]);
        Assert.False(result.Toolchains["go"]);
        Assert.False(result.Toolchains["java"]);
    }

    [Theory]
    [InlineData("c", "cpp")]
    [InlineData("cpp", "cpp")]
    [InlineData("csharp", "dotnet")]
    [InlineData("fsharp", "dotnet")]
    [InlineData("wasm", "rust")]
    [InlineData("ocaml", "ocaml")]
    public void ToolchainLanguageMappingUsesCanonicalKeys(string language, string expectedToolchain)
    {
        Assert.Equal(expectedToolchain, ToolchainDetection.ToolchainForLanguage(language));
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempRoot))
        {
            Directory.Delete(_tempRoot, recursive: true);
        }
    }

    private void WriteFile(string relativePath, string content)
    {
        var fullPath = Path.Combine(_tempRoot, relativePath.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(Path.GetDirectoryName(fullPath)!);
        File.WriteAllText(fullPath, content);
    }

    private static string FindRepositoryRoot()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory);
             directory is not null;
             directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(
                    directory.FullName,
                    "code",
                    "specs",
                    "fixtures",
                    "build-tool-v1",
                    "pure-domains.schema.json")))
            {
                return directory.FullName;
            }
        }

        throw new DirectoryNotFoundException("Could not locate the coding-adventures repository root.");
    }
}
