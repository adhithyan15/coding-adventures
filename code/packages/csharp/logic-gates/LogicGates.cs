namespace CodingAdventures.LogicGates;

/// <summary>
/// Thrown when a gate receives a value that is not the bit 0 or 1.
/// </summary>
/// <remarks>
/// It subclasses <see cref="ArgumentOutOfRangeException"/> so existing
/// <c>catch (ArgumentOutOfRangeException)</c> callers keep working;
/// <see cref="ArgumentException.ParamName"/> names the offending parameter and
/// <see cref="Value"/> carries the rejected integer.
/// </remarks>
public sealed class InvalidBitException : ArgumentOutOfRangeException
{
    /// <summary>Create the exception for parameter <paramref name="parameterName"/>.</summary>
    public InvalidBitException(string parameterName, int value)
        : base(parameterName, value, $"Bit values must be 0 or 1, got {value}.")
    {
        Value = value;
    }

    /// <summary>The name of the offending parameter (same as <c>ParamName</c>).</summary>
    public string ParameterName => ParamName!;

    /// <summary>The rejected integer value.</summary>
    public int Value { get; }
}

/// <summary>
/// Core digital logic gates over integer bits.
/// </summary>
/// <remarks>
/// A bit is the <c>int</c> 0 or 1; anything else throws
/// <see cref="InvalidBitException"/>. C# implicitly widens <c>char</c>,
/// <c>byte</c>, <c>sbyte</c>, <c>short</c> and <c>ushort</c> to <c>int</c>, so
/// <c>And('a', 1)</c> compiles and is rejected at run time as 97 (and
/// <c>And((byte)1, 1)</c> is accepted as the bit 1). <c>long</c>, <c>uint</c>,
/// <c>double</c> and <c>bool</c> have no implicit conversion and fail to
/// compile. Every gate validates all of its own arguments up front, in
/// parameter order, so the first invalid parameter is the one reported -- never
/// an inner helper's parameter name.
/// </remarks>
public static class LogicGates
{
    private static void ValidateBit(int value, string name)
    {
        if (value is not (0 or 1))
        {
            throw new InvalidBitException(name, value);
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
    public static int Nand(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return a == 1 && b == 1 ? 0 : 1;
    }

    /// <summary>Return NOT(OR(a, b)).</summary>
    public static int Nor(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return a == 1 || b == 1 ? 0 : 1;
    }

    /// <summary>Return NOT(XOR(a, b)).</summary>
    public static int Xnor(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return a == b ? 1 : 0;
    }

    /// <summary>Build NOT from NAND only.</summary>
    public static int NandNot(int a)
    {
        ValidateBit(a, nameof(a));
        return Nand(a, a);
    }

    /// <summary>Build AND from NAND only.</summary>
    public static int NandAnd(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return NandNot(Nand(a, b));
    }

    /// <summary>Build OR from NAND only.</summary>
    public static int NandOr(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
        return Nand(NandNot(a), NandNot(b));
    }

    /// <summary>Build XOR from NAND only.</summary>
    public static int NandXor(int a, int b)
    {
        ValidateBit(a, nameof(a));
        ValidateBit(b, nameof(b));
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

        for (var i = 0; i < inputs.Length; i++)
        {
            ValidateBit(inputs[i], $"inputs[{i}]");
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

        for (var i = 0; i < inputs.Length; i++)
        {
            ValidateBit(inputs[i], $"inputs[{i}]");
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
