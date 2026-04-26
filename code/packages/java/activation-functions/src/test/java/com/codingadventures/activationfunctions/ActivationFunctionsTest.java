package com.codingadventures.activationfunctions;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class ActivationFunctionsTest {

    private static final double EPS = 1e-10;

    // ========================================================================
    // Linear Tests
    // ========================================================================

    @Test void linearNegative() { assertEquals(-3.0, ActivationFunctions.linear(-3.0), EPS); }
    @Test void linearZero() { assertEquals(0.0, ActivationFunctions.linear(0.0), EPS); }
    @Test void linearPositive() { assertEquals(5.0, ActivationFunctions.linear(5.0), EPS); }
    @Test void linearDerivEverywhere() {
        assertEquals(1.0, ActivationFunctions.linearDerivative(-3.0), EPS);
        assertEquals(1.0, ActivationFunctions.linearDerivative(0.0), EPS);
        assertEquals(1.0, ActivationFunctions.linearDerivative(5.0), EPS);
    }

    // ========================================================================
    // Sigmoid Tests
    // ========================================================================

    @Test void sigmoidAtZero() { assertEquals(0.5, ActivationFunctions.sigmoid(0.0), EPS); }
    @Test void sigmoidPositive() { assertEquals(0.7310585786300049, ActivationFunctions.sigmoid(1.0), EPS); }
    @Test void sigmoidNegative() { assertEquals(0.2689414213699951, ActivationFunctions.sigmoid(-1.0), EPS); }
    @Test void sigmoidLargePositive() { assertEquals(0.9999546021312976, ActivationFunctions.sigmoid(10.0), 1e-8); }
    @Test void sigmoidOverflowNeg() { assertEquals(0.0, ActivationFunctions.sigmoid(-710.0)); }
    @Test void sigmoidOverflowPos() { assertEquals(1.0, ActivationFunctions.sigmoid(710.0)); }

    @Test
    void sigmoidSymmetry() {
        double[] values = {0.5, 1.0, 2.0, 5.0, 10.0};
        for (double x : values) {
            assertEquals(
                ActivationFunctions.sigmoid(-x),
                1.0 - ActivationFunctions.sigmoid(x),
                EPS, "Symmetry failed at x=" + x);
        }
    }

    @Test
    void sigmoidRange() {
        double[] values = {-100, -10, -1, 0, 1, 10, 100};
        for (double x : values) {
            double s = ActivationFunctions.sigmoid(x);
            assertTrue(s >= 0.0 && s <= 1.0, "Out of range at x=" + x);
        }
    }

    // ========================================================================
    // Sigmoid Derivative Tests
    // ========================================================================

    @Test void sigmoidDerivAtZero() { assertEquals(0.25, ActivationFunctions.sigmoidDerivative(0.0), EPS); }
    @Test void sigmoidDerivAtOne() { assertEquals(0.19661193324148185, ActivationFunctions.sigmoidDerivative(1.0), EPS); }
    @Test void sigmoidDerivSaturated() { assertEquals(0.0000453978, ActivationFunctions.sigmoidDerivative(10.0), 1e-8); }

    @Test
    void sigmoidDerivNonNegative() {
        double[] values = {-10, -1, 0, 1, 10};
        for (double x : values) {
            assertTrue(ActivationFunctions.sigmoidDerivative(x) >= 0.0);
        }
    }

    // ========================================================================
    // ReLU Tests
    // ========================================================================

    @Test void reluPositive() { assertEquals(5.0, ActivationFunctions.relu(5.0)); }
    @Test void reluNegative() { assertEquals(0.0, ActivationFunctions.relu(-3.0)); }
    @Test void reluZero() { assertEquals(0.0, ActivationFunctions.relu(0.0)); }

    @Test
    void reluIdempotence() {
        double[] values = {-5, -1, 0, 1, 5};
        for (double x : values) {
            double r = ActivationFunctions.relu(x);
            assertEquals(r, ActivationFunctions.relu(r), EPS);
        }
    }

    // ========================================================================
    // ReLU Derivative Tests
    // ========================================================================

    @Test void reluDerivPositive() { assertEquals(1.0, ActivationFunctions.reluDerivative(5.0)); }
    @Test void reluDerivNegative() { assertEquals(0.0, ActivationFunctions.reluDerivative(-3.0)); }
    @Test void reluDerivZero() { assertEquals(0.0, ActivationFunctions.reluDerivative(0.0)); }

    // ========================================================================
    // Leaky ReLU Tests
    // ========================================================================

    @Test void leakyReluPositive() { assertEquals(5.0, ActivationFunctions.leakyRelu(5.0), EPS); }
    @Test void leakyReluNegative() { assertEquals(-0.03, ActivationFunctions.leakyRelu(-3.0), EPS); }
    @Test void leakyReluZero() { assertEquals(0.0, ActivationFunctions.leakyRelu(0.0), EPS); }
    @Test void leakyReluDerivPositive() { assertEquals(1.0, ActivationFunctions.leakyReluDerivative(5.0), EPS); }
    @Test void leakyReluDerivNegative() { assertEquals(0.01, ActivationFunctions.leakyReluDerivative(-3.0), EPS); }
    @Test void leakyReluDerivZero() { assertEquals(0.01, ActivationFunctions.leakyReluDerivative(0.0), EPS); }

    // ========================================================================
    // Tanh Tests
    // ========================================================================

    @Test void tanhAtZero() { assertEquals(0.0, ActivationFunctions.tanh(0.0), EPS); }
    @Test void tanhPositive() { assertEquals(0.7615941559557649, ActivationFunctions.tanh(1.0), EPS); }
    @Test void tanhNegative() { assertEquals(-0.7615941559557649, ActivationFunctions.tanh(-1.0), EPS); }

    @Test
    void tanhOddSymmetry() {
        double[] values = {0.5, 1.0, 2.0, 5.0};
        for (double x : values) {
            assertEquals(
                ActivationFunctions.tanh(-x),
                -ActivationFunctions.tanh(x),
                EPS, "Odd symmetry failed at x=" + x);
        }
    }

    @Test
    void tanhRange() {
        double[] values = {-100, -10, -1, 0, 1, 10, 100};
        for (double x : values) {
            double t = ActivationFunctions.tanh(x);
            assertTrue(t >= -1.0 && t <= 1.0, "Out of range at x=" + x);
        }
    }

    // ========================================================================
    // Tanh Derivative Tests
    // ========================================================================

    @Test void tanhDerivAtZero() { assertEquals(1.0, ActivationFunctions.tanhDerivative(0.0), EPS); }
    @Test void tanhDerivAtOne() { assertEquals(0.4199743416140261, ActivationFunctions.tanhDerivative(1.0), EPS); }

    // ========================================================================
    // Softplus Tests
    // ========================================================================

    @Test void softplusAtZero() { assertEquals(0.6931471805599453, ActivationFunctions.softplus(0.0), EPS); }
    @Test void softplusPositive() { assertEquals(1.3132616875182228, ActivationFunctions.softplus(1.0), EPS); }
    @Test void softplusNegative() { assertEquals(0.31326168751822286, ActivationFunctions.softplus(-1.0), EPS); }
    @Test void softplusLargePositiveStable() { assertTrue(ActivationFunctions.softplus(1000.0) > 999.0); }
    @Test void softplusDerivativeEqualsSigmoid() {
        assertEquals(0.5, ActivationFunctions.softplusDerivative(0.0), EPS);
        assertEquals(ActivationFunctions.sigmoid(1.0), ActivationFunctions.softplusDerivative(1.0), EPS);
        assertEquals(ActivationFunctions.sigmoid(-1.0), ActivationFunctions.softplusDerivative(-1.0), EPS);
    }

    @Test
    void tanhDerivNonNegative() {
        double[] values = {-10, -1, 0, 1, 10};
        for (double x : values) {
            assertTrue(ActivationFunctions.tanhDerivative(x) >= 0.0);
        }
    }
}
