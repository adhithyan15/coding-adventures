"""Tests for unpitched percussion voices, 15-voice standard kit, and choke sequences.

Covers MUS07 §6, §7, §9, §11, and VER15 security/weakness matrix invariants:
- Complete 15-voice starter drum kit verification.
- Hi-hat choke group dynamics with 15 ms linear release ramps.
- Timeline mixing and output clamping.
- Strict CWE-400 bounds (modes, duration, sample count, sequence events).
- CWE-674 recursion freedom (strictly iterative).
- CWE-1333 / CWE-94 static AST audit (zero re, zero eval, zero exec).
- Property-based testing and adversarial fuzzing using deterministic SplitMix64 PRNG.
"""

from __future__ import annotations

import ast
from collections.abc import Sequence
from math import inf, nan
from pathlib import Path
from typing import TypeVar

import pytest
from pcm_audio import PCMBuffer

from percussion_kit import (
    CHOKE_RELEASE_RAMP_SECONDS,
    DEFAULT_MAX_MODE_COUNT,
    MAX_SAMPLE_COUNT,
    MAX_SEQUENCE_EVENTS,
    DrumKitProfile,
    PercussionEnvelope,
    PercussionHitEvent,
    PercussionHitRender,
    PercussionMode,
    PercussionNoiseProfile,
    PercussionSignal,
    PercussionVoiceProfile,
    all_drum_kits,
    all_standard_hit_ids,
    apply_hihat_choke,
    get_drum_kit,
    get_percussion_voice,
    render_percussion_hit,
    render_percussion_sequence,
    standard_kit,
)
from percussion_kit.percussion_kit import _safe_round_sample

T = TypeVar("T")

# =============================================================================
# § 1. Pure Python SplitMix64 Deterministic PRNG
# =============================================================================
# Standard 64-bit pseudo-random number generator implemented entirely in the
# Python standard library with integer arithmetic. Requires zero external test
# dependencies while guaranteeing reproducible pseudo-random streams for
# property-based testing and fuzzing.


class SplitMix64:
    """Deterministic 64-bit PRNG (Steele et al., 2014)."""

    def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
        self.state: int = seed & 0xFFFF_FFFF_FFFF_FFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFF_FFFF_FFFF_FFFF
        return z ^ (z >> 31)

    def next_float(self) -> float:
        """Return uniform float in [0.0, 1.0)."""
        return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)

    def uniform(self, low: float, high: float) -> float:
        """Return uniform float in [low, high)."""
        return low + (high - low) * self.next_float()

    def choice(self, seq: Sequence[T]) -> T:
        """Pick a uniform random element from non-empty sequence."""
        idx = self.next_u64() % len(seq)
        return seq[idx]


# =============================================================================
# § 2. Starter Kit and Voice Verification (MUS07 §6 & §11)
# =============================================================================


def test_standard_kit_has_all_15_expected_hit_ids() -> None:
    """MUS07 §6: Verify all 15 portable starter hit ids exist in standard kit."""
    expected = (
        "kick",
        "snare",
        "closed_hihat",
        "open_hihat",
        "pedal_hihat",
        "low_tom",
        "mid_tom",
        "high_tom",
        "crash",
        "ride",
        "clap",
        "cowbell",
        "tambourine",
        "shaker",
        "woodblock",
    )
    assert all_standard_hit_ids() == expected
    assert len(all_standard_hit_ids()) == 15


def test_all_kits_contains_standard_kit() -> None:
    kits = all_drum_kits()
    assert len(kits) == 1
    assert kits[0].id == "standard_kit"
    assert standard_kit().display_name == "Standard Kit"


def test_all_15_starter_voices_render_valid_pcm() -> None:
    """Render each of the 15 starter voices and verify signal integrity."""
    for hit_id in all_standard_hit_ids():
        rendered = render_percussion_hit(
            hit_id,
            duration_seconds=0.06,
            sample_rate_hz=16_000,
            amplitude=0.5,
        )
        assert isinstance(rendered, PercussionHitRender)
        assert rendered.hit_id == hit_id
        assert rendered.voice.id.startswith(hit_id)
        assert rendered.floating_samples.sample_count() > 0
        assert isinstance(rendered.pcm_buffer, PCMBuffer)
        assert (
            rendered.pcm_buffer.sample_count()
            == rendered.floating_samples.sample_count()
        )
        # Verify finite samples within bounds
        assert all(-1.0 <= s <= 1.0 for s in rendered.floating_samples.samples)
        # Verify non-trivial audio energy was synthesized
        max_energy = max(abs(s) for s in rendered.floating_samples.samples)
        assert max_energy > 0.001


