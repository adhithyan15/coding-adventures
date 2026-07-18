using System.Numerics;
using CodingAdventures.Sha512;
using Sha512Algorithm = CodingAdventures.Sha512.Sha512;

namespace CodingAdventures.Ed25519;

/// <summary>
/// Ed25519 deterministic digital signatures from RFC 8032.
/// </summary>
/// <remarks>
/// This educational implementation uses <see cref="BigInteger"/> field
/// arithmetic. It is written for clarity and is not guaranteed to execute in
/// constant time.
/// </remarks>
public static class Ed25519
{
    /// <summary>The encoded seed and public-key length.</summary>
    public const int KeyLength = 32;

    /// <summary>The secret-key and signature length.</summary>
    public const int ExtendedLength = 64;

    private static readonly BigInteger Prime = (BigInteger.One << 255) - 19;
    private static readonly BigInteger GroupOrder =
        (BigInteger.One << 252) + BigInteger.Parse("27742317777372353535851937790883648493");
    private static readonly BigInteger D = Multiply(-121665, Invert(121666));
    private static readonly BigInteger SqrtMinusOne =
        BigInteger.ModPow(2, (Prime - 1) / 4, Prime);
    private static readonly Point Identity =
        new(BigInteger.Zero, BigInteger.One, BigInteger.One, BigInteger.Zero);
    private static readonly Point BasePoint = CreateBasePoint();

    /// <summary>
    /// Generate a public key and 64-byte <c>seed || publicKey</c> secret key.
    /// </summary>
    public static (byte[] PublicKey, byte[] SecretKey) GenerateKeypair(byte[] seed)
    {
        ValidateLength(seed, KeyLength, nameof(seed), "seed");

        var digest = Sha512Algorithm.Hash(seed);
        var scalar = ClampScalar(digest.AsSpan(0, KeyLength));
        var publicKey = EncodePoint(ScalarMultiply(scalar, BasePoint));
        var secretKey = new byte[ExtendedLength];
        seed.CopyTo(secretKey, 0);
        publicKey.CopyTo(secretKey, KeyLength);
        return (publicKey, secretKey);
    }

    /// <summary>Sign a message with a 64-byte <c>seed || publicKey</c> key.</summary>
    public static byte[] Sign(byte[] message, byte[] secretKey)
    {
        ArgumentNullException.ThrowIfNull(message);
        ValidateLength(secretKey, ExtendedLength, nameof(secretKey), "secret key");

        var seed = secretKey.AsSpan(0, KeyLength).ToArray();
        var suppliedPublicKey = secretKey.AsSpan(KeyLength, KeyLength).ToArray();
        var (_, reconstructedSecretKey) = GenerateKeypair(seed);
        if (!secretKey.AsSpan().SequenceEqual(reconstructedSecretKey))
        {
            throw new ArgumentException(
                "Ed25519 secret key must be seed || publicKey.", nameof(secretKey));
        }

        var digest = Sha512Algorithm.Hash(seed);
        var scalar = ClampScalar(digest.AsSpan(0, KeyLength));
        var prefix = digest.AsSpan(KeyLength, KeyLength).ToArray();
        var nonce = ReduceScalar(HashParts(prefix, message));
        var encodedR = EncodePoint(ScalarMultiply(nonce, BasePoint));
        var challenge = ReduceScalar(HashParts(encodedR, suppliedPublicKey, message));
        var scalarS = ModGroup(nonce + challenge * scalar);

        var signature = new byte[ExtendedLength];
        encodedR.CopyTo(signature, 0);
        EncodeLittleEndian(scalarS).CopyTo(signature, KeyLength);
        return signature;
    }

    /// <summary>
    /// Verify a signature. Malformed signature and public-key encodings return
    /// <see langword="false"/> rather than throwing.
    /// </summary>
    public static bool Verify(byte[] message, byte[]? signature, byte[]? publicKey)
    {
        ArgumentNullException.ThrowIfNull(message);
        if (signature is null || signature.Length != ExtendedLength ||
            publicKey is null || publicKey.Length != KeyLength)
        {
            return false;
        }

        var encodedR = signature.AsSpan(0, KeyLength).ToArray();
        var scalarS = DecodeLittleEndian(signature.AsSpan(KeyLength, KeyLength));
        if (scalarS >= GroupOrder ||
            !TryDecodePoint(encodedR, out var pointR) ||
            !TryDecodePoint(publicKey, out var pointA))
        {
            return false;
        }

        var challenge = ReduceScalar(HashParts(encodedR, publicKey, message));
        var left = ScalarMultiply(scalarS, BasePoint);
        var right = Add(pointR, ScalarMultiply(challenge, pointA));
        return PointsEqual(left, right);
    }

    private static void ValidateLength(
        byte[]? value,
        int expectedLength,
        string parameterName,
        string description)
    {
        ArgumentNullException.ThrowIfNull(value, parameterName);
        if (value.Length != expectedLength)
        {
            throw new ArgumentException(
                $"Ed25519 {description} must be exactly {expectedLength} bytes.",
                parameterName);
        }
    }

    private static byte[] HashParts(params byte[][] parts)
    {
        var hasher = new Sha512Hasher();
        foreach (var part in parts)
        {
            hasher.Update(part);
        }

        return hasher.Digest();
    }

    private static BigInteger ClampScalar(ReadOnlySpan<byte> digestPrefix)
    {
        var clamped = digestPrefix.ToArray();
        clamped[0] &= 248;
        clamped[31] &= 127;
        clamped[31] |= 64;
        return DecodeLittleEndian(clamped);
    }

