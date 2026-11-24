package com.codingadventures.testbootstrap.fuzz;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.math.BigInteger;
import java.util.Arrays;

/**
 * The mutator algebra of the Java fuzz engine (VER06 section 3, VER06f section 3).
 *
 * <p>Every mutator is a pure function from bytes to bytes that matches the shared
 * {@code fuzz-v1} corpus byte for byte. <strong>No mutator modifies its
 * argument</strong>: each returns a fresh array, even a no-op returns a copy.
 *
 * <p>Each mutator has two entry points. The {@code long} form is for engine code,
 * where offsets and counts are machine integers. The {@code BigInteger} form
 * (names ending in {@code Big}) is for the corpus runner, which may be handed
 * integers of a thousand digits: it reduces its arguments <em>exactly</em> and
 * then calls the same core, so the two forms cannot drift apart.
 */
public final class Mutate {
  private Mutate() {}

  /** Hard ceiling on a campaign's iterations, on {@code mutateSequence} steps and on the energy base. */
  public static final int MAX_ITERATIONS = 1_000_000;

  /** Hard ceiling on an initial seed and on the intermediate size of a mutation sequence. */
  public static final int MAX_INPUT_BYTES = 1 << 20;

  /**
   * Hard ceiling on the cumulative bytes one {@link #mutateSequence} call may
   * copy. Each step copies the whole buffer, so {@code steps} alone does not bound
   * the work (CWE-400): 10^6 steps over a 1 MiB buffer would be 10^12 byte copies.
   */
  public static final long MAX_SEQUENCE_WORK = 1L << 28;

  /** The byte order of the multi-byte mutators. */
  public enum Endian {
    /** Least significant byte first. */
    LITTLE,
    /** Most significant byte first. */
    BIG
  }

  // ---------------------------------------------------------------------------
  // Interest lists (VER06 section 3.4). Values are unsigned words carried in
  // signed longs: 0xFFFFFFFFFFFFFFFFL is -1L, the same 64 bits.
  // ---------------------------------------------------------------------------

  private static final long[] INTEREST8 = {0, 1, 127, 128, 255};
  private static final long[] INTEREST16 = {0, 1, 255, 256, 32767, 32768, 65535};
  private static final long[] INTEREST32 = {0, 1, 65535, 65536, 0x7FFFFFFFL, 0x80000000L, 0xFFFFFFFFL};
  private static final long[] INTEREST64 = {
    0, 1, 0xFFFFFFFFL, 0x100000000L, 0x7FFFFFFFFFFFFFFFL, 0x8000000000000000L, 0xFFFFFFFFFFFFFFFFL
  };

  /** The arithmetic deltas {@link #mutateSequence} draws from. */
  private static final long[] DELTAS8 = {-35, -8, -1, 1, 8, 35};

  /**
   * A copy of the boundary values for a width in bytes (1, 2, 4 or 8), or
   * {@code null} for any other width. Values are unsigned 64-bit words.
   */
  public static long[] interestValues(int width) {
    return switch (width) {
      case 1 -> INTEREST8.clone();
      case 2 -> INTEREST16.clone();
      case 4 -> INTEREST32.clone();
      case 8 -> INTEREST64.clone();
      default -> null;
    };
  }

  /** The 8-bit arithmetic deltas of {@link #mutateSequence} (a copy). */
  public static long[] sequenceDeltas() {
    return DELTAS8.clone();
  }

  private static boolean validWidth(int w) {
    return w == 1 || w == 2 || w == 4 || w == 8;
  }

  // ---------------------------------------------------------------------------
  // Integer helpers. Offsets wrap with FLOOR modulo (-1 mod 8 == 7); counts and
  // lengths that exceed the machine range saturate.
  // ---------------------------------------------------------------------------

  private static int floorMod(long a, int m) {
    return (int) Math.floorMod(a, (long) m);
  }

  private static int modBig(BigInteger v, int m) {
    return v.mod(BigInteger.valueOf(m)).intValue(); // BigInteger.mod is non-negative
  }

  /** Clamps a big integer into the long range. */
  static long satLong(BigInteger v) {
    if (v.bitLength() < 64) {
      return v.longValue();
    }
    return v.signum() < 0 ? Long.MIN_VALUE : Long.MAX_VALUE;
  }

  /** {@code v} modulo 2^64 in two's complement (so -1 is all ones). */
  static long low64(BigInteger v) {
    return v.longValue();
  }

