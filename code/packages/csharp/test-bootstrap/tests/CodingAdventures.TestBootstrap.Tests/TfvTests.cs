using CodingAdventures.TestBootstrap.Tfv;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>The strict TFV v1 validator that backs the corpus_reject cases.</summary>
public sealed class TfvTests : HookedTest
{
    [Fact]
    public void AcceptsWellFormedDocuments()
    {
        string[] good =
        {
            "tfv 1\n",
            "tfv 1",
            "tfv 1\n# a comment\n\nr R0\n",
            "tfv 1\nr R1 Kop S706274\n",
            "tfv 1\nr R2 Ka N Kb T\n",
            "tfv 1\nr R1 Kv I-12\n",
            "tfv 1\nr R1 Kv I0\n",
            "tfv 1\nr R1 Kv F\n",
            "tfv 1\nr R1 Kv D3ff0000000000000\n",
            "tfv 1\nr R1 Kv Xff00\n",
            "tfv 1\nr R1 Kv X\n",
            "tfv 1\nr R1 Kv L2 I1 I2\n",
            "tfv 1\nr R1 Kv L0\n",
            "tfv 1\nr R1 Kv M2 S61 I1 S62 I2\n",
            "tfv 1\nr R1 Kv M0\n",
            "tfv 1\nr1/a.b_c-d R0\nR2 R0\n",
            "tfv 1\nr R1 Kv S" + "c3a9" + "\n",
        };
        foreach (string text in good)
        {
            TfvValidator.Validate(text);
            Assert.True(TfvValidator.IsValid(text), text);
        }
    }

    [Fact]
    public void RejectsMalformedDocuments()
    {
        string deep64 = "tfv 1\nr R1 Kv " + string.Concat(Enumerable.Repeat("L1 ", 64)) + "N\n";
        string deep65 = "tfv 1\nr R1 Kv " + string.Concat(Enumerable.Repeat("L1 ", 65)) + "N\n";
        Assert.True(TfvValidator.IsValid(deep64));
        Dictionary<string, string> bad = new()
        {
            ["empty"] = string.Empty,
            ["bad header"] = "bad header\n",
            ["header case"] = "TFV 1\n",
            ["header version"] = "tfv 2\n",
            ["header trailing space"] = "tfv 1 \n",
            ["uppercase hex"] = "tfv 1\nr R2 Kop S706274 Kdata XFF\n",
            ["odd hex"] = "tfv 1\nr R1 Kv Xf\n",
            ["odd text hex"] = "tfv 1\nr R1 Kv Sf\n",
            ["negative zero"] = "tfv 1\nr R1 Kv I-0\n",
            ["leading zero"] = "tfv 1\nr R1 Kv I007\n",
            ["empty int"] = "tfv 1\nr R1 Kv I\n",
            ["int junk"] = "tfv 1\nr R1 Kv I1_0\n",
            ["int plus"] = "tfv 1\nr R1 Kv I+1\n",
            ["huge int"] = "tfv 1\nr R1 Kv I" + new string('9', 1001) + "\n",
            ["short float"] = "tfv 1\nr R1 Kv D3ff\n",
            ["upper float"] = "tfv 1\nr R1 Kv D3FF0000000000000\n",
            ["null with body"] = "tfv 1\nr R1 Kv N1\n",
            ["unknown tag"] = "tfv 1\nr R1 Kv Q1\n",
            ["count too big"] = "tfv 1\nr R1 Kv L3 I1 I2\n",
            ["count too small"] = "tfv 1\nr R1 Kv L1 I1 I2\n",
            ["leading zero count"] = "tfv 1\nr R01 Kv N\n",
            ["empty count"] = "tfv 1\nr R\n",
            ["count not digits"] = "tfv 1\nr Rx\n",
            ["giant count"] = "tfv 1\nr R1 Kv L1000000000 N\n",
            ["giant record count"] = "tfv 1\nr R99999999999\n",
            ["trailing token"] = "tfv 1\nr R1 Kv N N\n",
            ["double space"] = "tfv 1\nr R1  Kv N\n",
            ["trailing space"] = "tfv 1\nr R1 Kv N \n",
            ["leading space"] = "tfv 1\n r R1 Kv N\n",
            ["duplicate field"] = "tfv 1\nr R2 Ka N Ka T\n",
            ["duplicate record"] = "tfv 1\nr R0\nr R0\n",
            ["bad field name"] = "tfv 1\nr R1 KA N\n",
            ["field name digit start"] = "tfv 1\nr R1 K1 N\n",
            ["empty field name"] = "tfv 1\nr R1 K N\n",
            ["missing K"] = "tfv 1\nr R1 v N\n",
            ["bad id"] = "tfv 1\nr! R0\n",
            ["missing R"] = "tfv 1\nr X0\n",
            ["id only"] = "tfv 1\nr\n",
            ["carriage return"] = "tfv 1\r\nr R0\n",
            ["cr in record"] = "tfv 1\nr R1 Kv N\r\n",
            ["bad utf8 text"] = "tfv 1\nr R1 Kv Sff\n",
            ["duplicate map key"] = "tfv 1\nr R1 Kv M2 I1 N I1 T\n",
            ["duplicate nan keys"] = "tfv 1\nr R1 Kv M2 D7ff8000000000000 N D7ff0000000000001 T\n",
            ["too deep"] = deep65,
            ["lone surrogate"] = "tfv 1\nr R0\n\ud800",
            ["oversize"] = "tfv 1\n#" + new string('x', TfvValidator.MaxInput) + "\n",
        };
        foreach (KeyValuePair<string, string> e in bad)
        {
            Assert.False(TfvValidator.IsValid(e.Value), e.Key + ": accepted");
            Assert.Throws<InvalidTfvException>(() => TfvValidator.Validate(e.Value));
        }
    }

