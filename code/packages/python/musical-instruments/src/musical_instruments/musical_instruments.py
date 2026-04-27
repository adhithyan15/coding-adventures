"""Naive instrument models for beginner-friendly virtual music.

The first version deliberately models instruments as:

```text
note frequency + harmonic profile + ADSR envelope -> sampled PCM
```

That is not a faithful physical simulation yet. It is a stable teaching layer
that lets a score, keyboard, or orchestra package select a named timbre while
the internals remain inspectable.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import isfinite
from numbers import Integral, Real

from note_frequency import Note, parse_note
from oscillator import SampleBuffer, UniformSampler, sample_count_for_duration
from pcm_audio import DEFAULT_SAMPLE_RATE_HZ, PCMBuffer, PCMFormat, encode_sample_buffer
from trig import PI, sin

TWO_PI = 2.0 * PI
DEFAULT_MAX_PARTIAL_COUNT = 64
DEFAULT_MAX_SAMPLE_COUNT = 10_000_000
SYNTHESIS_KINDS = frozenset({"sine", "additive", "silence"})

NoteInput = Note | str


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


def _parse_note_input(note: NoteInput) -> Note:
    if isinstance(note, Note):
        return note
    if isinstance(note, str):
        return parse_note(note)
    raise ValueError(f"note must be a note string or Note, got {note!r}")


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


@dataclass(frozen=True)
class ADSREnvelope:
    """Attack, decay, sustain, and release loudness shape."""

    attack_seconds: float
    decay_seconds: float
    sustain_level: float
    release_seconds: float

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "attack_seconds",
            _non_negative_float("attack_seconds", self.attack_seconds),
        )
        object.__setattr__(
            self,
            "decay_seconds",
            _non_negative_float("decay_seconds", self.decay_seconds),
        )
        object.__setattr__(
            self,
            "sustain_level",
            _unit_float("sustain_level", self.sustain_level),
        )
        object.__setattr__(
            self,
            "release_seconds",
            _non_negative_float("release_seconds", self.release_seconds),
        )

    def rendered_duration_seconds(self, note_duration_seconds: Real) -> float:
        return _non_negative_float("note_duration_seconds", note_duration_seconds) + (
            self.release_seconds
        )

    def value_at(self, time_seconds: Real, note_duration_seconds: Real) -> float:
        """Return the envelope gain for a rendered note-local time."""

        time = _non_negative_float("time_seconds", time_seconds)
        note_duration = _non_negative_float(
            "note_duration_seconds",
            note_duration_seconds,
        )
        if time < self.attack_seconds:
            if self.attack_seconds == 0.0:
                return 1.0
            return time / self.attack_seconds

        decay_end = self.attack_seconds + self.decay_seconds
        if time < decay_end:
            if self.decay_seconds == 0.0:
                return self.sustain_level
            progress = (time - self.attack_seconds) / self.decay_seconds
            return 1.0 + (self.sustain_level - 1.0) * progress

        if time < note_duration:
            return self.sustain_level

        release_time = time - note_duration
        if self.release_seconds == 0.0 or release_time >= self.release_seconds:
            return 0.0
        return self.sustain_level * (1.0 - release_time / self.release_seconds)


@dataclass(frozen=True)
class VariationProfile:
    """Reserved deterministic imperfection knobs for future realism."""

    pitch_jitter_cents: float = 0.0
    amplitude_jitter: float = 0.0
    timing_jitter_seconds: float = 0.0
    harmonic_jitter: float = 0.0
    seed: int | None = None

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "pitch_jitter_cents",
            _non_negative_float("pitch_jitter_cents", self.pitch_jitter_cents),
        )
        object.__setattr__(
            self,
            "amplitude_jitter",
            _non_negative_float("amplitude_jitter", self.amplitude_jitter),
        )
        object.__setattr__(
            self,
            "timing_jitter_seconds",
            _non_negative_float("timing_jitter_seconds", self.timing_jitter_seconds),
        )
        object.__setattr__(
            self,
            "harmonic_jitter",
            _non_negative_float("harmonic_jitter", self.harmonic_jitter),
        )
        if self.seed is not None:
            if isinstance(self.seed, bool) or not isinstance(self.seed, Integral):
                raise ValueError("seed must be an integer or None")
            object.__setattr__(self, "seed", int(self.seed))


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
        object.__setattr__(self, "harmonic_profile", tuple(self.harmonic_profile))
        for index, partial in enumerate(self.harmonic_profile):
            if not isinstance(partial, HarmonicPartial):
                raise ValueError(
                    f"harmonic_profile[{index}] must be a HarmonicPartial"
                )
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
        total = sum(partial.amplitude for partial in self.harmonic_profile)
        return total if total > 0.0 else 1.0


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


@dataclass(frozen=True)
class InstrumentSignal:
    """Continuous signal for one instrument playing one fundamental frequency."""

    fundamental_hz: float
    instrument: InstrumentProfile
    note_duration_seconds: float
    amplitude: float = 1.0
    sample_rate_hz: float = DEFAULT_SAMPLE_RATE_HZ

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "fundamental_hz",
            _positive_float("fundamental_hz", self.fundamental_hz),
        )
        if not isinstance(self.instrument, InstrumentProfile):
            raise ValueError("instrument must be an InstrumentProfile")
        object.__setattr__(
            self,
            "note_duration_seconds",
            _non_negative_float("note_duration_seconds", self.note_duration_seconds),
        )
        object.__setattr__(self, "amplitude", _unit_float("amplitude", self.amplitude))
        object.__setattr__(
            self,
            "sample_rate_hz",
            _positive_float("sample_rate_hz", self.sample_rate_hz),
        )

    def value_at(self, time_seconds: float) -> float:
        time = _non_negative_float("time_seconds", time_seconds)
        if self.instrument.synthesis_kind == "silence":
            return 0.0

        nyquist_hz = self.sample_rate_hz / 2.0
        raw = 0.0
        for partial in self.instrument.harmonic_profile:
            partial_hz = self.fundamental_hz * partial.frequency_multiplier
            if partial_hz >= nyquist_hz:
                continue
            raw += partial.amplitude * sin(
                TWO_PI * (partial_hz * time + partial.phase_cycles)
            )

        envelope = self.instrument.envelope_profile.value_at(
            time,
            self.note_duration_seconds,
        )
        return (
            self.amplitude
            * self.instrument.gain
            * envelope
            * raw
            / self.instrument.amplitude_normalizer()
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


_PRESETS = {
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

_GM_PROGRAMS = tuple(GMProgram(*row) for row in _GM_PROGRAM_ROWS)
_GM_BY_PROGRAM = {entry.program: entry for entry in _GM_PROGRAMS}


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


def render_instrument_note(
    note: NoteInput,
    duration_seconds: Real,
    *,
    instrument: str | InstrumentProfile = "sine",
    sample_rate_hz: Real = DEFAULT_SAMPLE_RATE_HZ,
    amplitude: Real = 1.0,
    start_time_seconds: Real = 0.0,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
    max_partial_count: int = DEFAULT_MAX_PARTIAL_COUNT,
) -> InstrumentNoteRender:
    """Render one note with a selectable naive instrument profile."""

    parsed_note = _parse_note_input(note)
    note_duration = _non_negative_float("duration_seconds", duration_seconds)
    sample_rate = _positive_float("sample_rate_hz", sample_rate_hz)
    profile = get_instrument(instrument)
    partial_limit = _positive_int("max_partial_count", max_partial_count)
    if len(profile.harmonic_profile) > partial_limit:
        raise ValueError(
            f"instrument has {len(profile.harmonic_profile)} partials, "
            f"above max_partial_count={partial_limit}"
        )

    fundamental_hz = parsed_note.frequency()
    if fundamental_hz >= sample_rate / 2.0:
        raise ValueError(
            f"fundamental_hz {fundamental_hz} must be below Nyquist "
            f"{sample_rate / 2.0}"
        )
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
    )
    samples = UniformSampler(sample_rate).sample(
        signal,
        rendered_duration,
        _finite_float("start_time_seconds", start_time_seconds),
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
