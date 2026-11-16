// ============================================================================
// ConformanceTest.java — bit-exact conformance against the TRIG-REF-1 vectors
// ============================================================================
//
// code/specs/fixtures/trig-reference-v1 holds every vector as one row
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// where x and y are IEEE binary64 *bit patterns* (16 lowercase hex digits),
// never decimals.  This test decodes x with Double.longBitsToDouble, calls the
// public function, encodes the result with Double.doubleToRawLongBits and
// compares the two 64-bit integers (PHY00a 6.5).  Printed doubles are never
// compared: they hide the last digits.  The only relaxation is NaN, which
// matches any NaN (VER04p "same": payload and sign of a NaN are not portable).
//
// Three kinds of check live here:
//
//   1. fixture integrity  — the files on disk are the ones the manifest hashes;
//   2. bit-for-bit match  — every row, per group, with failing rows counted per op;
//   3. accuracy vs `cr`   — every row that carries a correctly rounded value is
//                           within the stated number of representable steps.
//
// plus the constants (their bits are asserted, PHY00a 6.5 e) and the named
// contract points of PHY00a 3.3.

package com.codingadventures.trig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

class ConformanceTest {

    /** Gradle runs tests with the project directory (code/packages/java/trig) as cwd. */
    private static final Path FIXTURE =
        Paths.get("../../../specs/fixtures/trig-reference-v1");

    private static final String[] GROUPS = {"edge", "sweep-binade", "sweep-cw", "sweep-prng"};
    private static final long QNAN_BITS = 0x7FF8000000000000L;

    /** One parsed fixture row. */
    private record Row(String op, String x, String y, String cr, String flags) {}

    // -------------------------------------------------------------------------
    // Helpers: bit casts, parsing, dispatch
    // -------------------------------------------------------------------------

    private static double fromHex(String hex) {
        return Double.longBitsToDouble(Long.parseUnsignedLong(hex, 16));
    }

    private static long bitsOf(double v) {
        return Double.doubleToRawLongBits(v);
    }

    private static String hex16(long bits) {
        return String.format("%016x", bits);
    }

    private static List<Row> load(String group) throws IOException {
        String text = Files.readString(FIXTURE.resolve(group + ".tsv"), StandardCharsets.UTF_8);
        assertTrue(text.endsWith("\n"), group + ": final newline");
        List<Row> rows = new ArrayList<>();
        for (String line : text.split("\n", -1)) {
            if (line.isEmpty()) {
                continue;
            }
            String[] f = line.split("\t", -1);
            assertEquals(5, f.length, line);
            rows.add(new Row(f[0], f[1], f[2], f[3], f[4]));
        }
        return rows;
    }

    /** Call the public function named by {@code op}; an unknown op fails the test. */
    private static double call(String op, String x) {
        if (op.equals("atan2")) {
            int comma = x.indexOf(',');
            return Trig.atan2(fromHex(x.substring(0, comma)), fromHex(x.substring(comma + 1)));
        }
        double v = fromHex(x);
        switch (op) {
            case "sin":     return Trig.sin(v);
            case "cos":     return Trig.cos(v);
            case "tan":     return Trig.tan(v);
            case "radians": return Trig.radians(v);
            case "degrees": return Trig.degrees(v);
            case "sqrt":    return Trig.sqrt(v);
            case "atan":    return Trig.atan(v);
            default:
                fail("unknown op in fixture: " + op);
                return 0.0; // unreachable
        }
    }

    /** True when the row's result matches the reference under VER04p "same". */
    private static boolean matches(Row r) {
        if (r.y().equals("error")) {
            // sqrt of a negative: this port's idiomatic error is ArithmeticException.
            try {
                call(r.op(), r.x());
            } catch (ArithmeticException expected) {
                return true;
            }
            return false;
        }
        double got = call(r.op(), r.x());
        long want = Long.parseUnsignedLong(r.y(), 16);
        if (want == QNAN_BITS) {
            return Double.isNaN(got);
        }
        return bitsOf(got) == want;
    }

    // -------------------------------------------------------------------------
    // 1. Fixture integrity
    // -------------------------------------------------------------------------