    [Fact]
    public void MapKeysAreComparedByDeepEquality()
    {
        // Same text, different order of an inner map: deep-equal keys, so the outer map has a duplicate.
        string dup = "tfv 1\nr R1 Kv M2 M2 S61 I1 S62 I2 N M2 S62 I2 S61 I1 T\n";
        Assert.False(TfvValidator.IsValid(dup));
        string distinct = "tfv 1\nr R1 Kv M2 M1 S61 I1 N M1 S61 I2 T\n";
        Assert.True(TfvValidator.IsValid(distinct));
        // Signed zeros differ; NaNs coincide.
        Assert.True(TfvValidator.IsValid("tfv 1\nr R1 Kv M2 D0000000000000000 N D8000000000000000 T\n"));
        Assert.False(TfvValidator.IsValid("tfv 1\nr R1 Kv M2 D7ff8000000000000 N D7ff8000000000001 T\n"));
    }

    [Fact]
    public void ErrorsNameTheLineAndClipTokens()
    {
        InvalidTfvException e = Assert.Throws<InvalidTfvException>(() => TfvValidator.Validate("tfv 1\nr R0\nq R1 Kv I007\n"));
        Assert.Contains("line 3", e.Message);
        Assert.StartsWith("invalid TFV: ", e.Message, StringComparison.Ordinal);
        InvalidTfvException dup = Assert.Throws<InvalidTfvException>(() => TfvValidator.Validate("tfv 1\nr R0\nr R0\n"));
        Assert.Contains("line 3", dup.Message);
        Assert.Contains("duplicate record id", dup.Message);
        InvalidTfvException clipped = Assert.Throws<InvalidTfvException>(() => TfvValidator.Validate("tfv 1\nr R1 Kv X" + new string('G', 500) + "\n"));
        Assert.True(clipped.Message.Length < 250, clipped.Message.Length.ToString());
        Assert.Contains("...", clipped.Message);
        Assert.Throws<ArgumentNullException>(() => TfvValidator.Validate(null!));
    }

    [Fact]
    public void CountsDoNotReserveMemory()
    {
        // A claimed count of a billion elements must fail fast, with no allocation that scales with it.
        long before = GC.GetAllocatedBytesForCurrentThread();
        Assert.False(TfvValidator.IsValid("tfv 1\nr R1 Kv L999999999 N\n"));
        Assert.False(TfvValidator.IsValid("tfv 1\nr R1 Kv M999999999 N N\n"));
        Assert.True(GC.GetAllocatedBytesForCurrentThread() - before < 1_000_000);
    }
}
