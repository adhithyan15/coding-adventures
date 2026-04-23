namespace CodingAdventures.BuildTool.CSharp;

using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

// build-tool -- Incremental monorepo builder in C#
// =================================================
//
// This file keeps the entire C# port in one place so the educational story is
// easy to follow. The structure mirrors the other build-tool implementations in
// the repo:
//
//   discovery  -> walk BUILD files and infer package metadata
//   resolver   -> parse language manifests and build the dependency graph
//   git diff   -> determine what changed relative to origin/main
//   hasher     -> fall back to content hashing when git diff is unavailable
//   executor   -> run BUILD commands in topological batches
//   reporter   -> print a compact build summary
//   planner    -> emit JSON plans for CI and debugging

public sealed record PackageSpec(
    string Name,
    string Path,
    IReadOnlyList<string> BuildCommands,
    string Language);

public sealed record BuildResult(
    string PackageName,
    string Status,
    double Duration,
    string Stdout,
    string Stderr,
    int ReturnCode);

public sealed record CacheEntry(
    string PackageHash,
    string DepsHash,
    string LastBuilt,
    string Status);

public sealed class BuildPlanPackageEntry
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("rel_path")]
    public string RelativePath { get; init; } = string.Empty;

    [JsonPropertyName("language")]
    public string Language { get; init; } = string.Empty;

    [JsonPropertyName("build_commands")]
    public List<string> BuildCommands { get; init; } = [];
}

public sealed class BuildPlan
{
    [JsonPropertyName("schema_version")]
    public int SchemaVersion { get; init; }

    [JsonPropertyName("diff_base")]
    public string DiffBase { get; init; } = "origin/main";

    [JsonPropertyName("force")]
    public bool Force { get; init; }

    [JsonPropertyName("affected_packages")]
    public List<string>? AffectedPackages { get; init; }

    [JsonPropertyName("packages")]
    public List<BuildPlanPackageEntry> Packages { get; init; } = [];

    [JsonPropertyName("dependency_edges")]
    public List<List<string>> DependencyEdges { get; init; } = [];

    [JsonPropertyName("languages_needed")]
    public Dictionary<string, bool> LanguagesNeeded { get; init; } = [];
}

public sealed class CliOptions
{
    public string? Root { get; init; }
    public bool Force { get; init; }
    public bool DryRun { get; init; }
    public int? Jobs { get; init; }
    public string Language { get; init; } = "all";
    public string DiffBase { get; init; } = "origin/main";
    public string CacheFile { get; init; } = ".build-cache.json";
    public bool EmitPlan { get; init; }
    public string PlanFile { get; init; } = "build-plan.json";
    public bool DetectLanguages { get; init; }
    public bool ValidateBuildFiles { get; init; }
    public bool Help { get; init; }
}

public static class Cli
{
    public static CliOptions Parse(string[] args)
    {
        var values = new Dictionary<string, string?>(StringComparer.Ordinal)
        {
            ["language"] = "all",
            ["diff-base"] = "origin/main",
            ["cache-file"] = ".build-cache.json",
            ["plan-file"] = "build-plan.json",
        };
        var flags = new HashSet<string>(StringComparer.Ordinal);

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            if (!arg.StartsWith("--", StringComparison.Ordinal))
            {
                throw new ArgumentException($"Unexpected positional argument: {arg}");
            }

            var key = arg[2..];
            switch (key)
            {
                case "force":
                case "dry-run":
                case "emit-plan":
                case "detect-languages":
                case "validate-build-files":
                case "help":
                    flags.Add(key);
                    break;
                case "root":
                case "jobs":
                case "language":
                case "diff-base":
                case "cache-file":
                case "plan-file":
                    if (i + 1 >= args.Length)
                    {
                        throw new ArgumentException($"Missing value for --{key}");
                    }

                    values[key] = args[++i];
                    break;
                default:
                    throw new ArgumentException($"Unknown option: {arg}");
            }
        }

        int? jobs = null;
        if (values.TryGetValue("jobs", out var jobsText) && !string.IsNullOrWhiteSpace(jobsText))
        {
            if (!int.TryParse(jobsText, out var parsedJobs) || parsedJobs <= 0)
            {
                throw new ArgumentException("--jobs must be a positive integer");
            }

            jobs = parsedJobs;
        }

        return new CliOptions
        {
            Root = values.GetValueOrDefault("root"),
            Force = flags.Contains("force"),
            DryRun = flags.Contains("dry-run"),
            Jobs = jobs,
            Language = values["language"] ?? "all",
            DiffBase = values["diff-base"] ?? "origin/main",
            CacheFile = values["cache-file"] ?? ".build-cache.json",
            EmitPlan = flags.Contains("emit-plan"),
            PlanFile = values["plan-file"] ?? "build-plan.json",
            DetectLanguages = flags.Contains("detect-languages"),
            ValidateBuildFiles = flags.Contains("validate-build-files"),
            Help = flags.Contains("help"),
        };
    }

    public static string HelpText() =>
        """
        Usage: build-tool [options]

        Options:
          --root <dir>               Repo root directory (auto-detect from .git if not given)
          --force                    Rebuild everything regardless of cache
          --dry-run                  Show what would build without executing builds
          --jobs <n>                 Maximum number of parallel build jobs
          --language <lang>          Only build packages of this language
          --diff-base <ref>          Git ref to diff against (default: origin/main)
          --cache-file <file>        Path to the build cache file (default: .build-cache.json)
          --emit-plan                Write a build plan JSON file and exit
          --plan-file <file>         Path to write the build plan (default: build-plan.json)
          --detect-languages         Emit CI toolchain flags and exit
          --validate-build-files     Validate CI/build contracts before continuing
          --help                     Show this help message
        """;
}

public sealed class DirectedGraph
{
    private readonly Dictionary<string, HashSet<string>> _forward = new(StringComparer.Ordinal);
    private readonly Dictionary<string, HashSet<string>> _reverse = new(StringComparer.Ordinal);

    public void AddNode(string name)
    {
        if (!_forward.ContainsKey(name))
        {
            _forward[name] = new HashSet<string>(StringComparer.Ordinal);
        }

        if (!_reverse.ContainsKey(name))
        {
            _reverse[name] = new HashSet<string>(StringComparer.Ordinal);
        }
    }

    public void AddEdge(string from, string to)
    {
        AddNode(from);
        AddNode(to);
        _forward[from].Add(to);
        _reverse[to].Add(from);
    }

    public bool HasNode(string name) => _forward.ContainsKey(name);

    public IReadOnlyCollection<string> Successors(string name) =>
        _forward.TryGetValue(name, out var successors)
            ? successors.OrderBy(value => value, StringComparer.Ordinal).ToArray()
            : [];

    public IReadOnlyCollection<string> Predecessors(string name) =>
        _reverse.TryGetValue(name, out var predecessors)
            ? predecessors.OrderBy(value => value, StringComparer.Ordinal).ToArray()
            : [];

    public IReadOnlyList<IReadOnlyList<string>> IndependentGroups()
    {
        var inDegree = _reverse.ToDictionary(
            entry => entry.Key,
            entry => entry.Value.Count,
            StringComparer.Ordinal);

        var currentLevel = inDegree
            .Where(entry => entry.Value == 0)
            .Select(entry => entry.Key)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToList();

        var groups = new List<IReadOnlyList<string>>();
        var processed = 0;

        while (currentLevel.Count > 0)
        {
            groups.Add(currentLevel.ToArray());
            processed += currentLevel.Count;

            var nextLevel = new SortedSet<string>(StringComparer.Ordinal);
            foreach (var node in currentLevel)
            {
                foreach (var successor in _forward[node])
                {
                    inDegree[successor] -= 1;
                    if (inDegree[successor] == 0)
                    {
                        nextLevel.Add(successor);
                    }
                }
            }

            currentLevel = nextLevel.ToList();
        }

        if (processed != _forward.Count)
        {
            throw new InvalidOperationException("Dependency graph contains a cycle");
        }

        return groups;
    }

    public HashSet<string> AffectedNodes(IEnumerable<string> changed)
    {
        var affected = new HashSet<string>(StringComparer.Ordinal);
        foreach (var node in changed)
        {
            if (!HasNode(node))
            {
                continue;
            }

            affected.Add(node);
            foreach (var dependent in TransitiveDependents(node))
            {
                affected.Add(dependent);
            }
        }

        return affected;
    }

    public HashSet<string> TransitiveDependents(string node)
    {
        var visited = new HashSet<string>(StringComparer.Ordinal);
        var stack = new Stack<string>();

        if (_forward.TryGetValue(node, out var directDependents))
        {
            foreach (var dependent in directDependents)
            {
                stack.Push(dependent);
            }
        }

        while (stack.Count > 0)
        {
            var current = stack.Pop();
            if (!visited.Add(current))
            {
                continue;
            }

            foreach (var next in _forward[current])
            {
                stack.Push(next);
            }
        }

        return visited;
    }

