package com.codingadventures.wave;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class WaveTest {

    private static final double EPS = 1e-10;

    @Test void construction() {
        Wave w = new Wave(1.0, 440.0);
        assertEquals(1.0, w.getAmplitude());
        assertEquals(440.0, w.getFrequency());
        assertEquals(0.0, w.getPhase());
    }

    @Test void constructionWithPhase() {
        Wave w = new Wave(2.0, 100.0, Math.PI / 2);
        assertEquals(Math.PI / 2, w.getPhase(), EPS);
    }

    @Test void period() {
        assertEquals(0.25, new Wave(1.0, 4.0).period(), EPS);
    }

    @Test void angularFrequency() {
        assertEquals(2 * Math.PI, new Wave(1.0, 1.0).angularFrequency(), EPS);
    }

    @Test void zeroCrossing() {
        assertEquals(0.0, new Wave(1.0, 1.0).evaluate(0.0), EPS);
    }

    @Test void peak() {
        assertEquals(3.0, new Wave(3.0, 1.0).evaluate(0.25), 1e-9);
    }

    @Test void periodicity() {
        Wave w = new Wave(2.0, 5.0);
        double t = 0.123;
        assertEquals(w.evaluate(t), w.evaluate(t + w.period()), 1e-9);
    }

    @Test void phaseShift() {
        assertEquals(1.0, new Wave(1.0, 1.0, Math.PI / 2).evaluate(0.0), 1e-9);
    }

    @Test void trough() {
        assertEquals(-2.0, new Wave(2.0, 1.0).evaluate(0.75), 1e-9);
    }

    @Test void zeroAmplitude() {
        assertEquals(0.0, new Wave(0.0, 1.0).evaluate(0.5), EPS);
        assertEquals(0.0, new Wave(0.0, 1e300, Math.PI / 2).evaluate(Double.MAX_VALUE));
    }

    @Test void oppositePhase() {
        Wave w1 = new Wave(1.0, 1.0, 0.0);
        Wave w2 = new Wave(1.0, 1.0, Math.PI);
        assertEquals(0.0, w1.evaluate(0.3) + w2.evaluate(0.3), 1e-9);
    }

    @Test void negativeAmplitudeThrows() {
        assertThrows(IllegalArgumentException.class, () -> new Wave(-1.0, 1.0));
    }

    @Test void zeroFrequencyThrows() {
        assertThrows(IllegalArgumentException.class, () -> new Wave(1.0, 0.0));
    }

    @Test void nonFiniteAndOverflowingParametersThrow() {
        assertThrows(IllegalArgumentException.class, () -> new Wave(Double.NaN, 1.0));
        assertThrows(IllegalArgumentException.class, () -> new Wave(Double.POSITIVE_INFINITY, 1.0));
        assertThrows(IllegalArgumentException.class, () -> new Wave(1.0, Double.NaN));
        assertThrows(IllegalArgumentException.class, () -> new Wave(1.0, Double.POSITIVE_INFINITY));
        assertThrows(IllegalArgumentException.class, () -> new Wave(1.0, Double.MAX_VALUE));
        assertThrows(IllegalArgumentException.class, () -> new Wave(1.0, 1.0, Double.NaN));
        assertThrows(IllegalArgumentException.class, () -> new Wave(1.0, 1.0, Double.POSITIVE_INFINITY));
    }

    @Test void nonFiniteTimeThrows() {
        Wave wave = new Wave(1.0, 1.0);
        assertThrows(IllegalArgumentException.class, () -> wave.evaluate(Double.NaN));
        assertThrows(IllegalArgumentException.class, () -> wave.evaluate(Double.POSITIVE_INFINITY));
    }

    @Test void extremeFiniteEvaluationStaysAmplitudeBounded() {
        Wave wave = new Wave(Double.MAX_VALUE, 1e300, Math.PI / 2);
        double value = wave.evaluate(Double.MAX_VALUE);
        assertTrue(Double.isFinite(value));
        assertTrue(Math.abs(value) <= wave.getAmplitude());
    }

    @Test void highFrequency() {
        Wave w = new Wave(1.0, 1000.0);
        assertEquals(0.001, w.period(), EPS);
        assertEquals(1.0, w.evaluate(0.00025), 1e-8);
    }
}
