using System;
using System.Buffers.Binary;
using System.Numerics;
using System.Text;

namespace CodingAdventures.Md5;

// Md5.cs -- RFC 1321 message digests with explicit little-endian byte handling
// ============================================================================
//
// MD5 consumes bytes in 64-byte blocks and maintains four 32-bit state words.
// Two details matter most when implementing it correctly:
//
//   1. arithmetic is modulo 2^32
//   2. words are read and written in little-endian order
//
// That little-endian rule is the main difference from the SHA-family hashes.

/// <summary>
/// MD5 message digest algorithm (RFC 1321) implemented from scratch.
/// </summary>
public static class Md5
{
    /// <summary>
    /// Package version.
    /// </summary>
    public const string VERSION = "0.1.0";

    private static readonly uint[] T = BuildTTable();

    private static readonly byte[] S =
    [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
        5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    private const uint InitA = 0x67452301;
    private const uint InitB = 0xefcdab89;
    private const uint InitC = 0x98badcfe;
    private const uint InitD = 0x10325476;

    /// <summary>
    /// Convert bytes to lowercase hexadecimal.
    /// </summary>
    public static string ToHex(ReadOnlySpan<byte> bytes)
    {
        var builder = new StringBuilder(bytes.Length * 2);
        foreach (var value in bytes)
        {
            builder.Append(value.ToString("x2"));
        }

        return builder.ToString();
    }

    /// <summary>
    /// Compute the 16-byte MD5 digest of the provided data.
    /// </summary>
    public static byte[] SumMd5(ReadOnlySpan<byte> data)
    {
        var padded = Pad(data);
        var a = InitA;
        var b = InitB;
        var c = InitC;
        var d = InitD;

        for (var offset = 0; offset < padded.Length; offset += 64)
        {
            (a, b, c, d) = CompressState(a, b, c, d, padded.AsSpan(offset, 64));
        }

        return StateToBytes(a, b, c, d);
    }

    /// <summary>
    /// Compute MD5 and render it as a 32-character lowercase hexadecimal string.
    /// </summary>
    public static string HexString(ReadOnlySpan<byte> data) => ToHex(SumMd5(data));

    internal static byte[] FinalizeDigest(
        uint a,
        uint b,
        uint c,
        uint d,
        byte[] buffer,
        int bufferLength,
        ulong byteCount)
    {
        var afterBit = (bufferLength + 1) % 64;
        var zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit;
        var finalBlock = new byte[bufferLength + 1 + zeroCount + 8];

        Array.Copy(buffer, 0, finalBlock, 0, bufferLength);
        finalBlock[bufferLength] = 0x80;
        BinaryPrimitives.WriteUInt64LittleEndian(
            finalBlock.AsSpan(finalBlock.Length - 8),
            byteCount * 8);

        for (var offset = 0; offset < finalBlock.Length; offset += 64)
        {
            (a, b, c, d) = CompressState(a, b, c, d, finalBlock.AsSpan(offset, 64));
        }

        return StateToBytes(a, b, c, d);
    }

    private static uint[] BuildTTable()
    {
        var table = new uint[64];
        for (var index = 0; index < table.Length; index++)
        {
            table[index] = (uint)Math.Floor(Math.Abs(Math.Sin(index + 1.0)) * 4294967296.0);
        }

        return table;
    }

    private static byte[] Pad(ReadOnlySpan<byte> data)
    {
        var bitLength = (ulong)data.Length * 8;
        var afterBit = (data.Length + 1) % 64;
        var zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit;
        var result = new byte[data.Length + 1 + zeroCount + 8];

        data.CopyTo(result);
        result[data.Length] = 0x80;
        BinaryPrimitives.WriteUInt64LittleEndian(result.AsSpan(result.Length - 8), bitLength);
        return result;
    }

    internal static (uint A, uint B, uint C, uint D) CompressState(
        uint stateA,
        uint stateB,
        uint stateC,
        uint stateD,
        ReadOnlySpan<byte> block)
    {
        Span<uint> words = stackalloc uint[16];
        for (var index = 0; index < 16; index++)
        {
            words[index] = BinaryPrimitives.ReadUInt32LittleEndian(block.Slice(index * 4, 4));
        }

        var a = stateA;
        var b = stateB;
        var c = stateC;
        var d = stateD;

        for (var index = 0; index < 64; index++)
        {
            uint f;
            int g;

            if (index < 16)
            {
                f = (b & c) | (~b & d);
                g = index;
            }
            else if (index < 32)
            {
                f = (d & b) | (~d & c);
                g = (5 * index + 1) % 16;
            }
            else if (index < 48)
            {
                f = b ^ c ^ d;
                g = (3 * index + 5) % 16;
            }
            else
            {
                f = c ^ (b | ~d);
                g = (7 * index) % 16;
            }

            var inner = unchecked(a + f + words[g] + T[index]);
            var temp = unchecked(b + BitOperations.RotateLeft(inner, S[index]));
            a = d;
            d = c;
            c = b;
            b = temp;
        }

        return (
            unchecked(stateA + a),
            unchecked(stateB + b),
            unchecked(stateC + c),
            unchecked(stateD + d));
    }

    private static byte[] StateToBytes(uint a, uint b, uint c, uint d)
    {
        var digest = new byte[16];
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(0, 4), a);
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(4, 4), b);
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(8, 4), c);
        BinaryPrimitives.WriteUInt32LittleEndian(digest.AsSpan(12, 4), d);
        return digest;
    }
}

