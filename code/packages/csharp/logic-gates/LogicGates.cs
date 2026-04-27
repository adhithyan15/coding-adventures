namespace CodingAdventures.LogicGates;

/// <summary>
/// Core digital logic gates over integer bits.
/// </summary>
public static class LogicGates
{
    private static void ValidateBit(int value, string name)
    {
        if (value is not (0 or 1))
        {
            throw new ArgumentOutOfRangeException(name, value, "Bit values must be 0 or 1.");
        }
    }

    /// <summary>Invert a bit.</summary>
    public static int Not(int a)
    {
        ValidateBit(a, nameof(a));
        return a == 0 ? 1 : 0;
    }

    /// <summary>Return 1 only when both inputs are 1.</summary>
    public static int And(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return a == 1 && b == 1 ? 1 : 0;
    }

    /// <summary>Return 1 when either input is 1.</summary>
    public static int Or(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return a == 1 || b == 1 ? 1 : 0;
    }

    /// <summary>Return 1 when inputs differ.</summary>
    public static int Xor(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return a != b ? 1 : 0;
    }

    /// <summary>Return NOT(AND(a, b)).</summary>
    public static int Nand(int a, int b) => Not(And(a, b));

    /// <summary>Return NOT(OR(a, b)).</summary>
    public static int Nor(int a, int b) => Not(Or(a, b));

    /// <summary>Return NOT(XOR(a, b)).</summary>
    public static int Xnor(int a, int b) => Not(Xor(a, b));

    /// <summary>Build NOT from NAND only.</summary>
    public static int NandNot(int a) => Nand(a, a);

    /// <summary>Build AND from NAND only.</summary>
    public static int NandAnd(int a, int b) => NandNot(Nand(a, b));

    /// <summary>Build OR from NAND only.</summary>
    public static int NandOr(int a, int b) => Nand(NandNot(a), NandNot(b));

    /// <summary>Build XOR from NAND only.</summary>
    public static int NandXor(int a, int b)
    {
        var nand = Nand(a, b);
        return Nand(Nand(a, nand), Nand(b, nand));
    }

    /// <summary>AND over two or more inputs.</summary>
    public static int AndN(params int[] inputs)
    {
        ArgumentNullException.ThrowIfNull(inputs);
        if (inputs.Length < 2)
        {
            throw new ArgumentException("AndN requires at least two inputs.", nameof(inputs));
        }

        var result = And(inputs[0], inputs[1]);
        for (var i = 2; i < inputs.Length; i++)
        {
            result = And(result, inputs[i]);
        }

        return result;
    }

    /// <summary>OR over two or more inputs.</summary>
    public static int OrN(params int[] inputs)
    {
        ArgumentNullException.ThrowIfNull(inputs);
        if (inputs.Length < 2)
        {
            throw new ArgumentException("OrN requires at least two inputs.", nameof(inputs));
        }

        var result = Or(inputs[0], inputs[1]);
        for (var i = 2; i < inputs.Length; i++)
        {
            result = Or(result, inputs[i]);
        }

        return result;
    }

    /// <summary>Return odd parity over zero or more inputs.</summary>
    public static int XorN(params int[] inputs)
    {
        ArgumentNullException.ThrowIfNull(inputs);

        var result = 0;
        for (var i = 0; i < inputs.Length; i++)
        {
            ValidateBit(inputs[i], $"inputs[{i}]");
            result = Xor(result, inputs[i]);
        }

        return result;
    }
}
