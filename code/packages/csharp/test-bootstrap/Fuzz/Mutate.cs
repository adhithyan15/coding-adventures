using System.Numerics;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>The byte order of the multi-byte mutators.</summary>
public enum Endian
{
    /// <summary>Least significant byte first.</summary>
    Little,

    /// <summary>Most significant byte first.</summary>
    Big,
}

/// <summary>
/// The mutator algebra of the C# fuzz engine (VER06 section 3, VER06h section 3).
/// </summary>
/// <remarks>
/// Every mutator is a pure function from bytes to bytes that matches the shared
/// <c>fuzz-v1</c> corpus byte for byte. <b>No mutator modifies its argument</b>: each
/// returns a fresh array, even a no-op returns a copy.
/// <para>
/// Each mutator has two entry points. The <c>ulong</c> form is for engine code, where
/// offsets and counts are unsigned machine integers. The <see cref="BigInteger"/> form
/// (names ending in <c>Big</c>) is for the corpus runner, which may be handed integers of a
/// thousand digits and negative ones: it reduces its arguments <em>exactly</em> (offsets wrap
/// with FLOOR modulo, so -1 mod 8 is 7; counts and lengths saturate) and then calls the same
/// core, so the two forms cannot drift apart.
/// </para>
/// </remarks>
public static class Mutate
{
    /// <summary>Hard ceiling on a campaign's iterations, on <c>MutateSequence</c> steps and on the energy base.</summary>
    public const int MaxIterations = 1_000_000;

    /// <summary>Hard ceiling on an initial seed and on the intermediate size of a mutation sequence.</summary>
    public const int MaxInputBytes = 1 << 20;

    /// <summary>
    /// Hard ceiling on the cumulative bytes one <see cref="MutateSequence(byte[], ulong, int, int)"/>
    /// call may copy. Each step copies the whole buffer, so <c>steps</c> alone does not bound the
    /// work (CWE-400): 10^6 steps over a 1 MiB buffer would be 10^12 byte copies.
    /// </summary>
    public const long MaxSequenceWork = 1L << 28;

    // ---------------------------------------------------------------------------
    // Interest lists (VER06 section 3.4).
    // ---------------------------------------------------------------------------

    private static readonly ulong[] Interest8 = { 0, 1, 127, 128, 255 };
    private static readonly ulong[] Interest16 = { 0, 1, 255, 256, 32767, 32768, 65535 };
    private static readonly ulong[] Interest32 = { 0, 1, 65535, 65536, 0x7FFFFFFFUL, 0x80000000UL, 0xFFFFFFFFUL };

    private static readonly ulong[] Interest64 =
    {
        0, 1, 0xFFFFFFFFUL, 0x100000000UL, 0x7FFFFFFFFFFFFFFFUL, 0x8000000000000000UL, 0xFFFFFFFFFFFFFFFFUL,
    };

    /// <summary>The arithmetic deltas <see cref="MutateSequence(byte[], ulong, int, int)"/> draws from.</summary>
    private static readonly long[] Deltas8 = { -35, -8, -1, 1, 8, 35 };

    /// <summary>
    /// A copy of the boundary values for a width in bytes (1, 2, 4 or 8), or <c>null</c> for any
    /// other width.
    /// </summary>
    public static ulong[]? InterestValues(int width) => width switch
    {
        1 => (ulong[])Interest8.Clone(),
        2 => (ulong[])Interest16.Clone(),
        4 => (ulong[])Interest32.Clone(),
        8 => (ulong[])Interest64.Clone(),
        _ => null,
    };

    /// <summary>The 8-bit arithmetic deltas of <c>MutateSequence</c> (a copy).</summary>
    public static long[] SequenceDeltas() => (long[])Deltas8.Clone();

    private static bool ValidWidth(int w) => w == 1 || w == 2 || w == 4 || w == 8;

    // ---------------------------------------------------------------------------
    // Integer helpers.
    // ---------------------------------------------------------------------------

