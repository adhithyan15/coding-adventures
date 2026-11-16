package com.codingadventures.gradientdescent;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import java.util.SplittableRandom;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

class GradientDescentTest {

    private static final double TOLERANCE = 1e-12;

    private static InvalidGradientDescentInputException expectError(
            GradientDescentErrorCode expected, double[] weights, double[] gradients, double lr) {
        InvalidGradientDescentInputException e = assertThrows(
                InvalidGradientDescentInputException.class,
                () -> GradientDescent.sgd(weights, gradients, lr));
        assertSame(expected, e.getCode());
        assertEquals(expected.getErrorId(), e.getErrorId());
        assertEquals(expected.getMessage(), e.getMessage());
        return e;
    }

    private static double[] filled(int length, double value) {
        double[] values = new double[length];
        Arrays.fill(values, value);
        return values;
    }

    // ------------------------------------------------------------------ basics

    @Test
    void specParityVector() {
        double[] w = {1.0, -0.5, 2.0};
        double[] g = {0.1, -0.2, 0.0};
        double lr = 0.1;
        double[] result = GradientDescent.sgd(w, g, lr);
        assertArrayEquals(new double[] {0.99, -0.48, 2.0}, result, TOLERANCE);
        for (int i = 0; i < w.length; i++) {
            double step = lr * g[i];
            double reference = w[i] - step;
            assertEquals(Double.doubleToRawLongBits(reference),
                    Double.doubleToRawLongBits(result[i]),
                    "result must be bit-exact with the unfused reference");
        }
    }

    @Test
    void dyadicWitnessIsExact() {
        double[] result = GradientDescent.sgd(
                new double[] {1.0, -0.5, 2.0}, new double[] {0.5, -0.25, 0.0}, 0.5);
        assertArrayEquals(new double[] {0.75, -0.375, 2.0}, result, 0.0);
    }

    @Test
    void singleElementAndZeroLearningRate() {
        assertArrayEquals(new double[] {3.0}, GradientDescent.sgd(
                new double[] {3.0}, new double[] {100.0}, 0.0), 0.0);
    }

    @Test
    void negativeLearningRateIsGradientAscent() {
        assertArrayEquals(new double[] {2.0}, GradientDescent.sgd(
                new double[] {1.0}, new double[] {1.0}, -1.0), 0.0);
    }

    @Test
    void returnsFreshArrayAndDoesNotMutateInputs() {
        double[] w = {1.0, 2.0};
        double[] g = {0.5, 0.5};
        double[] result = GradientDescent.sgd(w, g, 1.0);
        assertNotSame(w, result);
        assertNotSame(g, result);
        assertArrayEquals(new double[] {1.0, 2.0}, w, 0.0);
        assertArrayEquals(new double[] {0.5, 0.5}, g, 0.0);
        assertArrayEquals(new double[] {0.5, 1.5}, result, 0.0);
    }

    @Test
    void subnormalAndSignedZeroInputsAreFinite() {
        double[] result = GradientDescent.sgd(
                new double[] {Double.MIN_VALUE, -0.0}, new double[] {-0.0, Double.MIN_VALUE}, 1.0);
        assertEquals(Double.MIN_VALUE, result[0], 0.0);
        assertEquals(-Double.MIN_VALUE, result[1], 0.0);
    }

    // --------------------------------------------------------- error plumbing

    @Test
    void errorCodesCarryContractIdsAndMessages() {
        String[][] table = {
            {"NULL_INPUT", "null-input", "weights and gradients must not be null"},
            {"LENGTH_MISMATCH", "length-mismatch", "weights and gradients must have the same length"},
            {"EMPTY_INPUT", "empty-input", "weights and gradients must not be empty"},
            {"INPUT_TOO_LARGE", "input-too-large", "input vectors exceed the maximum supported length"},
            {"NON_FINITE_LEARNING_RATE", "non-finite-learning-rate", "learning rate must be a finite number"},
            {"NON_FINITE_INPUT", "non-finite-input", "weights and gradients must be finite numbers"},
            {"NUMERIC_OVERFLOW", "numeric-overflow", "updated weight is not a finite number"},
        };
        assertEquals(table.length, GradientDescentErrorCode.values().length);
        for (String[] row : table) {
            GradientDescentErrorCode code = GradientDescentErrorCode.valueOf(row[0]);
            assertEquals(row[1], code.getErrorId());
            assertEquals(row[2], code.getMessage());
            InvalidGradientDescentInputException e = new InvalidGradientDescentInputException(code);
            assertSame(code, e.getCode());
            assertEquals(row[1], e.getErrorId());
            assertEquals(row[2], e.getMessage());
            assertTrue(e instanceof IllegalArgumentException);
        }
    }

