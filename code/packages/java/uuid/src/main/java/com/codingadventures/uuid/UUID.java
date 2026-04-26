// ============================================================================
// UUID.java — Universally Unique Identifiers (RFC 4122 + RFC 9562)
// ============================================================================
//
// A UUID is a 128-bit label formatted as 32 lowercase hex digits separated by
// hyphens into five groups: 8-4-4-4-12:
//
//   550e8400-e29b-41d4-a716-446655440000
//   ^^^^^^^^ ^^^^ ^    ^^^^  ^^^^^^^^^^^^
//   time-low  mid  ver  clk   node (48 bits)
//
// The version nibble (position 13, the M digit) identifies the generation
// algorithm. The variant field (first nibble of the 4th group) is always
// 8, 9, a, or b for standard RFC 4122 UUIDs (high two bits = 10xxxxxx).
//
// UUID versions implemented here:
// --------------------------------
// v1 — Time-based: encodes the current time as 100-ns intervals since the
//      Gregorian epoch (1582-10-15) plus a random node ID.
// v3 — Name-based (MD5): deterministic hash of a namespace UUID + name string.
// v4 — Random: 122 bits from SecureRandom; most commonly used.
// v5 — Name-based (SHA-1): deterministic hash; prefer over v3 for new code.
// v7 — Time-ordered random (RFC 9562): 48-bit millisecond timestamp in high
//      bits for database index locality, rest is random.
//
// Internal representation:
// ------------------------
// We store UUIDs as two longs: msb (most significant 64 bits) and lsb (least
// significant 64 bits), in big-endian bit ordering. This is the most efficient
// Java representation and matches the JDK's java.util.UUID layout exactly.
//
// Bit layout of msb / lsb:
//   msb[63:32] = time_low       (bits 0-31 of timestamp in v1)
//   msb[31:16] = time_mid       (bits 32-47)
//   msb[15:12] = version nibble (bits 48-51, or 12 bits for v1 time_hi)
//   msb[11:0]  = time_hi        (remaining time bits in v1)
//   lsb[63:62] = variant        (always 10 for RFC 4122)
//   lsb[61:48] = clock_seq      (14 bits)
//   lsb[47:0]  = node           (48 bits)
//

package com.codingadventures.uuid;

import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.SecureRandom;
import java.util.regex.Pattern;

/**
 * A 128-bit Universally Unique Identifier (UUID) per RFC 4122 and RFC 9562.
 *
 * <p>Stored as two {@code long} fields: {@link #msb} (most significant bits,
 * bytes 0–7) and {@link #lsb} (least significant bits, bytes 8–15).
 *
 * <p>Factory methods: {@link #v1()}, {@link #v3(UUID, String)},
 * {@link #v4()}, {@link #v5(UUID, String)}, {@link #v7()}.
 *
 * <pre>{@code
 * UUID u = UUID.v4();
 * System.out.println(u);                    // e.g. "550e8400-e29b-41d4-..."
 * System.out.println(u.version());          // 4
 * System.out.println(u.variant());          // "rfc4122"
 *
 * // Name-based (deterministic)
 * UUID dns = UUID.v5(UUID.NAMESPACE_DNS, "python.org");
 * assertEquals("886313e1-3b8a-5372-9b90-0c9aee199e5d", dns.toString());
 * }</pre>
 */
public final class UUID implements Comparable<UUID> {

    // =========================================================================
    // Constants
    // =========================================================================

    // UUID_PATTERN must be initialized BEFORE any static final UUID constants
    // that call fromString(), because Java initializes static fields in source
    // order and NAMESPACE_* calls fromString() at class-init time.
    //
    // Accepts:
    //   Standard:  "550e8400-e29b-41d4-a716-446655440000"
    //   Uppercase: "550E8400-E29B-41D4-A716-446655440000"
    //   Compact:   "550e8400e29b41d4a716446655440000"
    //   Braced:    "{550e8400-e29b-41d4-a716-446655440000}"
    //   URN:       "urn:uuid:550e8400-e29b-41d4-a716-446655440000"
    private static final Pattern UUID_PATTERN = Pattern.compile(
        "^\\s*(?:urn:uuid:)?\\{?"
        + "([0-9a-fA-F]{8})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{12})"
        + "\\}?\\s*$",
        Pattern.CASE_INSENSITIVE);

