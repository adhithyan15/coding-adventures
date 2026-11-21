"""Hostile input testing for CWE-20, CWE-190, CWE-400, CWE-674, and CWE-1333."""

from __future__ import annotations

import ast
import itertools
import os
import subprocess
from collections.abc import Iterator
from fractions import Fraction
from pathlib import Path
from unittest.mock import patch

import pytest

from music_machine import (
    MAX_EVENTS,
    MAX_INSTRUMENTS,
    MAX_JSON_DEPTH,
    MAX_LINE_COUNT,
    MAX_LINE_LENGTH,
    MAX_POLYPHONY,
    MAX_REPEAT_COUNT,
    MAX_SAMPLE_COUNT,
    MAX_SAMPLE_RATE_HZ,
    MAX_SCORE_BYTES,
    MAX_TRACKS,
    PORTABLE_FORMAT_VERSION,
    InstrumentDeclaration,
    MeterEvent,
    PhraseMotif,
    PortableScore,
    PortableScoreBuilder,
    PortableScoreEvent,
    RenderedPortableScore,
    RenderedScore,
    ScoreEvent,
    TempoEvent,
    TextScore,
    TrackDeclaration,
    parse_portable_score,
    parse_score,
    render_portable_score_to_pcm,
    render_score_to_pcm,
)
from music_machine.music_machine import MAX_PPQ, _safe_repr
from music_machine.score import (
    MusicalEvent,
    Score,
    ScoreParseError,
)
from music_machine.score import (
    parse_score as parse_simple_score,
)
from music_machine.score import (
    render_score as render_simple_score,
)


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
            TempoEvent(0, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            TempoEvent(bad_val, 120.0)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            MeterEvent(bad_val, "4/4")  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            InstrumentDeclaration("lead", "sine", gain=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PortableScoreEvent(
                "trk", 0, 120, "note", notes=("C4",), velocity=bad_val  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError):
            PortableScoreBuilder(ppq=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PortableScoreBuilder(sample_rate_hz=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError):
            MusicalEvent(note=None, duration_beats=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError):
            MusicalEvent(note=None, duration_beats=1.0, velocity=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError):
            Score(events=(), tempo_bpm=bad_val)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            TempoEvent(0, huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            PortableScoreEvent(
                "trk", 0, 120, "note", notes=("C4",), velocity=huge_real  # type: ignore[arg-type]
            )
        with pytest.raises(ScoreParseError, match="overflows float range"):
            MusicalEvent(note=None, duration_beats=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="overflows float range"):
            Score(events=(), tempo_bpm=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="overflows float range"):
            render_simple_score(Score(events=()), sample_rate_hz=huge_real)  # type: ignore[arg-type]

    def test_fractional_and_subnormal_sample_rates_rejected(self) -> None:
        with pytest.raises(ScoreParseError, match=">= 1"):
            render_simple_score(Score(events=()), sample_rate_hz=0.1)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match=">= 1"):
            render_simple_score(Score(events=()), sample_rate_hz=5e-324)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="sample_rate"):
            parse_score("sample_rate: 5e-324\nA4/q")
        with pytest.raises(ValueError, match="sample_rate"):
            parse_portable_score(
                f"format: {PORTABLE_FORMAT_VERSION}\nsample_rate: 5e-324\n"
            )

    def test_invalid_bpm_meter_note_and_string_fields(self) -> None:
        with pytest.raises(ValueError, match="tempo must be > 0.0"):
            parse_score("tempo: -5\nA4/q")
        with pytest.raises(ValueError, match="meter must look like"):
            parse_score("meter: 0/4\nA4/q")
        with pytest.raises(ValueError, match="meter must look like"):
            parse_score(f"meter: {'9' * 20}/4\nA4/q")
        with pytest.raises(ValueError, match="Invalid note"):
            parse_score("H4/q")
        with pytest.raises(ValueError, match="title must be a string"):
            PortableScoreBuilder(title=123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="title must be a string"):
            TextScore(
                title=123,  # type: ignore[arg-type]
                tempo_bpm=120.0,
                meter="4/4",
                amplitude=0.5,
                sample_rate_hz=8000,
                instrument_id="sine",
                gm_program=None,
                events=(ScoreEvent("note", "C4", "q", 1.0, 0.5, "C4/q"),),
            )
        with pytest.raises(ValueError, match="ppq .* exceeds MAX_PPQ"):
            PortableScoreBuilder(ppq=MAX_PPQ + 1)
        with pytest.raises(ValueError, match="format_version must be a string"):
            PortableScore.from_dict({"format_version": 123})
        with pytest.raises(ValueError, match="title must be a string"):
            PortableScore.from_dict(
                {"format_version": PORTABLE_FORMAT_VERSION, "title": 123}
            )
        with pytest.raises(ValueError, match="indent must be"):
            parse_portable_score(
                f"format: {PORTABLE_FORMAT_VERSION}\n"
            ).to_json(indent=True)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="indent must be"):
            parse_portable_score(
                f"format: {PORTABLE_FORMAT_VERSION}\n"
            ).to_json(indent=100)

    def test_stage_render_isinstance_checks(self) -> None:
        score = parse_score("sample_rate: 8000\nC4/e")
        rendered = render_score_to_pcm(score)
        with pytest.raises(ValueError, match="TextScore"):
            RenderedScore("bad", rendered.pcm_buffer, rendered.rendered_notes)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="PCMBuffer"):
            RenderedScore(rendered.score, "bad", rendered.rendered_notes)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="InstrumentNoteRender"):
            RenderedScore(rendered.score, rendered.pcm_buffer, ("bad",))  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="PercussionHitRender"):
            RenderedScore(rendered.score, rendered.pcm_buffer, (), ("bad",))  # type: ignore[arg-type]

        pscore = parse_portable_score(
            f"format: {PORTABLE_FORMAT_VERSION}\nsample_rate: 8000\n"
            "instrument lead kind=sine\ntrack mel instrument=lead\n"
            "event mel 0 120 note C4\n"
        )
        prendered = render_portable_score_to_pcm(pscore)
        with pytest.raises(ValueError, match="PortableScore"):
            RenderedPortableScore(
                "bad", prendered.pcm_buffer, prendered.rendered_notes  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="PCMBuffer"):
            RenderedPortableScore(
                prendered.score, "bad", prendered.rendered_notes  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="InstrumentNoteRender"):
            RenderedPortableScore(
                prendered.score, prendered.pcm_buffer, ("bad",)  # type: ignore[arg-type]
            )
        with pytest.raises(
            ValueError, match="PercussionHitRender or PercussionHitEvent"
        ):
            RenderedPortableScore(
                prendered.score, prendered.pcm_buffer, (), ("bad",)  # type: ignore[arg-type]
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
            PortableScoreBuilder(ppq=huge_int)
        assert len(str(exc1.value)) < 160

        with pytest.raises(ValueError, match="<out-of-range int>") as exc2:
            TempoEvent(0, huge_int)
        assert len(str(exc2.value)) < 160

        with pytest.raises(ScoreParseError, match="<out-of-range int>") as exc3:
            Score(events=(), tempo_bpm=huge_int)
        assert len(str(exc3.value)) < 160

    def test_tempo_map_overflow_on_huge_tick_caught(self) -> None:
        builder = PortableScoreBuilder(ppq=120, sample_rate_hz=8000)
        builder.add_instrument("lead", kind="sine")
        builder.add_track("mel", instrument_id="lead")
        builder.add_note("mel", 10**400, 120, "C4")
        score = builder.build()
        with pytest.raises(ValueError, match="overflows float range"):
            render_portable_score_to_pcm(score)

    def test_long_tick_token_rejected_in_o1(self) -> None:
        huge_tick = "9" * 5000
        with pytest.raises(ValueError, match="<str len=5000>") as exc:
            parse_portable_score(
                f"format: {PORTABLE_FORMAT_VERSION}\ntempo {huge_tick} 120\n"
            )
        assert len(str(exc.value)) < 160


class TestCwe400ResourceExhaustionBounds:
    def test_pre_flight_caps_on_bytes_lines_events_tracks_polyphony_and_repeats(
        self,
    ) -> None:
        # Line count flood (> MAX_LINE_COUNT) before splitlines materialization
        many_blank_lines = "\n" * (MAX_LINE_COUNT + 5)
        with pytest.raises(ValueError, match="MAX_LINE_COUNT"):
            parse_score(many_blank_lines)
        with pytest.raises(ValueError, match="MAX_LINE_COUNT"):
            parse_portable_score(many_blank_lines)
        with pytest.raises(ScoreParseError, match="MAX_LINE_COUNT"):
            parse_simple_score(many_blank_lines)

        # Oversized parameter limits rejected
        with pytest.raises(ValueError, match="MAX_SCORE_BYTES"):
            parse_score("A4/q", max_score_length=MAX_SCORE_BYTES + 1)
        with pytest.raises(ValueError, match="MAX_LINE_LENGTH"):
            parse_score("A4/q", max_line_length=MAX_LINE_LENGTH + 1)
        with pytest.raises(ValueError, match="MAX_EVENTS"):
            parse_score("A4/q", max_event_count=MAX_EVENTS + 1)

        # Polyphony cap in text parser, PortableScoreEvent, and PortableScoreBuilder
        chord_text = ",".join(["C4"] * (MAX_POLYPHONY + 1))
        with pytest.raises(ValueError, match="limit="):
            parse_portable_score(
                f"format: {PORTABLE_FORMAT_VERSION}\n"
                "instrument lead kind=sine\ntrack mel instrument=lead\n"
                f"event mel 0 120 note {chord_text}\n"
            )
        with pytest.raises(ValueError, match="limit="):
            PortableScoreEvent(
                "mel", 0, 120, "note", notes=("C4",) * (MAX_POLYPHONY + 1)
            )
        builder = PortableScoreBuilder()
        with pytest.raises(ValueError, match="limit="):
            builder.add_chord("mel", 0, 120, ("C4",) * (MAX_POLYPHONY + 1))

        # Repeat count cap in apply_motif / apply_section
        motif = PhraseMotif(duration_tick=120, events=())
        with pytest.raises(ValueError, match="MAX_REPEAT_COUNT"):
            builder.apply_motif(motif, "mel", 0, repeat_count=MAX_REPEAT_COUNT + 1)

        # Instrument & track caps in builder
        inst_builder = PortableScoreBuilder()
        for i in range(MAX_INSTRUMENTS):
            inst_builder.add_instrument(f"i_{i}", kind="sine")
        with pytest.raises(ValueError, match="MAX_INSTRUMENTS"):
            inst_builder.add_instrument("i_extra", kind="sine")
        for i in range(MAX_TRACKS):
            inst_builder.add_track(f"t_{i}", instrument_id="i_0")
        with pytest.raises(ValueError, match="MAX_TRACKS"):
            inst_builder.add_track("t_extra", instrument_id="i_0")

    def test_generators_and_lying_len_rejected_before_materialization(self) -> None:
        ev = ScoreEvent("note", "C4", "q", 1.0, 0.5, "C4/q")
        pev = PortableScoreEvent("mel", 0, 120, "note", notes=("C4",))
        inst = InstrumentDeclaration("lead", "sine")
        trk = TrackDeclaration("mel", "lead")

        class LyingSized:
            def __len__(self) -> int:
                return 10**100

            def __iter__(self) -> Iterator[PortableScoreEvent]:
                yield pev

        with pytest.raises(ValueError, match="MAX_EVENTS"):
            PortableScore(
                format_version=PORTABLE_FORMAT_VERSION,
                title="t",
                ppq=120,
                sample_rate_hz=8000,
                tempo_events=(),
                meter_events=(),
                instruments=(inst,),
                tracks=(trk,),
                events=LyingSized(),  # type: ignore[arg-type]
            )

        with pytest.raises(ScoreParseError, match="MAX_EVENTS"):
            Score(events=LyingSized())  # type: ignore[arg-type]

        consumed = 0

        def inf_events() -> Iterator[ScoreEvent]:
            nonlocal consumed
            while True:
                consumed += 1
                yield ev

        with pytest.raises(ValueError, match=f"MAX_EVENTS={MAX_EVENTS}"):
            TextScore(
                title="t",
                tempo_bpm=120.0,
                meter="4/4",
                amplitude=0.5,
                sample_rate_hz=8000,
                instrument_id="sine",
                gm_program=None,
                events=inf_events(),  # type: ignore[arg-type]
            )
        assert consumed == MAX_EVENTS + 1


class TestAdditionalHardeningCoverage:
    def test_score_py_defensive_branches(self) -> None:
        from music_machine.score import _safe_repr as score_safe_repr

        assert score_safe_repr(1.5) == "1.5"
        assert score_safe_repr(object()) == "<object>"

        with pytest.raises(ScoreParseError, match="Note or None"):
            MusicalEvent(note="C4", duration_beats=1.0)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="positive finite"):
            MusicalEvent(note=None, duration_beats=0.0)
        with pytest.raises(ScoreParseError, match="within"):
            MusicalEvent(note=None, duration_beats=1.0, velocity=1.5)

        ev = MusicalEvent(note=None, duration_beats=1.0)
        with pytest.raises(ScoreParseError, match="positive finite"):
            ev.duration_seconds(True)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="overflows float"):
            ev.duration_seconds(10**400)

        with pytest.raises(ScoreParseError, match="title"):
            Score(events=(), title=123)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="iterable"):
            Score(events="bad")  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="MusicalEvent"):
            Score(events=("bad",))  # type: ignore[arg-type]

        def inf_simple() -> Iterator[MusicalEvent]:
            while True:
                yield ev

        with pytest.raises(ScoreParseError, match="MAX_EVENTS"):
            Score(events=inf_simple())  # type: ignore[arg-type]

        with pytest.raises(ScoreParseError, match="max_score_bytes"):
            parse_simple_score("A4 1", max_score_bytes=True)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="max_score_bytes"):
            parse_simple_score("A4 1", max_score_bytes=-1)
        with pytest.raises(ScoreParseError, match="max_event_count"):
            parse_simple_score("A4 1", max_event_count=True)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="max_event_count"):
            parse_simple_score("A4 1", max_event_count=-1)
        with pytest.raises(ScoreParseError, match="exceeds max_score_bytes"):
            parse_simple_score("é" * 3, max_score_bytes=4)

        with pytest.raises(ScoreParseError, match="max_sample_count"):
            render_simple_score(Score(events=()), max_sample_count=True)  # type: ignore[arg-type]

    def test_music_machine_defensive_branches(self, tmp_path: Path) -> None:
        from music_machine.music_machine import (
            _integer_directive,
            _validate_file_path,
            parse_portable_score_file,
            parse_score_file,
        )

        assert _safe_repr(object()) == "<object>"
        with pytest.raises(ValueError, match="out of range"):
            _integer_directive("ppq", "1e30")
        with pytest.raises(ValueError, match="str or Path"):
            _validate_file_path(123)  # type: ignore[arg-type]

        # ScoreEvent / TextScore / InstrumentDeclaration / PortableScore checks
        with pytest.raises(ValueError, match="duration_symbol"):
            ScoreEvent("rest", None, 123, 1.0, 0.5, "R/q")  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="source_token"):
            ScoreEvent("rest", None, "q", 1.0, 0.5, 123)  # type: ignore[arg-type]
        ev = ScoreEvent("rest", None, "q", 1.0, 0.5, "R/q")
        with pytest.raises(ValueError, match="drumkit"):
            TextScore("t", 120.0, "4/4", 0.5, 8000, "sine", None, (ev,), drumkit=123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="instrument_id"):
            TextScore("t", 120.0, "4/4", 0.5, 8000, 123, None, (ev,))  # type: ignore[arg-type]

        with pytest.raises(ValueError, match="profile_id"):
            InstrumentDeclaration("i", 123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="kind"):
            InstrumentDeclaration("i", "sine", kind=123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="kit"):
            InstrumentDeclaration("i", "drumkit", kind="drumkit", kit=123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="custom_profile"):
            InstrumentDeclaration("i", "sine", custom_profile="bad")  # type: ignore[arg-type]

        with pytest.raises(ValueError, match="notes must be an iterable"):
            PortableScoreEvent("trk", 0, 120, "rest", notes="bad")  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="notes must be an iterable"):
            PortableScoreEvent("trk", 0, 120, "hit", notes="bad", hit_id="kick")  # type: ignore[arg-type]

        with pytest.raises(ValueError, match="format_version"):
            PortableScore(
                "bad", "t", 120, 8000, (), (), (), (), ()
            )
        with pytest.raises(ValueError, match="title"):
            PortableScore(
                PORTABLE_FORMAT_VERSION, 123, 120, 8000, (), (), (), (), ()  # type: ignore[arg-type]
            )

        # Builder caps for tempo, meter, note, hit, rest, beats_to_ticks
        b = PortableScoreBuilder()
        for i in range(MAX_EVENTS):
            b.add_tempo(i, 120.0)
        with pytest.raises(ValueError, match="MAX_EVENTS"):
            b.add_tempo(MAX_EVENTS, 120.0)
        for i in range(MAX_EVENTS):
            b.add_meter(i, "4/4")
        with pytest.raises(ValueError, match="MAX_EVENTS"):
            b.add_meter(MAX_EVENTS, "4/4")
        for i in range(MAX_EVENTS):
            b.add_rest("mel", i * 10, 10)
        with pytest.raises(ValueError, match="MAX_EVENTS"):
            b.add_note("mel", 0, 10, "C4")
        with pytest.raises(ValueError, match="MAX_EVENTS"):
            b.add_hit("mel", 0, 10, "kick")
        with pytest.raises(ValueError, match="MAX_EVENTS"):
            b.add_rest("mel", 0, 10)
        with pytest.raises(ValueError, match="overflows tick range"):
            b.beats_to_ticks(1e308)

        # UTF-8 multibyte budget rejections and parser section caps
        with pytest.raises(ValueError, match="max_score_length"):
            parse_score("title: éé\nA4/q", max_score_length=11)
        with pytest.raises(ValueError, match="max_score_length"):
            parse_portable_score(
                f"format: {PORTABLE_FORMAT_VERSION}\ntitle: éé\n",
                max_score_length=38,
            )
        with pytest.raises(ValueError, match="exceeds max_bytes"):
            PortableScore.from_json('{"title": "éé"}', max_bytes=15)

        many_tempos = (
            f"format: {PORTABLE_FORMAT_VERSION}\n"
            + "\n".join(f"tempo {i} 120" for i in range(MAX_EVENTS + 1))
        )
        with pytest.raises(ValueError, match="tempo_events count"):
            parse_portable_score(many_tempos)

        many_meters = (
            f"format: {PORTABLE_FORMAT_VERSION}\n"
            + "\n".join(f"meter {i} 4/4" for i in range(MAX_EVENTS + 1))
        )
        with pytest.raises(ValueError, match="meter_events count"):
            parse_portable_score(many_meters)

        many_insts = (
            f"format: {PORTABLE_FORMAT_VERSION}\n"
            + "\n".join(
                f"instrument i_{i} kind=sine" for i in range(MAX_INSTRUMENTS + 1)
            )
        )
        with pytest.raises(ValueError, match="instrument count"):
            parse_portable_score(many_insts)

        many_tracks = (
            f"format: {PORTABLE_FORMAT_VERSION}\n"
            "instrument i_0 kind=sine\n"
            + "\n".join(f"track t_{i} instrument=i_0" for i in range(MAX_TRACKS + 1))
        )
        with pytest.raises(ValueError, match="track count"):
            parse_portable_score(many_tracks)

        # File loader bad limit
        valid_file = tmp_path / "ok.txt"
        valid_file.write_text("C4/q", encoding="utf-8")
        with pytest.raises(ValueError, match="max_score_length"):
            parse_score_file(valid_file, max_score_length=True)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="max_score_length"):
            parse_portable_score_file(valid_file, max_score_length=True)  # type: ignore[arg-type]

    def test_cwe22_hardened_file_path_and_bounded_read(self, tmp_path: Path) -> None:
        from music_machine.music_machine import (
            MAX_PATH_LENGTH,
            _read_score_file_bounded,
            _validate_file_path,
            parse_portable_score_file,
            parse_score_file,
        )

        secret = tmp_path / "secret.txt"
        secret.write_text("C4/q\n", encoding="utf-8")

        # Bool / non-str fspath / empty / oversized path / traversal / ADS / reserved
        with pytest.raises(ValueError, match="str or PathLike"):
            _validate_file_path(True)  # type: ignore[arg-type]

        class BadPathLike:
            def __fspath__(self) -> bytes:
                return b"abc.txt"

        with pytest.raises(ValueError, match="resolve to str"):
            _validate_file_path(BadPathLike())  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="MAX_PATH_LENGTH"):
            _validate_file_path("a" * (MAX_PATH_LENGTH + 1))
        with pytest.raises(ValueError, match="must not be empty"):
            _validate_file_path("   ")
        with pytest.raises(ValueError, match="path traversal"):
            _validate_file_path(str(tmp_path / "sub" / ".." / "secret.txt"))
        with pytest.raises(ValueError, match="alternate data stream"):
            _validate_file_path(str(secret) + ":stream")
        with pytest.raises(ValueError, match="reserved device"):
            _validate_file_path("CON.txt")
        with pytest.raises(ValueError, match="reserved device"):
            _validate_file_path(str(tmp_path / "NUL"))

        # Ancestor directory symlink or junction
        real_dir = tmp_path / "real_dir"
        real_dir.mkdir()
        (real_dir / "s.txt").write_text("C4/q\n", encoding="utf-8")

        sym_dir = tmp_path / "sym_dir"
        try:
            sym_dir.symlink_to(real_dir, target_is_directory=True)
            has_dir_sym = True
        except (OSError, NotImplementedError):
            has_dir_sym = False
        if has_dir_sym:
            with pytest.raises(ValueError, match="ancestor"):
                parse_score_file(sym_dir / "s.txt")

        if os.name == "nt":
            junc_dir = tmp_path / "junc_dir"
            res = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(junc_dir), str(real_dir)],
                capture_output=True,
                text=True,
                check=False,
            )
            if res.returncode == 0:
                with pytest.raises(ValueError, match="ancestor"):
                    parse_score_file(junc_dir / "s.txt")
                with pytest.raises(ValueError, match="ancestor"):
                    parse_portable_score_file(junc_dir / "s.txt")

        # Mocked isjunction on ancestor
        with (
            patch(
                "music_machine.music_machine._is_symlink_or_junction",
                side_effect=lambda p: p == Path(os.path.abspath(real_dir)),
            ),
            pytest.raises(ValueError, match="ancestor"),
        ):
            parse_score_file(real_dir / "s.txt")

        # Non-regular file & TOCTOU file growth beyond limit
        with pytest.raises(FileNotFoundError):
            parse_score_file(real_dir)
        big_file = real_dir / "big.txt"
        big_file.write_text("C4/q\n" * 20, encoding="utf-8")
        fake_stat = os.stat_result((0o100644, 0, 0, 1, 0, 0, 5, 0, 0, 0))
        with (
            patch("os.fstat", return_value=fake_stat),
            pytest.raises(ValueError, match="exceeds limit"),
        ):
            _read_score_file_bounded(big_file, 10)
        non_reg_stat = os.stat_result((0o040755, 0, 0, 1, 0, 0, 5, 0, 0, 0))
        with (
            patch("os.fstat", return_value=non_reg_stat),
            pytest.raises(ValueError, match="not a regular file"),
        ):
            _read_score_file_bounded(big_file, 10)

    def test_finding2_generators_and_type_checks_in_events_and_capture(self) -> None:
        from music_machine import ArrangementSectionEvent, PhraseMotifEvent

        # Infinite generator in rest/hit notes must fail immediately without hang/OOM
        with pytest.raises(ValueError, match="rest events cannot include notes"):
            PortableScoreEvent(
                "mel", 0, 120, "rest", notes=itertools.repeat("C4")  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="hit events cannot include notes"):
            PortableScoreEvent(
                "mel",
                0,
                120,
                "hit",
                notes=itertools.repeat("C4"),  # type: ignore[arg-type]
                hit_id="kick",
            )

        with pytest.raises(ValueError, match="iterable"):
            PhraseMotifEvent(0, 120, "note", notes="C4")  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="MAX_POLYPHONY"):
            PhraseMotifEvent(
                0, 120, "note", notes=itertools.repeat("C4")  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="rest motif events cannot include notes"):
            PhraseMotifEvent(
                0, 120, "rest", notes=itertools.repeat("C4")  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="hit motif events cannot include notes"):
            PhraseMotifEvent(
                0,
                120,
                "hit",
                notes=itertools.repeat("C4"),  # type: ignore[arg-type]
                hit_id="kick",
            )
        with pytest.raises(ValueError, match="hit_id string"):
            PhraseMotifEvent(0, 120, "hit", hit_id=True)  # type: ignore[arg-type]

        with pytest.raises(ValueError, match="iterable"):
            ArrangementSectionEvent("mel", 0, 120, "note", notes="C4")  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="MAX_POLYPHONY"):
            ArrangementSectionEvent(
                "mel", 0, 120, "note", notes=itertools.repeat("C4")  # type: ignore[arg-type]
            )
        with pytest.raises(
            ValueError, match="rest section events cannot include notes"
        ):
            ArrangementSectionEvent(
                "mel", 0, 120, "rest", notes=itertools.repeat("C4")  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="hit section events cannot include notes"):
            ArrangementSectionEvent(
                "mel",
                0,
                120,
                "hit",
                notes=itertools.repeat("C4"),  # type: ignore[arg-type]
                hit_id="kick",
            )
        with pytest.raises(ValueError, match="hit_id string"):
            ArrangementSectionEvent("mel", 0, 120, "hit", hit_id=123)  # type: ignore[arg-type]

        builder = PortableScoreBuilder()
        with pytest.raises(ValueError, match="MAX_TRACKS"):
            builder.capture_section(
                start_tick=0,
                end_tick=120,
                track_ids=itertools.repeat("mel"),  # type: ignore[arg-type]
            )

    def test_finding3_drumkit_directive_and_text_score_validation(self) -> None:
        parsed = parse_score("drumkit: standard_kit\nsample_rate: 8000\nkick/q\n")
        assert parsed.instrument_id == "drumkit"
        assert parsed.drumkit == "standard_kit"

        hit_ev = ScoreEvent("hit", None, "q", 1.0, 0.5, "kick/q", hit_id="kick")
        with pytest.raises(ValueError, match="unknown drum kit"):
            TextScore(
                "BadKit",
                120.0,
                "4/4",
                0.5,
                8000,
                "sine",
                None,
                (hit_ev,),
                drumkit="FAKE_NONEXISTENT_KIT",
            )

    def test_finding4_transpose_semitones_and_velocity_scale_mapping(self) -> None:
        from music_machine import (
            MAX_TRANSPOSE_SEMITONES,
            ArrangementSection,
            ArrangementSectionEvent,
            PhraseMotifEvent,
        )

        builder = PortableScoreBuilder()
        motif = PhraseMotif(
            duration_tick=120,
            events=(PhraseMotifEvent(0, 120, "note", notes=("C4",)),),
        )
        with pytest.raises(ValueError, match="MAX_TRANSPOSE_SEMITONES"):
            builder.apply_motif(motif, "mel", 0, transpose_semitones=10**5000)
        with pytest.raises(ValueError, match="MAX_TRANSPOSE_SEMITONES"):
            builder.apply_motif(
                motif, "mel", 0, transpose_semitones=MAX_TRANSPOSE_SEMITONES + 1
            )
        # In-bounds transpose_semitones=1200 pushing out of valid octave range
        with pytest.raises(ValueError, match="(?i)octave"):
            builder.apply_motif(motif, "mel", 0, transpose_semitones=1200)

        sec = ArrangementSection(
            duration_tick=120,
            events=(ArrangementSectionEvent("mel", 0, 120, "note", notes=("C4",)),),
        )
        with pytest.raises(ValueError, match="MAX_TRANSPOSE_SEMITONES"):
            builder.apply_section(sec, 0, transpose_semitones=10**5000)
        with pytest.raises(ValueError, match="MAX_TRANSPOSE_SEMITONES"):
            builder.apply_section(
                sec, 0, transpose_semitones={"mel": MAX_TRANSPOSE_SEMITONES + 1}
            )

        # velocity_scale mapping enforces [0.0, 1.0] consistently with scalar
        builder.apply_section(sec, 0, velocity_scale={"mel": 0.0})
        with pytest.raises(ValueError, match=r"\[0.0, 1.0\]"):
            builder.apply_section(sec, 0, velocity_scale={"mel": 1.5})

        too_many_tracks = {f"t_{i}": 0 for i in range(MAX_TRACKS + 1)}
        with pytest.raises(ValueError, match="MAX_TRACKS"):
            builder.apply_section(sec, 0, transpose_semitones=too_many_tracks)