    private static String manifest() throws IOException {
        return Files.readString(FIXTURE.resolve("manifest.json"), StandardCharsets.UTF_8);
    }

    @Test
    void manifestHashesAndCountsMatchTheFiles() throws Exception {
        String m = manifest();
        assertTrue(m.contains("\"algorithm\": \"TRIG-REF-1\""));
        for (String group : GROUPS) {
            byte[] data = Files.readAllBytes(FIXTURE.resolve(group + ".tsv"));
            StringBuilder sha = new StringBuilder();
            for (byte b : MessageDigest.getInstance("SHA-256").digest(data)) {
                sha.append(String.format("%02x", b));
            }
            Matcher g = Pattern.compile(
                "\"file\": \"" + group + "\\.tsv\",\\s*\"name\": \"" + group
                    + "\",\\s*\"rows\": (\\d+),\\s*\"sha256\": \"([0-9a-f]{64})\"").matcher(m);
            assertTrue(g.find(), "manifest group " + group);
            assertEquals(g.group(2), sha.toString(), group + " sha256");
            long newlines = 0;
            for (byte b : data) {
                if (b == '\n') {
                    newlines++;
                }
            }
            assertEquals(Long.parseLong(g.group(1)), newlines, group + " row count");
        }
    }

    @Test
    void rowSyntaxIsClosed() throws IOException {
        Pattern hex = Pattern.compile("[0-9a-f]{16}");
        List<String> ops = List.of("sin", "cos", "tan", "radians", "degrees", "sqrt", "atan", "atan2");
        for (String group : GROUPS) {
            for (Row r : load(group)) {
                assertTrue(ops.contains(r.op()), "op " + r.op());
                String[] inputs = r.op().equals("atan2") ? r.x().split(",") : new String[] {r.x()};
                assertEquals(r.op().equals("atan2") ? 2 : 1, inputs.length);
                for (String in : inputs) {
                    assertTrue(hex.matcher(in).matches(), "x field " + in);
                }
                assertTrue(r.y().equals("error") || hex.matcher(r.y()).matches(), "y " + r.y());
                assertTrue(r.cr().equals("-") || hex.matcher(r.cr()).matches(), "cr " + r.cr());
            }
        }
    }

    // -------------------------------------------------------------------------
    // Constants: the manifest's table equals the bits the module embeds
    // -------------------------------------------------------------------------

    private static long manifestConstant(String name) throws IOException {
        Matcher c = Pattern.compile("\"" + name + "\": \"([0-9a-f]{16})\"").matcher(manifest());
        assertTrue(c.find(), "manifest constant " + name);
        return Long.parseUnsignedLong(c.group(1), 16);
    }

    @Test
    void constantsMatchTheManifest() throws IOException {
        assertEquals(manifestConstant("PI"), bitsOf(Trig.PI));
        assertEquals(manifestConstant("TWO_PI"), bitsOf(Trig.TWO_PI));
        assertEquals(manifestConstant("HALF_PI"), bitsOf(Trig.HALF_PI));
        assertEquals(manifestConstant("INV_PIO2"), bitsOf(Trig.INV_PIO2));
        for (int i = 0; i < 5; i++) {
            assertEquals(manifestConstant("PIO2_" + (i + 1)), bitsOf(Trig.PIO2[i]), "PIO2_" + (i + 1));
        }
        assertEquals(manifestConstant("MAGIC"), bitsOf(Trig.MAGIC));
        assertEquals(manifestConstant("LIMIT"), bitsOf(Trig.LIMIT));
        assertEquals(manifestConstant("TINY"), bitsOf(Trig.TINY));
        for (int n = 1; n <= 9; n++) {
            assertEquals(manifestConstant("S" + n), bitsOf(Trig.S[n]), "S" + n);
        }
        for (int n = 2; n <= 9; n++) {
            assertEquals(manifestConstant("C" + n), bitsOf(Trig.C[n]), "C" + n);
        }
        assertEquals(manifestConstant("RAD_PER_DEG"), bitsOf(Trig.RAD_PER_DEG));
        assertEquals(manifestConstant("DEG_PER_RAD"), bitsOf(Trig.DEG_PER_RAD));
    }

