// Package wave models sinusoidal waves — the fundamental building block
// of acoustics, optics, electronics, and quantum mechanics.
//
// # What Is a Wave?
//
// A wave is a periodic disturbance that transfers energy through space or a
// medium. Think of ripples on a pond, sound traveling through air, or light
// crossing the vacuum of space. Despite their diversity, all these phenomena
// share a common mathematical description: the sinusoidal wave function.
//
// # The Sinusoidal Wave Equation
//
// The simplest wave is described by:
//
//	y(t) = A * sin(2 * pi * f * t + phi)
//
// Where:
//   - A (Amplitude): the maximum displacement from equilibrium.
//     A louder sound has a larger amplitude; a brighter light has a larger
//     amplitude in its electric field.
//   - f (Frequency): how many complete cycles occur per second, measured in
//     Hertz (Hz). Middle C on a piano is about 261.6 Hz.
//   - t (Time): the moment at which we evaluate the wave.
//   - phi (Phase): the initial offset of the wave in radians. A phase of 0
//     means the wave starts at zero and rises; a phase of pi/2 means it
//     starts at its peak.
//
// # Derived Properties
//
// From frequency alone, we can derive two useful quantities:
//
//   - Period (T): the time for one complete cycle. T = 1/f.
//     A 440 Hz tone (concert A) has a period of about 2.27 milliseconds.
//
//   - Angular Frequency (omega): the rate of change of the phase angle.
//     omega = 2 * pi * f. This is often more convenient in physics because
//     it absorbs the 2*pi factor that would otherwise appear everywhere.
//
// # Why This Package Exists
//
// This package sits in the PHY01 (physics layer 1) of our educational stack.
// It depends on the trig package (MATH01) for sine computation, demonstrating
// how higher-level scientific concepts build on mathematical primitives.
// By constructing waves from our own trig functions (which themselves use
// Maclaurin series), we maintain a complete chain of understanding from
// Taylor polynomials all the way up to wave physics.
package wave