    /// <summary>Floor modulo of an arbitrary integer by a positive machine integer (result in [0, m)).</summary>
    private static int ModBig(BigInteger v, int m)
    {
        BigInteger r = BigInteger.Remainder(v, m);
        if (r.Sign < 0)
        {
            r += m;
        }

        return (int)r;
    }

    private static int ModU(ulong v, int m) => (int)(v % (ulong)m);

    /// <summary>Clamps a big integer into [0, 2^64): a negative value saturates to 0.</summary>
    internal static ulong SatU64(BigInteger v)
    {
        if (v.Sign < 0)
        {
            return 0;
        }

        return v > ulong.MaxValue ? ulong.MaxValue : (ulong)v;
    }

    /// <summary><paramref name="v"/> modulo 2^64 in two's complement (so -1 is all ones).</summary>
    internal static ulong Low64(BigInteger v) => (ulong)(v & ulong.MaxValue);

    private static byte[] Copy(byte[] b) => (byte[])b.Clone();

    // ---------------------------------------------------------------------------
    // Flips, by parity
    // ---------------------------------------------------------------------------

    /// <summary>
    /// Inverts <c>count</c> consecutive units (bits or bytes) starting at <c>baseOff</c>, wrapping
    /// around a buffer of n units.
    /// </summary>
    /// <remarks>
    /// The window of <c>count</c> covers every unit <c>count / n</c> times plus the first
    /// <c>count % n</c> units (counting from the wrapped start) once more. A unit flipped an even
    /// number of times is back where it began (XOR twice), so only parity matters:
    /// <code>
    ///   flips(step) = count / n + (1 if step &lt; count % n else 0)
    /// </code>
    /// That is O(len) work for ANY count, where a loop of <c>count</c> single flips would never
    /// finish for count = 10^18 (VER06 section 9.2).
    /// Worked example, 2 bytes = 16 bits, bit offset 0, count 17: 17 / 16 = 1 (every bit once,
    /// odd) and 17 % 16 = 1 (bit 0 once more, so twice, even): the result inverts bits 1..15 and
    /// leaves bit 0.
    /// </remarks>
    private static byte[] ParityFlip(byte[] data, bool bits, ulong baseOff, bool oddFull, ulong rem)
    {
        byte[] buf = Copy(data);
        if (oddFull)
        {
            for (int i = 0; i < buf.Length; i++)
            {
                buf[i] ^= 0xFF;
            }
        }

        ulong n = bits ? (ulong)buf.Length * 8UL : (ulong)buf.Length;
        for (ulong step = 0; step < rem; step++)
        {
            ulong pos = (baseOff + step) % n;
            if (bits)
            {
                buf[(int)(pos / 8)] ^= (byte)(1 << (int)(pos % 8));
            }
            else
            {
                buf[(int)pos] ^= 0xFF;
            }
        }

        return buf;
    }

    private static byte[] Flip(byte[] data, bool bits, ulong offset, ulong count)
    {
        if (data.Length == 0 || count == 0)
        {
            return Copy(data);
        }

        ulong n = bits ? (ulong)data.Length * 8UL : (ulong)data.Length;
        return ParityFlip(data, bits, offset % n, ((count / n) & 1) == 1, count % n);
    }

    private static byte[] FlipBig(byte[] data, bool bits, BigInteger offset, BigInteger count)
    {
        if (data.Length == 0 || count.Sign <= 0)
        {
            return Copy(data);
        }

        ulong n = bits ? (ulong)data.Length * 8UL : (ulong)data.Length;
        BigInteger bn = n;
        BigInteger q = BigInteger.DivRem(count, bn, out BigInteger r);
        BigInteger baseOff = BigInteger.Remainder(offset, bn);
        if (baseOff.Sign < 0)
        {
            baseOff += bn;
        }

        return ParityFlip(data, bits, (ulong)baseOff, !q.IsEven, (ulong)r);
    }