    public HashSet<string> TransitiveDependencies(string node)
    {
        var visited = new HashSet<string>(StringComparer.Ordinal);
        var stack = new Stack<string>();

        if (_reverse.TryGetValue(node, out var directDependencies))
        {
            foreach (var dependency in directDependencies)
            {
                stack.Push(dependency);
            }
        }

        while (stack.Count > 0)
        {
            var current = stack.Pop();
            if (!visited.Add(current))
            {
                continue;
            }

            foreach (var dependency in _reverse[current])
            {
                stack.Push(dependency);
            }
        }

        return visited;
    }

    public IReadOnlyList<(string From, string To)> Edges() =>
        _forward
            .OrderBy(entry => entry.Key, StringComparer.Ordinal)
            .SelectMany(entry => entry.Value.OrderBy(value => value, StringComparer.Ordinal)
                .Select(value => (entry.Key, value)))
            .ToArray();
}

public static class Discovery
{
    public static readonly HashSet<string> SkipDirs = new(StringComparer.Ordinal)
    {
        ".git",
        ".hg",
        ".svn",
        ".venv",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        "__pycache__",
        "node_modules",
        "vendor",
        "dist",
        "build",
        "target",
        ".claude",
        "Pods",
        ".dart_tool",
        ".build",
        ".gradle",
        "gradle-build",
        "bin",
        "obj",
    };

    private static readonly string[] KnownLanguages =
    [
        "python", "ruby", "go", "typescript", "rust", "elixir", "lua", "perl",
        "swift", "dart", "haskell", "wasm", "java", "kotlin", "csharp",
        "fsharp", "dotnet", "starlark",
    ];

    public static IReadOnlyList<string> ReadLines(string filePath)
    {
        if (!File.Exists(filePath))
        {
            return [];
        }

        return File
            .ReadAllLines(filePath)
            .Select(line => line.Trim())
            .Where(line => line.Length > 0 && !line.StartsWith('#'))
            .ToArray();
    }

    public static string InferLanguage(string directoryPath)
    {
        var normalized = directoryPath.Replace('\\', '/');
        var parts = normalized.Split('/', StringSplitOptions.RemoveEmptyEntries);
        foreach (var language in KnownLanguages)
        {
            if (parts.Contains(language, StringComparer.Ordinal))
            {
                return language;
            }
        }

        return "unknown";
    }

    public static string InferPackageName(string directoryPath, string language)
    {
        var normalized = directoryPath.Replace('\\', '/');
        var baseName = Path.GetFileName(directoryPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        return normalized.Contains("/programs/", StringComparison.Ordinal)
            ? $"{language}/programs/{baseName}"
            : $"{language}/{baseName}";
    }

    public static string? GetBuildFile(string directory, string? platformOverride = null)
    {
        var platform = platformOverride ?? PlatformName();

        static string Combine(string directory, string file) => Path.Combine(directory, file);

        if (platform == "darwin" && File.Exists(Combine(directory, "BUILD_mac")))
        {
            return Combine(directory, "BUILD_mac");
        }

        if (platform == "linux" && File.Exists(Combine(directory, "BUILD_linux")))
        {
            return Combine(directory, "BUILD_linux");
        }

        if ((platform == "win32" || platform == "windows") && File.Exists(Combine(directory, "BUILD_windows")))
        {
            return Combine(directory, "BUILD_windows");
        }

        if ((platform == "darwin" || platform == "linux") && File.Exists(Combine(directory, "BUILD_mac_and_linux")))
        {
            return Combine(directory, "BUILD_mac_and_linux");
        }

        return File.Exists(Combine(directory, "BUILD")) ? Combine(directory, "BUILD") : null;
    }

    public static IReadOnlyList<PackageSpec> DiscoverPackages(string codeRoot)
    {
        var packages = new List<PackageSpec>();
        Walk(codeRoot, packages);
        packages.Sort((left, right) => StringComparer.Ordinal.Compare(left.Name, right.Name));
        return packages;
    }

    private static void Walk(string directory, List<PackageSpec> packages)
    {
        var directoryName = Path.GetFileName(directory);
        if (SkipDirs.Contains(directoryName))
        {
            return;
        }

        var buildFile = GetBuildFile(directory);
        if (buildFile is not null)
        {
            var language = InferLanguage(directory);
            packages.Add(new PackageSpec(
                InferPackageName(directory, language),
                Path.GetFullPath(directory),
                ReadLines(buildFile),
                language));
            return;
        }

        foreach (var child in Directory.EnumerateDirectories(directory).OrderBy(path => path, StringComparer.Ordinal))
        {
            Walk(child, packages);
        }
    }

    private static string PlatformName()
    {
        if (OperatingSystem.IsMacOS())
        {
            return "darwin";
        }

        if (OperatingSystem.IsWindows())
        {
            return "win32";
        }

        return "linux";
    }
}

public static class Resolver
{
    public static DirectedGraph ResolveDependencies(IReadOnlyList<PackageSpec> packages)
    {
        var graph = new DirectedGraph();
        foreach (var package in packages)
        {
            graph.AddNode(package.Name);
        }

        var knownNamesByLanguage = packages
            .Select(package => package.Language)
            .Distinct(StringComparer.Ordinal)
            .ToDictionary(
                language => language,
                language => BuildKnownNamesForLanguage(packages, language),
                StringComparer.Ordinal);

        foreach (var package in packages)
        {
            var knownNames = knownNamesByLanguage[package.Language];
            IReadOnlyList<string> dependencies = package.Language switch
            {
                "python" => ParsePythonDeps(package, knownNames),
                "ruby" => ParseRubyDeps(package, knownNames),
                "go" => ParseGoDeps(package, knownNames),
                "typescript" => ParseTypeScriptDeps(package, knownNames),
                "rust" => ParseRustDeps(package, knownNames),
                "wasm" => ParseRustDeps(package, knownNames),
                "elixir" => ParseElixirDeps(package, knownNames),
                "lua" => ParseLuaDeps(package, knownNames),
                "perl" => ParsePerlDeps(package, knownNames),
                "swift" => ParseSwiftDeps(package, knownNames),
                "dart" => ParseDartDeps(package, knownNames),
                "haskell" => ParseHaskellDeps(package, knownNames),
                "java" or "kotlin" => ParseGradleDeps(package, knownNames),
                "dotnet" or "csharp" or "fsharp" => ParseDotnetDeps(package, knownNames),
                _ => [],
            };

            foreach (var dependency in dependencies.Distinct(StringComparer.Ordinal))
            {
                graph.AddEdge(dependency, package.Name);
            }
        }

        return graph;
    }

    public static Dictionary<string, string> BuildKnownNames(IReadOnlyList<PackageSpec> packages) =>
        BuildKnownNamesForLanguage(packages, string.Empty);

