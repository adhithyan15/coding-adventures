package com.codingadventures.gradientdescent

import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotSame
import org.junit.jupiter.api.Assertions.assertSame
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.SplittableRandom

class GradientDescentTest {

    companion object {
        private const val TOLERANCE = 1e-12

        private fun expectError(
            expected: GradientDescentErrorCode,
            weights: DoubleArray?,
            gradients: DoubleArray?,
            lr: Double
        ): InvalidGradientDescentInputException {
            val e = assertThrows(InvalidGradientDescentInputException::class.java) {
                GradientDescent.sgd(weights, gradients, lr)
            }
            assertSame(expected, e.code)
            assertEquals(expected.errorId, e.errorId)
            assertEquals(expected.message, e.message)
            return e
        }

        private fun filled(length: Int, value: Double): DoubleArray {
            val values = DoubleArray(length)
            values.fill(value)
            return values
        }
    }

    @Test
    fun specParityVector() {
        val w = doubleArrayOf(1.0, -0.5, 2.0)
        val g = doubleArrayOf(0.1, -0.2, 0.0)
        val lr = 0.1
        val result = GradientDescent.sgd(w, g, lr)
        assertArrayEquals(doubleArrayOf(0.99, -0.48, 2.0), result, TOLERANCE)
        for (i in w.indices) {
            val step = lr * g[i]
            val reference = w[i] - step
            assertEquals(
                java.lang.Double.doubleToRawLongBits(reference),
                java.lang.Double.doubleToRawLongBits(result[i]),
                "result must be bit-exact with the unfused reference"
            )
        }
    }

    @Test
    fun dyadicWitnessIsExact() {
        val result = GradientDescent.sgd(
            doubleArrayOf(1.0, -0.5, 2.0),
            doubleArrayOf(0.5, -0.25, 0.0),
            0.5
        )
        assertArrayEquals(doubleArrayOf(0.75, -0.375, 2.0), result, 0.0)
    }

    @Test
    fun singleElementAndZeroLearningRate() {
        assertArrayEquals(
            doubleArrayOf(3.0),
            GradientDescent.sgd(doubleArrayOf(3.0), doubleArrayOf(100.0), 0.0),
            0.0
        )
    }

    @Test
    fun negativeLearningRateIsGradientAscent() {
        assertArrayEquals(
            doubleArrayOf(2.0),
            GradientDescent.sgd(doubleArrayOf(1.0), doubleArrayOf(1.0), -1.0),
            0.0
        )
    }

    @Test
    fun returnsFreshArrayAndDoesNotMutateInputs() {
        val w = doubleArrayOf(1.0, 2.0)
        val g = doubleArrayOf(0.5, 0.5)
        val result = GradientDescent.sgd(w, g, 1.0)
        assertNotSame(w, result)
        assertNotSame(g, result)
        assertArrayEquals(doubleArrayOf(1.0, 2.0), w, 0.0)
        assertArrayEquals(doubleArrayOf(0.5, 0.5), g, 0.0)
        assertArrayEquals(doubleArrayOf(0.5, 1.5), result, 0.0)
    }

    @Test
    fun subnormalAndSignedZeroInputsAreFinite() {
        val result = GradientDescent.sgd(
            doubleArrayOf(Double.MIN_VALUE, -0.0),
            doubleArrayOf(-0.0, Double.MIN_VALUE),
            1.0
        )
        assertEquals(Double.MIN_VALUE, result[0], 0.0)
        assertEquals(-Double.MIN_VALUE, result[1], 0.0)
    }

    @Test
    fun errorCodesCarryContractIdsAndMessages() {
        val ids = mutableSetOf<String>()
        for (code in GradientDescentErrorCode.values()) {
            assertTrue(ids.add(code.errorId), "duplicate error id: ${code.errorId}")
            assertTrue(code.message.isNotEmpty())
            val ex = InvalidGradientDescentInputException(code)
            assertEquals(code.errorId, ex.errorId)
            assertEquals(code.message, ex.message)
            assertSame(code, ex.code)
        }
        assertEquals(7, ids.size)
        assertEquals("0.1.0", GradientDescentConstants.VERSION)
        assertEquals(1_048_576, GradientDescentConstants.MAX_VECTOR_LENGTH)
        assertEquals("<redacted>", GradientDescentMessages.REDACTED_PLACEHOLDER)
        assertEquals("<invalid>", GradientDescentMessages.INVALID_PLACEHOLDER)
    }