    /** Decimal literals the source still writes parse to the pinned bit patterns. */
    @Test
    void decimalLiteralsHaveThePinnedBits() {
        assertEquals(0x3CD203AF9EE75616L, bitsOf(1e-15));   // tan pole threshold, sqrt tolerance
        assertEquals(0x7FE1CCF385EBC8A0L, bitsOf(1.0e308)); // tan pole sentinel
        assertEquals(0x3C670EF54646D497L, bitsOf(1e-17));   // atan series cut-off
        assertEquals(0x01A56E1FC2F8F359L, bitsOf(1e-300));  // sqrt subnormal floor
        assertEquals(0x3E40000000000000L, bitsOf(7.450580596923828e-9)); // 2^-27
    }

    @Test
    void degreeConstantsEqualTheDivisions() {
        assertEquals(Trig.RAD_PER_DEG, Trig.PI / 180.0);
        assertEquals(Trig.DEG_PER_RAD, 180.0 / Trig.PI);
    }

    // -------------------------------------------------------------------------
    // 2. Bit-for-bit match, per group, failing rows counted per op
    // -------------------------------------------------------------------------

    private static void checkGroup(String group) throws IOException {
        Map<String, Integer> failing = new TreeMap<>();
        List<String> first = new ArrayList<>();
        int checked = 0;
        for (Row r : load(group)) {
            checked++;
            if (!matches(r)) {
                failing.merge(r.op(), 1, Integer::sum);
                if (first.size() < 8) {
                    first.add(r.op() + " " + r.x() + " want " + r.y());
                }
            }
        }
        assertTrue(checked > 0, group + " has rows");
        assertTrue(failing.isEmpty(),
            group + ": failing rows per op " + failing + "; first " + first);
    }

    @Test
    void edgeVectorsMatchBitForBit() throws IOException {
        checkGroup("edge");
    }

    @Test
    void sweepBinadeVectorsMatchBitForBit() throws IOException {
        checkGroup("sweep-binade");
    }

    @Test
    void sweepCwVectorsMatchBitForBit() throws IOException {
        checkGroup("sweep-cw");
    }

    @Test
    void sweepPrngVectorsMatchBitForBit() throws IOException {
        checkGroup("sweep-prng");
    }

    // -------------------------------------------------------------------------
    // 3. Accuracy against the correctly rounded column
    // -------------------------------------------------------------------------

    /** Map a double's bits onto a monotone integer line so that steps can be counted. */
    private static long ordered(long bits) {
        long mag = bits & 0x7FFFFFFFFFFFFFFFL;
        return bits < 0 ? -mag : mag;
    }

    @Test
    void accuracyAgainstTheCorrectlyRoundedColumn() throws IOException {
        Map<String, Long> limit = Map.of("sin", 3L, "cos", 3L, "radians", 1L, "degrees", 1L);
        // PHY00a 3.8: NO accuracy claim for tan (sin / cos plus a division and a pinned
        // pole sentinel), although tan rows carry a cr.  A tan gate is a separate follow-up.
        String noClaim = "tan";
        Map<String, Long> worst = new TreeMap<>();
        for (String group : GROUPS) {
            for (Row r : load(group)) {
                if (r.cr().equals("-") || r.op().equals(noClaim)) {
                    continue;
                }
                // Fail closed: any other op with a cr value must have a stated limit.
                assertTrue(limit.containsKey(r.op()),
                    "op " + r.op() + " has a cr value but no accuracy limit");
                long got = bitsOf(call(r.op(), r.x()));
                long steps = Math.abs(ordered(got) - ordered(Long.parseUnsignedLong(r.cr(), 16)));
                worst.merge(r.op(), steps, Math::max);
            }
        }
        for (Map.Entry<String, Long> e : limit.entrySet()) {
            long w = worst.getOrDefault(e.getKey(), 0L);
            assertTrue(w <= e.getValue(), e.getKey() + ": worst " + w + " steps > " + e.getValue());
        }
    }

    @Test
    void anUnknownOpFails() {
        assertThrows(org.opentest4j.AssertionFailedError.class, () -> call("sinh", "0000000000000000"));
    }

    // -------------------------------------------------------------------------
    // Contract points of PHY00a 3.3 asserted directly
    // -------------------------------------------------------------------------