    private static Dictionary<string, string> BuildKnownNamesForLanguage(
        IReadOnlyList<PackageSpec> packages,
        string language)
    {
        var known = new Dictionary<string, string>(StringComparer.Ordinal);
        var knownLanguage = new Dictionary<string, string>(StringComparer.Ordinal);
        var scope = DependencyScope(language);

        void SetKnown(string key, string value, string packagePath, string packageLanguage)
        {
            if (!known.TryGetValue(key, out var existing))
            {
                known[key] = value;
                knownLanguage[key] = packageLanguage;
                return;
            }

            var existingLanguage = knownLanguage[key];
            var existingIsProgram = existing.Contains("/programs/", StringComparison.Ordinal);
            var currentIsProgram = packagePath.Replace('\\', '/').Contains("/programs/", StringComparison.Ordinal);

            if (existingIsProgram && !currentIsProgram)
            {
                known[key] = value;
                knownLanguage[key] = packageLanguage;
                return;
            }

            if (!existingIsProgram && currentIsProgram)
            {
                return;
            }

            switch (scope)
            {
                case "wasm":
                    if (existingLanguage == "rust")
                    {
                        return;
                    }

                    if (packageLanguage == "rust")
                    {
                        known[key] = value;
                        knownLanguage[key] = packageLanguage;
                        return;
                    }

                    break;
                case "dotnet":
                    if (existingLanguage == language)
                    {
                        return;
                    }

                    if (packageLanguage == language)
                    {
                        known[key] = value;
                        knownLanguage[key] = packageLanguage;
                        return;
                    }

                    break;
            }

            if (!currentIsProgram)
            {
                known[key] = value;
                knownLanguage[key] = packageLanguage;
            }
        }

        foreach (var package in packages)
        {
            if (language.Length > 0 && !InDependencyScope(package.Language, scope))
            {
                continue;
            }

            var dirName = Path.GetFileName(package.Path).ToLowerInvariant();
            switch (package.Language)
            {
                case "python":
                    SetKnown($"coding-adventures-{dirName}", package.Name, package.Path, package.Language);
                    break;
                case "ruby":
                    SetKnown($"coding_adventures_{dirName}", package.Name, package.Path, package.Language);
                    break;
                case "go":
                    foreach (var line in File.ReadLines(Path.Combine(package.Path, "go.mod")))
                    {
                        if (line.StartsWith("module ", StringComparison.Ordinal))
                        {
                            var modulePath = line["module ".Length..].Trim().ToLowerInvariant();
                            known[modulePath] = package.Name;
                            knownLanguage[modulePath] = package.Language;
                            break;
                        }
                    }

                    break;
                case "typescript":
                    SetKnown($"@coding-adventures/{dirName}", package.Name, package.Path, package.Language);
                    SetKnown(dirName, package.Name, package.Path, package.Language);
                    TrySetPackageJsonName(package, SetKnown);
                    break;
                case "rust":
                    SetKnown(dirName, package.Name, package.Path, package.Language);
                    if (ReadCargoPackageName(package.Path) is { Length: > 0 } cargoName)
                    {
                        SetKnown(cargoName, package.Name, package.Path, package.Language);
                    }

                    break;
                case "wasm":
                    if (ReadCargoPackageName(package.Path) is { Length: > 0 } wasmCargoName)
                    {
                        SetKnown(wasmCargoName, package.Name, package.Path, package.Language);
                    }

                    break;
                case "elixir":
                {
                    var baseName = dirName.Replace("-", "_", StringComparison.Ordinal);
                    SetKnown($"coding_adventures_{baseName}", package.Name, package.Path, package.Language);
                    SetKnown(baseName, package.Name, package.Path, package.Language);
                    TrySetRegexName(Path.Combine(package.Path, "mix.exs"), @"app:\s*:([a-z0-9_]+)", package, SetKnown);
                    break;
                }
                case "dart":
                {
                    var baseName = dirName.Replace("-", "_", StringComparison.Ordinal);
                    SetKnown($"coding_adventures_{baseName}", package.Name, package.Path, package.Language);
                    SetKnown(baseName, package.Name, package.Path, package.Language);
                    TrySetRegexName(Path.Combine(package.Path, "pubspec.yaml"), @"(?m)^name\s*:\s*([a-z0-9_]+)\s*$", package, SetKnown);
                    break;
                }
                case "lua":
                    SetKnown($"coding-adventures-{dirName.Replace("_", "-", StringComparison.Ordinal)}", package.Name, package.Path, package.Language);
                    break;
                case "perl":
                    SetKnown($"coding-adventures-{dirName}", package.Name, package.Path, package.Language);
                    break;
                case "swift":
                case "java":
                case "kotlin":
                case "dotnet":
                case "csharp":
                case "fsharp":
                    SetKnown(dirName, package.Name, package.Path, package.Language);
                    break;
                case "haskell":
                    SetKnown($"coding-adventures-{dirName}", package.Name, package.Path, package.Language);
                    break;
            }
        }

        return known;
    }

    private static void TrySetPackageJsonName(
        PackageSpec package,
        Action<string, string, string, string> setKnown)
    {
        var filePath = Path.Combine(package.Path, "package.json");
        if (!File.Exists(filePath))
        {
            return;
        }

        try
        {
            using var document = JsonDocument.Parse(File.ReadAllText(filePath));
            if (document.RootElement.TryGetProperty("name", out var nameElement) &&
                nameElement.ValueKind == JsonValueKind.String &&
                nameElement.GetString() is { Length: > 0 } name)
            {
                setKnown(name.Trim().ToLowerInvariant(), package.Name, package.Path, package.Language);
            }
        }
        catch
        {
            // Ignore malformed package.json files during discovery.
        }
    }

    private static void TrySetRegexName(
        string filePath,
        string pattern,
        PackageSpec package,
        Action<string, string, string, string> setKnown)
    {
        if (!File.Exists(filePath))
        {
            return;
        }

        var match = Regex.Match(File.ReadAllText(filePath), pattern, RegexOptions.IgnoreCase);
        if (match.Success)
        {
            setKnown(match.Groups[1].Value.Trim().ToLowerInvariant(), package.Name, package.Path, package.Language);
        }
    }

    private static string DependencyScope(string language) =>
        language switch
        {
            "csharp" or "fsharp" or "dotnet" => "dotnet",
            "wasm" => "wasm",
            _ => language,
        };

    private static bool InDependencyScope(string packageLanguage, string scope) =>
        scope switch
        {
            "dotnet" => packageLanguage is "csharp" or "fsharp" or "dotnet",
            "wasm" => packageLanguage is "wasm" or "rust",
            _ => packageLanguage == scope,
        };

    private static string? ReadCargoPackageName(string packagePath)
    {
        var cargoToml = Path.Combine(packagePath, "Cargo.toml");
        if (!File.Exists(cargoToml))
        {
            return null;
        }

        var match = Regex.Match(File.ReadAllText(cargoToml), @"(?m)^\s*name\s*=\s*""([^""]+)""");
        return match.Success ? match.Groups[1].Value.Trim().ToLowerInvariant() : null;
    }

    private static IReadOnlyList<string> ParsePythonDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var pyproject = Path.Combine(package.Path, "pyproject.toml");
        if (!File.Exists(pyproject))
        {
            return [];
        }

        var dependencies = new List<string>();
        var inDeps = false;
        foreach (var line in File.ReadLines(pyproject))
        {
            var trimmed = line.Trim();
            if (!inDeps)
            {
                if (trimmed.StartsWith("dependencies", StringComparison.Ordinal) && trimmed.Contains('=', StringComparison.Ordinal))
                {
                    var afterEquals = trimmed[(trimmed.IndexOf('=') + 1)..].Trim();
                    if (afterEquals.StartsWith("[", StringComparison.Ordinal))
                    {
                        if (afterEquals.Contains(']', StringComparison.Ordinal))
                        {
                            ExtractQuotedDeps(afterEquals, knownNames, dependencies);
                            break;
                        }

                        inDeps = true;
                        ExtractQuotedDeps(afterEquals, knownNames, dependencies);
                    }
                }

                continue;
            }

            if (trimmed.Contains(']', StringComparison.Ordinal))
            {
                ExtractQuotedDeps(trimmed, knownNames, dependencies);
                break;
            }

            ExtractQuotedDeps(trimmed, knownNames, dependencies);
        }

        return dependencies;
    }

    private static void ExtractQuotedDeps(string line, Dictionary<string, string> knownNames, List<string> dependencies)
    {
        foreach (Match match in Regex.Matches(line, @"[""']([^""']+)[""']"))
        {
            var depName = Regex.Split(match.Groups[1].Value, @"[>=<!~\s;]")[0].Trim().ToLowerInvariant();
            if (knownNames.TryGetValue(depName, out var packageName))
            {
                dependencies.Add(packageName);
            }
        }
    }

    private static IReadOnlyList<string> ParseRubyDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var gemspec = Directory
            .EnumerateFiles(package.Path, "*.gemspec", SearchOption.TopDirectoryOnly)
            .FirstOrDefault();
        if (gemspec is null)
        {
            return [];
        }

        var dependencies = new List<string>();
        foreach (Match match in Regex.Matches(File.ReadAllText(gemspec), @"spec\.add_dependency\s+[""']([^""']+)[""']"))
        {
            var gemName = match.Groups[1].Value.Trim().ToLowerInvariant();
            if (knownNames.TryGetValue(gemName, out var packageName))
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseGoDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var goMod = Path.Combine(package.Path, "go.mod");
        if (!File.Exists(goMod))
        {
            return [];
        }

        var dependencies = new List<string>();
        var inRequireBlock = false;
        foreach (var line in File.ReadLines(goMod))
        {
            var trimmed = line.Trim();
            if (trimmed == "require (")
            {
                inRequireBlock = true;
                continue;
            }

            if (trimmed == ")")
            {
                inRequireBlock = false;
                continue;
            }

            if (!inRequireBlock && !trimmed.StartsWith("require ", StringComparison.Ordinal))
            {
                continue;
            }

            var cleaned = trimmed.StartsWith("require ", StringComparison.Ordinal)
                ? trimmed["require ".Length..].Trim()
                : trimmed;
            var parts = cleaned.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length > 0 && knownNames.TryGetValue(parts[0].ToLowerInvariant(), out var packageName))
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseTypeScriptDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var packageJson = Path.Combine(package.Path, "package.json");
        if (!File.Exists(packageJson))
        {
            return [];
        }

        try
        {
            using var document = JsonDocument.Parse(File.ReadAllText(packageJson));
            var dependencies = new List<string>();
            foreach (var propertyName in new[] { "dependencies", "devDependencies" })
            {
                if (!document.RootElement.TryGetProperty(propertyName, out var block) ||
                    block.ValueKind != JsonValueKind.Object)
                {
                    continue;
                }

                foreach (var property in block.EnumerateObject())
                {
                    var key = property.Name.Trim().ToLowerInvariant();
                    if (knownNames.TryGetValue(key, out var packageName))
                    {
                        dependencies.Add(packageName);
                    }
                }
            }

            return dependencies;
        }
        catch
        {
            return [];
        }
    }

