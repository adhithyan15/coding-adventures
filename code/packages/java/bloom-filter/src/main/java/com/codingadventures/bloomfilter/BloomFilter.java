// ============================================================================
// BloomFilter.java — Probabilistic set membership filter
// ============================================================================
//
// A Bloom filter answers "Have I seen this element before?" with two possible
// answers:
//
//   "Definitely NO"  — zero false negatives.  If the filter says NO, the
//                      element was NEVER added.  Trust this completely.
//
//   "Probably YES"   — small, tunable probability of false positives.  The
//                      filter says YES, but occasionally the element was never
//                      added.  The false positive rate is controlled by the
//                      parameters you supply at construction time.
//
// This asymmetry makes Bloom filters ideal as a fast pre-flight check before
// expensive operations (disk reads, network calls, cache lookups).  If the
// filter says NO, skip the expensive operation.  If it says YES, do it (and
// occasionally hit a false positive, which is acceptable).
//
// ============================================================================
// Real-world deployments
// ============================================================================
//
//   LevelDB / RocksDB / Cassandra — avoid disk seeks for missing SSTable keys
//   Chrome Safe Browsing          — local check before calling Google's servers
//   Akamai CDN                    — only cache URLs seen at least twice
//   Bitcoin SPV clients           — filter transactions by address
//
// ============================================================================
// How it works: bit array + multiple hash functions
// ============================================================================
//
//   The filter is a bit array of m bits, all initially 0.
//
//   To ADD an element:    compute k bit positions; set those k bits to 1.
//   To CHECK an element:  compute the same k positions; if ALL are 1, return
//                         "probably yes"; if ANY is 0, return "definitely no".
//
//   Example (m=16 bits, k=3 hash functions):
//
//   Empty:       0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0
//                0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15
//
//   Add "alice" (h1→3, h2→7, h3→11):
//                0  0  0  1  0  0  0  1  0  0  0  1  0  0  0  0
//                               ↑              ↑              ↑
//
//   Add "bob"  (h1→1, h2→5, h3→11):
//                0  1  0  1  0  1  0  1  0  0  0  1  0  0  0  0
//                   ↑          ↑                    ↑ (shared w/ alice)
//
//   Check "carol" (h1→2, ...): bit 2 is 0 → "Definitely NO"
//   Check "dave"  (h1→1, h2→5, h3→11): all 1 → "Probably YES" — FALSE POSITIVE!
//
// ============================================================================
// Why deletion is impossible
// ============================================================================
//
//   Bit 11 was set by BOTH "alice" and "bob".  Clearing bob's bits would
//   clear bit 11, breaking alice's membership.  Use a Counting Bloom Filter
//   (with 4-bit counters per cell) for deletable sets.
//
// ============================================================================
// Optimal parameters
// ============================================================================
//
//   Given expected item count n and desired false positive rate p:
//
//     m = ceil(-n × ln(p) / ln(2)²)   ← optimal number of bits
//     k = round((m / n) × ln(2))      ← optimal number of hash functions
//
//   Memory comparison (n = 1,000,000 elements):
//
//     FPR    Bits/elem   Total bits    Memory
//     -----  ---------   ----------    --------
//     10%    4.79        4,792,536     585 KB
//      1%    9.58        9,585,059     1.14 MB
//     0.1%  14.38       14,377,588     1.72 MB
//     vs. exact HashSet: ~40 MB (35× larger!)
//
// ============================================================================
// Double hashing: k positions from two hash functions
// ============================================================================
//
//   Computing k independent hash functions is expensive.  Instead, use the
//   "double hashing" trick: given two functions h1 and h2,
//
//     g_i(x) = (h1(x) + i × h2(x)) mod m   for i = 0, 1, ..., k-1
//
//   This generates k distinct positions from only two hash computations.
//   Used by Google's Guava library, Redis, and most production implementations.
//
//   h1 = FNV-1a 32-bit (Fowler-Noll-Vo), decorated with fmix32 finalizer
//   h2 = DJB2 64-bit (Dan Bernstein), folded to 32 bits then fmix32-decorated
//
//   The fmix32 finalizer (from MurmurHash3, public domain) breaks the
//   correlation between h1 and h2 for strings sharing a common prefix.
//
// ============================================================================
// Bit array storage
// ============================================================================
//
//   Bits are packed into a byte array, 8 bits per byte:
//
//     bit index i → byte index  = i >>> 3    (i / 8)
//                  bit offset   = i & 7      (i % 8)
//
//     Set bit i:  bytes[i >>> 3] |= (1 << (i & 7))
//     Test bit i: (bytes[i >>> 3] & (1 << (i & 7))) != 0
//

