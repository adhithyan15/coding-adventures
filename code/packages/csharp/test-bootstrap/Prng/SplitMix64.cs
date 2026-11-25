using System.Numerics;
using System.Text;

namespace CodingAdventures.TestBootstrap.Prng;

/// <summary>
/// Deterministic randomness shared by every test engine in the C# lane
/// (specification: VER05g section 3, VER05e section 2).
/// </summary>
/// <remarks>
/// <para>
/// Why a hand-written PRNG instead of <see cref="Random"/>? A property test is only
/// useful if a failure can be replayed. "Seed 42 failed" must mean the same sequence
/// of candidates on every machine, every runtime release and every other language
/// lane. SplitMix64 is five lines of 64-bit arithmetic that any language can
/// reproduce bit for bit, and the shared <c>pbt-v1</c> corpus pins its outputs.
/// </para>
/// <h2>SplitMix64</h2>
/// <para>
/// The state is one 64-bit word. Each step adds the golden-ratio constant and
/// scrambles the sum with two xor-shift-multiply rounds:
/// </para>
/// <code>
///   state  += 0x9e3779b97f4a7c15
///   z       = state
///   z       = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
///   z       = (z ^ (z >> 27)) * 0x94d049bb133111eb
///   output  = z ^ (z >> 31)
/// </code>
/// <para>
/// C# <c>ulong</c> arithmetic wraps modulo 2^64 in an unchecked context, which is
/// exactly what the algorithm assumes, and <c>&gt;&gt;</c> on an unsigned type is
/// a logical shift, so no masking and no sign trap exists. Pinned vector: seed 0's
/// first output is <c>0xe220a8397b1dcdaf</c>.
/// </para>
/// <h2>Bounded draws without bias</h2>
/// <para>
/// <c>x % n</c> is biased unless n divides 2^64: residues below 2^64 mod n occur
/// once more often than the rest. Rejection sampling removes the bias. With the
/// threshold <c>t = (2^64 - n) mod n</c>, every raw value below t is thrown away;
/// the 2^64 - t values that remain split into exactly equal residue classes.
/// </para>
/// <para>
/// Worked example, n = 3: 2^64 = 3 * 6148914691236517205 + 1, so t = 1. Only the
/// raw value 0 is rejected; the other 2^64 - 1 values fall evenly into 0, 1, 2.
/// </para>
/// <para>Not safe for concurrent use.</para>
/// </remarks>
public sealed class SplitMix64
{
    private const ulong GoldenGamma = 0x9e3779b97f4a7c15UL;
    private const ulong MixMul1 = 0xbf58476d1ce4e5b9UL;
    private const ulong MixMul2 = 0x94d049bb133111ebUL;
    private const ulong FnvOffsetBasis = 0xcbf29ce484222325UL;
    private const ulong FnvPrime = 0x00000100000001b3UL;

    private static readonly BigInteger TwoTo64 = BigInteger.One << 64;

    /// <summary>A generator whose stream is fully determined by <paramref name="seed"/>.</summary>
    public SplitMix64(ulong seed)
    {
        State = seed;
    }

    /// <summary>The current internal state (the next <see cref="NextU64"/> starts from it).</summary>
    public ulong State { get; set; }

    /// <summary>Advances the stream and returns the next 64 random bits.</summary>
    public ulong NextU64()
    {
        unchecked
        {
            State += GoldenGamma;
            ulong z = State;
            z = (z ^ (z >> 30)) * MixMul1;
            z = (z ^ (z >> 27)) * MixMul2;
            return z ^ (z >> 31);
        }
    }

