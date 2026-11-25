package com.codingadventures.logicgates;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.List;
import java.util.function.IntBinaryOperator;
import java.util.function.IntUnaryOperator;
import org.junit.jupiter.api.Test;

/**
 * Lane-local exhaustive validation of the 11 basic gates (LG02).
 *
 * <p>Note: long/double/boolean arguments do not compile (parameters are int);
 * char/byte/short widen to int and are rejected at runtime.
 */
class LogicGatesExhaustiveTest {

    private static final String[] NAMES2 =
        {"AND", "OR", "XOR", "NAND", "NOR", "XNOR", "nandAND", "nandOR", "nandXOR"};
    private static final IntBinaryOperator[] FN2 = {
        LogicGates::AND, LogicGates::OR, LogicGates::XOR, LogicGates::NAND,
        LogicGates::NOR, LogicGates::XNOR, LogicGates::nandAND,
        LogicGates::nandOR, LogicGates::nandXOR
    };
    // Literal truth tables, rows ordered (0,0) (0,1) (1,0) (1,1).
    private static final int[][] TABLE2 = {
        {0, 0, 0, 1}, // AND
        {0, 1, 1, 1}, // OR
        {0, 1, 1, 0}, // XOR
        {1, 1, 1, 0}, // NAND
        {1, 0, 0, 0}, // NOR
        {1, 0, 0, 1}, // XNOR
        {0, 0, 0, 1}, // nandAND
        {0, 1, 1, 1}, // nandOR
        {0, 1, 1, 0}, // nandXOR
    };

    private static final IntUnaryOperator[] FN1 = {LogicGates::NOT, LogicGates::nandNOT};
    private static final String[] NAMES1 = {"NOT", "nandNOT"};

    private static List<Integer> invalidValues() {
        List<Integer> v = new ArrayList<>();
        for (int i = -1024; i <= 1024; i++) {
            if (i != 0 && i != 1) v.add(i);
        }
        v.add(Integer.MIN_VALUE);
        v.add(Integer.MIN_VALUE + 1);
        v.add(Integer.MAX_VALUE - 1);
        v.add(Integer.MAX_VALUE);
        return v;
    }

    @Test
    void truthTables() {
        for (int g = 0; g < FN2.length; g++) {
            for (int row = 0; row < 4; row++) {
                int a = row >> 1, b = row & 1;
                assertEquals(TABLE2[g][row], FN2[g].applyAsInt(a, b), NAMES2[g] + "(" + a + "," + b + ")");
            }
        }
        for (int g = 0; g < 2; g++) {
            assertEquals(1, FN1[g].applyAsInt(0), NAMES1[g] + "(0)");
            assertEquals(0, FN1[g].applyAsInt(1), NAMES1[g] + "(1)");
        }
    }

    @Test
    void nandBuiltEqualsDirect() {
        for (int a = 0; a <= 1; a++) {
            assertEquals(LogicGates.NOT(a), LogicGates.nandNOT(a));
            for (int b = 0; b <= 1; b++) {
                assertEquals(LogicGates.AND(a, b), LogicGates.nandAND(a, b));
                assertEquals(LogicGates.OR(a, b), LogicGates.nandOR(a, b));
                assertEquals(LogicGates.XOR(a, b), LogicGates.nandXOR(a, b));
            }
        }
    }

    @Test
    void resultsAreExactlyZeroOrOne() {
        for (int a = 0; a <= 1; a++) {
            for (IntUnaryOperator f : FN1) {
                int r = f.applyAsInt(a);
                assertTrue(r == 0 || r == 1);
            }
            for (int b = 0; b <= 1; b++) {
                for (IntBinaryOperator f : FN2) {
                    int r = f.applyAsInt(a, b);
                    assertTrue(r == 0 || r == 1);
                }
            }
        }
    }

    private static void expectReject(String param, int value, org.junit.jupiter.api.function.Executable call) {
        InvalidBitException e = assertThrows(InvalidBitException.class, call);
        assertEquals(param, e.getParameter());
        assertEquals(value, e.getValue());
        assertInstanceOf(IllegalArgumentException.class, e);
    }

    @Test
    void oneInputGatesRejectEveryInvalidInt() {
        for (IntUnaryOperator f : FN1) {
            for (int v : invalidValues()) {
                expectReject("a", v, () -> f.applyAsInt(v));
            }
        }
    }

    @Test
    void twoInputGatesRejectEveryInvalidIntInEveryPosition() {
        for (IntBinaryOperator f : FN2) {
            for (int v : invalidValues()) {
                for (int ok = 0; ok <= 1; ok++) {
                    final int o = ok;
                    expectReject("a", v, () -> f.applyAsInt(v, o));
                    expectReject("b", v, () -> f.applyAsInt(o, v));
                }
                // Both invalid: the first invalid parameter (a) is reported.
                expectReject("a", v, () -> f.applyAsInt(v, 7));
            }
            // a valid, b invalid.
            expectReject("b", 7, () -> f.applyAsInt(1, 7));
        }
    }

    @Test
    void charByteShortWidenAndAreRejectedAtRuntime() {
        char c = 'a';
        expectReject("a", 97, () -> LogicGates.AND(c, 1));
        expectReject("b", 97, () -> LogicGates.nandOR(1, c));
        expectReject("a", 97, () -> LogicGates.nandNOT(c));
        byte by = 2;
        expectReject("a", 2, () -> LogicGates.NOT(by));
        byte neg = -1;
        expectReject("b", -1, () -> LogicGates.XOR(0, neg));
        short sh = 300;
        expectReject("a", 300, () -> LogicGates.NAND(sh, 0));
        // Valid widened values behave as plain ints.
        assertEquals(1, LogicGates.AND((byte) 1, (short) 1));
        assertEquals(1, LogicGates.NOT((char) 0));
    }