    /// <summary>
    /// Inverts <paramref name="count"/> consecutive bits from <c>bitOffset mod (8 * len)</c>. Bits are
    /// numbered LSB-first within each byte: bit 9 of <c>00 00</c> is bit 1 of byte 1, so
    /// <c>BitFlip(00 00, 9, 1)</c> is <c>00 02</c>. An empty input or a zero count is a no-op.
    /// </summary>
    public static byte[] BitFlip(byte[] data, ulong bitOffset, ulong count) => Flip(data, true, bitOffset, count);

    /// <summary><see cref="BitFlip"/> for arbitrary-size integers.</summary>
    public static byte[] BitFlipBig(byte[] data, BigInteger bitOffset, BigInteger count) => FlipBig(data, true, bitOffset, count);

    /// <summary>XORs <paramref name="count"/> consecutive bytes with 0xFF from <c>byteOffset mod len</c>.</summary>
    public static byte[] ByteFlip(byte[] data, ulong byteOffset, ulong count) => Flip(data, false, byteOffset, count);

    /// <summary><see cref="ByteFlip"/> for arbitrary-size integers.</summary>
    public static byte[] ByteFlipBig(byte[] data, BigInteger byteOffset, BigInteger count) => FlipBig(data, false, byteOffset, count);

    // ---------------------------------------------------------------------------
    // Arithmetic and boundary values
    // ---------------------------------------------------------------------------

    private static ulong ReadUint(byte[] b, int at, int w, Endian e)
    {
        ulong v = 0;
        for (int i = 0; i < w; i++)
        {
            ulong x = b[at + i];
            if (e == Endian.Big)
            {
                v = (v << 8) | x;
            }
            else
            {
                v |= x << (8 * i);
            }
        }

        return v;
    }

    /// <summary>Stores the low <c>w</c> bytes of <c>v</c>: that is "v mod 2^(8w)".</summary>
    private static void WriteUint(byte[] b, int at, int w, Endian e, ulong v)
    {
        for (int i = 0; i < w; i++)
        {
            int shift = e == Endian.Big ? 8 * (w - 1 - i) : 8 * i;
            b[at + i] = (byte)(v >> shift);
        }
    }

    private static byte[] ArithAt(byte[] data, int at, int w, ulong delta, Endian e)
    {
        byte[] buf = Copy(data);
        unchecked
        {
            WriteUint(buf, at, w, e, ReadUint(buf, at, w, e) + delta); // mod 2^64, then truncated
        }

        return buf;
    }

    private static byte[] BoundaryAt(byte[] data, int at, int w, ulong val, Endian e)
    {
        byte[] buf = Copy(data);
        WriteUint(buf, at, w, e, val);
        return buf;
    }

    /// <summary>
    /// Adds the signed <paramref name="delta"/> modulo 2^(8 * width) to the integer at the offset.
    /// <c>width</c> is in {1, 2, 4, 8}; any other width, or <c>len &lt; width</c>, returns the input
    /// unchanged. The offset wraps over the <c>len - width + 1</c> positions where a whole integer fits.
    /// </summary>
    /// <remarks>
    /// Worked example (u16, little-endian): <c>ff 00</c> is 0x00ff; adding 1 gives 0x0100, stored
    /// little-endian as <c>00 01</c>.
    /// </remarks>
    public static byte[] ByteArithmetic(byte[] data, ulong byteOffset, long delta, int width, Endian e)
    {
        if (!ValidWidth(width) || data.Length < width)
        {
            return Copy(data);
        }

        return ArithAt(data, ModU(byteOffset, data.Length - width + 1), width, unchecked((ulong)delta), e);
    }

    /// <summary><see cref="ByteArithmetic"/> for arbitrary-size integers (the delta is taken modulo 2^64).</summary>
    public static byte[] ByteArithmeticBig(byte[] data, BigInteger byteOffset, BigInteger delta, int width, Endian e)
    {
        if (!ValidWidth(width) || data.Length < width)
        {
            return Copy(data);
        }

        return ArithAt(data, ModBig(byteOffset, data.Length - width + 1), width, Low64(delta), e);
    }