package com.codingadventures.bloomfilter;

import java.nio.charset.StandardCharsets;

/**
 * Space-efficient probabilistic set membership filter.
 *
 * <p>Never has false negatives: if {@link #contains} returns {@code false},
 * the element is guaranteed not to be in the set.
 *
 * <p>May have false positives: if {@link #contains} returns {@code true},
 * the element is probably in the set — but occasionally it was never added.
 * The false positive rate is controlled by {@code expectedItems} and
 * {@code falsePositiveRate}.
 *
 * <pre>{@code
 * BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
 * bf.add("hello");
 * bf.contains("hello");   // true  (definitely was added)
 * bf.contains("world");   // false (definitely not added — zero FN)
 * }</pre>
 */
public final class BloomFilter<T> {

    // =========================================================================
    // Internal hash function constants
    // =========================================================================

    // FNV-1a 32-bit constants (Fowler-Noll-Vo)
    // The offset basis is the initial hash value; the prime drives bit spreading.
    private static final int FNV32_OFFSET_BASIS = 0x811C9DC5; // 2166136261
    private static final int FNV32_PRIME        = 0x01000193; // 16777619

    // 64-bit mask for DJB2 arithmetic
    private static final long MASK64 = 0xFFFFFFFFFFFFFFFFL;

    // 32-bit unsigned mask for folding DJB2 output to 32 bits
    private static final long MASK32 = 0xFFFFFFFFL;

    // Maximum allowed bit array size — 2^30 bits ≈ 128 MB.
    // Prevents unbounded memory allocation from large expectedItems inputs.
    // Supports ~150 million elements at 1% FPR with optimal parameters.
    private static final long MAX_BITS = 1L << 30;

    // =========================================================================
    // State
    // =========================================================================

    private final int      m;           // total number of bits
    private final int      k;           // number of hash functions
    private final int      nExpected;   // capacity we were sized for (0 = unknown)
    private final byte[]   bits;        // packed bit array: (m + 7) / 8 bytes
    private       int      bitsSet;     // number of bits currently set to 1
    private       int      n;           // number of elements added

    // =========================================================================
    // Construction
    // =========================================================================

    /**
     * Create a Bloom filter sized for the given parameters.
     *
     * <p>Automatically computes the optimal bit count {@code m} and hash
     * function count {@code k} using the standard formulas:
     *
     * <pre>
     *   m = ceil(-n × ln(p) / ln(2)²)
     *   k = max(1, round((m / n) × ln(2)))
     * </pre>
     *
     * @param expectedItems     how many distinct elements you plan to add (n)
     * @param falsePositiveRate target false positive rate, e.g. 0.01 = 1%
     * @throws IllegalArgumentException if either parameter is out of range
     */
    public BloomFilter(int expectedItems, double falsePositiveRate) {
        if (expectedItems <= 0) {
            throw new IllegalArgumentException(
                "expectedItems must be positive, got: " + expectedItems);
        }
        if (falsePositiveRate <= 0.0 || falsePositiveRate >= 1.0) {
            throw new IllegalArgumentException(
                "falsePositiveRate must be in the open interval (0, 1), got: "
                + falsePositiveRate);
        }

        double n_ = expectedItems;
        double p  = falsePositiveRate;
        double ln2 = Math.log(2.0);

        // Optimal bit count.
        long mLong = (long) Math.ceil(-n_ * Math.log(p) / (ln2 * ln2));
        if (mLong > MAX_BITS) {
            throw new IllegalArgumentException(
                "Required bit array (" + mLong + " bits) exceeds the maximum allowed "
                + "size (" + MAX_BITS + " bits = 128 MB). "
                + "Reduce expectedItems or increase falsePositiveRate.");
        }

        this.m         = (int) mLong;
        this.k         = Math.max(1, (int) Math.round((this.m / n_) * ln2));
        this.nExpected = expectedItems;
        this.bits      = new byte[(this.m + 7) / 8];
        this.bitsSet   = 0;
        this.n         = 0;
    }

