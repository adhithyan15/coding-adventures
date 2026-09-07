namespace CodingAdventures.BuildTool.CSharp;

using System.Buffers.Binary;
using System.Diagnostics;
using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

public sealed record SourceCollectionCandidate(
    string Path,
    string Kind,
    bool Tracked,
    byte[] Content);

public sealed record SourceCollectionRequest(
    string Language,
    string PackageRoot,
    string Mode,
    string RegistrySha256,
    IReadOnlyList<string> DeclaredSources,
    IReadOnlyList<SourceCollectionCandidate> Candidates);

public sealed record SelectedSourceFile(string Path, string Digest);

public sealed record PackageHashInput(string Path, byte[] Content);

public sealed record SourceHashLimits(
    int MaximumCandidates,
    int MaximumSelectedInputs,
    ulong MaximumFileBytes,
    ulong MaximumPackageBytes)
{
    public static SourceHashLimits Default { get; } = new(100_000, 50_000, 64UL * 1024 * 1024, 1024UL * 1024 * 1024);
}

public sealed class SourceHashException : Exception
{
    public SourceHashException(string code)
        : base(code)
    {
        Code = code;
    }

    internal SourceHashException(string code, string message)
        : base(message)
    {
        Code = code;
    }

    public string Code { get; }
}

internal sealed class LanguageSourceInputRegistry
{
    [JsonPropertyName("schema_version")]
    public int SchemaVersion { get; init; }

    [JsonPropertyName("universal_inputs")]
    public required UniversalSourceInputs UniversalInputs { get; init; }

    [JsonPropertyName("languages")]
    public required List<LanguageSourceInputs> Languages { get; init; }
}

internal sealed class UniversalSourceInputs
{
    [JsonPropertyName("build_filenames")]
    public required List<string> BuildFilenames { get; init; }

    [JsonPropertyName("generated_directory_components")]
    public required List<string> GeneratedDirectoryComponents { get; init; }

    [JsonPropertyName("root_exact_basenames")]
    public required List<string> RootExactBasenames { get; init; }
}

internal sealed class LanguageSourceInputs
{
    [JsonPropertyName("language")]
    public required string Language { get; init; }

    [JsonPropertyName("recursive_suffixes")]
    public required List<string> RecursiveSuffixes { get; init; }

    [JsonPropertyName("recursive_exact_basenames")]
    public required List<string> RecursiveExactBasenames { get; init; }

    [JsonPropertyName("root_exact_basenames")]
    public required List<string> RootExactBasenames { get; init; }

    [JsonPropertyName("root_variable_suffixes")]
    public required List<string> RootVariableSuffixes { get; init; }

    [JsonPropertyName("root_exact_relative_paths")]
    public required List<string> RootExactRelativePaths { get; init; }

    [JsonPropertyName("package_exact_inputs")]
    public required List<PackageExactSourceInputs> PackageExactInputs { get; init; }

    [JsonPropertyName("case_alias_groups")]
    public required List<List<string>> CaseAliasGroups { get; init; }

    [JsonPropertyName("scoped_inputs")]
    public required List<ScopedSourceInputs> ScopedInputs { get; init; }
}

internal sealed class PackageExactSourceInputs
{
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    [JsonPropertyName("package_root")]
    public required string PackageRoot { get; init; }

    [JsonPropertyName("paths")]
    public required List<string> Paths { get; init; }

    [JsonPropertyName("reason")]
    public required string Reason { get; init; }

    [JsonPropertyName("owner")]
    public required string Owner { get; init; }
}

internal sealed class ScopedSourceInputs
{
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    [JsonPropertyName("role")]
    public required string Role { get; init; }

    [JsonPropertyName("decision")]
    public required string Decision { get; init; }

    [JsonPropertyName("scope")]
    public required string Scope { get; init; }

    [JsonPropertyName("path_prefix")]
    public string? PathPrefix { get; init; }

    [JsonPropertyName("suffixes")]
    public required List<string> Suffixes { get; init; }

    [JsonPropertyName("exact_basenames")]
    public required List<string> ExactBasenames { get; init; }

    [JsonPropertyName("reason")]
    public required string Reason { get; init; }

    [JsonPropertyName("owner")]
    public required string Owner { get; init; }
}

internal sealed class RepositorySourceInputBoundaryRegistry
{
    [JsonPropertyName("schema_version")]
    public int SchemaVersion { get; init; }

    [JsonPropertyName("language_source_input_registry_sha256")]
    public required string LanguageSourceInputRegistrySha256 { get; init; }

    [JsonPropertyName("boundaries")]
    public required List<RepositorySourceBoundary> Boundaries { get; init; }
}

internal sealed class RepositorySourceBoundary
{
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    [JsonPropertyName("input_origin")]
    public required string InputOrigin { get; init; }

    [JsonPropertyName("applies_to")]
    public required RepositorySourceApplicability AppliesTo { get; init; }

    [JsonPropertyName("inputs")]
    public required List<RepositorySourceBoundaryInput> Inputs { get; init; }

    [JsonPropertyName("reason")]
    public required string Reason { get; init; }

    [JsonPropertyName("owner")]
    public required string Owner { get; init; }
}

internal sealed class RepositorySourceApplicability
{
    [JsonPropertyName("exact_roots")]
    public required List<string> ExactRoots { get; init; }

    [JsonPropertyName("descendant_roots")]
    public required List<string> DescendantRoots { get; init; }

    [JsonPropertyName("excluded_roots")]
    public required List<string> ExcludedRoots { get; init; }
}

internal sealed class RepositorySourceBoundaryInput
{
    [JsonPropertyName("path")]
    public required string Path { get; init; }

    [JsonPropertyName("role")]
    public required string Role { get; init; }

    [JsonPropertyName("generated_component")]
    public string? GeneratedComponent { get; init; }
}

internal sealed record TrackedGitFile(string Mode, string ObjectId, int Stage, string Path);

internal sealed record PackageSourceCollection(IReadOnlyList<string> Files);

public static partial class Hasher
{
    private const int MaximumDeclaredSourcePatterns = 256;
    private const int MaximumDeclaredSourcePatternBytes = 64 * 1024;
    private const ulong MaximumGlobMatchWork = 50_000_000;

    public const string LanguageSourceInputRegistryDigest =
        "f49bfe8c7c9c0fb9b534ecc9ca4a614f3684abe32bdb0edac82d99bdc806fb70";
    public const string RepositorySourceInputBoundaryDigest =
        "963cc4090e165752fd3a62921b699dfff8f0677b49d7236812398a8abed0a25f";

