using Gates = CodingAdventures.LogicGates.LogicGates;

namespace CodingAdventures.Arithmetic;

/// <summary>
/// Result of a ripple-carry addition.
/// </summary>
public sealed record RippleCarryResult(IReadOnlyList<int> Sum, int CarryOut, bool Overflow);

/// <summary>
/// Adder circuits built from logic gates.
/// </summary>
public static class Adders
{
    /// <summary>Add two bits and return sum and carry.</summary>
    public static (int Sum, int Carry) HalfAdder(int a, int b) =>
        (Gates.Xor(a, b), Gates.And(a, b));

    /// <summary>Add two bits plus carry-in and return sum and carry-out.</summary>
    public static (int Sum, int CarryOut) FullAdder(int a, int b, int carryIn)
    {
        var (partialSum, partialCarry) = HalfAdder(a, b);
        var (sum, carry2) = HalfAdder(partialSum, carryIn);
        return (sum, Gates.Or(partialCarry, carry2));
    }

    /// <summary>Add two same-width bit vectors with an optional carry-in.</summary>
    public static RippleCarryResult RippleCarryAdder(
        IReadOnlyList<int> a,
        IReadOnlyList<int> b,
        int carryIn = 0)
    {
        ArgumentNullException.ThrowIfNull(a);
        ArgumentNullException.ThrowIfNull(b);
        if (a.Count != b.Count)
        {
            throw new ArgumentException($"a and b must have the same length, got {a.Count} and {b.Count}.");
        }

        if (a.Count == 0)
        {
            throw new ArgumentException("bit lists must not be empty.");
        }

        var sum = new List<int>(a.Count);
        var carry = carryIn;
        for (var i = 0; i < a.Count; i++)
        {
            var (sumBit, carryOut) = FullAdder(a[i], b[i], carry);
            sum.Add(sumBit);
            carry = carryOut;
        }

        var aSign = a[^1];
        var bSign = b[^1];
        var resultSign = sum[^1];
        var overflow = aSign == bSign && resultSign != aSign;

        return new RippleCarryResult(sum.AsReadOnly(), carry, overflow);
    }
}

/// <summary>
/// ALU operation codes.
/// </summary>
public enum AluOp
{
    /// <summary>Add A and B.</summary>
    Add,
    /// <summary>Subtract B from A.</summary>
    Sub,
    /// <summary>Bitwise AND.</summary>
    And,
    /// <summary>Bitwise OR.</summary>
    Or,
    /// <summary>Bitwise XOR.</summary>
    Xor,
    /// <summary>Bitwise NOT of A.</summary>
    Not,
}

/// <summary>
/// Result of an ALU operation and its status flags.
/// </summary>
public sealed record AluResult(
    IReadOnlyList<int> Value,
    bool Zero,
    bool Carry,
    bool Negative,
    bool Overflow)
{
    /// <summary>Alias for consumers following the Rust package name.</summary>
    public IReadOnlyList<int> Result => Value;
}

/// <summary>
/// N-bit arithmetic logic unit.
/// </summary>
public sealed class Alu
{
    /// <summary>Create an ALU with the requested bit width.</summary>
    public Alu(int bitWidth = 8)
    {
        if (bitWidth < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(bitWidth), "bit_width must be at least 1.");
        }

        BitWidth = bitWidth;
    }

    /// <summary>The number of bits expected in operands.</summary>
    public int BitWidth { get; }

    /// <summary>Execute an operation over LSB-first bit vectors.</summary>
    public AluResult Execute(AluOp op, IReadOnlyList<int> a, IReadOnlyList<int> b)
    {
        ArgumentNullException.ThrowIfNull(a);
        ArgumentNullException.ThrowIfNull(b);
        if (a.Count != BitWidth)
        {
            throw new ArgumentException($"a must have {BitWidth} bits, got {a.Count}.", nameof(a));
        }

        if (op != AluOp.Not && b.Count != BitWidth)
        {
            throw new ArgumentException($"b must have {BitWidth} bits, got {b.Count}.", nameof(b));
        }

        bool carry;
        IReadOnlyList<int> value;
        switch (op)
        {
            case AluOp.Add:
            {
                var result = Adders.RippleCarryAdder(a, b);
                value = result.Sum;
                carry = result.CarryOut == 1;
                break;
            }

            case AluOp.Sub:
            {
                var notB = b.Select(Gates.Not).ToArray();
                var result = Adders.RippleCarryAdder(a, notB, carryIn: 1);
                value = result.Sum;
                carry = result.CarryOut == 1;
                break;
            }

            case AluOp.And:
                value = Bitwise(a, b, Gates.And);
                carry = false;
                break;

            case AluOp.Or:
                value = Bitwise(a, b, Gates.Or);
                carry = false;
                break;

            case AluOp.Xor:
                value = Bitwise(a, b, Gates.Xor);
                carry = false;
                break;

            case AluOp.Not:
                value = a.Select(Gates.Not).ToArray();
                carry = false;
                break;

            default:
                throw new ArgumentOutOfRangeException(nameof(op), op, "Unknown ALU operation.");
        }

        var zero = value.All(bit => bit == 0);
        var negative = value.Count > 0 && value[^1] == 1;
        var overflow = SignedOverflow(op, a, b, value);

        return new AluResult(value, zero, carry, negative, overflow);
    }

    private static IReadOnlyList<int> Bitwise(
        IReadOnlyList<int> a,
        IReadOnlyList<int> b,
        Func<int, int, int> gate)
    {
        var result = new int[a.Count];
        for (var i = 0; i < a.Count; i++)
        {
            result[i] = gate(a[i], b[i]);
        }

        return result;
    }

    private static bool SignedOverflow(
        AluOp op,
        IReadOnlyList<int> a,
        IReadOnlyList<int> b,
        IReadOnlyList<int> result)
    {
        if (op is not (AluOp.Add or AluOp.Sub))
        {
            return false;
        }

        var aSign = a[^1];
        var bSign = op == AluOp.Add ? b[^1] : Gates.Not(b[^1]);
        var resultSign = result[^1];
        return aSign == bSign && resultSign != aSign;
    }
}

/// <summary>
/// Package metadata.
/// </summary>
public static class ArithmeticPackage
{
    /// <summary>The package version.</summary>
    public const string Version = "0.1.0";
}
