"""Tests for naive instrument profiles, timbre models, and note rendering.

Covers MUS05 §4-§8, §10, and VER15 security/weakness matrix invariants:
- Built-in presets and 128-program General MIDI catalog.
- Additive harmonic synthesis and ADSR envelope stages with release tails.
- Deterministic SplitMix64 PRNG and seeded VariationProfile micro-variations.
- Strict CWE-400 bounds (MAX_PARTIALS, MAX_NOTE_DURATION_SECONDS,
  MAX_SAMPLE_RATE_HZ, MAX_SAMPLE_COUNT).
- CWE-674 recursion elimination (strictly iterative loops).
- CWE-1333 / CWE-94 static AST audit (zero re, zero eval, zero exec).
- Property-based testing and adversarial fuzzing using deterministic SplitMix64 PRNG.
"""

from __future__ import annotations

import ast
from math import inf, nan
from pathlib import Path

import pytest
from note_frequency import parse_note
from pcm_audio import PCMBuffer

from musical_instruments import (
    DEFAULT_MAX_PARTIAL_COUNT,
    DEFAULT_MAX_SAMPLE_COUNT,
    MAX_NOTE_DURATION_SECONDS,
    MAX_PARTIALS,
    MAX_SAMPLE_COUNT,
    MAX_SAMPLE_RATE_HZ,
    ADSREnvelope,
    GMProgram,
    HarmonicPartial,
    InstrumentNoteRender,
    InstrumentProfile,
    InstrumentSignal,
    SplitMix64,
    VariationProfile,
    all_gm_programs,
    all_instruments,
    get_gm_program,
    get_instrument,
    instrument_for_gm_program,
    render_instrument_note,
)
from musical_instruments.musical_instruments import (
    _safe_round_sample,
    _sample_rate,
)

# =============================================================================
# § 1. SplitMix64 Deterministic PRNG Reference Vectors (VER05 §2.1 / TF01 §5.1)
# =============================================================================


def test_splitmix64_golden_reference_vectors() -> None:
    """Verify SplitMix64 against official golden vectors from cases.json."""
    # Seed 0 golden sequence
    rng0 = SplitMix64(0)
    assert rng0.next_u64() == 16294208416658607535
    assert rng0.next_u64() == 7960286522194355700
    assert rng0.next_u64() == 487617019471545679
    assert rng0.next_u64() == 17909611376780542444

    # Seed 18446744073709551615 (2^64 - 1) golden sequence
    rng_max = SplitMix64(18446744073709551615)
    assert rng_max.next_u64() == 16490336266968443936
    assert rng_max.next_u64() == 16834447057089888969
    assert rng_max.next_u64() == 4048727598324417001

    # Seed 1234567 golden sequence
    rng_sub = SplitMix64(1234567)
    assert rng_sub.next_u64() == 6457827717110365317
    assert rng_sub.next_u64() == 3203168211198807973
    assert rng_sub.next_u64() == 9817491932198370423


def test_splitmix64_float_generation() -> None:
    """Verify next_float_signed in [-1.0, 1.0] and next_float_unit in [0.0, 1.0]."""
    rng = SplitMix64(42)
    for _ in range(100):
        fs = rng.next_float_signed()
        assert -1.0 <= fs <= 1.0
        fu = rng.next_float_unit()
        assert 0.0 <= fu <= 1.0


def test_splitmix64_validation_rejects_non_integers() -> None:
    with pytest.raises(ValueError, match="seed"):
        SplitMix64("bad")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="seed"):
        SplitMix64(True)  # type: ignore[arg-type]


# =============================================================================
# § 2. Starter & Built-in Presets (MUS05 §8)
# =============================================================================


def test_exported_constants_and_defaults() -> None:
    """Verify package constants conform to MUS05 and VER15 bounds."""
    assert MAX_PARTIALS == 64
    assert DEFAULT_MAX_PARTIAL_COUNT == 64
    assert MAX_NOTE_DURATION_SECONDS == 60.0
    assert MAX_SAMPLE_RATE_HZ == 192_000.0
    assert MAX_SAMPLE_COUNT == 16_777_216
    assert DEFAULT_MAX_SAMPLE_COUNT == 10_000_000


