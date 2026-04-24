from __future__ import annotations

from math import nan

import pytest
from pcm_audio import PCMBuffer

from musical_instruments import (
    ADSREnvelope,
    GMProgram,
    HarmonicPartial,
    InstrumentProfile,
    InstrumentSignal,
    VariationProfile,
    all_gm_programs,
    all_instruments,
    get_gm_program,
    get_instrument,
    instrument_for_gm_program,
    render_instrument_note,
)


def test_builtin_presets_are_available() -> None:
    ids = {instrument.id for instrument in all_instruments()}

    assert {
        "sine",
        "flute_naive",
        "clarinet_naive",
        "violin_naive",
        "piano_naive",
        "pluck_naive",
        "celesta_naive",
        "glockenspiel_naive",
        "vibraphone_naive",
        "marimba_naive",
        "xylophone_naive",
        "tubular_bells_naive",
        "timpani_naive",
        "kalimba_naive",
    } <= ids


def test_general_midi_catalog_has_128_programs() -> None:
    programs = all_gm_programs()

    assert len(programs) == 128
    assert programs[0] == GMProgram(1, "Acoustic Grand Piano", "piano", "piano_naive")
    assert programs[73] == GMProgram(74, "Flute", "pipe", "flute_naive")
    assert programs[-1] == GMProgram(128, "Gunshot", "sound effects", "effect_naive")


def test_general_midi_programs_resolve_to_profiles() -> None:
    assert get_gm_program(41).name == "Violin"
    assert instrument_for_gm_program(41).id == "violin_naive"
    assert instrument_for_gm_program(10).id == "glockenspiel_naive"
    assert instrument_for_gm_program(12).id == "vibraphone_naive"
    assert instrument_for_gm_program(13).id == "marimba_naive"
    assert instrument_for_gm_program(14).id == "xylophone_naive"
    assert instrument_for_gm_program(15).id == "tubular_bells_naive"
    assert instrument_for_gm_program(48).id == "timpani_naive"
    assert instrument_for_gm_program(109).id == "kalimba_naive"
    assert instrument_for_gm_program(74).id == "flute_naive"


def test_render_returns_inspectable_samples_and_pcm() -> None:
    rendered = render_instrument_note(
        "A4",
        0.01,
        instrument="flute_naive",
        sample_rate_hz=8_000,
        amplitude=0.25,
    )

    assert rendered.note.frequency() == pytest.approx(440.0)
    assert rendered.fundamental_hz == pytest.approx(440.0)
    assert rendered.instrument.id == "flute_naive"
    assert rendered.floating_samples.sample_count() > 80
    assert isinstance(rendered.pcm_buffer, PCMBuffer)
    assert rendered.pcm_buffer.sample_count() == (
        rendered.floating_samples.sample_count()
    )


def test_instruments_change_the_sample_sequence_for_same_note() -> None:
    flute = render_instrument_note(
        "A4",
        0.02,
        instrument="flute_naive",
        sample_rate_hz=8_000,
        amplitude=0.5,
    )
    violin = render_instrument_note(
        "A4",
        0.02,
        instrument="violin_naive",
        sample_rate_hz=8_000,
        amplitude=0.5,
    )

    assert flute.fundamental_hz == pytest.approx(violin.fundamental_hz)
    assert flute.floating_samples.samples != violin.floating_samples.samples


def test_pitched_percussion_profiles_share_pitch_but_change_timbre() -> None:
    glockenspiel = render_instrument_note(
        "C5",
        0.25,
        instrument="glockenspiel_naive",
        sample_rate_hz=12_000,
        amplitude=0.4,
    )
    marimba = render_instrument_note(
        "C5",
        0.25,
        instrument="marimba_naive",
        sample_rate_hz=12_000,
        amplitude=0.4,
    )
    vibraphone = render_instrument_note(
        "C5",
        0.25,
        instrument="vibraphone_naive",
        sample_rate_hz=12_000,
        amplitude=0.4,
    )

    assert glockenspiel.fundamental_hz == pytest.approx(marimba.fundamental_hz)
    assert marimba.fundamental_hz == pytest.approx(vibraphone.fundamental_hz)
    assert glockenspiel.floating_samples.samples != marimba.floating_samples.samples
    assert marimba.floating_samples.samples != vibraphone.floating_samples.samples


def test_adsr_envelope_values_cover_main_stages() -> None:
    envelope = ADSREnvelope(
        attack_seconds=0.1,
        decay_seconds=0.1,
        sustain_level=0.4,
        release_seconds=0.2,
    )

    assert envelope.value_at(0.0, 1.0) == pytest.approx(0.0)
    assert envelope.value_at(0.05, 1.0) == pytest.approx(0.5)
    assert envelope.value_at(0.1, 1.0) == pytest.approx(1.0)
    assert envelope.value_at(0.2, 1.0) == pytest.approx(0.4)
    assert envelope.value_at(0.8, 1.0) == pytest.approx(0.4)
    assert envelope.value_at(1.1, 1.0) == pytest.approx(0.2)
    assert envelope.value_at(1.2, 1.0) == pytest.approx(0.0)


def test_zero_length_adsr_stages_are_supported() -> None:
    envelope = ADSREnvelope(
        attack_seconds=0.0,
        decay_seconds=0.0,
        sustain_level=0.25,
        release_seconds=0.0,
    )

    assert envelope.value_at(0.0, 1.0) == pytest.approx(0.25)
    assert envelope.value_at(1.1, 1.0) == pytest.approx(0.0)


