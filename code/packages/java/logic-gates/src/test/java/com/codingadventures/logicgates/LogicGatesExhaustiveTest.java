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
        assertEquals("bit[2]", e.getParameter());
        assertEquals(3, e.getValue());
    }
}