    private static IReadOnlyList<string> ParseDartDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var pubspec = Path.Combine(package.Path, "pubspec.yaml");
        if (!File.Exists(pubspec))
        {
            return [];
        }

        var dependencies = new List<string>();
        var currentBlock = string.Empty;
        foreach (var line in File.ReadLines(pubspec))
        {
            var trimmed = line.Trim();
            if (trimmed.Length == 0 || trimmed.StartsWith('#'))
            {
                continue;
            }

            if (!char.IsWhiteSpace(line[0]) && trimmed.EndsWith(':'))
            {
                currentBlock = trimmed.TrimEnd(':') switch
                {
                    "dependencies" or "dev_dependencies" => trimmed.TrimEnd(':'),
                    _ => string.Empty,
                };
                continue;
            }

            if (currentBlock.Length == 0 || line.TakeWhile(char.IsWhiteSpace).Count() < 2)
            {
                continue;
            }

            if (trimmed.StartsWith("sdk:", StringComparison.Ordinal) ||
                trimmed.StartsWith("path:", StringComparison.Ordinal) ||
                !trimmed.Contains(':', StringComparison.Ordinal))
            {
                continue;
            }

            var depName = trimmed.Split(':', 2)[0].Trim().ToLowerInvariant();
            if (knownNames.TryGetValue(depName, out var packageName) && packageName != package.Name)
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseRustDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var cargoToml = Path.Combine(package.Path, "Cargo.toml");
        if (!File.Exists(cargoToml))
        {
            return [];
        }

        var dependencies = new List<string>();
        var inDeps = false;
        foreach (var line in File.ReadLines(cargoToml))
        {
            var trimmed = line.Trim();
            if (trimmed.StartsWith("[", StringComparison.Ordinal))
            {
                inDeps = trimmed == "[dependencies]";
                continue;
            }

            if (!inDeps || !trimmed.Contains("path", StringComparison.Ordinal) || !trimmed.Contains('=', StringComparison.Ordinal))
            {
                continue;
            }

            var parts = trimmed.Split('=', 2);
            var crateName = parts[0].Trim().ToLowerInvariant();
            if (knownNames.TryGetValue(crateName, out var packageName))
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseElixirDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var mixExs = Path.Combine(package.Path, "mix.exs");
        if (!File.Exists(mixExs))
        {
            return [];
        }

        var dependencies = new List<string>();
        foreach (Match match in Regex.Matches(File.ReadAllText(mixExs), @"\{:([a-z0-9_]+),\s*path:\s*""[^""]+""", RegexOptions.IgnoreCase))
        {
            var appName = match.Groups[1].Value.Trim().ToLowerInvariant();
            if (knownNames.TryGetValue(appName, out var packageName))
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseLuaDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var rockspec = Directory
            .EnumerateFiles(package.Path, "*.rockspec", SearchOption.TopDirectoryOnly)
            .FirstOrDefault();
        if (rockspec is null)
        {
            return [];
        }

        var dependencies = new List<string>();
        var inDeps = false;
        foreach (var line in File.ReadLines(rockspec))
        {
            var trimmed = line.Trim();
            if (!inDeps)
            {
                if (trimmed.Contains("dependencies", StringComparison.Ordinal) &&
                    trimmed.Contains('=', StringComparison.Ordinal) &&
                    trimmed.Contains('{', StringComparison.Ordinal))
                {
                    inDeps = true;
                }
            }

            if (!inDeps)
            {
                continue;
            }

            foreach (Match match in Regex.Matches(trimmed, @"""([^""]+)"""))
            {
                var depName = Regex.Split(match.Groups[1].Value, @"[>=<!~\s]")[0].Trim().ToLowerInvariant();
                if (knownNames.TryGetValue(depName, out var packageName))
                {
                    dependencies.Add(packageName);
                }
            }

            if (trimmed.Contains('}', StringComparison.Ordinal))
            {
                break;
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParsePerlDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var cpanfile = Path.Combine(package.Path, "cpanfile");
        if (!File.Exists(cpanfile))
        {
            return [];
        }

        var dependencies = new List<string>();
        foreach (var line in File.ReadLines(cpanfile))
        {
            var trimmed = line.Trim();
            if (trimmed.Length == 0 || trimmed.StartsWith('#'))
            {
                continue;
            }

            var match = Regex.Match(trimmed, @"requires\s+['""]coding-adventures-([^'""]+)['""]");
            if (!match.Success)
            {
                continue;
            }

            var depName = $"coding-adventures-{match.Groups[1].Value.Trim().ToLowerInvariant()}";
            if (knownNames.TryGetValue(depName, out var packageName))
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseSwiftDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var manifest = Path.Combine(package.Path, "Package.swift");
        if (!File.Exists(manifest))
        {
            return [];
        }

        var dependencies = new List<string>();
        foreach (var line in File.ReadLines(manifest))
        {
            var trimmed = line.Trim();
            if (trimmed.Length == 0 || trimmed.StartsWith("//", StringComparison.Ordinal))
            {
                continue;
            }

            var match = Regex.Match(trimmed, @"\.package\s*\(\s*path\s*:\s*""([^""]+)""");
            if (!match.Success)
            {
                continue;
            }

            var cleaned = Path.GetFullPath(Path.Combine(package.Path, match.Groups[1].Value));
            var depDir = Path.GetFileName(cleaned).ToLowerInvariant();
            if (depDir is "." or "..")
            {
                continue;
            }

            if (knownNames.TryGetValue(depDir, out var packageName))
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseHaskellDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var cabalFile = Directory
            .EnumerateFiles(package.Path, "*.cabal", SearchOption.TopDirectoryOnly)
            .FirstOrDefault();
        if (cabalFile is null)
        {
            return [];
        }

        var dependencies = new List<string>();
        foreach (Match match in Regex.Matches(File.ReadAllText(cabalFile), @"coding-adventures-([a-z0-9-]+)", RegexOptions.IgnoreCase))
        {
            var depName = $"coding-adventures-{match.Groups[1].Value.Trim().ToLowerInvariant()}";
            if (knownNames.TryGetValue(depName, out var packageName) && packageName != package.Name)
            {
                dependencies.Add(packageName);
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseDotnetDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var projectFiles = Directory
            .EnumerateFiles(package.Path, "*.*proj", SearchOption.TopDirectoryOnly)
            .Where(path => path.EndsWith(".csproj", StringComparison.Ordinal) || path.EndsWith(".fsproj", StringComparison.Ordinal))
            .ToArray();

        var dependencies = new List<string>();
        foreach (var projectFile in projectFiles)
        {
            foreach (Match match in Regex.Matches(
                         File.ReadAllText(projectFile),
                         @"<ProjectReference\s+Include\s*=\s*""\.\.[\\/](?<dep>[^/\\\""]+)[\\/][^\""]*""",
                         RegexOptions.IgnoreCase))
            {
                var depDir = match.Groups["dep"].Value.Trim().ToLowerInvariant();
                if (depDir.Contains('/') || depDir.Contains('\\') || depDir == "..")
                {
                    continue;
                }

                if (knownNames.TryGetValue(depDir, out var packageName))
                {
                    dependencies.Add(packageName);
                }
            }
        }

        return dependencies;
    }

    private static IReadOnlyList<string> ParseGradleDeps(PackageSpec package, Dictionary<string, string> knownNames)
    {
        var settingsFile = Path.Combine(package.Path, "settings.gradle.kts");
        if (!File.Exists(settingsFile))
        {
            return [];
        }

        var dependencies = new List<string>();
        foreach (var line in File.ReadLines(settingsFile))
        {
            var trimmed = line.Trim();
            if (trimmed.Length == 0 || trimmed.StartsWith("//", StringComparison.Ordinal))
            {
                continue;
            }

            foreach (Match match in Regex.Matches(trimmed, @"includeBuild\s*\(\s*""\.\./([^""]+)""\s*\)"))
            {
                var depDir = match.Groups[1].Value.Trim().ToLowerInvariant();
                if (depDir.Contains('/') || depDir.Contains('\\') || depDir == "..")
                {
                    continue;
                }

                if (knownNames.TryGetValue(depDir, out var packageName))
                {
                    dependencies.Add(packageName);
                }
            }
        }

        return dependencies;
    }
}

public static class GitDiff
{
    public static IReadOnlyList<string> GetChangedFiles(string repoRoot, string diffBase = "origin/main")
    {
        foreach (var arguments in new[]
                 {
                     $"diff --name-only {diffBase}...HEAD",
                     $"diff --name-only {diffBase} HEAD",
                 })
        {
            var result = ProcessRunner.RunProcess("git", arguments, repoRoot, useShell: false);
            if (result.ReturnCode == 0)
            {
                return result.Stdout
                    .Split('\n', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                    .Where(line => line.Length > 0)
                    .ToArray();
            }
        }

        return [];
    }

    public static HashSet<string> MapFilesToPackages(
        IEnumerable<string> changedFiles,
        IReadOnlyList<PackageSpec> packages,
        string repoRoot)
    {
        var packagePaths = packages
            .Select(package => new
            {
                Package = package,
                RelativePath = Path.GetRelativePath(repoRoot, package.Path).Replace('\\', '/'),
            })
            .OrderByDescending(entry => entry.RelativePath.Length)
            .ToArray();

        var changed = new HashSet<string>(StringComparer.Ordinal);
        foreach (var file in changedFiles)
        {
            var normalizedFile = file.Replace('\\', '/');
            foreach (var entry in packagePaths)
            {
                if (normalizedFile == entry.RelativePath ||
                    normalizedFile.StartsWith(entry.RelativePath + "/", StringComparison.Ordinal))
                {
                    changed.Add(entry.Package.Name);
                    break;
                }
            }
        }

        return changed;
    }
}

public sealed class BuildCache
{
    private readonly Dictionary<string, CacheEntry> _entries = new(StringComparer.Ordinal);
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };

    public IReadOnlyDictionary<string, CacheEntry> Entries =>
        new Dictionary<string, CacheEntry>(_entries, StringComparer.Ordinal);

    public void Load(string filePath)
    {
        _entries.Clear();
        if (!File.Exists(filePath))
        {
            return;
        }

        try
        {
            using var document = JsonDocument.Parse(File.ReadAllText(filePath));
            foreach (var property in document.RootElement.EnumerateObject())
            {
                var entry = property.Value;
                if (entry.TryGetProperty("package_hash", out var packageHash) &&
                    entry.TryGetProperty("deps_hash", out var depsHash) &&
                    entry.TryGetProperty("last_built", out var lastBuilt) &&
                    entry.TryGetProperty("status", out var status) &&
                    packageHash.ValueKind == JsonValueKind.String &&
                    depsHash.ValueKind == JsonValueKind.String &&
                    lastBuilt.ValueKind == JsonValueKind.String &&
                    status.ValueKind == JsonValueKind.String)
                {
                    _entries[property.Name] = new CacheEntry(
                        packageHash.GetString() ?? string.Empty,
                        depsHash.GetString() ?? string.Empty,
                        lastBuilt.GetString() ?? string.Empty,
                        status.GetString() ?? string.Empty);
                }
            }
        }
        catch
        {
            _entries.Clear();
        }
    }

    public void Save(string filePath)
    {
        var payload = _entries
            .OrderBy(entry => entry.Key, StringComparer.Ordinal)
            .ToDictionary(
                entry => entry.Key,
                entry => new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["package_hash"] = entry.Value.PackageHash,
                    ["deps_hash"] = entry.Value.DepsHash,
                    ["last_built"] = entry.Value.LastBuilt,
                    ["status"] = entry.Value.Status,
                },
                StringComparer.Ordinal);

        Directory.CreateDirectory(Path.GetDirectoryName(filePath) ?? ".");
        var tempPath = filePath + ".tmp";
        File.WriteAllText(tempPath, JsonSerializer.Serialize(payload, JsonOptions) + Environment.NewLine);
        File.Move(tempPath, filePath, overwrite: true);
    }

    public bool NeedsBuild(string packageName, string packageHash, string depsHash)
    {
        if (!_entries.TryGetValue(packageName, out var entry))
        {
            return true;
        }

        return entry.Status == "failed" ||
               entry.PackageHash != packageHash ||
               entry.DepsHash != depsHash;
    }

    public void Record(string packageName, string packageHash, string depsHash, string status)
    {
        _entries[packageName] = new CacheEntry(
            packageHash,
            depsHash,
            DateTimeOffset.UtcNow.ToString("O"),
            status);
    }
}

public static class Hasher
{
    private static readonly Dictionary<string, HashSet<string>> SourceExtensions = new(StringComparer.Ordinal)
    {
        ["python"] = [".py", ".toml", ".cfg"],
        ["ruby"] = [".rb", ".gemspec"],
        ["go"] = [".go"],
        ["typescript"] = [".ts", ".json"],
        ["rust"] = [".rs", ".toml"],
        ["wasm"] = [".rs", ".toml"],
        ["elixir"] = [".ex", ".exs"],
        ["lua"] = [".lua", ".rockspec"],
        ["perl"] = [".pl", ".pm", ".t", ".xs"],
        ["swift"] = [".swift"],
        ["dart"] = [".dart", ".yaml"],
        ["haskell"] = [".hs", ".cabal"],
        ["java"] = [".java", ".kts"],
        ["kotlin"] = [".kt", ".kts"],
        ["csharp"] = [".cs", ".csproj", ".json", ".md"],
        ["fsharp"] = [".fs", ".fsproj", ".json", ".md"],
        ["dotnet"] = [".cs", ".fs", ".csproj", ".fsproj", ".json", ".md"],
    };

    private static readonly Dictionary<string, HashSet<string>> SpecialNames = new(StringComparer.Ordinal)
    {
        ["ruby"] = ["Gemfile", "Rakefile"],
        ["go"] = ["go.mod", "go.sum"],
        ["rust"] = ["Cargo.lock"],
        ["typescript"] = ["package-lock.json"],
        ["elixir"] = ["mix.lock"],
        ["perl"] = ["Makefile.PL", "Build.PL", "cpanfile", "MANIFEST", "META.json", "META.yml"],
        ["swift"] = ["Package.swift"],
        ["dart"] = ["pubspec.yaml"],
        ["java"] = ["build.gradle.kts", "settings.gradle.kts", "gradle.properties"],
        ["kotlin"] = ["build.gradle.kts", "settings.gradle.kts", "gradle.properties"],
    };

    public static IReadOnlyList<string> CollectSourceFiles(PackageSpec package)
    {
        var extensions = SourceExtensions.GetValueOrDefault(package.Language, new HashSet<string>(StringComparer.Ordinal));
        var specialNames = SpecialNames.GetValueOrDefault(package.Language, new HashSet<string>(StringComparer.Ordinal));

        var files = new List<string>();
        foreach (var filePath in WalkFiles(package.Path))
        {
            var fileName = Path.GetFileName(filePath);
            if (fileName is "BUILD" or "BUILD_mac" or "BUILD_linux" or "BUILD_windows" or "BUILD_mac_and_linux")
            {
                files.Add(filePath);
                continue;
            }

            if (extensions.Contains(Path.GetExtension(filePath)))
            {
                files.Add(filePath);
                continue;
            }

            if (specialNames.Contains(fileName))
            {
                files.Add(filePath);
            }
        }

        return files
            .OrderBy(filePath => Path.GetRelativePath(package.Path, filePath), StringComparer.Ordinal)
            .ToArray();
    }

    public static string HashFile(string filePath)
    {
        using var sha256 = SHA256.Create();
        return Convert.ToHexString(sha256.ComputeHash(File.ReadAllBytes(filePath))).ToLowerInvariant();
    }

    public static string HashPackage(PackageSpec package)
    {
        var combined = new StringBuilder();
        foreach (var file in CollectSourceFiles(package))
        {
            combined.Append(HashFile(file));
            combined.Append('\n');
        }

        using var sha256 = SHA256.Create();
        return Convert.ToHexString(sha256.ComputeHash(Encoding.UTF8.GetBytes(combined.ToString()))).ToLowerInvariant();
    }

    public static string HashDependencies(
        string packageName,
        DirectedGraph graph,
        IReadOnlyDictionary<string, string> packageHashes)
    {
        var parts = graph.TransitiveDependencies(packageName)
            .OrderBy(name => name, StringComparer.Ordinal)
            .Select(name => packageHashes.TryGetValue(name, out var hash) ? hash : string.Empty);
        var combined = string.Join('\n', parts);
        using var sha256 = SHA256.Create();
        return Convert.ToHexString(sha256.ComputeHash(Encoding.UTF8.GetBytes(combined))).ToLowerInvariant();
    }

    private static IEnumerable<string> WalkFiles(string directory)
    {
        foreach (var childDirectory in Directory.EnumerateDirectories(directory).OrderBy(path => path, StringComparer.Ordinal))
        {
            var name = Path.GetFileName(childDirectory);
            if (Discovery.SkipDirs.Contains(name))
            {
                continue;
            }

            foreach (var file in WalkFiles(childDirectory))
            {
                yield return file;
            }
        }

        foreach (var file in Directory.EnumerateFiles(directory).OrderBy(path => path, StringComparer.Ordinal))
        {
            yield return file;
        }
    }
}

public static class Reporter
{
    public static string FormatDuration(double seconds) =>
        seconds < 0.01 ? "-" : $"{seconds:0.0}s";

    public static string FormatReport(IReadOnlyDictionary<string, BuildResult> results)
    {
        var lines = new List<string> { string.Empty, "Build Report", "============" };
        if (results.Count == 0)
        {
            lines.Add("No packages processed.");
            return string.Join(Environment.NewLine, lines) + Environment.NewLine;
        }

        var maxNameLength = Math.Max("Package".Length, results.Keys.Max(name => name.Length));
        lines.Add($"{ "Package".PadRight(maxNameLength) }   { "Status".PadRight(12) } Duration");

        foreach (var name in results.Keys.OrderBy(value => value, StringComparer.Ordinal))
        {
            var result = results[name];
            var duration = result.Status == "dep-skipped"
                ? "- (dep failed)"
                : FormatDuration(result.Duration);
            lines.Add($"{name.PadRight(maxNameLength)}   {result.Status.ToUpperInvariant().PadRight(12)} {duration}");
        }

        var total = results.Count;
        var built = results.Values.Count(result => result.Status == "built");
        var skipped = results.Values.Count(result => result.Status == "skipped");
        var failed = results.Values.Count(result => result.Status == "failed");
        var depSkipped = results.Values.Count(result => result.Status == "dep-skipped");
        var wouldBuild = results.Values.Count(result => result.Status == "would-build");

        var summary = new StringBuilder().Append($"{Environment.NewLine}Total: {total} packages");
        if (built > 0) summary.Append($" | {built} built");
        if (skipped > 0) summary.Append($" | {skipped} skipped");
        if (failed > 0) summary.Append($" | {failed} failed");
        if (depSkipped > 0) summary.Append($" | {depSkipped} dep-skipped");
        if (wouldBuild > 0) summary.Append($" | {wouldBuild} would-build");
        lines.Add(summary.ToString());

        return string.Join(Environment.NewLine, lines) + Environment.NewLine;
    }

    public static void PrintReport(IReadOnlyDictionary<string, BuildResult> results) =>
        Console.Write(FormatReport(results));
}

public static class PlanFile
{
    public const int CurrentSchemaVersion = 1;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
    };

    public static void Write(BuildPlan plan, string filePath)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(filePath) ?? ".");
        File.WriteAllText(filePath, JsonSerializer.Serialize(plan, JsonOptions) + Environment.NewLine);
    }

    public static BuildPlan Read(string filePath)
    {
        var plan = JsonSerializer.Deserialize<BuildPlan>(File.ReadAllText(filePath), JsonOptions)
                   ?? throw new InvalidOperationException($"Invalid plan file: {filePath}");
        if (plan.SchemaVersion != CurrentSchemaVersion)
        {
            throw new InvalidOperationException(
                $"Plan file {filePath}: schema_version is {plan.SchemaVersion}, expected {CurrentSchemaVersion}");
        }

        return plan;
    }
}

public sealed class CiWorkflowChange
{
    public HashSet<string> Toolchains { get; init; } = new(StringComparer.Ordinal);
    public bool RequiresFullRebuild { get; init; }
}

public static class CiWorkflow
{
    public const string WorkflowPath = ".github/workflows/ci.yml";

    private static readonly Dictionary<string, string[]> ToolchainMarkers = new(StringComparer.Ordinal)
    {
        ["python"] = ["needs_python", "setup-python", "python-version", "setup-uv", "pytest"],
        ["ruby"] = ["needs_ruby", "setup-ruby", "ruby-version", "bundler"],
        ["go"] = ["needs_go", "setup-go", "go-version"],
        ["typescript"] = ["needs_typescript", "setup-node", "node-version", "vitest"],
        ["rust"] = ["needs_rust", "rust-toolchain", "cargo", "rustc", "tarpaulin"],
        ["elixir"] = ["needs_elixir", "setup-beam", "elixir-version", "otp-version"],
        ["lua"] = ["needs_lua", "gh-actions-lua", "gh-actions-luarocks", "luarocks"],
        ["perl"] = ["needs_perl", "cpanm", "perl --version"],
        ["swift"] = ["needs_swift", "swift --version"],
        ["dart"] = ["needs_dart", "setup-dart", "dart --version"],
        ["java"] = ["needs_java", "setup-java", "java-version", "gradle"],
        ["kotlin"] = ["needs_kotlin", "setup-java", "gradle"],
        ["haskell"] = ["needs_haskell", "haskell-actions/setup", "ghc-version", "cabal-version"],
        ["dotnet"] = ["needs_dotnet", "setup-dotnet", "dotnet-version", "dotnet --version"],
    };

    private static readonly string[] UnsafeMarkers =
    [
        "./build-tool",
        "build-tool.exe",
        "-detect-languages",
        "-emit-plan",
        "-force",
        "-plan-file",
        "-validate-build-files",
        "actions/checkout",
        "build-plan",
        "cancel-in-progress:",
        "concurrency:",
        "diff-base",
        "download-artifact",
        "matrix:",
        "permissions:",
        "runs-on:",
        "strategy:",
        "upload-artifact",
    ];

    public static CiWorkflowChange AnalyzeChanges(string repoRoot, string diffBase) =>
        AnalyzePatch(GetFileDiff(repoRoot, diffBase, WorkflowPath));

    public static CiWorkflowChange AnalyzePatch(string patch)
    {
        var toolchains = new HashSet<string>(StringComparer.Ordinal);
        var hunk = new List<string>();

        CiWorkflowChange? Flush()
        {
            var classification = ClassifyHunk(hunk);
            hunk.Clear();
            if (classification.RequiresFullRebuild)
            {
                return classification;
            }

            foreach (var toolchain in classification.Toolchains)
            {
                toolchains.Add(toolchain);
            }

            return null;
        }

        foreach (var line in patch.Split('\n'))
        {
            if (line.StartsWith("@@", StringComparison.Ordinal))
            {
                var result = Flush();
                if (result is not null)
                {
                    return result;
                }

                continue;
            }

            if (line.StartsWith("diff --git ", StringComparison.Ordinal) ||
                line.StartsWith("index ", StringComparison.Ordinal) ||
                line.StartsWith("--- ", StringComparison.Ordinal) ||
                line.StartsWith("+++ ", StringComparison.Ordinal))
            {
                continue;
            }

            hunk.Add(line);
        }

        return Flush() ?? new CiWorkflowChange { Toolchains = toolchains, RequiresFullRebuild = false };
    }

    public static IReadOnlyList<string> SortedToolchains(HashSet<string> toolchains) =>
        toolchains.OrderBy(value => value, StringComparer.Ordinal).ToArray();

    private static CiWorkflowChange ClassifyHunk(IReadOnlyList<string> lines)
    {
        var hunkToolchains = new HashSet<string>(StringComparer.Ordinal);
        var changedToolchains = new HashSet<string>(StringComparer.Ordinal);
        var changedLines = new List<string>();

        foreach (var line in lines)
        {
            if (line.Length == 0 || !IsDiffLine(line))
            {
                continue;
            }

            var content = line[1..].Trim();
            foreach (var toolchain in DetectToolchains(content))
            {
                hunkToolchains.Add(toolchain);
            }

            if (!IsChangedLine(line) || content.Length == 0 || content.StartsWith('#'))
            {
                continue;
            }

            changedLines.Add(content);
            foreach (var toolchain in DetectToolchains(content))
            {
                changedToolchains.Add(toolchain);
            }
        }

        if (changedLines.Count == 0)
        {
            return new CiWorkflowChange();
        }

        var resolvedToolchains = changedToolchains.Count > 0
            ? changedToolchains
            : hunkToolchains.Count == 1
                ? hunkToolchains
                : [];
        if (resolvedToolchains.Count == 0)
        {
            return new CiWorkflowChange { RequiresFullRebuild = true };
        }

        foreach (var content in changedLines)
        {
            if (UnsafeMarkers.Any(marker => content.Contains(marker, StringComparison.OrdinalIgnoreCase)))
            {
                return new CiWorkflowChange { RequiresFullRebuild = true };
            }

            if (DetectToolchains(content).Count > 0 || IsToolchainScopedStructuralLine(content))
            {
                continue;
            }

            return new CiWorkflowChange { RequiresFullRebuild = true };
        }

        return new CiWorkflowChange { Toolchains = resolvedToolchains, RequiresFullRebuild = false };
    }

    private static HashSet<string> DetectToolchains(string content)
    {
        var normalized = content.ToLowerInvariant();
        var found = new HashSet<string>(StringComparer.Ordinal);
        foreach (var (toolchain, markers) in ToolchainMarkers)
        {
            if (markers.Any(marker => normalized.Contains(marker, StringComparison.Ordinal)))
            {
                found.Add(toolchain);
            }
        }

        return found;
    }

    private static bool IsToolchainScopedStructuralLine(string content)
    {
        foreach (var prefix in new[]
                 { "if:", "run:", "shell:", "with:", "env:", "{", "}", "else", "fi", "then", "printf ", "echo ", "curl ", "powershell ", "call ", "cd " })
        {
            if (content.StartsWith(prefix, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    private static bool IsDiffLine(string line) => line.StartsWith(' ') || IsChangedLine(line);

    private static bool IsChangedLine(string line) => line.StartsWith('+') || line.StartsWith('-');

    private static string GetFileDiff(string repoRoot, string diffBase, string relativePath)
    {
        foreach (var arguments in new[]
                 {
                     $"diff --unified=0 {diffBase}...HEAD -- {relativePath}",
                     $"diff --unified=0 {diffBase} HEAD -- {relativePath}",
                 })
        {
            var result = ProcessRunner.RunProcess("git", arguments, repoRoot, useShell: false);
            if (result.ReturnCode == 0)
            {
                return result.Stdout;
            }
        }

        return string.Empty;
    }
}

public static class Validator
{
    private static readonly HashSet<string> CiManagedToolchainLanguages = new(StringComparer.Ordinal)
    {
        "python", "ruby", "typescript", "rust", "elixir", "lua", "perl", "java", "kotlin", "haskell", "dart", "dotnet",
    };

    public static string? ValidateBuildContracts(string repoRoot, IReadOnlyList<PackageSpec> packages)
    {
        return ValidateCiFullBuildToolchains(repoRoot, packages);
    }

    public static string? ValidateCiFullBuildToolchains(string repoRoot, IReadOnlyList<PackageSpec> packages)
    {
        var ciPath = Path.Combine(repoRoot, ".github", "workflows", "ci.yml");
        if (!File.Exists(ciPath))
        {
            return null;
        }

        var workflow = File.ReadAllText(ciPath);
        if (!workflow.Contains("Full build on main merge", StringComparison.Ordinal))
        {
            return null;
        }

        var compactWorkflow = Regex.Replace(workflow, @"\s+", string.Empty);
        var missingOutputBinding = new List<string>();
        var missingMainForce = new List<string>();

        var languages = packages
            .Select(package => BuildToolApp.ToolchainForLanguage(package.Language))
            .Where(language => CiManagedToolchainLanguages.Contains(language))
            .Distinct(StringComparer.Ordinal)
            .OrderBy(language => language, StringComparer.Ordinal);

        foreach (var language in languages)
        {
            var outputBinding = $"needs_{language}:${{{{steps.toolchains.outputs.needs_{language}}}}}";
            if (!compactWorkflow.Contains(outputBinding, StringComparison.Ordinal))
            {
                missingOutputBinding.Add(language);
            }

            if (!compactWorkflow.Contains($"needs_{language}=true", StringComparison.Ordinal))
            {
                missingMainForce.Add(language);
            }
        }

        if (missingOutputBinding.Count == 0 && missingMainForce.Count == 0)
        {
            return null;
        }

        var parts = new List<string>();
        if (missingOutputBinding.Count > 0)
        {
            parts.Add($"detect outputs for forced main full builds are not normalized through steps.toolchains for: {string.Join(", ", missingOutputBinding)}");
        }

        if (missingMainForce.Count > 0)
        {
            parts.Add($"forced main full-build path does not explicitly enable toolchains for: {string.Join(", ", missingMainForce)}");
        }

        return $"{ciPath.Replace('\\', '/')}: {string.Join("; ", parts)}";
    }
}

public sealed class ProcessResult
{
    public required string Stdout { get; init; }
    public required string Stderr { get; init; }
    public required int ReturnCode { get; init; }
}

public static class ProcessRunner
{
    public static ProcessResult RunShellCommand(string command, string workingDirectory)
    {
        if (OperatingSystem.IsWindows())
        {
            return RunProcess("cmd", $"/c {command}", workingDirectory, useShell: false);
        }

        return RunProcess("/bin/sh", $"-lc \"{command.Replace("\"", "\\\"", StringComparison.Ordinal)}\"", workingDirectory, useShell: false);
    }

    public static ProcessResult RunProcess(string fileName, string arguments, string workingDirectory, bool useShell)
    {
        using var process = new Process();
        process.StartInfo = new ProcessStartInfo
        {
            FileName = fileName,
            Arguments = arguments,
            WorkingDirectory = workingDirectory,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = useShell,
            CreateNoWindow = true,
        };

        process.Start();
        var stdout = process.StandardOutput.ReadToEnd();
        var stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();

        return new ProcessResult
        {
            Stdout = stdout,
            Stderr = stderr,
            ReturnCode = process.ExitCode,
        };
    }
}

public sealed class ExecuteBuildsOptions
{
    public required IReadOnlyList<PackageSpec> Packages { get; init; }
    public required DirectedGraph Graph { get; init; }
    public required BuildCache Cache { get; init; }
    public required IReadOnlyDictionary<string, string> PackageHashes { get; init; }
    public required IReadOnlyDictionary<string, string> DependencyHashes { get; init; }
    public bool Force { get; init; }
    public bool DryRun { get; init; }
    public int? MaxJobs { get; init; }
    public HashSet<string>? AffectedSet { get; init; }
}

public static class Executor
{
    public static async Task<Dictionary<string, BuildResult>> ExecuteBuildsAsync(ExecuteBuildsOptions options)
    {
        var packagesByName = options.Packages.ToDictionary(package => package.Name, StringComparer.Ordinal);
        var results = new Dictionary<string, BuildResult>(StringComparer.Ordinal);
        var groups = options.Graph.IndependentGroups();
        var maxJobs = Math.Max(1, options.MaxJobs ?? Environment.ProcessorCount);

        foreach (var group in groups)
        {
            var ready = new List<PackageSpec>();
            foreach (var packageName in group)
            {
                if (!packagesByName.TryGetValue(packageName, out var package))
                {
                    continue;
                }

                var dependencyFailed = options.Graph
                    .Predecessors(packageName)
                    .Where(results.ContainsKey)
                    .Select(name => results[name])
                    .Any(result => result.Status is "failed" or "dep-skipped");
                if (dependencyFailed)
                {
                    results[packageName] = new BuildResult(packageName, "dep-skipped", 0, string.Empty, string.Empty, 0);
                    continue;
                }

                var needsBuild = options.Force
                    || options.AffectedSet is not null && options.AffectedSet.Contains(packageName)
                    || options.AffectedSet is null &&
                       options.Cache.NeedsBuild(packageName, options.PackageHashes[packageName], options.DependencyHashes[packageName]);

                if (!needsBuild)
                {
                    results[packageName] = new BuildResult(packageName, "skipped", 0, string.Empty, string.Empty, 0);
                    continue;
                }

                if (options.DryRun)
                {
                    results[packageName] = new BuildResult(packageName, "would-build", 0, string.Empty, string.Empty, 0);
                    continue;
                }

                ready.Add(package);
            }

            using var semaphore = new SemaphoreSlim(maxJobs);
            var tasks = ready.Select(async package =>
            {
                await semaphore.WaitAsync().ConfigureAwait(false);
                try
                {
                    return await RunPackageBuildAsync(package).ConfigureAwait(false);
                }
                finally
                {
                    semaphore.Release();
                }
            }).ToArray();

            foreach (var result in await Task.WhenAll(tasks).ConfigureAwait(false))
            {
                results[result.PackageName] = result;
                options.Cache.Record(
                    result.PackageName,
                    options.PackageHashes[result.PackageName],
                    options.DependencyHashes[result.PackageName],
                    result.Status == "built" ? "success" : "failed");
            }
        }

        return results;
    }

    private static async Task<BuildResult> RunPackageBuildAsync(PackageSpec package)
    {
        var stopwatch = Stopwatch.StartNew();
        var stdout = new StringBuilder();
        var stderr = new StringBuilder();

        foreach (var command in package.BuildCommands)
        {
            var result = await Task.Run(() => ProcessRunner.RunShellCommand(command, package.Path)).ConfigureAwait(false);
            stdout.Append(result.Stdout);
            stderr.Append(result.Stderr);
            if (result.ReturnCode != 0)
            {
                stopwatch.Stop();
                return new BuildResult(
                    package.Name,
                    "failed",
                    stopwatch.Elapsed.TotalSeconds,
                    stdout.ToString(),
                    stderr.ToString(),
                    result.ReturnCode);
            }
        }

        stopwatch.Stop();
        return new BuildResult(package.Name, "built", stopwatch.Elapsed.TotalSeconds, stdout.ToString(), stderr.ToString(), 0);
    }
}

public static class BuildToolApp
{
    public static readonly string[] AllToolchains =
    [
        "python", "ruby", "go", "typescript", "rust", "elixir", "lua", "perl",
        "swift", "dart", "java", "kotlin", "haskell", "dotnet",
    ];

    public static string ToolchainForLanguage(string language) =>
        language switch
        {
            "wasm" => "rust",
            "csharp" or "fsharp" or "dotnet" => "dotnet",
            _ => language,
        };

    public static async Task<int> RunAsync(string[] args)
    {
        CliOptions options;
        try
        {
            options = Cli.Parse(args);
        }
        catch (ArgumentException exception)
        {
            Console.Error.WriteLine($"Error: {exception.Message}");
            Console.Error.WriteLine(Cli.HelpText());
            return 1;
        }

        if (options.Help)
        {
            Console.WriteLine(Cli.HelpText());
            return 0;
        }

        var repoRoot = options.Root is not null
            ? Path.GetFullPath(options.Root)
            : FindRepoRoot();
        if (repoRoot is null)
        {
            Console.Error.WriteLine("Error: Could not find repo root (.git directory).");
            Console.Error.WriteLine("Use --root to specify the repo root.");
            return 1;
        }

        var codeRoot = Path.Combine(repoRoot, "code");
        if (!Directory.Exists(codeRoot))
        {
            Console.Error.WriteLine($"Error: {codeRoot} does not exist.");
            return 1;
        }

        var packages = Discovery.DiscoverPackages(codeRoot);
        if (packages.Count == 0)
        {
            Console.Error.WriteLine("No packages found.");
            return 0;
        }

        if (!string.Equals(options.Language, "all", StringComparison.Ordinal))
        {
            packages = packages
                .Where(package => package.Language == options.Language)
                .ToArray();
            if (packages.Count == 0)
            {
                Console.Error.WriteLine($"No {options.Language} packages found.");
                return 0;
            }
        }

        if (options.ValidateBuildFiles)
        {
            var validationError = Validator.ValidateBuildContracts(repoRoot, packages);
            if (validationError is not null)
            {
                Console.Error.WriteLine("BUILD/CI validation failed:");
                Console.Error.WriteLine($"  - {validationError}");
                Console.Error.WriteLine("Fix the BUILD file or CI workflow so isolated and full-build runs stay correct.");
                return 1;
            }
        }

        Console.WriteLine($"Discovered {packages.Count} packages");

        var graph = Resolver.ResolveDependencies(packages);
        HashSet<string>? affectedSet = null;
        var ciToolchains = new HashSet<string>(StringComparer.Ordinal);
        var force = options.Force;

        if (!force)
        {
            var changedFiles = GitDiff.GetChangedFiles(repoRoot, options.DiffBase);
            if (changedFiles.Count > 0)
            {
                if (changedFiles.Contains(CiWorkflow.WorkflowPath, StringComparer.Ordinal))
                {
                    var ciChange = CiWorkflow.AnalyzeChanges(repoRoot, options.DiffBase);
                    if (ciChange.RequiresFullRebuild)
                    {
                        Console.WriteLine("Git diff: ci.yml changed in shared ways -- rebuilding everything");
                        force = true;
                    }
                    else
                    {
                        ciToolchains = ciChange.Toolchains;
                        if (ciToolchains.Count > 0)
                        {
                            Console.WriteLine($"Git diff: ci.yml changed only toolchain-scoped setup for {string.Join(", ", CiWorkflow.SortedToolchains(ciToolchains))}");
                        }
                    }
                }

                if (!force)
                {
                    var changedPackages = GitDiff.MapFilesToPackages(changedFiles, packages, repoRoot);
                    affectedSet = changedPackages.Count > 0
                        ? graph.AffectedNodes(changedPackages)
                        : [];
                    Console.WriteLine(changedPackages.Count > 0
                        ? $"Git diff: {changedPackages.Count} packages changed, {affectedSet.Count} affected (including dependents)"
                        : "Git diff: no package files changed -- nothing to build");
                }
            }
            else
            {
                Console.WriteLine("Git diff unavailable -- falling back to hash-based cache");
            }
        }

        var languagesNeeded = AllToolchains.ToDictionary(toolchain => toolchain, _ => false, StringComparer.Ordinal);
        languagesNeeded["go"] = true;
        if (force || affectedSet is null)
        {
            foreach (var toolchain in AllToolchains)
            {
                languagesNeeded[toolchain] = true;
            }
        }
        else
        {
            foreach (var package in packages.Where(package => affectedSet.Contains(package.Name)))
            {
                languagesNeeded[ToolchainForLanguage(package.Language)] = true;
            }

            foreach (var toolchain in ciToolchains)
            {
                languagesNeeded[toolchain] = true;
            }
        }

        if (options.EmitPlan)
        {
            var planPath = Path.IsPathRooted(options.PlanFile)
                ? options.PlanFile
                : Path.Combine(repoRoot, options.PlanFile);
            var plan = new BuildPlan
            {
                SchemaVersion = PlanFile.CurrentSchemaVersion,
                DiffBase = options.DiffBase,
                Force = force,
                AffectedPackages = affectedSet?.OrderBy(name => name, StringComparer.Ordinal).ToList(),
                Packages = packages
                    .Select(package => new BuildPlanPackageEntry
                    {
                        Name = package.Name,
                        RelativePath = Path.GetRelativePath(repoRoot, package.Path).Replace('\\', '/'),
                        Language = package.Language,
                        BuildCommands = package.BuildCommands.ToList(),
                    })
                    .ToList(),
                DependencyEdges = graph.Edges().Select(edge => new List<string> { edge.From, edge.To }).ToList(),
                LanguagesNeeded = languagesNeeded,
            };

            PlanFile.Write(plan, planPath);
            Console.WriteLine($"Build plan written to {planPath}");
            if (options.DetectLanguages)
            {
                OutputLanguageFlags(languagesNeeded);
            }

            return 0;
        }

        if (options.DetectLanguages)
        {
            OutputLanguageFlags(languagesNeeded);
            return 0;
        }

        var packageHashes = packages.ToDictionary(package => package.Name, Hasher.HashPackage, StringComparer.Ordinal);
        var dependencyHashes = packages.ToDictionary(
            package => package.Name,
            package => Hasher.HashDependencies(package.Name, graph, packageHashes),
            StringComparer.Ordinal);

        var cachePath = Path.IsPathRooted(options.CacheFile)
            ? options.CacheFile
            : Path.Combine(repoRoot, options.CacheFile);
        var cache = new BuildCache();
        cache.Load(cachePath);

        var results = await Executor.ExecuteBuildsAsync(new ExecuteBuildsOptions
        {
            Packages = packages,
            Graph = graph,
            Cache = cache,
            PackageHashes = packageHashes,
            DependencyHashes = dependencyHashes,
            Force = force,
            DryRun = options.DryRun,
            MaxJobs = options.Jobs,
            AffectedSet = affectedSet,
        }).ConfigureAwait(false);

        if (!options.DryRun)
        {
            cache.Save(cachePath);
        }

        Reporter.PrintReport(results);
        return results.Values.Any(result => result.Status == "failed") ? 1 : 0;
    }

    public static string? FindRepoRoot(string? start = null)
    {
        var current = Path.GetFullPath(start ?? Environment.CurrentDirectory);
        while (true)
        {
            if (Directory.Exists(Path.Combine(current, ".git")))
            {
                return current;
            }

            var parent = Directory.GetParent(current);
            if (parent is null)
            {
                return null;
            }

            current = parent.FullName;
        }
    }

    private static void OutputLanguageFlags(IReadOnlyDictionary<string, bool> languagesNeeded)
    {
        var githubOutput = Environment.GetEnvironmentVariable("GITHUB_OUTPUT");
        foreach (var toolchain in AllToolchains)
        {
            var line = $"needs_{toolchain}={(languagesNeeded.TryGetValue(toolchain, out var needed) && needed ? "true" : "false")}";
            Console.WriteLine(line);
            if (!string.IsNullOrWhiteSpace(githubOutput))
            {
                File.AppendAllText(githubOutput, line + Environment.NewLine);
            }
        }
    }
}