    // ----------------- 5-Class ML02 Optimizer Security Showcase -----------------

    @Test
    fun class1PrecedenceNullOverLengthMismatch() {
        expectError(GradientDescentErrorCode.NULL_INPUT, null, doubleArrayOf(1.0), 0.1)
        expectError(GradientDescentErrorCode.NULL_INPUT, doubleArrayOf(1.0), null, 0.1)
        expectError(GradientDescentErrorCode.NULL_INPUT, null, null, 0.1)
    }

    @Test
    fun class1PrecedenceLengthMismatchOverEmpty() {
        expectError(GradientDescentErrorCode.LENGTH_MISMATCH, doubleArrayOf(), doubleArrayOf(1.0), 0.1)
        expectError(GradientDescentErrorCode.LENGTH_MISMATCH, doubleArrayOf(1.0), doubleArrayOf(), 0.1)
    }

    @Test
    fun class1PrecedenceEmptyOverInputTooLarge() {
        expectError(GradientDescentErrorCode.EMPTY_INPUT, doubleArrayOf(), doubleArrayOf(), 0.1)
    }

    @Test
    fun class1PrecedenceInputTooLargeOverNonFiniteLearningRate() {
        val tooLarge = filled(GradientDescentConstants.MAX_VECTOR_LENGTH + 1, 1.0)
        expectError(GradientDescentErrorCode.INPUT_TOO_LARGE, tooLarge, tooLarge, Double.NaN)
    }

    @Test
    fun class1PrecedenceNonFiniteLearningRateOverNonFiniteInput() {
        expectError(
            GradientDescentErrorCode.NON_FINITE_LEARNING_RATE,
            doubleArrayOf(Double.NaN),
            doubleArrayOf(1.0),
            Double.NaN
        )
        expectError(
            GradientDescentErrorCode.NON_FINITE_LEARNING_RATE,
            doubleArrayOf(1.0),
            doubleArrayOf(Double.POSITIVE_INFINITY),
            Double.POSITIVE_INFINITY
        )
    }

    @Test
    fun class1PrecedenceNonFiniteInputOverNumericOverflow() {
        expectError(
            GradientDescentErrorCode.NON_FINITE_INPUT,
            doubleArrayOf(1e308, Double.NaN),
            doubleArrayOf(-1e308, 0.0),
            1.0
        )
    }

    @Test
    fun class2VectorDosCeilingAtBoundary() {
        val atLimit = filled(GradientDescentConstants.MAX_VECTOR_LENGTH, 1.0)
        val result = GradientDescent.sgd(atLimit, atLimit, 0.5)
        assertEquals(GradientDescentConstants.MAX_VECTOR_LENGTH, result.size)
        assertEquals(0.5, result[0], 0.0)

        val overLimit = filled(GradientDescentConstants.MAX_VECTOR_LENGTH + 1, 1.0)
        expectError(GradientDescentErrorCode.INPUT_TOO_LARGE, overLimit, overLimit, 0.5)
    }

    @Test
    fun class3NonFiniteWeightsAndGradientsRejected() {
        val badValues = doubleArrayOf(Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY)
        for (bad in badValues) {
            expectError(GradientDescentErrorCode.NON_FINITE_INPUT, doubleArrayOf(bad), doubleArrayOf(1.0), 0.1)
            expectError(GradientDescentErrorCode.NON_FINITE_INPUT, doubleArrayOf(1.0), doubleArrayOf(bad), 0.1)
            expectError(GradientDescentErrorCode.NON_FINITE_LEARNING_RATE, doubleArrayOf(1.0), doubleArrayOf(1.0), bad)
        }
    }

    @Test
    fun class3NumericOverflowRefused() {
        // Step addition causes overflow
        expectError(
            GradientDescentErrorCode.NUMERIC_OVERFLOW,
            doubleArrayOf(1e308),
            doubleArrayOf(-1e308),
            1.0
        )
        // Multiplication causes step overflow
        expectError(
            GradientDescentErrorCode.NUMERIC_OVERFLOW,
            doubleArrayOf(0.0),
            doubleArrayOf(1e200),
            1e200
        )
    }

