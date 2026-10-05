// A build graph is a mathematical value, not a request to inspect a checkout.
// This module deliberately accepts materialized snapshots only. In particular,
// it never consults Git, a file, the process environment, or the wall clock.
namespace CodingAdventures.BuildTool.CSharp;

using System.Buffers;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

public sealed record GraphEdge(string Prerequisite, string Dependent);
public sealed record GraphInput(IReadOnlyList<string> Packages, IReadOnlyList<GraphEdge> Edges);
public sealed record GraphResult(string? ErrorCode, IReadOnlyList<GraphEdge> Edges,
    IReadOnlyList<IReadOnlyList<string>> Levels);

public sealed record DiffPackage(string Name, string RelPath, string SourceMode,
    IReadOnlyList<string>? SourceGlobs);
public sealed record DiffSelectionInput(IReadOnlyList<DiffPackage> Packages,
    IReadOnlyList<GraphEdge> Edges, IReadOnlyList<string> ForcedPackages,
    string UnknownPathPolicy, IReadOnlyList<string> ChangedPaths,
    string? BoundarySha256, RepositoryBoundary? Boundary);
public sealed record DiffSelectionResult(string? ErrorCode,
    IReadOnlyList<string> ChangedPackages, IReadOnlyList<string> AffectedPackages,
    IReadOnlyList<string> PrerequisitePackages);

// These records mirror the inert boundary registry shape. The optional
// generated_component is omitted, not serialized as null, in canonical JSON.
public sealed record RepositoryBoundary(
    [property: JsonPropertyName("schema_version")] int SchemaVersion,
    [property: JsonPropertyName("language_source_input_registry_sha256")] string LanguageSourceInputRegistrySha256,
    [property: JsonPropertyName("boundaries")] IReadOnlyList<RepositoryBoundaryRule> Boundaries);
public sealed record RepositoryBoundaryRule(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("input_origin")] string InputOrigin,
    [property: JsonPropertyName("applies_to")] RepositoryBoundaryApplicability AppliesTo,
    [property: JsonPropertyName("inputs")] IReadOnlyList<RepositoryBoundaryInput> Inputs,
    [property: JsonPropertyName("reason")] string Reason,
    [property: JsonPropertyName("owner")] string Owner);
public sealed record RepositoryBoundaryApplicability(
    [property: JsonPropertyName("exact_roots")] IReadOnlyList<string> ExactRoots,
    [property: JsonPropertyName("descendant_roots")] IReadOnlyList<string> DescendantRoots,
    [property: JsonPropertyName("excluded_roots")] IReadOnlyList<string> ExcludedRoots);
public sealed record RepositoryBoundaryInput(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("role")] string Role,
    [property: JsonPropertyName("generated_component")]
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? GeneratedComponent);

