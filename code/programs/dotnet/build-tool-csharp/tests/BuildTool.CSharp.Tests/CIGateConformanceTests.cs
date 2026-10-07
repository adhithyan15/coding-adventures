namespace CodingAdventures.BuildTool.CSharp.Tests;

using System.Text.Json;

public sealed class CIGateConformanceTests
{
    private static readonly string[] ExpectedIds =
    [
        "ci-gate-selection/character-classes", "ci-gate-selection/force",
        "ci-gate-selection/machinery", "ci-gate-selection/match-work-at-limit",
        "ci-gate-selection/match-work-over-limit", "ci-gate-selection/null-affected",
        "ci-gate-selection/null-changed-files", "ci-gate-selection/package-and-path",
        "ci-gate-selection/recursive-glob", "ci-gate-selection/shared-pattern-at-limit",
        "ci-gate-selection/unrelated-change",
    ];

    [Fact]
    public void EveryNeutralCaseRunsThroughTypedCSharpEngine()
    {
        var paths = Directory.GetFiles(FindCases(), "ci-gate-selection-*.json");
        Assert.Equal(ExpectedIds, paths.Select(path =>
        {
            using var file = JsonDocument.Parse(File.ReadAllText(path));
            return file.RootElement.GetProperty("id").GetString();
        }).Order(StringComparer.Ordinal));

        foreach (var path in paths)
        {
            using var file = JsonDocument.Parse(File.ReadAllText(path));
            var root = file.RootElement;
            var id = root.GetProperty("id").GetString();
            Assert.Equal("ci_gate_selection", root.GetProperty("domain").GetString());
            Assert.Equal("ci_gate_selection", root.GetProperty("input").GetProperty("operation").GetString());
            var options = root.GetProperty("input").GetProperty("options");
            var registry = options.GetProperty("registry");
            var gates = registry.GetProperty("gates").EnumerateArray().Select(gate =>
                new CIGateDefinition(gate.GetProperty("id").GetString()!,
                    gate.GetProperty("scope").GetString()!, gate.GetProperty("description").GetString()!,
                    Strings(gate.GetProperty("packages"))!, Strings(gate.GetProperty("paths"))!)).ToArray();
            var actual = CIGateSelection.Evaluate(new CIGateSelectionInput(
                new CIGateRegistry(registry.GetProperty("schema_version").GetInt32(), gates),
                Strings(options.GetProperty("affected_packages")),
                Strings(options.GetProperty("changed_files")),
                options.GetProperty("force").GetBoolean()));
            var expected = root.GetProperty("expected");
            Assert.Equal(id, expected.GetProperty("case_id").GetString());
            var diagnostics = expected.GetProperty("diagnostics").EnumerateArray().ToArray();
            if (expected.GetProperty("outcome").GetString() == "error")
            {
                Assert.Single(diagnostics);
                Assert.Equal(diagnostics[0].GetProperty("code").GetString(), actual.ErrorCode);
                Assert.Equal("error", diagnostics[0].GetProperty("severity").GetString());
                Assert.Empty(actual.Gates);
                Assert.False(expected.GetProperty("result").TryGetProperty("gates", out _));
            }
            else
            {
                Assert.Empty(diagnostics);
                Assert.Null(actual.ErrorCode);
                var expectedGates = expected.GetProperty("result").GetProperty("gates").EnumerateArray()
                    .Select(gate => new CIGateVerdict(gate.GetProperty("id").GetString()!,
                        gate.GetProperty("required").GetBoolean(),
                        gate.GetProperty("output_name").GetString()!)).ToArray();
                Assert.Equal(expectedGates, actual.Gates);
            }
        }
    }

    [Fact]
    public void InvalidRegistryDoesNotBypassValidationOnForce()
    {
        var invalid = new CIGateRegistry(1,
            [new CIGateDefinition("BAD", "job", "invalid id", ["rust/alpha"], [])]);
        var actual = CIGateSelection.Evaluate(new CIGateSelectionInput(invalid, null, null, true));
        Assert.NotNull(actual.ErrorCode);
        Assert.Empty(actual.Gates);
    }

    [Fact]
    public void OmittedScopeDefaultsToJobLikeTheGoOracle()
    {
        var registry = new CIGateRegistry(1,
            [new CIGateDefinition("default-job", "", "implicit job scope", ["rust/alpha"], [])]);
        var actual = CIGateSelection.Evaluate(new CIGateSelectionInput(registry, ["rust/alpha"], [], false));
        Assert.Null(actual.ErrorCode);
        Assert.Equal([new CIGateVerdict("default-job", true, "run_default_job")], actual.Gates);
    }

    private static string[]? Strings(JsonElement value) => value.ValueKind == JsonValueKind.Null
        ? null : value.EnumerateArray().Select(item => item.GetString()!).ToArray();

    private static string FindCases()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null;
            directory = directory.Parent)
        {
            var path = Path.Combine(directory.FullName, "code", "specs", "fixtures", "build-tool-v1", "cases");
            if (Directory.Exists(path)) return path;
        }
        throw new DirectoryNotFoundException("build-tool-v1 cases not found");
    }
}
