import pytest
from note_frequency import parse_note

from music_machine.score import (
    MAX_SAMPLE_RATE_HZ,
    MusicalEvent,
    Score,
    ScoreParseError,
    parse_score,
    render_score,
    score_duration_seconds,
)


class TestParseScore:
    def test_parses_directives_and_events(self) -> None:
        score = parse_score(
            """
            # Happy Birthday first bar
            title: Birthday Test
            tempo: 120

            A4 0.5
            R 0.25 0.6
            C5 1
            """
        )

        assert score.title == "Birthday Test"
        assert score.tempo_bpm == 120.0
        assert len(score.events) == 3
        assert score.events[0].note is not None
        assert score.events[0].duration_beats == 0.5
        assert score.events[0].velocity == 1.0
        assert score.events[1].note is None
        assert score.events[1].velocity == 0.6

    def test_unknown_directive_errors(self) -> None:
        with pytest.raises(ScoreParseError, match="unknown directive"):
            parse_score("key: C\n")

    def test_malformed_directive_errors(self) -> None:
        with pytest.raises(ScoreParseError, match="malformed directive"):
            parse_score("title:\n")
        with pytest.raises(ScoreParseError, match="malformed directive"):
            parse_score(": value\n")

    def test_invalid_event_fails(self) -> None:
        with pytest.raises(ScoreParseError):
            parse_score("A4\n")

    def test_invalid_event_extra_tokens(self) -> None:
        with pytest.raises(ScoreParseError, match="malformed event line"):
            parse_score("A4 1.0 0.8 extra\n")

    def test_invalid_pitch_fails(self) -> None:
        with pytest.raises(ScoreParseError, match="Invalid note"):
            parse_score("H4 1.0\n")

    def test_invalid_duration_tokens(self) -> None:
        with pytest.raises(ScoreParseError, match="invalid duration"):
            parse_score("A4 fast\n")
        with pytest.raises(ScoreParseError, match="positive finite number"):
            parse_score("A4 0\n")
        with pytest.raises(ScoreParseError, match="positive finite number"):
            parse_score("A4 -1.0\n")

    def test_invalid_velocity_tokens(self) -> None:
        with pytest.raises(ScoreParseError, match="invalid velocity"):
            parse_score("A4 1.0 loud\n")
        with pytest.raises(ScoreParseError, match=r"within \[0.0, 1.0\]"):
            parse_score("A4 1.0 -0.1\n")
        with pytest.raises(ScoreParseError, match=r"within \[0.0, 1.0\]"):
            parse_score("A4 1.0 1.5\n")

    def test_tempo_directive_validations(self) -> None:
        with pytest.raises(ScoreParseError, match="positive finite number"):
            parse_score("tempo: -10\nA4 1.0\n")
        with pytest.raises(ScoreParseError, match="tempo must be a finite number"):
            parse_score("tempo: fast\nA4 1.0\n")

    def test_score_requires_string(self) -> None:
        with pytest.raises(ScoreParseError, match="must be a string"):
            parse_score(123)  # type: ignore[arg-type]

    def test_score_max_bytes_budget(self) -> None:
        with pytest.raises(ScoreParseError, match="exceeds max_score_bytes"):
            parse_score("A4 1.0\n", max_score_bytes=3)

    def test_score_max_events_budget(self) -> None:
        with pytest.raises(ScoreParseError, match="exceeds max_event_count"):
            parse_score("A4 1.0\nB4 1.0\n", max_event_count=1)


class TestDuration:
    def test_score_duration_seconds(self) -> None:
        score = Score(
            tempo_bpm=120.0,
            events=(
                MusicalEvent(note=None, duration_beats=1.0),
                MusicalEvent(note=None, duration_beats=2.0),
            ),
        )
        assert score_duration_seconds(score) == 1.5

    def test_score_duration_seconds_wrong_type(self) -> None:
        with pytest.raises(ScoreParseError, match="must be a Score instance"):
            score_duration_seconds("not a score")  # type: ignore[arg-type]

    def test_event_duration_seconds_validations(self) -> None:
        event = MusicalEvent(note=None, duration_beats=1.0)
        with pytest.raises(ScoreParseError, match="positive finite number"):
            event.duration_seconds(0.0)
        with pytest.raises(ScoreParseError, match="positive finite number"):
            event.duration_seconds(float("nan"))

    def test_score_post_init_validations(self) -> None:
        with pytest.raises(ScoreParseError, match="finite positive number"):
            Score(events=(), tempo_bpm=True)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="positive finite number"):
            Score(events=(), tempo_bpm=0.0)


class TestRenderScore:
    def test_renders_notes_and_rests(self) -> None:
        score = parse_score(
            """
            tempo: 60
            A4 1
            R 1
            B4 1
            """
        )
        samples = render_score(score, sample_rate_hz=8)

        # 3 beats at 1 beat/sec -> 3 seconds at 8 Hz => 24 samples
        assert len(samples) == 24
        assert any(sample != 0.0 for sample in samples[:8])
        assert any(sample == 0.0 for sample in samples[8:16])

    def test_render_score_wrong_type(self) -> None:
        with pytest.raises(ScoreParseError, match="must be a Score instance"):
            render_score("not a score", sample_rate_hz=8)  # type: ignore[arg-type]

    def test_bad_sample_rate_rejected(self) -> None:
        score = Score(tempo_bpm=120.0, events=())
        with pytest.raises(ScoreParseError, match="must be greater than zero"):
            render_score(score, sample_rate_hz=0)
        with pytest.raises(ScoreParseError, match="must be an integer"):
            render_score(score, sample_rate_hz=True)  # type: ignore[arg-type]
        with pytest.raises(ScoreParseError, match="exceeds MAX_SAMPLE_RATE_HZ"):
            render_score(score, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1)

    def test_sample_budget_limits_and_zero_frame_skip(self) -> None:
        note = parse_note("A4")
        score = Score(
            tempo_bpm=60.0,
            events=(
                MusicalEvent(note=note, duration_beats=2.0),
                MusicalEvent(note=None, duration_beats=0.0000001),
            ),
        )
        with pytest.raises(ScoreParseError, match="would require"):
            render_score(score, sample_rate_hz=100, max_sample_count=10)

        # Renders with zero-frame skip succeeding
        samples = render_score(score, sample_rate_hz=10)
        assert len(samples) == 20

    def test_max_events_exceeded(self) -> None:
        from music_machine.score import MAX_EVENTS

        lines = ["title: Long", "tempo: 120"] + ["C4 0.25"] * (MAX_EVENTS + 1)
        with pytest.raises(ScoreParseError, match="event count exceeds MAX_EVENTS"):
            parse_score("\n".join(lines))

    def test_render_score_non_finite_overflow_guard(self) -> None:
        ev = MusicalEvent(note=None, duration_beats=1.0)
        object.__setattr__(ev, "duration_beats", float("nan"))
        score = Score(tempo_bpm=120.0, events=(ev,))
        with pytest.raises(ScoreParseError, match="non-finite sample count"):
            render_score(score, sample_rate_hz=44100)