    /** The nil UUID: all 128 bits are zero. */
    public static final UUID NIL = new UUID(0L, 0L);

    /** The max UUID: all 128 bits are one. */
    public static final UUID MAX = new UUID(0xFFFFFFFFFFFFFFFFL, 0xFFFFFFFFFFFFFFFFL);

    /** RFC 4122 namespace for fully-qualified domain names. */
    public static final UUID NAMESPACE_DNS  = fromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8");
    /** RFC 4122 namespace for URLs. */
    public static final UUID NAMESPACE_URL  = fromString("6ba7b811-9dad-11d1-80b4-00c04fd430c8");
    /** RFC 4122 namespace for ISO OIDs. */
    public static final UUID NAMESPACE_OID  = fromString("6ba7b812-9dad-11d1-80b4-00c04fd430c8");
    /** RFC 4122 namespace for X.500 distinguished names. */
    public static final UUID NAMESPACE_X500 = fromString("6ba7b814-9dad-11d1-80b4-00c04fd430c8");

    // =========================================================================
    // Fields
    // =========================================================================

    /** Bytes 0–7 (time-low, time-mid, version nibble, time-hi in v1; random in v4). */
    public final long msb;
    /** Bytes 8–15 (variant + clock-seq + node). */
    public final long lsb;

    // =========================================================================
    // Constructors
    // =========================================================================

    /**
     * Construct a UUID from its two 64-bit halves.
     *
     * @param msb most-significant 64 bits (bytes 0–7)
     * @param lsb least-significant 64 bits (bytes 8–15)
     */
    public UUID(long msb, long lsb) {
        this.msb = msb;
        this.lsb = lsb;
    }

    // =========================================================================
    // Factory: parsing
    // =========================================================================

    /**
     * Parse a UUID from its string representation.
     *
     * <p>Accepts standard (hyphenated), compact (no hyphens), braced, and URN
     * forms. Case-insensitive.
     *
     * @param text the UUID string
     * @return the parsed UUID
     * @throws UUIDException if the string is not a valid UUID
     */
    public static UUID fromString(String text) {
        if (text == null) throw new UUIDException("UUID string must not be null");
        var m = UUID_PATTERN.matcher(text.strip());
        if (!m.matches()) throw new UUIDException("Invalid UUID string: '" + text + "'");
        String hex = m.group(1) + m.group(2) + m.group(3) + m.group(4) + m.group(5);
        long msbVal = Long.parseUnsignedLong(hex.substring(0,  16), 16);
        long lsbVal = Long.parseUnsignedLong(hex.substring(16, 32), 16);
        return new UUID(msbVal, lsbVal);
    }

    /**
     * Construct a UUID from exactly 16 bytes in network (big-endian) byte order.
     *
     * @param bytes 16-byte array
     * @return the UUID
     * @throws UUIDException if the array is not exactly 16 bytes
     */
    public static UUID fromBytes(byte[] bytes) {
        if (bytes == null || bytes.length != 16) {
            throw new UUIDException(
                "UUID bytes must be exactly 16, got " + (bytes == null ? "null" : bytes.length));
        }
        long msbVal = bytesToLong(bytes, 0);
        long lsbVal = bytesToLong(bytes, 8);
        return new UUID(msbVal, lsbVal);
    }

