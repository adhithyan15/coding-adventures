import pytest

from music_machine.score import (
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

    def test_invalid_event_fails(self) -> None:
        with pytest.raises(ScoreParseError):
            parse_score("A4\n")


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

    def test_bad_sample_rate_rejected(self) -> None:
        score = Score(tempo_bpm=120.0, events=())
        with pytest.raises(ScoreParseError, match="sample_rate_hz"):
            render_score(score, sample_rate_hz=0)
