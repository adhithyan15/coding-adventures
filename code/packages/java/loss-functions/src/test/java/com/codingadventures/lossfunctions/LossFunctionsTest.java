package com.codingadventures.lossfunctions;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class LossFunctionsTest {

    private static final double TOL = 1e-7;

    // ========================================================================
    // MSE Tests
    // ========================================================================

    @Test
    void testMSEParityVector() {
        // From spec: y_true=[1,0,0], y_pred=[0.9,0.1,0.2] → 0.02
        double result = LossFunctions.mse(
            new double[]{1.0, 0.0, 0.0},
            new double[]{0.9, 0.1, 0.2});
        assertEquals(0.02, result, TOL);
    }

    @Test
    void testMSEPerfectPrediction() {
        double result = LossFunctions.mse(
            new double[]{1.0, 2.0, 3.0},
            new double[]{1.0, 2.0, 3.0});
        assertEquals(0.0, result, TOL);
    }

    @Test
    void testMSESymmetry() {
        double[] a = {1.0, 2.0, 3.0};
        double[] b = {1.5, 2.5, 3.5};
        assertEquals(LossFunctions.mse(a, b), LossFunctions.mse(b, a), TOL);
    }

    @Test
    void testMSEDerivative() {
        double[] grad = LossFunctions.mseDerivative(
            new double[]{1.0, 0.0, 0.0},
            new double[]{0.9, 0.1, 0.2});
        assertArrayEquals(new double[]{-2.0/30.0, 2.0/30.0, 4.0/30.0}, grad, TOL);
    }

    @Test
    void testMSEDerivativeSignConvention() {
        double[] grad = LossFunctions.mseDerivative(new double[]{0.0}, new double[]{1.0});
        assertTrue(grad[0] > 0.0);
    }

    // ========================================================================
    // MAE Tests
    // ========================================================================

    @Test
    void testMAEParityVector() {
        double result = LossFunctions.mae(
            new double[]{1.0, 0.0, 0.0},
            new double[]{0.9, 0.1, 0.2});
        assertEquals(0.1333333333, result, TOL);
    }

    @Test
    void testMAEPerfectPrediction() {
        double result = LossFunctions.mae(
            new double[]{1.0, 2.0, 3.0},
            new double[]{1.0, 2.0, 3.0});
        assertEquals(0.0, result, TOL);
    }

    @Test
    void testMAEDerivative() {
        double[] grad = LossFunctions.maeDerivative(
            new double[]{1.0, 0.0, 0.0},
            new double[]{0.9, 0.1, 0.2});
        assertArrayEquals(new double[]{-1.0/3.0, 1.0/3.0, 1.0/3.0}, grad, TOL);
    }

    @Test
    void testMAEDerivativeAtZero() {
        double[] grad = LossFunctions.maeDerivative(new double[]{1.0}, new double[]{1.0});
        assertEquals(0.0, grad[0], TOL);
    }

    // ========================================================================
    // BCE Tests
    // ========================================================================

    @Test
    void testBCEParityVector() {
        double result = LossFunctions.bce(
            new double[]{1.0, 0.0, 1.0},
            new double[]{0.9, 0.1, 0.8});
        assertEquals(0.1446215275, result, 1e-6);
    }

    @Test
    void testBCENonNegative() {
        double result = LossFunctions.bce(
            new double[]{1.0, 0.0, 1.0},
            new double[]{0.5, 0.5, 0.5});
        assertTrue(result >= 0.0);
    }

    @Test
    void testBCEDerivativeFinite() {
        double[] grad = LossFunctions.bceDerivative(
            new double[]{1.0, 0.0},
            new double[]{0.9, 0.1});
        for (double g : grad) {
            assertFalse(Double.isNaN(g));
            assertFalse(Double.isInfinite(g));
        }
    }

    @Test
    void testBCEHandlesEdgePredictions() {
        double result = LossFunctions.bce(new double[]{1.0, 0.0}, new double[]{0.0, 1.0});
        assertFalse(Double.isNaN(result));
        assertFalse(Double.isInfinite(result));
    }

    // ========================================================================
    // CCE Tests
    // ========================================================================

    @Test
    void testCCEParityVector() {
        double result = LossFunctions.cce(
            new double[]{1.0, 0.0, 0.0},
            new double[]{0.8, 0.1, 0.1});
        assertEquals(0.07438118, result, 1e-5);
    }

    @Test
    void testCCEOnlyCorrectClassContributes() {
        double loss1 = LossFunctions.cce(
            new double[]{1.0, 0.0, 0.0}, new double[]{0.8, 0.1, 0.1});
        double loss2 = LossFunctions.cce(
            new double[]{1.0, 0.0, 0.0}, new double[]{0.8, 0.05, 0.15});
        assertEquals(loss1, loss2, TOL);
    }

    @Test
    void testCCENonNegative() {
        double result = LossFunctions.cce(
            new double[]{0.0, 1.0, 0.0},
            new double[]{0.1, 0.7, 0.2});
        assertTrue(result >= 0.0);
    }

    @Test
    void testCCEDerivative() {
        double[] grad = LossFunctions.cceDerivative(
            new double[]{1.0, 0.0, 0.0},
            new double[]{0.8, 0.1, 0.1});
        assertTrue(grad[0] < 0.0);
        assertEquals(0.0, grad[1], 1e-5);
        assertEquals(0.0, grad[2], 1e-5);
    }

    @Test
    void testCCEHandlesZeroPredictions() {
        double result = LossFunctions.cce(new double[]{1.0, 0.0}, new double[]{0.0, 1.0});
        assertFalse(Double.isNaN(result));
        assertFalse(Double.isInfinite(result));
    }

    // ========================================================================
    // Validation Tests
    // ========================================================================

    @Test
    void testEmptyArrayThrows() {
        assertThrows(IllegalArgumentException.class, () ->
            LossFunctions.mse(new double[]{}, new double[]{}));
    }

    @Test
    void testMismatchedLengthThrows() {
        assertThrows(IllegalArgumentException.class, () ->
            LossFunctions.mse(new double[]{1.0}, new double[]{1.0, 2.0}));
    }

    // ========================================================================
    // Single Element Tests
    // ========================================================================

    @Test
    void testSingleElementMSE() {
        assertEquals(0.25, LossFunctions.mse(new double[]{1.0}, new double[]{0.5}), TOL);
    }

    @Test
    void testSingleElementMAE() {
        assertEquals(0.5, LossFunctions.mae(new double[]{1.0}, new double[]{0.5}), TOL);
    }
}