    @Test
    fun class4AliasSafetyAndCallerMutationIsolation() {
        val x = doubleArrayOf(1.0, -2.0, 4.0)
        val result = GradientDescent.sgd(x, x, 0.5)
        assertArrayEquals(doubleArrayOf(0.5, -1.0, 2.0), result, 0.0)
        assertArrayEquals(doubleArrayOf(1.0, -2.0, 4.0), x, 0.0)
        x[0] = 999.0
        assertEquals(0.5, result[0], 0.0)
    }

    @Test
    fun class5PayloadBlindConfidentiality() {
        val sentinelWeight = 13371337.0
        val sentinelRate = 0.4242
        val weights = filled(7, sentinelWeight)
        val gradients = filled(7, Double.NaN)

        val forbidden = listOf(
            sentinelWeight.toString(),
            "13371337",
            sentinelRate.toString(),
            "0.4242",
            "7"
        )

        val runs = listOf(
            { GradientDescent.sgd(weights, gradients, sentinelRate) },
            { GradientDescent.sgd(weights, doubleArrayOf(1.0), sentinelRate) },
            { GradientDescent.sgd(doubleArrayOf(sentinelWeight), doubleArrayOf(1.0), Double.NaN) }
        )

        for (run in runs) {
            val ex = assertThrows(InvalidGradientDescentInputException::class.java) { run() }
            val message = ex.message ?: ""
            val asString = ex.toString()
            for (secret in forbidden) {
                assertFalse(message.contains(secret), "message leaked sensitive value: $secret")
                assertFalse(asString.contains(secret), "toString leaked sensitive value: $secret")
            }
        }
    }

    @Test
    fun prngDyadicMultiStepLinearity() {
        val rng = SplittableRandom(0xC0FFEE)
        fun dyadic(maxK: Int, shift: Int): Double {
            return (rng.nextInt(2 * maxK + 1) - maxK).toDouble() / (1 shl shift)
        }

        for (iteration in 0 until 500) {
            val length = 1 + rng.nextInt(16)
            val weights = DoubleArray(length) { dyadic(64, 4) }
            val gradients = DoubleArray(length) { dyadic(64, 4) }
            val a = dyadic(8, 3)
            val b = dyadic(8, 3)
            val twoSteps = GradientDescent.sgd(GradientDescent.sgd(weights, gradients, a), gradients, b)
            val oneStep = GradientDescent.sgd(weights, gradients, a + b)
            assertArrayEquals(twoSteps, oneStep, 0.0)
        }
    }

    @Test
    fun adversarialMutationFuzzLoop() {
        val rng = SplittableRandom(0x5EED1234)
        val specials = doubleArrayOf(
            Double.NaN,
            Double.POSITIVE_INFINITY,
            Double.NEGATIVE_INFINITY,
            0.0,
            -0.0,
            1e308,
            -1e308,
            Double.MIN_VALUE,
            Double.MAX_VALUE
        )

        fun pick(): Double {
            return if (rng.nextInt(4) == 0) {
                specials[rng.nextInt(specials.size)]
            } else {
                (rng.nextDouble() - 0.5) * 1e6
            }
        }

        val knownCodes = GradientDescentErrorCode.values().toSet()
        var accepted = 0
        var rejected = 0

        for (i in 0 until 10_000) {
            val wLen = rng.nextInt(9)
            val gLen = if (rng.nextInt(10) == 0) rng.nextInt(9) else wLen
            val weights = DoubleArray(wLen) { pick() }
            val gradients = DoubleArray(gLen) { pick() }
            val lr = pick()
            try {
                val result = GradientDescent.sgd(weights, gradients, lr)
                assertEquals(wLen, result.size)
                for (v in result) {
                    assertTrue(v.isFinite())
                }
                accepted++
            } catch (e: InvalidGradientDescentInputException) {
                assertTrue(knownCodes.contains(e.code))
                rejected++
            }
        }

        assertTrue(accepted > 0)
        assertTrue(rejected > 0)
    }
}
