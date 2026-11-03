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
    public void DiscoveryAndHashingReachOnlyRegisteredLegacySiteRoots()
    {
        WriteFile("code/sites/blog/BUILD", "echo build\n");
        WriteFile(
            "code/sites/blog/data/2026-05-08-capability-typed-stages.md",
            "# post\n");
        WriteFile("code/sites/blog/data/ignored.txt", "ignored\n");
        WriteFile("code/sites/unreviewed/BUILD", "echo ignored\n");
        WriteFile("code/sites/blog/nested/BUILD", "echo nested\n");

        var packages = Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code"));
        var package = Assert.Single(packages);
        Assert.Equal("unknown/blog", package.Name);
        Assert.Equal("unknown", package.Language);
        Assert.Equal(
            ["BUILD", "data/2026-05-08-capability-typed-stages.md", "nested/BUILD"],
            Hasher.CollectSourceFiles(package)
                .Select(path => Path.GetRelativePath(package.Path, path).Replace('\\', '/'))
                .OrderBy(path => path, StringComparer.Ordinal));

        var initialHash = Hasher.HashPackage(package);
        WriteFile("code/sites/blog/data/ignored.txt", "changed decoy\n");
        Assert.Equal(initialHash, Hasher.HashPackage(package));
        WriteFile(
            "code/sites/blog/data/2026-05-08-capability-typed-stages.md",
            "# changed post\n");
        Assert.NotEqual(initialHash, Hasher.HashPackage(package));
    }

    [Fact]
    public void DiscoveryConsumesTheSharedCanonicalLanguageRegistry()
    {
        using var fixture = JsonDocument.Parse(File.ReadAllText(Path.Combine(
            RepositoryRoot,
            "code",
            "specs",
            "fixtures",
            "build-tool-v1",
            "cases",
            "discovery-language-registry.json")));

        foreach (var file in fixture.RootElement.GetProperty("workspace").GetProperty("files").EnumerateArray())
        {
            WriteFile(
                file.GetProperty("path").GetString()!,
                file.GetProperty("content_utf8").GetString()!);
        }

        var actual = Discovery
            .DiscoverPackages(Path.Combine(_tempRoot, "code"), "linux")
            .Select(package => string.Join(
                '\0',
                package.Name,
                package.Language,
                Path.GetRelativePath(_tempRoot, package.Path).Replace('\\', '/')))
            .ToArray();
        var expected = fixture.RootElement
            .GetProperty("expected")
            .GetProperty("result")
            .GetProperty("packages")
            .EnumerateArray()
            .Select(package => string.Join(
                '\0',
                package.GetProperty("name").GetString(),
                package.GetProperty("language").GetString(),
                package.GetProperty("rel_path").GetString()))
            .ToArray();

        Assert.Equal(expected, actual);
    }

    [Fact]
    public void DiscoverySkipsOnlyTheExactDuneBuildComponent()
    {
        WriteFile("code/packages/ocaml/generated-a/_build/exact/BUILD", "echo generated\n");
        WriteFile("code/packages/ocaml/generated-b/_Build/case-source/BUILD", "echo source\n");
        WriteFile("code/packages/ocaml/generated-c/_build-example/near-source/BUILD", "echo source\n");

        var names = Discovery
            .DiscoverPackages(Path.Combine(_tempRoot, "code"))
            .Select(package => package.Name)
            .ToArray();

        Assert.DoesNotContain("ocaml/exact", names);
        Assert.Contains("ocaml/case-source", names);
        Assert.Contains("ocaml/near-source", names);
    }

    [Fact]
    public void DiscoveryDoesNotFollowLinkedPackageDirectoriesOrBuildFiles()
    {
        WriteFile("outside/linked-package/BUILD", "echo outside\n");
        WriteFile("outside/linked-build", "echo outside\n");
        var laneRoot = Path.Combine(_tempRoot, "code", "packages", "csharp");
        var packageRoot = Path.Combine(laneRoot, "linked-build");
        Directory.CreateDirectory(packageRoot);
        try
        {
            Directory.CreateSymbolicLink(
                Path.Combine(laneRoot, "linked-package"),
                Path.Combine(_tempRoot, "outside", "linked-package"));
            File.CreateSymbolicLink(
                Path.Combine(packageRoot, "BUILD"),
                Path.Combine(_tempRoot, "outside", "linked-build"));
        }
        catch (Exception error) when (error is UnauthorizedAccessException or IOException)
        {
            return;
        }

        var names = Discovery
            .DiscoverPackages(Path.Combine(_tempRoot, "code"))
            .Select(package => package.Name)
            .ToArray();

        Assert.DoesNotContain("csharp/linked-package", names);
        Assert.DoesNotContain("csharp/linked-build", names);
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

    // The Go reference's TestCargoLinkDependencyNames, case for case, so the
    // two readers are held to the same manifest shapes. Each rule it pins is
    // a step of the Rust paragraph of code/specs/build-tool-conformance.md.
    public static TheoryData<string, string, string[]> CargoLinkCases => new()
    {
        { "top-level inline", "[dependencies]\na = { path = \"../a\" }\n", ["a"] },
        { "build-dependencies", "[build-dependencies]\na = { path = \"../a\" }\n", ["a"] },
        { "legacy build_dependencies", "[build_dependencies]\na = { path = \"../a\" }\n", ["a"] },
        { "quoted cfg target", "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\na = { path = \"../a\" }\n", ["a"] },
        { "bare triple target build", "[target.x86_64-pc-windows-msvc.build-dependencies]\na = { path = \"../a\" }\n", ["a"] },
        { "legacy target build", "[target.'cfg(unix)'.build_dependencies]\na = { path = \"../a\" }\n", ["a"] },
        { "spaced header segments", "[ target . 'cfg(unix)' . dependencies ]\na = { path = \"../a\" }\n", ["a"] },
        { "quoted table segment", "[\"dependencies\"]\na = { path = \"../a\" }\n", ["a"] },
        { "sub-table", "[dependencies.a]\nversion = \"1\"\npath = \"../a\"\n", ["a"] },
        { "sub-table rename before path", "[build-dependencies.x]\npackage = \"A\"\npath = '../a'\n", ["a"] },
        { "target sub-table, dotted quoted triple", "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\npath = \"../a\"\n", ["a"] },
        { "quoted sub-table key", "[dependencies.\"a\"]\npath = \"../a\"\n", ["a"] },
        { "sub-table without path", "[dependencies.a]\nversion = \"1\"\n", [] },
        { "sub-table first path wins, ended by header", "[dependencies.a]\npath = \"../a\"\npath = \"../b\"\n[package]\nname = \"z\"\n", ["a"] },
        { "sub-table non-string path", "[dependencies.a]\npath = 1\n", [] },
        { "sub-table empty path", "[dependencies.a]\npath = \"\"\n", [] },
        { "sub-table capitalised Path key", "[dependencies.a]\nPath = \"../a\"\n", [] },
        { "rename", "[dependencies]\nx = { package = \"Logic-Gates\", path = \"../g\" }\n", ["logic-gates"] },
        { "capitalised Package is not a rename", "[dependencies]\nx = { Package = \"y\", path = \"../g\" }\n", ["x"] },
        { "capitalised Path is not a path", "[dependencies]\nx = { Path = \"../g\" }\n", [] },
        { "version plus path with trailing comment", "[dependencies]\na = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n", ["a"] },
        { "hash inside a string is not a comment", "[dependencies]\na = { path = \"../a#1\" }\n", ["a"] },
        { "quoted entry key", "[dependencies]\n\"a\" = { path = \"../a\" }\n", ["a"] },
        { "registry entry with path feature", "[dependencies]\na = { version = \"1\", features = [\"path\", \"x\"] }\nb = \"1\"\n", [] },
        { "nested table path is not the entry's", "[dependencies]\na = { x = { path = \"../a\" } }\n", [] },
        { "features array commas do not split fields", "[dependencies]\na = { features = [\"p\", \"q\"], path = \"../a\" }\n", ["a"] },
        { "empty path", "[dependencies]\na = { path = \"\" }\n", [] },
        { "array continued onto later lines", "[dependencies]\na = { path = \"../a\", features = [\n    \"x\", # [not-a-header]\n] }\nb = { path = \"../b\" }\n", ["a", "b"] },
        { "continuation lines are not headers", "[dependencies]\na = { path = \"../a\", features = [\n[\"x\"]\n] }\nb = { path = \"../b\" }\n", ["a", "b"] },
        { "braces continued onto later lines", "[dependencies]\na = { version = \"1\",\n  path = \"../a\" }\n", ["a"] },
        { "file ends inside the value", "[dependencies]\na = { path = \"../a\", features = [\n  \"x\",\n", [] },
        { "continued value must end with a brace", "[dependencies]\na = { path = \"../a\", features = [\n] } x\n", [] },
        { "dotted key is out of scope", "[dependencies]\na.path = \"../a\"\n", [] },
        { "commented-out entry", "[dependencies]\n# a = { path = \"../a\" }\n", [] },
        { "dev-dependencies", "[dev-dependencies]\na = { path = \"../a\" }\n[dev_dependencies]\nb = { path = \"../b\" }\n", [] },
        { "target dev-dependencies", "[target.'cfg(unix)'.dev-dependencies]\na = { path = \"../a\" }\n", [] },
        { "dev-dependencies sub-table", "[dev-dependencies.a]\npath = \"../a\"\n", [] },
        { "patch, replace, workspace", "[patch.crates-io]\na = { path = \"../a\" }\n[workspace.dependencies]\nb = { path = \"../b\" }\n[replace]\n\"c:1.0.0\" = { path = \"../c\" }\n", [] },
        { "capitalised table name", "[Dependencies]\na = { path = \"../a\" }\n", [] },
        { "features sub-table of a dependency", "[dependencies.a.features]\npath = \"../a\"\n", [] },
        { "target table itself", "[target.'cfg(unix)']\na = { path = \"../a\" }\n", [] },
        { "array of tables ends the table", "[dependencies]\n[[bin]]\na = { path = \"../a\" }\n", [] },
        { "malformed header ends the table", "[dependencies]\n[a b]\nc = { path = \"../c\" }\n", [] },
        { "unterminated quoted header ends the table", "[dependencies]\n[target.'cfg(unix).dependencies]\nc = { path = \"../c\" }\n", [] },
        { "CRLF and BOM", "\uFEFF[dependencies]\r\na = { path = \"../a\" }\r\n", ["a"] },
        { "union across tables keeps duplicates for the caller", "[dependencies]\na = { path = \"../a\" }\n[build-dependencies]\na = { path = \"../a\" }\n", ["a", "a"] },
        { "quoted inline path key", "[dependencies]\na = { \"path\" = \"../a\" }\n", ["a"] },
        { "literal-quoted inline path key", "[dependencies]\na = { 'path' = \"../a\" }\n", ["a"] },
        { "quoted inline package key renames", "[dependencies]\nx = { \"package\" = \"a\", path = \"../a\" }\n", ["a"] },
        { "quoted capitalised inline Path key", "[dependencies]\na = { \"Path\" = \"../a\" }\n", [] },
        { "escaped inline path key is compared verbatim", "[dependencies]\na = { \"pa\\u0074h\" = \"../a\" }\n", [] },
        { "dotted inline field key", "[dependencies]\na = { path.x = \"../a\" }\n", [] },
        { "quoted sub-table path key", "[dependencies.a]\n\"path\" = \"../a\"\n", ["a"] },
        { "quoted sub-table package key", "[dependencies.x]\n'package' = \"a\"\npath = \"../a\"\n", ["a"] },
        { "quoted capitalised sub-table Path key", "[dependencies.a]\n\"Path\" = \"../a\"\n", [] },
        { "open string continues across the join", "[dependencies]\na = { path = \"../a\", x = \"q [\n\" }\n", ["a"] },
        { "line-final backslash escapes the joining space", "[dependencies]\na = { x = \"q\\\n\", path = \"../a\" }\n", ["a"] },
    };

    [Fact]
    public void CargoLinkCaseTableMatchesTheGoReference()
    {
        Assert.Equal(57, CargoLinkCases.Count);
    }

    // Rescanning the whole joined value for every appended line is
    // quadratic: a 20,000-line continuation (about 500 KB) took tens of
    // seconds. Carrying the scanner state makes it one pass; the bound is
    // generous so a loaded CI machine cannot flake it.
    [Fact]
    public void CargoContinuationJoinIsLinear()
    {
        var manifest = "[dependencies]\na = { path = \"../a\", features = [\n"
            + string.Concat(Enumerable.Repeat("  \"feature-name-padding\",\n", 20_000)) + "] }\n";
        var stopwatch = System.Diagnostics.Stopwatch.StartNew();
        Assert.Equal(["a"], Resolver.CargoLinkDependencyNames(manifest));
        Assert.True(stopwatch.Elapsed < TimeSpan.FromSeconds(10), $"took {stopwatch.Elapsed}");
    }

    // `(?m)^\s*name` runs `\s*` across every following blank line from each
    // line start and backtracks when the match then fails (here: an
    // unterminated name), which is quadratic in a run of blank lines.
    [Fact]
    public void CargoPackageNameReaderIsLinearInBlankLines()
    {
        WriteFile("code/packages/rust/late-dir/Cargo.toml", new string('\n', 100_000) + "name = \"x");
        var package = new PackageSpec("rust/late-dir", Path.Combine(_tempRoot, "code/packages/rust/late-dir"), [], "rust", []);
        var stopwatch = System.Diagnostics.Stopwatch.StartNew();
        var known = Resolver.BuildKnownNames([package]);
        Assert.True(stopwatch.Elapsed < TimeSpan.FromSeconds(5), $"took {stopwatch.Elapsed}");
        Assert.Equal(["late-dir"], known.Keys);

        WriteFile("code/packages/rust/late-dir/Cargo.toml", new string('\n', 100_000) + "[package]\nname = \"Late-Crate\"\n");
        Assert.Equal("rust/late-dir", Resolver.BuildKnownNames([package])["late-crate"]);
    }

    // Step 5: a candidate resolving to the crate itself -- by its directory
    // alias, by its [package] name through a renamed sub-table, or from a
    // target table -- is DEPENDENCY_SELF_EDGE, neither dropped nor drawn.
    [Theory]
    [InlineData("[dependencies]\napp-dir = { path = \".\" }\n")]
    [InlineData("[build-dependencies.self]\npackage = \"app-crate\"\npath = \".\"\n")]
    [InlineData("[target.'cfg(unix)'.dependencies]\napp-crate = { path = \".\" }\n")]
    public void CargoSelfEdgeIsADiagnostic(string entry)
    {
        WriteFile("code/packages/rust/app-dir/BUILD", "cargo test\n");
        WriteFile("code/packages/rust/app-dir/Cargo.toml", "[package]\nname = \"app-crate\"\n\n" + entry);

        var error = Assert.Throws<DependencySelfEdgeException>(
            () => Resolver.ResolveDependencies(Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code"))));

        Assert.Equal("DEPENDENCY_SELF_EDGE", error.Code);
        Assert.Equal("rust/app-dir", error.Package);
        Assert.Equal("code/packages/rust/app-dir/Cargo.toml", error.Manifest);
        Assert.Equal("rust/app-dir", error.Dependency);
        Assert.Equal(
            "DEPENDENCY_SELF_EDGE: package=rust/app-dir manifest=code/packages/rust/app-dir/Cargo.toml dependency=rust/app-dir",
            error.Message);
        Assert.DoesNotContain(_tempRoot, error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task AppExitsTwoForACargoSelfEdge()
    {
        WriteFile("code/packages/rust/app/BUILD", "cargo test\n");
        WriteFile("code/packages/rust/app/Cargo.toml", "[package]\nname = \"app\"\n\n[dependencies]\napp = { path = \".\" }\n");

        var originalError = Console.Error;
        using var stderr = new StringWriter();
        Console.SetError(stderr);
        int exitCode;
        try
        {
            exitCode = await BuildToolApp.RunAsync(["--root", _tempRoot, "--force", "--dry-run"]);
        }
        finally
        {
            Console.SetError(originalError);
        }

        Assert.Equal(2, exitCode);
        Assert.Contains(
            "DEPENDENCY_SELF_EDGE: package=rust/app manifest=code/packages/rust/app/Cargo.toml dependency=rust/app",
            stderr.ToString(),
            StringComparison.Ordinal);
    }

    [Theory]
    [MemberData(nameof(CargoLinkCases))]
    public void CargoLinkDependencyNamesFollowTheLineContract(string name, string manifest, string[] want)
    {
        Assert.True(
            want.SequenceEqual(Resolver.CargoLinkDependencyNames(manifest)),
            $"case {name}: got [{string.Join(", ", Resolver.CargoLinkDependencyNames(manifest))}]");
    }

    [Fact]
    public void ResolverCollapsesACrateNamedByThreeLinkTables()
    {
        WriteFile("code/packages/rust/lib/BUILD", "cargo test\n");
        WriteFile("code/packages/rust/lib/Cargo.toml", "[package]\nname = \"lib\"\n");
        WriteFile("code/packages/rust/app/BUILD", "cargo test\n");
        WriteFile(
            "code/packages/rust/app/Cargo.toml",
            "[package]\nname = \"app\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n"
            + "serde = { path = \"../vendored-serde\" }\n\n[build-dependencies]\nlib = { path = \"../lib\" }\n\n"
            + "[target.'cfg(windows)'.dependencies.lib]\npath = \"../lib\"\n");

        var graph = Resolver.ResolveDependencies(Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code")));

        Assert.Equal(["rust/lib"], graph.Predecessors("rust/app"));
        Assert.Single(graph.Edges());
    }

    [Fact]
    public void ResolverReadsBuildTargetAndRenamedCargoTablesButNotDevOrPatch()
    {
        foreach (var crate in new[] { "builder", "unix-glue", "tester", "patched" })
        {
            WriteFile($"code/packages/rust/{crate}/BUILD", "cargo test\n");
            WriteFile($"code/packages/rust/{crate}/Cargo.toml", $"[package]\nname = \"{crate}\"\n");
        }

        WriteFile("code/packages/rust/app/BUILD", "cargo test\n");
        WriteFile(
            "code/packages/rust/app/Cargo.toml",
            "[package]\nname = \"app\"\n\n[build-dependencies]\nbuilder = { path = \"../builder\" }\n\n"
            + "[target.\"cfg(unix)\".dependencies.unix_alias]\npackage = \"Unix-Glue\"\npath = \"../unix-glue\"\n\n"
            + "[dev-dependencies]\ntester = { path = \"../tester\" }\n\n"
            + "[patch.crates-io]\npatched = { path = \"../patched\" }\n");

        var graph = Resolver.ResolveDependencies(Discovery.DiscoverPackages(Path.Combine(_tempRoot, "code")));

        Assert.Equal(["rust/builder", "rust/unix-glue"], graph.Predecessors("rust/app"));
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
    public void ToolchainForceFullStillRejectsAnUnsupportedSelectedLanguage()
    {
        var result = ToolchainDetection.EvaluateSnapshot(
            "linux",
            true,
            [new ToolchainPackageSnapshot("zig/app", "zig", new Dictionary<string, string> { ["BUILD"] = "" })],
            null,
            []);

        Assert.Equal("error", result.Outcome);
        Assert.Empty(result.Toolchains);
        var diagnostic = Assert.Single(result.Diagnostics);
        Assert.Equal("TOOLCHAIN_UNSUPPORTED", diagnostic.Code);
        Assert.Equal("zig/app", diagnostic.Package);
    }

    [Fact]
    public void ProductionToolchainMappingTreatsTheStarlarkBuildBucketAsGo()
    {
        Assert.Equal("go", BuildToolApp.ToolchainForLanguage("starlark"));
    }

    [Fact]
    public void ProductionForceFullEnablesTheClosedRegistryWithoutClassifyingSpecialBuckets()
    {
        var result = ToolchainDetection.EvaluateProductionPackages(
            [new PackageSpec("unknown/fixture", string.Empty, [], "unknown", [])],
            null,
            true,
            []);

        Assert.Equal("ok", result.Outcome);
        Assert.All(result.Toolchains, entry => Assert.True(entry.Value, entry.Key));
        Assert.Empty(result.Diagnostics);
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