    /**
     * Create a filter with explicit bit count and hash count.
     *
     * <p>Bypasses the auto-sizing formula. Useful when you know the exact
     * parameters (e.g., replicating another implementation).
     *
     * @param bitCount  total number of bits m
     * @param hashCount number of hash functions k
     */
    public BloomFilter(int bitCount, int hashCount, boolean explicit) {
        if (bitCount <= 0) {
            throw new IllegalArgumentException(
                "bitCount must be positive, got: " + bitCount);
        }
        if (hashCount <= 0) {
            throw new IllegalArgumentException(
                "hashCount must be positive, got: " + hashCount);
        }
        if (bitCount > MAX_BITS) {
            throw new IllegalArgumentException(
                "bitCount (" + bitCount + ") exceeds the maximum allowed size ("
                + MAX_BITS + " bits = 128 MB).");
        }
        this.m         = bitCount;
        this.k         = hashCount;
        this.nExpected = 0;  // no capacity target
        this.bits      = new byte[(bitCount + 7) / 8];
        this.bitsSet   = 0;
        this.n         = 0;
    }

    // =========================================================================
    // Internal hash functions
    // =========================================================================
    //
    // Both functions operate on the UTF-8 encoding of element.toString().
    //
    // FNV-1a 32-bit: XOR then multiply for each byte.
    //   Good avalanche for short strings; the prime is sparse in binary
    //   (few set bits) → fast multiply on older hardware.
    //
    // DJB2: multiply-by-33 (via shift+add) then add the byte.
    //   hash = 5381; for each byte: hash = hash * 33 + byte
    //   Why 33? It's prime and produces good distribution for ASCII strings.
    //   Why 5381? Empirically by Bernstein — fewer collisions for English words.
    //
    // fmix32 (MurmurHash3 finalizer): a bijective bit mixer that breaks the
    //   correlation between fnv1a and djb2 for strings with common prefixes.
    //   Without mixing, the two hashes cluster and push the actual FPR above
    //   the theoretical value.  Three constants from Austin Appleby, public domain.

    /**
     * FNV-1a 32-bit hash.
     *
     * <p>Processes one byte at a time:
     * <ol>
     *   <li>XOR the current hash with the byte (mixes the byte in).</li>
     *   <li>Multiply by the FNV prime (avalanches the change across 32 bits).</li>
     * </ol>
     *
     * <p>Known test vectors: {@code fnv1a32("") == 0x811C9DC5},
     * {@code fnv1a32("a") == 0x050C5D1F}.
     */
    private static int fnv1a32(byte[] data) {
        int h = FNV32_OFFSET_BASIS;
        for (byte b : data) {
            h ^= (b & 0xFF);      // XOR the byte (unsigned)
            h *= FNV32_PRIME;     // multiply by FNV prime
        }
        return h;
    }

    /**
     * DJB2 hash (Dan Bernstein), output folded to 32 bits.
     *
     * <p>Algorithm: {@code hash = hash * 33 + byte} for each byte.
     * The multiply-by-33 is written as {@code (hash << 5) + hash} (one shift +
     * one add) to exploit fast shift-add on hardware without a multiplier.
     *
     * <p>The full 64-bit output is folded: {@code (h ^ (h >>> 32)) & 0xFFFFFFFF}.
     */
    private static int djb2_32(byte[] data) {
        long h = 5381L;
        for (byte b : data) {
            // h = h * 33 + b, masked to 64 bits
            h = ((h << 5) + h + (b & 0xFFL)) & MASK64;
        }
        // Fold to 32 bits: XOR the high 32 bits into the low 32 bits
        return (int) ((h ^ (h >>> 32)) & MASK32);
    }