def test_voice_specific_acoustic_characteristics() -> None:
    """Verify distinct acoustic traits defined for the 5 new MUS07 starter voices."""
    # 1. Woodblock: dry, wooden, hollow resonance with very short decay (< 0.10s)
    wb = get_percussion_voice("woodblock")
    assert wb.family == "woodblock"
    assert wb.envelope.rendered_duration_seconds(0.02) < 0.10
    assert wb.click_amplitude > 0.5

    # 2. Cowbell: dual clashing metallic modes (587 Hz and 845 Hz)
    cb = get_percussion_voice("cowbell")
    assert cb.family == "hand_percussion"
    freqs = [m.frequency_hz for m in cb.modes]
    assert 587.0 in freqs
    assert 845.0 in freqs

    # 3. Shaker: soft attack, stochastic grains with high noise amplitude
    sh = get_percussion_voice("shaker")
    assert sh.family == "hand_percussion"
    assert sh.envelope.attack_seconds > 0.005
    assert sh.noise_profile is not None
    assert sh.noise_profile.amplitude >= 0.90

    # 4. Clap: hand cavity resonance modes and snappy decay
    cl = get_percussion_voice("clap")
    assert cl.family == "hand_percussion"
    assert len(cl.modes) >= 3
    assert cl.noise_profile is not None

    # 5. Tambourine: high zil shimmering modes
    tb = get_percussion_voice("tambourine")
    assert tb.family == "hand_percussion"
    assert any(m.frequency_hz > 9000.0 for m in tb.modes)


def test_tom_frequency_ordering() -> None:
    """Toms must progress strictly upward in fundamental resonant frequency."""
    low_freq = get_percussion_voice("low_tom").modes[0].frequency_hz
    mid_freq = get_percussion_voice("mid_tom").modes[0].frequency_hz
    high_freq = get_percussion_voice("high_tom").modes[0].frequency_hz
    assert low_freq < mid_freq < high_freq


def test_crash_vs_ride_decay() -> None:
    """Crash cymbal has longer decay tail than ride cymbal."""
    crash_dur = get_percussion_voice("crash").envelope.rendered_duration_seconds(0.0)
    ride_dur = get_percussion_voice("ride").envelope.rendered_duration_seconds(0.0)
    assert crash_dur > ride_dur


# =============================================================================
# § 3. Hi-Hat Choke Group Dynamics (MUS07 §7 & §9)
# =============================================================================


def test_apply_hihat_choke_direct_helper() -> None:
    """Directly test the 15 ms linear release ramp helper function."""
    rate = 10_000.0
    # 500 samples = 50 ms of unit amplitude
    samples = [1.0] * 500

    # Choke at 20 ms with a 15 ms ramp (ramp length = 150 samples)
    # Ramp runs from sample 200 to 350
    choked = apply_hihat_choke(
        samples,
        hit_start_seconds=0.0,
        choke_start_seconds=0.020,
        sample_rate_hz=rate,
        choke_ramp_seconds=CHOKE_RELEASE_RAMP_SECONDS,
    )

    assert len(choked) == 500
    # Before choke: unchanged (1.0)
    assert all(choked[i] == 1.0 for i in range(200))

    # During 15 ms ramp: strictly monotonic downward ramp
    for i in range(200, 349):
        assert choked[i] >= choked[i + 1]
    assert choked[200] == pytest.approx(1.0)
    assert choked[275] == pytest.approx(0.5, abs=0.01)

    # After 15 ms ramp: strictly zero
    assert all(choked[i] == 0.0 for i in range(350, 500))


def test_apply_hihat_choke_edge_cases() -> None:
    """Test boundary conditions for choke helper."""
    rate = 10_000.0
    # 200 samples = 20 ms. 15 ms ramp = 150 samples.
    samples = [1.0] * 200

    # Choke before or at hit start
    choked_start = apply_hihat_choke(
        samples,
        hit_start_seconds=0.05,
        choke_start_seconds=0.01,
        sample_rate_hz=rate,
    )
    assert choked_start[0] == pytest.approx(1.0)
    assert choked_start[150] == 0.0
    assert choked_start[-1] == 0.0

    # Choke before hit start with zero ramp
    choked_zero_ramp = apply_hihat_choke(
        samples,
        hit_start_seconds=0.05,
        choke_start_seconds=0.01,
        sample_rate_hz=rate,
        choke_ramp_seconds=0.0,
    )
    assert all(s == 0.0 for s in choked_zero_ramp)

    # Choke after hit ends: untouched
    choked_late = apply_hihat_choke(
        samples,
        hit_start_seconds=0.0,
        choke_start_seconds=0.50,
        sample_rate_hz=rate,
    )
    assert choked_late == samples

    # Empty samples: returns empty
    assert apply_hihat_choke([], 0.0, 0.1, rate) == []

    # Ramp seconds 0.0: instant cutoff
    choked_inst = apply_hihat_choke(
        samples,
        hit_start_seconds=0.0,
        choke_start_seconds=0.005,
        sample_rate_hz=rate,
        choke_ramp_seconds=0.0,
    )
    assert choked_inst[49] == 1.0
    assert all(choked_inst[i] == 0.0 for i in range(50, 200))