    /// <summary>
    /// Overwrites <paramref name="width"/> bytes with <c>val mod 2^(8 * width)</c> at the offset. Same
    /// width and short-buffer rules as <see cref="ByteArithmetic"/>. Worked example:
    /// <c>BoundaryInsert(00 00 00 00, 0, 4, 0x80000000, little)</c> is <c>00 00 00 80</c>.
    /// </summary>
    public static byte[] BoundaryInsert(byte[] data, ulong byteOffset, int width, ulong val, Endian e)
    {
        if (!ValidWidth(width) || data.Length < width)
        {
            return Copy(data);
        }

        return BoundaryAt(data, ModU(byteOffset, data.Length - width + 1), width, val, e);
    }

    /// <summary><see cref="BoundaryInsert"/> for arbitrary-size integers (a negative value is its two's complement).</summary>
    public static byte[] BoundaryInsertBig(byte[] data, BigInteger byteOffset, int width, BigInteger val, Endian e)
    {
        if (!ValidWidth(width) || data.Length < width)
        {
            return Copy(data);
        }

        return BoundaryAt(data, ModBig(byteOffset, data.Length - width + 1), width, Low64(val), e);
    }

    // ---------------------------------------------------------------------------
    // Blocks and splice
    // ---------------------------------------------------------------------------

    private static byte[] DeleteAt(byte[] data, int at, ulong length)
    {
        int cut = (int)Math.Min(length, (ulong)(data.Length - at));
        byte[] output = new byte[data.Length - cut];
        Array.Copy(data, 0, output, 0, at);
        Array.Copy(data, at + cut, output, at, data.Length - at - cut);
        return output;
    }

    /// <summary>Deletes up to <paramref name="length"/> bytes at <c>offset mod len</c>, clamped at the end.</summary>
    public static byte[] BlockDelete(byte[] data, ulong offset, ulong length)
    {
        if (data.Length == 0 || length == 0)
        {
            return Copy(data);
        }

        return DeleteAt(data, ModU(offset, data.Length), length);
    }

    /// <summary><see cref="BlockDelete"/> for arbitrary-size integers.</summary>
    public static byte[] BlockDeleteBig(byte[] data, BigInteger offset, BigInteger length)
    {
        if (data.Length == 0 || length.Sign <= 0)
        {
            return Copy(data);
        }

        return DeleteAt(data, ModBig(offset, data.Length), SatU64(length));
    }

    private static byte[] InsertAt(byte[] data, int at, byte[] ins, int insFrom, int insLen)
    {
        byte[] output = new byte[data.Length + insLen];
        Array.Copy(data, 0, output, 0, at);
        Array.Copy(ins, insFrom, output, at, insLen);
        Array.Copy(data, at, output, at + insLen, data.Length - at);
        return output;
    }

    /// <summary>Inserts at <c>offset mod (len + 1)</c>; an empty input becomes the insert.</summary>
    public static byte[] BlockInsert(byte[] data, ulong offset, byte[] insert)
    {
        if (data.Length == 0)
        {
            return Copy(insert);
        }

        return InsertAt(data, ModU(offset, data.Length + 1), insert, 0, insert.Length);
    }

    /// <summary><see cref="BlockInsert"/> for an arbitrary-size offset.</summary>
    public static byte[] BlockInsertBig(byte[] data, BigInteger offset, byte[] insert)
    {
        if (data.Length == 0)
        {
            return Copy(insert);
        }

        return InsertAt(data, ModBig(offset, data.Length + 1), insert, 0, insert.Length);
    }

    private static byte[] DuplicateAt(byte[] data, int src, ulong length, int dst)
    {
        int n = (int)Math.Min(length, (ulong)(data.Length - src));
        return InsertAt(data, dst, data, src, n);
    }

    /// <summary>
    /// Copies up to <paramref name="length"/> bytes from <c>srcOffset mod len</c> and inserts them at
    /// <c>dstOffset mod (len + 1)</c> (the end of the buffer is a valid insertion point); the copy is
    /// clamped at the end of the source.
    /// </summary>
    public static byte[] BlockDuplicate(byte[] data, ulong srcOffset, ulong length, ulong dstOffset)
    {
        if (data.Length == 0 || length == 0)
        {
            return Copy(data);
        }

        return DuplicateAt(data, ModU(srcOffset, data.Length), length, ModU(dstOffset, data.Length + 1));
    }

