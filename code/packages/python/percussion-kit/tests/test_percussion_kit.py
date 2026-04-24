from __future__ import annotations

from math import nan

import pytest
from pcm_audio import PCMBuffer

from percussion_kit import (
    DEFAULT_MAX_MODE_COUNT,
    DrumKitProfile,
    PercussionEnvelope,
    PercussionMode,
    PercussionNoiseProfile,
    PercussionSignal,
    PercussionVoiceProfile,
    all_drum_kits,
    all_standard_hit_ids,
    get_drum_kit,
    get_percussion_voice,
    render_percussion_hit,
    standard_kit,
)


def test_standard_kit_has_expected_hit_ids() -> None:
    assert all_standard_hit_ids() == (
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
    )


def test_all_kits_contains_standard_kit() -> None:
    kits = all_drum_kits()

    assert len(kits) == 1
    assert kits[0].id == "standard_kit"
    assert standard_kit().display_name == "Standard Kit"


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
        rendered.pcm_buffer.sample_count()
        == rendered.floating_samples.sample_count()
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
        get_percussion_voice("cowbell")
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