    /**
     * MurmurHash3 32-bit finalizer (fmix32).
     *
     * <p>A bijective (invertible) bit mixer: every output bit depends on every
     * input bit. Applied independently to h1 and h2 it breaks their correlation
     * for strings sharing a common prefix — preventing cluster-based FPR bloat.
     *
     * <p>Constants from Austin Appleby's MurmurHash3 (public domain).
     */
    private static int fmix32(int h) {
        h ^= (h >>> 16);
        h *= 0x85EBCA6B;
        h ^= (h >>> 13);
        h *= 0xC2B2AE35;
        h ^= (h >>> 16);
        return h;
    }

    /**
     * Compute k bit indices for the element using double hashing.
     *
     * <p>Double hashing trick: derive k hash functions from two:
     *
     * <pre>
     *   g_i(x) = (h1(x) + i × h2(x)) mod m   for i = 0, 1, ..., k-1
     * </pre>
     *
     * <p>h2 is forced odd (bit 0 set) so that with an odd-sized array all m
     * positions are reachable before repeating — maximising spread.
     */
    private int[] hashIndices(T element) {
        byte[] raw = element.toString().getBytes(StandardCharsets.UTF_8);

        // Two independent (decorrelated) hash values.
        int h1 = fmix32(fnv1a32(raw));
        int h2 = fmix32(djb2_32(raw)) | 1;  // force odd

        // Convert to longs to do unsigned arithmetic safely.
        long lh1 = h1 & 0xFFFFFFFFL;
        long lh2 = h2 & 0xFFFFFFFFL;

        int[] indices = new int[k];
        for (int i = 0; i < k; i++) {
            // (h1 + i*h2) mod m — all arithmetic in long to avoid overflow.
            indices[i] = (int) ((lh1 + (long) i * lh2) % (long) m);
        }
        return indices;
    }

    // =========================================================================
    // Core operations
    // =========================================================================

    /**
     * Add an element to the filter.
     *
     * <p>Sets up to k bits in the bit array. Bits already set (by previous
     * elements) are not double-counted in {@code bitsSet}.
     *
     * <p>O(k) time.
     *
     * @param element the element to add; {@code toString()} is used as its key
     */
    public void add(T element) {
        for (int idx : hashIndices(element)) {
            int byteIdx = idx >>> 3;        // idx / 8
            int bitMask = 1 << (idx & 7);   // idx % 8
            if ((bits[byteIdx] & bitMask) == 0) {
                bits[byteIdx] |= bitMask;
                bitsSet++;
            }
        }
        n++;
    }

    /**
     * Check if an element might be in the filter.
     *
     * <ul>
     *   <li><b>Returns {@code false}</b> → element is DEFINITELY not in the filter.
     *       Zero false negatives. At least one of the k bit positions is 0.</li>
     *   <li><b>Returns {@code true}</b> → element is PROBABLY in the filter.
     *       False positive rate bounded by the parameters at construction.</li>
     * </ul>
     *
     * <p>O(k) time.
     *
     * @param element the element to check
     * @return {@code true} if probably present, {@code false} if definitely absent
     */
    public boolean contains(T element) {
        for (int idx : hashIndices(element)) {
            int byteIdx = idx >>> 3;
            int bitMask = 1 << (idx & 7);
            if ((bits[byteIdx] & bitMask) == 0) {
                return false;  // definitely absent
            }
        }
        return true;  // probably present
    }

    // =========================================================================
    // Properties and statistics
    // =========================================================================

    /**
     * Total number of bits in the filter (m). Fixed at construction.
     */
    public int bitCount() { return m; }

    /**
     * Number of hash functions used (k). Fixed at construction.
     */
    public int hashCount() { return k; }