  private static byte[] copy(byte[] b) {
    return b.clone();
  }

  // ---------------------------------------------------------------------------
  // Flips, by parity
  // ---------------------------------------------------------------------------

  /**
   * Inverts {@code count} consecutive units (bits when {@code unitBits} is 1, bytes
   * when 8) starting at {@code base}, wrapping around a buffer of n units.
   *
   * <p>The window of {@code count} covers every unit {@code count / n} times plus
   * the first {@code count % n} units (counting from the wrapped start) once more.
   * A unit flipped an even number of times is back where it began (XOR twice), so
   * only parity matters:
   *
   * <pre>
   *   flips(step) = count / n + (1 if step &lt; count % n else 0)
   * </pre>
   *
   * That is O(len) work for ANY count, where a loop of {@code count} single flips
   * would never finish for count = 10^18 (VER06 section 9.2).
   *
   * <p>Worked example, 2 bytes = 16 bits, bit offset 0, count 17: 17 / 16 = 1
   * (every bit once, odd) and 17 % 16 = 1 (bit 0 once more, so twice, even): the
   * result inverts bits 1..15 and leaves bit 0.
   */
  private static byte[] parityFlip(byte[] data, int unitBits, long base, boolean oddFull, long rem) {
    byte[] buf = copy(data);
    if (oddFull) {
      for (int i = 0; i < buf.length; i++) {
        buf[i] ^= (byte) 0xFF;
      }
    }
    long n = unitBits == 1 ? buf.length * 8L : buf.length;
    for (long step = 0; step < rem; step++) {
      long pos = (base + step) % n;
      if (unitBits == 1) {
        buf[(int) (pos / 8)] ^= (byte) (1 << (pos % 8));
      } else {
        buf[(int) pos] ^= (byte) 0xFF;
      }
    }
    return buf;
  }

  private static byte[] flip(byte[] data, int unitBits, long offset, long count) {
    if (data.length == 0 || count <= 0) {
      return copy(data);
    }
    long n = unitBits == 1 ? data.length * 8L : data.length;
    return parityFlip(data, unitBits, Math.floorMod(offset, n), ((count / n) & 1) == 1, count % n);
  }

  private static byte[] flipBig(byte[] data, int unitBits, BigInteger offset, BigInteger count) {
    if (data.length == 0 || count.signum() <= 0) {
      return copy(data);
    }
    long n = unitBits == 1 ? data.length * 8L : data.length;
    BigInteger bn = BigInteger.valueOf(n);
    BigInteger[] qr = count.divideAndRemainder(bn);
    long base = offset.mod(bn).longValue();
    return parityFlip(data, unitBits, base, qr[0].testBit(0), qr[1].longValue());
  }

  /**
   * Inverts {@code count} consecutive bits from {@code bitOffset mod (8 * len)}. Bits
   * are numbered LSB-first within each byte: bit 9 of {@code 00 00} is bit 1 of
   * byte 1, so {@code bitFlip(00 00, 9, 1)} is {@code 00 02}. An empty input or
   * {@code count <= 0} is a no-op.
   */
  public static byte[] bitFlip(byte[] data, long bitOffset, long count) {
    return flip(data, 1, bitOffset, count);
  }

  /** {@link #bitFlip} for arbitrary-size integers. */
  public static byte[] bitFlipBig(byte[] data, BigInteger bitOffset, BigInteger count) {
    return flipBig(data, 1, bitOffset, count);
  }

  /** XORs {@code count} consecutive bytes with 0xFF from {@code byteOffset mod len}. */
  public static byte[] byteFlip(byte[] data, long byteOffset, long count) {
    return flip(data, 8, byteOffset, count);
  }

  /** {@link #byteFlip} for arbitrary-size integers. */
  public static byte[] byteFlipBig(byte[] data, BigInteger byteOffset, BigInteger count) {
    return flipBig(data, 8, byteOffset, count);
  }

  // ---------------------------------------------------------------------------
  // Arithmetic and boundary values
  // ---------------------------------------------------------------------------

  private static long readUint(byte[] b, int at, int w, Endian e) {
    long v = 0;
    for (int i = 0; i < w; i++) {
      long x = b[at + i] & 0xFFL;
      if (e == Endian.BIG) {
        v = (v << 8) | x;
      } else {
        v |= x << (8 * i);
      }
    }
    return v;
  }

