namespace CodingAdventures.LogicGates.Tests;

public sealed class LogicGatesExhaustiveTests
{
    private static readonly int[] Bits = [0, 1];

    private static readonly int[] BadValues = BuildBadValues();

    private static int[] BuildBadValues()
    {
        var list = new List<int>();
        for (var v = -1024; v <= 1024; v++)
        {
            if (v != 0 && v != 1)
            {
                list.Add(v);
            }
        }

        list.AddRange([int.MinValue, int.MinValue + 1, int.MaxValue - 1, int.MaxValue]);
        return [.. list];
    }

    // Expected outputs for (0,0),(0,1),(1,0),(1,1); unary gates: input 0, input 1.
    private static readonly (string Name, int Arity, Func<int, int, int> F, int[] Expected)[] Gates =
    [
        ("Not", 1, (a, _) => LogicGates.Not(a), [1, 0]),
        ("And", 2, LogicGates.And, [0, 0, 0, 1]),
        ("Or", 2, LogicGates.Or, [0, 1, 1, 1]),
        ("Xor", 2, LogicGates.Xor, [0, 1, 1, 0]),
        ("Nand", 2, LogicGates.Nand, [1, 1, 1, 0]),
        ("Nor", 2, LogicGates.Nor, [1, 0, 0, 0]),
        ("Xnor", 2, LogicGates.Xnor, [1, 0, 0, 1]),
        ("NandNot", 1, (a, _) => LogicGates.NandNot(a), [1, 0]),
        ("NandAnd", 2, LogicGates.NandAnd, [0, 0, 0, 1]),
        ("NandOr", 2, LogicGates.NandOr, [0, 1, 1, 1]),
        ("NandXor", 2, LogicGates.NandXor, [0, 1, 1, 0]),
    ];

    [Fact]
    public void LiteralTruthTablesForAllElevenGates()
    {
        Assert.Equal(11, Gates.Length);
        foreach (var (name, arity, f, expected) in Gates)
        {
            if (arity == 1)
            {
                Assert.True(expected[0] == f(0, 0), name);
                Assert.True(expected[1] == f(1, 0), name);
                continue;
            }

            var i = 0;
            foreach (var a in Bits)
            {
                foreach (var b in Bits)
                {
                    Assert.True(expected[i] == f(a, b), $"{name}({a},{b})");
                    i++;
                }
            }
        }
    }

    [Fact]
    public void NandBuiltEqualsDirect()
    {
        foreach (var a in Bits)
        {
            Assert.Equal(LogicGates.Not(a), LogicGates.NandNot(a));
            foreach (var b in Bits)
            {
                Assert.Equal(LogicGates.And(a, b), LogicGates.NandAnd(a, b));
                Assert.Equal(LogicGates.Or(a, b), LogicGates.NandOr(a, b));
                Assert.Equal(LogicGates.Xor(a, b), LogicGates.NandXor(a, b));
            }
        }
    }

    [Fact]
    public void ValidInputsNeverThrowAndResultsAreBits()
    {
        foreach (var (name, _, f, _) in Gates)
        {
            foreach (var a in Bits)
            {
                foreach (var b in Bits)
                {
                    var r = f(a, b);
                    Assert.True(r is 0 or 1, $"{name}({a},{b}) = {r}");
                }
            }
        }
    }

    private static void AssertRejected(Func<int> call, string param, int value)
    {
        var ex = Assert.Throws<InvalidBitException>(() => { call(); });
        Assert.Equal(param, ex.ParamName);
        Assert.Equal(param, ex.ParameterName);
        Assert.Equal(value, ex.Value);
        Assert.IsAssignableFrom<ArgumentOutOfRangeException>(ex);
    }

    [Fact]
    public void EveryBadValueInEveryPositionIsRejectedWithRightNameAndValue()
    {
        foreach (var (_, arity, f, _) in Gates)
        {
            foreach (var bad in BadValues)
            {
                if (arity == 1)
                {
                    AssertRejected(() => f(bad, 0), "a", bad);
                    continue;
                }

                foreach (var good in Bits)
                {
                    AssertRejected(() => f(bad, good), "a", bad);
                    AssertRejected(() => f(good, bad), "b", bad);
                }

                // Both invalid: the first parameter is reported.
                AssertRejected(() => f(bad, bad), "a", bad);
                AssertRejected(() => f(bad, 7), "a", bad);
            }
        }
    }

    [Fact]
    public void NandBuiltGatesDoNotLeakInnerParameterNames()
    {
        AssertRejected(() => LogicGates.NandOr(1, 2), "b", 2);
        AssertRejected(() => LogicGates.NandOr(0, -1), "b", -1);
        AssertRejected(() => LogicGates.NandXor(1, 2), "b", 2);
        AssertRejected(() => LogicGates.NandAnd(1, 2), "b", 2);
    }