    /**
     * Number of bits currently set to 1.
     */
    public int bitsSet() { return bitsSet; }

    /**
     * Number of elements added via {@link #add}.
     */
    public int size() { return n; }

    /**
     * Fraction of bits currently set to 1: {@code bitsSet / bitCount}.
     *
     * <p>Starts at 0.0 for an empty filter. At fill ratio ≈ 0.5, the filter
     * is near its optimal operating point. Approaches 1.0 as it fills up.
     */
    public double fillRatio() {
        return (double) bitsSet / (double) m;
    }

    /**
     * Estimated current false positive rate based on fill ratio.
     *
     * <p>Formula: {@code fillRatio^k}.  Returns 0.0 for an empty filter.
     * Rises toward 1.0 as the filter is over-filled.
     */
    public double estimatedFalsePositiveRate() {
        if (bitsSet == 0) return 0.0;
        return Math.pow(fillRatio(), k);
    }

    /**
     * True if more elements than {@code expectedItems} have been added.
     *
     * <p>When over capacity, the actual FPR rises above the target rate.
     * The filter still works (no false negatives) but false positives increase.
     *
     * <p>Always returns {@code false} for filters created with the explicit
     * constructor (no capacity was specified).
     */
    public boolean isOverCapacity() {
        if (nExpected == 0) return false;
        return n > nExpected;
    }

    /**
     * Memory usage of the bit array in bytes: {@code ceil(m / 8)}.
     */
    public int sizeBytes() { return bits.length; }

    // =========================================================================
    // Static utility methods
    // =========================================================================

    /**
     * Optimal bit array size for {@code n} elements and false positive rate {@code p}.
     *
     * <p>Formula: {@code m = ceil(-n × ln(p) / ln(2)²)}
     *
     * @param n number of expected elements
     * @param p target false positive rate (0 < p < 1)
     * @return optimal bit count
     */
    public static long optimalM(long n, double p) {
        if (n <= 0) throw new IllegalArgumentException("n must be positive");
        if (p <= 0.0 || p >= 1.0) throw new IllegalArgumentException("p must be in (0, 1)");
        double ln2 = Math.log(2.0);
        return (long) Math.ceil(-n * Math.log(p) / (ln2 * ln2));
    }

    /**
     * Optimal number of hash functions for {@code m} bits and {@code n} elements.
     *
     * <p>Formula: {@code k = max(1, round((m / n) × ln(2)))}
     *
     * <p>Intuition: k_optimal ≈ 0.693 × (m/n). At this k, each bit has about a
     * 50% chance of being set, which minimises the false positive rate.
     *
     * @param m bit count
     * @param n element count
     * @return optimal hash function count
     */
    public static int optimalK(long m, long n) {
        if (n <= 0) throw new IllegalArgumentException("n must be positive");
        return Math.max(1, (int) Math.round((double) m / (double) n * Math.log(2.0)));
    }

    /**
     * How many elements can be stored in {@code memoryBytes} at rate {@code p}?
     *
     * <p>Inverse of {@link #optimalM}: {@code n = -m × ln(2)² / ln(p)}
     *
     * @param memoryBytes available memory in bytes
     * @param p           target false positive rate
     * @return maximum element count
     */
    public static long capacityForMemory(long memoryBytes, double p) {
        if (p <= 0.0 || p >= 1.0) throw new IllegalArgumentException("p must be in (0, 1)");
        long m = memoryBytes * 8L;  // bytes to bits
        double ln2 = Math.log(2.0);
        return (long) (-m * (ln2 * ln2) / Math.log(p));
    }

    // =========================================================================
    // toString
    // =========================================================================

    @Override
    public String toString() {
        return String.format(
            "BloomFilter(m=%d, k=%d, bitsSet=%d/%d (%.2f%%), ~fp=%.4f%%)",
            m, k, bitsSet, m, fillRatio() * 100.0,
            estimatedFalsePositiveRate() * 100.0);
    }
}
