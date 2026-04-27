package com.codingadventures.activationfunctions

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ActivationFunctionsTest {

    private val eps = 1e-10

    // ========================================================================
    // Linear Tests
    // ========================================================================

    @Test fun `linear negative`() = assertEquals(-3.0, ActivationFunctions.linear(-3.0), eps)
    @Test fun `linear zero`() = assertEquals(0.0, ActivationFunctions.linear(0.0), eps)
    @Test fun `linear positive`() = assertEquals(5.0, ActivationFunctions.linear(5.0), eps)
    @Test fun `linear deriv everywhere`() {
        assertEquals(1.0, ActivationFunctions.linearDerivative(-3.0), eps)
        assertEquals(1.0, ActivationFunctions.linearDerivative(0.0), eps)
        assertEquals(1.0, ActivationFunctions.linearDerivative(5.0), eps)
    }

    // ========================================================================
    // Sigmoid Tests
    // ========================================================================

    @Test fun `sigmoid at zero`() = assertEquals(0.5, ActivationFunctions.sigmoid(0.0), eps)
    @Test fun `sigmoid positive`() = assertEquals(0.7310585786300049, ActivationFunctions.sigmoid(1.0), eps)
    @Test fun `sigmoid negative`() = assertEquals(0.2689414213699951, ActivationFunctions.sigmoid(-1.0), eps)
    @Test fun `sigmoid large positive`() = assertEquals(0.9999546021312976, ActivationFunctions.sigmoid(10.0), 1e-8)
    @Test fun `sigmoid overflow negative`() = assertEquals(0.0, ActivationFunctions.sigmoid(-710.0))
    @Test fun `sigmoid overflow positive`() = assertEquals(1.0, ActivationFunctions.sigmoid(710.0))

    @Test
    fun `sigmoid symmetry`() {
        for (x in doubleArrayOf(0.5, 1.0, 2.0, 5.0, 10.0)) {
            assertEquals(
                ActivationFunctions.sigmoid(-x),
                1.0 - ActivationFunctions.sigmoid(x),
                eps, "Symmetry failed at x=$x")
        }
    }

    @Test
    fun `sigmoid range`() {
        for (x in doubleArrayOf(-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0)) {
            val s = ActivationFunctions.sigmoid(x)
            assertTrue(s in 0.0..1.0, "Out of range at x=$x")
        }
    }

    // ========================================================================
    // Sigmoid Derivative Tests
    // ========================================================================

    @Test fun `sigmoid deriv at zero`() = assertEquals(0.25, ActivationFunctions.sigmoidDerivative(0.0), eps)
    @Test fun `sigmoid deriv at one`() = assertEquals(0.19661193324148185, ActivationFunctions.sigmoidDerivative(1.0), eps)
    @Test fun `sigmoid deriv saturated`() = assertEquals(0.0000453978, ActivationFunctions.sigmoidDerivative(10.0), 1e-8)

    @Test
    fun `sigmoid deriv non-negative`() {
        for (x in doubleArrayOf(-10.0, -1.0, 0.0, 1.0, 10.0)) {
            assertTrue(ActivationFunctions.sigmoidDerivative(x) >= 0.0)
        }
    }

    // ========================================================================
    // ReLU Tests
    // ========================================================================

    @Test fun `relu positive`() = assertEquals(5.0, ActivationFunctions.relu(5.0))
    @Test fun `relu negative`() = assertEquals(0.0, ActivationFunctions.relu(-3.0))
    @Test fun `relu zero`() = assertEquals(0.0, ActivationFunctions.relu(0.0))

    @Test
    fun `relu idempotence`() {
        for (x in doubleArrayOf(-5.0, -1.0, 0.0, 1.0, 5.0)) {
            val r = ActivationFunctions.relu(x)
            assertEquals(r, ActivationFunctions.relu(r), eps)
        }
    }

    // ========================================================================
    // ReLU Derivative Tests
    // ========================================================================

    @Test fun `relu deriv positive`() = assertEquals(1.0, ActivationFunctions.reluDerivative(5.0))
    @Test fun `relu deriv negative`() = assertEquals(0.0, ActivationFunctions.reluDerivative(-3.0))
    @Test fun `relu deriv zero`() = assertEquals(0.0, ActivationFunctions.reluDerivative(0.0))

    // ========================================================================
    // Leaky ReLU Tests
    // ========================================================================

    @Test fun `leaky relu positive`() = assertEquals(5.0, ActivationFunctions.leakyRelu(5.0), eps)
    @Test fun `leaky relu negative`() = assertEquals(-0.03, ActivationFunctions.leakyRelu(-3.0), eps)
    @Test fun `leaky relu zero`() = assertEquals(0.0, ActivationFunctions.leakyRelu(0.0), eps)
    @Test fun `leaky relu deriv positive`() = assertEquals(1.0, ActivationFunctions.leakyReluDerivative(5.0), eps)
    @Test fun `leaky relu deriv negative`() = assertEquals(0.01, ActivationFunctions.leakyReluDerivative(-3.0), eps)
    @Test fun `leaky relu deriv zero`() = assertEquals(0.01, ActivationFunctions.leakyReluDerivative(0.0), eps)

    // ========================================================================
    // Tanh Tests
    // ========================================================================

    @Test fun `tanh at zero`() = assertEquals(0.0, ActivationFunctions.tanh(0.0), eps)
    @Test fun `tanh positive`() = assertEquals(0.7615941559557649, ActivationFunctions.tanh(1.0), eps)
    @Test fun `tanh negative`() = assertEquals(-0.7615941559557649, ActivationFunctions.tanh(-1.0), eps)

    @Test
    fun `tanh odd symmetry`() {
        for (x in doubleArrayOf(0.5, 1.0, 2.0, 5.0)) {
            assertEquals(
                ActivationFunctions.tanh(-x),
                -ActivationFunctions.tanh(x),
                eps, "Odd symmetry failed at x=$x")
        }
    }

    @Test
    fun `tanh range`() {
        for (x in doubleArrayOf(-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0)) {
            val t = ActivationFunctions.tanh(x)
            assertTrue(t in -1.0..1.0, "Out of range at x=$x")
        }
    }

    // ========================================================================
    // Tanh Derivative Tests
    // ========================================================================

    @Test fun `tanh deriv at zero`() = assertEquals(1.0, ActivationFunctions.tanhDerivative(0.0), eps)
    @Test fun `tanh deriv at one`() = assertEquals(0.4199743416140261, ActivationFunctions.tanhDerivative(1.0), eps)

    // ========================================================================
    // Softplus Tests
    // ========================================================================

    @Test fun `softplus at zero`() = assertEquals(0.6931471805599453, ActivationFunctions.softplus(0.0), eps)
    @Test fun `softplus positive`() = assertEquals(1.3132616875182228, ActivationFunctions.softplus(1.0), eps)
    @Test fun `softplus negative`() = assertEquals(0.31326168751822286, ActivationFunctions.softplus(-1.0), eps)
    @Test fun `softplus large positive stable`() = assertTrue(ActivationFunctions.softplus(1000.0) > 999.0)
    @Test fun `softplus derivative equals sigmoid`() {
        assertEquals(0.5, ActivationFunctions.softplusDerivative(0.0), eps)
        assertEquals(ActivationFunctions.sigmoid(1.0), ActivationFunctions.softplusDerivative(1.0), eps)
        assertEquals(ActivationFunctions.sigmoid(-1.0), ActivationFunctions.softplusDerivative(-1.0), eps)
    }

    @Test
    fun `tanh deriv non-negative`() {
        for (x in doubleArrayOf(-10.0, -1.0, 0.0, 1.0, 10.0)) {
            assertTrue(ActivationFunctions.tanhDerivative(x) >= 0.0)
        }
    }
}
