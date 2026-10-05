namespace CodingAdventures.BuildTool.CSharp.Tests;

using System.Text.Json;

public sealed class GraphDiffConformanceTests
{
    private static readonly string Cases = FindCases();
    private static readonly string[] GraphIds =
    [
        "graph/canonical-edge-order", "graph/chain", "graph/cycle", "graph/diamond",
        "graph/empty", "graph/isolated", "graph/multiple-components", "graph/partial-cycle-no-output",
    ];
    private static readonly string[] DiffIds =
    [
        "diff-selection/exact-build-fronts", "diff-selection/forced-package",
        "diff-selection/known-unmatched-near-build", "diff-selection/match-work-at-limit",
        "diff-selection/match-work-over-limit", "diff-selection/package-prefix",
        "diff-selection/repository-boundary-reverse-index", "diff-selection/shared-input-multiconsumer",
        "diff-selection/strict-glob-character-classes", "diff-selection/transitive-package-change",
        "diff-selection/unknown-path-all", "diff-selection/unknown-path-error",
    ];

    [Fact]
    public void EveryGraphFixtureUsesTypedProductionOperation()
    {
        var paths = Directory.GetFiles(Cases, "graph-*.json");
        Assert.Equal(GraphIds, paths.Select(ReadId).Order(StringComparer.Ordinal));
        foreach (var path in paths)
        {
            using var document = JsonDocument.Parse(File.ReadAllText(path));
            var root = document.RootElement;
            var options = root.GetProperty("input").GetProperty("options");
            var actual = GraphDiffCore.EvaluateGraph(new GraphInput(
                Strings(options.GetProperty("packages")), Edges(options.GetProperty("edges"))));
            var expected = root.GetProperty("expected");
            Assert.Equal(expected.GetProperty("diagnostics").EnumerateArray()
                .Select(value => value.GetProperty("code").GetString()).SingleOrDefault(), actual.ErrorCode);
            if (actual.ErrorCode is not null)
            {
                Assert.Empty(actual.Edges);
                Assert.Empty(actual.Levels);
            }
            else
            {
                var result = expected.GetProperty("result");
                Assert.Equal(Edges(result.GetProperty("edges")), actual.Edges);
                var levels = result.GetProperty("levels").EnumerateArray().Select(Strings).ToArray();
                Assert.Equal(levels.Length, actual.Levels.Count);
                for (var index = 0; index < levels.Length; index++)
                    Assert.Equal(levels[index], actual.Levels[index]);
            }
        }
    }

    [Fact]
    public void EveryDiffFixtureUsesTypedProductionOperation()
    {
        var paths = Directory.GetFiles(Cases, "diff-selection-*.json");
        Assert.Equal(DiffIds, paths.Select(ReadId).Order(StringComparer.Ordinal));
        var boundary = JsonSerializer.Deserialize<RepositoryBoundary>(
            File.ReadAllText(Path.Combine(Directory.GetParent(Cases)!.FullName,
                "repository-source-input-boundary.json")))!;
        foreach (var path in paths)
        {
            using var document = JsonDocument.Parse(File.ReadAllText(path));
            var root = document.RootElement;
            var input = root.GetProperty("input");
            var options = input.GetProperty("options");
            var packages = options.GetProperty("packages").EnumerateArray().Select(value =>
                new DiffPackage(value.GetProperty("name").GetString()!,
                    value.GetProperty("rel_path").GetString()!,
                    value.GetProperty("source_mode").GetString()!,
                    value.TryGetProperty("source_globs", out var globs) ? Strings(globs) : null)).ToArray();
            var digest = options.TryGetProperty("boundary_sha256", out var sha)
                ? sha.GetString() : null;
            var actual = GraphDiffCore.EvaluateDiffSelection(new DiffSelectionInput(
                packages, Edges(options.GetProperty("edges")),
                Strings(options.GetProperty("forced_packages")),
                options.GetProperty("unknown_path_policy").GetString()!,
                Strings(input.GetProperty("changed_paths")), digest,
                digest is null ? null : boundary));
            var expected = root.GetProperty("expected");
            Assert.Equal(expected.GetProperty("diagnostics").EnumerateArray()
                .Select(value => value.GetProperty("code").GetString()).SingleOrDefault(), actual.ErrorCode);
            if (actual.ErrorCode is not null)
            {
                Assert.Empty(actual.ChangedPackages);
                Assert.Empty(actual.AffectedPackages);
                Assert.Empty(actual.PrerequisitePackages);
            }
            else
            {
                var result = expected.GetProperty("result");
                Assert.Equal(Strings(result.GetProperty("changed_packages")), actual.ChangedPackages);
                Assert.Equal(Strings(result.GetProperty("affected_packages")), actual.AffectedPackages);
                Assert.Equal(Strings(result.GetProperty("prerequisite_packages")), actual.PrerequisitePackages);
            }
        }
    }

    [Fact]
    public void InvalidReferencesAndBoundaryDigestPrecedeMatchWork()
    {
        var graph = GraphDiffCore.EvaluateGraph(new GraphInput(["a/a"], [new GraphEdge("a/a", "b/b")]));
        Assert.NotNull(graph.ErrorCode);
        Assert.Empty(graph.Edges);
        var diff = GraphDiffCore.EvaluateDiffSelection(new DiffSelectionInput(
            [new DiffPackage("a/a", "a", "strict_globs", ["*"])], [], [], "error",
            ["a/x"], new string('0', 64), null));
        Assert.NotNull(diff.ErrorCode);
        Assert.NotEqual("DIFF_MATCH_LIMIT_EXCEEDED", diff.ErrorCode);
        Assert.Empty(diff.ChangedPackages);
    }

