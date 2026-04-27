namespace CodingAdventures.BuildTool.CSharp.Tests;

using System.Text.Json;

public sealed class BuildToolTests : IDisposable
{
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
}