    private static readonly JsonSerializerOptions RegistryJsonOptions = new()
    {
        PropertyNameCaseInsensitive = false,
        UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };
    private static readonly LanguageSourceInputRegistry LanguageRegistry =
        JsonSerializer.Deserialize<LanguageSourceInputRegistry>(
            SourceInputRegistryProjection.LanguageJson,
            RegistryJsonOptions)
        ?? throw new InvalidOperationException("checked language source-input projection is invalid");
    private static readonly RepositorySourceInputBoundaryRegistry BoundaryRegistry =
        JsonSerializer.Deserialize<RepositorySourceInputBoundaryRegistry>(
            SourceInputRegistryProjection.RepositoryBoundaryJson,
            RegistryJsonOptions)
        ?? throw new InvalidOperationException("checked repository source-input projection is invalid");
    private static readonly Dictionary<string, LanguageSourceInputs> LanguageInputs =
        LanguageRegistry.Languages.ToDictionary(entry => entry.Language, StringComparer.Ordinal);
    private static readonly HashSet<string> GeneratedDirectoryComponents =
        new(LanguageRegistry.UniversalInputs.GeneratedDirectoryComponents, StringComparer.Ordinal);
    private static readonly HashSet<string> WindowsReservedBasenames = BuildWindowsReservedBasenames();
    private static readonly bool EmbeddedRegistriesValidated = ValidateEmbeddedRegistries();

    public static string LanguageSourceInputRegistryJson =>
        JsonSerializer.Serialize(LanguageRegistry, RegistryJsonOptions);

    public static string RepositorySourceInputBoundaryJson =>
        JsonSerializer.Serialize(BoundaryRegistry, RegistryJsonOptions);

    public static string CanonicalLanguageSourceInputRegistryDigest(string json) =>
        CanonicalRegistryDigest(
            json,
            "coding-adventures/build-tool-language-source-input-registry/v1");

    public static string CanonicalRepositorySourceInputBoundaryDigest(string json) =>
        CanonicalRegistryDigest(
            json,
            "coding-adventures/build-tool-repository-source-input-boundary/v1");

    private static bool ValidateEmbeddedRegistries()
    {
        if (LanguageRegistry.SchemaVersion != 1 ||
            BoundaryRegistry.SchemaVersion != 1 ||
            !string.Equals(
                BoundaryRegistry.LanguageSourceInputRegistrySha256,
                LanguageSourceInputRegistryDigest,
                StringComparison.Ordinal) ||
            !string.Equals(
                CanonicalLanguageSourceInputRegistryDigest(SourceInputRegistryProjection.LanguageJson),
                LanguageSourceInputRegistryDigest,
                StringComparison.Ordinal) ||
            !string.Equals(
                CanonicalRepositorySourceInputBoundaryDigest(SourceInputRegistryProjection.RepositoryBoundaryJson),
                RepositorySourceInputBoundaryDigest,
                StringComparison.Ordinal))
        {
            throw new InvalidOperationException("checked source-input registry projection is invalid");
        }
        return true;
    }

    public static IReadOnlyList<SelectedSourceFile> SelectSourceCandidates(SourceCollectionRequest request) =>
        SelectSourceCandidates(request, SourceHashLimits.Default);

    public static IReadOnlyList<SelectedSourceFile> SelectSourceCandidates(
        SourceCollectionRequest request,
        SourceHashLimits limits)
    {
        ValidateLimits(limits);
        ArgumentNullException.ThrowIfNull(request);
        ArgumentNullException.ThrowIfNull(request.Candidates);
        ArgumentNullException.ThrowIfNull(request.DeclaredSources);

        if (!LanguageInputs.TryGetValue(request.Language, out var language))
        {
            throw new SourceHashException("SOURCE_HASH_LANGUAGE_UNKNOWN");
        }
        ValidatePackageRoot(request.PackageRoot, request.Language);
        if (request.Candidates.Count > limits.MaximumCandidates)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
        }