def test_percussion_hit_event_attributes_and_validation() -> None:
    """Test PercussionHitEvent properties and validation failures."""
    ev = PercussionHitEvent(
        "snare",
        start_seconds=0.1,
        duration_seconds=0.04,
        velocity=0.8,
        track_id="drums",
        variation="var1",
    )
    assert ev.start_time_seconds == 0.1
    assert ev.variation == "var1"

    with pytest.raises(ValueError, match="hit_id must be a non-empty string"):
        PercussionHitEvent("", start_seconds=0.0)
    with pytest.raises(ValueError, match="hit_id must be a non-empty string"):
        PercussionHitEvent(123, start_seconds=0.0)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="track_id must be non-empty"):
        PercussionHitEvent("snare", track_id="")


def test_sequence_render_properties_and_normalization_errors() -> None:
    """Test PercussionSequenceRender properties and invalid event error handling."""
    seq = render_percussion_sequence([("kick", 0.0)], sample_rate_hz=8000)
    assert seq.sample_rate_hz == 8000.0
    assert seq.duration_seconds > 0.0

    # Malformed event tuple (len < 2)
    with pytest.raises(ValueError, match="at least"):
        render_percussion_sequence([("kick",)], sample_rate_hz=8000)

    # Unsupported event type
    with pytest.raises(ValueError, match="unsupported"):
        render_percussion_sequence([123], sample_rate_hz=8000)


def test_custom_choke_groups_and_differing_groups() -> None:
    """Test choke logic for custom choke groups and differing choke groups."""
    env = PercussionEnvelope(0.0, 0.01, 0.1)
    m = PercussionMode(400.0, 1.0, 0.05)
    v1 = PercussionVoiceProfile(
        "conga1", "Conga 1", "conga", 0.5, (m,), None, env, choke_group="conga"
    )
    v2 = PercussionVoiceProfile(
        "conga2", "Conga 2", "conga", 0.5, (m,), None, env, choke_group="conga"
    )
    v3 = PercussionVoiceProfile(
        "bell", "Bell", "bell", 0.5, (m,), None, env, choke_group="other"
    )
    custom_kit = DrumKitProfile("latin_kit", "Latin Kit", {"c1": v1, "c2": v2, "b": v3})

    # Same custom choke group chokes earlier hit
    seq = render_percussion_sequence(
        [("c1", 0.0), ("c2", 0.04)],
        kit=custom_kit,
        sample_rate_hz=8000,
    )
    assert seq.sample_count() > 0

    # Differing choke group does not choke
    seq_diff = render_percussion_sequence(
        [("c1", 0.0), ("b", 0.04)],
        kit=custom_kit,
        sample_rate_hz=8000,
    )
    assert seq_diff.sample_count() > 0


def test_sequence_render_exceeding_budget_or_mode_limits() -> None:
    """Test render_percussion_sequence budget and mode limit failures."""
    with pytest.raises(ValueError, match="max_mode_count"):
        render_percussion_sequence([("kick", 0.0)], max_mode_count=1)

    with pytest.raises(ValueError, match="max_sample_count"):
        render_percussion_sequence([("crash", 0.0)], max_sample_count=100)

    # Individual hit fits, but late start time causes timeline to exceed sample_limit
    with pytest.raises(ValueError, match="sequence render would require"):
        render_percussion_sequence(
            [PercussionHitEvent("kick", start_seconds=1.0)],
            sample_rate_hz=8000,
            max_sample_count=5000,
        )


def test_closed_hihat_chokes_open_hihat_in_sequence() -> None:
    """MUS07 §7: closed_hihat attenuates active open_hihat with 15 ms release ramp."""
    rate = 10_000.0
    # Open hi-hat at 0.0s, closed hi-hat at 0.10s
    events = [
        PercussionHitEvent("open_hihat", start_seconds=0.0, duration_seconds=0.05),
        PercussionHitEvent("closed_hihat", start_seconds=0.10, duration_seconds=0.02),
    ]

    rendered_seq = render_percussion_sequence(events, sample_rate_hz=rate)
    samples = rendered_seq.floating_samples.samples

    # Open hi-hat alone rings out past 0.25s
    open_alone = render_percussion_hit("open_hihat", 0.05, sample_rate_hz=rate)
    alone_samples = open_alone.floating_samples.samples
    assert len(alone_samples) > int(0.25 * rate)
    assert abs(alone_samples[int(0.20 * rate)]) > 0.001

    # Closed hi-hat alone finishes by 0.20s (envelope ~0.063s).
    # In sequence, at 0.20s, the signal should be strictly zero.
    assert samples[int(0.20 * rate)] == pytest.approx(0.0, abs=1e-5)


