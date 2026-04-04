// ============================================================================
// Wave.kt — Simple Harmonic Wave Model
// ============================================================================
//
// y(t) = A · sin(2π·f·t + φ)
//
// A sinusoidal wave is the fundamental building block of signal processing.
//
// Layer: PHY01 (physics layer 1 — leaf package, zero dependencies)
// Spec:  code/specs/PHY01-wave.md
// ============================================================================

package com.codingadventures.wave

import kotlin.math.PI
import kotlin.math.sin

/**
 * An immutable sinusoidal wave: y(t) = A · sin(2π·f·t + φ).
 *
 * @property amplitude peak displacement (must be >= 0)
 * @property frequency cycles per second in Hz (must be > 0)
 * @property phase starting offset in radians
 */
data class Wave(
    val amplitude: Double,
    val frequency: Double,
    val phase: Double = 0.0
) {
    init {
        require(amplitude >= 0) { "Amplitude must be non-negative (got $amplitude)" }
        require(frequency > 0) { "Frequency must be positive (got $frequency)" }
    }

    /** Period: time for one complete cycle. T = 1/f */
    val period: Double get() = 1.0 / frequency

    /** Angular frequency in radians per second. ω = 2π·f */
    val angularFrequency: Double get() = 2.0 * PI * frequency

    /**
     * Evaluate the wave at time [t] (seconds).
     *
     * @return displacement y(t) = A · sin(2π·f·t + φ)
     */
    fun evaluate(t: Double): Double =
        amplitude * sin(2.0 * PI * frequency * t + phase)
}