    [Fact]
    public void MultiInputGatesValidateAllInputsUpFront()
    {
        var ex = Assert.Throws<InvalidBitException>(() => { LogicGates.AndN(1, 1, 5); });
        Assert.Equal("inputs[2]", ex.ParamName);
        Assert.Equal(5, ex.Value);
        ex = Assert.Throws<InvalidBitException>(() => { LogicGates.OrN(0, 0, 0, -3); });
        Assert.Equal("inputs[3]", ex.ParamName);
        ex = Assert.Throws<InvalidBitException>(() => { LogicGates.XorN(1, 2); });
        Assert.Equal("inputs[1]", ex.ParamName);
    }

    // C# implicitly widens these to int, so they compile and are rejected at
    // runtime by value (long/uint/double/bool do not convert at all).
    [Fact]
    public void ImplicitlyWidenedNonIntArgumentsAreRejectedByValue()
    {
        char c = 'a';
        byte by = 7;
        sbyte sb = -1;
        short sh = 300;
        ushort us = 65535;

        AssertRejected(() => LogicGates.And(c, 1), "a", 97);
        AssertRejected(() => LogicGates.Or(0, by), "b", 7);
        AssertRejected(() => LogicGates.Not(sb), "a", -1);
        AssertRejected(() => LogicGates.Xor(sh, 0), "a", 300);
        AssertRejected(() => LogicGates.Nand(1, us), "b", 65535);
        AssertRejected(() => LogicGates.NandOr(1, c), "b", 97);

        // A widened value that happens to be 0 or 1 is a valid bit.
        byte one = 1;
        Assert.Equal(1, LogicGates.And(one, one));
    }
}

public sealed class LogicGatesNaryTests
{
    private static readonly int[] Vals = [-1, 0, 1, 2];

    private delegate int Gate(params int[] inputs);

    private static IEnumerable<(string Name, Gate G, int Min, Func<int[], int> Ref)> Gates()
    {
        yield return ("AndN", LogicGates.AndN, 2, a => a.All(x => x == 1) ? 1 : 0);
        yield return ("OrN", LogicGates.OrN, 2, a => a.Any(x => x == 1) ? 1 : 0);
        yield return ("XorN", LogicGates.XorN, 0, a => a.Sum() % 2);
    }

    [Fact]
    public void NullIsArgumentNull()
    {
        Assert.Throws<ArgumentNullException>(() => LogicGates.AndN(null!));
        Assert.Throws<ArgumentNullException>(() => LogicGates.OrN(null!));
        Assert.Throws<ArgumentNullException>(() => LogicGates.XorN(null!));
        // `AndN(null)` without the bang binds to the params array as null too.
        Assert.Throws<ArgumentNullException>(() => LogicGates.XorN((int[])null!));
    }

    [Fact]
    public void LengthCap()
    {
        foreach (var (_, g, _, _) in Gates())
        {
            Assert.Equal(0, g(new int[LogicGates.MaxInputs]));
            var ex = Assert.Throws<ArgumentException>(() => g(new int[LogicGates.MaxInputs + 1]));
            Assert.IsNotType<InvalidBitException>(ex);
            Assert.Equal("inputs", ex.ParamName);
        }
    }

    [Fact]
    public void ShapeWinsOverBitErrors()
    {
        var big = new int[LogicGates.MaxInputs + 1];
        big[0] = 7;
        foreach (var (_, g, _, _) in Gates())
        {
            Assert.Throws<ArgumentException>(() => g(big));
        }

        Assert.Throws<ArgumentException>(() => LogicGates.AndN(9));
        Assert.Throws<ArgumentException>(() => LogicGates.OrN(9));
    }

    [Fact]
    public void FirstBadElementIsNamed()
    {
        var a = new int[100];
        a[41] = 3;
        a[77] = -1;
        foreach (var (_, g, _, _) in Gates())
        {
            var ex = Assert.Throws<InvalidBitException>(() => g(a));
            Assert.Equal("inputs[41]", ex.ParamName);
            Assert.Equal(3, ex.Value);
        }
    }

    [Fact]
    public void ExhaustiveSmallLengths()
    {
        foreach (var (name, g, min, reference) in Gates())
        {
            for (var len = 0; len <= 8; len++)
            {
                var total = 1;
                for (var k = 0; k < len; k++) total *= Vals.Length;
                for (var code = 0; code < total; code++)
                {
                    var arr = new int[len];
                    var c = code;
                    for (var k = 0; k < len; k++) { arr[k] = Vals[c % 4]; c /= 4; }
                    var before = (int[])arr.Clone();

                    if (len < min)
                    {
                        Assert.Throws<ArgumentException>(() => g(arr));
                    }
                    else
                    {
                        var bad = Array.FindIndex(arr, x => x is not (0 or 1));
                        if (bad >= 0)
                        {
                            var ex = Assert.Throws<InvalidBitException>(() => g(arr));
                            Assert.Equal($"inputs[{bad}]", ex.ParamName);
                            Assert.Equal(arr[bad], ex.Value);
                        }
                        else
                        {
                            Assert.Equal(reference(arr), g(arr));
                        }
                    }

                    Assert.True(before.SequenceEqual(arr), name + " mutated caller array");
                }
            }
        }
    }
}
