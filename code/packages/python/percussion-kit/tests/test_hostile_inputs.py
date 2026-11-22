"""Hostile input testing for CWE-20, CWE-190, CWE-400, CWE-674, and CWE-1333."""

from __future__ import annotations

import ast
from collections.abc import Iterator
from fractions import Fraction
from pathlib import Path

import pytest

from percussion_kit import (
    MAX_HIT_DURATION_SECONDS,
    MAX_PERCUSSION_MODES,
    MAX_SAMPLE_COUNT,
    MAX_SAMPLE_RATE_HZ,
    MAX_SEQUENCE_EVENTS,
    DrumKitProfile,
    PercussionEnvelope,
    PercussionHitEvent,
    PercussionHitRender,
    PercussionMode,
    PercussionNoiseProfile,
    PercussionSequenceRender,
    PercussionSignal,
    PercussionVoiceProfile,
    apply_hihat_choke,
    get_drum_kit,
    get_percussion_voice,
    render_percussion_hit,
    render_percussion_sequence,
)
from percussion_kit.percussion_kit import _safe_repr


class TestCwe20TypeAndNumericValidation:
    @pytest.mark.parametrize(
        "bad_val",
        [
            True,
            False,
            float("nan"),
            float("inf"),
            float("-inf"),
            "1.0",
            None,
            complex(1.0, 0.0),
        ],
    )
    def test_rejects_bool_non_finite_and_non_real_params(
        self, bad_val: object
    ) -> None:
        with pytest.raises(ValueError):
            PercussionMode(bad_val, 1.0, 0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionMode(100.0, bad_val, 0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionMode(100.0, 1.0, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionMode(100.0, 1.0, 0.1, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionNoiseProfile(bad_val, 0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionNoiseProfile(0.5, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionEnvelope(bad_val, 0.0, 0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionEnvelope(0.0, bad_val, 0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionEnvelope(0.0, 0.0, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionHitEvent("kick", start_seconds=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionHitEvent("kick", duration_seconds=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PercussionHitEvent("kick", velocity=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_percussion_hit("kick", bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_percussion_hit("kick", 0.05, sample_rate_hz=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_percussion_hit("kick", 0.05, amplitude=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_percussion_hit("kick", 0.05, max_sample_count=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_percussion_hit("kick", 0.05, max_mode_count=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_percussion_sequence([], sample_rate_hz=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            apply_hihat_choke([0.1, bad_val], 0.0, 0.01, 8_000)  # type: ignore[list-item]

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            PercussionMode(huge_real, 1.0, 0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            PercussionEnvelope(huge_real, 0.0, 0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            render_percussion_hit("kick", huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            render_percussion_hit("kick", 0.05, sample_rate_hz=huge_real)  # type: ignore[arg-type]

    def test_subnormal_sample_rate_rejected(self) -> None:
        with pytest.raises(ValueError, match="subnormal"):
            render_percussion_hit("kick", 0.05, sample_rate_hz=5e-324)
        with pytest.raises(ValueError, match="subnormal"):
            render_percussion_sequence([], sample_rate_hz=5e-324)

    def test_string_and_dict_event_validation(self) -> None:
        env = PercussionEnvelope(0.0, 0.01, 0.1)
        mode = PercussionMode(100.0, 1.0, 0.1)
        voice = get_percussion_voice("kick")

        with pytest.raises(ValueError, match="id must be a non-empty string"):
            PercussionVoiceProfile(123, "Kick", "kick", 0.5, (mode,), None, env)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="display_name must be a non-empty string"):
            PercussionVoiceProfile("k", 123, "kick", 0.5, (mode,), None, env)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="family must be a non-empty string"):
            PercussionVoiceProfile("k", "Kick", 123, 0.5, (mode,), None, env)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="choke_group must be a string"):
            PercussionVoiceProfile(
                "k", "Kick", "kick", 0.5, (mode,), None, env, choke_group=123  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="id must be a non-empty string"):
            DrumKitProfile(123, "Kit", {"kick": voice})  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="display_name must be a non-empty string"):
            DrumKitProfile("kit", 123, {"kick": voice})  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="mapping"):
            DrumKitProfile("kit", "Kit", [("kick", voice)])  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="non-empty strings"):
            DrumKitProfile("kit", "Kit", {123: voice})  # type: ignore[dict-item]
        with pytest.raises(ValueError, match="track_id must be non-empty string"):
            PercussionHitEvent("kick", track_id=123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="variation must be a string"):
            PercussionHitEvent("kick", variation=123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="missing required key 'hit_id'"):
            render_percussion_sequence([{"start_seconds": 0.0}], sample_rate_hz=8_000)

        with pytest.raises(ValueError, match="amplitude sum overflows"):
            PercussionVoiceProfile(
                "ovf",
                "Ovf",
                "drum",
                1.0,
                (
                    PercussionMode(100.0, 1.5e308, 1.0),
                    PercussionMode(200.0, 1.5e308, 1.0),
                ),
                None,
                env,
            )

    @pytest.mark.parametrize(
        ("field", "match"),
        [
            ("hit_id", "hit_id must be a non-empty string"),
            ("voice", "voice must be a PercussionVoiceProfile"),
            ("signal", "signal must be a PercussionSignal"),
            ("floating_samples", "floating_samples must be a SampleBuffer"),
            ("pcm_buffer", "pcm_buffer must be a PCMBuffer"),
        ],
    )
    def test_percussion_hit_render_rejects_invalid_stage_fields(
        self, field: str, match: str
    ) -> None:
        valid = render_percussion_hit("kick", 0.02, sample_rate_hz=8_000)
        kwargs: dict[str, object] = {
            "hit_id": valid.hit_id,
            "voice": valid.voice,
            "signal": valid.signal,
            "floating_samples": valid.floating_samples,
            "pcm_buffer": valid.pcm_buffer,
        }
        kwargs[field] = "" if field == "hit_id" else "invalid"
        with pytest.raises(ValueError, match=match):
            PercussionHitRender(**kwargs)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        ("field", "match"),
        [
            ("events", "PercussionHitEvent"),
            ("floating_samples", "floating_samples must be a SampleBuffer"),
            ("pcm_buffer", "pcm_buffer must be a PCMBuffer"),
        ],
    )
    def test_percussion_sequence_render_rejects_invalid_stage_fields(
        self, field: str, match: str
    ) -> None:
        valid = render_percussion_sequence([("kick", 0.0)], sample_rate_hz=8_000)
        kwargs: dict[str, object] = {
            "events": valid.events,
            "floating_samples": valid.floating_samples,
            "pcm_buffer": valid.pcm_buffer,
        }
        kwargs[field] = ("bad",) if field == "events" else "invalid"
        with pytest.raises(ValueError, match=match):
            PercussionSequenceRender(**kwargs)  # type: ignore[arg-type]

        with pytest.raises(ValueError, match="iterable"):
            PercussionSequenceRender(
                "bad", valid.floating_samples, valid.pcm_buffer  # type: ignore[arg-type]
            )


class TestCwe190HugeIntegersAndSafeRepr:
    @pytest.mark.parametrize(
        "huge_int",
        [10**5000, -(10**5000)],
        ids=["pos-10e5000", "neg-10e5000"],
    )
    def test_huge_int_fails_in_o1_via_safe_repr(self, huge_int: int) -> None:
        assert _safe_repr(huge_int) == "<out-of-range int>"
        with pytest.raises(ValueError, match="<out-of-range int>") as exc1:
            render_percussion_hit("kick", 0.01, max_sample_count=huge_int)
        assert len(str(exc1.value)) < 160

        with pytest.raises(ValueError, match="<out-of-range int>") as exc2:
            PercussionSignal(
                voice=get_percussion_voice("kick"),
                hit_duration_seconds=huge_int,  # type: ignore[arg-type]
            )
        assert len(str(exc2.value)) < 160

    def test_safe_repr_formats_long_strings(self) -> None:
        long_str = "x" * 5000
        with pytest.raises(ValueError, match="<str len=5000>") as exc1:
            get_drum_kit(long_str)
        assert len(str(exc1.value)) < 160

        with pytest.raises(ValueError, match="<str len=5000>") as exc2:
            get_percussion_voice(long_str)
        assert len(str(exc2.value)) < 160


class TestCwe400ResourceExhaustionBounds:
    def test_oversized_and_generator_modes_rejected_before_alloc(self) -> None:
        env = PercussionEnvelope(0.0, 0.01, 0.1)
        mode = PercussionMode(100.0, 1.0, 0.1)

        class LyingSized:
            def __len__(self) -> int:
                return 10**100

            def __iter__(self) -> Iterator[PercussionMode]:
                yield mode

        with pytest.raises(ValueError, match="MAX_PERCUSSION_MODES"):
            PercussionVoiceProfile(
                "v", "V", "f", 0.5, LyingSized(), None, env  # type: ignore[arg-type]
            )

        consumed = 0

        def infinite_modes() -> Iterator[PercussionMode]:
            nonlocal consumed
            while True:
                consumed += 1
                yield mode

        with pytest.raises(ValueError, match="MAX_PERCUSSION_MODES"):
            PercussionVoiceProfile(
                "v", "V", "f", 0.5, infinite_modes(), None, env  # type: ignore[arg-type]
            )
        assert consumed == MAX_PERCUSSION_MODES + 1

        with pytest.raises(ValueError, match="iterable"):
            PercussionVoiceProfile("v", "V", "f", 0.5, "bad", None, env)  # type: ignore[arg-type]

    def test_oversized_and_generator_events_and_samples_rejected_before_alloc(
        self,
    ) -> None:
        class LyingSized:
            def __len__(self) -> int:
                return 10**100

            def __iter__(self) -> Iterator[tuple[str, float]]:
                yield ("kick", 0.0)

        with pytest.raises(ValueError, match="max_sequence_events"):
            render_percussion_sequence(LyingSized(), sample_rate_hz=8_000)  # type: ignore[arg-type]

        with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
            apply_hihat_choke(LyingSized(), 0.0, 0.01, 8_000)  # type: ignore[arg-type]

        with pytest.raises(ValueError, match="iterable"):
            render_percussion_sequence("kick", sample_rate_hz=8_000)
        with pytest.raises(ValueError, match="iterable"):
            apply_hihat_choke("bad", 0.0, 0.01, 8_000)  # type: ignore[arg-type]

        consumed = 0

        def infinite_events() -> Iterator[tuple[str, float]]:
            nonlocal consumed
            while True:
                consumed += 1
                yield ("kick", 0.0)

        with pytest.raises(ValueError, match="max_sequence_events"):
            render_percussion_sequence(
                infinite_events(), sample_rate_hz=8_000, max_sequence_events=5
            )
        assert consumed == 6

        with pytest.raises(ValueError, match="MAX_SEQUENCE_EVENTS"):
            render_percussion_sequence(
                [("kick", 0.0)],
                sample_rate_hz=8_000,
                max_sequence_events=MAX_SEQUENCE_EVENTS + 1,
            )

    def test_pre_flight_duration_rate_and_sample_caps(self) -> None:
        with pytest.raises(ValueError, match="MAX_HIT_DURATION_SECONDS"):
            render_percussion_hit("kick", MAX_HIT_DURATION_SECONDS + 0.1)
        with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
            render_percussion_hit("kick", 0.01, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
        with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
            render_percussion_hit("kick", 0.01, max_sample_count=MAX_SAMPLE_COUNT + 1)
        with pytest.raises(ValueError, match="sequence render would require"):
            render_percussion_sequence(
                [PercussionHitEvent("kick", start_seconds=100.0)],
                sample_rate_hz=8_000,
                max_sample_count=10_000,
            )


class TestCwe1333AndCwe674StaticAstAudit:
    def test_zero_regex_eval_exec_and_zero_recursion(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "percussion_kit"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            func_defs: dict[str, ast.FunctionDef] = {}
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(
                        alias.name not in ("re", "regex") for alias in node.names
                    )
                elif isinstance(node, ast.ImportFrom):
                    assert node.module not in ("re", "regex")
                elif isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                    assert node.func.id not in ("eval", "exec")
                elif isinstance(node, ast.FunctionDef):
                    func_defs[node.name] = node
            for name, fn in func_defs.items():
                for child in ast.walk(fn):
                    if isinstance(child, ast.Call) and isinstance(
                        child.func, ast.Name
                    ):
                        assert child.func.id != name