    /** Return true if {@code text} is a valid UUID string in any supported format. */
    public static boolean isValid(String text) {
        if (text == null) return false;
        return UUID_PATTERN.matcher(text.strip()).matches();
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /**
     * The version field (bits 48–51 of the UUID, i.e., high nibble of byte 6).
     *
     * <p>Returns 0 for NIL and MAX which have no meaningful version.
     */
    public int version() {
        return (int) ((msb >> 12) & 0xF);
    }

    /**
     * The variant field encoded as a human-readable string.
     *
     * <ul>
     *   <li>{@code "rfc4122"} — standard RFC 4122 UUID (variant 10xx)</li>
     *   <li>{@code "microsoft"} — legacy Microsoft GUID (variant 110x)</li>
     *   <li>{@code "ncs"} — NCS backward-compatible (variant 0xxx)</li>
     *   <li>{@code "reserved"} — future use (variant 1111)</li>
     * </ul>
     */
    public String variant() {
        int top = (int) ((lsb >>> 62) & 0x3);
        if (top <= 1) return "ncs";          // 0xxx or 01xx
        if (top == 2) return "rfc4122";      // 10xx
        // top == 3 → 11xx; distinguish 110x (microsoft) vs 111x (reserved)
        int top3 = (int) ((lsb >>> 61) & 0x7);
        return top3 == 7 ? "reserved" : "microsoft";
    }

    /** True if this is the nil UUID (all zeros). */
    public boolean isNil() { return msb == 0L && lsb == 0L; }

    /** True if this is the max UUID (all ones). */
    public boolean isMax() { return msb == 0xFFFFFFFFFFFFFFFFL && lsb == 0xFFFFFFFFFFFFFFFFL; }

    /**
     * Return the 16 bytes of this UUID in network (big-endian) byte order.
     *
     * <p>bytes[0] is the most significant byte; bytes[15] is the least.
     */
    public byte[] toBytes() {
        byte[] b = new byte[16];
        longToBytes(msb, b, 0);
        longToBytes(lsb, b, 8);
        return b;
    }

    // =========================================================================
    // String representation
    // =========================================================================

    /**
     * Return the standard 8-4-4-4-12 lowercase hyphenated UUID string.
     *
     * <p>Example: {@code "550e8400-e29b-41d4-a716-446655440000"}
     */
    @Override
    public String toString() {
        String hex = String.format("%016x%016x", msb, lsb);
        return hex.substring(0, 8)  + "-"
             + hex.substring(8, 12) + "-"
             + hex.substring(12, 16) + "-"
             + hex.substring(16, 20) + "-"
             + hex.substring(20);
    }

    // =========================================================================
    // Equality / Comparison / Hash
    // =========================================================================

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof UUID other)) return false;
        return msb == other.msb && lsb == other.lsb;
    }

    @Override
    public int hashCode() {
        return Long.hashCode(msb) * 31 + Long.hashCode(lsb);
    }

    /**
     * Lexicographic order by bytes: same as comparing the string representations,
     * and corresponds to temporal order for v7 UUIDs.
     */
    @Override
    public int compareTo(UUID other) {
        int c = Long.compareUnsigned(msb, other.msb);
        return c != 0 ? c : Long.compareUnsigned(lsb, other.lsb);
    }

    // =========================================================================
    // Factory: UUID v4 — Random
    // =========================================================================

    // SecureRandom is thread-safe and lazily initialised.
    private static final SecureRandom SECURE_RANDOM = new SecureRandom();

    /**
     * Generate a UUID v4 (random).
     *
     * <p>Uses {@link SecureRandom} (backed by the OS CSPRNG) for 122 bits of
     * entropy. The remaining 6 bits encode the version (4) and variant (10xx).
     *
     * <p>This is the most commonly used UUID version and is suitable for any
     * context where a unique, non-guessable identifier is needed.
     *
     * @return a new random UUID
     */
    public static UUID v4() {
        byte[] b = new byte[16];
        SECURE_RANDOM.nextBytes(b);
        return stampVersionVariant(b, 4);
    }

    // =========================================================================
    // Factory: UUID v7 — Time-Ordered Random (RFC 9562)
    // =========================================================================
    //
    // Bit layout:
    //   Bits  0-47:  timestamp_ms (48-bit Unix timestamp in milliseconds)
    //   Bits 48-51:  version = 7
    //   Bits 52-63:  rand_a (12 random bits)
    //   Bits 64-65:  variant = 10
    //   Bits 66-127: rand_b (62 random bits)
    //
    // Why time-ordered? v4 UUIDs are random and therefore non-sequential,
    // which causes B-tree page splits on insert and poor cache behaviour.
    // v7 puts a millisecond timestamp in the high 48 bits, so inserts are
    // roughly sequential and the index stays compact.

    /**
     * Generate a UUID v7 (time-ordered random, RFC 9562).
     *
     * <p>The first 48 bits are the current Unix timestamp in milliseconds,
     * ensuring lexicographic sort order matches creation time order. This makes
     * v7 UUIDs ideal as database primary keys.
     *
     * @return a new time-ordered UUID
     */
    public static UUID v7() {
        long tsMs = System.currentTimeMillis();
        byte[] rand = new byte[10];
        SECURE_RANDOM.nextBytes(rand);

        // Build the 128-bit UUID:
        //   byte 0-5:  timestamp_ms (48 bits, big-endian)
        //   byte 6:    version nibble (7) | rand_a[0] & 0x0F  (high 4 bits of rand_a)
        //   byte 7:    rand_a[1]                              (low 8 bits of rand_a)
        //   byte 8:    0x80 | (rand_b[0] & 0x3F)             (variant 10 + 6 bits rand_b)
        //   byte 9-15: rand_b[1..7]
        byte[] raw = new byte[16];
        raw[0] = (byte) ((tsMs >> 40) & 0xFF);
        raw[1] = (byte) ((tsMs >> 32) & 0xFF);
        raw[2] = (byte) ((tsMs >> 24) & 0xFF);
        raw[3] = (byte) ((tsMs >> 16) & 0xFF);
        raw[4] = (byte) ((tsMs >>  8) & 0xFF);
        raw[5] = (byte) ( tsMs        & 0xFF);
        raw[6] = (byte) (0x70 | (rand[0] & 0x0F));
        raw[7] = rand[1];
        raw[8] = (byte) (0x80 | (rand[2] & 0x3F));
        System.arraycopy(rand, 3, raw, 9, 7);
        return fromBytes(raw);
    }

    // =========================================================================
    // Factory: UUID v1 — Time-Based
    // =========================================================================
    //
    // The 60-bit timestamp counts 100-nanosecond intervals since 1582-10-15
    // (the Gregorian epoch), chosen so the timestamp never wraps before 3400 AD.
    //
    // Timestamp field layout:
    //   time_low  (bytes 0-3): timestamp bits 0-31  (least significant)
    //   time_mid  (bytes 4-5): timestamp bits 32-47
    //   version + time_hi (bytes 6-7): version nibble + timestamp bits 48-59
    //
    // We use a random node ID (48 bits with the multicast bit set) instead of
    // the real MAC address, because reading the MAC is unreliable and a privacy
    // risk in modern environments.

    // Number of 100-ns intervals between 1582-10-15 and 1970-01-01
    private static final long GREGORIAN_OFFSET = 122_192_928_000_000_000L;

    // Random 14-bit clock sequence, initialised once per JVM, incremented on
    // clock regression (we omit the regression detection for simplicity).
    private static final int CLOCK_SEQ;
    static {
        byte[] csb = new byte[2];
        SECURE_RANDOM.nextBytes(csb);
        CLOCK_SEQ = ((csb[0] & 0xFF) << 8 | (csb[1] & 0xFF)) & 0x3FFF;
    }

    /**
     * Generate a UUID v1 (time-based).
     *
     * <p>Encodes the current UTC time as 100-ns intervals since 1582-10-15.
     * Uses a random 48-bit node ID (multicast bit set per RFC 4122 §4.5) in
     * place of the real MAC address.
     *
     * @return a new time-based UUID
     */
    public static UUID v1() {
        // time.time_ns() gives nanoseconds; convert to 100-ns intervals and add offset.
        long timestamp = System.nanoTime() / 100 + GREGORIAN_OFFSET;

        // Split the 60-bit timestamp into three fields.
        long timeLow  = timestamp & 0xFFFFFFFFL;
        long timeMid  = (timestamp >> 32) & 0xFFFFL;
        long timeHi   = (timestamp >> 48) & 0x0FFFL;

        // Stamp version nibble into time_hi.
        long timeHiAndVersion = (1L << 12) | timeHi;

        // Build msb (bytes 0–7): time_low[31:0] | time_mid[15:0] | time_hi_version[15:0]
        long msbVal = (timeLow << 32) | (timeMid << 16) | timeHiAndVersion;

        // Clock sequence: 14-bit random value with variant bits stamped in.
        //   byte 8 (top): 0b10xxxxxx | (clock_seq >> 8)
        //   byte 9 (bot): clock_seq & 0xFF
        long clockSeqHi = 0x80L | (CLOCK_SEQ >> 8);
        long clockSeqLow = CLOCK_SEQ & 0xFF;

        // Random node (48 bits), multicast bit set (RFC 4122 §4.5).
        byte[] nodeBytes = new byte[6];
        SECURE_RANDOM.nextBytes(nodeBytes);
        nodeBytes[0] |= 0x01;  // set multicast bit

        long node = 0L;
        for (byte nb : nodeBytes) node = (node << 8) | (nb & 0xFF);

        long lsbVal = (clockSeqHi << 56) | (clockSeqLow << 48) | node;
        return new UUID(msbVal, lsbVal);
    }

    // =========================================================================
    // Factory: UUID v5 — Name-Based (SHA-1)
    // =========================================================================
    //
    // Algorithm (RFC 4122 §4.3):
    //   1. Concatenate namespace.toBytes() + name.getBytes(UTF-8).
    //   2. Compute SHA-1 → 20 bytes.
    //   3. Take the first 16 bytes (truncate last 4).
    //   4. Stamp version nibble to 5.
    //   5. Stamp variant bits to 10xx.
    //
    // Deterministic: the same (namespace, name) always yields the same UUID in
    // every language, because the algorithm is standardised.

    /**
     * Generate a UUID v5 (name-based, SHA-1).
     *
     * <p>Deterministic: same {@code namespace} and {@code name} always produce
     * the same UUID, in any RFC 4122-compliant implementation.
     *
     * <p>RFC test vector:
     * {@code v5(NAMESPACE_DNS, "python.org")} → {@code 886313e1-3b8a-5372-9b90-0c9aee199e5d}
     *
     * @param namespace the UUID namespace (e.g. {@link #NAMESPACE_DNS})
     * @param name      the name string (encoded as UTF-8)
     * @return the name-based UUID
     */
    public static UUID v5(UUID namespace, String name) {
        byte[] data = concat(namespace.toBytes(), name.getBytes(java.nio.charset.StandardCharsets.UTF_8));
        byte[] digest = digest(data, "SHA-1");
        byte[] raw = new byte[16];
        System.arraycopy(digest, 0, raw, 0, 16);
        return stampVersionVariant(raw, 5);
    }

    // =========================================================================
    // Factory: UUID v3 — Name-Based (MD5)
    // =========================================================================
    //
    // Same as v5 but using MD5 (16 bytes, no truncation needed).
    // Use v3 only for compatibility with existing systems; prefer v5 for new code.

    /**
     * Generate a UUID v3 (name-based, MD5).
     *
     * <p>Deterministic: same {@code namespace} and {@code name} always produce
     * the same UUID. Prefer {@link #v5} for new code.
     *
     * <p>RFC test vector:
     * {@code v3(NAMESPACE_DNS, "python.org")} → {@code 6fa459ea-ee8a-3ca4-894e-db77e160355e}
     *
     * @param namespace the UUID namespace (e.g. {@link #NAMESPACE_DNS})
     * @param name      the name string (encoded as UTF-8)
     * @return the name-based UUID
     */
    public static UUID v3(UUID namespace, String name) {
        byte[] data = concat(namespace.toBytes(), name.getBytes(java.nio.charset.StandardCharsets.UTF_8));
        byte[] digest = digest(data, "MD5");
        return stampVersionVariant(digest, 3);
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /**
     * Set the version nibble and RFC 4122 variant bits in a 16-byte array,
     * then wrap in a UUID.
     *
     * <ul>
     *   <li>byte[6] high nibble → version</li>
     *   <li>byte[8] high 2 bits → 10 (RFC 4122 variant)</li>
     * </ul>
     */
    private static UUID stampVersionVariant(byte[] b, int version) {
        b[6] = (byte) ((b[6] & 0x0F) | (version << 4));
        b[8] = (byte) ((b[8] & 0x3F) | 0x80);
        return fromBytes(b);
    }

    private static byte[] concat(byte[] a, byte[] b) {
        byte[] result = new byte[a.length + b.length];
        System.arraycopy(a, 0, result, 0, a.length);
        System.arraycopy(b, 0, result, a.length, b.length);
        return result;
    }

    private static byte[] digest(byte[] data, String algorithm) {
        try {
            MessageDigest md = MessageDigest.getInstance(algorithm);
            return md.digest(data);
        } catch (NoSuchAlgorithmException e) {
            throw new UUIDException("Hash algorithm not available: " + algorithm, e);
        }
    }

    /** Read 8 bytes from {@code b} starting at {@code offset} as a big-endian long. */
    private static long bytesToLong(byte[] b, int offset) {
        long v = 0;
        for (int i = 0; i < 8; i++) {
            v = (v << 8) | (b[offset + i] & 0xFF);
        }
        return v;
    }

    /** Write {@code v} as 8 big-endian bytes into {@code b} starting at {@code offset}. */
    private static void longToBytes(long v, byte[] b, int offset) {
        for (int i = 7; i >= 0; i--) {
            b[offset + i] = (byte) (v & 0xFF);
            v >>>= 8;
        }
    }
}