        return request.Mode switch
        {
            "extension" or "declared_sources" => SelectPackageCandidates(request, language, limits),
            "repository_boundary" => SelectBoundaryCandidates(request, limits),
            _ => throw new SourceHashException("SOURCE_HASH_MODE_INVALID"),
        };
    }

    public static string HashPackageInputs(IReadOnlyList<PackageHashInput> inputs) =>
        HashPackageInputs(inputs, SourceHashLimits.Default);

    public static string HashPackageInputs(
        IReadOnlyList<PackageHashInput> inputs,
        SourceHashLimits limits)
    {
        ValidateLimits(limits);
        ArgumentNullException.ThrowIfNull(inputs);
        if (inputs.Count > limits.MaximumSelectedInputs)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
        }

        var identities = new Dictionary<string, string>(StringComparer.Ordinal);
        var exactPaths = new HashSet<string>(StringComparer.Ordinal);
        ulong totalBytes = 0;
        foreach (var input in inputs)
        {
            ArgumentNullException.ThrowIfNull(input);
            ArgumentNullException.ThrowIfNull(input.Content);
            ValidatePortablePath(input.Path);
            RegisterPortableIdentity(input.Path, exactPaths, identities);
            totalBytes = CheckedByteTotal(totalBytes, (ulong)input.Content.LongLength, limits);
        }
        ValidateFilePathTopology(inputs.Select(input => input.Path));

        using var digest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        foreach (var input in inputs.OrderBy(input => input.Path, Utf8StringComparer.Instance))
        {
            AppendFrame(digest, Encoding.UTF8.GetBytes(input.Path));
            AppendFrame(digest, input.Content);
        }
        return Convert.ToHexString(digest.GetHashAndReset()).ToLowerInvariant();
    }

    public static IReadOnlyList<string> CollectSourceFiles(PackageSpec package)
    {
        var repositoryRoot = FindRepositoryRoot(package.Path);
        using var secureScope = SecureSourceFileReader.RetainRepositoryRoot(repositoryRoot);
        var files = CollectPackageLocalFiles(
            package,
            repositoryRoot,
            SourceHashLimits.Default,
            secureScope).Files;
        secureScope.Validate();
        return files;
    }

    internal static IReadOnlyList<string> CollectSourceFiles(PackageSpec package, SourceHashLimits limits)
    {
        ValidateLimits(limits);
        var repositoryRoot = FindRepositoryRoot(package.Path);
        using var secureScope = SecureSourceFileReader.RetainRepositoryRoot(repositoryRoot);
        var files = CollectPackageLocalFiles(package, repositoryRoot, limits, secureScope).Files;
        secureScope.Validate();
        return files;
    }

    public static string HashFile(string filePath)
    {
        var root = Path.GetDirectoryName(Path.GetFullPath(filePath))
            ?? throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        using var secureScope = SecureSourceFileReader.RetainRepositoryRoot(root);
        var snapshot = secureScope.ReadFile(
            filePath,
            SourceHashLimits.Default.MaximumFileBytes,
            SourceHashLimits.Default.MaximumFileBytes);
        secureScope.Validate();
        return LowerSha256(snapshot.Content);
    }

    public static string HashPackage(PackageSpec package)
    {
        var repositoryRoot = FindRepositoryRoot(package.Path);
        return HashPackage(package, repositoryRoot, null);
    }

    internal static string HashPackage(
        PackageSpec package,
        string repositoryRoot,
        IReadOnlyDictionary<string, TrackedGitFile>? trackedBoundaryInputs)
    {
        try
        {
            var limits = SourceHashLimits.Default;
            using var secureScope = SecureSourceFileReader.RetainRepositoryRoot(repositoryRoot);
            var localCollection = CollectPackageLocalFiles(package, repositoryRoot, limits, secureScope);
            var packageRoot = PortableRelativePath(repositoryRoot, package.Path);
            ValidatePackageRoot(packageRoot, package.Language);
            var allFiles = new Dictionary<string, string>(StringComparer.Ordinal);
            foreach (var file in localCollection.Files)
            {
                allFiles.Add(PortableRelativePath(repositoryRoot, file), file);
            }

            foreach (var boundaryPath in BoundaryPaths(packageRoot))
            {
                if (trackedBoundaryInputs is null ||
                    !trackedBoundaryInputs.TryGetValue(boundaryPath, out var tracked) ||
                    tracked.Stage != 0 ||
                    tracked.Mode is not ("100644" or "100755"))
                {
                    throw new SourceHashException("SOURCE_HASH_TRACKED_INPUT_UNAVAILABLE");
                }
                var fullPath = ContainedPath(repositoryRoot, boundaryPath);
                if (allFiles.TryGetValue(boundaryPath, out var existing) &&
                    !string.Equals(existing, fullPath, StringComparison.Ordinal))
                {
                    throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
                }
                allFiles[boundaryPath] = fullPath;
            }

            if (allFiles.Count > limits.MaximumSelectedInputs)
            {
                throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
            }
            var exactPaths = new HashSet<string>(StringComparer.Ordinal);
            var identities = new Dictionary<string, string>(StringComparer.Ordinal);
            foreach (var path in allFiles.Keys)
            {
                ValidatePortablePath(path);
                RegisterPortableIdentity(path, exactPaths, identities);
            }
            ValidateFilePathTopology(allFiles.Keys);

            using var digest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
            var fileStates = new List<(string Path, SecureObjectState State)>(allFiles.Count);
            var expectedBuildPath = package.BuildFileName is null
                ? null
                : $"{packageRoot}/{package.BuildFileName}";
            if ((package.BuildFileName is null) != (package.BuildFileSha256 is null) ||
                (package.BuildFileName is not null &&
                 (package.BuildFileName.Contains('/') || package.BuildFileName.Contains('\\'))))
            {
                throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
            }
            var buildFileBound = expectedBuildPath is null;
            ulong totalBytes = 0;
            foreach (var pair in allFiles.OrderBy(pair => pair.Key, Utf8StringComparer.Instance))
            {
                var remaining = limits.MaximumPackageBytes - totalBytes;
                var snapshot = secureScope.ReadFile(
                    pair.Value,
                    limits.MaximumFileBytes,
                    remaining);
                totalBytes = CheckedByteTotal(totalBytes, (ulong)snapshot.Content.LongLength, limits);
                if (string.Equals(pair.Key, expectedBuildPath, StringComparison.Ordinal))
                {
                    if (!string.Equals(
                            LowerSha256(snapshot.Content),
                            package.BuildFileSha256,
                            StringComparison.Ordinal))
                    {
                        throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                    }
                    buildFileBound = true;
                }
                AppendFrame(digest, Encoding.UTF8.GetBytes(pair.Key));
                AppendFrame(digest, snapshot.Content);
                fileStates.Add((pair.Value, snapshot.State));
            }

            foreach (var file in fileStates)
            {
                if (secureScope.FileState(file.Path) != file.State)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
            }
            if (!buildFileBound)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            secureScope.Validate();
            return Convert.ToHexString(digest.GetHashAndReset()).ToLowerInvariant();
        }
        catch (SourceHashException)
        {
            throw new SourceHashException(
                "HASH_PACKAGE_FAILED",
                $"HASH_PACKAGE_FAILED: package={QuoteDiagnostic(package.Name)}");
        }
        catch
        {
            throw new SourceHashException(
                "HASH_PACKAGE_FAILED",
                $"HASH_PACKAGE_FAILED: package={QuoteDiagnostic(package.Name)}");
        }
    }

    internal static IReadOnlyDictionary<string, TrackedGitFile> CaptureTrackedBoundarySnapshot(
        string repositoryRoot,
        IReadOnlyList<PackageSpec> packages)
    {
        var paths = packages
            .SelectMany(package => BoundaryPaths(PortableRelativePath(repositoryRoot, package.Path)))
            .Distinct(StringComparer.Ordinal)
            .OrderBy(path => path, Utf8StringComparer.Instance)
            .ToArray();
        if (paths.Length == 0)
        {
            return new Dictionary<string, TrackedGitFile>(StringComparer.Ordinal);
        }
        if (paths.Length > 256)
        {
            throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
        }

        string stdout;
        try
        {
            stdout = StrictUtf8.GetString(RunBoundedGitIndexQuery(repositoryRoot, paths));
        }
        catch
        {
            throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
        }

        var requested = new HashSet<string>(paths, StringComparer.Ordinal);
        var result = new Dictionary<string, TrackedGitFile>(StringComparer.Ordinal);
        foreach (var record in stdout.Split('\0', StringSplitOptions.RemoveEmptyEntries))
        {
            var tab = record.IndexOf('\t');
            if (tab <= 0)
            {
                throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
            }
            var fields = record[..tab].Split(' ', StringSplitOptions.RemoveEmptyEntries);
            var path = record[(tab + 1)..];
            if (fields.Length != 3 ||
                !int.TryParse(fields[2], NumberStyles.None, CultureInfo.InvariantCulture, out var stage) ||
                stage != 0 ||
                fields[0] is not ("100644" or "100755") ||
                !Regex.IsMatch(fields[1], "^[0-9a-f]{40,64}$", RegexOptions.CultureInvariant) ||
                !requested.Contains(path) ||
                result.ContainsKey(path))
            {
                throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
            }
            result.Add(path, new TrackedGitFile(fields[0], fields[1], stage, path));
        }
        if (result.Count != paths.Length)
        {
            throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
        }
        return result;
    }

    private static readonly UTF8Encoding StrictUtf8 = new(
        encoderShouldEmitUTF8Identifier: false,
        throwOnInvalidBytes: true);

    private static byte[] RunBoundedGitIndexQuery(string repositoryRoot, IReadOnlyList<string> paths)
    {
        using var process = new Process();
        var startInfo = new ProcessStartInfo
        {
            FileName = FindTrustedGitExecutable(),
            WorkingDirectory = Path.GetFullPath(repositoryRoot),
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        startInfo.Environment.Clear();
        CopyRequiredWindowsEnvironment(startInfo, "SystemRoot");
        CopyRequiredWindowsEnvironment(startInfo, "WINDIR");
        CopyRequiredWindowsEnvironment(startInfo, "ComSpec");
        startInfo.Environment["GIT_CONFIG_NOSYSTEM"] = "1";
        startInfo.Environment["GIT_CONFIG_GLOBAL"] = OperatingSystem.IsWindows() ? "NUL" : "/dev/null";
        startInfo.Environment["GIT_OPTIONAL_LOCKS"] = "0";
        startInfo.Environment["GIT_TERMINAL_PROMPT"] = "0";
        startInfo.Environment["LC_ALL"] = "C";
        startInfo.Environment["LANG"] = "C";
        startInfo.ArgumentList.Add("ls-files");
        startInfo.ArgumentList.Add("--stage");
        startInfo.ArgumentList.Add("-z");
        startInfo.ArgumentList.Add("--");
        foreach (var path in paths)
        {
            startInfo.ArgumentList.Add(path);
        }
        process.StartInfo = startInfo;

        try
        {
            if (!process.Start())
            {
                throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
            }
            using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            var stdoutTask = ReadBoundedStream(process.StandardOutput.BaseStream, 1024 * 1024, cancellation.Token);
            var stderrTask = ReadBoundedStream(process.StandardError.BaseStream, 1024 * 1024, cancellation.Token);
            process.WaitForExitAsync(cancellation.Token).GetAwaiter().GetResult();
            var stdout = stdoutTask.GetAwaiter().GetResult();
            _ = stderrTask.GetAwaiter().GetResult();
            if (process.ExitCode != 0)
            {
                throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
            }
            return stdout;
        }
        catch
        {
            if (!process.HasExited)
            {
                process.Kill(entireProcessTree: true);
                process.WaitForExit();
            }
            throw;
        }
    }

    private static async Task<byte[]> ReadBoundedStream(
        Stream stream,
        int maximumBytes,
        CancellationToken cancellationToken)
    {
        using var output = new MemoryStream();
        var buffer = new byte[16 * 1024];
        while (true)
        {
            var count = await stream.ReadAsync(buffer, cancellationToken).ConfigureAwait(false);
            if (count == 0)
            {
                return output.ToArray();
            }
            if (output.Length + count > maximumBytes)
            {
                throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
            }
            output.Write(buffer, 0, count);
        }
    }

    private static string FindTrustedGitExecutable()
    {
        var candidates = new List<string>();
        if (OperatingSystem.IsWindows())
        {
            foreach (var root in new[]
            {
                Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
                Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            })
            {
                if (string.IsNullOrWhiteSpace(root))
                {
                    continue;
                }
                candidates.Add(Path.Combine(root, "Git", "cmd", "git.exe"));
                candidates.Add(Path.Combine(root, "Git", "bin", "git.exe"));
                candidates.Add(Path.Combine(root, "Programs", "Git", "cmd", "git.exe"));
            }
        }
        else
        {
            candidates.AddRange(["/usr/bin/git", "/usr/local/bin/git", "/opt/homebrew/bin/git", "/opt/local/bin/git"]);
        }

        return candidates.FirstOrDefault(File.Exists)
            ?? throw new SourceHashException("SOURCE_HASH_TRACKED_SNAPSHOT_FAILED");
    }

    private static void CopyRequiredWindowsEnvironment(ProcessStartInfo startInfo, string name)
    {
        if (!OperatingSystem.IsWindows())
        {
            return;
        }
        var value = Environment.GetEnvironmentVariable(name);
        if (!string.IsNullOrEmpty(value))
        {
            startInfo.Environment[name] = value;
        }
    }

    internal static bool TrackedSnapshotsEqual(
        IReadOnlyDictionary<string, TrackedGitFile> left,
        IReadOnlyDictionary<string, TrackedGitFile> right)
    {
        return left.Count == right.Count && left.All(pair =>
            right.TryGetValue(pair.Key, out var value) && pair.Value == value);
    }

    internal static bool BoundaryInputAppliesTo(string inputPath, string packageRoot) =>
        BoundaryRegistry.Boundaries.Any(boundary =>
            boundary.Inputs.Any(input => string.Equals(input.Path, inputPath, StringComparison.Ordinal)) &&
            BoundaryApplies(boundary, packageRoot));

    private static IReadOnlyList<SelectedSourceFile> SelectPackageCandidates(
        SourceCollectionRequest request,
        LanguageSourceInputs language,
        SourceHashLimits limits)
    {
        if (!string.Equals(request.RegistrySha256, LanguageSourceInputRegistryDigest, StringComparison.Ordinal))
        {
            throw new SourceHashException("SOURCE_HASH_REGISTRY_MISMATCH");
        }
        if (request.DeclaredSources.Count > MaximumDeclaredSourcePatterns)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
        }
        foreach (var pattern in request.DeclaredSources)
        {
            ValidatePortableGlob(pattern);
        }
        if (request.DeclaredSources.Sum(pattern => Encoding.UTF8.GetByteCount(pattern)) >
                MaximumDeclaredSourcePatternBytes)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
        }

        var linkBoundaries = new HashSet<string>(StringComparer.Ordinal);
        var identities = new Dictionary<string, string>(StringComparer.Ordinal);
        var exactPaths = new HashSet<string>(StringComparer.Ordinal);
        foreach (var candidate in request.Candidates)
        {
            ValidateCandidate(candidate, repositoryRelative: false, limits);
            RegisterPortableIdentity(candidate.Path, exactPaths, identities);
            if (candidate.Kind is "symlink" or "reparse_point")
            {
                linkBoundaries.Add(candidate.Path);
            }
        }
        ValidateCandidateTopology(request.Candidates);

        var packageExactPaths = language.PackageExactInputs
            .Where(entry => string.Equals(entry.PackageRoot, request.PackageRoot, StringComparison.Ordinal))
            .SelectMany(entry => entry.Paths)
            .ToHashSet(StringComparer.Ordinal);
        var selected = new List<SelectedSourceFile>();
        ulong selectedBytes = 0;
        ulong globMatchWork = 0;
        foreach (var candidate in request.Candidates)
        {
            if (candidate.Kind != "file" ||
                IsAtOrBelowBoundary(candidate.Path, linkBoundaries) ||
                candidate.Path.Split('/').Any(GeneratedDirectoryComponents.Contains))
            {
                continue;
            }

            var basename = PortableBasename(candidate.Path);
            var rootOnly = !candidate.Path.Contains('/');
            var included = LanguageRegistry.UniversalInputs.BuildFilenames.Contains(basename, StringComparer.Ordinal);
            included |= rootOnly && LanguageRegistry.UniversalInputs.RootExactBasenames.Contains(basename, StringComparer.Ordinal);
            included |= rootOnly && language.RootExactBasenames.Contains(basename, StringComparer.Ordinal);
            included |= rootOnly && language.RootVariableSuffixes.Any(suffix => basename.EndsWith(suffix, StringComparison.Ordinal));
            included |= language.RootExactRelativePaths.Contains(candidate.Path, StringComparer.Ordinal);
            included |= packageExactPaths.Contains(candidate.Path);

            if (request.Mode == "extension")
            {
                included |= language.RecursiveExactBasenames.Contains(basename, StringComparer.Ordinal);
                included |= language.RecursiveSuffixes.Any(suffix => basename.EndsWith(suffix, StringComparison.Ordinal));
                included |= language.ScopedInputs.Any(rule => ScopedInputMatches(rule, candidate.Path, basename));
            }
            else if (!included)
            {
                foreach (var pattern in request.DeclaredSources)
                {
                    var patternRunes = checked((ulong)pattern.EnumerateRunes().Count() + 1);
                    var pathRunes = checked((ulong)candidate.Path.EnumerateRunes().Count() + 1);
                    ulong cost;
                    try
                    {
                        cost = checked(patternRunes * pathRunes);
                        globMatchWork = checked(globMatchWork + cost);
                    }
                    catch (OverflowException)
                    {
                        throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
                    }
                    if (globMatchWork > MaximumGlobMatchWork)
                    {
                        throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
                    }
                    if (PortableGlobMatches(pattern, candidate.Path))
                    {
                        included = true;
                        break;
                    }
                }
            }

            if (!included)
            {
                continue;
            }
            if (selected.Count >= limits.MaximumSelectedInputs)
            {
                throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
            }
            selectedBytes = CheckedByteTotal(selectedBytes, (ulong)candidate.Content.LongLength, limits);
            selected.Add(new SelectedSourceFile(candidate.Path, LowerSha256(candidate.Content)));
        }
        ValidateCandidateTopology(request.Candidates);
        return selected.OrderBy(file => file.Path, Utf8StringComparer.Instance).ToArray();
    }

    private static void ValidateCandidateTopology(IReadOnlyList<SourceCollectionCandidate> candidates)
    {
        var ordered = candidates.OrderBy(candidate => candidate.Path, Utf8StringComparer.Instance).ToArray();
        for (var index = 0; index + 1 < ordered.Length; index++)
        {
            var candidate = ordered[index];
            if (candidate.Kind == "file" &&
                ordered[index + 1].Path.StartsWith(candidate.Path + "/", StringComparison.Ordinal))
            {
                throw new SourceHashException("SOURCE_HASH_PATH_COLLISION");
            }
        }
    }

    private static void ValidateFilePathTopology(IEnumerable<string> paths)
    {
        var ordered = paths.OrderBy(path => path, Utf8StringComparer.Instance).ToArray();
        for (var index = 0; index + 1 < ordered.Length; index++)
        {
            if (ordered[index + 1].StartsWith(ordered[index] + "/", StringComparison.Ordinal))
            {
                throw new SourceHashException("SOURCE_HASH_PATH_COLLISION");
            }
        }
    }

    private static IReadOnlyList<SelectedSourceFile> SelectBoundaryCandidates(
        SourceCollectionRequest request,
        SourceHashLimits limits)
    {
        if (!string.Equals(request.RegistrySha256, RepositorySourceInputBoundaryDigest, StringComparison.Ordinal) ||
            !string.Equals(
                BoundaryRegistry.LanguageSourceInputRegistrySha256,
                LanguageSourceInputRegistryDigest,
                StringComparison.Ordinal))
        {
            throw new SourceHashException("SOURCE_HASH_REGISTRY_MISMATCH");
        }

        var allowedPaths = BoundaryPaths(request.PackageRoot).ToHashSet(StringComparer.Ordinal);
        var identities = new Dictionary<string, string>(StringComparer.Ordinal);
        var exactPaths = new HashSet<string>(StringComparer.Ordinal);
        var selected = new List<SelectedSourceFile>();
        ulong selectedBytes = 0;
        foreach (var candidate in request.Candidates)
        {
            ValidateCandidate(candidate, repositoryRelative: true, limits);
            RegisterPortableIdentity(candidate.Path, exactPaths, identities);
            if (candidate.Kind != "file" || !candidate.Tracked || !allowedPaths.Contains(candidate.Path))
            {
                continue;
            }
            if (selected.Count >= limits.MaximumSelectedInputs)
            {
                throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
            }
            selectedBytes = CheckedByteTotal(selectedBytes, (ulong)candidate.Content.LongLength, limits);
            selected.Add(new SelectedSourceFile(candidate.Path, LowerSha256(candidate.Content)));
        }
        return selected.OrderBy(file => file.Path, Utf8StringComparer.Instance).ToArray();
    }

    private static PackageSourceCollection CollectPackageLocalFiles(
        PackageSpec package,
        string repositoryRoot,
        SourceHashLimits limits,
        SecureSourceFileReader.Scope secureScope)
    {
        if (!LanguageInputs.ContainsKey(package.Language))
        {
            throw new SourceHashException("SOURCE_HASH_LANGUAGE_UNKNOWN");
        }
        var packageRoot = PortableRelativePath(repositoryRoot, package.Path);
        ValidatePackageRoot(packageRoot, package.Language);
        var candidates = new List<SourceCollectionCandidate>();
        var candidateCount = 0;
        CollectDirectory(
            package.Path,
            string.Empty,
            candidates,
            limits,
            secureScope,
            ref candidateCount);
        var declaredSources = package.DeclaredSources ?? [];
        var mode = package.IsStarlark ? "declared_sources" : "extension";
        var selected = SelectSourceCandidates(new SourceCollectionRequest(
            package.Language,
            packageRoot,
            mode,
            LanguageSourceInputRegistryDigest,
            declaredSources,
            candidates), limits);
        var files = selected
            .Select(file => ContainedPath(package.Path, file.Path))
            .ToArray();
        return new PackageSourceCollection(files);
    }

    private static void CollectDirectory(
        string directory,
        string relativeDirectory,
        List<SourceCollectionCandidate> candidates,
        SourceHashLimits limits,
        SecureSourceFileReader.Scope secureScope,
        ref int candidateCount)
    {
        var entries = secureScope.EnumerateDirectory(
            directory,
            limits.MaximumCandidates - candidateCount);
        candidateCount += entries.Count;
        foreach (var entry in entries.OrderBy(item => item.Name, Utf8StringComparer.Instance))
        {
            var name = entry.Name;
            var relative = relativeDirectory.Length == 0 ? name : $"{relativeDirectory}/{name}";
            ValidatePortablePath(relative);
            var fullPath = Path.Combine(directory, name);
            if (entry.Kind == SecureDirectoryEntryKind.Linked)
            {
                var linkKind = OperatingSystem.IsWindows() ? "reparse_point" : "symlink";
                candidates.Add(new SourceCollectionCandidate(relative, linkKind, false, []));
                continue;
            }
            if (entry.Kind == SecureDirectoryEntryKind.Directory)
            {
                if (!GeneratedDirectoryComponents.Contains(name))
                {
                    CollectDirectory(
                        fullPath,
                        relative,
                        candidates,
                        limits,
                        secureScope,
                        ref candidateCount);
                }
                continue;
            }
            if (entry.Kind != SecureDirectoryEntryKind.Regular)
            {
                throw new SourceHashException("SOURCE_HASH_LINK_REJECTED");
            }
            candidates.Add(new SourceCollectionCandidate(relative, "file", false, []));
        }
    }

    private static void ValidateCandidate(
        SourceCollectionCandidate candidate,
        bool repositoryRelative,
        SourceHashLimits limits)
    {
        ArgumentNullException.ThrowIfNull(candidate);
        ArgumentNullException.ThrowIfNull(candidate.Content);
        if (candidate.Kind is not ("file" or "symlink" or "reparse_point"))
        {
            throw new SourceHashException("SOURCE_HASH_CANDIDATE_INVALID");
        }
        ValidatePortablePath(candidate.Path);
        if (repositoryRelative && !candidate.Path.StartsWith("code/", StringComparison.Ordinal))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }
        if ((ulong)candidate.Content.LongLength > limits.MaximumFileBytes)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
        }
    }

    private static IReadOnlyList<string> BoundaryPaths(string packageRoot) =>
        BoundaryRegistry.Boundaries
            .Where(boundary => BoundaryApplies(boundary, packageRoot))
            .SelectMany(boundary => boundary.Inputs)
            .Select(input => input.Path)
            .Distinct(StringComparer.Ordinal)
            .OrderBy(path => path, Utf8StringComparer.Instance)
            .ToArray();

    private static bool BoundaryApplies(RepositorySourceBoundary boundary, string packageRoot)
    {
        if (boundary.AppliesTo.ExactRoots.Contains(packageRoot, StringComparer.Ordinal))
        {
            return true;
        }
        return !boundary.AppliesTo.ExcludedRoots.Contains(packageRoot, StringComparer.Ordinal) &&
               boundary.AppliesTo.DescendantRoots.Any(root =>
                   packageRoot.StartsWith(root + "/", StringComparison.Ordinal));
    }

    private static bool ScopedInputMatches(ScopedSourceInputs rule, string path, string basename)
    {
        if (rule.Scope == "root")
        {
            if (path.Contains('/'))
            {
                return false;
            }
        }
        else if (rule.PathPrefix is null ||
                 !path.StartsWith(rule.PathPrefix + "/", StringComparison.Ordinal))
        {
            return false;
        }
        return rule.ExactBasenames.Contains(basename, StringComparer.Ordinal) ||
               rule.Suffixes.Any(suffix => basename.EndsWith(suffix, StringComparison.Ordinal));
    }

    private static bool PortableGlobMatches(string pattern, string path)
    {
        var patternParts = pattern.Split('/');
        var pathParts = path.Split('/');
        var memo = new Dictionary<(int Pattern, int Path), bool>();
        bool Match(int patternIndex, int pathIndex)
        {
            if (memo.TryGetValue((patternIndex, pathIndex), out var cached))
            {
                return cached;
            }
            bool result;
            if (patternIndex == patternParts.Length)
            {
                result = pathIndex == pathParts.Length;
            }
            else if (patternParts[patternIndex] == "**")
            {
                result = Match(patternIndex + 1, pathIndex) ||
                         (pathIndex < pathParts.Length && Match(patternIndex, pathIndex + 1));
            }
            else
            {
                result = pathIndex < pathParts.Length &&
                         GlobSegmentMatches(patternParts[patternIndex], pathParts[pathIndex]) &&
                         Match(patternIndex + 1, pathIndex + 1);
            }
            memo[(patternIndex, pathIndex)] = result;
            return result;
        }
        return Match(0, 0);
    }

    private static bool GlobSegmentMatches(string pattern, string value)
    {
        var patternRunes = pattern.EnumerateRunes().ToArray();
        var valueRunes = value.EnumerateRunes().ToArray();
        var memo = new Dictionary<(int Pattern, int Value), bool>();

        bool Match(int patternIndex, int valueIndex)
        {
            if (memo.TryGetValue((patternIndex, valueIndex), out var cached))
            {
                return cached;
            }

            bool result;
            if (patternIndex == patternRunes.Length)
            {
                result = valueIndex == valueRunes.Length;
            }
            else if (patternRunes[patternIndex].Value == '*')
            {
                result = Match(patternIndex + 1, valueIndex) ||
                         (valueIndex < valueRunes.Length && Match(patternIndex, valueIndex + 1));
            }
            else if (valueIndex == valueRunes.Length)
            {
                result = false;
            }
            else if (patternRunes[patternIndex].Value == '?')
            {
                result = Match(patternIndex + 1, valueIndex + 1);
            }
            else if (TryMatchCharacterClass(
                         patternRunes,
                         patternIndex,
                         valueRunes[valueIndex],
                         out var nextPatternIndex,
                         out var classMatched))
            {
                result = classMatched && Match(nextPatternIndex, valueIndex + 1);
            }
            else
            {
                result = patternRunes[patternIndex] == valueRunes[valueIndex] &&
                         Match(patternIndex + 1, valueIndex + 1);
            }

            memo[(patternIndex, valueIndex)] = result;
            return result;
        }

        return Match(0, 0);
    }

    private static bool TryMatchCharacterClass(
        IReadOnlyList<Rune> pattern,
        int openingIndex,
        Rune value,
        out int nextPatternIndex,
        out bool matched)
    {
        nextPatternIndex = openingIndex;
        matched = false;
        if (pattern[openingIndex].Value != '[')
        {
            return false;
        }

        var cursor = openingIndex + 1;
        if (cursor >= pattern.Count)
        {
            return false;
        }
        var negate = pattern[cursor].Value == '!';
        if (negate)
        {
            cursor++;
        }
        var closingIndex = cursor;
        if (closingIndex < pattern.Count && pattern[closingIndex].Value == ']')
        {
            closingIndex++;
        }
        while (closingIndex < pattern.Count && pattern[closingIndex].Value != ']')
        {
            closingIndex++;
        }
        if (closingIndex == pattern.Count || closingIndex == openingIndex + 1)
        {
            return false;
        }

        cursor = openingIndex + 1;
        if (negate)
        {
            cursor++;
        }
        var memberMatched = false;
        while (cursor < closingIndex)
        {
            var lower = pattern[cursor];
            if (cursor + 2 < closingIndex && pattern[cursor + 1].Value == '-')
            {
                var upper = pattern[cursor + 2];
                memberMatched |= lower.Value <= value.Value && value.Value <= upper.Value;
                cursor += 3;
            }
            else
            {
                memberMatched |= lower == value;
                cursor++;
            }
        }

        nextPatternIndex = closingIndex + 1;
        matched = negate ? !memberMatched : memberMatched;
        return true;
    }

    private static void ValidatePortableGlob(string pattern)
    {
        if (string.IsNullOrEmpty(pattern) ||
            pattern.EnumerateRunes().Count() > 512 ||
            !string.Equals(pattern, TrackedArtifactUnicode17.Nfc(pattern), StringComparison.Ordinal) ||
            pattern.StartsWith('/') ||
            pattern.Contains('\\') ||
            pattern.Contains("//", StringComparison.Ordinal) ||
            (pattern.Length >= 2 && char.IsAsciiLetter(pattern[0]) && pattern[1] == ':') ||
            pattern.EnumerateRunes().Any(IsUnsafeGlobRune) ||
            HasAmbiguousCharacterClass(pattern) ||
            pattern.Split('/').Any(segment =>
                segment is "" or "." or ".." ||
                segment.EndsWith(' ') ||
                segment.EndsWith('.') ||
                (!segment.Any(character => "*[]{}".Contains(character)) &&
                 WindowsReservedBasenames.Contains(
                     TrackedArtifactUnicode17.FullUppercase(segment.Split('.', 2)[0])))))
        {
            throw new SourceHashException("SOURCE_HASH_GLOB_INVALID");
        }
    }

    private static bool HasAmbiguousCharacterClass(string pattern)
    {
        var runes = pattern.EnumerateRunes().ToArray();
        var index = 0;
        while (index < runes.Length)
        {
            if (runes[index].Value != '[')
            {
                index++;
                continue;
            }
            var cursor = index + 1;
            if (cursor < runes.Length && runes[cursor].Value == '!')
            {
                cursor++;
            }
            var closing = cursor;
            if (closing < runes.Length && runes[closing].Value == ']')
            {
                closing++;
            }
            while (closing < runes.Length && runes[closing].Value != ']')
            {
                closing++;
            }
            if (closing == runes.Length)
            {
                index++;
                continue;
            }
            for (var member = cursor; member < closing; member++)
            {
                if (member + 1 < closing && runes[member].Value == runes[member + 1].Value &&
                    runes[member].Value is '-' or '&' or '~' or '|')
                {
                    return true;
                }
                if (member + 2 < closing && runes[member + 1].Value == '-' &&
                    runes[member].Value > runes[member + 2].Value)
                {
                    return true;
                }
            }
            index = closing + 1;
        }
        return false;
    }

    private static void ValidatePortablePath(string path)
    {
        if (string.IsNullOrEmpty(path) ||
            path.EnumerateRunes().Count() > 512 ||
            !string.Equals(path, TrackedArtifactUnicode17.Nfc(path), StringComparison.Ordinal) ||
            path.StartsWith('/') ||
            path.Contains('\\') ||
            path.Contains("//", StringComparison.Ordinal) ||
            (path.Length >= 2 && char.IsAsciiLetter(path[0]) && path[1] == ':') ||
            path.EnumerateRunes().Any(IsUnsafePathRune))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }

        foreach (var component in path.Split('/'))
        {
            if (component is "" or "." or ".." ||
                component.EndsWith(' ') ||
                component.EndsWith('.'))
            {
                throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
            }
            var basename = TrackedArtifactUnicode17.FullUppercase(component.Split('.', 2)[0]);
            if (WindowsReservedBasenames.Contains(basename))
            {
                throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
            }
        }
    }

    private static void ValidatePackageRoot(string packageRoot, string expectedLanguage)
    {
        ValidatePortablePath(packageRoot);
        var components = packageRoot.Split('/');
        if (components.Length < 4 ||
            components[0] != "code" ||
            components[1] is not ("packages" or "programs") ||
            components[2] != expectedLanguage)
        {
            throw new SourceHashException("SOURCE_HASH_PACKAGE_ROOT_INVALID");
        }
    }

    private static bool IsUnsafePathRune(Rune rune)
    {
        var category = Rune.GetUnicodeCategory(rune);
        return rune.Value < 0x20 ||
               rune.Value is 0x7f or 0x3c or 0x3e or 0x3a or 0x22 or 0x7c or 0x3f or 0x2a ||
               category is UnicodeCategory.Control or UnicodeCategory.Format or
                   UnicodeCategory.LineSeparator or UnicodeCategory.ParagraphSeparator;
    }

    private static bool IsUnsafeGlobRune(Rune rune) =>
        rune.Value is 0x3c or 0x3e or 0x3a or 0x22 or 0x7c or 0x3f ||
        (rune.Value < 0x20) ||
        Rune.GetUnicodeCategory(rune) is UnicodeCategory.Control or UnicodeCategory.Format or
            UnicodeCategory.LineSeparator or UnicodeCategory.ParagraphSeparator;

    private static void RegisterPortableIdentity(
        string path,
        HashSet<string> exactPaths,
        Dictionary<string, string> identities)
    {
        if (!exactPaths.Add(path))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_COLLISION");
        }
        var identity = TrackedArtifactUnicode17.CaseFold(TrackedArtifactUnicode17.Nfc(path));
        if (identities.TryGetValue(identity, out var prior) &&
            !string.Equals(prior, path, StringComparison.Ordinal))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_COLLISION");
        }
        identities[identity] = path;
    }

    private static bool IsAtOrBelowBoundary(string path, HashSet<string> boundaries) =>
        boundaries.Any(boundary =>
            string.Equals(path, boundary, StringComparison.Ordinal) ||
            path.StartsWith(boundary + "/", StringComparison.Ordinal));

    private static string PortableBasename(string path)
    {
        var slash = path.LastIndexOf('/');
        return slash < 0 ? path : path[(slash + 1)..];
    }

    private static string PortableRelativePath(string root, string path)
    {
        var fullRoot = Path.GetFullPath(root);
        var fullPath = Path.GetFullPath(path);
        var relative = Path.GetRelativePath(fullRoot, fullPath).Replace('\\', '/');
        ValidatePortablePath(relative);
        if (relative == ".." || relative.StartsWith("../", StringComparison.Ordinal))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }
        return relative;
    }

    private static string ContainedPath(string root, string relative)
    {
        ValidatePortablePath(relative);
        var fullRoot = Path.GetFullPath(root);
        var fullPath = Path.GetFullPath(Path.Combine(fullRoot, relative.Replace('/', Path.DirectorySeparatorChar)));
        var check = Path.GetRelativePath(fullRoot, fullPath).Replace('\\', '/');
        if (check == ".." || check.StartsWith("../", StringComparison.Ordinal))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }
        return fullPath;
    }

    private static string FindRepositoryRoot(string packagePath)
    {
        for (var directory = new DirectoryInfo(Path.GetFullPath(packagePath)); directory is not null; directory = directory.Parent)
        {
            if (Directory.Exists(Path.Combine(directory.FullName, "code")))
            {
                return directory.FullName;
            }
        }
        throw new SourceHashException("SOURCE_HASH_PACKAGE_ROOT_INVALID");
    }

    private static ulong CheckedByteTotal(ulong current, ulong fileBytes, SourceHashLimits limits)
    {
        if (fileBytes > limits.MaximumFileBytes ||
            ulong.MaxValue - current < fileBytes ||
            current + fileBytes > limits.MaximumPackageBytes)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
        }
        return current + fileBytes;
    }

    private static void ValidateLimits(SourceHashLimits limits)
    {
        ArgumentNullException.ThrowIfNull(limits);
        var hard = SourceHashLimits.Default;
        if (limits.MaximumCandidates <= 0 || limits.MaximumCandidates > hard.MaximumCandidates ||
            limits.MaximumSelectedInputs <= 0 || limits.MaximumSelectedInputs > hard.MaximumSelectedInputs ||
            limits.MaximumFileBytes == 0 || limits.MaximumFileBytes > hard.MaximumFileBytes ||
            limits.MaximumPackageBytes == 0 || limits.MaximumPackageBytes > hard.MaximumPackageBytes ||
            limits.MaximumPackageBytes < limits.MaximumFileBytes)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_CONFIGURATION_INVALID");
        }
    }

    private static void AppendFrame(IncrementalHash digest, byte[] bytes)
    {
        Span<byte> length = stackalloc byte[sizeof(ulong)];
        BinaryPrimitives.WriteUInt64BigEndian(length, (ulong)bytes.LongLength);
        digest.AppendData(length);
        digest.AppendData(bytes);
    }

    private static string LowerSha256(byte[] bytes) =>
        Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();

    private static string CanonicalRegistryDigest(string json, string domain)
    {
        using var document = JsonDocument.Parse(json);
        using var buffer = new MemoryStream();
        using (var writer = new Utf8JsonWriter(buffer, new JsonWriterOptions
        {
            Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
            Indented = false,
        }))
        {
            WriteCanonicalJson(writer, document.RootElement);
        }
        using var digest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        digest.AppendData(Encoding.ASCII.GetBytes(domain));
        digest.AppendData([0]);
        AppendFrame(digest, buffer.ToArray());
        return Convert.ToHexString(digest.GetHashAndReset()).ToLowerInvariant();
    }

    private static void WriteCanonicalJson(Utf8JsonWriter writer, JsonElement element)
    {
        switch (element.ValueKind)
        {
            case JsonValueKind.Object:
                writer.WriteStartObject();
                foreach (var property in element.EnumerateObject().OrderBy(property => property.Name, StringComparer.Ordinal))
                {
                    writer.WritePropertyName(property.Name);
                    WriteCanonicalJson(writer, property.Value);
                }
                writer.WriteEndObject();
                break;
            case JsonValueKind.Array:
                writer.WriteStartArray();
                foreach (var child in element.EnumerateArray())
                {
                    WriteCanonicalJson(writer, child);
                }
                writer.WriteEndArray();
                break;
            case JsonValueKind.String:
                writer.WriteStringValue(element.GetString());
                break;
            case JsonValueKind.Number:
                writer.WriteRawValue(element.GetRawText(), skipInputValidation: false);
                break;
            case JsonValueKind.True:
                writer.WriteBooleanValue(true);
                break;
            case JsonValueKind.False:
                writer.WriteBooleanValue(false);
                break;
            case JsonValueKind.Null:
                writer.WriteNullValue();
                break;
            default:
                throw new SourceHashException("SOURCE_HASH_REGISTRY_INVALID");
        }
    }

    private static HashSet<string> BuildWindowsReservedBasenames()
    {
        var values = new HashSet<string>(StringComparer.Ordinal)
        {
            "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$",
        };
        foreach (var prefix in new[] { "COM", "LPT" })
        {
            foreach (var suffix in new[] { "1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³" })
            {
                values.Add(prefix + suffix);
            }
        }
        return values;
    }

    private static string QuoteDiagnostic(string value)
    {
        var builder = new StringBuilder("\"");
        foreach (var rune in value.EnumerateRunes())
        {
            if (rune.Value == '\"')
            {
                builder.Append("\\\"");
            }
            else if (rune.Value == '\\')
            {
                builder.Append("\\\\");
            }
            else if (Rune.GetUnicodeCategory(rune) is UnicodeCategory.Control or UnicodeCategory.Format or
                         UnicodeCategory.LineSeparator or UnicodeCategory.ParagraphSeparator)
            {
                builder.Append(rune.Value <= 0xffff ? $"\\u{rune.Value:x4}" : $"\\U{rune.Value:x8}");
            }
            else
            {
                builder.Append(rune.ToString());
            }
        }
        return builder.Append('\"').ToString();
    }

    private sealed class Utf8StringComparer : IComparer<string>
    {
        internal static Utf8StringComparer Instance { get; } = new();

        public int Compare(string? left, string? right)
        {
            if (ReferenceEquals(left, right))
            {
                return 0;
            }
            if (left is null)
            {
                return -1;
            }
            if (right is null)
            {
                return 1;
            }
            return Encoding.UTF8.GetBytes(left).AsSpan().SequenceCompareTo(Encoding.UTF8.GetBytes(right));
        }
    }
}