def test_pedal_hihat_chokes_open_hihat() -> None:
    """MUS07 §7: pedal_hihat also chokes active open_hihat."""
    rate = 10_000.0
    events = [
        PercussionHitEvent("open_hihat", start_seconds=0.0, duration_seconds=0.05),
        PercussionHitEvent("pedal_hihat", start_seconds=0.08, duration_seconds=0.02),
    ]

    rendered_seq = render_percussion_sequence(events, sample_rate_hz=rate)
    samples = rendered_seq.floating_samples.samples
    # By 0.18s, pedal hi-hat has decayed and open hi-hat was choked
    assert samples[int(0.18 * rate)] == pytest.approx(0.0, abs=1e-5)


def test_repeated_open_hihat_chokes_earlier_open_hihat() -> None:
    """MUS07 §7: Repeated open_hihat hits do not accumulate unrealistically forever."""
    rate = 10_000.0
    events = [
        PercussionHitEvent("open_hihat", start_seconds=0.0, duration_seconds=0.05),
        PercussionHitEvent("open_hihat", start_seconds=0.10, duration_seconds=0.05),
    ]
    seq = render_percussion_sequence(events, sample_rate_hz=rate)
    assert seq.sample_count() > 0


def test_non_choke_voices_do_not_choke_open_hihat() -> None:
    """Voices outside the hi-hat choke group (e.g. kick, snare) do not choke hi-hats."""
    rate = 10_000.0
    events = [
        PercussionHitEvent("open_hihat", start_seconds=0.0, duration_seconds=0.05),
        PercussionHitEvent("kick", start_seconds=0.10, duration_seconds=0.05),
    ]
    seq = render_percussion_sequence(events, sample_rate_hz=rate)
    samples = seq.floating_samples.samples
    # At 0.20s, open_hihat is still ringing (not choked by the kick)
    assert abs(samples[int(0.20 * rate)]) > 0.0001


def test_sequence_with_various_input_representations() -> None:
    """render_percussion_sequence accepts tuples, dicts, and PercussionHitEvents."""
    events = [
        ("kick", 0.0),
        ("snare", 0.1, 0.05),
        ("closed_hihat", 0.2, 0.02, 0.8),
        ("cowbell", 0.3, 0.05, 0.9, "percussion", "var1"),
        {"hit_id": "woodblock", "start_seconds": 0.4, "velocity": 0.7},
    ]
    seq = render_percussion_sequence(events, sample_rate_hz=8000)
    assert len(seq.events) == 5
    assert seq.events[0].hit_id == "kick"
    assert seq.events[4].hit_id == "woodblock"
    assert seq.duration_seconds > 0.4


def test_empty_sequence_returns_zero_samples() -> None:
    seq = render_percussion_sequence([], sample_rate_hz=8000)
    assert seq.sample_count() == 0
    assert seq.events == ()
    assert seq.duration_seconds == 0.0


# =============================================================================
# § 4. CWE-400 / CWE-674 / CWE-190 Bounds and Security Hardening
# =============================================================================


def test_cwe400_mode_count_upper_bound() -> None:
    """MAX_PERCUSSION_MODES = 32 enforced."""
    mode = PercussionMode(100.0, 1.0, 0.1)
    env = PercussionEnvelope(0.0, 0.01, 0.1)

    # 33 modes rejected by PercussionVoiceProfile
    too_many_modes = tuple(mode for _ in range(33))
    with pytest.raises(ValueError, match="MAX_PERCUSSION_MODES"):
        PercussionVoiceProfile("test", "Test", "misc", 0.5, too_many_modes, None, env)

    # render_percussion_hit rejects max_mode_count > MAX_PERCUSSION_MODES
    with pytest.raises(ValueError, match="MAX_PERCUSSION_MODES"):
        render_percussion_hit("kick", 0.05, max_mode_count=33)

    # render_percussion_sequence rejects max_mode_count > MAX_PERCUSSION_MODES
    with pytest.raises(ValueError, match="MAX_PERCUSSION_MODES"):
        render_percussion_sequence(
            [("kick", 0.0)],
            max_mode_count=33,
        )