    private static BigInteger ReduceScalar(ReadOnlySpan<byte> bytes) =>
        DecodeLittleEndian(bytes) % GroupOrder;

    private static Point CreateBasePoint()
    {
        var y = Multiply(4, Invert(5));
        if (!TryRecoverX(y, 0, out var x))
        {
            throw new InvalidOperationException("Unable to construct the Ed25519 base point.");
        }

        return FromAffine(x, y);
    }

    private static Point FromAffine(BigInteger x, BigInteger y) =>
        new(Mod(x), Mod(y), BigInteger.One, Multiply(x, y));

    private static Point Add(Point left, Point right)
    {
        var a = Multiply(Subtract(left.Y, left.X), Subtract(right.Y, right.X));
        var b = Multiply(Add(left.Y, left.X), Add(right.Y, right.X));
        var c = Multiply(2 * D, Multiply(left.T, right.T));
        var d = Multiply(2, Multiply(left.Z, right.Z));
        var e = Subtract(b, a);
        var f = Subtract(d, c);
        var g = Add(d, c);
        var h = Add(b, a);
        return new Point(
            Multiply(e, f),
            Multiply(g, h),
            Multiply(f, g),
            Multiply(e, h));
    }

    private static Point Double(Point point)
    {
        var a = Square(point.X);
        var b = Square(point.Y);
        var c = Multiply(2, Square(point.Z));
        var d = Mod(-a);
        var e = Subtract(Subtract(Square(Add(point.X, point.Y)), a), b);
        var g = Add(d, b);
        var f = Subtract(g, c);
        var h = Subtract(d, b);
        return new Point(
            Multiply(e, f),
            Multiply(g, h),
            Multiply(f, g),
            Multiply(e, h));
    }

    private static Point ScalarMultiply(BigInteger scalar, Point point)
    {
        var result = Identity;
        var addend = point;
        var remaining = scalar;

        while (remaining > 0)
        {
            if (!remaining.IsEven)
            {
                result = Add(result, addend);
            }

            addend = Double(addend);
            remaining >>= 1;
        }

        return result;
    }

    private static byte[] EncodePoint(Point point)
    {
        var inverseZ = Invert(point.Z);
        var x = Multiply(point.X, inverseZ);
        var y = Multiply(point.Y, inverseZ);
        var encoded = EncodeLittleEndian(y);
        if (!x.IsEven)
        {
            encoded[31] |= 0x80;
        }

        return encoded;
    }

    private static bool TryDecodePoint(ReadOnlySpan<byte> encoded, out Point point)
    {
        point = Identity;
        if (encoded.Length != KeyLength)
        {
            return false;
        }

        var sign = (encoded[31] >> 7) & 1;
        var yBytes = encoded.ToArray();
        yBytes[31] &= 0x7f;
        var y = DecodeLittleEndian(yBytes);
        if (y >= Prime || !TryRecoverX(y, sign, out var x))
        {
            return false;
        }

        point = FromAffine(x, y);
        return true;
    }

    private static bool TryRecoverX(BigInteger y, int sign, out BigInteger x)
    {
        var ySquared = Square(y);
        var xSquared = Multiply(Subtract(ySquared, 1), Invert(Add(Multiply(D, ySquared), 1)));
        var candidate = BigInteger.ModPow(xSquared, (Prime + 3) / 8, Prime);
        if (Square(candidate) != xSquared)
        {
            candidate = Multiply(candidate, SqrtMinusOne);
        }

        if (Square(candidate) != xSquared || (candidate.IsZero && sign == 1))
        {
            x = BigInteger.Zero;
            return false;
        }

        x = ((int)(candidate & BigInteger.One)) == sign ? candidate : Mod(-candidate);
        return true;
    }

    private static bool PointsEqual(Point left, Point right) =>
        Multiply(left.X, right.Z) == Multiply(right.X, left.Z) &&
        Multiply(left.Y, right.Z) == Multiply(right.Y, left.Z);

    private static BigInteger DecodeLittleEndian(ReadOnlySpan<byte> bytes) =>
        new(bytes, isUnsigned: true, isBigEndian: false);

    private static byte[] EncodeLittleEndian(BigInteger value)
    {
        var encoded = value.ToByteArray(isUnsigned: true, isBigEndian: false);
        var result = new byte[KeyLength];
        encoded.CopyTo(result, 0);
        return result;
    }

    private static BigInteger Add(BigInteger left, BigInteger right) => Mod(left + right);

    private static BigInteger Subtract(BigInteger left, BigInteger right) => Mod(left - right);

    private static BigInteger Multiply(BigInteger left, BigInteger right) => Mod(left * right);

    private static BigInteger Square(BigInteger value) => Mod(value * value);

    private static BigInteger Invert(BigInteger value) =>
        BigInteger.ModPow(Mod(value), Prime - 2, Prime);

    private static BigInteger ModGroup(BigInteger value)
    {
        var reduced = value % GroupOrder;
        return reduced.Sign < 0 ? reduced + GroupOrder : reduced;
    }

    private static BigInteger Mod(BigInteger value)
    {
        var reduced = value % Prime;
        return reduced.Sign < 0 ? reduced + Prime : reduced;
    }

    private readonly record struct Point(
        BigInteger X,
        BigInteger Y,
        BigInteger Z,
        BigInteger T);
}
