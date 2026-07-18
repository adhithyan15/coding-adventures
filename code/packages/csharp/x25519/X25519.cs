using System.Numerics;

namespace CodingAdventures.X25519;

/// <summary>
/// X25519 elliptic-curve Diffie-Hellman from RFC 7748.
/// </summary>
/// <remarks>
/// This educational implementation uses <see cref="BigInteger"/> field
/// arithmetic. The Montgomery ladder has a fixed algorithmic shape, but
/// <see cref="BigInteger"/> is not guaranteed to execute in constant time.
/// </remarks>
public static class Curve25519
{
    /// <summary>The encoded size of an X25519 scalar or u-coordinate.</summary>
    public const int KeyLength = 32;

    private static readonly BigInteger Prime = (BigInteger.One << 255) - 19;
    private static readonly BigInteger PrimeMinusTwo = Prime - 2;
    private static readonly BigInteger A24 = 121665;

    private static readonly byte[] BasePointBytes =
        [9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    /// <summary>Return the standard Curve25519 base point, u = 9.</summary>
    public static byte[] BasePoint => BasePointBytes.ToArray();

    /// <summary>
    /// Multiply a 32-byte scalar by a 32-byte Montgomery u-coordinate.
    /// </summary>
    /// <exception cref="ArgumentNullException">An input is null.</exception>
    /// <exception cref="ArgumentException">An input is not exactly 32 bytes.</exception>
    /// <exception cref="InvalidOperationException">The result is all zero.</exception>
    public static byte[] Compute(byte[] scalar, byte[] uCoordinate)
    {
        ValidateInput(scalar, nameof(scalar));
        ValidateInput(uCoordinate, nameof(uCoordinate));

        var k = DecodeScalar(scalar);
        var u = DecodeUCoordinate(uCoordinate);

        var x1 = u;
        var x2 = BigInteger.One;
        var z2 = BigInteger.Zero;
        var x3 = u;
        var z3 = BigInteger.One;
        var swap = 0;

        for (var bit = 254; bit >= 0; bit--)
        {
            var scalarBit = (int)((k >> bit) & BigInteger.One);
            swap ^= scalarBit;
            ConditionalSwap(swap, ref x2, ref x3);
            ConditionalSwap(swap, ref z2, ref z3);
            swap = scalarBit;

            var a = Add(x2, z2);
            var aa = Square(a);
            var b = Subtract(x2, z2);
            var bb = Square(b);
            var e = Subtract(aa, bb);
            var c = Add(x3, z3);
            var d = Subtract(x3, z3);
            var da = Multiply(d, a);
            var cb = Multiply(c, b);

            x3 = Square(Add(da, cb));
            z3 = Multiply(x1, Square(Subtract(da, cb)));
            x2 = Multiply(aa, bb);
            z2 = Multiply(e, Add(aa, Multiply(A24, e)));
        }

        ConditionalSwap(swap, ref x2, ref x3);
        ConditionalSwap(swap, ref z2, ref z3);

        var affine = Multiply(x2, BigInteger.ModPow(z2, PrimeMinusTwo, Prime));
        var encoded = EncodeUCoordinate(affine);
        if (encoded.All(value => value == 0))
        {
            throw new InvalidOperationException(
                "X25519 produced the all-zero output (low-order point).");
        }

        return encoded;
    }

    /// <summary>Derive a public key by multiplying by the base point.</summary>
    public static byte[] ComputeBase(byte[] scalar) => Compute(scalar, BasePointBytes);

    /// <summary>Derive the public key for a 32-byte private key.</summary>
    public static byte[] GenerateKeypair(byte[] privateKey) => ComputeBase(privateKey);

    private static void ValidateInput(byte[]? value, string parameterName)
    {
        ArgumentNullException.ThrowIfNull(value, parameterName);
        if (value.Length != KeyLength)
        {
            throw new ArgumentException("X25519 inputs must be exactly 32 bytes.", parameterName);
        }
    }

    private static BigInteger DecodeScalar(byte[] scalar)
    {
        var clamped = scalar.ToArray();
        clamped[0] &= 248;
        clamped[31] &= 127;
        clamped[31] |= 64;
        return new BigInteger(clamped, isUnsigned: true, isBigEndian: false);
    }

    private static BigInteger DecodeUCoordinate(byte[] uCoordinate)
    {
        var masked = uCoordinate.ToArray();
        masked[31] &= 127;
        return new BigInteger(masked, isUnsigned: true, isBigEndian: false);
    }

    private static byte[] EncodeUCoordinate(BigInteger value)
    {
        var encoded = Mod(value).ToByteArray(isUnsigned: true, isBigEndian: false);
        var result = new byte[KeyLength];
        encoded.CopyTo(result, 0);
        return result;
    }

    private static void ConditionalSwap(int swap, ref BigInteger left, ref BigInteger right)
    {
        var mask = -new BigInteger(swap);
        var difference = mask & (left ^ right);
        left ^= difference;
        right ^= difference;
    }

    private static BigInteger Add(BigInteger left, BigInteger right) => Mod(left + right);

    private static BigInteger Subtract(BigInteger left, BigInteger right) => Mod(left - right);

    private static BigInteger Multiply(BigInteger left, BigInteger right) => Mod(left * right);

    private static BigInteger Square(BigInteger value) => Mod(value * value);

    private static BigInteger Mod(BigInteger value)
    {
        var reduced = value % Prime;
        return reduced.Sign < 0 ? reduced + Prime : reduced;
    }
}