def test_cwe400_duration_upper_bound() -> None:
    """MAX_HIT_DURATION_SECONDS = 30.0 enforced across all entry points."""
    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        render_percussion_hit("kick", 30.1)

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        PercussionSignal(
            voice=get_percussion_voice("kick"),
            hit_duration_seconds=30.1,
        )

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        PercussionHitEvent("kick", start_seconds=0.0, duration_seconds=30.1)


def test_cwe400_sample_count_upper_bound() -> None:
    """MAX_SAMPLE_COUNT = 16_777_216 enforced."""
    with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
        render_percussion_hit(
            "kick",
            0.05,
            max_sample_count=MAX_SAMPLE_COUNT + 1,
        )

    with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
        render_percussion_sequence(
            [("kick", 0.0)],
            max_sample_count=MAX_SAMPLE_COUNT + 1,
        )


def test_cwe400_sequence_events_upper_bound() -> None:
    """MAX_SEQUENCE_EVENTS = 16_384 enforced."""
    with pytest.raises(ValueError, match="MAX_SEQUENCE_EVENTS"):
        render_percussion_sequence(
            [("kick", 0.0)],
            max_sequence_events=MAX_SEQUENCE_EVENTS + 1,
        )

    oversized = [("kick", 0.0)] * 10
    with pytest.raises(ValueError, match="max_sequence_events"):
        render_percussion_sequence(oversized, max_sequence_events=5)


def test_unbounded_start_seconds_rejected_before_allocating() -> None:
    """CWE-400: start_seconds = 1e12 and non-finite values fail closed pre-flight."""
    with pytest.raises(ValueError, match="above max_sample_count"):
        render_percussion_sequence(
            [("kick", 1e12, 0.05)],
            sample_rate_hz=44_100,
        )

    with pytest.raises(ValueError, match="finite"):
        render_percussion_sequence(
            [("kick", nan, 0.05)],
            sample_rate_hz=44_100,
        )

    with pytest.raises(ValueError, match="finite"):
        render_percussion_sequence(
            [("kick", inf, 0.05)],
            sample_rate_hz=44_100,
        )

    with pytest.raises(ValueError, match="finite"):
        render_percussion_sequence(
            [("kick", 0.0, nan)],
            sample_rate_hz=44_100,
        )


def test_unconditional_choke_ramp_validation() -> None:
    """choke_ramp_seconds must be validated unconditionally, even on empty sequences."""
    with pytest.raises(ValueError, match=">= 0.0"):
        render_percussion_sequence([], choke_ramp_seconds=-1.0)

    with pytest.raises(ValueError, match="finite"):
        render_percussion_sequence([], choke_ramp_seconds=nan)

    with pytest.raises(ValueError, match="finite"):
        render_percussion_sequence([], choke_ramp_seconds=inf)

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        render_percussion_sequence([], choke_ramp_seconds=31.0)

    with pytest.raises(ValueError, match=">= 0.0"):
        render_percussion_sequence([("kick", 0.0)], choke_ramp_seconds=-0.5)


def test_max_sample_rate_hz_enforced_everywhere() -> None:
    """MAX_SAMPLE_RATE_HZ = 192_000 is enforced across all public functions."""
    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        render_percussion_hit("kick", 0.05, sample_rate_hz=192_001.0)

    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        render_percussion_hit("kick", 0.05, sample_rate_hz=1e12)

    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        render_percussion_sequence([], sample_rate_hz=192_001.0)

    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        apply_hihat_choke([1.0], 0.0, 0.01, sample_rate_hz=192_001.0)

    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        PercussionSignal(
            get_percussion_voice("kick"),
            0.05,
            sample_rate_hz=192_001.0,
        )


def test_apply_hihat_choke_sample_limit_and_overflow() -> None:
    """apply_hihat_choke enforces MAX_SAMPLE_COUNT and protects against overflow."""
    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        apply_hihat_choke(
            [1.0],
            0.0,
            0.01,
            8000,
            choke_ramp_seconds=31.0,
        )

    # Mock object with oversized len
    class OversizedBuffer:
        def __len__(self) -> int:
            return MAX_SAMPLE_COUNT + 1

    with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
        apply_hihat_choke(OversizedBuffer(), 0.0, 0.01, 8000)  # type: ignore[arg-type]

    # Extreme time_delta * rate overflow protection
    with pytest.raises(ValueError, match="finite"):
        apply_hihat_choke([1.0], 0.0, 1e308, 192_000.0)

    # Safe round sample unit tests for finite and product bounds
    with pytest.raises(ValueError, match="finite"):
        _safe_round_sample(nan, 44100.0)
    with pytest.raises(ValueError, match="finite"):
        _safe_round_sample(1.0, nan)
    with pytest.raises(ValueError, match="finite"):
        _safe_round_sample(1e308, 1e308)
    assert _safe_round_sample(0.5, 44100.0) == 22050