    @Test
    void exceptionRejectsNullCode() {
        assertThrows(NullPointerException.class, () -> new InvalidGradientDescentInputException(null));
    }

    @Test
    void constantsAndPlaceholders() {
        assertEquals("0.1.0", GradientDescentConstants.VERSION);
        assertEquals(1_048_576, GradientDescentConstants.MAX_VECTOR_LENGTH);
        assertEquals("<redacted>", GradientDescentMessages.REDACTED_PLACEHOLDER);
        assertEquals("<invalid>", GradientDescentMessages.INVALID_PLACEHOLDER);
    }

    @Test
    void eachRuleFiresInIsolation() {
        expectError(GradientDescentErrorCode.NULL_INPUT, null, new double[] {1.0}, 0.1);
        expectError(GradientDescentErrorCode.NULL_INPUT, new double[] {1.0}, null, 0.1);
        expectError(GradientDescentErrorCode.NULL_INPUT, null, null, 0.1);
        expectError(GradientDescentErrorCode.LENGTH_MISMATCH,
                new double[] {1.0, 2.0}, new double[] {1.0}, 0.1);
        expectError(GradientDescentErrorCode.EMPTY_INPUT, new double[0], new double[0], 0.1);
        expectError(GradientDescentErrorCode.NON_FINITE_LEARNING_RATE,
                new double[] {1.0}, new double[] {1.0}, Double.NaN);
        expectError(GradientDescentErrorCode.NON_FINITE_INPUT,
                new double[] {Double.NaN}, new double[] {1.0}, 0.1);
        expectError(GradientDescentErrorCode.NUMERIC_OVERFLOW,
                new double[] {1e308}, new double[] {-1e308}, 1.0);
    }

    // ============================================================ 5-class showcase

    /** Class 1: validation precedence, one test per adjacent pair of rules. */
    @Nested
    class ValidationPrecedence {
        @Test
        void nullBeatsMismatch() {
            expectError(GradientDescentErrorCode.NULL_INPUT, null, new double[] {1.0, 2.0}, Double.NaN);
            expectError(GradientDescentErrorCode.NULL_INPUT, new double[0], null, Double.NaN);
        }

        @Test
        void mismatchBeatsEmpty() {
            expectError(GradientDescentErrorCode.LENGTH_MISMATCH, new double[0], new double[] {1.0}, 0.1);
            expectError(GradientDescentErrorCode.LENGTH_MISMATCH, new double[] {1.0}, new double[0], 0.1);
        }

        @Test
        void emptyBeatsNonFiniteLearningRate() {
            expectError(GradientDescentErrorCode.EMPTY_INPUT, new double[0], new double[0], Double.NaN);
        }

        @Test
        void mismatchBeatsTooLarge() {
            expectError(GradientDescentErrorCode.LENGTH_MISMATCH,
                    new double[GradientDescentConstants.MAX_VECTOR_LENGTH + 1], new double[1], 0.1);
        }

        @Test
        void tooLargeBeatsNonFiniteLearningRate() {
            int n = GradientDescentConstants.MAX_VECTOR_LENGTH + 1;
            expectError(GradientDescentErrorCode.INPUT_TOO_LARGE, new double[n], new double[n], Double.NaN);
        }

        @Test
        void tooLargeBeatsNonFiniteInput() {
            int n = GradientDescentConstants.MAX_VECTOR_LENGTH + 1;
            expectError(GradientDescentErrorCode.INPUT_TOO_LARGE,
                    filled(n, Double.NaN), filled(n, Double.POSITIVE_INFINITY), 0.1);
        }

        @Test
        void nonFiniteLearningRateBeatsNonFiniteInput() {
            expectError(GradientDescentErrorCode.NON_FINITE_LEARNING_RATE,
                    new double[] {Double.NaN}, new double[] {Double.NaN}, Double.NaN);
        }

        @Test
        void nonFiniteInputBeatsOverflowEvenWhenOverflowComesFirst() {
            // Element 0 would overflow; element 1 is NaN. The full pre-pass must
            // report the NaN, proving no arithmetic ran before validation finished.
            expectError(GradientDescentErrorCode.NON_FINITE_INPUT,
                    new double[] {1e308, 1.0}, new double[] {-1e308, Double.NaN}, 1.0);
        }
    }

    /** Class 2: CWE-400 resource ceiling. */
    @Nested
    class ResourceCeiling {
        @Test
        void oneOverTheLimitIsRejected() {
            int n = GradientDescentConstants.MAX_VECTOR_LENGTH + 1;
            assertEquals(1_048_577, n);
            expectError(GradientDescentErrorCode.INPUT_TOO_LARGE, new double[n], new double[n], 0.1);
        }