    [Fact]
    public void StructuralFailuresReturnNoPartialOutput()
    {
        var duplicate = GraphDiffCore.EvaluateGraph(new GraphInput(["a/a", "a/a"], []));
        Assert.Equal("GRAPH_INVALID_INPUT", duplicate.ErrorCode);
        Assert.Empty(duplicate.Levels);
        Assert.Equal("GRAPH_INVALID_INPUT", GraphDiffCore.EvaluateGraph(
            new GraphInput(["a/a\n"], [])).ErrorCode);
        var cycle = GraphDiffCore.EvaluateGraph(new GraphInput(["a/a", "b/b", "c/c"],
            [new GraphEdge("a/a", "b/b"), new GraphEdge("b/b", "a/a")]));
        Assert.Equal("GRAPH_CYCLE", cycle.ErrorCode);
        Assert.Empty(cycle.Edges);
        Assert.Empty(cycle.Levels); // The disconnected ready node must not leak.

        var invalidGlob = GraphDiffCore.EvaluateDiffSelection(new DiffSelectionInput(
            [new DiffPackage("a/a", "a", "strict_globs", ["src/[z-a].cs"])],
            [], ["a/a"], "error", ["outside"], null, null));
        Assert.Equal("DIFF_INVALID_INPUT", invalidGlob.ErrorCode);
        Assert.Empty(invalidGlob.ChangedPackages);
        var alias = GraphDiffCore.EvaluateDiffSelection(new DiffSelectionInput(
            [new DiffPackage("a/a", "a", "package_prefix", null)], [], [], "error",
            ["a/SS.txt", "a/ß.txt"], null, null));
        Assert.Equal("DIFF_INVALID_INPUT", alias.ErrorCode);
        var malformedUtf16 = GraphDiffCore.EvaluateDiffSelection(new DiffSelectionInput(
            [new DiffPackage("a/a", "a", "package_prefix", null)], [], [], "error",
            ["a/\ud800"], null, null));
        Assert.Equal("DIFF_INVALID_INPUT", malformedUtf16.ErrorCode);
    }

    [Fact]
    public void DigestFailurePrecedesLimitAndLimitPrecedesUnknownPath()
    {
        var overLimit = Path.Combine(Cases, "diff-selection-match-work-over-limit.json");
        using var document = JsonDocument.Parse(File.ReadAllText(overLimit));
        var input = document.RootElement.GetProperty("input");
        var options = input.GetProperty("options");
        var packages = options.GetProperty("packages").EnumerateArray().Select(value =>
            new DiffPackage(value.GetProperty("name").GetString()!,
                value.GetProperty("rel_path").GetString()!,
                value.GetProperty("source_mode").GetString()!,
                Strings(value.GetProperty("source_globs")))).ToArray();
        var request = new DiffSelectionInput(packages, [], [], "error",
            Strings(input.GetProperty("changed_paths")).Append("outside/unknown").ToArray(),
            null, null);
        Assert.Equal("DIFF_MATCH_LIMIT_EXCEEDED",
            GraphDiffCore.EvaluateDiffSelection(request).ErrorCode);
        Assert.Equal("DIFF_BOUNDARY_INVALID",
            GraphDiffCore.EvaluateDiffSelection(request with
            {
                BoundarySha256 = new string('0', 64),
            }).ErrorCode);
        var boundary = JsonSerializer.Deserialize<RepositoryBoundary>(
            File.ReadAllText(Path.Combine(Directory.GetParent(Cases)!.FullName,
                "repository-source-input-boundary.json")))!;
        Assert.Equal("DIFF_BOUNDARY_INVALID",
            GraphDiffCore.EvaluateDiffSelection(request with
            {
                BoundarySha256 = Hasher.RepositorySourceInputBoundaryDigest,
                Boundary = boundary with { SchemaVersion = 2 },
            }).ErrorCode);
    }

    [Fact]
    public void ExactBuildFrontAndPortableClassesAreNotHostGlobs()
    {
        var result = GraphDiffCore.EvaluateDiffSelection(new DiffSelectionInput(
            [new DiffPackage("a/a", "a", "strict_globs", ["src/[!a].cs"])],
            [], [], "error", ["a/nested/BUILD", "a/src/b.cs", "a/src/a.cs"],
            null, null));
        Assert.Null(result.ErrorCode);
        Assert.Equal(["a/a"], result.ChangedPackages);
        var nearBuild = GraphDiffCore.EvaluateDiffSelection(new DiffSelectionInput(
            [new DiffPackage("a/a", "a", "strict_globs", [])],
            [], [], "error", ["a/nested/BUILD_debug"], null, null));
        Assert.Null(nearBuild.ErrorCode);
        Assert.Empty(nearBuild.ChangedPackages);
    }

    private static string ReadId(string path)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        return document.RootElement.GetProperty("id").GetString()!;
    }

    private static string[] Strings(JsonElement array) =>
        array.EnumerateArray().Select(value => value.GetString()!).ToArray();

    private static GraphEdge[] Edges(JsonElement array) =>
        array.EnumerateArray().Select(value => new GraphEdge(
            value[0].GetString()!, value[1].GetString()!)).ToArray();

    private static string FindCases()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory);
             directory is not null; directory = directory.Parent)
        {
            var cases = Path.Combine(directory.FullName, "code", "specs", "fixtures",
                "build-tool-v1", "cases");
            if (Directory.Exists(cases)) return cases;
        }
        throw new DirectoryNotFoundException("build-tool-v1 cases not found");
    }
}