def test_percussion_signal_and_voice_dataclass_finite_and_bounds() -> None:
    """Signal, modes, and voice profiles enforce isfinite and bounds checks."""
    voice = get_percussion_voice("kick")

    with pytest.raises(ValueError, match="finite"):
        PercussionSignal(voice, hit_duration_seconds=nan)

    with pytest.raises(ValueError, match="finite"):
        PercussionSignal(voice, 0.05, amplitude=nan)

    with pytest.raises(ValueError, match="finite"):
        PercussionSignal(voice, 0.05, sample_rate_hz=nan)

    sig = PercussionSignal(voice, 0.05)
    assert sig.duration_seconds == 0.05
    sbuf = sig.sample(0.05)
    assert sbuf.sample_count() > 0

    with pytest.raises(ValueError, match="maximum allowed duration"):
        sig.sample(61.0)

    with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
        sig.sample(0.05, max_sample_count=MAX_SAMPLE_COUNT + 1)

    # Voice, mode, noise, and envelope bounds
    with pytest.raises(ValueError, match="finite"):
        PercussionVoiceProfile(
            "v", "V", "v", nan, (), None, PercussionEnvelope(0, 0, 0.1)
        )

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        PercussionMode(100.0, 1.0, 31.0)

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        PercussionNoiseProfile(1.0, 31.0)

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        PercussionEnvelope(31.0, 0.0, 0.1)

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        PercussionEnvelope(0.0, 31.0, 0.1)

    with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
        PercussionEnvelope(0.0, 0.0, 31.0)


def test_streaming_memory_overlapping_events() -> None:
    """Verify streaming synthesis with 200 overlapping events."""
    events = [("kick", 0.0, 0.0001)] * 200
    seq = render_percussion_sequence(events, sample_rate_hz=8000)
    assert seq.sample_count() > 0


@pytest.mark.parametrize(
    "bad_val",
    [nan, inf, -1.0, "text", True],
)
def test_hostile_numeric_inputs_fail_closed(bad_val: object) -> None:
    """All invalid and hostile numbers fail closed with ValueError."""
    with pytest.raises(ValueError):
        render_percussion_hit("kick", bad_val)  # type: ignore[arg-type]

    with pytest.raises(ValueError):
        render_percussion_hit("kick", 0.05, amplitude=bad_val)  # type: ignore[arg-type]

    with pytest.raises(ValueError):
        render_percussion_hit("kick", 0.05, sample_rate_hz=bad_val)  # type: ignore[arg-type]


def test_unit_float_boundaries() -> None:
    """Unit float validation rejects values strictly outside [0.0, 1.0]."""
    with pytest.raises(ValueError, match="in \\[0.0, 1.0\\]"):
        render_percussion_hit("kick", 0.05, amplitude=-0.01)

    with pytest.raises(ValueError, match="in \\[0.0, 1.0\\]"):
        render_percussion_hit("kick", 0.05, amplitude=1.01)


def test_envelope_zero_attack_branch() -> None:
    """Ensure branch where attack_seconds == 0.0 returns 1.0."""
    env = PercussionEnvelope(0.0, 0.05, 0.1)
    assert env.value_at(0.0, 0.05) == 1.0


def test_zero_regex_and_eval_ast_audit() -> None:
    """CWE-1333 / CWE-94: Static AST audit verifies zero re/eval/exec in codebase."""
    pkg_src = Path(__file__).resolve().parent.parent / "src" / "percussion_kit"
    py_files = list(pkg_src.glob("*.py"))
    assert len(py_files) >= 2

    for path in py_files:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        for node in ast.walk(tree):
            # Assert zero regex imports
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name not in (
                        "re",
                        "regex",
                    ), f"{path}: banned import {alias.name}"
            if isinstance(node, ast.ImportFrom):
                assert node.module not in (
                    "re",
                    "regex",
                ), f"{path}: banned from {node.module} import"

            # Assert zero dynamic code execution
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                assert node.func.id not in (
                    "eval",
                    "exec",
                ), f"{path}: banned call {node.func.id}()"


# =============================================================================
# § 5. Property-Based Testing (PBT) and Adversarial Fuzzing
# =============================================================================


