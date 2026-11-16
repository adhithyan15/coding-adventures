"""Naive instrument models for beginner-friendly virtual music.

The first version deliberately models instruments as:

```text
note frequency + harmonic profile + ADSR envelope -> sampled PCM
```

That is not a faithful physical simulation yet. It is a stable teaching layer
that lets a score, keyboard, or orchestra package select a named timbre while
the internals remain inspectable.

This module enforces CWE-400 resource exhaustion ceilings, CWE-674 recursion
elimination, and CWE-1333/94 regular expression and code injection freedom
in compliance with VER15.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import isfinite
from numbers import Integral, Real

from note_frequency import Note, parse_note
from oscillator import SampleBuffer, UniformSampler
from pcm_audio import DEFAULT_SAMPLE_RATE_HZ, PCMBuffer, PCMFormat, encode_sample_buffer
from trig import PI, sin

# =============================================================================
# § 1. Mathematical Constants and Security Bounds (VER15 / CWE-400 / CWE-674)
# =============================================================================
# In accordance with CWE-400 (Uncontrolled Resource Consumption), all memory
# allocations, harmonic partial counts, time windows, and sample buffers are
# bounded by explicit upper ceilings. All synthesis routines are strictly
# iterative, ensuring provable resistance to CWE-674 (Uncontrolled Recursion).
# Zero regular expression or dynamic evaluation libraries are imported,
# eliminating CWE-1333 (ReDoS) and CWE-94 (Code Injection).

TWO_PI: float = 2.0 * PI

MAX_PARTIALS: int = 64
MAX_NOTE_DURATION_SECONDS: float = 60.0
MAX_SAMPLE_RATE_HZ: float = 192_000.0
MAX_SAMPLE_COUNT: int = 16_777_216

DEFAULT_MAX_PARTIAL_COUNT: int = MAX_PARTIALS
DEFAULT_MAX_SAMPLE_COUNT: int = 10_000_000
SYNTHESIS_KINDS: frozenset[str] = frozenset({"sine", "additive", "silence"})

NoteInput = Note | str


# =============================================================================
# § 2. Defensive Validation Helpers (VER15 / CWE-190 / CWE-20 / CWE-400)
# =============================================================================
# Every input parameter passes through strict numeric validation: rejecting
# NaN, infinities, negative values, and non-numeric types before entering the
# signal generation pipeline.


def _finite_float(name: str, value: Real) -> float:
    """Convert a real number to float and reject non-finite values."""
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")
    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")
    return converted


def _non_negative_float(name: str, value: Real) -> float:
    """Convert a real number to float and ensure it is >= 0.0."""
    converted = _finite_float(name, value)
    if converted < 0.0:
        raise ValueError(f"{name} must be >= 0.0, got {converted}")
    return converted


def _positive_float(name: str, value: Real) -> float:
    """Convert a real number to float and ensure it is > 0.0."""
    converted = _finite_float(name, value)
    if converted <= 0.0:
        raise ValueError(f"{name} must be > 0.0, got {converted}")
    return converted


def _sample_rate(name: str, value: Real) -> float:
    """Convert and validate audio sampling rate within (0, MAX_SAMPLE_RATE_HZ]."""
    converted = _positive_float(name, value)
    if converted > MAX_SAMPLE_RATE_HZ:
        raise ValueError(
            f"{name} {converted} exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
        )
    return converted


def _safe_round_sample(time_val: float, rate: float) -> int:
    """Safely multiply time by sample rate and round to integer index (CWE-190)."""
    if not isfinite(time_val) or not isfinite(rate):
        raise ValueError("time and rate must be finite")
    prod = time_val * rate
    if not isfinite(prod):
        raise ValueError("time * rate product must be finite")
    return int(round(prod))


def _unit_float(name: str, value: Real) -> float:
    """Convert a real number to float and ensure it falls within [0.0, 1.0]."""
    converted = _finite_float(name, value)
    if not 0.0 <= converted <= 1.0:
        raise ValueError(f"{name} must be in [0.0, 1.0], got {converted}")
    return converted


def _positive_int(name: str, value: int) -> int:
    """Convert an integral value to int and ensure it is > 0."""
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer > 0, got {value!r}")
    converted = int(value)
    if converted <= 0:
        raise ValueError(f"{name} must be > 0, got {converted}")
    return converted


def _parse_note_input(note: NoteInput) -> Note:
    """Resolve NoteInput to a validated Note instance."""
    if isinstance(note, Note):
        return note
    if isinstance(note, str):
        return parse_note(note)
    raise ValueError(f"note must be a note string or Note, got {note!r}")


# =============================================================================
# § 3. Deterministic SplitMix64 PRNG Engine (VER05 §2.1 / TF01 §5.1)
# =============================================================================
# Pure-Python deterministic SplitMix64 pseudo-random number generator.
# Operates strictly on 64-bit unsigned integers modulo 2^64 with zero ambient
# entropy, eliminating timing side-channels and non-determinism.


class SplitMix64:
    """Pure-Python deterministic SplitMix64 PRNG (VER05 §2.1 / TF01 §5.1).

    Generates reproducible 64-bit pseudo-random streams with zero ambient
    entropy and zero third-party dependencies, discharging CWE-330 for
    non-cryptographic deterministic timbre variation.
    """

    __slots__ = ("_state",)

    def __init__(self, seed: int) -> None:
        if isinstance(seed, bool) or not isinstance(seed, Integral):
            raise ValueError(f"seed must be an integer, got {seed!r}")
        self._state: int = int(seed) & 0xFFFF_FFFF_FFFF_FFFF

    def next_u64(self) -> int:
        """Advance internal state and return next 64-bit pseudo-random word."""
        self._state = (self._state + 0x9E37_79B9_7F4A_7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self._state
        z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & 0xFFFF_FFFF_FFFF_FFFF
        return (z ^ (z >> 31)) & 0xFFFF_FFFF_FFFF_FFFF

    def next_float_signed(self) -> float:
        """Return a uniform pseudo-random float in [-1.0, 1.0]."""
        val = self.next_u64()
        return (val / 9223372036854775807.5) - 1.0

    def next_float_unit(self) -> float:
        """Return a uniform pseudo-random float in [0.0, 1.0]."""
        val = self.next_u64()
        return val / 18446744073709551615.0


# =============================================================================
# § 4. Harmonic Profiles and Partial Oscillators (MUS05 §5)
# =============================================================================
# In additive synthesis, each musical timbre is decomposed into a sum of
# discrete sine partials, each characterized by a frequency multiplier,
# relative amplitude, and initial phase cycle offset.


@dataclass(frozen=True)
class HarmonicPartial:
    """One sine component inside an additive instrument profile."""

    frequency_multiplier: float
    amplitude: float
    phase_cycles: float = 0.0

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "frequency_multiplier",
            _positive_float("frequency_multiplier", self.frequency_multiplier),
        )
        object.__setattr__(
            self,
            "amplitude",
            _non_negative_float("amplitude", self.amplitude),
        )
        object.__setattr__(
            self,
            "phase_cycles",
            _finite_float("phase_cycles", self.phase_cycles),
        )


# =============================================================================
# § 5. ADSR Envelopes and Loudness Shapes (MUS05 §6)
# =============================================================================
# Standard 4-stage loudness envelope: attack, decay, sustain, and release.
# Stage durations are strictly bounded by MAX_NOTE_DURATION_SECONDS to prevent
# uncontrolled sample budget inflation (CWE-400).


@dataclass(frozen=True)
class ADSREnvelope:
    """Attack, decay, sustain, and release loudness shape."""

    attack_seconds: float
    decay_seconds: float
    sustain_level: float
    release_seconds: float

    def __post_init__(self) -> None:
        atk = _non_negative_float("attack_seconds", self.attack_seconds)
        if atk > MAX_NOTE_DURATION_SECONDS:
            raise ValueError(
                f"attack_seconds {atk} exceeds "
                f"MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
            )
        object.__setattr__(self, "attack_seconds", atk)

        dec = _non_negative_float("decay_seconds", self.decay_seconds)
        if dec > MAX_NOTE_DURATION_SECONDS:
            raise ValueError(
                f"decay_seconds {dec} exceeds "
                f"MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
            )
        object.__setattr__(self, "decay_seconds", dec)

        object.__setattr__(
            self,
            "sustain_level",
            _unit_float("sustain_level", self.sustain_level),
        )

        rel = _non_negative_float("release_seconds", self.release_seconds)
        if rel > MAX_NOTE_DURATION_SECONDS:
            raise ValueError(
                f"release_seconds {rel} exceeds "
                f"MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
            )
        object.__setattr__(self, "release_seconds", rel)

    def rendered_duration_seconds(self, note_duration_seconds: Real) -> float:
        """Compute total duration including post-note release tail."""
        note_dur = _non_negative_float("note_duration_seconds", note_duration_seconds)
        if note_dur > MAX_NOTE_DURATION_SECONDS:
            raise ValueError(
                f"note_duration_seconds {note_dur} exceeds "
                f"MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
            )
        return note_dur + self.release_seconds

    def value_at(self, time_seconds: Real, note_duration_seconds: Real) -> float:
        """Return the envelope gain for a rendered note-local time."""
        time = _non_negative_float("time_seconds", time_seconds)
        note_duration = _non_negative_float(
            "note_duration_seconds",
            note_duration_seconds,
        )
        if note_duration > MAX_NOTE_DURATION_SECONDS:
            raise ValueError(
                f"note_duration_seconds {note_duration} exceeds "
                f"MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
            )

        if time < self.attack_seconds:
            return time / self.attack_seconds

        decay_end = self.attack_seconds + self.decay_seconds
        if time < decay_end:
            progress = (time - self.attack_seconds) / self.decay_seconds
            return 1.0 + (self.sustain_level - 1.0) * progress

        if time < note_duration:
            return self.sustain_level

        release_time = time - note_duration
        if self.release_seconds == 0.0 or release_time >= self.release_seconds:
            return 0.0
        return self.sustain_level * (1.0 - release_time / self.release_seconds)


# =============================================================================
# § 6. Seeded Variation Profiles (MUS05 §7)
# =============================================================================
# Knobs controlling subtle micro-variations in pitch, amplitude, attack
# timing, and overtone distribution. When seed is provided, variation is
# driven strictly by SplitMix64, ensuring reproducible humanization.


@dataclass(frozen=True)
class VariationProfile:
    """Deterministic variation profile for natural instrument imperfections."""

    pitch_cents_jitter: float = 0.0
    amplitude_jitter: float = 0.0
    attack_jitter_ms: float = 0.0
    seed: int | None = None
    harmonic_jitter: float = 0.0

    def __init__(
        self,
        pitch_cents_jitter: Real = 0.0,
        amplitude_jitter: Real = 0.0,
        attack_jitter_ms: Real = 0.0,
        seed: int | None = None,
        *,
        pitch_jitter_cents: Real | None = None,
        timing_jitter_seconds: Real | None = None,
        harmonic_jitter: Real = 0.0,
    ) -> None:
        if pitch_jitter_cents is not None:
            pitch_cents_jitter = pitch_jitter_cents
        if timing_jitter_seconds is not None:
            attack_jitter_ms = (
                _non_negative_float("timing_jitter_seconds", timing_jitter_seconds)
                * 1000.0
            )

        p_jitter = _non_negative_float("pitch_cents_jitter", pitch_cents_jitter)
        a_jitter = _non_negative_float("amplitude_jitter", amplitude_jitter)
        atk_jitter = _non_negative_float("attack_jitter_ms", attack_jitter_ms)
        h_jitter = _non_negative_float("harmonic_jitter", harmonic_jitter)

        norm_seed: int | None = None
        if seed is not None:
            if isinstance(seed, bool) or not isinstance(seed, Integral):
                raise ValueError("seed must be an integer or None")
            norm_seed = int(seed)

        object.__setattr__(self, "pitch_cents_jitter", p_jitter)
        object.__setattr__(self, "amplitude_jitter", a_jitter)
        object.__setattr__(self, "attack_jitter_ms", atk_jitter)
        object.__setattr__(self, "seed", norm_seed)
        object.__setattr__(self, "harmonic_jitter", h_jitter)

    @property
    def pitch_jitter_cents(self) -> float:
        """Alias for pitch_cents_jitter."""
        return self.pitch_cents_jitter

    @property
    def timing_jitter_seconds(self) -> float:
        """Alias for attack_jitter_ms in seconds."""
        return self.attack_jitter_ms / 1000.0


# =============================================================================
# § 7. Instrument Profiles and Timbre Catalog (MUS05 §4 & §8)
# =============================================================================
# Naive instrument profiles with harmonic recipes and envelope shapes.


@dataclass(frozen=True)
class InstrumentProfile:
    """A selectable naive instrument timbre."""

    id: str
    display_name: str
    synthesis_kind: str
    gain: float
    harmonic_profile: tuple[HarmonicPartial, ...]
    envelope_profile: ADSREnvelope
    variation_profile: VariationProfile = VariationProfile()
    gm_program: int | None = None
    family: str = "custom"

    def __post_init__(self) -> None:
        object.__setattr__(self, "id", str(self.id))
        if self.id == "":
            raise ValueError("id must be non-empty")
        object.__setattr__(self, "display_name", str(self.display_name))
        if self.display_name == "":
            raise ValueError("display_name must be non-empty")
        if self.synthesis_kind not in SYNTHESIS_KINDS:
            raise ValueError(f"unsupported synthesis_kind {self.synthesis_kind!r}")
        object.__setattr__(self, "gain", _unit_float("gain", self.gain))

        partials = tuple(self.harmonic_profile)
        if len(partials) > MAX_PARTIALS:
            raise ValueError(
                f"harmonic_profile partial count {len(partials)} exceeds "
                f"MAX_PARTIALS={MAX_PARTIALS}"
            )
        object.__setattr__(self, "harmonic_profile", partials)

        for index, partial in enumerate(self.harmonic_profile):
            if not isinstance(partial, HarmonicPartial):
                raise ValueError(f"harmonic_profile[{index}] must be a HarmonicPartial")
        if self.synthesis_kind != "silence" and not self.harmonic_profile:
            raise ValueError("non-silent instruments need at least one partial")
        if not isinstance(self.envelope_profile, ADSREnvelope):
            raise ValueError("envelope_profile must be an ADSREnvelope")
        if not isinstance(self.variation_profile, VariationProfile):
            raise ValueError("variation_profile must be a VariationProfile")
        if self.gm_program is not None:
            program = _positive_int("gm_program", self.gm_program)
            if program > 128:
                raise ValueError("gm_program must be in [1, 128]")
            object.__setattr__(self, "gm_program", program)
        object.__setattr__(self, "family", str(self.family))

    def amplitude_normalizer(self) -> float:
        """Sum of partial amplitudes for safe unity scaling before gain."""
        total = sum(partial.amplitude for partial in self.harmonic_profile)
        return total if total > 0.0 else 1.0


# =============================================================================
# § 8. General MIDI Program Map (MUS05 §4)
# =============================================================================


@dataclass(frozen=True)
class GMProgram:
    """One General-MIDI-style melodic program entry."""

    program: int
    name: str
    family: str
    instrument_id: str

    def __post_init__(self) -> None:
        program = _positive_int("program", self.program)
        if program > 128:
            raise ValueError("program must be in [1, 128]")
        object.__setattr__(self, "program", program)
        object.__setattr__(self, "name", str(self.name))
        object.__setattr__(self, "family", str(self.family))
        object.__setattr__(self, "instrument_id", str(self.instrument_id))


# =============================================================================
# § 9. Continuous Instrument Signal Generation (MUS05 §10)
# =============================================================================
# Evaluates the instantaneous amplitude of an instrument voice at any continuous
# time t >= 0.0. Jitter variations are applied deterministically via SplitMix64
# during signal construction, ensuring zero allocation inside value_at().


@dataclass(frozen=True)
class InstrumentSignal:
    """Continuous signal for one instrument playing one fundamental frequency."""

    fundamental_hz: float
    instrument: InstrumentProfile
    note_duration_seconds: float
    amplitude: float = 1.0
    sample_rate_hz: float = DEFAULT_SAMPLE_RATE_HZ
    variation_profile: VariationProfile | None = None

    def __post_init__(self) -> None:
        fund_hz = _positive_float("fundamental_hz", self.fundamental_hz)
        object.__setattr__(self, "fundamental_hz", fund_hz)
        if not isinstance(self.instrument, InstrumentProfile):
            raise ValueError("instrument must be an InstrumentProfile")

        dur = _non_negative_float("note_duration_seconds", self.note_duration_seconds)
        if dur > MAX_NOTE_DURATION_SECONDS:
            raise ValueError(
                f"note_duration_seconds {dur} exceeds "
                f"MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
            )
        object.__setattr__(self, "note_duration_seconds", dur)

        object.__setattr__(self, "amplitude", _unit_float("amplitude", self.amplitude))
        sr = _sample_rate("sample_rate_hz", self.sample_rate_hz)
        object.__setattr__(self, "sample_rate_hz", sr)

        var = self.variation_profile
        if var is None:
            var = self.instrument.variation_profile
        elif not isinstance(var, VariationProfile):
            raise ValueError("variation_profile must be a VariationProfile or None")
        object.__setattr__(self, "variation_profile", var)

        # Deterministic micro-variation via SplitMix64 PRNG
        if var.seed is not None:
            rng = SplitMix64(var.seed)

            # 1. Pitch jitter in cents: f' = f * 2^(cents / 1200)
            pitch_draw = rng.next_float_signed()
            delta_cents = pitch_draw * var.pitch_cents_jitter
            ratio = 2.0 ** (delta_cents / 1200.0)
            eff_fund = fund_hz * ratio

            # 2. Amplitude jitter
            amp_draw = rng.next_float_signed()
            amp_factor = 1.0 + amp_draw * var.amplitude_jitter
            eff_amp = max(0.0, min(1.0, self.amplitude * amp_factor))

            # 3. Attack jitter (ms to seconds)
            atk_draw = rng.next_float_signed()
            atk_delta = atk_draw * (var.attack_jitter_ms / 1000.0)
            base_env = self.instrument.envelope_profile
            eff_atk = max(
                0.0, min(MAX_NOTE_DURATION_SECONDS, base_env.attack_seconds + atk_delta)
            )
            eff_env = ADSREnvelope(
                attack_seconds=eff_atk,
                decay_seconds=base_env.decay_seconds,
                sustain_level=base_env.sustain_level,
                release_seconds=base_env.release_seconds,
            )

            # 4. Harmonic jitter
            if var.harmonic_jitter > 0.0:
                varied_partials: list[HarmonicPartial] = []
                for p in self.instrument.harmonic_profile:
                    h_draw = rng.next_float_signed()
                    p_factor = 1.0 + h_draw * var.harmonic_jitter
                    p_amp = max(0.0, p.amplitude * p_factor)
                    varied_partials.append(
                        HarmonicPartial(p.frequency_multiplier, p_amp, p.phase_cycles)
                    )
                eff_partials = tuple(varied_partials)
            else:
                eff_partials = self.instrument.harmonic_profile
        else:
            eff_fund = fund_hz
            eff_amp = self.amplitude
            eff_env = self.instrument.envelope_profile
            eff_partials = self.instrument.harmonic_profile

        object.__setattr__(self, "effective_fundamental_hz", eff_fund)
        object.__setattr__(self, "effective_amplitude", eff_amp)
        object.__setattr__(self, "effective_envelope", eff_env)
        object.__setattr__(self, "effective_harmonic_profile", eff_partials)

        total = sum(p.amplitude for p in eff_partials)
        norm = total if total > 0.0 else 1.0
        object.__setattr__(self, "_normalizer", norm)

    def value_at(self, time_seconds: float) -> float:
        """Compute the instantaneous amplitude at note-local time_seconds."""
        time = _non_negative_float("time_seconds", time_seconds)
        if self.instrument.synthesis_kind == "silence":
            return 0.0

        nyquist_hz = self.sample_rate_hz / 2.0
        raw = 0.0
        eff_fund = self.effective_fundamental_hz
        for partial in self.effective_harmonic_profile:
            partial_hz = eff_fund * partial.frequency_multiplier
            if partial_hz >= nyquist_hz:
                continue
            raw += partial.amplitude * sin(
                TWO_PI * (partial_hz * time + partial.phase_cycles)
            )

        envelope = self.effective_envelope.value_at(
            time,
            self.note_duration_seconds,
        )
        return (
            self.effective_amplitude
            * self.instrument.gain
            * envelope
            * raw
            / self._normalizer
        )


@dataclass(frozen=True)
class InstrumentNoteRender:
    """The inspectable output of rendering one note with one instrument."""

    note: Note
    fundamental_hz: float
    instrument: InstrumentProfile
    signal: InstrumentSignal
    floating_samples: SampleBuffer
    pcm_buffer: PCMBuffer


def _partials(*values: tuple[float, float]) -> tuple[HarmonicPartial, ...]:
    return tuple(
        HarmonicPartial(frequency_multiplier=multiplier, amplitude=amplitude)
        for multiplier, amplitude in values
    )


# =============================================================================
# Built-in Preset Library (MUS05 §8)
# =============================================================================

SINE = InstrumentProfile(
    id="sine",
    display_name="Pure Sine",
    synthesis_kind="sine",
    gain=1.0,
    harmonic_profile=_partials((1.0, 1.0)),
    envelope_profile=ADSREnvelope(0.005, 0.010, 0.90, 0.030),
    family="synth",
)
SILENCE = InstrumentProfile(
    id="silence",
    display_name="Silence",
    synthesis_kind="silence",
    gain=0.0,
    harmonic_profile=(),
    envelope_profile=ADSREnvelope(0.0, 0.0, 0.0, 0.0),
    family="utility",
)
FLUTE_NAIVE = InstrumentProfile(
    id="flute_naive",
    display_name="Naive Flute",
    synthesis_kind="additive",
    gain=0.95,
    harmonic_profile=_partials((1.0, 1.00), (2.0, 0.10), (3.0, 0.03)),
    envelope_profile=ADSREnvelope(0.080, 0.050, 0.85, 0.080),
    family="pipe",
)
CLARINET_NAIVE = InstrumentProfile(
    id="clarinet_naive",
    display_name="Naive Clarinet",
    synthesis_kind="additive",
    gain=0.78,
    harmonic_profile=_partials((1.0, 1.00), (3.0, 0.45), (5.0, 0.20), (7.0, 0.08)),
    envelope_profile=ADSREnvelope(0.040, 0.060, 0.75, 0.070),
    family="reed",
)
VIOLIN_NAIVE = InstrumentProfile(
    id="violin_naive",
    display_name="Naive Violin",
    synthesis_kind="additive",
    gain=0.62,
    harmonic_profile=_partials(
        (1.0, 1.00),
        (2.0, 0.55),
        (3.0, 0.35),
        (4.0, 0.20),
        (5.0, 0.12),
    ),
    envelope_profile=ADSREnvelope(0.070, 0.080, 0.80, 0.120),
    family="strings",
)
PIANO_NAIVE = InstrumentProfile(
    id="piano_naive",
    display_name="Naive Piano",
    synthesis_kind="additive",
    gain=0.82,
    harmonic_profile=_partials(
        (1.0, 1.00),
        (2.0, 0.45),
        (3.0, 0.25),
        (4.0, 0.14),
        (5.0, 0.08),
    ),
    envelope_profile=ADSREnvelope(0.005, 0.350, 0.18, 0.180),
    family="piano",
)
PLUCK_NAIVE = InstrumentProfile(
    id="pluck_naive",
    display_name="Naive Plucked String",
    synthesis_kind="additive",
    gain=0.66,
    harmonic_profile=_partials(
        (1.0, 1.00),
        (2.0, 0.60),
        (3.0, 0.38),
        (4.0, 0.22),
        (5.0, 0.13),
        (6.0, 0.08),
    ),
    envelope_profile=ADSREnvelope(0.003, 0.450, 0.05, 0.080),
    family="plucked",
)
BRASS_NAIVE = InstrumentProfile(
    id="brass_naive",
    display_name="Naive Brass",
    synthesis_kind="additive",
    gain=0.64,
    harmonic_profile=_partials(
        (1.0, 1.00),
        (2.0, 0.70),
        (3.0, 0.42),
        (4.0, 0.22),
        (5.0, 0.14),
    ),
    envelope_profile=ADSREnvelope(0.030, 0.070, 0.78, 0.090),
    family="brass",
)
ORGAN_NAIVE = InstrumentProfile(
    id="organ_naive",
    display_name="Naive Organ",
    synthesis_kind="additive",
    gain=0.55,
    harmonic_profile=_partials((1.0, 1.00), (2.0, 0.75), (3.0, 0.45), (4.0, 0.25)),
    envelope_profile=ADSREnvelope(0.010, 0.010, 0.95, 0.040),
    family="organ",
)
MALLET_NAIVE = InstrumentProfile(
    id="mallet_naive",
    display_name="Naive Mallet",
    synthesis_kind="additive",
    gain=0.74,
    harmonic_profile=_partials((1.0, 1.00), (2.7, 0.28), (5.1, 0.12)),
    envelope_profile=ADSREnvelope(0.002, 0.600, 0.02, 0.120),
    family="chromatic percussion",
)
CELESTA_NAIVE = InstrumentProfile(
    id="celesta_naive",
    display_name="Naive Celesta",
    synthesis_kind="additive",
    gain=0.60,
    harmonic_profile=_partials((1.0, 1.00), (2.0, 0.35), (4.0, 0.18), (6.3, 0.08)),
    envelope_profile=ADSREnvelope(0.004, 0.900, 0.10, 0.250),
    family="chromatic percussion",
)
GLOCKENSPIEL_NAIVE = InstrumentProfile(
    id="glockenspiel_naive",
    display_name="Naive Glockenspiel",
    synthesis_kind="additive",
    gain=0.50,
    harmonic_profile=_partials((1.0, 1.00), (2.8, 0.55), (5.4, 0.30), (8.9, 0.15)),
    envelope_profile=ADSREnvelope(0.001, 0.900, 0.02, 0.500),
    family="chromatic percussion",
)
VIBRAPHONE_NAIVE = InstrumentProfile(
    id="vibraphone_naive",
    display_name="Naive Vibraphone",
    synthesis_kind="additive",
    gain=0.58,
    harmonic_profile=_partials((1.0, 1.00), (2.0, 0.35), (3.9, 0.16), (6.8, 0.06)),
    envelope_profile=ADSREnvelope(0.003, 0.800, 0.28, 0.450),
    family="chromatic percussion",
)
MARIMBA_NAIVE = InstrumentProfile(
    id="marimba_naive",
    display_name="Naive Marimba",
    synthesis_kind="additive",
    gain=0.68,
    harmonic_profile=_partials((1.0, 1.00), (4.0, 0.30), (10.2, 0.10)),
    envelope_profile=ADSREnvelope(0.002, 0.550, 0.03, 0.120),
    family="chromatic percussion",
)
XYLOPHONE_NAIVE = InstrumentProfile(
    id="xylophone_naive",
    display_name="Naive Xylophone",
    synthesis_kind="additive",
    gain=0.64,
    harmonic_profile=_partials((1.0, 1.00), (3.2, 0.32), (6.1, 0.14)),
    envelope_profile=ADSREnvelope(0.001, 0.350, 0.01, 0.080),
    family="chromatic percussion",
)
TUBULAR_BELLS_NAIVE = InstrumentProfile(
    id="tubular_bells_naive",
    display_name="Naive Tubular Bells",
    synthesis_kind="additive",
    gain=0.46,
    harmonic_profile=_partials(
        (1.0, 1.00),
        (2.0, 0.20),
        (3.0, 0.12),
        (4.2, 0.28),
        (6.8, 0.14),
    ),
    envelope_profile=ADSREnvelope(0.003, 1.300, 0.04, 0.800),
    family="chromatic percussion",
)
TIMPANI_NAIVE = InstrumentProfile(
    id="timpani_naive",
    display_name="Naive Timpani",
    synthesis_kind="additive",
    gain=0.70,
    harmonic_profile=_partials(
        (1.0, 1.00),
        (1.5, 0.25),
        (2.0, 0.38),
        (2.44, 0.18),
        (3.0, 0.08),
    ),
    envelope_profile=ADSREnvelope(0.003, 0.700, 0.06, 0.200),
    family="pitched percussion",
)
KALIMBA_NAIVE = InstrumentProfile(
    id="kalimba_naive",
    display_name="Naive Kalimba",
    synthesis_kind="additive",
    gain=0.63,
    harmonic_profile=_partials((1.0, 1.00), (2.0, 0.22), (3.0, 0.08), (6.0, 0.18)),
    envelope_profile=ADSREnvelope(0.002, 0.450, 0.04, 0.150),
    family="pitched percussion",
)
SYNTH_LEAD_NAIVE = InstrumentProfile(
    id="synth_lead_naive",
    display_name="Naive Synth Lead",
    synthesis_kind="additive",
    gain=0.58,
    harmonic_profile=_partials(
        (1.0, 1.00),
        (2.0, 0.85),
        (3.0, 0.65),
        (4.0, 0.48),
        (5.0, 0.36),
        (6.0, 0.24),
    ),
    envelope_profile=ADSREnvelope(0.004, 0.040, 0.80, 0.040),
    family="synth lead",
)
SYNTH_PAD_NAIVE = InstrumentProfile(
    id="synth_pad_naive",
    display_name="Naive Synth Pad",
    synthesis_kind="additive",
    gain=0.46,
    harmonic_profile=_partials((1.0, 1.00), (2.0, 0.35), (3.0, 0.18), (5.0, 0.10)),
    envelope_profile=ADSREnvelope(0.450, 0.400, 0.75, 0.550),
    family="synth pad",
)
NOISE_EFFECT_NAIVE = InstrumentProfile(
    id="effect_naive",
    display_name="Naive Effect",
    synthesis_kind="additive",
    gain=0.45,
    harmonic_profile=_partials((1.0, 1.00), (1.5, 0.55), (2.25, 0.35), (3.7, 0.18)),
    envelope_profile=ADSREnvelope(0.020, 0.700, 0.20, 0.250),
    family="sound effects",
)

_PRESETS: dict[str, InstrumentProfile] = {
    profile.id: profile
    for profile in (
        SINE,
        SILENCE,
        FLUTE_NAIVE,
        CLARINET_NAIVE,
        VIOLIN_NAIVE,
        PIANO_NAIVE,
        PLUCK_NAIVE,
        BRASS_NAIVE,
        ORGAN_NAIVE,
        MALLET_NAIVE,
        CELESTA_NAIVE,
        GLOCKENSPIEL_NAIVE,
        VIBRAPHONE_NAIVE,
        MARIMBA_NAIVE,
        XYLOPHONE_NAIVE,
        TUBULAR_BELLS_NAIVE,
        TIMPANI_NAIVE,
        KALIMBA_NAIVE,
        SYNTH_LEAD_NAIVE,
        SYNTH_PAD_NAIVE,
        NOISE_EFFECT_NAIVE,
    )
}

_GM_PROGRAM_ROWS: tuple[tuple[int, str, str, str], ...] = (
    (1, "Acoustic Grand Piano", "piano", "piano_naive"),
    (2, "Bright Acoustic Piano", "piano", "piano_naive"),
    (3, "Electric Grand Piano", "piano", "piano_naive"),
    (4, "Honky-tonk Piano", "piano", "piano_naive"),
    (5, "Electric Piano 1", "piano", "piano_naive"),
    (6, "Electric Piano 2", "piano", "piano_naive"),
    (7, "Harpsichord", "piano", "pluck_naive"),
    (8, "Clavinet", "piano", "pluck_naive"),
    (9, "Celesta", "chromatic percussion", "celesta_naive"),
    (10, "Glockenspiel", "chromatic percussion", "glockenspiel_naive"),
    (11, "Music Box", "chromatic percussion", "mallet_naive"),
    (12, "Vibraphone", "chromatic percussion", "vibraphone_naive"),
    (13, "Marimba", "chromatic percussion", "marimba_naive"),
    (14, "Xylophone", "chromatic percussion", "xylophone_naive"),
    (15, "Tubular Bells", "chromatic percussion", "tubular_bells_naive"),
    (16, "Dulcimer", "chromatic percussion", "pluck_naive"),
    (17, "Drawbar Organ", "organ", "organ_naive"),
    (18, "Percussive Organ", "organ", "organ_naive"),
    (19, "Rock Organ", "organ", "organ_naive"),
    (20, "Church Organ", "organ", "organ_naive"),
    (21, "Reed Organ", "organ", "organ_naive"),
    (22, "Accordion", "organ", "organ_naive"),
    (23, "Harmonica", "organ", "organ_naive"),
    (24, "Tango Accordion", "organ", "organ_naive"),
    (25, "Acoustic Guitar (nylon)", "guitar", "pluck_naive"),
    (26, "Acoustic Guitar (steel)", "guitar", "pluck_naive"),
    (27, "Electric Guitar (jazz)", "guitar", "pluck_naive"),
    (28, "Electric Guitar (clean)", "guitar", "pluck_naive"),
    (29, "Electric Guitar (muted)", "guitar", "pluck_naive"),
    (30, "Overdriven Guitar", "guitar", "synth_lead_naive"),
    (31, "Distortion Guitar", "guitar", "synth_lead_naive"),
    (32, "Guitar Harmonics", "guitar", "pluck_naive"),
    (33, "Acoustic Bass", "bass", "pluck_naive"),
    (34, "Electric Bass (finger)", "bass", "pluck_naive"),
    (35, "Electric Bass (pick)", "bass", "pluck_naive"),
    (36, "Fretless Bass", "bass", "violin_naive"),
    (37, "Slap Bass 1", "bass", "pluck_naive"),
    (38, "Slap Bass 2", "bass", "pluck_naive"),
    (39, "Synth Bass 1", "bass", "synth_lead_naive"),
    (40, "Synth Bass 2", "bass", "synth_lead_naive"),
    (41, "Violin", "strings", "violin_naive"),
    (42, "Viola", "strings", "violin_naive"),
    (43, "Cello", "strings", "violin_naive"),
    (44, "Contrabass", "strings", "violin_naive"),
    (45, "Tremolo Strings", "strings", "violin_naive"),
    (46, "Pizzicato Strings", "strings", "pluck_naive"),
    (47, "Orchestral Harp", "strings", "pluck_naive"),
    (48, "Timpani", "strings", "timpani_naive"),
    (49, "String Ensemble 1", "ensemble", "violin_naive"),
    (50, "String Ensemble 2", "ensemble", "violin_naive"),
    (51, "SynthStrings 1", "ensemble", "synth_pad_naive"),
    (52, "SynthStrings 2", "ensemble", "synth_pad_naive"),
    (53, "Choir Aahs", "ensemble", "synth_pad_naive"),
    (54, "Voice Oohs", "ensemble", "synth_pad_naive"),
    (55, "Synth Voice", "ensemble", "synth_pad_naive"),
    (56, "Orchestra Hit", "ensemble", "brass_naive"),
    (57, "Trumpet", "brass", "brass_naive"),
    (58, "Trombone", "brass", "brass_naive"),
    (59, "Tuba", "brass", "brass_naive"),
    (60, "Muted Trumpet", "brass", "brass_naive"),
    (61, "French Horn", "brass", "brass_naive"),
    (62, "Brass Section", "brass", "brass_naive"),
    (63, "SynthBrass 1", "brass", "synth_lead_naive"),
    (64, "SynthBrass 2", "brass", "synth_lead_naive"),
    (65, "Soprano Sax", "reed", "clarinet_naive"),
    (66, "Alto Sax", "reed", "clarinet_naive"),
    (67, "Tenor Sax", "reed", "clarinet_naive"),
    (68, "Baritone Sax", "reed", "clarinet_naive"),
    (69, "Oboe", "reed", "clarinet_naive"),
    (70, "English Horn", "reed", "clarinet_naive"),
    (71, "Bassoon", "reed", "clarinet_naive"),
    (72, "Clarinet", "reed", "clarinet_naive"),
    (73, "Piccolo", "pipe", "flute_naive"),
    (74, "Flute", "pipe", "flute_naive"),
    (75, "Recorder", "pipe", "flute_naive"),
    (76, "Pan Flute", "pipe", "flute_naive"),
    (77, "Blown Bottle", "pipe", "flute_naive"),
    (78, "Shakuhachi", "pipe", "flute_naive"),
    (79, "Whistle", "pipe", "sine"),
    (80, "Ocarina", "pipe", "flute_naive"),
    (81, "Lead 1 (square)", "synth lead", "synth_lead_naive"),
    (82, "Lead 2 (sawtooth)", "synth lead", "synth_lead_naive"),
    (83, "Lead 3 (calliope)", "synth lead", "synth_lead_naive"),
    (84, "Lead 4 (chiff)", "synth lead", "synth_lead_naive"),
    (85, "Lead 5 (charang)", "synth lead", "synth_lead_naive"),
    (86, "Lead 6 (voice)", "synth lead", "synth_pad_naive"),
    (87, "Lead 7 (fifths)", "synth lead", "synth_lead_naive"),
    (88, "Lead 8 (bass + lead)", "synth lead", "synth_lead_naive"),
    (89, "Pad 1 (new age)", "synth pad", "synth_pad_naive"),
    (90, "Pad 2 (warm)", "synth pad", "synth_pad_naive"),
    (91, "Pad 3 (polysynth)", "synth pad", "synth_pad_naive"),
    (92, "Pad 4 (choir)", "synth pad", "synth_pad_naive"),
    (93, "Pad 5 (bowed)", "synth pad", "synth_pad_naive"),
    (94, "Pad 6 (metallic)", "synth pad", "synth_pad_naive"),
    (95, "Pad 7 (halo)", "synth pad", "synth_pad_naive"),
    (96, "Pad 8 (sweep)", "synth pad", "synth_pad_naive"),
    (97, "FX 1 (rain)", "synth effects", "effect_naive"),
    (98, "FX 2 (soundtrack)", "synth effects", "effect_naive"),
    (99, "FX 3 (crystal)", "synth effects", "effect_naive"),
    (100, "FX 4 (atmosphere)", "synth effects", "effect_naive"),
    (101, "FX 5 (brightness)", "synth effects", "effect_naive"),
    (102, "FX 6 (goblins)", "synth effects", "effect_naive"),
    (103, "FX 7 (echoes)", "synth effects", "effect_naive"),
    (104, "FX 8 (sci-fi)", "synth effects", "effect_naive"),
    (105, "Sitar", "ethnic", "pluck_naive"),
    (106, "Banjo", "ethnic", "pluck_naive"),
    (107, "Shamisen", "ethnic", "pluck_naive"),
    (108, "Koto", "ethnic", "pluck_naive"),
    (109, "Kalimba", "ethnic", "kalimba_naive"),
    (110, "Bagpipe", "ethnic", "clarinet_naive"),
    (111, "Fiddle", "ethnic", "violin_naive"),
    (112, "Shanai", "ethnic", "clarinet_naive"),
    (113, "Tinkle Bell", "percussive", "glockenspiel_naive"),
    (114, "Agogo", "percussive", "glockenspiel_naive"),
    (115, "Steel Drums", "percussive", "vibraphone_naive"),
    (116, "Woodblock", "percussive", "mallet_naive"),
    (117, "Taiko Drum", "percussive", "mallet_naive"),
    (118, "Melodic Tom", "percussive", "timpani_naive"),
    (119, "Synth Drum", "percussive", "effect_naive"),
    (120, "Reverse Cymbal", "percussive", "effect_naive"),
    (121, "Guitar Fret Noise", "sound effects", "effect_naive"),
    (122, "Breath Noise", "sound effects", "effect_naive"),
    (123, "Seashore", "sound effects", "effect_naive"),
    (124, "Bird Tweet", "sound effects", "effect_naive"),
    (125, "Telephone Ring", "sound effects", "effect_naive"),
    (126, "Helicopter", "sound effects", "effect_naive"),
    (127, "Applause", "sound effects", "effect_naive"),
    (128, "Gunshot", "sound effects", "effect_naive"),
)

_GM_PROGRAMS: tuple[GMProgram, ...] = tuple(GMProgram(*row) for row in _GM_PROGRAM_ROWS)
_GM_BY_PROGRAM: dict[int, GMProgram] = {entry.program: entry for entry in _GM_PROGRAMS}


def all_instruments() -> tuple[InstrumentProfile, ...]:
    """Return the built-in naive instrument profiles."""
    return tuple(_PRESETS.values())


def all_gm_programs() -> tuple[GMProgram, ...]:
    """Return the full 128-entry GM-style melodic program catalog."""
    return _GM_PROGRAMS


def get_instrument(instrument: str | InstrumentProfile) -> InstrumentProfile:
    """Resolve a built-in profile id or return an already-built profile."""
    if isinstance(instrument, InstrumentProfile):
        return instrument
    if not isinstance(instrument, str):
        raise ValueError("instrument must be an id or InstrumentProfile")
    try:
        return _PRESETS[instrument]
    except KeyError as exc:
        raise ValueError(f"unknown instrument {instrument!r}") from exc


def get_gm_program(program: int) -> GMProgram:
    """Return one GM-style program entry by 1-based program number."""
    checked = _positive_int("program", program)
    try:
        return _GM_BY_PROGRAM[checked]
    except KeyError as exc:
        raise ValueError("program must be in [1, 128]") from exc


def instrument_for_gm_program(program: int) -> InstrumentProfile:
    """Return the naive profile currently assigned to a GM-style program."""
    return get_instrument(get_gm_program(program).instrument_id)


# =============================================================================
# § 10. Note Rendering Pipeline (MUS05 §10)
# =============================================================================
# Renders a musical note event into floating-point audio samples and 16-bit PCM.
# Enforces sample budgets and frequency/duration bounds before allocating.


def _validate_render_budget(
    duration_seconds: float,
    sample_rate_hz: float,
    max_sample_count: int,
) -> None:
    """Ensure synthesis does not exceed allowed sample limits (CWE-400)."""
    sample_limit = _positive_int("max_sample_count", max_sample_count)
    if sample_limit > MAX_SAMPLE_COUNT:
        raise ValueError(
            f"max_sample_count {sample_limit} exceeds "
            f"MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )
    count = _safe_round_sample(duration_seconds, sample_rate_hz)
    if count > sample_limit:
        raise ValueError(
            f"render would create {count} samples, "
            f"above max_sample_count={sample_limit}"
        )


def render_instrument_note(
    note: NoteInput,
    duration_seconds: Real,
    *,
    instrument: str | InstrumentProfile = "sine",
    sample_rate_hz: Real = DEFAULT_SAMPLE_RATE_HZ,
    amplitude: Real = 1.0,
    start_time_seconds: Real = 0.0,
    variation_profile: VariationProfile | None = None,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
    max_partial_count: int = DEFAULT_MAX_PARTIAL_COUNT,
) -> InstrumentNoteRender:
    """Render one note with a selectable naive instrument profile."""
    parsed_note = _parse_note_input(note)
    note_duration = _non_negative_float("duration_seconds", duration_seconds)
    if note_duration > MAX_NOTE_DURATION_SECONDS:
        raise ValueError(
            f"duration_seconds {note_duration} exceeds "
            f"MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
        )

    sample_rate = _sample_rate("sample_rate_hz", sample_rate_hz)
    profile = get_instrument(instrument)

    partial_limit = _positive_int("max_partial_count", max_partial_count)
    if partial_limit > MAX_PARTIALS:
        raise ValueError(
            f"max_partial_count {partial_limit} exceeds MAX_PARTIALS={MAX_PARTIALS}"
        )
    if len(profile.harmonic_profile) > partial_limit:
        raise ValueError(
            f"instrument has {len(profile.harmonic_profile)} partials, "
            f"above max_partial_count={partial_limit}"
        )

    fundamental_hz = _positive_float("fundamental_hz", parsed_note.frequency())
    nyquist_hz = sample_rate / 2.0
    if fundamental_hz >= nyquist_hz:
        raise ValueError(
            f"fundamental_hz {fundamental_hz} must be below Nyquist {nyquist_hz}"
        )

    if variation_profile is not None and not isinstance(
        variation_profile, VariationProfile
    ):
        raise ValueError("variation_profile must be a VariationProfile or None")

    rendered_duration = profile.envelope_profile.rendered_duration_seconds(
        note_duration
    )
    _validate_render_budget(rendered_duration, sample_rate, max_sample_count)

    signal = InstrumentSignal(
        fundamental_hz=fundamental_hz,
        instrument=profile,
        note_duration_seconds=note_duration,
        amplitude=_unit_float("amplitude", amplitude),
        sample_rate_hz=sample_rate,
        variation_profile=variation_profile,
    )
    start_time = _finite_float("start_time_seconds", start_time_seconds)
    samples = UniformSampler(sample_rate).sample(
        signal,
        rendered_duration,
        start_time,
    )
    pcm_format = PCMFormat(sample_rate_hz=sample_rate)
    pcm_buffer = encode_sample_buffer(samples, pcm_format)

    return InstrumentNoteRender(
        note=parsed_note,
        fundamental_hz=fundamental_hz,
        instrument=profile,
        signal=signal,
        floating_samples=samples,
        pcm_buffer=pcm_buffer,
    )