    /// <summary><see cref="BlockDuplicate"/> for arbitrary-size integers.</summary>
    public static byte[] BlockDuplicateBig(byte[] data, BigInteger srcOffset, BigInteger length, BigInteger dstOffset)
    {
        if (data.Length == 0 || length.Sign <= 0)
        {
            return Copy(data);
        }

        return DuplicateAt(data, ModBig(srcOffset, data.Length), SatU64(length), ModBig(dstOffset, data.Length + 1));
    }

    private static int ClampSplit(ulong split, int n) => (int)Math.Min(split, (ulong)n);

    /// <summary><c>target[:ts] + donor[ds:]</c> with both splits clamped into their buffers.</summary>
    public static byte[] Splice(byte[] target, byte[] donor, ulong targetSplit, ulong donorSplit)
    {
        int t = ClampSplit(targetSplit, target.Length);
        int d = ClampSplit(donorSplit, donor.Length);
        byte[] output = new byte[t + donor.Length - d];
        Array.Copy(target, 0, output, 0, t);
        Array.Copy(donor, d, output, t, donor.Length - d);
        return output;
    }

    /// <summary><see cref="Splice"/> for arbitrary-size splits (a negative split clamps to 0).</summary>
    public static byte[] SpliceBig(byte[] target, byte[] donor, BigInteger targetSplit, BigInteger donorSplit)
    {
        return Splice(target, donor, SatU64(targetSplit), SatU64(donorSplit));
    }

    /// <summary>The length <see cref="Splice"/> must produce, computed independently for the runner's invariant.</summary>
    internal static long SpliceLength(int targetLen, int donorLen, ulong targetSplit, ulong donorSplit)
    {
        return (long)ClampSplit(targetSplit, targetLen) + donorLen - ClampSplit(donorSplit, donorLen);
    }

    // ---------------------------------------------------------------------------
    // MutateSequence
    // ---------------------------------------------------------------------------

    /// <summary>
    /// Applies <paramref name="steps"/> seeded mutations, each chosen by one <c>Bounded(7)</c> draw:
    /// <code>
    ///   draw  step
    ///   0     bit flip, count 1
    ///   1     byte flip, count 1
    ///   2     byte arithmetic, width 1, delta from (-35 -8 -1 1 8 35)
    ///   3     boundary insert, width 1, value from (0 1 127 128 255)
    ///   4     block delete (length 1..len/2); skipped if len &lt;= 1
    ///   5     block insert of one random byte
    ///   6     block duplicate (length 1..len/2); skipped if len &lt;= 1
    /// </code>
    /// An empty buffer is refilled to <c>00</c> before every step. The draw order is the cross-lane
    /// contract: the same (data, seed, steps) gives the same bytes in every language. Draws for a
    /// skipped step are still consumed.
    /// <para>
    /// <paramref name="maxLen"/> is VER06 section 3.5's "maximum length budget": a block insert or
    /// duplicate that would grow the buffer past it is skipped AFTER its draws are consumed, so the
    /// stream stays aligned and memory stays bounded.
    /// </para>
    /// A negative <paramref name="steps"/> runs none. <paramref name="steps"/> above
    /// <see cref="MaxIterations"/>, an input above <see cref="MaxInputBytes"/>, or a call whose
    /// cumulative copying would exceed <see cref="MaxSequenceWork"/> throws <see cref="ArgumentException"/>.
    /// </summary>
    public static byte[] MutateSequence(byte[] data, ulong seed, int steps, int maxLen)
    {
        return MutateSequence(data, new SplitMix64(seed), steps, maxLen, MaxSequenceWork);
    }

    /// <summary>The same over a caller-built stream and an explicit work cap (the sabotage and cap-test seam).</summary>
    internal static byte[] MutateSequence(byte[] data, SplitMix64 rng, int steps, int maxLen, long workCap)
    {
        return Counted(data, rng, steps, maxLen, workCap).Data;
    }

