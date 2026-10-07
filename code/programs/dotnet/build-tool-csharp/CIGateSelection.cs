namespace CodingAdventures.BuildTool.CSharp;

// CI scheduling is a pure decision. These records contain snapshots supplied by
// a caller, not paths to read or commands to run. In particular null means the
// change detector failed, whereas an empty collection means it found nothing.
public sealed record CIGateDefinition(string Id, string Scope, string Description,
    IReadOnlyList<string> Packages, IReadOnlyList<string> Paths);
public sealed record CIGateRegistry(int SchemaVersion, IReadOnlyList<CIGateDefinition> Gates);
public sealed record CIGateSelectionInput(CIGateRegistry Registry,
    IReadOnlyList<string>? AffectedPackages, IReadOnlyList<string>? ChangedFiles, bool Force);
public sealed record CIGateVerdict(string Id, bool Required, string OutputName);
public sealed record CIGateSelectionResult(IReadOnlyList<CIGateVerdict> Gates, string? ErrorCode);

public static class CIGateSelection
{
    public const ulong MaximumMatchWork = 50_000_000;
    public const string MatchLimitError = "CI_GATE_MATCH_LIMIT_EXCEEDED";
    private const string InvalidRegistry = "CI_GATE_REGISTRY_INVALID";

    // Validate before the fail-open shortcuts: a broken registry cannot be
    // laundered into an apparently successful all-true decision.
    public static CIGateSelectionResult Evaluate(CIGateSelectionInput input)
    {
        if (input is null || !ValidRegistry(input.Registry)) return Failure(InvalidRegistry);
        var gates = input.Registry.Gates.OrderBy(gate => gate.Id, StringComparer.Ordinal).ToArray();
        var changed = input.ChangedFiles;
        if (input.Force || input.AffectedPackages is null || changed is null || TouchesMachinery(changed))
            return new CIGateSelectionResult(gates.Select(gate => Verdict(gate, true)).ToArray(), null);

        if (!TryPreflight(gates, changed, out var candidates)) return Failure(MatchLimitError);
        var affected = new HashSet<string>(input.AffectedPackages, StringComparer.Ordinal);
        var patternMatches = new Dictionary<string, bool>(StringComparer.Ordinal);
        bool Matches(string pattern)
        {
            if (patternMatches.TryGetValue(pattern, out var prior)) return prior;
            var mask = candidates[pattern];
            for (var index = 0; index < changed.Count; index++)
                if (mask[index] && Hasher.PortableGlobMatches(pattern, changed[index]))
                    return patternMatches[pattern] = true;
            return patternMatches[pattern] = false;
        }

        // The preflight charged all distinct pattern/path pairs before any
        // package intersection or early path hit can short-circuit matching.
        return new CIGateSelectionResult(gates.Select(gate => Verdict(gate,
            gate.Packages.Any(affected.Contains) || gate.Paths.Any(Matches))).ToArray(), null);
    }

    private static CIGateSelectionResult Failure(string code) => new([], code);
    private static CIGateVerdict Verdict(CIGateDefinition gate, bool required) =>
        new(gate.Id, required, "run_" + gate.Id.Replace('-', '_'));
    private static int ScalarCount(string text) => text.EnumerateRunes().Count();

    private static bool ValidRegistry(CIGateRegistry? registry)
    {
        if (registry is null || registry.SchemaVersion != 1 || registry.Gates is null ||
            registry.Gates.Count is < 1 or > 128) return false;
        var ids = new HashSet<string>(StringComparer.Ordinal);
        var outputs = new HashSet<string>(StringComparer.Ordinal);
        foreach (var gate in registry.Gates)
        {
            if (gate is null || string.IsNullOrEmpty(gate.Id) || ScalarCount(gate.Id) > 80 ||
                gate.Id.Any(ch => !(ch is >= 'a' and <= 'z') && !char.IsAsciiDigit(ch) && ch is not '-' and not '_') ||
                !ids.Add(gate.Id) || !outputs.Add(gate.Id.Replace('-', '_')) ||
                gate.Scope is not (null or "" or "job" or "step") || string.IsNullOrWhiteSpace(gate.Description) ||
                ScalarCount(gate.Description) > 240 || gate.Packages is null || gate.Paths is null ||
                gate.Packages.Count > 4096 || gate.Paths.Count > 4096 ||
                gate.Packages.Count + gate.Paths.Count == 0 ||
                gate.Packages.Distinct(StringComparer.Ordinal).Count() != gate.Packages.Count ||
                gate.Paths.Distinct(StringComparer.Ordinal).Count() != gate.Paths.Count)
                return false;
            foreach (var package in gate.Packages)
                if (!ValidPackage(package)) return false;
            foreach (var pattern in gate.Paths)
            {
                if (pattern is null) return false;
                try { Hasher.ValidatePortableGlob(pattern); }
                catch (SourceHashException) { return false; }
            }
        }
        return true;
    }