def test_builtin_presets_are_available() -> None:
    ids = {instrument.id for instrument in all_instruments()}

    assert {
        "sine",
        "flute_naive",
        "clarinet_naive",
        "violin_naive",
        "piano_naive",
        "pluck_naive",
        "brass_naive",
        "organ_naive",
        "mallet_naive",
        "celesta_naive",
        "glockenspiel_naive",
        "vibraphone_naive",
        "marimba_naive",
        "xylophone_naive",
        "tubular_bells_naive",
        "timpani_naive",
        "kalimba_naive",
        "synth_lead_naive",
        "synth_pad_naive",
        "effect_naive",
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


# =============================================================================
# § 3. Signal Generation and Note Rendering (MUS05 §10)
# =============================================================================


def test_render_returns_inspectable_samples_and_pcm() -> None:
    rendered = render_instrument_note(
        "A4",
        0.01,
        instrument="flute_naive",
        sample_rate_hz=8_000,
        amplitude=0.25,
    )

    assert isinstance(rendered, InstrumentNoteRender)
    assert rendered.note.frequency() == pytest.approx(440.0)
    assert rendered.fundamental_hz == pytest.approx(440.0)
    assert rendered.instrument.id == "flute_naive"
    assert rendered.floating_samples.sample_count() > 80
    assert isinstance(rendered.pcm_buffer, PCMBuffer)
    assert (
        rendered.pcm_buffer.sample_count() == rendered.floating_samples.sample_count()
    )


def test_render_accepts_parsed_note_instance() -> None:
    """Accept Note instance directly in render_instrument_note."""
    note_obj = parse_note("D4")
    rendered = render_instrument_note(
        note_obj,
        0.01,
        instrument="sine",
        sample_rate_hz=8_000,
    )
    assert rendered.fundamental_hz == pytest.approx(note_obj.frequency())


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


def test_amplitude_normalizer_on_instruments() -> None:
    flute = get_instrument("flute_naive")
    assert flute.amplitude_normalizer() > 0.0

    silence = get_instrument("silence")
    assert silence.amplitude_normalizer() == 1.0


# =============================================================================
# § 4. ADSR Envelope Stages and Bounds (MUS05 §6)
# =============================================================================


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


def test_adsr_envelope_bounds_enforced() -> None:
    """Enforce MAX_NOTE_DURATION_SECONDS bounds on ADSR stages."""
    with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
        ADSREnvelope(60.1, 0.1, 0.5, 0.1)
    with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
        ADSREnvelope(0.1, 60.1, 0.5, 0.1)
    with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
        ADSREnvelope(0.1, 0.1, 0.5, 60.1)

    env = ADSREnvelope(0.1, 0.1, 0.5, 0.1)
    with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
        env.rendered_duration_seconds(60.1)
    with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
        env.value_at(0.05, 60.1)


# =============================================================================
# § 5. Seeded VariationProfile & Deterministic Humanization (MUS05 §7)
# =============================================================================


def test_variation_profile_defaults_and_normalization() -> None:
    var = VariationProfile()
    assert var.pitch_cents_jitter == 0.0
    assert var.amplitude_jitter == 0.0
    assert var.attack_jitter_ms == 0.0
    assert var.seed is None
    assert var.harmonic_jitter == 0.0
    assert var.pitch_jitter_cents == 0.0
    assert var.timing_jitter_seconds == 0.0

    var_seeded = VariationProfile(
        pitch_cents_jitter=5.0,
        amplitude_jitter=0.1,
        attack_jitter_ms=2.5,
        seed=12345,
        harmonic_jitter=0.05,
    )
    assert var_seeded.pitch_cents_jitter == 5.0
    assert var_seeded.amplitude_jitter == 0.1
    assert var_seeded.attack_jitter_ms == 2.5
    assert var_seeded.seed == 12345
    assert var_seeded.harmonic_jitter == 0.05
    assert var_seeded.pitch_jitter_cents == 5.0
    assert var_seeded.timing_jitter_seconds == pytest.approx(0.0025)


def test_variation_profile_alias_constructors() -> None:
    """Accept pitch_jitter_cents and timing_jitter_seconds kwargs."""
    var = VariationProfile(
        pitch_jitter_cents=10.0,
        timing_jitter_seconds=0.005,
        amplitude_jitter=0.2,
        seed=999,
    )
    assert var.pitch_cents_jitter == 10.0
    assert var.attack_jitter_ms == pytest.approx(5.0)
    assert var.seed == 999


def test_variation_seed_determinism_and_reproducibility() -> None:
    """Identical seeds produce bit-for-bit identical floating and PCM samples."""
    var = VariationProfile(
        pitch_cents_jitter=15.0,
        amplitude_jitter=0.2,
        attack_jitter_ms=5.0,
        seed=424242,
        harmonic_jitter=0.1,
    )
    render_a = render_instrument_note(
        "C4",
        0.05,
        instrument="violin_naive",
        sample_rate_hz=16_000,
        variation_profile=var,
    )
    render_b = render_instrument_note(
        "C4",
        0.05,
        instrument="violin_naive",
        sample_rate_hz=16_000,
        variation_profile=var,
    )

    assert render_a.floating_samples.samples == render_b.floating_samples.samples
    assert render_a.pcm_buffer.samples == render_b.pcm_buffer.samples


def test_different_seeds_produce_different_signals() -> None:
    var1 = VariationProfile(pitch_cents_jitter=25.0, seed=1001)
    var2 = VariationProfile(pitch_cents_jitter=25.0, seed=1002)

    render1 = render_instrument_note(
        "C4",
        0.05,
        instrument="flute_naive",
        sample_rate_hz=16_000,
        variation_profile=var1,
    )
    render2 = render_instrument_note(
        "C4",
        0.05,
        instrument="flute_naive",
        sample_rate_hz=16_000,
        variation_profile=var2,
    )

    assert render1.floating_samples.samples != render2.floating_samples.samples


def test_unseeded_variation_is_machine_perfect() -> None:
    """When seed is None, variation is inactive and matches pure preset."""
    var_unseeded = VariationProfile(pitch_cents_jitter=50.0, seed=None)
    pure_render = render_instrument_note(
        "A4",
        0.02,
        instrument="piano_naive",
        sample_rate_hz=16_000,
    )
    var_render = render_instrument_note(
        "A4",
        0.02,
        instrument="piano_naive",
        sample_rate_hz=16_000,
        variation_profile=var_unseeded,
    )

    assert pure_render.floating_samples.samples == var_render.floating_samples.samples


def test_variation_profile_attached_to_instrument_profile() -> None:
    """InstrumentProfile carrying a seeded VariationProfile affects rendering."""
    base_flute = get_instrument("flute_naive")
    seeded_var = VariationProfile(
        pitch_cents_jitter=20.0,
        amplitude_jitter=0.15,
        attack_jitter_ms=3.0,
        seed=777,
    )
    custom_flute = InstrumentProfile(
        id="human_flute",
        display_name="Human Flute",
        synthesis_kind=base_flute.synthesis_kind,
        gain=base_flute.gain,
        harmonic_profile=base_flute.harmonic_profile,
        envelope_profile=base_flute.envelope_profile,
        variation_profile=seeded_var,
        gm_program=74,
    )

    unvaried = render_instrument_note(
        "A4", 0.05, instrument=base_flute, sample_rate_hz=12_000
    )
    varied = render_instrument_note(
        "A4", 0.05, instrument=custom_flute, sample_rate_hz=12_000
    )

    assert unvaried.floating_samples.samples != varied.floating_samples.samples


# =============================================================================
# § 6. CWE-400 Bounds and Security Limits
# =============================================================================


def test_cwe400_max_partials_bound() -> None:
    """MAX_PARTIALS = 64 enforced on profile and render."""
    too_many = tuple(HarmonicPartial(i + 1, 1.0) for i in range(MAX_PARTIALS + 1))
    env = ADSREnvelope(0.01, 0.01, 0.5, 0.01)

    with pytest.raises(ValueError, match="MAX_PARTIALS"):
        InstrumentProfile("overloaded", "Overloaded", "additive", 0.5, too_many, env)

    with pytest.raises(ValueError, match="MAX_PARTIALS"):
        render_instrument_note("A4", 0.01, max_partial_count=MAX_PARTIALS + 1)

    # Render rejects when instrument profile has more partials than max_partial_count
    with pytest.raises(ValueError, match="above max_partial_count"):
        render_instrument_note(
            "A4", 0.01, instrument="flute_naive", max_partial_count=1
        )


def test_cwe400_max_note_duration_bound() -> None:
    """MAX_NOTE_DURATION_SECONDS = 60.0 enforced everywhere."""
    with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
        render_instrument_note("A4", 60.1)

    with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
        InstrumentSignal(
            fundamental_hz=440.0,
            instrument=get_instrument("sine"),
            note_duration_seconds=60.1,
        )


def test_cwe400_max_sample_rate_hz_bound() -> None:
    """MAX_SAMPLE_RATE_HZ = 192_000.0 enforced across entry points."""
    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        render_instrument_note("A4", 0.01, sample_rate_hz=192_001.0)

    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        render_instrument_note("A4", 0.01, sample_rate_hz=1e12)

    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        InstrumentSignal(
            fundamental_hz=440.0,
            instrument=get_instrument("sine"),
            note_duration_seconds=0.1,
            sample_rate_hz=192_001.0,
        )

    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        _sample_rate("rate", 192_001.0)


def test_cwe400_max_sample_count_bound() -> None:
    """MAX_SAMPLE_COUNT = 16_777_216 enforced."""
    with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
        render_instrument_note("A4", 0.01, max_sample_count=MAX_SAMPLE_COUNT + 1)


def test_cwe190_float_to_int_overflow_protection() -> None:
    """Guards float-to-int rounding against OverflowError (CWE-190)."""
    with pytest.raises(ValueError, match="finite"):
        _safe_round_sample(nan, 44100.0)
    with pytest.raises(ValueError, match="finite"):
        _safe_round_sample(1.0, nan)
    with pytest.raises(ValueError, match="finite"):
        _safe_round_sample(inf, 44100.0)
    with pytest.raises(ValueError, match="finite"):
        _safe_round_sample(1e308, 1e308)

    assert _safe_round_sample(0.5, 44100.0) == 22050


# =============================================================================
# § 7. Adversarial Fuzzing and Input Validation
# =============================================================================


@pytest.mark.parametrize("bad_val", [nan, inf, -1.0, "text", True])
def test_hostile_numeric_inputs_fail_closed(bad_val: object) -> None:
    with pytest.raises(ValueError):
        render_instrument_note("A4", bad_val)  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        render_instrument_note("A4", 0.01, amplitude=bad_val)  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        render_instrument_note("A4", 0.01, sample_rate_hz=bad_val)  # type: ignore[arg-type]


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
    with pytest.raises(ValueError, match="seed"):
        VariationProfile(seed=True)
    with pytest.raises(ValueError, match=">= 0.0"):
        VariationProfile(pitch_cents_jitter=-1.0)
    with pytest.raises(ValueError, match=">= 0.0"):
        VariationProfile(amplitude_jitter=-0.1)
    with pytest.raises(ValueError, match=">= 0.0"):
        VariationProfile(attack_jitter_ms=-1.0)
    with pytest.raises(ValueError, match=">= 0.0"):
        VariationProfile(harmonic_jitter=-0.05)


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


@pytest.mark.parametrize("program", [0, 129, True])
def test_invalid_gm_programs_fail(program: object) -> None:
    with pytest.raises(ValueError):
        get_gm_program(program)  # type: ignore[arg-type]


def test_direct_gm_program_validation() -> None:
    with pytest.raises(ValueError, match=r"\[1, 128\]"):
        GMProgram(129, "Too Far", "bad", "sine")


def test_unknown_instrument_fails() -> None:
    with pytest.raises(ValueError, match="unknown instrument"):
        get_instrument("laser_harpsichord")


def test_existing_profile_and_bad_profile_lookup_paths() -> None:
    profile = get_instrument("sine")
    assert get_instrument(profile) is profile
    with pytest.raises(ValueError, match="instrument"):
        get_instrument(123)  # type: ignore[arg-type]


def test_instrument_signal_requires_profile() -> None:
    with pytest.raises(ValueError, match="InstrumentProfile"):
        InstrumentSignal(
            fundamental_hz=440.0,
            instrument=object(),  # type: ignore[arg-type]
            note_duration_seconds=1.0,
        )


def test_instrument_signal_variation_profile_type_guard() -> None:
    with pytest.raises(ValueError, match="VariationProfile"):
        InstrumentSignal(
            fundamental_hz=440.0,
            instrument=get_instrument("sine"),
            note_duration_seconds=1.0,
            variation_profile=object(),  # type: ignore[arg-type]
        )


def test_render_rejects_unrepresentable_and_unbounded_work() -> None:
    with pytest.raises(ValueError, match="note string"):
        render_instrument_note(object(), 0.1)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="Nyquist"):
        render_instrument_note("A4", 0.1, sample_rate_hz=800)
    with pytest.raises(ValueError, match="max_sample_count"):
        render_instrument_note("A4", 10.0, max_sample_count=10)
    with pytest.raises(ValueError, match="VariationProfile"):
        render_instrument_note("A4", 0.01, variation_profile=object())  # type: ignore[arg-type]


