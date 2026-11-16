"""Deterministic naive percussion voices and sequences for drum-kit style rendering.

This module implements MUS07: Unpitched Percussion and Drum Kit Events.
It provides:
- The complete 15-voice starter drum kit (MUS07 §6 & §11).
- Single-hit deterministic synthesis with resonant modes, pseudo-noise, and envelopes.
- Multi-event sequence rendering with hi-hat choke group dynamics (MUS07 §7 & §9).
- Hardened CWE-400 / CWE-674 resource exhaustion and recursion bounds.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from math import exp, floor, isfinite
from numbers import Integral, Real

from oscillator import SampleBuffer, UniformSampler, sample_count_for_duration
from pcm_audio import DEFAULT_SAMPLE_RATE_HZ, PCMBuffer, PCMFormat, encode_sample_buffer
from trig import PI, sin

# =============================================================================
# § 1. Mathematical Constants and Security Bounds (VER15 / CWE-400 / CWE-674)
# =============================================================================
# In accordance with CWE-400 (Uncontrolled Resource Consumption), all memory
# allocations, mode counts, time windows, and sample buffers are bounded by
# explicit upper ceilings. All synthesis routines are strictly iterative,
# ensuring provable resistance to CWE-674 (Uncontrolled Recursion).
# Zero regular expression or dynamic evaluation libraries are imported,
# eliminating CWE-1333 (ReDoS) and CWE-94 (Code Injection).

TWO_PI: float = 2.0 * PI

MAX_PERCUSSION_MODES: int = 32
MAX_HIT_DURATION_SECONDS: float = 30.0
MAX_SAMPLE_RATE_HZ: float = 192_000.0
MAX_SAMPLE_COUNT: int = 16_777_216
MAX_SEQUENCE_EVENTS: int = 16_384
CHOKE_RELEASE_RAMP_SECONDS: float = 0.015

DEFAULT_MAX_MODE_COUNT: int = MAX_PERCUSSION_MODES
DEFAULT_MAX_SAMPLE_COUNT: int = 10_000_000


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


def _fractional_part(value: float) -> float:
    """Return the fractional part of a floating-point value: x - floor(x)."""
    return value - floor(value)


def _pseudo_noise(sample_index: int, seed: int) -> float:
    """Deterministic hash-based pseudo-random noise sample in [-1.0, 1.0].

    Uses the golden ratio phase offset and sine fraction folding to generate
    statistically uniform noise without allocating state or using external PRNGs.
    """
    phase = (sample_index + 1) * (seed * 0.161_803_398_874_989_5 + 0.618_033_988_75)
    # trig.sin is NaN beyond 2**29 radians (PHY00a 3.4.1) and this phase grows
    # without bound (seed 1e9 is past it at sample 0).  The sine has period 1 in
    # cycles, so keep only the fractional cycle first (PHY00a 7); the subtraction
    # is exact in binary64.  Python, Rust and TypeScript do exactly this.
    cycles = _fractional_part(phase)
    return 2.0 * _fractional_part(sin(TWO_PI * cycles) * 43_758.545_312_3) - 1.0


# =============================================================================
# § 3. Signal Data Models: Modes, Noise, and Envelopes (MUS07 §4)
# =============================================================================


@dataclass(frozen=True)
class PercussionMode:
    """One decaying resonant mode inside a percussion voice (MUS07 §4.2).

    Represents a physical resonance of a drum shell, membrane, zil, or wooden body.
    Unlike pitched harmonics, percussion modes need not be integer multiples of
    a fundamental frequency.
    """

    frequency_hz: float
    amplitude: float
    decay_seconds: float
    phase_cycles: float = 0.0

    def __post_init__(self) -> None:
        freq = _positive_float("frequency_hz", self.frequency_hz)
        object.__setattr__(self, "frequency_hz", freq)

        amp = _non_negative_float("amplitude", self.amplitude)
        object.__setattr__(self, "amplitude", amp)

        decay = _positive_float("decay_seconds", self.decay_seconds)
        if decay > MAX_HIT_DURATION_SECONDS:
            raise ValueError(
                f"decay_seconds {decay} exceeds "
                f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
            )
        object.__setattr__(self, "decay_seconds", decay)

        phase = _finite_float("phase_cycles", self.phase_cycles)
        object.__setattr__(self, "phase_cycles", phase)


@dataclass(frozen=True)
class PercussionNoiseProfile:
    """Deterministic decaying noise burst (MUS07 §4.1)."""

    amplitude: float
    decay_seconds: float
    seed: int = 1

    def __post_init__(self) -> None:
        amp = _non_negative_float("amplitude", self.amplitude)
        object.__setattr__(self, "amplitude", amp)

        decay = _positive_float("decay_seconds", self.decay_seconds)
        if decay > MAX_HIT_DURATION_SECONDS:
            raise ValueError(
                f"decay_seconds {decay} exceeds "
                f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
            )
        object.__setattr__(self, "decay_seconds", decay)

        if isinstance(self.seed, bool) or not isinstance(self.seed, Integral):
            raise ValueError("seed must be an integer")
        object.__setattr__(self, "seed", int(self.seed))


@dataclass(frozen=True)
class PercussionEnvelope:
    """Fast percussion amplitude shape: attack, hold, then decay (MUS07 §4.3)."""

    attack_seconds: float
    hold_seconds: float
    decay_seconds: float

    def __post_init__(self) -> None:
        att = _non_negative_float("attack_seconds", self.attack_seconds)
        if att > MAX_HIT_DURATION_SECONDS:
            raise ValueError(
                f"attack_seconds {att} exceeds "
                f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
            )
        object.__setattr__(self, "attack_seconds", att)

        hld = _non_negative_float("hold_seconds", self.hold_seconds)
        if hld > MAX_HIT_DURATION_SECONDS:
            raise ValueError(
                f"hold_seconds {hld} exceeds "
                f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
            )
        object.__setattr__(self, "hold_seconds", hld)

        dec = _positive_float("decay_seconds", self.decay_seconds)
        if dec > MAX_HIT_DURATION_SECONDS:
            raise ValueError(
                f"decay_seconds {dec} exceeds "
                f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
            )
        object.__setattr__(self, "decay_seconds", dec)

    def rendered_duration_seconds(self, hit_duration_seconds: Real) -> float:
        """Compute the total physical decay duration in seconds."""
        return _non_negative_float("hit_duration_seconds", hit_duration_seconds) + (
            self.decay_seconds
        )

    def value_at(self, time_seconds: Real, hit_duration_seconds: Real) -> float:
        """Evaluate envelope gain in [0.0, 1.0] at a given elapsed time."""
        time = _non_negative_float("time_seconds", time_seconds)
        hit_duration = _non_negative_float("hit_duration_seconds", hit_duration_seconds)
        if time < self.attack_seconds:
            return time / self.attack_seconds

        sustain_end = self.attack_seconds + max(self.hold_seconds, hit_duration)
        if time < sustain_end:
            return 1.0

        decay_time = time - sustain_end
        if decay_time >= self.decay_seconds:
            return 0.0
        return max(0.0, 1.0 - decay_time / self.decay_seconds)


@dataclass(frozen=True)
class PercussionVoiceProfile:
    """A deterministic unpitched percussion voice profile (MUS07 §5.2)."""

    id: str
    display_name: str
    family: str
    gain: float
    modes: tuple[PercussionMode, ...]
    noise_profile: PercussionNoiseProfile | None
    envelope: PercussionEnvelope
    click_amplitude: float = 0.0
    choke_group: str | None = None

    def __post_init__(self) -> None:
        object.__setattr__(self, "id", str(self.id))
        if self.id == "":
            raise ValueError("id must be non-empty")
        object.__setattr__(self, "display_name", str(self.display_name))
        if self.display_name == "":
            raise ValueError("display_name must be non-empty")
        object.__setattr__(self, "family", str(self.family))
        if self.family == "":
            raise ValueError("family must be non-empty")

        gain = _unit_float("gain", self.gain)
        object.__setattr__(self, "gain", gain)

        modes_tuple = tuple(self.modes)
        if len(modes_tuple) > MAX_PERCUSSION_MODES:
            raise ValueError(
                f"modes count {len(modes_tuple)} exceeds "
                f"MAX_PERCUSSION_MODES={MAX_PERCUSSION_MODES}"
            )
        object.__setattr__(self, "modes", modes_tuple)
        for index, mode in enumerate(self.modes):
            if not isinstance(mode, PercussionMode):
                raise ValueError(f"modes[{index}] must be a PercussionMode")

        if self.noise_profile is not None and not isinstance(
            self.noise_profile,
            PercussionNoiseProfile,
        ):
            raise ValueError("noise_profile must be a PercussionNoiseProfile or None")
        if not isinstance(self.envelope, PercussionEnvelope):
            raise ValueError("envelope must be a PercussionEnvelope")

        click = _non_negative_float("click_amplitude", self.click_amplitude)
        object.__setattr__(self, "click_amplitude", click)

        if self.choke_group is not None:
            object.__setattr__(self, "choke_group", str(self.choke_group))

    def amplitude_normalizer(self) -> float:
        """Compute the sum of peak component amplitudes for gain normalization."""
        total = self.click_amplitude
        total += sum(mode.amplitude for mode in self.modes)
        if self.noise_profile is not None:
            total += self.noise_profile.amplitude
        return total if total > 0.0 else 1.0


@dataclass(frozen=True)
class DrumKitProfile:
    """A named percussion kit mapping hit ids to voice profiles (MUS07 §5.3)."""

    id: str
    display_name: str
    voices_by_hit_id: dict[str, PercussionVoiceProfile]

    def __post_init__(self) -> None:
        object.__setattr__(self, "id", str(self.id))
        if self.id == "":
            raise ValueError("id must be non-empty")
        object.__setattr__(self, "display_name", str(self.display_name))
        if self.display_name == "":
            raise ValueError("display_name must be non-empty")
        normalized: dict[str, PercussionVoiceProfile] = {}
        for hit_id, voice in self.voices_by_hit_id.items():
            converted_hit = str(hit_id)
            if converted_hit == "":
                raise ValueError("hit ids must be non-empty")
            if not isinstance(voice, PercussionVoiceProfile):
                raise ValueError("every kit voice must be a PercussionVoiceProfile")
            normalized[converted_hit] = voice
        if not normalized:
            raise ValueError("voices_by_hit_id must be non-empty")
        object.__setattr__(self, "voices_by_hit_id", normalized)


@dataclass(frozen=True)
class PercussionSignal:
    """Continuous signal generator for one percussion hit."""

    voice: PercussionVoiceProfile
    hit_duration_seconds: float
    amplitude: float = 1.0
    sample_rate_hz: float = DEFAULT_SAMPLE_RATE_HZ

    def __post_init__(self) -> None:
        if not isinstance(self.voice, PercussionVoiceProfile):
            raise ValueError("voice must be a PercussionVoiceProfile")
        dur = _non_negative_float("hit_duration_seconds", self.hit_duration_seconds)
        if dur > MAX_HIT_DURATION_SECONDS:
            raise ValueError(
                f"hit_duration_seconds {dur} exceeds "
                f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
            )
        object.__setattr__(self, "hit_duration_seconds", dur)

        amp = _unit_float("amplitude", self.amplitude)
        object.__setattr__(self, "amplitude", amp)

        rate = _sample_rate("sample_rate_hz", self.sample_rate_hz)
        object.__setattr__(self, "sample_rate_hz", rate)

    @property
    def duration_seconds(self) -> float:
        """Alias for hit_duration_seconds."""
        return self.hit_duration_seconds

    def sample(
        self,
        duration_seconds: Real,
        *,
        max_sample_count: int = MAX_SAMPLE_COUNT,
    ) -> SampleBuffer:
        """Sample this signal for a given duration with strict CWE-400 bounds."""
        dur = _non_negative_float("duration_seconds", duration_seconds)
        if dur > MAX_HIT_DURATION_SECONDS * 2.0:
            raise ValueError(f"duration_seconds {dur} exceeds maximum allowed duration")
        limit = _positive_int("max_sample_count", max_sample_count)
        if limit > MAX_SAMPLE_COUNT:
            raise ValueError(
                f"max_sample_count {limit} exceeds MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
            )
        _validate_render_budget(dur, self.sample_rate_hz, limit)
        return UniformSampler(self.sample_rate_hz).sample(self, dur, 0.0)

    def value_at(self, time_seconds: float) -> float:
        """Sample the composite percussion waveform at time_seconds."""
        time = _non_negative_float("time_seconds", time_seconds)
        envelope = self.voice.envelope.value_at(time, self.hit_duration_seconds)
        if envelope <= 0.0:
            return 0.0

        sample_index = _safe_round_sample(time, self.sample_rate_hz)

        raw = 0.0
        if self.voice.click_amplitude > 0.0 and time < 0.002:
            raw += self.voice.click_amplitude * (1.0 - time / 0.002)

        for mode in self.voice.modes:
            raw += (
                mode.amplitude
                * exp(-time / mode.decay_seconds)
                * sin(
                    TWO_PI
                    * _fractional_part(mode.frequency_hz * time + mode.phase_cycles)
                )
            )

        if self.voice.noise_profile is not None:
            raw += (
                self.voice.noise_profile.amplitude
                * exp(-time / self.voice.noise_profile.decay_seconds)
                * _pseudo_noise(sample_index, self.voice.noise_profile.seed)
            )

        return (
            self.amplitude
            * self.voice.gain
            * envelope
            * raw
            / self.voice.amplitude_normalizer()
        )


@dataclass(frozen=True)
class PercussionHitRender:
    """Inspectable render result for one percussion hit."""

    hit_id: str
    voice: PercussionVoiceProfile
    signal: PercussionSignal
    floating_samples: SampleBuffer
    pcm_buffer: PCMBuffer


@dataclass(frozen=True, init=False)
class PercussionHitEvent:
    """A scheduled unpitched percussion hit event in a sequence (MUS07 §5.1)."""

    hit_id: str
    start_seconds: float
    duration_seconds: float
    velocity: float
    track_id: str
    variation: str | None

    def __init__(
        self,
        hit_id: str,
        start_seconds: Real = 0.0,
        duration_seconds: Real = 0.05,
        velocity: Real = 1.0,
        track_id: str = "drums",
        variation: str | None = None,
        *,
        start_time_seconds: Real | None = None,
    ) -> None:
        if not isinstance(hit_id, str) or hit_id == "":
            raise ValueError("hit_id must be a non-empty string")
        object.__setattr__(self, "hit_id", hit_id)

        effective_start = (
            start_time_seconds if start_time_seconds is not None else start_seconds
        )
        st = _non_negative_float("start_seconds", effective_start)
        object.__setattr__(self, "start_seconds", st)

        dur = _non_negative_float("duration_seconds", duration_seconds)
        if dur > MAX_HIT_DURATION_SECONDS:
            raise ValueError(
                f"duration_seconds {dur} exceeds "
                f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
            )
        object.__setattr__(self, "duration_seconds", dur)

        vel = _unit_float("velocity", velocity)
        object.__setattr__(self, "velocity", vel)

        tid = str(track_id)
        if tid == "":
            raise ValueError("track_id must be non-empty")
        object.__setattr__(self, "track_id", tid)

        var = str(variation) if variation is not None else None
        object.__setattr__(self, "variation", var)

    @property
    def start_time_seconds(self) -> float:
        """Alias for start_seconds matching score/timeline convention."""
        return self.start_seconds


@dataclass(frozen=True)
class PercussionSequenceRender:
    """Inspectable render result for a sequence of percussion hits (MUS07 §9)."""

    events: tuple[PercussionHitEvent, ...]
    floating_samples: SampleBuffer
    pcm_buffer: PCMBuffer

    @property
    def sample_rate_hz(self) -> float:
        """Sampling rate in Hertz."""
        return self.floating_samples.sample_rate_hz

    @property
    def duration_seconds(self) -> float:
        """Total duration of the rendered sequence in seconds."""
        return self.pcm_buffer.duration_seconds()

    def sample_count(self) -> int:
        """Number of PCM / floating samples in the mixed sequence."""
        return self.floating_samples.sample_count()


# =============================================================================
# § 4. Standard Kit Starter Voice Profiles (MUS07 §6 & §11)
# =============================================================================


def _modes(*values: tuple[float, float, float]) -> tuple[PercussionMode, ...]:
    return tuple(
        PercussionMode(
            frequency_hz=frequency_hz,
            amplitude=amplitude,
            decay_seconds=decay_seconds,
        )
        for frequency_hz, amplitude, decay_seconds in values
    )


# 1. Bass drum / kick
KICK_NAIVE = PercussionVoiceProfile(
    id="kick_naive",
    display_name="Naive Kick",
    family="kick",
    gain=0.90,
    modes=_modes((58.0, 1.0, 0.20), (94.0, 0.35, 0.12), (142.0, 0.10, 0.06)),
    noise_profile=PercussionNoiseProfile(0.08, 0.015, seed=11),
    envelope=PercussionEnvelope(0.0, 0.015, 0.22),
    click_amplitude=0.55,
)

# 2. Snare drum
SNARE_NAIVE = PercussionVoiceProfile(
    id="snare_naive",
    display_name="Naive Snare",
    family="snare",
    gain=0.75,
    modes=_modes((180.0, 0.30, 0.16), (330.0, 0.15, 0.09)),
    noise_profile=PercussionNoiseProfile(0.95, 0.11, seed=23),
    envelope=PercussionEnvelope(0.0, 0.008, 0.18),
    click_amplitude=0.28,
)

# 3. Closed hi-hat
CLOSED_HIHAT_NAIVE = PercussionVoiceProfile(
    id="closed_hihat_naive",
    display_name="Naive Closed Hi-Hat",
    family="hat",
    gain=0.45,
    modes=_modes((6200.0, 0.24, 0.05), (9100.0, 0.18, 0.04)),
    noise_profile=PercussionNoiseProfile(0.92, 0.05, seed=31),
    envelope=PercussionEnvelope(0.0, 0.003, 0.06),
    click_amplitude=0.10,
    choke_group="hihat",
)

# 4. Open hi-hat
OPEN_HIHAT_NAIVE = PercussionVoiceProfile(
    id="open_hihat_naive",
    display_name="Naive Open Hi-Hat",
    family="hat",
    gain=0.42,
    modes=_modes((5900.0, 0.24, 0.20), (8700.0, 0.16, 0.24)),
    noise_profile=PercussionNoiseProfile(0.95, 0.26, seed=37),
    envelope=PercussionEnvelope(0.0, 0.004, 0.32),
    click_amplitude=0.08,
    choke_group="hihat",
)

# 5. Pedal / foot hi-hat
PEDAL_HIHAT_NAIVE = PercussionVoiceProfile(
    id="pedal_hihat_naive",
    display_name="Naive Pedal Hi-Hat",
    family="hat",
    gain=0.40,
    modes=_modes((5600.0, 0.18, 0.06), (8200.0, 0.12, 0.05)),
    noise_profile=PercussionNoiseProfile(0.78, 0.07, seed=41),
    envelope=PercussionEnvelope(0.0, 0.002, 0.08),
    click_amplitude=0.12,
    choke_group="hihat",
)

# 6. Low tom
LOW_TOM_NAIVE = PercussionVoiceProfile(
    id="low_tom_naive",
    display_name="Naive Low Tom",
    family="tom",
    gain=0.72,
    modes=_modes((110.0, 1.0, 0.22), (170.0, 0.28, 0.16), (250.0, 0.10, 0.10)),
    noise_profile=PercussionNoiseProfile(0.10, 0.03, seed=43),
    envelope=PercussionEnvelope(0.0, 0.010, 0.20),
    click_amplitude=0.20,
)

# 7. Middle tom
MID_TOM_NAIVE = PercussionVoiceProfile(
    id="mid_tom_naive",
    display_name="Naive Mid Tom",
    family="tom",
    gain=0.70,
    modes=_modes((150.0, 1.0, 0.20), (225.0, 0.25, 0.14), (320.0, 0.08, 0.08)),
    noise_profile=PercussionNoiseProfile(0.08, 0.025, seed=47),
    envelope=PercussionEnvelope(0.0, 0.008, 0.18),
    click_amplitude=0.18,
)

# 8. High tom
HIGH_TOM_NAIVE = PercussionVoiceProfile(
    id="high_tom_naive",
    display_name="Naive High Tom",
    family="tom",
    gain=0.68,
    modes=_modes((205.0, 1.0, 0.17), (295.0, 0.22, 0.12), (405.0, 0.07, 0.07)),
    noise_profile=PercussionNoiseProfile(0.06, 0.020, seed=53),
    envelope=PercussionEnvelope(0.0, 0.007, 0.15),
    click_amplitude=0.16,
)

# 9. Crash cymbal
CRASH_NAIVE = PercussionVoiceProfile(
    id="crash_naive",
    display_name="Naive Crash",
    family="cymbal",
    gain=0.38,
    modes=_modes((3200.0, 0.18, 0.35), (5100.0, 0.12, 0.48), (7900.0, 0.08, 0.55)),
    noise_profile=PercussionNoiseProfile(1.0, 0.60, seed=59),
    envelope=PercussionEnvelope(0.0, 0.004, 0.65),
    click_amplitude=0.12,
)

# 10. Ride cymbal
RIDE_NAIVE = PercussionVoiceProfile(
    id="ride_naive",
    display_name="Naive Ride",
    family="cymbal",
    gain=0.36,
    modes=_modes((2600.0, 0.14, 0.32), (4100.0, 0.10, 0.44), (6500.0, 0.08, 0.50)),
    noise_profile=PercussionNoiseProfile(0.82, 0.42, seed=61),
    envelope=PercussionEnvelope(0.0, 0.004, 0.45),
    click_amplitude=0.10,
)

# 11. Hand clap
CLAP_NAIVE = PercussionVoiceProfile(
    id="clap_naive",
    display_name="Naive Clap",
    family="hand_percussion",
    gain=0.70,
    modes=_modes((800.0, 0.40, 0.06), (1250.0, 0.35, 0.05), (2500.0, 0.20, 0.03)),
    noise_profile=PercussionNoiseProfile(0.88, 0.08, seed=67),
    envelope=PercussionEnvelope(0.0, 0.008, 0.14),
    click_amplitude=0.35,
)

# 12. Cowbell
COWBELL_NAIVE = PercussionVoiceProfile(
    id="cowbell_naive",
    display_name="Naive Cowbell",
    family="hand_percussion",
    gain=0.65,
    modes=_modes((587.0, 1.0, 0.25), (845.0, 0.70, 0.20), (1450.0, 0.15, 0.10)),
    noise_profile=PercussionNoiseProfile(0.08, 0.015, seed=71),
    envelope=PercussionEnvelope(0.0, 0.004, 0.28),
    click_amplitude=0.45,
)

# 13. Tambourine
TAMBOURINE_NAIVE = PercussionVoiceProfile(
    id="tambourine_naive",
    display_name="Naive Tambourine",
    family="hand_percussion",
    gain=0.45,
    modes=_modes((4800.0, 0.25, 0.12), (7200.0, 0.20, 0.15), (10500.0, 0.15, 0.18)),
    noise_profile=PercussionNoiseProfile(0.90, 0.16, seed=73),
    envelope=PercussionEnvelope(0.0, 0.003, 0.22),
    click_amplitude=0.22,
)

# 14. Shaker
SHAKER_NAIVE = PercussionVoiceProfile(
    id="shaker_naive",
    display_name="Naive Shaker",
    family="hand_percussion",
    gain=0.40,
    modes=_modes((3200.0, 0.08, 0.08), (6400.0, 0.06, 0.06)),
    noise_profile=PercussionNoiseProfile(0.95, 0.10, seed=79),
    envelope=PercussionEnvelope(0.010, 0.005, 0.12),
    click_amplitude=0.04,
)

# 15. Woodblock
WOODBLOCK_NAIVE = PercussionVoiceProfile(
    id="woodblock_naive",
    display_name="Naive Woodblock",
    family="woodblock",
    gain=0.72,
    modes=_modes((780.0, 1.0, 0.04), (1650.0, 0.30, 0.025), (2800.0, 0.10, 0.015)),
    noise_profile=PercussionNoiseProfile(0.05, 0.010, seed=83),
    envelope=PercussionEnvelope(0.0, 0.002, 0.055),
    click_amplitude=0.60,
)

STANDARD_KIT = DrumKitProfile(
    id="standard_kit",
    display_name="Standard Kit",
    voices_by_hit_id={
        "kick": KICK_NAIVE,
        "snare": SNARE_NAIVE,
        "closed_hihat": CLOSED_HIHAT_NAIVE,
        "open_hihat": OPEN_HIHAT_NAIVE,
        "pedal_hihat": PEDAL_HIHAT_NAIVE,
        "low_tom": LOW_TOM_NAIVE,
        "mid_tom": MID_TOM_NAIVE,
        "high_tom": HIGH_TOM_NAIVE,
        "crash": CRASH_NAIVE,
        "ride": RIDE_NAIVE,
        "clap": CLAP_NAIVE,
        "cowbell": COWBELL_NAIVE,
        "tambourine": TAMBOURINE_NAIVE,
        "shaker": SHAKER_NAIVE,
        "woodblock": WOODBLOCK_NAIVE,
    },
)

_KITS = {STANDARD_KIT.id: STANDARD_KIT}


# =============================================================================
# § 5. Kit and Voice Lookups
# =============================================================================


def standard_kit() -> DrumKitProfile:
    """Return the built-in standard drum kit containing all 15 MUS07 voices."""
    return STANDARD_KIT


def all_drum_kits() -> tuple[DrumKitProfile, ...]:
    """Return all built-in percussion kits."""
    return tuple(_KITS.values())


def all_standard_hit_ids() -> tuple[str, ...]:
    """Return the portable starter hit ids for the built-in standard kit."""
    return tuple(STANDARD_KIT.voices_by_hit_id)


def get_drum_kit(kit: str | DrumKitProfile) -> DrumKitProfile:
    """Resolve a built-in kit id or return an already-built kit profile."""
    if isinstance(kit, DrumKitProfile):
        return kit
    if not isinstance(kit, str):
        raise ValueError("kit must be an id or DrumKitProfile")
    try:
        return _KITS[kit]
    except KeyError as exc:
        raise ValueError(f"unknown drum kit {kit!r}") from exc


def get_percussion_voice(
    hit_id: str,
    *,
    kit: str | DrumKitProfile = "standard_kit",
) -> PercussionVoiceProfile:
    """Resolve one hit id through a drum kit."""
    if not isinstance(hit_id, str) or hit_id == "":
        raise ValueError("hit_id must be a non-empty string")
    active_kit = get_drum_kit(kit)
    try:
        return active_kit.voices_by_hit_id[hit_id]
    except KeyError as exc:
        raise ValueError(f"unknown hit {hit_id!r} in kit {active_kit.id!r}") from exc


def _validate_render_budget(
    duration_seconds: float,
    sample_rate_hz: float,
    max_sample_count: int,
) -> None:
    count = sample_count_for_duration(duration_seconds, sample_rate_hz)
    limit = _positive_int("max_sample_count", max_sample_count)
    if count > limit:
        raise ValueError(
            f"render would create {count} samples, above max_sample_count={limit}"
        )


# =============================================================================
# § 6. Single Hit Rendering Pipeline (MUS07 §9)
# =============================================================================


def render_percussion_hit(
    hit_id: str,
    duration_seconds: Real,
    *,
    kit: str | DrumKitProfile = "standard_kit",
    sample_rate_hz: Real = DEFAULT_SAMPLE_RATE_HZ,
    amplitude: Real = 1.0,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
    max_mode_count: int = DEFAULT_MAX_MODE_COUNT,
) -> PercussionHitRender:
    """Render one unpitched percussion hit through a deterministic kit voice.

    Enforces CWE-400 resource limits on duration, sample count, and resonant modes.
    """
    voice = get_percussion_voice(hit_id, kit=kit)
    mode_limit = _positive_int("max_mode_count", max_mode_count)
    if mode_limit > MAX_PERCUSSION_MODES:
        raise ValueError(
            f"max_mode_count {mode_limit} exceeds "
            f"MAX_PERCUSSION_MODES={MAX_PERCUSSION_MODES}"
        )
    if len(voice.modes) > mode_limit:
        raise ValueError(
            f"voice has {len(voice.modes)} modes, above max_mode_count={mode_limit}"
        )

    hit_duration = _non_negative_float("duration_seconds", duration_seconds)
    if hit_duration > MAX_HIT_DURATION_SECONDS:
        raise ValueError(
            f"duration_seconds {hit_duration} exceeds "
            f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
        )

    sample_rate = _sample_rate("sample_rate_hz", sample_rate_hz)
    sample_limit = _positive_int("max_sample_count", max_sample_count)
    if sample_limit > MAX_SAMPLE_COUNT:
        raise ValueError(
            f"max_sample_count {sample_limit} exceeds "
            f"MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )

    rendered_duration = voice.envelope.rendered_duration_seconds(hit_duration)
    _validate_render_budget(rendered_duration, sample_rate, sample_limit)

    amp = _unit_float("amplitude", amplitude)
    signal = PercussionSignal(
        voice=voice,
        hit_duration_seconds=hit_duration,
        amplitude=amp,
        sample_rate_hz=sample_rate,
    )
    samples = UniformSampler(sample_rate).sample(signal, rendered_duration, 0.0)
    pcm_format = PCMFormat(sample_rate_hz=sample_rate)
    pcm_buffer = encode_sample_buffer(samples, pcm_format)

    return PercussionHitRender(
        hit_id=hit_id,
        voice=voice,
        signal=signal,
        floating_samples=samples,
        pcm_buffer=pcm_buffer,
    )


# =============================================================================
# § 7. Hi-Hat Choke Group Dynamics (MUS07 §7)
# =============================================================================


def apply_hihat_choke(
    samples: list[float] | tuple[float, ...],
    hit_start_seconds: Real,
    choke_start_seconds: Real,
    sample_rate_hz: Real,
    *,
    choke_ramp_seconds: Real = CHOKE_RELEASE_RAMP_SECONDS,
) -> list[float]:
    """Apply a linear release ramp attenuation to a hi-hat sample buffer.

    Per MUS07 §7 and §9, when a closed or pedal hi-hat hit occurs while an
    open hi-hat is actively ringing, the open hi-hat must be attenuated
    smoothly using a 15 ms linear release ramp starting at the choke time.

    Parameters:
        samples: The un-choked sample buffer of the ringing hit.
        hit_start_seconds: The timeline timestamp when this hit began.
        choke_start_seconds: The timeline timestamp when the choking event begins.
        sample_rate_hz: Audio sample rate in Hertz.
        choke_ramp_seconds: Linear release ramp duration (default 15 ms).
    """
    total_len = len(samples)
    if total_len > MAX_SAMPLE_COUNT:
        raise ValueError(
            f"samples length {total_len} exceeds MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )

    hit_start = _non_negative_float("hit_start_seconds", hit_start_seconds)
    choke_start = _non_negative_float("choke_start_seconds", choke_start_seconds)
    rate = _sample_rate("sample_rate_hz", sample_rate_hz)
    ramp_dur = _non_negative_float("choke_ramp_seconds", choke_ramp_seconds)
    if ramp_dur > MAX_HIT_DURATION_SECONDS:
        raise ValueError(
            f"choke_ramp_seconds {ramp_dur} exceeds "
            f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
        )

    output = list(samples)
    if total_len == 0:
        return output

    if choke_start <= hit_start:
        ramp_samples = _safe_round_sample(ramp_dur, rate)
        if ramp_samples <= 0:
            return [0.0] * total_len
        for i in range(min(total_len, ramp_samples)):
            output[i] *= max(0.0, 1.0 - i / ramp_samples)
        for i in range(ramp_samples, total_len):
            output[i] = 0.0
        return output

    time_delta = choke_start - hit_start
    choke_idx = _safe_round_sample(time_delta, rate)
    if choke_idx >= total_len:
        return output

    ramp_samples = _safe_round_sample(ramp_dur, rate)
    if ramp_samples <= 0:
        for i in range(choke_idx, total_len):
            output[i] = 0.0
        return output

    ramp_end = min(total_len, choke_idx + ramp_samples)
    for i in range(choke_idx, ramp_end):
        fraction = (i - choke_idx) / ramp_samples
        output[i] *= max(0.0, min(1.0, 1.0 - fraction))

    for i in range(ramp_end, total_len):
        output[i] = 0.0

    return output


def _normalize_event(event: object) -> PercussionHitEvent:
    """Coerce supported event representations into a PercussionHitEvent."""
    if isinstance(event, PercussionHitEvent):
        return event
    if isinstance(event, (tuple, list)):
        if len(event) == 2:
            return PercussionHitEvent(hit_id=event[0], start_seconds=event[1])
        if len(event) == 3:
            return PercussionHitEvent(
                hit_id=event[0],
                start_seconds=event[1],
                duration_seconds=event[2],
            )
        if len(event) == 4:
            return PercussionHitEvent(
                hit_id=event[0],
                start_seconds=event[1],
                duration_seconds=event[2],
                velocity=event[3],
            )
        if len(event) >= 5:
            return PercussionHitEvent(
                hit_id=event[0],
                start_seconds=event[1],
                duration_seconds=event[2],
                velocity=event[3],
                track_id=event[4],
                variation=event[5] if len(event) > 5 else None,
            )
        raise ValueError(
            "event tuple/list must contain at least (hit_id, start_seconds), "
            f"got {event!r}"
        )
    if isinstance(event, dict):
        st = event.get("start_seconds", event.get("start_time_seconds", 0.0))
        vel = event.get("velocity", event.get("amplitude", 1.0))
        return PercussionHitEvent(
            hit_id=event["hit_id"],
            start_seconds=st,
            duration_seconds=event.get("duration_seconds", 0.05),
            velocity=vel,
            track_id=event.get("track_id", "drums"),
            variation=event.get("variation", None),
        )
    raise ValueError(f"unsupported percussion event type: {type(event).__name__}")


def _is_choked_by(
    earlier_voice: PercussionVoiceProfile,
    earlier_hit_id: str,
    later_voice: PercussionVoiceProfile,
    later_hit_id: str,
) -> bool:
    """Determine whether a later percussion hit chokes an active earlier hit."""
    if earlier_voice.choke_group is None or later_voice.choke_group is None:
        return False
    if earlier_voice.choke_group != later_voice.choke_group:
        return False
    if earlier_voice.choke_group == "hihat":
        # MUS07 §7: closed/pedal hihat or open hihat chokes active open hihat
        is_earlier_open = (
            earlier_hit_id == "open_hihat"
            or earlier_voice.id == "open_hihat_naive"
            or "open" in earlier_voice.id.lower()
        )
        is_later_choker = (
            later_hit_id in ("closed_hihat", "pedal_hihat", "open_hihat")
            or later_voice.id
            in ("closed_hihat_naive", "pedal_hihat_naive", "open_hihat_naive")
            or "closed" in later_voice.id.lower()
            or "pedal" in later_voice.id.lower()
            or "open" in later_voice.id.lower()
        )
        return is_earlier_open and is_later_choker
    return True


# =============================================================================
# § 8. Sequence Rendering and Timeline Mixing (MUS07 §9)
# =============================================================================


def render_percussion_sequence(
    events: Iterable[object],
    *,
    kit: str | DrumKitProfile = "standard_kit",
    sample_rate_hz: Real = DEFAULT_SAMPLE_RATE_HZ,
    choke_ramp_seconds: Real = CHOKE_RELEASE_RAMP_SECONDS,
    max_sample_count: int = MAX_SAMPLE_COUNT,
    max_sequence_events: int = MAX_SEQUENCE_EVENTS,
    max_mode_count: int = MAX_PERCUSSION_MODES,
) -> PercussionSequenceRender:
    """Render a sequence of scheduled percussion hits with choke groups (MUS07 §7 & §9).

    This strictly iterative, streaming synthesis function enforces all CWE-400
    bounds and guarantees peak working memory is strictly O(max_end_sample)
    rather than O(MAX_SEQUENCE_EVENTS * MAX_SAMPLE_COUNT).
    """
    # 1. Unconditional validation of sample rate and choke ramp parameters
    sample_rate = _sample_rate("sample_rate_hz", sample_rate_hz)
    ramp_dur = _non_negative_float("choke_ramp_seconds", choke_ramp_seconds)
    if ramp_dur > MAX_HIT_DURATION_SECONDS:
        raise ValueError(
            f"choke_ramp_seconds {ramp_dur} exceeds "
            f"MAX_HIT_DURATION_SECONDS={MAX_HIT_DURATION_SECONDS}"
        )

    event_limit = _positive_int("max_sequence_events", max_sequence_events)
    if event_limit > MAX_SEQUENCE_EVENTS:
        raise ValueError(
            f"max_sequence_events {event_limit} exceeds "
            f"MAX_SEQUENCE_EVENTS={MAX_SEQUENCE_EVENTS}"
        )
    sample_limit = _positive_int("max_sample_count", max_sample_count)
    if sample_limit > MAX_SAMPLE_COUNT:
        raise ValueError(
            f"max_sample_count {sample_limit} exceeds "
            f"MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )
    mode_limit = _positive_int("max_mode_count", max_mode_count)
    if mode_limit > MAX_PERCUSSION_MODES:
        raise ValueError(
            f"max_mode_count {mode_limit} exceeds "
            f"MAX_PERCUSSION_MODES={MAX_PERCUSSION_MODES}"
        )

    # 2. Normalize events and check sequence count
    normalized_events: list[PercussionHitEvent] = []
    for ev in events:
        normalized_events.append(_normalize_event(ev))
        if len(normalized_events) > event_limit:
            raise ValueError(f"event count exceeds max_sequence_events={event_limit}")

    pcm_format = PCMFormat(sample_rate_hz=sample_rate)
    if not normalized_events:
        empty_buf = SampleBuffer(sample_rate_hz=sample_rate, samples=())
        empty_pcm = encode_sample_buffer(empty_buf, pcm_format)
        return PercussionSequenceRender(
            events=(),
            floating_samples=empty_buf,
            pcm_buffer=empty_pcm,
        )

    # 3. Pre-flight budget & choke calculation (CWE-400 finding 1)
    # Compute each event's start_sample, hit_sample_count, and earliest choke time
    # BEFORE rendering any hit and BEFORE allocating timeline array!
    active_kit = get_drum_kit(kit)
    num_events = len(normalized_events)
    event_voices: list[PercussionVoiceProfile] = []
    event_start_indices: list[int] = []
    event_rendered_durs: list[float] = []
    max_end_sample = 0

    for ev in normalized_events:
        voice = get_percussion_voice(ev.hit_id, kit=active_kit)
        if len(voice.modes) > mode_limit:
            raise ValueError(
                f"voice {voice.id!r} has {len(voice.modes)} modes, "
                f"above max_mode_count={mode_limit}"
            )
        event_voices.append(voice)

        rendered_dur = voice.envelope.rendered_duration_seconds(ev.duration_seconds)
        event_rendered_durs.append(rendered_dur)

        _validate_render_budget(rendered_dur, sample_rate, sample_limit)

        start_idx = _safe_round_sample(ev.start_seconds, sample_rate)
        event_start_indices.append(start_idx)
        hit_samples = sample_count_for_duration(rendered_dur, sample_rate)
        hit_end = start_idx + hit_samples

        if hit_end > sample_limit:
            raise ValueError(
                f"sequence render would require {hit_end} samples, "
                f"above max_sample_count={sample_limit}"
            )
        if hit_end > max_end_sample:
            max_end_sample = hit_end

    # Determine earliest choke time for each event from event metadata
    earliest_choke_times: list[float | None] = [None] * num_events
    for i in range(num_events):
        v_i = event_voices[i]
        if v_i.choke_group is None:
            continue
        ev_i = normalized_events[i]
        choke_t: float | None = None
        for j in range(num_events):
            if i == j:
                continue
            ev_j = normalized_events[j]
            if ev_j.start_seconds < ev_i.start_seconds:
                continue
            if _is_choked_by(v_i, ev_i.hit_id, event_voices[j], ev_j.hit_id) and (
                choke_t is None or ev_j.start_seconds < choke_t
            ):
                choke_t = ev_j.start_seconds
        earliest_choke_times[i] = choke_t

    # 4. Allocate timeline (guaranteed <= sample_limit <= MAX_SAMPLE_COUNT)
    timeline = [0.0] * max_end_sample

    # 5. Streaming Hit Synthesis: render and mix ONE hit at a time!
    # Discards individual hit buffers immediately after mixing into timeline,
    # ensuring peak working memory is strictly O(max_end_sample).
    for i in range(num_events):
        ev = normalized_events[i]
        voice = event_voices[i]
        rendered_dur = event_rendered_durs[i]
        st = event_start_indices[i]

        sig = PercussionSignal(
            voice=voice,
            hit_duration_seconds=ev.duration_seconds,
            amplitude=ev.velocity,
            sample_rate_hz=sample_rate,
        )
        sbuf = UniformSampler(sample_rate).sample(sig, rendered_dur, 0.0)
        hit_samples = list(sbuf.samples)

        choke_t = earliest_choke_times[i]
        if choke_t is not None:
            hit_samples = apply_hihat_choke(
                hit_samples,
                ev.start_seconds,
                choke_t,
                sample_rate,
                choke_ramp_seconds=ramp_dur,
            )

        for offset, sample_val in enumerate(hit_samples):
            timeline[st + offset] += sample_val

    # Safely clamp mixed floating point samples to [-1.0, 1.0]
    clamped = tuple(min(1.0, max(-1.0, s)) for s in timeline)
    sample_buf = SampleBuffer(sample_rate_hz=sample_rate, samples=clamped)
    pcm_buf = encode_sample_buffer(sample_buf, pcm_format)

    return PercussionSequenceRender(
        events=tuple(normalized_events),
        floating_samples=sample_buf,
        pcm_buffer=pcm_buf,
    )