    @Test
    void sinOfNegativeZeroIsNegativeZero() {
        assertEquals(0x8000000000000000L, bitsOf(Trig.sin(-0.0)));
        assertEquals(0L, bitsOf(Trig.sin(0.0)));
    }

    @Test
    void cosOfZeroIsOne() {
        assertEquals(1.0, Trig.cos(0.0));
        assertEquals(1.0, Trig.cos(-0.0));
    }

    @Test
    void tinyArgumentsAreReturnedUnchanged() {
        double tiny = fromHex("3e3fffffffffffff");
        assertEquals(bitsOf(tiny), bitsOf(Trig.sin(tiny)));
        assertEquals(bitsOf(Double.MIN_VALUE), bitsOf(Trig.sin(Double.MIN_VALUE)));
        assertEquals(1.0, Trig.cos(tiny));
    }

    @Test
    void outsideTheDomainIsNaN() {
        double[] xs = {
            Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY, 1e20,
            536870912.0 * (1.0 + 0x1p-52), Double.MAX_VALUE,
        };
        for (double x : xs) {
            assertTrue(Double.isNaN(Trig.sin(x)), "sin " + x);
            assertTrue(Double.isNaN(Trig.cos(x)), "cos " + x);
            assertTrue(Double.isNaN(Trig.tan(x)), "tan " + x);
        }
    }

    @Test
    void theLastInDomainValueIsFinite() {
        assertEquals(0x3FD4E67C0E2622DFL, bitsOf(Trig.sin(536870912.0)));
        assertEquals(0xBFEE3EDD2DFEEF90L, bitsOf(Trig.cos(536870912.0)));
    }

    @Test
    void radiansAndDegreesAreSingleMultiplies() {
        assertEquals(Trig.PI, Trig.radians(180.0));
        assertEquals(180.0, Trig.degrees(Trig.PI));
        assertEquals(Double.POSITIVE_INFINITY, Trig.degrees(1.7e308));
        assertTrue(Double.isNaN(Trig.radians(Double.NaN)));
        assertEquals(0x8000000000000000L, bitsOf(Trig.radians(-0.0)));
        assertEquals(0x8000000000000000L, bitsOf(Trig.degrees(-0.0)));
    }

    @Test
    void tanPoleSentinel() {
        assertEquals(1e308, Trig.tan(Trig.HALF_PI));
        assertEquals(-1e308, Trig.tan(-Trig.HALF_PI));
    }

    @Test
    void atan2OfNaNIsNaN() {
        double nan = Double.NaN;
        assertTrue(Double.isNaN(Trig.atan2(nan, 1.0)));
        assertTrue(Double.isNaN(Trig.atan2(1.0, nan)));
        assertTrue(Double.isNaN(Trig.atan2(-1.0, nan)));
        assertTrue(Double.isNaN(Trig.atan2(nan, nan)));
        assertTrue(Double.isNaN(Trig.atan2(nan, -1.0)));
        assertTrue(Double.isNaN(Trig.atan2(nan, 0.0)));
    }

    @Test
    void atan2PinnedConventions() {
        assertEquals(Trig.PI, Trig.atan2(-0.0, -1.0));
        assertEquals(0.0, Trig.atan2(0.0, 0.0));
        assertTrue(Double.isNaN(Trig.atan2(Double.POSITIVE_INFINITY, Double.POSITIVE_INFINITY)));
        assertEquals(Trig.HALF_PI, Trig.atan2(1e300, 1e-300));
    }

    @Test
    void atanKeepsTheSignOfZero() {
        assertEquals(0x8000000000000000L, bitsOf(Trig.atan(-0.0)));
    }

    @Test
    void negativeQuadrantArithmetic() {
        // k = -1 and k = -5 map to quadrant 3: "& 3" is the floor modulus.
        assertEquals(-Trig.sin(Trig.HALF_PI), Trig.sin(-Trig.HALF_PI));
        assertEquals(0.0, Trig.cos(-5 * Trig.HALF_PI), 1e-15);
        assertFalse(Double.isNaN(Trig.sin(-5 * Trig.HALF_PI)));
    }
}