        @Test
        void exactlyTheLimitIsAccepted() {
            int n = GradientDescentConstants.MAX_VECTOR_LENGTH;
            assertEquals(1_048_576, n);
            double[] result = GradientDescent.sgd(filled(n, 1.0), filled(n, 0.5), 0.5);
            assertEquals(n, result.length);
            assertEquals(0.75, result[0], 0.0);
            assertEquals(0.75, result[n - 1], 0.0);
        }
    }

    /** Class 3: CWE-682 numeric poisoning and overflow refusal. */
    @Nested
    class NumericPoisoning {
        private final double[] specials = {
            Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY
        };

        @Test
        void nonFiniteWeightsRejected() {
            for (double s : specials) {
                expectError(GradientDescentErrorCode.NON_FINITE_INPUT,
                        new double[] {1.0, s}, new double[] {0.1, 0.1}, 0.1);
            }
        }

        @Test
        void nonFiniteGradientsRejected() {
            for (double s : specials) {
                expectError(GradientDescentErrorCode.NON_FINITE_INPUT,
                        new double[] {1.0, 1.0}, new double[] {0.1, s}, 0.1);
            }
        }

        @Test
        void nonFiniteLearningRateRejected() {
            for (double s : specials) {
                expectError(GradientDescentErrorCode.NON_FINITE_LEARNING_RATE,
                        new double[] {1.0}, new double[] {0.1}, s);
            }
        }

        @Test
        void additionOverflowRefused() {
            // 1e308 - (-1.0 * 1e308) = 2e308 = +Inf
            expectError(GradientDescentErrorCode.NUMERIC_OVERFLOW,
                    new double[] {0.0, 1e308}, new double[] {0.0, -1e308}, 1.0);
        }

        @Test
        void negativeOverflowRefused() {
            expectError(GradientDescentErrorCode.NUMERIC_OVERFLOW,
                    new double[] {-1e308}, new double[] {1e308}, 1.0);
        }

        @Test
        void stepOverflowRefused() {
            // lr * g alone overflows even though w, g and lr are all finite.
            expectError(GradientDescentErrorCode.NUMERIC_OVERFLOW,
                    new double[] {0.0}, new double[] {1e308}, 1e10);
        }
    }

    /** Class 4: alias safety and purity. */
    @Nested
    class AliasSafety {
        @Test
        void sameArrayForWeightsAndGradients() {
            double[] x = {1.0, -2.0, 4.0};
            double[] result = GradientDescent.sgd(x, x, 0.5);
            assertArrayEquals(new double[] {0.5, -1.0, 2.0}, result, 0.0);
            assertArrayEquals(new double[] {1.0, -2.0, 4.0}, x, 0.0);
            assertNotSame(x, result);
        }

        @Test
        void outputAndInputsAreIndependentAfterReturn() {
            double[] w = {1.0, 2.0};
            double[] g = {1.0, 1.0};
            double[] result = GradientDescent.sgd(w, g, 0.5);
            result[0] = 99.0;
            assertEquals(1.0, w[0], 0.0);
            assertEquals(1.0, g[0], 0.0);
            w[1] = -99.0;
            g[1] = -99.0;
            assertEquals(1.5, result[1], 0.0);
        }
    }

    /** Class 5: CWE-209 payload-blind errors. */
    @Nested
    class PayloadBlind {
        private static final double SENTINEL_WEIGHT = 13371337.0;
        private static final double SENTINEL_LR = 0.4242;
        private static final int SENTINEL_LENGTH = 7;

        private void assertBlind(InvalidGradientDescentInputException e) {
            String[] forbidden = {
                String.valueOf(SENTINEL_WEIGHT), "13371337", String.valueOf(SENTINEL_LR), "4242",
                String.valueOf(SENTINEL_LENGTH), String.valueOf(SENTINEL_LENGTH - 1),
                "NaN", "Infinity",
            };
            String[] surfaces = {e.getMessage(), e.getLocalizedMessage(), e.toString(), e.getErrorId()};
            for (String surface : surfaces) {
                for (String f : forbidden) {
                    assertFalse(surface.contains(f), "error text leaked payload: " + f);
                }
            }
        }

        private double[] sentinelWeights() {
            double[] w = filled(SENTINEL_LENGTH, 1.0);
            w[3] = SENTINEL_WEIGHT;
            return w;
        }

        @Test
        void mismatchIsBlind() {
            assertBlind(expectError(GradientDescentErrorCode.LENGTH_MISMATCH,
                    sentinelWeights(), filled(SENTINEL_LENGTH - 1, 0.1), SENTINEL_LR));
        }