    /// <summary>
    /// Thrown when a sequence exhausts its work cap. It carries the bytes of copying the call really
    /// did before refusing, so a caller keeping an aggregate budget can charge a FAILED call too.
    /// </summary>
    internal sealed class WorkExhausted : ArgumentException
    {
        internal WorkExhausted(long cap, long work)
            : base("mutateSequence work budget of " + cap + " byte copies exhausted")
        {
            Work = work;
        }

        /// <summary>Bytes copied before the refusal.</summary>
        internal long Work { get; }
    }

    /// <summary>A mutated buffer and the bytes of buffer copying the sequence actually did.</summary>
    internal readonly record struct CountedResult(byte[] Data, long Work);

    /// <summary>
    /// <see cref="MutateSequence(byte[], ulong, int, int)"/> that also reports REAL work: the corpus
    /// runner charges it against its aggregate budget (block insert and duplicate grow the buffer,
    /// so steps x initial length underestimates).
    /// </summary>
    internal static CountedResult MutateSequenceCounted(byte[] data, ulong seed, int steps, int maxLen, long workCap)
    {
        return Counted(data, new SplitMix64(seed), steps, maxLen, workCap);
    }

    private static CountedResult Counted(byte[] data, SplitMix64 rng, int steps, int maxLen, long workCap)
    {
        if (steps > MaxIterations)
        {
            throw new ArgumentException("steps exceeds MaxIterations", nameof(steps));
        }

        if (data.Length > MaxInputBytes)
        {
            throw new ArgumentException("input exceeds MaxInputBytes", nameof(data));
        }

        byte[] cur = Copy(data);
        long work = cur.Length; // the initial copy is work too
        if (work > workCap)
        {
            throw new WorkExhausted(workCap, 0);
        }

        for (int i = 0; i < steps; i++)
        {
            if (cur.Length == 0)
            {
                cur = new byte[] { 0 };
            }

            if (work + cur.Length > workCap)
            {
                throw new WorkExhausted(workCap, work);
            }

            work += cur.Length;
            int n = cur.Length;
            switch ((int)rng.Bounded(7))
            {
                case 0:
                    cur = Flip(cur, true, rng.Bounded((ulong)n * 8UL), 1);
                    break;
                case 1:
                    cur = Flip(cur, false, rng.Bounded((ulong)n), 1);
                    break;
                case 2:
                    {
                        long delta = Deltas8[(int)rng.Bounded((ulong)Deltas8.Length)];
                        cur = ArithAt(cur, (int)rng.Bounded((ulong)n), 1, unchecked((ulong)delta), Endian.Little);
                        break;
                    }

                case 3:
                    {
                        ulong val = Interest8[(int)rng.Bounded((ulong)Interest8.Length)];
                        cur = BoundaryAt(cur, (int)rng.Bounded((ulong)n), 1, val, Endian.Little);
                        break;
                    }

                case 4:
                    if (n > 1)
                    {
                        ulong length = 1 + rng.Bounded((ulong)Math.Max(1, n / 2));
                        cur = DeleteAt(cur, (int)rng.Bounded((ulong)n), length);
                    }

                    break;
                case 5:
                    {
                        byte ins = (byte)rng.Bounded(256);
                        int at = (int)rng.Bounded((ulong)n + 1UL);
                        if (n + 1 <= maxLen)
                        {
                            cur = InsertAt(cur, at, new[] { ins }, 0, 1);
                        }

                        break;
                    }

                default: // 6
                    if (n > 1)
                    {
                        ulong length = 1 + rng.Bounded((ulong)Math.Max(1, n / 2));
                        int src = (int)rng.Bounded((ulong)n);
                        int dst = (int)rng.Bounded((ulong)n + 1UL);
                        if (n + (int)Math.Min(length, (ulong)(n - src)) <= maxLen)
                        {
                            cur = DuplicateAt(cur, src, length, dst);
                        }
                    }

                    break;
            }
        }

        return new CountedResult(cur, work);
    }
}