  /** Stores the low {@code w} bytes of {@code v}: that is "v mod 2^(8w)". */
  private static void writeUint(byte[] b, int at, int w, Endian e, long v) {
    for (int i = 0; i < w; i++) {
      int shift = e == Endian.BIG ? 8 * (w - 1 - i) : 8 * i;
      b[at + i] = (byte) (v >>> shift);
    }
  }

  private static byte[] arithAt(byte[] data, int at, int w, long delta, Endian e) {
    byte[] buf = copy(data);
    writeUint(buf, at, w, e, readUint(buf, at, w, e) + delta); // mod 2^64, then truncated
    return buf;
  }

  private static byte[] boundaryAt(byte[] data, int at, int w, long val, Endian e) {
    byte[] buf = copy(data);
    writeUint(buf, at, w, e, val);
    return buf;
  }

  /**
   * Adds the signed {@code delta} modulo 2^(8 * width) to the integer at the offset.
   * {@code width} is in {1, 2, 4, 8}; any other width, or {@code len < width},
   * returns the input unchanged. The offset wraps over the {@code len - width + 1}
   * positions where a whole integer fits.
   *
   * <p>Worked example (u16, little-endian): {@code ff 00} is 0x00ff; adding 1 gives
   * 0x0100, stored little-endian as {@code 00 01}.
   */
  public static byte[] byteArithmetic(byte[] data, long byteOffset, long delta, int width, Endian e) {
    if (!validWidth(width) || data.length < width) {
      return copy(data);
    }
    return arithAt(data, floorMod(byteOffset, data.length - width + 1), width, delta, e);
  }

  /** {@link #byteArithmetic} for arbitrary-size integers. */
  public static byte[] byteArithmeticBig(byte[] data, BigInteger byteOffset, BigInteger delta, int width, Endian e) {
    if (!validWidth(width) || data.length < width) {
      return copy(data);
    }
    return arithAt(data, modBig(byteOffset, data.length - width + 1), width, low64(delta), e);
  }

  /**
   * Overwrites {@code width} bytes with {@code val mod 2^(8 * width)} at the offset.
   * {@code val} is an unsigned word carried in a long. Same width and short-buffer
   * rules as {@link #byteArithmetic}. Worked example:
   * {@code boundaryInsert(00 00 00 00, 0, 4, 0x80000000, little)} is
   * {@code 00 00 00 80}.
   */
  public static byte[] boundaryInsert(byte[] data, long byteOffset, int width, long val, Endian e) {
    if (!validWidth(width) || data.length < width) {
      return copy(data);
    }
    return boundaryAt(data, floorMod(byteOffset, data.length - width + 1), width, val, e);
  }

  /** {@link #boundaryInsert} for arbitrary-size integers (a negative val is its two's complement). */
  public static byte[] boundaryInsertBig(byte[] data, BigInteger byteOffset, int width, BigInteger val, Endian e) {
    if (!validWidth(width) || data.length < width) {
      return copy(data);
    }
    return boundaryAt(data, modBig(byteOffset, data.length - width + 1), width, low64(val), e);
  }

  // ---------------------------------------------------------------------------
  // Blocks and splice
  // ---------------------------------------------------------------------------

  private static byte[] deleteAt(byte[] data, int at, long length) {
    int end = at + (int) Math.min(length, (long) (data.length - at));
    byte[] out = new byte[data.length - (end - at)];
    System.arraycopy(data, 0, out, 0, at);
    System.arraycopy(data, end, out, at, data.length - end);
    return out;
  }

  /** Deletes up to {@code length} bytes at {@code offset mod len}, clamped at the end. */
  public static byte[] blockDelete(byte[] data, long offset, long length) {
    if (data.length == 0 || length <= 0) {
      return copy(data);
    }
    return deleteAt(data, floorMod(offset, data.length), length);
  }

  /** {@link #blockDelete} for arbitrary-size integers. */
  public static byte[] blockDeleteBig(byte[] data, BigInteger offset, BigInteger length) {
    if (data.length == 0 || length.signum() <= 0) {
      return copy(data);
    }
    return deleteAt(data, modBig(offset, data.length), satLong(length));
  }

  private static byte[] insertAt(byte[] data, int at, byte[] ins, int insFrom, int insLen) {
    byte[] out = new byte[data.length + insLen];
    System.arraycopy(data, 0, out, 0, at);
    System.arraycopy(ins, insFrom, out, at, insLen);
    System.arraycopy(data, at, out, at + insLen, data.length - at);
    return out;
  }