    @Test
    void exceptionMessageNamesParameterAndValue() {
        InvalidBitException e = assertThrows(InvalidBitException.class, () -> LogicGates.AND(1, 5));
        assertEquals("b must be 0 or 1, got: 5", e.getMessage());
    }

    @Test
    void multiInputStillValidates() {
        InvalidBitException e = assertThrows(InvalidBitException.class, () -> LogicGates.XOR_N(1, 0, 3));
        assertEquals("inputs[2]", e.getParameter());
        assertEquals(3, e.getValue());
    }

    // ---- N-ary gates: shape, snapshot, parameter names (LGE-JAVA-NARY) ----

    private interface NaryGate { int apply(int... inputs); }

    private static final String[] NARY_NAMES = {"AND_N", "OR_N", "XOR_N"};
    private static final NaryGate[] NARY = {LogicGates::AND_N, LogicGates::OR_N, LogicGates::XOR_N};

    @Test
    void nullArrayIsPackageErrorNotNpe() {
        for (int g = 0; g < 3; g++) {
            NaryGate gate = NARY[g];
            InvalidShapeException e = assertThrows(InvalidShapeException.class,
                () -> gate.apply((int[]) null), NARY_NAMES[g]);
            assertEquals("inputs", e.getParameter());
            assertInstanceOf(IllegalArgumentException.class, e);
        }
    }

    @Test
    void shortArraysAreShapeErrorsForAndOrOnly() {
        for (int g = 0; g < 2; g++) {
            NaryGate gate = NARY[g];
            assertThrows(InvalidShapeException.class, () -> gate.apply(), NARY_NAMES[g]);
            assertThrows(InvalidShapeException.class, () -> gate.apply(1), NARY_NAMES[g]);
        }
        assertEquals(0, LogicGates.XOR_N());
        assertEquals(1, LogicGates.XOR_N(1));
        assertEquals(0, LogicGates.XOR_N(0));
    }

    @Test
    void shapeErrorWinsOverBitError() {
        for (int g = 0; g < 2; g++) {
            NaryGate gate = NARY[g];
            assertThrows(InvalidShapeException.class, () -> gate.apply(new int[] {5}), NARY_NAMES[g]);
        }
        int[] bigBad = new int[LogicGates.MAX_INPUTS + 1];
        bigBad[0] = 7;
        for (int g = 0; g < 3; g++) {
            NaryGate gate = NARY[g];
            assertThrows(InvalidShapeException.class, () -> gate.apply(bigBad), NARY_NAMES[g]);
        }
        // XOR_N accepts a single element, so there the bit error is reported.
        assertThrows(InvalidBitException.class, () -> LogicGates.XOR_N(new int[] {5}));
    }

    @Test
    void oversizedArrayRejected() {
        int[] big = new int[LogicGates.MAX_INPUTS + 1];
        for (int g = 0; g < 3; g++) {
            NaryGate gate = NARY[g];
            assertThrows(InvalidShapeException.class, () -> gate.apply(big), NARY_NAMES[g]);
        }
        int[] max = new int[LogicGates.MAX_INPUTS];
        assertEquals(0, LogicGates.OR_N(max));
        assertEquals(0, LogicGates.XOR_N(max));
        assertEquals(0, LogicGates.AND_N(max));
    }

    @Test
    void badElementNamedInputsIndexInAllNaryGates() {
        for (int g = 0; g < 3; g++) {
            NaryGate gate = NARY[g];
            InvalidBitException e = assertThrows(InvalidBitException.class,
                () -> gate.apply(1, 0, 7, 9), NARY_NAMES[g]);
            assertEquals("inputs[2]", e.getParameter());
            assertEquals(7, e.getValue());
            InvalidBitException first = assertThrows(InvalidBitException.class,
                () -> gate.apply(-1, 1), NARY_NAMES[g]);
            assertEquals("inputs[0]", first.getParameter());
        }
    }

    /** Over {-1,0,1,2}, lengths 0..8: result is the right bit or an exception, never anything else. */
    @Test
    void naryNeverReturnsNonBitExhaustive() {
        int[] alphabet = {-1, 0, 1, 2};
        for (int g = 0; g < 3; g++) {
            final NaryGate gate = NARY[g];
            for (int len = 0; len <= 8; len++) {
                long total = 1L << (2 * len);
                for (long code = 0; code < total; code++) {
                    int[] in = new int[len];
                    boolean valid = true;
                    int ones = 0;
                    for (int i = 0; i < len; i++) {
                        in[i] = alphabet[(int) ((code >> (2 * i)) & 3)];
                        if (in[i] != 0 && in[i] != 1) valid = false;
                        ones += in[i] == 1 ? 1 : 0;
                    }
                    int[] before = in.clone();
                    boolean tooShort = g < 2 && len < 2;
                    if (!valid || tooShort) {
                        assertThrows(IllegalArgumentException.class, () -> gate.apply(in));
                    } else {
                        int expected = g == 0 ? (ones == len ? 1 : 0) : g == 1 ? (ones > 0 ? 1 : 0) : ones & 1;
                        assertEquals(expected, gate.apply(in));
                    }
                    assertArrayEquals(before, in, "input array must not be mutated");
                }
            }
        }
    }
}
