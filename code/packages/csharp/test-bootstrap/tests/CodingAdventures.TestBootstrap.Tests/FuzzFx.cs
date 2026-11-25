using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Fuzz;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Shared helpers of the fuzz tests: the committed fixture, hex, big integers, case surgery.</summary>
internal static class FuzzFx
{
    /// <summary>The repository root, found by walking up from the test binary.</summary>
    internal static string RepoRoot()
    {
        DirectoryInfo? dir = new(AppContext.BaseDirectory);
        while (dir != null)
        {
            if (File.Exists(Path.Combine(dir.FullName, "code", "specs", "fixtures", "fuzz-v1", "cases.json")))
            {
                return dir.FullName;
            }

            dir = dir.Parent;
        }

        throw new InvalidOperationException("could not find code/specs/fixtures/fuzz-v1/cases.json above the test binary");
    }

    internal static string FixturePath() => Path.Combine(RepoRoot(), "code", "specs", "fixtures", "fuzz-v1", "cases.json");

    internal static List<object?> LoadCases() => FuzzCorpus.Parse(File.ReadAllBytes(FixturePath()));

    internal static byte[] Hx(string s) => Convert.FromHexString(s);

    internal static string Hex(byte[] b) => Convert.ToHexString(b).ToLowerInvariant();

    internal static BigInteger Bg(string s) => Fx.Big(s);

    internal static Dictionary<string, object?> IntNode(string v) => new() { ["kind"] = "int", ["value"] = v };

    internal static Dictionary<string, object?> BytesNode(string hex) => new() { ["kind"] = "bytes", ["hex"] = hex };

    internal static Dictionary<string, object?> CaseAt(List<object?> cases, int i) => Fx.Obj(cases[i]);

    internal static Dictionary<string, object?> FirstOf(List<object?> cases, string op)
    {
        return Fx.Obj(cases.First(c => (string)Fx.Obj(c)["op"]! == op));
    }

    /// <summary>The case list with the case that has the victim's id replaced by the victim.</summary>
    internal static List<object?> Swapped(List<object?> cases, Dictionary<string, object?> victim)
    {
        List<object?> output = new();
        foreach (object? c in cases)
        {
            output.Add((string)Fx.Obj(c)["id"]! == (string)victim["id"]! ? victim : c);
        }

        return output;
    }

    internal static List<string> Problems(List<object?> cases) => FuzzCorpus.Evaluate(cases).Problems.ToList();

    internal static bool AnyContains(IEnumerable<string> problems, string fragment) => problems.Any(p => p.Contains(fragment));

    internal static Endian EndianOf(string s) => s == "big" ? Endian.Big : Endian.Little;
}

/// <summary>Base class: every fuzz test starts and ends with production hooks.</summary>
public abstract class FuzzHookedTest : IDisposable
{
    protected FuzzHookedTest()
    {
        FuzzHooks.Reset();
    }

    public void Dispose()
    {
        FuzzHooks.Reset();
        GC.SuppressFinalize(this);
    }
}
