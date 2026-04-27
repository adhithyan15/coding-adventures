"""Deterministic naive percussion voices for drum-kit style rendering."""

from __future__ import annotations

from dataclasses import dataclass
from math import exp, floor, isfinite
from numbers import Integral, Real

from oscillator import SampleBuffer, UniformSampler, sample_count_for_duration
from pcm_audio import DEFAULT_SAMPLE_RATE_HZ, PCMBuffer, PCMFormat, encode_sample_buffer
from trig import PI, sin

TWO_PI = 2.0 * PI
DEFAULT_MAX_MODE_COUNT = 32
DEFAULT_MAX_SAMPLE_COUNT = 10_000_000


def _finite_float(name: str, value: Real) -> float:
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")
    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")
    return converted


def _non_negative_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted < 0.0:
        raise ValueError(f"{name} must be >= 0.0, got {converted}")
    return converted


def _positive_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted <= 0.0:
        raise ValueError(f"{name} must be > 0.0, got {converted}")
    return converted


def _unit_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if not 0.0 <= converted <= 1.0:
        raise ValueError(f"{name} must be in [0.0, 1.0], got {converted}")
    return converted


def _positive_int(name: str, value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer > 0, got {value!r}")
    converted = int(value)
    if converted <= 0:
        raise ValueError(f"{name} must be > 0, got {converted}")
    return converted


def _fractional_part(value: float) -> float:
    return value - floor(value)


def _pseudo_noise(sample_index: int, seed: int) -> float:
    phase = (sample_index + 1) * (seed * 0.161_803_398_874_989_5 + 0.618_033_988_75)
    return 2.0 * _fractional_part(sin(TWO_PI * phase) * 43_758.545_312_3) - 1.0


@dataclass(frozen=True)
class PercussionMode:
    """One decaying resonant mode inside a percussion voice."""

    frequency_hz: float
    amplitude: float
    decay_seconds: float
    phase_cycles: float = 0.0

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "frequency_hz",
            _positive_float("frequency_hz", self.frequency_hz),
        )
        object.__setattr__(
            self,
            "amplitude",
            _non_negative_float("amplitude", self.amplitude),
        )
        object.__setattr__(
            self,
            "decay_seconds",
            _positive_float("decay_seconds", self.decay_seconds),
        )
        object.__setattr__(
            self,
            "phase_cycles",
            _finite_float("phase_cycles", self.phase_cycles),
        )


@dataclass(frozen=True)
class PercussionNoiseProfile:
    """A deterministic decaying noise burst used for snare, hats, and cymbals."""

    amplitude: float
    decay_seconds: float
    seed: int = 1

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "amplitude",
            _non_negative_float("amplitude", self.amplitude),
        )
        object.__setattr__(
            self,
            "decay_seconds",
            _positive_float("decay_seconds", self.decay_seconds),
        )
        if isinstance(self.seed, bool) or not isinstance(self.seed, Integral):
            raise ValueError("seed must be an integer")
        object.__setattr__(self, "seed", int(self.seed))


@dataclass(frozen=True)
class PercussionEnvelope:
    """Fast percussion amplitude shape: attack, hold, then exponential-style tail."""

    attack_seconds: float
    hold_seconds: float
    decay_seconds: float

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "attack_seconds",
            _non_negative_float("attack_seconds", self.attack_seconds),
        )
        object.__setattr__(
            self,
            "hold_seconds",
            _non_negative_float("hold_seconds", self.hold_seconds),
        )
        object.__setattr__(
            self,
            "decay_seconds",
            _positive_float("decay_seconds", self.decay_seconds),
        )

    def rendered_duration_seconds(self, hit_duration_seconds: Real) -> float:
        return _non_negative_float("hit_duration_seconds", hit_duration_seconds) + (
            self.decay_seconds
        )

    def value_at(self, time_seconds: Real, hit_duration_seconds: Real) -> float:
        time = _non_negative_float("time_seconds", time_seconds)
        hit_duration = _non_negative_float("hit_duration_seconds", hit_duration_seconds)
        if time < self.attack_seconds:
            if self.attack_seconds == 0.0:
                return 1.0
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
    """A deterministic naive unpitched percussion voice."""

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
        object.__setattr__(self, "gain", _unit_float("gain", self.gain))
        object.__setattr__(self, "modes", tuple(self.modes))
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
        object.__setattr__(
            self,
            "click_amplitude",
            _non_negative_float("click_amplitude", self.click_amplitude),
        )
        if self.choke_group is not None:
            object.__setattr__(self, "choke_group", str(self.choke_group))

    def amplitude_normalizer(self) -> float:
        total = self.click_amplitude
        total += sum(mode.amplitude for mode in self.modes)
        if self.noise_profile is not None:
            total += self.noise_profile.amplitude
        return total if total > 0.0 else 1.0