        @Test
        void nonFiniteLearningRateIsBlind() {
            assertBlind(expectError(GradientDescentErrorCode.NON_FINITE_LEARNING_RATE,
                    sentinelWeights(), filled(SENTINEL_LENGTH, 0.1), Double.NaN));
        }

        @Test
        void nonFiniteInputIsBlind() {
            double[] g = filled(SENTINEL_LENGTH, 0.1);
            g[SENTINEL_LENGTH - 1] = Double.POSITIVE_INFINITY;
            assertBlind(expectError(GradientDescentErrorCode.NON_FINITE_INPUT,
                    sentinelWeights(), g, SENTINEL_LR));
        }

        @Test
        void overflowIsBlind() {
            double[] w = sentinelWeights();
            double[] g = filled(SENTINEL_LENGTH, 0.1);
            w[SENTINEL_LENGTH - 1] = Double.MAX_VALUE;
            g[SENTINEL_LENGTH - 1] = -Double.MAX_VALUE;
            assertBlind(expectError(GradientDescentErrorCode.NUMERIC_OVERFLOW, w, g, SENTINEL_LR));
        }

        @Test
        void allMessagesAreStaticAndDigitFree() {
            for (GradientDescentErrorCode code : GradientDescentErrorCode.values()) {
                assertFalse(code.getMessage().matches(".*\\d.*"), code.name());
            }
        }
    }

    // ===================================================== property and fuzz

    @Test
    void dyadicMultiStepLinearity() {
        // Small dyadic values (k / 2^m) keep every intermediate exactly
        // representable, so two steps of size a and b must equal one step of
        // size (a + b) bit for bit.
        SplittableRandom rng = new SplittableRandom(0x4D4C3032L);
        for (int trial = 0; trial < 1_000; trial++) {
            int n = 1 + rng.nextInt(8);
            double[] w = new double[n];
            double[] g = new double[n];
            for (int i = 0; i < n; i++) {
                w[i] = (rng.nextInt(2049) - 1024) / 256.0;
                g[i] = (rng.nextInt(2049) - 1024) / 256.0;
            }
            double a = (1 + rng.nextInt(16)) / 16.0;
            double b = (1 + rng.nextInt(16)) / 16.0;
            double[] twoSteps = GradientDescent.sgd(GradientDescent.sgd(w, g, a), g, b);
            double[] oneStep = GradientDescent.sgd(w, g, a + b);
            for (int i = 0; i < n; i++) {
                assertEquals(Double.doubleToRawLongBits(oneStep[i]),
                        Double.doubleToRawLongBits(twoSteps[i]));
            }
        }
    }

    @Test
    void adversarialMutationFuzz() {
        double[] pool = {
            Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY, 0.0, -0.0,
            Double.MAX_VALUE, -Double.MAX_VALUE, 1e308, -1e308, Double.MIN_VALUE,
            -Double.MIN_VALUE, Double.MIN_NORMAL, 1.0, -1.0, 0.5, 1e-300,
        };
        Set<String> knownIds = new HashSet<>();
        for (GradientDescentErrorCode code : GradientDescentErrorCode.values()) {
            knownIds.add(code.getErrorId());
        }
        SplittableRandom rng = new SplittableRandom(20261115L);
        int successes = 0;
        int failures = 0;
        for (int iter = 0; iter < 10_000; iter++) {
            double[] w = randomVector(rng, pool, rng.nextInt(9));
            double[] g = rng.nextInt(4) == 0 ? randomVector(rng, pool, rng.nextInt(9))
                    : randomVector(rng, pool, w.length);
            if (rng.nextInt(50) == 0) {
                w = null;
            }
            double lr = rng.nextBoolean() ? pool[rng.nextInt(pool.length)] : rng.nextDouble() * 2.0 - 1.0;
            try {
                double[] result = GradientDescent.sgd(w, g, lr);
                assertEquals(w.length, result.length);
                for (double v : result) {
                    assertTrue(Double.isFinite(v));
                }
                successes++;
            } catch (InvalidGradientDescentInputException e) {
                assertTrue(knownIds.contains(e.getErrorId()));
                failures++;
            } catch (RuntimeException e) {
                fail("unexpected exception type: " + e.getClass().getName());
            }
        }
        assertTrue(successes > 0);
        assertTrue(failures > 0);
    }

    private static double[] randomVector(SplittableRandom rng, double[] pool, int length) {
        double[] v = new double[length];
        for (int i = 0; i < length; i++) {
            v[i] = rng.nextInt(3) == 0 ? pool[rng.nextInt(pool.length)] : rng.nextDouble() * 10.0 - 5.0;
        }
        return v;
    }
}
