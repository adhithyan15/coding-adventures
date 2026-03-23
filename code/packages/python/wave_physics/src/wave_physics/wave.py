"""
Wave Physics — Simple Harmonic Wave Model
==========================================

This module models a **simple harmonic wave**, the most fundamental building
block in wave physics.  Every electromagnetic wave — light, radio, X-rays —
can be decomposed into simple harmonic components via Fourier analysis, so
understanding this single waveform unlocks the entire electromagnetic
spectrum.

The wave equation
-----------------

A simple harmonic wave is described by:

    y(t) = A * sin(2 * pi * f * t + phi)

where:

    y(t)  — the wave's value at time t  (same units as amplitude)
    A     — **amplitude**: the peak displacement from zero.
            Think of it as "how tall the wave is."
            For a sound wave, larger amplitude = louder.
            For a light wave, larger amplitude = brighter.
    f     — **frequency**: how many full cycles occur per second,
            measured in Hertz (Hz).  A 440 Hz sound wave completes
            440 full oscillations every second — that's the note A4.
    t     — time, in seconds.
    phi   — **phase**: shifts the wave left or right along the time
            axis, measured in radians.  A phase of 0 means the wave
            starts at zero and rises; a phase of pi/2 means the wave
            starts at its peak.

Derived quantities
------------------

From frequency alone we can derive two useful values:

    T = 1 / f          — the **period**: time for one full cycle (seconds).
    omega = 2 * pi * f — the **angular frequency**: radians per second.
                          This is how fast the angle inside the sin()
                          advances.  It saves writing "2 * pi * f"
                          repeatedly in physics equations.

Why sin() from trig, not math?
------------------------------

We import sin and PI from our own `trig` package rather than Python's
built-in `math` module.  The trig package is a lower layer in our
educational stack — it implements trigonometric functions from first
principles using Taylor series.  Using it here demonstrates how layers
build on each other: trig -> wave_physics -> (future EM packages).
"""

from trig import PI, sin


# ── Wave class ────────────────────────────────────────────────────────────────


class Wave:
    """A simple harmonic wave: y(t) = A * sin(2 * pi * f * t + phi).

    Parameters
    ----------
    amplitude : float
        Peak displacement from zero.  Must be >= 0.
        (An amplitude of 0 is technically valid — it's a flat line.)
    frequency : float
        Cycles per second (Hz).  Must be > 0.
        (A frequency of 0 would mean the wave never oscillates,
        which isn't really a wave at all.)
    phase : float, optional
        Phase offset in radians.  Defaults to 0.0.
        A phase of PI/2 shifts the wave so it starts at its peak.

    Raises
    ------
    ValueError
        If amplitude < 0 or frequency <= 0.

    Examples
    --------
    >>> w = Wave(amplitude=1.0, frequency=440.0)
    >>> w.evaluate(0.0)        # sin(0) = 0
    0.0
    >>> w.period()             # 1/440 seconds
    0.002272727272727...
    """

    def __init__(self, amplitude: float, frequency: float, phase: float = 0.0) -> None:
        # ── Validation ────────────────────────────────────────────────
        # Amplitude must be non-negative.  A wave with negative amplitude
        # doesn't make physical sense — you'd just flip the phase by PI
        # instead.  We allow zero because a zero-amplitude wave is a
        # valid degenerate case (silence, darkness, etc.).
        if amplitude < 0:
            raise ValueError(
                f"Amplitude must be >= 0, got {amplitude}.  "
                "A negative amplitude has no physical meaning — "
                "use a phase shift of PI to invert the wave instead."
            )

        # Frequency must be strictly positive.  A zero-frequency wave
        # would have an infinite period and never oscillate — that's
        # just a constant, not a wave.
        if frequency <= 0:
            raise ValueError(
                f"Frequency must be > 0, got {frequency}.  "
                "A wave must oscillate at least once per second."
            )

        self._amplitude = float(amplitude)
        self._frequency = float(frequency)
        self._phase = float(phase)

    # ── Properties ────────────────────────────────────────────────────
    # We use read-only properties so that a Wave is immutable after
    # creation.  This makes it safe to share Wave objects across
    # threads or store them in sets/dicts without worrying about
    # someone changing the frequency out from under you.

    @property
    def amplitude(self) -> float:
        """Peak displacement from zero (non-negative)."""
        return self._amplitude

    @property
    def frequency(self) -> float:
        """Cycles per second, in Hertz (positive)."""
        return self._frequency

    @property
    def phase(self) -> float:
        """Phase offset in radians."""
        return self._phase

    # ── Derived quantities ────────────────────────────────────────────

    def period(self) -> float:
        """Time for one complete cycle, in seconds.

        The period T is the reciprocal of frequency:

            T = 1 / f

        A 2 Hz wave completes one cycle every 0.5 seconds.
        A 440 Hz wave (concert A) has a period of about 2.27 milliseconds.
        """
        return 1.0 / self._frequency

    def angular_frequency(self) -> float:
        """Radians per second (omega = 2 * pi * f).

        Angular frequency converts "cycles per second" into
        "radians per second."  Since one full cycle = 2*PI radians,
        we simply multiply frequency by 2*PI.

        This quantity appears everywhere in physics:
        - Hooke's law for springs: omega = sqrt(k/m)
        - LC circuits: omega = 1/sqrt(LC)
        - Quantum mechanics: E = hbar * omega
        """
        return 2.0 * PI * self._frequency

    # ── Core evaluation ───────────────────────────────────────────────

    def evaluate(self, t: float) -> float:
        """Compute the wave's value at time t.

        Applies the fundamental wave equation:

            y(t) = A * sin(2 * pi * f * t + phi)

        Step by step:
        1. Compute the angle: theta = 2 * PI * f * t + phase
           This tells us "where in the cycle" we are at time t.
        2. Take sin(theta) to get a value between -1 and +1.
        3. Multiply by amplitude to scale to the wave's actual size.

        Parameters
        ----------
        t : float
            Time in seconds.  Can be negative (looking back in time)
            or any positive value.

        Returns
        -------
        float
            The wave's displacement at time t.  Always in the range
            [-amplitude, +amplitude].

        Examples
        --------
        A 1 Hz wave with amplitude 1 at various times:

        >>> w = Wave(1.0, 1.0)
        >>> w.evaluate(0.0)     # sin(0) = 0
        0.0
        >>> w.evaluate(0.25)    # sin(PI/2) = 1  (quarter cycle)
        1.0
        >>> w.evaluate(0.5)     # sin(PI) = 0    (half cycle)
        0.0
        >>> w.evaluate(0.75)    # sin(3*PI/2) = -1 (three-quarter cycle)
        -1.0
        """
        # The angle (in radians) tells us where we are in the cycle.
        # At t=0 with phase=0, the angle is 0 and sin(0) = 0.
        # At t = 1/(4f) (quarter period), the angle is PI/2 and sin = 1.
        theta = 2.0 * PI * self._frequency * t + self._phase

        return self._amplitude * sin(theta)

    # ── String representation ─────────────────────────────────────────

    def __repr__(self) -> str:
        return (
            f"Wave(amplitude={self._amplitude}, "
            f"frequency={self._frequency}, "
            f"phase={self._phase})"
        )