@dataclass(frozen=True)
class DrumKitProfile:
    """A named percussion kit mapping hit ids to voice profiles."""

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
    """Continuous signal for one percussion hit."""

    voice: PercussionVoiceProfile
    hit_duration_seconds: float
    amplitude: float = 1.0
    sample_rate_hz: float = DEFAULT_SAMPLE_RATE_HZ

    def __post_init__(self) -> None:
        if not isinstance(self.voice, PercussionVoiceProfile):
            raise ValueError("voice must be a PercussionVoiceProfile")
        object.__setattr__(
            self,
            "hit_duration_seconds",
            _non_negative_float("hit_duration_seconds", self.hit_duration_seconds),
        )
        object.__setattr__(self, "amplitude", _unit_float("amplitude", self.amplitude))
        object.__setattr__(
            self,
            "sample_rate_hz",
            _positive_float("sample_rate_hz", self.sample_rate_hz),
        )

    def value_at(self, time_seconds: float) -> float:
        time = _non_negative_float("time_seconds", time_seconds)
        envelope = self.voice.envelope.value_at(time, self.hit_duration_seconds)
        if envelope <= 0.0:
            return 0.0

        sample_index = int(round(time * self.sample_rate_hz))

        raw = 0.0
        if self.voice.click_amplitude > 0.0 and time < 0.002:
            raw += self.voice.click_amplitude * (1.0 - time / 0.002)

        for mode in self.voice.modes:
            raw += mode.amplitude * exp(-time / mode.decay_seconds) * sin(
                TWO_PI * (mode.frequency_hz * time + mode.phase_cycles)
            )

        if self.voice.noise_profile is not None:
            raw += self.voice.noise_profile.amplitude * exp(
                -time / self.voice.noise_profile.decay_seconds
            ) * _pseudo_noise(sample_index, self.voice.noise_profile.seed)

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


def _modes(*values: tuple[float, float, float]) -> tuple[PercussionMode, ...]:
    return tuple(
        PercussionMode(
            frequency_hz=frequency_hz,
            amplitude=amplitude,
            decay_seconds=decay_seconds,
        )
        for frequency_hz, amplitude, decay_seconds in values
    )


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
    },
)

_KITS = {STANDARD_KIT.id: STANDARD_KIT}


def standard_kit() -> DrumKitProfile:
    """Return the built-in standard drum kit."""

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
    """Render one unpitched percussion hit through a deterministic kit voice."""

    voice = get_percussion_voice(hit_id, kit=kit)
    mode_limit = _positive_int("max_mode_count", max_mode_count)
    if len(voice.modes) > mode_limit:
        raise ValueError(
            f"voice has {len(voice.modes)} modes, above max_mode_count={mode_limit}"
        )

    hit_duration = _non_negative_float("duration_seconds", duration_seconds)
    sample_rate = _positive_float("sample_rate_hz", sample_rate_hz)
    rendered_duration = voice.envelope.rendered_duration_seconds(hit_duration)
    _validate_render_budget(rendered_duration, sample_rate, max_sample_count)

    signal = PercussionSignal(
        voice=voice,
        hit_duration_seconds=hit_duration,
        amplitude=_unit_float("amplitude", amplitude),
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