    /// <summary>
    /// A uniform value in [0, n) for 1 &lt;= n &lt;= 2^64-1.
    /// </summary>
    /// <remarks>
    /// It always draws at least one word, even for n == 1 (where the answer is 0):
    /// that is the VER05 section 2.2 loop run once, and it keeps the stream position
    /// of every generator identical across language lanes. n == 0 is a caller bug.
    /// A span of exactly 2^64 cannot be written as a <c>ulong</c>: use <see cref="NextU64"/>.
    /// </remarks>
    public ulong Bounded(ulong n)
    {
        if (n == 0)
        {
            throw new ArgumentOutOfRangeException(nameof(n), "Bounded(0) has no valid result");
        }

        // 0UL - n is 2^64 - n modulo 2^64, so this is (2^64 - n) mod n, unsigned.
        // (Python's "(-n) % n" is 0 for every n: a different, wrong number.)
        ulong threshold = Threshold(n);
        while (true)
        {
            ulong x = NextU64();
            if (x >= threshold)
            {
                return x % n;
            }
        }
    }

    /// <summary>The honest rejection threshold <c>(2^64 - n) mod n</c>.</summary>
    internal static ulong Threshold(ulong n)
    {
        return unchecked(0UL - n) % n;
    }

    /// <summary>
    /// A uniform value in [0, span) for any span &gt;= 1.
    /// </summary>
    /// <remarks>
    /// <code>
    ///   span &lt;= 2^64-1   Bounded(span)
    ///   span == 2^64     one raw word
    ///   span  &gt; 2^64     k-word rejection sampling
    /// </code>
    /// For a wide span take k = max(2, ceil(bitlen(span-1)/64)) words, first draw most
    /// significant, giving raw uniform on [0, 2^(64k)). A plain raw % span would
    /// over-weight the low residues (for span = 3*2^126 the low third would be drawn
    /// half the time instead of a third), so raw values below
    /// t = (2^(64k) - span) mod span are redrawn. Acceptance probability is at least one half.
    /// </remarks>
    public BigInteger BoundedBig(BigInteger span)
    {
        if (span.Sign <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(span), "BoundedBig needs span >= 1");
        }

        int cmp = span.CompareTo(TwoTo64);
        if (cmp < 0)
        {
            return Bounded((ulong)span);
        }

        if (cmp == 0)
        {
            return NextU64();
        }

        long bits = (long)(span - BigInteger.One).GetBitLength();

        // span > 2^64 here, so span-1 has at least 65 bits and k >= 2 always holds.
        int words = (int)((bits + 63) / 64);

        BigInteger modulus = BigInteger.One << (64 * words);
        BigInteger threshold = (modulus - span) % span;
        while (true)
        {
            BigInteger raw = BigInteger.Zero;
            for (int i = 0; i < words; i++)
            {
                raw = (raw << 64) | NextU64();
            }

            if (raw >= threshold)
            {
                return raw % span;
            }
        }
    }

    /// <summary>
    /// 64-bit FNV-1a: start from the offset basis; for each byte xor it in, then
    /// multiply by the FNV prime (mod 2^64).
    /// </summary>
    /// <remarks>
    /// <code>
    ///   Fnv1a64("")  = 0xcbf29ce484222325
    ///   Fnv1a64("a") = 0xaf63dc4c8601ec8c
    /// </code>
    /// </remarks>
    public static ulong Fnv1a64(ReadOnlySpan<byte> data)
    {
        ulong h = FnvOffsetBasis;
        unchecked
        {
            foreach (byte b in data)
            {
                h = (h ^ b) * FnvPrime;
            }
        }

        return h;
    }

    /// <summary>FNV-1a over the UTF-8 bytes of <paramref name="s"/> (lone surrogates become U+FFFD).</summary>
    public static ulong Fnv1a64(string s)
    {
        return Fnv1a64(Encoding.UTF8.GetBytes(s));
    }

    /// <summary>
    /// The isolated seed of one property inside a run (VER05 section 2.3):
    /// <c>SubSeed(run, id) = SplitMix64(run XOR Fnv1a64(utf8(id))).NextU64()</c>.
    /// </summary>
    /// <remarks>
    /// Two properties in one run never share a stream, and adding or reordering
    /// properties never changes another property's draws. The final NextU64 is a
    /// scramble: without it two ids whose hashes differ in one bit would produce
    /// seeds differing in one bit, so their streams would be strongly related.
    /// </remarks>
    public static ulong SubSeed(ulong runSeed, string id)
    {
        return new SplitMix64(runSeed ^ Fnv1a64(id)).NextU64();
    }
}