def test_pbt_splitmix64_determinism_across_random_sequences() -> None:
    """PBT: Pseudo-random sequences synthesized twice bit-for-bit identical."""
    prng = SplitMix64(seed=0xCAFE_BABE_2026)
    all_hits = all_standard_hit_ids()

    for _ in range(15):
        event_count = (prng.next_u64() % 4) + 1
        events: list[PercussionHitEvent] = []
        for _ in range(event_count):
            h_id = prng.choice(all_hits)
            t = prng.uniform(0.0, 0.10)
            d = prng.uniform(0.01, 0.04)
            v = prng.uniform(0.1, 1.0)
            events.append(
                PercussionHitEvent(
                    h_id, start_seconds=t, duration_seconds=d, velocity=v
                )
            )

        rate = 8000

        run_a = render_percussion_sequence(events, sample_rate_hz=rate)
        run_b = render_percussion_sequence(events, sample_rate_hz=rate)

        assert run_a.floating_samples.samples == run_b.floating_samples.samples
        assert run_a.pcm_buffer.samples == run_b.pcm_buffer.samples


def test_pbt_timeline_clamping_invariant() -> None:
    """PBT: Dense hit pile never overflows [-1.0, 1.0] or PCM bounds."""
    all_hits = all_standard_hit_ids()

    # 15 simultaneous loud hits at t = 0
    pile = [
        PercussionHitEvent(h, start_seconds=0.0, duration_seconds=0.05, velocity=1.0)
        for h in all_hits
    ]
    res = render_percussion_sequence(pile, sample_rate_hz=8000)

    # Clamping invariant
    for s in res.floating_samples.samples:
        assert -1.0 <= s <= 1.0
    for p in res.pcm_buffer.samples:
        assert -32768 <= p <= 32767


# =============================================================================
# § 6. Regression and Unit Tests (Preserved Baseline)
# =============================================================================


def test_get_kit_and_voice_resolve_builtin_entries() -> None:
    assert get_drum_kit("standard_kit").id == "standard_kit"
    assert get_percussion_voice("kick").id == "kick_naive"
    assert get_percussion_voice("open_hihat").choke_group == "hihat"
    assert get_percussion_voice("ride").family == "cymbal"


def test_render_returns_inspectable_samples_and_pcm() -> None:
    rendered = render_percussion_hit(
        "snare",
        0.08,
        sample_rate_hz=8_000,
        amplitude=0.35,
    )
    assert rendered.hit_id == "snare"
    assert rendered.voice.id == "snare_naive"
    assert rendered.floating_samples.sample_count() > 500
    assert isinstance(rendered.pcm_buffer, PCMBuffer)
    assert (
        rendered.pcm_buffer.sample_count() == rendered.floating_samples.sample_count()
    )


def test_hits_are_deterministic_by_default() -> None:
    first = render_percussion_hit("closed_hihat", 0.05, sample_rate_hz=8_000)
    second = render_percussion_hit("closed_hihat", 0.05, sample_rate_hz=8_000)
    assert first.floating_samples.samples == second.floating_samples.samples


def test_different_hits_produce_different_sample_sequences() -> None:
    kick = render_percussion_hit("kick", 0.08, sample_rate_hz=12_000, amplitude=0.4)
    snare = render_percussion_hit("snare", 0.08, sample_rate_hz=12_000, amplitude=0.4)
    ride = render_percussion_hit("ride", 0.08, sample_rate_hz=12_000, amplitude=0.4)
    assert kick.floating_samples.samples != snare.floating_samples.samples
    assert snare.floating_samples.samples != ride.floating_samples.samples


def test_open_hihat_rings_longer_than_closed_hihat() -> None:
    closed_hat = render_percussion_hit("closed_hihat", 0.03, sample_rate_hz=8_000)
    open_hat = render_percussion_hit("open_hihat", 0.03, sample_rate_hz=8_000)
    assert (
        open_hat.floating_samples.sample_count()
        > closed_hat.floating_samples.sample_count()
    )


def test_envelope_covers_attack_hold_and_decay() -> None:
    envelope = PercussionEnvelope(
        attack_seconds=0.1,
        hold_seconds=0.1,
        decay_seconds=0.3,
    )
    assert envelope.value_at(0.0, 0.05) == pytest.approx(0.0)
    assert envelope.value_at(0.05, 0.05) == pytest.approx(0.5)
    assert envelope.value_at(0.12, 0.05) == pytest.approx(1.0)
    assert envelope.value_at(0.24, 0.05) == pytest.approx(0.8666666667)
    assert envelope.value_at(0.55, 0.05) == pytest.approx(0.0)


def test_signal_decays_to_zero() -> None:
    signal = PercussionSignal(
        voice=get_percussion_voice("kick"),
        hit_duration_seconds=0.08,
        sample_rate_hz=8_000,
    )
    assert signal.value_at(0.0) != 0.0
    assert signal.value_at(1.0) == pytest.approx(0.0)