public static class GraphDiffCore
{
    private const long MatchWorkCeiling = 50_000_000;
    private static readonly Regex PackageName = new(
        @"\A[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)+\z",
        RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
    private static readonly Regex Slug = new(@"\A[a-z0-9]+(?:-[a-z0-9]+)*\z",
        RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
    private static readonly Regex Digest = new(@"\A[0-9a-f]{64}\z",
        RegexOptions.CultureInvariant | RegexOptions.NonBacktracking);
    private static readonly HashSet<string> BuildFronts =
    ["BUILD", "BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux"];
    private static readonly HashSet<string> InputOrigins =
    ["c", "cpp", "csharp", "dart", "dotnet", "elixir", "fsharp", "go", "haskell",
        "java", "kotlin", "lua", "mosaic", "ocaml", "perl", "python", "repository",
        "ruby", "rust", "starlark", "swift", "twig", "typescript", "wasm"];
    private static readonly JsonSerializerOptions BoundaryJson = new()
    {
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    private static GraphResult GraphError(string code) => new(code, [], []);
    private static DiffSelectionResult DiffError(string code) => new(code, [], [], []);

    public static GraphResult EvaluateGraph(GraphInput input)
    {
        if (input is null || !ValidGraph(input.Packages, input.Edges))
            return GraphError("GRAPH_INVALID_INPUT");

        // Kahn's algorithm emits complete levels. No level is published until
        // the whole graph is known to be acyclic, so cycle errors stay empty.
        var outgoing = input.Packages.ToDictionary(name => name,
            _ => new List<string>(), StringComparer.Ordinal);
        var indegree = input.Packages.ToDictionary(name => name, _ => 0, StringComparer.Ordinal);
        foreach (var edge in input.Edges)
        {
            outgoing[edge.Prerequisite].Add(edge.Dependent);
            indegree[edge.Dependent]++;
        }
        var levels = new List<IReadOnlyList<string>>();
        var ready = indegree.Where(pair => pair.Value == 0).Select(pair => pair.Key)
            .Order(ScalarComparer.Instance).ToArray();
        var emitted = 0;
        while (ready.Length != 0)
        {
            levels.Add(ready);
            emitted += ready.Length;
            var next = new List<string>();
            foreach (var name in ready)
                foreach (var dependent in outgoing[name])
                    if (--indegree[dependent] == 0) next.Add(dependent);
            ready = next.Order(ScalarComparer.Instance).ToArray();
        }
        if (emitted != input.Packages.Count) return GraphError("GRAPH_CYCLE");
        var edges = input.Edges.OrderBy(edge => edge.Prerequisite, ScalarComparer.Instance)
            .ThenBy(edge => edge.Dependent, ScalarComparer.Instance).ToArray();
        return new GraphResult(null, edges, levels);
    }

    public static DiffSelectionResult EvaluateDiffSelection(DiffSelectionInput input)
    {
        if (input is null || !ValidDiff(input)) return DiffError("DIFF_INVALID_INPUT");
        if (input.BoundarySha256 is not null &&
            !ValidBoundaryDigest(input.BoundarySha256, input.Boundary))
            return DiffError("DIFF_BOUNDARY_INVALID");

        // Complete Cartesian preflight is deliberately before both matching
        // and unknown-path policy. A quick early match cannot hide later work.
        long work = 0;
        foreach (var package in input.Packages)
        {
            if (package.SourceMode != "strict_globs") continue;
            long factor = 0;
            foreach (var pattern in package.SourceGlobs!)
                factor = checked(factor + pattern.EnumerateRunes().Count() + 1);
            foreach (var path in input.ChangedPaths)
            {
                if (!RelativeTo(path, package.RelPath, out var relative) ||
                    IsBuildFront(relative)) continue;
                try
                {
                    work = checked(work + checked(factor *
                        (relative.EnumerateRunes().Count() + 1L)));
                }
                catch (OverflowException) { return DiffError("DIFF_MATCH_LIMIT_EXCEEDED"); }
                if (work > MatchWorkCeiling) return DiffError("DIFF_MATCH_LIMIT_EXCEEDED");
            }
        }

        var seeds = new HashSet<string>(input.ForcedPackages, StringComparer.Ordinal);
        foreach (var path in input.ChangedPaths)
        {
            var known = false;
            foreach (var package in input.Packages)
            {
                if (!RelativeTo(path, package.RelPath, out var relative)) continue;
                known = true;
                if (package.SourceMode == "package_prefix" || IsBuildFront(relative) ||
                    package.SourceGlobs!.Any(pattern => Hasher.PortableGlobMatches(pattern, relative)))
                    seeds.Add(package.Name);
            }
            if (input.BoundarySha256 is not null)
            {
                foreach (var rule in input.Boundary!.Boundaries)
                {
                    if (!rule.Inputs.Any(item => item.Path == path)) continue;
                    foreach (var package in input.Packages)
                    {
                        if (!BoundaryApplies(rule.AppliesTo, package.RelPath)) continue;
                        known = true;
                        seeds.Add(package.Name);
                    }
                }
            }
            if (!known)
            {
                if (input.UnknownPathPolicy == "error") return DiffError("DIFF_UNKNOWN_PATH");
                seeds.UnionWith(input.Packages.Select(package => package.Name));
            }
        }
        var forward = input.Packages.ToDictionary(package => package.Name,
            _ => new List<string>(), StringComparer.Ordinal);
        var reverse = input.Packages.ToDictionary(package => package.Name,
            _ => new List<string>(), StringComparer.Ordinal);
        foreach (var edge in input.Edges)
        {
            forward[edge.Prerequisite].Add(edge.Dependent);
            reverse[edge.Dependent].Add(edge.Prerequisite);
        }
        var affected = Closure(seeds, forward);
        var prerequisites = Closure(affected, reverse);
        prerequisites.ExceptWith(affected);
        return new DiffSelectionResult(null, Sort(seeds), Sort(affected), Sort(prerequisites));
    }

    private static bool ValidGraph(IReadOnlyList<string>? packages, IReadOnlyList<GraphEdge>? edges)
    {
        if (packages is null || edges is null || packages.Count > 4096 || edges.Count > 16384 ||
            packages.Any(name => !ValidPackageName(name))) return false;
        var names = new HashSet<string>(packages, StringComparer.Ordinal);
        if (names.Count != packages.Count) return false;
        var pairs = new HashSet<GraphEdge>();
        return edges.All(edge => edge is not null && names.Contains(edge.Prerequisite) &&
            names.Contains(edge.Dependent) && pairs.Add(edge));
    }

    private static bool ValidDiff(DiffSelectionInput input)
    {
        if (input.Packages is null || input.Edges is null || input.ForcedPackages is null ||
            input.ChangedPaths is null || input.Packages.Count > 4096 ||
            input.ForcedPackages.Count > 4096 || input.ChangedPaths.Count > 4096 ||
            input.UnknownPathPolicy is not ("all" or "error") ||
            (input.BoundarySha256 is null && input.Boundary is not null)) return false;
        var names = new HashSet<string>(StringComparer.Ordinal);
        var rootIdentities = new HashSet<string>(StringComparer.Ordinal);
        foreach (var package in input.Packages)
        {
            if (package is null || !ValidPackageName(package.Name) ||
                !names.Add(package.Name) || !ValidPath(package.RelPath) ||
                !rootIdentities.Add(Identity(package.RelPath))) return false;
            if (package.SourceMode == "package_prefix")
            {
                if (package.SourceGlobs is not null) return false;
            }
            else if (package.SourceMode == "strict_globs")
            {
                if (package.SourceGlobs is null || package.SourceGlobs.Count > 256 ||
                    package.SourceGlobs.Distinct(StringComparer.Ordinal).Count() != package.SourceGlobs.Count ||
                    package.SourceGlobs.Any(pattern => !ValidGlob(pattern))) return false;
            }
            else return false;
        }
        if (!ValidGraph(names.ToArray(), input.Edges) ||
            input.ForcedPackages.Any(name => !names.Contains(name)) ||
            input.ForcedPackages.Distinct(StringComparer.Ordinal).Count() != input.ForcedPackages.Count ||
            input.ChangedPaths.Any(path => !ValidPath(path)) ||
            input.ChangedPaths.Select(Identity).Distinct(StringComparer.Ordinal).Count() != input.ChangedPaths.Count)
            return false;
        return true;
    }

    private static bool ValidPackageName(string? name) =>
        name is { Length: > 0 and <= 240 } && PackageName.IsMatch(name);

    private static bool ValidPath(string? path)
    {
        if (path is null || !ValidScalars(path)) return false;
        try { Hasher.ValidatePortablePath(path); return true; }
        catch (SourceHashException) { return false; }
        catch (ArgumentException) { return false; }
    }

    private static bool ValidGlob(string? pattern)
    {
        if (pattern is null || !ValidScalars(pattern)) return false;
        try { Hasher.ValidatePortableGlob(pattern); return true; }
        catch (SourceHashException) { return false; }
        catch (ArgumentException) { return false; }
    }

    private static string Identity(string path) =>
        TrackedArtifactUnicode17.CaseFold(TrackedArtifactUnicode17.Nfc(path));

    // .NET strings can contain a lone UTF-16 surrogate. Such a string has no
    // valid UTF-8 representation and must not be silently mapped to U+FFFD.
    private static bool ValidScalars(string value)
    {
        var remaining = value.AsSpan();
        while (!remaining.IsEmpty)
        {
            if (Rune.DecodeFromUtf16(remaining, out _, out var consumed) != OperationStatus.Done)
                return false;
            remaining = remaining[consumed..];
        }
        return true;
    }

    private static bool RelativeTo(string path, string root, out string relative)
    {
        if (path == root) { relative = ""; return true; }
        if (path.StartsWith(root + "/", StringComparison.Ordinal))
        {
            relative = path[(root.Length + 1)..]; return true;
        }
        relative = ""; return false;
    }

    private static bool IsBuildFront(string relative) =>
        relative.Length != 0 && BuildFronts.Contains(relative[(relative.LastIndexOf('/') + 1)..]);

    private static bool BoundaryApplies(RepositoryBoundaryApplicability scope, string root) =>
        scope.ExactRoots.Contains(root, StringComparer.Ordinal) ||
        (!scope.ExcludedRoots.Contains(root, StringComparer.Ordinal) &&
         scope.DescendantRoots.Any(parent => root.StartsWith(parent + "/", StringComparison.Ordinal)));

    private static bool ValidBoundaryDigest(string expected, RepositoryBoundary? boundary)
    {
        if (!Digest.IsMatch(expected) || !ValidBoundary(boundary)) return false;
        var json = JsonSerializer.Serialize(boundary, BoundaryJson);
        return string.Equals(Hasher.CanonicalRepositorySourceInputBoundaryDigest(json),
            expected, StringComparison.Ordinal);
    }

    private static bool ValidBoundary(RepositoryBoundary? boundary)
    {
        if (boundary is null || boundary.SchemaVersion != 1 ||
            boundary.LanguageSourceInputRegistrySha256 != Hasher.LanguageSourceInputRegistryDigest ||
            boundary.Boundaries is null or { Count: < 1 or > 256 }) return false;
        var ids = new HashSet<string>(StringComparer.Ordinal);
        var scopes = 0;
        var authorizations = 0;
        foreach (var rule in boundary.Boundaries)
        {
            if (rule is null || !ValidSlug(rule.Id) || !ids.Add(rule.Id) ||
                !ValidSlug(rule.Owner) || !InputOrigins.Contains(rule.InputOrigin ?? "") ||
                rule.Reason is null || !ValidScalars(rule.Reason) ||
                rule.Reason.EnumerateRunes().Count() is < 1 or > 512 ||
                rule.AppliesTo is null || rule.Inputs is null or { Count: < 1 or > 64 }) return false;
            var scope = rule.AppliesTo;
            if (scope.ExactRoots is null || scope.DescendantRoots is null || scope.ExcludedRoots is null ||
                scope.ExactRoots.Count > 4096 || scope.DescendantRoots.Count > 64 ||
                scope.ExcludedRoots.Count > 4096 ||
                scope.ExactRoots.Count + scope.DescendantRoots.Count == 0) return false;
            var roots = scope.ExactRoots.Concat(scope.DescendantRoots).Concat(scope.ExcludedRoots).ToArray();
            if (roots.Any(root => !ValidPath(root)) ||
                roots.Select(Identity).Distinct(StringComparer.Ordinal).Count() != roots.Length) return false;
            scopes += roots.Length;
            authorizations += (scope.ExactRoots.Count + scope.DescendantRoots.Count) * rule.Inputs.Count;
            if (scopes > 8192 || authorizations > 32768) return false;
            if (rule.Inputs.Any(item => item is null || !ValidPath(item.Path) ||
                item.Role is not ("cross_package_exact" or "generated_pruning_exception" or "shared_ancestor") ||
                (item.Role == "generated_pruning_exception") != (item.GeneratedComponent is not null) ||
                (item.GeneratedComponent is not null &&
                 (item.GeneratedComponent.EnumerateRunes().Count() is < 1 or > 128 ||
                  !ValidScalars(item.GeneratedComponent) ||
                  item.GeneratedComponent.Contains('/') || item.GeneratedComponent.Contains('\\'))))) return false;
            if (rule.Inputs.Select(item => Identity(item.Path)).Distinct(StringComparer.Ordinal).Count() !=
                rule.Inputs.Count) return false;
        }
        return true;
    }

    private static bool ValidSlug(string? value) =>
        value is { Length: > 0 and <= 120 } && Slug.IsMatch(value);

    private static HashSet<string> Closure(IEnumerable<string> seeds,
        IReadOnlyDictionary<string, List<string>> adjacency)
    {
        var visited = new HashSet<string>(seeds, StringComparer.Ordinal);
        var stack = new Stack<string>(seeds);
        while (stack.TryPop(out var current))
            foreach (var next in adjacency[current])
                if (visited.Add(next)) stack.Push(next);
        return visited;
    }

    private static string[] Sort(IEnumerable<string> names) =>
        names.Order(ScalarComparer.Instance).ToArray();

    // Ordinal UTF-16 is not Unicode scalar order for supplementary codepoints.
    private sealed class ScalarComparer : IComparer<string>
    {
        public static readonly ScalarComparer Instance = new();
        public int Compare(string? left, string? right)
        {
            if (ReferenceEquals(left, right)) return 0;
            if (left is null) return -1;
            if (right is null) return 1;
            using var l = left.EnumerateRunes().GetEnumerator();
            using var r = right.EnumerateRunes().GetEnumerator();
            while (true)
            {
                var moreLeft = l.MoveNext();
                var moreRight = r.MoveNext();
                if (!moreLeft || !moreRight) return moreLeft.CompareTo(moreRight);
                var comparison = l.Current.Value.CompareTo(r.Current.Value);
                if (comparison != 0) return comparison;
            }
        }
    }
}