  /** Inserts at {@code offset mod (len + 1)}; an empty input becomes the insert. */
  public static byte[] blockInsert(byte[] data, long offset, byte[] insert) {
    if (data.length == 0) {
      return copy(insert);
    }
    return insertAt(data, floorMod(offset, data.length + 1), insert, 0, insert.length);
  }

  /** {@link #blockInsert} for an arbitrary-size offset. */
  public static byte[] blockInsertBig(byte[] data, BigInteger offset, byte[] insert) {
    if (data.length == 0) {
      return copy(insert);
    }
    return insertAt(data, modBig(offset, data.length + 1), insert, 0, insert.length);
  }

  private static byte[] duplicateAt(byte[] data, int src, long length, int dst) {
    int n = (int) Math.min(length, (long) (data.length - src));
    return insertAt(data, dst, Arrays.copyOfRange(data, src, src + n), 0, n);
  }

  /**
   * Copies up to {@code length} bytes from {@code srcOffset mod len} and inserts them
   * at {@code dstOffset mod (len + 1)} (the end of the buffer is a valid insertion
   * point); the copy is clamped at the end of the source.
   */
  public static byte[] blockDuplicate(byte[] data, long srcOffset, long length, long dstOffset) {
    if (data.length == 0 || length <= 0) {
      return copy(data);
    }
    return duplicateAt(data, floorMod(srcOffset, data.length), length, floorMod(dstOffset, data.length + 1));
  }

  /** {@link #blockDuplicate} for arbitrary-size integers. */
  public static byte[] blockDuplicateBig(byte[] data, BigInteger srcOffset, BigInteger length, BigInteger dstOffset) {
    if (data.length == 0 || length.signum() <= 0) {
      return copy(data);
    }
    return duplicateAt(data, modBig(srcOffset, data.length), satLong(length), modBig(dstOffset, data.length + 1));
  }

  private static int clampSplit(long split, int n) {
    return (int) Math.max(0, Math.min(split, (long) n));
  }

  /** {@code target[:ts] + donor[ds:]} with both splits clamped into their buffers. */
  public static byte[] splice(byte[] target, byte[] donor, long targetSplit, long donorSplit) {
    int t = clampSplit(targetSplit, target.length);
    int d = clampSplit(donorSplit, donor.length);
    byte[] out = new byte[t + donor.length - d];
    System.arraycopy(target, 0, out, 0, t);
    System.arraycopy(donor, d, out, t, donor.length - d);
    return out;
  }

  /** {@link #splice} for arbitrary-size splits. */
  public static byte[] spliceBig(byte[] target, byte[] donor, BigInteger targetSplit, BigInteger donorSplit) {
    return splice(target, donor, satLong(targetSplit), satLong(donorSplit));
  }

  /** The length {@link #splice} must produce, computed independently for the runner's invariant. */
  static long spliceLength(int targetLen, int donorLen, long targetSplit, long donorSplit) {
    return (long) clampSplit(targetSplit, targetLen) + donorLen - clampSplit(donorSplit, donorLen);
  }

  // ---------------------------------------------------------------------------
  // mutateSequence
  // ---------------------------------------------------------------------------

  /**
   * Applies {@code steps} seeded mutations, each chosen by one {@code bounded(7)} draw:
   *
   * <pre>
   *   draw  step
   *   0     bit flip, count 1
   *   1     byte flip, count 1
   *   2     byte arithmetic, width 1, delta from (-35 -8 -1 1 8 35)
   *   3     boundary insert, width 1, value from (0 1 127 128 255)
   *   4     block delete (length 1..len/2); skipped if len &lt;= 1
   *   5     block insert of one random byte
   *   6     block duplicate (length 1..len/2); skipped if len &lt;= 1
   * </pre>
   *
   * An empty buffer is refilled to {@code 00} before every step. The draw order is
   * the cross-lane contract: the same (data, seed, steps) gives the same bytes in
   * every language. Draws for a skipped step are still consumed.
   *
   * <p>{@code maxLen} is VER06 section 3.5's "maximum length budget": a block insert
   * or duplicate that would grow the buffer past it is skipped AFTER its draws are
   * consumed, so the stream stays aligned and memory stays bounded.
   *
   * <p>A negative {@code steps} runs none. {@code steps} above {@link #MAX_ITERATIONS},
   * an input above {@link #MAX_INPUT_BYTES}, or a call whose cumulative copying
   * would exceed {@link #MAX_SEQUENCE_WORK} throws {@link IllegalArgumentException}.
   */
  public static byte[] mutateSequence(byte[] data, long seed, int steps, int maxLen) {
    return mutateSequence(data, new SplitMix64(seed), steps, maxLen);
  }