    private static bool ValidPackage(string? value)
    {
        if (string.IsNullOrEmpty(value) || ScalarCount(value) > 240) return false;
        var parts = value.Split('/');
        return parts.Length >= 2 && parts.All(part => part.Length > 0 && LowerOrDigit(part[0]) &&
            part.Skip(1).All(ch => LowerOrDigit(ch) || ch is '.' or '_' or '-'));
    }
    private static bool LowerOrDigit(char ch) => ch is >= 'a' and <= 'z' or >= '0' and <= '9';

    private static bool TouchesMachinery(IReadOnlyList<string> changed) => changed.Any(file =>
        file is ".github/workflows/ci.yml" or "code/specs/data/ci-gates.json" or
            "code/programs/go/build-tool/main.go" ||
        file.StartsWith("code/programs/go/build-tool/internal/cigates/", StringComparison.Ordinal) ||
        file.StartsWith("code/programs/go/build-tool/internal/globmatch/", StringComparison.Ordinal) ||
        file.StartsWith("code/programs/go/build-tool/internal/gitdiff/", StringComparison.Ordinal));

    private static bool TryPreflight(IReadOnlyList<CIGateDefinition> gates,
        IReadOnlyList<string> files, out Dictionary<string, bool[]> candidates)
    {
        var patterns = gates.SelectMany(gate => gate.Paths).Distinct(StringComparer.Ordinal)
            .Order(StringComparer.Ordinal).ToArray();
        candidates = new Dictionary<string, bool[]>(patterns.Length, StringComparer.Ordinal);
        // Every pair consumes at least two units. This guard avoids allocating
        // an unbounded pattern/file matrix before the more precise accounting.
        if (patterns.Length > 0 && files.Count > (long)(MaximumMatchWork / 2) / patterns.Length) return false;
        ulong remaining = MaximumMatchWork;
        var splitFiles = files.Select(SplitSegments).ToArray();
        var fileFactors = files.Select(file => (ulong)ScalarCount(file) + 1).ToArray();
        foreach (var pattern in patterns)
        {
            var parts = SplitSegments(pattern);
            var first = Array.FindIndex(parts, HasMeta);
            var last = Array.FindLastIndex(parts, HasMeta);
            var prefix = first < 0 ? parts : parts[..first];
            var suffix = last < 0 ? [] : parts[(last + 1)..];
            ulong literalCost = (ulong)prefix.Sum(part => ScalarCount(part) + 1) +
                (ulong)suffix.Sum(part => ScalarCount(part) + 1);
            var factor = (ulong)ScalarCount(pattern) + 1;
            var mask = new bool[files.Count];
            for (var index = 0; index < files.Count; index++)
            {
                if (literalCost > remaining) return false;
                remaining -= literalCost;
                var fileParts = splitFiles[index];
                var couldMatch = first < 0
                    ? fileParts.SequenceEqual(prefix, StringComparer.Ordinal)
                    : fileParts.Length >= prefix.Length + suffix.Length &&
                      fileParts.Take(prefix.Length).SequenceEqual(prefix, StringComparer.Ordinal) &&
                      fileParts.Skip(fileParts.Length - suffix.Length).SequenceEqual(suffix, StringComparer.Ordinal);
                if (!couldMatch) continue;
                if (fileFactors[index] > remaining / factor) return false;
                remaining -= factor * fileFactors[index];
                mask[index] = true;
            }
            candidates.Add(pattern, mask);
        }
        return true;
    }

    private static bool HasMeta(string segment) => segment.IndexOfAny(['*', '?', '[', ']']) >= 0;
    private static string[] SplitSegments(string path) => path.TrimEnd('/').Split('/',
        StringSplitOptions.RemoveEmptyEntries);
}
