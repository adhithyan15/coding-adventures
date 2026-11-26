using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Shared helpers: fixture lookup, deep copies, hook hygiene.</summary>
internal static class Fx
{
    /// <summary>The committed fixture, found by walking up from the test binary. Never a copy.</summary>
    internal static string FixturePath()
    {
        DirectoryInfo? dir = new(AppContext.BaseDirectory);
        while (dir != null)
        {
            string candidate = Path.Combine(dir.FullName, "code", "specs", "fixtures", "pbt-v1", "cases.json");
            if (File.Exists(candidate))
            {
                return candidate;
            }

            dir = dir.Parent;
        }

        throw new InvalidOperationException("could not find code/specs/fixtures/pbt-v1/cases.json above the test binary");
    }

    internal static List<object?> LoadCases() => Corpus.Parse(File.ReadAllBytes(FixturePath()));

    internal static object? DeepCopy(object? node)
    {
        switch (node)
        {
            case Dictionary<string, object?> m:
                {
                    Dictionary<string, object?> output = new();
                    foreach (KeyValuePair<string, object?> e in m)
                    {
                        output[e.Key] = DeepCopy(e.Value);
                    }

                    return output;
                }

            case List<object?> l:
                return l.Select(DeepCopy).ToList();
            default:
                return node; // string, bool, JsonNumber, null: immutable
        }
    }

    internal static List<object?> CopyCases(IEnumerable<object?> cases) => cases.Select(DeepCopy).ToList();

    internal static Dictionary<string, object?> Obj(object? o) => (Dictionary<string, object?>)o!;

    internal static byte[] Utf8(string s) => System.Text.Encoding.UTF8.GetBytes(s);

    internal static BigInteger Big(string s) => BigInteger.Parse(s, System.Globalization.CultureInfo.InvariantCulture);
}

/// <summary>Base class: every test starts and ends with production hooks.</summary>
public abstract class HookedTest : IDisposable
{
    protected HookedTest()
    {
        Hooks.Reset();
    }

    public void Dispose()
    {
        Hooks.Reset();
        GC.SuppressFinalize(this);
    }
}