class TestCwe674JsonDepthAndRecursionAudit:
    def test_json_depth_cap_enforced_iteratively(self) -> None:
        deep_obj = '{"a":' * (MAX_JSON_DEPTH + 1) + "1" + "}" * (MAX_JSON_DEPTH + 1)
        with pytest.raises(ValueError, match="MAX_JSON_DEPTH"):
            PortableScore.from_json(deep_obj)

        deep_arr = "[" * (MAX_JSON_DEPTH + 1) + "1" + "]" * (MAX_JSON_DEPTH + 1)
        with pytest.raises(ValueError, match="MAX_JSON_DEPTH"):
            PortableScore.from_json(deep_arr)

        # Strings containing braces/brackets must not trigger false depth errors
        valid_json = parse_portable_score(
            f"format: {PORTABLE_FORMAT_VERSION}\n"
            f"title: {'[' * 100}{'}' * 100}\n"
            "instrument lead kind=sine\ntrack mel instrument=lead\n"
            "event mel 0 120 note C4\n"
        ).to_json()
        parsed = PortableScore.from_json(valid_json)
        assert "[" * 100 in parsed.title

        # Invalid max_bytes / max_depth parameters rejected
        with pytest.raises(ValueError, match="MAX_SCORE_BYTES"):
            PortableScore.from_json("{}", max_bytes=MAX_SCORE_BYTES + 1)
        with pytest.raises(ValueError, match="MAX_JSON_DEPTH"):
            PortableScore.from_json("{}", max_depth=MAX_JSON_DEPTH + 1)
        with pytest.raises(ValueError, match="max_sample_count"):
            render_simple_score(Score(events=()), max_sample_count=MAX_SAMPLE_COUNT + 1)
        with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
            render_simple_score(
                Score(events=()), sample_rate_hz=int(MAX_SAMPLE_RATE_HZ) + 1
            )


class TestCwe1333AndCwe674StaticAstAudit:
    def test_zero_regex_eval_exec_and_zero_recursion(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "music_machine"
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