/// <summary>
/// Streaming MD5 hasher that accepts data in multiple chunks.
/// </summary>
public sealed class Md5Hasher
{
    private uint _a = 0x67452301;
    private uint _b = 0xefcdab89;
    private uint _c = 0x98badcfe;
    private uint _d = 0x10325476;
    private readonly byte[] _buffer = new byte[64];
    private int _bufferLength;
    private ulong _byteCount;

    /// <summary>
    /// Create a new streaming hasher with MD5's initial state.
    /// </summary>
    public Md5Hasher()
    {
    }

    private Md5Hasher(
        uint a,
        uint b,
        uint c,
        uint d,
        byte[] buffer,
        int bufferLength,
        ulong byteCount)
    {
        _a = a;
        _b = b;
        _c = c;
        _d = d;
        Array.Copy(buffer, _buffer, buffer.Length);
        _bufferLength = bufferLength;
        _byteCount = byteCount;
    }

    /// <summary>
    /// Feed more bytes into the hash state.
    /// </summary>
    public Md5Hasher Update(ReadOnlySpan<byte> data)
    {
        _byteCount += (ulong)data.Length;
        var offset = 0;

        if (_bufferLength > 0)
        {
            var needed = 64 - _bufferLength;
            var take = Math.Min(needed, data.Length);
            data[..take].CopyTo(_buffer.AsSpan(_bufferLength));
            _bufferLength += take;
            offset += take;

            if (_bufferLength == 64)
            {
                (_a, _b, _c, _d) = CompressBufferedBlock();
                _bufferLength = 0;
            }
        }

        while (offset + 64 <= data.Length)
        {
            (_a, _b, _c, _d) = CompressSpan(data.Slice(offset, 64));
            offset += 64;
        }

        if (offset < data.Length)
        {
            data[offset..].CopyTo(_buffer.AsSpan(_bufferLength));
            _bufferLength += data.Length - offset;
        }

        return this;
    }

    /// <summary>
    /// Return the current digest without mutating the hasher state.
    /// </summary>
    public byte[] Digest()
    {
        var bufferCopy = new byte[_bufferLength];
        Array.Copy(_buffer, bufferCopy, _bufferLength);
        return Md5.FinalizeDigest(_a, _b, _c, _d, bufferCopy, _bufferLength, _byteCount);
    }

    /// <summary>
    /// Alias for <see cref="Digest"/>.
    /// </summary>
    public byte[] SumMd5() => Digest();

    /// <summary>
    /// Return the current digest as lowercase hexadecimal.
    /// </summary>
    public string HexDigest() => Md5.ToHex(Digest());

    /// <summary>
    /// Clone the current streaming state.
    /// </summary>
    public Md5Hasher Copy() => new(_a, _b, _c, _d, _buffer, _bufferLength, _byteCount);

    private (uint A, uint B, uint C, uint D) CompressBufferedBlock() =>
        CompressSpan(_buffer);

    private (uint A, uint B, uint C, uint D) CompressSpan(ReadOnlySpan<byte> block) =>
        Md5.CompressState(_a, _b, _c, _d, block);
}