import (
	"errors"

	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

// ============================================================================
// Errors
// ============================================================================

// ErrNegativeAmplitude is returned when a wave is constructed with a negative
// amplitude. Amplitude represents the magnitude of displacement and must be
// non-negative. (An amplitude of zero is valid — it represents a flat line,
// i.e., no wave at all.)
var ErrNegativeAmplitude = errors.New("amplitude must be non-negative")

// ErrZeroFrequency is returned when a wave is constructed with zero (or
// negative) frequency. Frequency represents cycles per second and must be
// positive. A frequency of zero would mean the wave never oscillates, which
// is not a wave — it's a constant.
var ErrZeroFrequency = errors.New("frequency must be positive")

// ============================================================================
// Wave Type
// ============================================================================

// Wave represents a sinusoidal wave with fixed amplitude, frequency, and phase.
//
// # Fields
//
//   - Amplitude: peak displacement from equilibrium (must be >= 0)
//   - Frequency: cycles per second in Hertz (must be > 0)
//   - Phase: initial phase offset in radians (any real number)
//
// # Example
//
// A standard concert A tone (440 Hz) with unit amplitude and no phase offset:
//
//	w, _ := wave.New(1.0, 440.0, 0.0)
//	value := w.Evaluate(0.0)  // returns 0.0 (sine starts at zero)
type Wave struct {
	Amplitude float64
	Frequency float64
	Phase     float64
}

// ============================================================================
// Constructor
// ============================================================================

// New creates a Wave with the given amplitude, frequency, and phase.
//
// # Validation Rules
//
//   - Amplitude must be >= 0 (negative amplitudes are physically meaningless;
//     if you want an inverted wave, use a phase offset of pi).
//   - Frequency must be > 0 (zero frequency is not a wave; negative frequency
//     is equivalent to positive frequency with a phase shift, so we disallow
//     it for clarity).
//   - Phase can be any real number (it will be naturally reduced by the trig
//     functions' range reduction).
//
// # Examples
//
//	// A 1 Hz wave with amplitude 5 and no phase offset:
//	w, err := wave.New(5.0, 1.0, 0.0)
//
//	// An error case — negative amplitude:
//	w, err := wave.New(-1.0, 1.0, 0.0)  // err == ErrNegativeAmplitude
func New(amplitude, frequency, phase float64) (*Wave, error) {
	// Validate amplitude: must be non-negative.
	// Amplitude is the absolute peak value of the wave. A negative peak
	// doesn't make physical sense — you'd use phase to invert the wave.
	if amplitude < 0 {
		return nil, ErrNegativeAmplitude
	}

	// Validate frequency: must be strictly positive.
	// A wave with zero frequency would never oscillate. A wave with negative
	// frequency is mathematically equivalent to positive frequency with a
	// phase shift of pi, so we enforce positive for simplicity.
	if frequency <= 0 {
		return nil, ErrZeroFrequency
	}

	return &Wave{
		Amplitude: amplitude,
		Frequency: frequency,
		Phase:     phase,
	}, nil
}

// ============================================================================
// Derived Properties
// ============================================================================

// Period returns the time (in seconds) for one complete cycle of the wave.
//
// # The Relationship Between Period and Frequency
//
// Period and frequency are reciprocals of each other:
//
//	T = 1 / f
//	f = 1 / T
//
// This makes intuitive sense:
//   - A 1 Hz wave completes 1 cycle per second, so its period is 1 second.
//   - A 2 Hz wave completes 2 cycles per second, so its period is 0.5 seconds.
//   - A 440 Hz wave (concert A) has a period of ~0.00227 seconds (~2.27 ms).
//
// # Units
//
// If frequency is in Hertz (cycles/second), then period is in seconds.
func (w *Wave) Period() float64 {
	return 1.0 / w.Frequency
}

// AngularFrequency returns omega = 2 * pi * f, the rate of phase change
// in radians per second.
//
// # Why Angular Frequency?
//
// In physics, angular frequency (omega) is often more natural than ordinary
// frequency (f) because it directly measures how fast the phase angle changes.
// Many formulas become simpler with omega:
//
//	y(t) = A * sin(omega * t + phi)     // using angular frequency
//	y(t) = A * sin(2*pi*f*t + phi)      // using ordinary frequency
//
// The relationship is straightforward:
//
//	omega = 2 * pi * f
//
// # Units
//
// Angular frequency is measured in radians per second (rad/s).
// A 1 Hz wave has omega = 2*pi ≈ 6.283 rad/s.
func (w *Wave) AngularFrequency() float64 {
	return 2.0 * trig.PI * w.Frequency
}

// ============================================================================
// Evaluation
// ============================================================================

// Evaluate computes the wave's displacement at time t.
//
// # The Core Formula
//
// This is the heart of wave physics:
//
//	y(t) = A * sin(2 * pi * f * t + phi)
//
// Let's break down what each piece contributes:
//
//  1. (2 * pi * f * t): This is the phase accumulated over time. At t=0,
//     the accumulated phase is 0. After one full period (t = 1/f), the
//     accumulated phase is 2*pi — one complete rotation around the unit
//     circle.
//
//  2. (+ phi): The initial phase offset shifts the wave left or right in
//     time. A phase of pi/2 makes the wave start at its peak instead of
//     at zero.
//
//  3. sin(...): The sine function maps the total phase angle to a value
//     between -1 and +1, creating the characteristic oscillation.
//
//  4. A * ...: The amplitude scales the oscillation. A wave with A=5
//     oscillates between -5 and +5.
//
// # Example: Tracing a 1 Hz Wave
//
//	w, _ := New(1.0, 1.0, 0.0)  // A=1, f=1 Hz, phase=0
//
//	w.Evaluate(0.00)  // sin(0)       = 0.0    (starts at zero)
//	w.Evaluate(0.25)  // sin(pi/2)    = 1.0    (reaches peak at quarter period)
//	w.Evaluate(0.50)  // sin(pi)      = 0.0    (crosses zero again)
//	w.Evaluate(0.75)  // sin(3*pi/2)  = -1.0   (reaches trough)
//	w.Evaluate(1.00)  // sin(2*pi)    = 0.0    (completes one cycle)
func (w *Wave) Evaluate(t float64) float64 {
	// Compute the total phase angle at time t.
	// This combines the time-dependent phase (2*pi*f*t) with the initial
	// phase offset (phi).
	angle := 2.0*trig.PI*w.Frequency*t + w.Phase

	// Apply the sine function and scale by amplitude.
	// We use our own trig.Sin which is built from Maclaurin series —
	// no standard library needed.
	return w.Amplitude * trig.Sin(angle)
}