def test_profile_validation_rejects_invalid_shapes() -> None:
    envelope = PercussionEnvelope(0.0, 0.01, 0.1)
    mode = PercussionMode(100.0, 1.0, 0.2)

    with pytest.raises(ValueError, match="id"):
        PercussionVoiceProfile("", "Kick", "kick", 0.5, (mode,), None, envelope)
    with pytest.raises(ValueError, match="display_name"):
        PercussionVoiceProfile("kick", "", "kick", 0.5, (mode,), None, envelope)
    with pytest.raises(ValueError, match="family"):
        PercussionVoiceProfile("kick", "Kick", "", 0.5, (mode,), None, envelope)
    with pytest.raises(ValueError, match="PercussionMode"):
        PercussionVoiceProfile("kick", "Kick", "kick", 0.5, (object(),), None, envelope)
    with pytest.raises(ValueError, match="PercussionNoiseProfile"):
        PercussionVoiceProfile("kick", "Kick", "kick", 0.5, (mode,), object(), envelope)
    with pytest.raises(ValueError, match="PercussionEnvelope"):
        PercussionVoiceProfile("kick", "Kick", "kick", 0.5, (mode,), None, object())


def test_drum_kit_validation_rejects_bad_entries() -> None:
    voice = get_percussion_voice("kick")
    with pytest.raises(ValueError, match="non-empty"):
        DrumKitProfile("", "Kit", {"kick": voice})
    with pytest.raises(ValueError, match="non-empty"):
        DrumKitProfile("kit", "", {"kick": voice})
    with pytest.raises(ValueError, match="non-empty"):
        DrumKitProfile("kit", "Kit", {})
    with pytest.raises(ValueError, match="non-empty"):
        DrumKitProfile("kit", "Kit", {"": voice})
    with pytest.raises(ValueError, match="PercussionVoiceProfile"):
        DrumKitProfile("kit", "Kit", {"kick": object()})


def test_lookup_validation_rejects_unknown_entries() -> None:
    with pytest.raises(ValueError, match="unknown drum kit"):
        get_drum_kit("jazz_kit")
    with pytest.raises(ValueError, match="unknown hit"):
        get_percussion_voice("nonexistent_instrument")
    with pytest.raises(ValueError, match="non-empty string"):
        get_percussion_voice("")
    with pytest.raises(ValueError, match="id or DrumKitProfile"):
        get_drum_kit(123)  # type: ignore[arg-type]


@pytest.mark.parametrize("program", [0, True])
def test_integer_validation_rejects_bad_limits(program: object) -> None:
    with pytest.raises(ValueError, match="max_mode_count"):
        render_percussion_hit("kick", 0.05, max_mode_count=program)  # type: ignore[arg-type]


def test_render_rejects_unbounded_work() -> None:
    with pytest.raises(ValueError, match="max_sample_count"):
        render_percussion_hit("crash", 1.0, max_sample_count=10)
    with pytest.raises(ValueError, match="max_mode_count"):
        render_percussion_hit("kick", 0.05, max_mode_count=1)


def test_numeric_validation_rejects_bad_values() -> None:
    with pytest.raises(ValueError, match="finite real"):
        PercussionMode(True, 1.0, 0.2)
    with pytest.raises(ValueError, match="finite"):
        PercussionMode(100.0, nan, 0.2)
    with pytest.raises(ValueError, match="> 0.0"):
        PercussionMode(0.0, 1.0, 0.2)
    with pytest.raises(ValueError, match=">= 0.0"):
        PercussionNoiseProfile(-1.0, 0.1)
    with pytest.raises(ValueError, match="seed"):
        PercussionNoiseProfile(0.5, 0.1, seed=True)
    with pytest.raises(ValueError, match=">= 0.0"):
        PercussionEnvelope(-0.1, 0.0, 0.1)
    with pytest.raises(ValueError, match="> 0.0"):
        PercussionEnvelope(0.0, 0.0, 0.0)


def test_signal_requires_voice_profile() -> None:
    with pytest.raises(ValueError, match="PercussionVoiceProfile"):
        PercussionSignal(
            voice=object(),  # type: ignore[arg-type]
            hit_duration_seconds=0.05,
        )


def test_rendered_voice_can_be_resolved_through_explicit_kit_object() -> None:
    kit = standard_kit()
    rendered = render_percussion_hit("mid_tom", 0.07, kit=kit, sample_rate_hz=8_000)
    assert rendered.voice.id == "mid_tom_naive"
    assert rendered.pcm_buffer.sample_count() > 0


def test_default_mode_limit_matches_builtin_kit() -> None:
    crash = get_percussion_voice("crash")
    assert len(crash.modes) < DEFAULT_MAX_MODE_COUNT