def test_silence_profile_renders_zero_samples() -> None:
    rendered = render_instrument_note(
        "C4",
        0.01,
        instrument="silence",
        sample_rate_hz=8_000,
    )

    assert set(rendered.floating_samples.samples) == {0.0}
    assert set(rendered.pcm_buffer.samples) == {0}


def test_high_partials_are_skipped_above_nyquist() -> None:
    signal = InstrumentSignal(
        fundamental_hz=100.0,
        instrument=InstrumentProfile(
            id="mostly_aliasing",
            display_name="Mostly Aliasing",
            synthesis_kind="additive",
            gain=1.0,
            harmonic_profile=(
                HarmonicPartial(1.0, 1.0),
                HarmonicPartial(100.0, 1000.0),
            ),
            envelope_profile=ADSREnvelope(0.0, 0.0, 1.0, 0.0),
        ),
        note_duration_seconds=1.0,
        sample_rate_hz=1_000,
    )

    assert signal.value_at(0.0025) == pytest.approx(1.0 / 1001.0)


@pytest.mark.parametrize("program", [0, 129, True])
def test_invalid_gm_programs_fail(program: object) -> None:
    with pytest.raises(ValueError):
        get_gm_program(program)  # type: ignore[arg-type]


def test_invalid_profiles_fail_before_rendering() -> None:
    with pytest.raises(ValueError, match="partial"):
        render_instrument_note(
            "A4",
            0.01,
            instrument=InstrumentProfile(
                id="too_many",
                display_name="Too Many",
                synthesis_kind="additive",
                gain=1.0,
                harmonic_profile=tuple(
                    HarmonicPartial(index + 1, 1.0) for index in range(3)
                ),
                envelope_profile=ADSREnvelope(0.0, 0.0, 1.0, 0.0),
            ),
            max_partial_count=2,
        )


def test_profile_validation_rejects_invalid_shapes() -> None:
    envelope = ADSREnvelope(0.0, 0.0, 1.0, 0.0)
    partial = HarmonicPartial(1.0, 1.0)

    with pytest.raises(ValueError, match="id"):
        InstrumentProfile("", "No Id", "additive", 1.0, (partial,), envelope)
    with pytest.raises(ValueError, match="display_name"):
        InstrumentProfile("no_name", "", "additive", 1.0, (partial,), envelope)
    with pytest.raises(ValueError, match="synthesis_kind"):
        InstrumentProfile("bad_kind", "Bad", "sample", 1.0, (partial,), envelope)
    with pytest.raises(ValueError, match="HarmonicPartial"):
        InstrumentProfile("bad_partial", "Bad", "additive", 1.0, (object(),), envelope)
    with pytest.raises(ValueError, match="at least one partial"):
        InstrumentProfile("empty", "Empty", "additive", 1.0, (), envelope)
    with pytest.raises(ValueError, match="ADSREnvelope"):
        InstrumentProfile("bad_env", "Bad", "additive", 1.0, (partial,), object())
    with pytest.raises(ValueError, match="VariationProfile"):
        InstrumentProfile(
            "bad_var",
            "Bad",
            "additive",
            1.0,
            (partial,),
            envelope,
            variation_profile=object(),
        )
    with pytest.raises(ValueError, match="gm_program"):
        InstrumentProfile(
            "bad_gm",
            "Bad",
            "additive",
            1.0,
            (partial,),
            envelope,
            gm_program=129,
        )


def test_numeric_validation_rejects_bad_values() -> None:
    with pytest.raises(ValueError, match="finite real"):
        HarmonicPartial(True, 1.0)
    with pytest.raises(ValueError, match="finite"):
        HarmonicPartial(1.0, nan)
    with pytest.raises(ValueError, match="> 0.0"):
        HarmonicPartial(0.0, 1.0)
    with pytest.raises(ValueError, match=">= 0.0"):
        HarmonicPartial(1.0, -1.0)
    with pytest.raises(ValueError, match=r"\[0.0, 1.0\]"):
        ADSREnvelope(0.0, 0.0, 1.5, 0.0)


def test_invalid_variation_seed_fails() -> None:
    with pytest.raises(ValueError, match="seed"):
        VariationProfile(seed=True)


def test_variation_seed_is_normalized() -> None:
    assert VariationProfile(seed=42).seed == 42


def test_unknown_instrument_fails() -> None:
    with pytest.raises(ValueError, match="unknown instrument"):
        get_instrument("laser_harpsichord")


def test_existing_profile_and_bad_profile_lookup_paths() -> None:
    profile = get_instrument("sine")

    assert get_instrument(profile) is profile
    with pytest.raises(ValueError, match="instrument"):
        get_instrument(123)  # type: ignore[arg-type]


def test_direct_gm_program_validation() -> None:
    with pytest.raises(ValueError, match=r"\[1, 128\]"):
        GMProgram(129, "Too Far", "bad", "sine")


def test_instrument_signal_requires_profile() -> None:
    with pytest.raises(ValueError, match="InstrumentProfile"):
        InstrumentSignal(
            fundamental_hz=440.0,
            instrument=object(),  # type: ignore[arg-type]
            note_duration_seconds=1.0,
        )


def test_render_rejects_unrepresentable_and_unbounded_work() -> None:
    with pytest.raises(ValueError, match="note string"):
        render_instrument_note(object(), 0.1)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="Nyquist"):
        render_instrument_note("A4", 0.1, sample_rate_hz=800)
    with pytest.raises(ValueError, match="max_sample_count"):
        render_instrument_note("A4", 10.0, max_sample_count=10)