# =============================================================================
# § 8. Static AST Audit (CWE-1333 / CWE-94 / CWE-674)
# =============================================================================


def test_cwe1333_cwe94_cwe674_static_ast_audit() -> None:
    """Static AST audit verifies zero re/eval/exec and zero recursion."""
    pkg_src = Path(__file__).resolve().parent.parent / "src" / "musical_instruments"
    py_files = list(pkg_src.glob("*.py"))
    assert len(py_files) >= 2

    for path in py_files:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        func_defs: dict[str, ast.FunctionDef] = {}

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

            if isinstance(node, ast.FunctionDef):
                func_defs[node.name] = node

        # Assert zero recursion: no function body contains a direct call to itself
        for name, func_node in func_defs.items():
            for child in ast.walk(func_node):
                if isinstance(child, ast.Call) and isinstance(child.func, ast.Name):
                    assert child.func.id != name, f"{path}: recursive function {name}()"


# =============================================================================
# § 9. Property-Based Testing (PBT)
# =============================================================================


def test_pbt_splitmix64_note_render_bounds_and_determinism() -> None:
    """PBT: Synthesized samples are bounded and reproducible under SplitMix64."""
    prng = SplitMix64(seed=0xFEED_FACE_2026)
    instruments = ["sine", "flute_naive", "violin_naive", "piano_naive"]
    notes = ["C4", "E4", "G4", "A4", "C5"]

    for _ in range(12):
        inst = instruments[prng.next_u64() % len(instruments)]
        note = notes[prng.next_u64() % len(notes)]
        seed_val = int(prng.next_u64() & 0x7FFF_FFFF)
        dur = 0.01 + (prng.next_float_unit() * 0.04)

        var = VariationProfile(
            pitch_cents_jitter=10.0,
            amplitude_jitter=0.1,
            attack_jitter_ms=2.0,
            seed=seed_val,
            harmonic_jitter=0.05,
        )

        r1 = render_instrument_note(
            note,
            dur,
            instrument=inst,
            sample_rate_hz=16_000,
            variation_profile=var,
        )
        r2 = render_instrument_note(
            note,
            dur,
            instrument=inst,
            sample_rate_hz=16_000,
            variation_profile=var,
        )

        # Determinism invariant
        assert r1.floating_samples.samples == r2.floating_samples.samples
        assert r1.pcm_buffer.samples == r2.pcm_buffer.samples

        # Sample bounds invariants
        for s in r1.floating_samples.samples:
            assert -1.0 <= s <= 1.0
        for p in r1.pcm_buffer.samples:
            assert -32768 <= p <= 32767
