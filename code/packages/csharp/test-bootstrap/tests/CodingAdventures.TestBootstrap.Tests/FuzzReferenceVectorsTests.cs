using System.Numerics;
using System.Text.RegularExpressions;
using CodingAdventures.TestBootstrap.Fuzz;
using static CodingAdventures.TestBootstrap.Tests.FuzzFx;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// The 820 differential vectors of the Go lane, computed by the independent Python oracle
/// <c>scripts/fuzz_conformance.py</c> (generator: <c>go/test-bootstrap/fuzz/testdata/gen_reference_vectors.py</c>).
/// The Go table is a committed source file, so it is read from its committed path; there is no copy.
/// The fuzz-v1 corpus alone cannot pin these (VER06h section 9 item 9).
/// </summary>
public sealed class FuzzReferenceVectorsTests
{
    private sealed record Vector(string Op, List<string> Args, string Out);

    private static readonly Regex Row = new("^\\t\\{\"([a-z_]+)\", \\[\\]string\\{(.*)\\}, \"([0-9a-f]*)\"\\},$", RegexOptions.CultureInvariant);
    private static readonly Regex Arg = new("\"([^\"]*)\"", RegexOptions.CultureInvariant);

    private static List<Vector> Load()
    {
        string file = Path.Combine(RepoRoot(), "code", "packages", "go", "test-bootstrap", "fuzz", "reference_vectors_test.go");
        string[] lines = File.ReadAllLines(file);
        int start = Array.IndexOf(lines, "var referenceVectors = []refVector{");
        Assert.True(start >= 0, "the vector table moved");
        List<Vector> output = new();
        for (int i = start + 1; i < lines.Length && lines[i] != "}"; i++)
        {
            Match m = Row.Match(lines[i]);
            Assert.True(m.Success, "unparseable vector row " + (i + 1) + ": " + lines[i]);
            List<string> args = Arg.Matches(m.Groups[2].Value).Select(a => a.Groups[1].Value).ToList();
            output.Add(new Vector(m.Groups[1].Value, args, m.Groups[3].Value));
        }

        return output;
    }

    private static bool FitsU64(string decimalText)
    {
        BigInteger v = Bg(decimalText);
        return v.Sign >= 0 && v <= ulong.MaxValue;
    }

    private static bool FitsI64(string decimalText)
    {
        BigInteger v = Bg(decimalText);
        return v >= long.MinValue && v <= long.MaxValue;
    }

    [Fact]
    public void AllVectorsMatchBothTheBigAndTheUlongEntryPoints()
    {
        List<Vector> vectors = Load();
        Assert.Equal(820, vectors.Count);
        SortedDictionary<string, int> counts = new();
        int ulongPaths = 0;
        for (int i = 0; i < vectors.Count; i++)
        {
            Vector v = vectors[i];
            List<string> a = v.Args;
            byte[] want = Hx(v.Out);
            byte[] got;
            byte[]? small = null;
            counts[v.Op] = counts.GetValueOrDefault(v.Op) + 1;
            switch (v.Op)
            {
                case "bit_flip":
                    got = Mutate.BitFlipBig(Hx(a[0]), Bg(a[1]), Bg(a[2]));
                    if (FitsU64(a[1]) && FitsU64(a[2]))
                    {
                        small = Mutate.BitFlip(Hx(a[0]), ulong.Parse(a[1]), ulong.Parse(a[2]));
                    }

                    break;
                case "byte_flip":
                    got = Mutate.ByteFlipBig(Hx(a[0]), Bg(a[1]), Bg(a[2]));
                    if (FitsU64(a[1]) && FitsU64(a[2]))
                    {
                        small = Mutate.ByteFlip(Hx(a[0]), ulong.Parse(a[1]), ulong.Parse(a[2]));
                    }

                    break;
                case "arithmetic":
                    {
                        int w = int.Parse(a[3]);
                        got = Mutate.ByteArithmeticBig(Hx(a[0]), Bg(a[1]), Bg(a[2]), w, EndianOf(a[4]));
                        if (FitsU64(a[1]) && FitsI64(a[2]))
                        {
                            small = Mutate.ByteArithmetic(Hx(a[0]), ulong.Parse(a[1]), long.Parse(a[2]), w, EndianOf(a[4]));
                        }

                        break;
                    }

                case "boundary":
                    {
                        int w = int.Parse(a[2]);
                        got = Mutate.BoundaryInsertBig(Hx(a[0]), Bg(a[1]), w, Bg(a[3]), EndianOf(a[4]));
                        if (FitsU64(a[1]) && FitsU64(a[3]))
                        {
                            small = Mutate.BoundaryInsert(Hx(a[0]), ulong.Parse(a[1]), w, ulong.Parse(a[3]), EndianOf(a[4]));
                        }

                        break;
                    }

                case "block_delete":
                    got = Mutate.BlockDeleteBig(Hx(a[0]), Bg(a[1]), Bg(a[2]));
                    if (FitsU64(a[1]) && FitsU64(a[2]))
                    {
                        small = Mutate.BlockDelete(Hx(a[0]), ulong.Parse(a[1]), ulong.Parse(a[2]));
                    }

                    break;
                case "block_insert":
                    got = Mutate.BlockInsertBig(Hx(a[0]), Bg(a[1]), Hx(a[2]));
                    if (FitsU64(a[1]))
                    {
                        small = Mutate.BlockInsert(Hx(a[0]), ulong.Parse(a[1]), Hx(a[2]));
                    }

                    break;
                case "block_duplicate":
                    got = Mutate.BlockDuplicateBig(Hx(a[0]), Bg(a[1]), Bg(a[2]), Bg(a[3]));
                    if (FitsU64(a[1]) && FitsU64(a[2]) && FitsU64(a[3]))
                    {
                        small = Mutate.BlockDuplicate(Hx(a[0]), ulong.Parse(a[1]), ulong.Parse(a[2]), ulong.Parse(a[3]));
                    }

                    break;
                case "splice":
                    got = Mutate.SpliceBig(Hx(a[0]), Hx(a[1]), Bg(a[2]), Bg(a[3]));
                    if (FitsU64(a[2]) && FitsU64(a[3]))
                    {
                        small = Mutate.Splice(Hx(a[0]), Hx(a[1]), ulong.Parse(a[2]), ulong.Parse(a[3]));
                    }

                    break;
                case "sequence":
                    got = Mutate.MutateSequence(Hx(a[0]), Mutate.Low64(Bg(a[1])), int.Parse(a[2]), int.Parse(a[3]));
                    break;
                default:
                    Assert.Fail("unknown vector op " + v.Op);
                    return;
            }

            Assert.True(want.AsSpan().SequenceEqual(got), "vector " + i + " " + v.Op + " [" + string.Join(",", a) + "] (big path)");
            if (small != null)
            {
                ulongPaths++;
                Assert.True(want.AsSpan().SequenceEqual(small), "vector " + i + " " + v.Op + " [" + string.Join(",", a) + "] (ulong path)");
            }
        }

        foreach (string op in new[] { "bit_flip", "byte_flip", "arithmetic", "boundary", "block_delete", "block_insert", "block_duplicate", "splice", "sequence" })
        {
            Assert.True(counts.GetValueOrDefault(op) >= 50, "only " + counts.GetValueOrDefault(op) + " reference vectors for " + op);
        }

        Assert.True(ulongPaths > 250, "the ulong entry points must be exercised too: " + ulongPaths);
    }
}
