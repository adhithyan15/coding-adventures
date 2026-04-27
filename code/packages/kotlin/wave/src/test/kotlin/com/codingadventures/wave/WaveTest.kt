package com.codingadventures.wave

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.math.PI
import kotlin.test.assertEquals

class WaveTest {

    private val eps = 1e-10

    @Test fun `construction`() {
        val w = Wave(1.0, 440.0)
        assertEquals(1.0, w.amplitude); assertEquals(440.0, w.frequency); assertEquals(0.0, w.phase)
    }

    @Test fun `construction with phase`() {
        assertEquals(PI / 2, Wave(2.0, 100.0, PI / 2).phase, eps)
    }

    @Test fun `period`() = assertEquals(0.25, Wave(1.0, 4.0).period, eps)
    @Test fun `angular frequency`() = assertEquals(2 * PI, Wave(1.0, 1.0).angularFrequency, eps)
    @Test fun `zero crossing`() = assertEquals(0.0, Wave(1.0, 1.0).evaluate(0.0), eps)
    @Test fun `peak`() = assertEquals(3.0, Wave(3.0, 1.0).evaluate(0.25), 1e-9)

    @Test fun `periodicity`() {
        val w = Wave(2.0, 5.0)
        assertEquals(w.evaluate(0.123), w.evaluate(0.123 + w.period), 1e-9)
    }

    @Test fun `phase shift`() = assertEquals(1.0, Wave(1.0, 1.0, PI / 2).evaluate(0.0), 1e-9)
    @Test fun `trough`() = assertEquals(-2.0, Wave(2.0, 1.0).evaluate(0.75), 1e-9)
    @Test fun `zero amplitude`() = assertEquals(0.0, Wave(0.0, 1.0).evaluate(0.5), eps)

    @Test fun `opposite phase`() {
        val w1 = Wave(1.0, 1.0, 0.0)
        val w2 = Wave(1.0, 1.0, PI)
        assertEquals(0.0, w1.evaluate(0.3) + w2.evaluate(0.3), 1e-9)
    }

    @Test fun `negative amplitude throws`() { assertThrows<IllegalArgumentException> { Wave(-1.0, 1.0) } }
    @Test fun `zero frequency throws`() { assertThrows<IllegalArgumentException> { Wave(1.0, 0.0) } }

    @Test fun `high frequency`() {
        val w = Wave(1.0, 1000.0)
        assertEquals(0.001, w.period, eps)
        assertEquals(1.0, w.evaluate(0.00025), 1e-8)
    }
}
