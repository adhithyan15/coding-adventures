using System;
using System.Collections.Generic;

namespace CodingAdventures.Gf256;

// Gf256.cs -- Byte arithmetic where addition is XOR and multiplication wraps in a field
// ======================================================================================
//
// GF(2^8) has exactly 256 elements: the byte values 0 through 255.
// The surprise is that arithmetic is not ordinary integer arithmetic:
//
//   add(a, b) = a XOR b
//
// because each bit is a coefficient in GF(2), where 1 + 1 = 0.
//
// For multiplication we work modulo the primitive polynomial 0x11D, which is
// the common Reed-Solomon choice used in QR and storage-style error correction.

/// <summary>
/// GF(2^8) arithmetic over the primitive polynomial 0x11D.
/// </summary>
public static class Gf256
{
    /// <summary>
    /// Package version.
    /// </summary>
    public const string VERSION = "0.1.0";

    /// <summary>
    /// Additive identity.
    /// </summary>
    public const byte ZERO = 0;

    /// <summary>
    /// Multiplicative identity.
    /// </summary>
    public const byte ONE = 1;

    /// <summary>
    /// Primitive polynomial x^8 + x^4 + x^3 + x^2 + 1.
    /// </summary>
    public const int PRIMITIVE_POLYNOMIAL = 0x11d;

    private static readonly int[] LogTable = new int[256];
    private static readonly int[] AlogTable = new int[256];

    static Gf256()
    {
        var value = 1;
        for (var index = 0; index < 255; index++)
        {
            AlogTable[index] = value;
            LogTable[value] = index;

            value <<= 1;
            if (value >= 256)
            {
                value ^= PRIMITIVE_POLYNOMIAL;
            }
        }

        AlogTable[255] = 1;
    }

    /// <summary>
    /// Antilog table where ALOG[i] = 2^i in GF(256).
    /// </summary>
    public static IReadOnlyList<int> ALOG => Array.AsReadOnly(AlogTable);

    /// <summary>
    /// Log table where LOG[x] = i such that 2^i = x in GF(256).
    /// </summary>
    public static IReadOnlyList<int> LOG => Array.AsReadOnly(LogTable);

    /// <summary>
    /// Add two field elements. In characteristic 2 this is XOR.
    /// </summary>
    public static byte Add(byte a, byte b) => (byte)(a ^ b);

    /// <summary>
    /// Subtract two field elements. In characteristic 2 this is also XOR.
    /// </summary>
    public static byte Subtract(byte a, byte b) => (byte)(a ^ b);

    /// <summary>
    /// Multiply two field elements using log and antilog tables.
    /// </summary>
    public static byte Multiply(byte a, byte b)
    {
        if (a == 0 || b == 0)
        {
            return 0;
        }

        var exponent = (LogTable[a] + LogTable[b]) % 255;
        return (byte)AlogTable[exponent];
    }

    /// <summary>
    /// Divide a by b in GF(256).
    /// </summary>
    public static byte Divide(byte a, byte b)
    {
        if (b == 0)
        {
            throw new InvalidOperationException("GF256: division by zero");
        }

        if (a == 0)
        {
            return 0;
        }

        var exponent = ((LogTable[a] - LogTable[b] + 255) % 255 + 255) % 255;
        return (byte)AlogTable[exponent];
    }

    /// <summary>
    /// Raise a field element to a non-negative integer power.
    /// </summary>
    public static byte Power(byte @base, int exponent)
    {
        if (exponent < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(exponent), "Exponent must be non-negative.");
        }

        if (@base == 0)
        {
            return exponent == 0 ? ONE : ZERO;
        }

        if (exponent == 0)
        {
            return ONE;
        }

        var tableIndex = ((LogTable[@base] * exponent) % 255 + 255) % 255;
        return (byte)AlogTable[tableIndex];
    }

    /// <summary>
    /// Compute the multiplicative inverse of a non-zero field element.
    /// </summary>
    public static byte Inverse(byte a)
    {
        if (a == 0)
        {
            throw new InvalidOperationException("GF256: zero has no multiplicative inverse");
        }

        return (byte)AlogTable[255 - LogTable[a]];
    }

    /// <summary>
    /// Return the additive identity.
    /// </summary>
    public static byte Zero() => ZERO;

    /// <summary>
    /// Return the multiplicative identity.
    /// </summary>
    public static byte One() => ONE;

    /// <summary>
    /// Create a GF(2^8) field for a different primitive polynomial.
    /// </summary>
    public static Gf256Field CreateField(int polynomial) => new(polynomial);
}

/// <summary>
/// GF(2^8) field parameterized by an arbitrary primitive polynomial.
/// </summary>
public sealed class Gf256Field
{
    private readonly byte _reduce;

    /// <summary>
    /// Create a field using the provided primitive polynomial.
    /// </summary>
    public Gf256Field(int polynomial)
    {
        if (polynomial <= 0 || polynomial > 0x1FF)
        {
            throw new ArgumentOutOfRangeException(nameof(polynomial), "Polynomial must fit in 9 bits.");
        }

        Polynomial = polynomial;
        _reduce = (byte)(polynomial & 0xFF);
    }

    /// <summary>
    /// Primitive polynomial backing this field instance.
    /// </summary>
    public int Polynomial { get; }

    /// <summary>
    /// Add two field elements.
    /// </summary>
    public byte Add(byte a, byte b) => (byte)(a ^ b);

    /// <summary>
    /// Subtract two field elements.
    /// </summary>
    public byte Subtract(byte a, byte b) => (byte)(a ^ b);

    /// <summary>
    /// Multiply two field elements via Russian peasant multiplication.
    /// </summary>
    public byte Multiply(byte a, byte b) => MultiplyCore(a, b);

    /// <summary>
    /// Divide a by b in this field.
    /// </summary>
    public byte Divide(byte a, byte b)
    {
        if (b == 0)
        {
            throw new InvalidOperationException("GF256Field: division by zero");
        }

        return MultiplyCore(a, Power(b, 254));
    }

    /// <summary>
    /// Raise a field element to a non-negative integer power.
    /// </summary>
    public byte Power(byte @base, int exponent)
    {
        if (exponent < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(exponent), "Exponent must be non-negative.");
        }

        if (@base == 0)
        {
            return exponent == 0 ? Gf256.ONE : Gf256.ZERO;
        }

        if (exponent == 0)
        {
            return Gf256.ONE;
        }

        var result = Gf256.ONE;
        var factor = @base;
        var remaining = exponent;

        while (remaining > 0)
        {
            if ((remaining & 1) != 0)
            {
                result = MultiplyCore(result, factor);
            }

            factor = MultiplyCore(factor, factor);
            remaining >>= 1;
        }

        return result;
    }

    /// <summary>
    /// Compute the multiplicative inverse of a non-zero field element.
    /// </summary>
    public byte Inverse(byte a)
    {
        if (a == 0)
        {
            throw new InvalidOperationException("GF256Field: zero has no multiplicative inverse");
        }

        return Power(a, 254);
    }

    private byte MultiplyCore(byte a, byte b)
    {
        var result = 0;
        var left = a;
        var right = b;

        for (var bit = 0; bit < 8; bit++)
        {
            if ((right & 1) != 0)
            {
                result ^= left;
            }

            var highBit = left & 0x80;
            left = (byte)(left << 1);
            if (highBit != 0)
            {
                left ^= _reduce;
            }

            right >>= 1;
        }

        return (byte)result;
    }
}