  /** {@link #mutateSequence(byte[], long, int, int)} over a caller-built stream (the sabotage seam). */
  static byte[] mutateSequence(byte[] data, SplitMix64 rng, int steps, int maxLen) {
    return mutateSequence(data, rng, steps, maxLen, MAX_SEQUENCE_WORK);
  }

  /** The same with an explicit work cap, so a test can make the cap bind. */
  static byte[] mutateSequence(byte[] data, SplitMix64 rng, int steps, int maxLen, long workCap) {
    return counted(data, rng, steps, maxLen, workCap).data();
  }

  /**
   * Thrown when a sequence exhausts its work cap. It carries the bytes of copying the
   * call really did before refusing, so a caller keeping an aggregate budget can charge
   * a FAILED call too.
   */
  static final class WorkExhausted extends IllegalArgumentException {
    private static final long serialVersionUID = 1L;
    final long work;

    WorkExhausted(long cap, long work) {
      super("mutateSequence work budget of " + cap + " byte copies exhausted");
      this.work = work;
    }
  }

  /** A mutated buffer and the bytes of buffer copying the sequence actually did. */
  record Counted(byte[] data, long work) {}

  /**
   * Public-seed form of {@link #counted(byte[], SplitMix64, int, int, long)}: the corpus
   * runner uses it to charge REAL work (block insert and duplicate grow the buffer, so
   * steps x initial length underestimates) against its aggregate budget.
   */
  static Counted mutateSequenceCounted(byte[] data, long seed, int steps, int maxLen, long workCap) {
    return counted(data, new SplitMix64(seed), steps, maxLen, workCap);
  }

  private static Counted counted(byte[] data, SplitMix64 rng, int steps, int maxLen, long workCap) {
    if (steps > MAX_ITERATIONS) {
      throw new IllegalArgumentException("steps exceeds MAX_ITERATIONS");
    }
    if (data.length > MAX_INPUT_BYTES) {
      throw new IllegalArgumentException("input exceeds MAX_INPUT_BYTES");
    }
    byte[] cur = copy(data);
    long work = cur.length; // the initial copy is work too
    if (work > workCap) {
      throw new WorkExhausted(workCap, 0);
    }
    for (int i = 0; i < steps; i++) {
      if (cur.length == 0) {
        cur = new byte[] {0};
      }
      if (work + cur.length > workCap) {
        throw new WorkExhausted(workCap, work);
      }
      work += cur.length;
      int n = cur.length;
      switch ((int) rng.bounded(7)) {
        case 0 -> cur = flip(cur, 1, rng.bounded(n * 8L), 1);
        case 1 -> cur = flip(cur, 8, rng.bounded(n), 1);
        case 2 -> {
          long delta = DELTAS8[(int) rng.bounded(DELTAS8.length)];
          cur = arithAt(cur, (int) rng.bounded(n), 1, delta, Endian.LITTLE);
        }
        case 3 -> {
          long val = INTEREST8[(int) rng.bounded(INTEREST8.length)];
          cur = boundaryAt(cur, (int) rng.bounded(n), 1, val, Endian.LITTLE);
        }
        case 4 -> {
          if (n > 1) {
            long length = 1 + rng.bounded(Math.max(1, n / 2));
            cur = deleteAt(cur, (int) rng.bounded(n), length);
          }
        }
        case 5 -> {
          byte ins = (byte) rng.bounded(256);
          int at = (int) rng.bounded(n + 1L);
          if (n + 1 <= maxLen) {
            cur = insertAt(cur, at, new byte[] {ins}, 0, 1);
          }
        }
        default -> { // 6
          if (n > 1) {
            long length = 1 + rng.bounded(Math.max(1, n / 2));
            int src = (int) rng.bounded(n);
            int dst = (int) rng.bounded(n + 1L);
            if (n + (int) Math.min(length, (long) (n - src)) <= maxLen) {
              cur = duplicateAt(cur, src, length, dst);
            }
          }
        }
      }
    }
    return new Counted(cur, work);
  }
}
