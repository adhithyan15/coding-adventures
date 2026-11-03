package com.codingadventures.rng;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

/**
 * The shared rng corpus (spec rng.md, "Conformance corpus"), read from its
 * flat form {@code code/specs/fixtures/rng-v1/cases.txt}.
 *
 * <p>The corpus pins exact outputs every language's rng package must
 * reproduce; its results come from the independent reference
 * {@code code/scripts/rng_conformance.py}. Each line is whitespace-separated:
 *
 * <pre>
 *   case &lt;id&gt; &lt;generator&gt; &lt;seed&gt;
 *   next_u32 &lt;result&gt;
 *   next_u64 &lt;result&gt;
 *   next_float &lt;16 hex digits of the IEEE-754 bits&gt;
 *   next_int_in_range &lt;min&gt; &lt;max&gt; &lt;result | error&gt;
 *   end
 * </pre>
 *
 * <p>Gradle runs tests from the project directory, so the path is relative
 * to {@code code/packages/java/rng}. Java has no unsigned long; u32 and u64
 * values are compared through {@link Long#parseUnsignedLong}.
 */
class ConformanceTest {

    /** The three generators behind one interface. */
    private interface AnyRng {
        long u32();
        long u64();
        double f64();
        long inRange(long min, long max);
    }

    private static AnyRng make(String name, long seed) {
        switch (name) {
            case "lcg": {
                Rng.Lcg g = Rng.Lcg.of(seed);
                return new AnyRng() {
                    public long u32() { return g.nextU32(); }
                    public long u64() { return g.nextU64(); }
                    public double f64() { return g.nextFloat(); }
                    public long inRange(long a, long b) { return g.nextIntInRange(a, b); }
                };
            }
            case "xorshift64": {
                Rng.Xorshift64 g = Rng.Xorshift64.of(seed);
                return new AnyRng() {
                    public long u32() { return g.nextU32(); }
                    public long u64() { return g.nextU64(); }
                    public double f64() { return g.nextFloat(); }
                    public long inRange(long a, long b) { return g.nextIntInRange(a, b); }
                };
            }
            case "pcg32": {
                Rng.Pcg32 g = Rng.Pcg32.of(seed);
                return new AnyRng() {
                    public long u32() { return g.nextU32(); }
                    public long u64() { return g.nextU64(); }
                    public double f64() { return g.nextFloat(); }
                    public long inRange(long a, long b) { return g.nextIntInRange(a, b); }
                };
            }
            default:
                throw new IllegalArgumentException("unknown generator " + name);
        }
    }

    @Test
    void everyCorpusCaseMatches() throws IOException {
        List<String> lines = Files.readAllLines(
                Path.of("../../../specs/fixtures/rng-v1/cases.txt"), StandardCharsets.UTF_8);
        int cases = 0;
        int calls = 0;
        String id = "";
        AnyRng g = null;
        for (String line : lines) {
            if (line.startsWith("#") || line.isEmpty()) {
                continue;
            }
            String[] t = line.split(" ");
            switch (t[0]) {
                case "case":
                    id = t[1];
                    g = make(t[2], Long.parseUnsignedLong(t[3]));
                    cases++;
                    break;
                case "end":
                    g = null;
                    break;
                case "next_u32":
                    calls++;
                    assertEquals(Long.parseUnsignedLong(t[1]), g.u32(), id);
                    break;
                case "next_u64":
                    calls++;
                    assertEquals(Long.parseUnsignedLong(t[1]), g.u64(), id);
                    break;
                case "next_float":
                    calls++;
                    assertEquals(Long.parseUnsignedLong(t[1], 16), Double.doubleToRawLongBits(g.f64()), id);
                    break;
                case "next_int_in_range": {
                    calls++;
                    final long min = Long.parseLong(t[1]);
                    final long max = Long.parseLong(t[2]);
                    final AnyRng gen = g;
                    if (t[3].equals("error")) {
                        // Thrown before any draw, so the sequence carries on.
                        assertThrows(IllegalArgumentException.class, () -> gen.inRange(min, max), id);
                    } else {
                        assertEquals(Long.parseLong(t[3]), gen.inRange(min, max), id);
                    }
                    break;
                }
                default:
                    fail("malformed corpus line: " + line);
            }
        }
        // Guard against a truncated or empty file passing vacuously.
        assertTrue(cases >= 27, "cases: " + cases);
        assertTrue(calls >= 150, "calls: " + calls);
    }

    // ── Regressions for the v1 range bug ─────────────────────────────────────

    @Test
    void wholeLongDomainNoLongerDividesByZero() {
        Rng.Lcg g = Rng.Lcg.of(3);
        boolean negative = false;
        boolean positive = false;
        for (int i = 0; i < 64; i++) {
            long v = g.nextIntInRange(Long.MIN_VALUE, Long.MAX_VALUE);
            negative |= v < 0;
            positive |= v > 0;
        }
        assertTrue(negative && positive);
    }

    @Test
    void wideRangesReachBeyondTwoToThe32() {
        Rng.Pcg32 g = Rng.Pcg32.of(3);
        boolean beyond = false;
        for (int i = 0; i < 64; i++) {
            beyond |= g.nextIntInRange(0, 1L << 40) >= 1L << 32;
        }
        assertTrue(beyond);
    }

    @Test
    void halfDomainPlusOneTerminates() {
        Rng.Xorshift64 g = Rng.Xorshift64.of(3);
        for (int i = 0; i < 100; i++) {
            assertTrue(g.nextIntInRange(Long.MIN_VALUE, 0) <= 0);
        }
    }
}
