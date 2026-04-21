from __future__ import annotations

import sys
from types import SimpleNamespace

import pytest
from pcm_audio import PCMBuffer, PCMFormat

from music_machine import (
    HAPPY_BIRTHDAY_TEXT,
    RenderedScore,
    ScoreEvent,
    TextScore,
    beats_for_duration_symbol,
    parse_score,
    play_score,
    play_score_text,
    render_score_to_pcm,
)


def test_duration_symbols_include_dotted_values() -> None:
    assert beats_for_duration_symbol("w") == 4.0
    assert beats_for_duration_symbol("h") == 2.0
    assert beats_for_duration_symbol("q") == 1.0
    assert beats_for_duration_symbol("e") == 0.5
    assert beats_for_duration_symbol("s") == 0.25
    assert beats_for_duration_symbol("q.") == 1.5
    assert beats_for_duration_symbol("h.") == 3.0


def test_duration_symbol_errors_are_useful() -> None:
    with pytest.raises(ValueError, match="unknown duration"):
        beats_for_duration_symbol("thirtysecond")

    with pytest.raises(ValueError, match="at most one dot"):
        beats_for_duration_symbol("q..")

    with pytest.raises(ValueError, match="non-empty"):
        beats_for_duration_symbol("")


def test_parse_score_reads_metadata_notes_rests_and_barlines() -> None:
    score = parse_score(
        """
        title: Tiny Song
        tempo: 120
        meter: 3/4
        amplitude: 0.25
        sample_rate: 8000

        C4/q D4/e | R/e rest/q
        """
    )

    assert score.title == "Tiny Song"
    assert score.tempo_bpm == 120.0
    assert score.meter == "3/4"
    assert score.amplitude == 0.25
    assert score.sample_rate_hz == 8000
    assert score.seconds_per_beat() == 0.5
    assert score.total_duration_seconds() == 1.5
    assert [event.kind for event in score.events] == ["note", "note", "rest", "rest"]
    assert [event.note for event in score.events] == ["C4", "D4", None, None]
    assert [event.source_token for event in score.events] == [
        "C4/q",
        "D4/e",
        "R/e",
        "rest/q",
    ]


def test_parse_score_defaults_metadata() -> None:
    score = parse_score("A4/q")

    assert score.title == "Untitled"
    assert score.tempo_bpm == 120.0
    assert score.meter == "4/4"
    assert score.amplitude == 0.18
    assert score.sample_rate_hz == 44_100


def test_parse_score_uses_known_tempo_for_dotted_duration_math() -> None:
    score = parse_score(
        """
        tempo: 60

        A4/q. R/e
        """
    )

    first, second = score.events
    assert first.beat_count == 1.5
    assert first.duration_seconds == 1.5
    assert second.beat_count == 0.5
    assert second.duration_seconds == 0.5


@pytest.mark.parametrize(
    ("score_text", "message"),
    [
        ("tempo: 0\nA4/q", "tempo must be > 0.0"),
        ("tempo: fast\nA4/q", "tempo must be a number"),
        ("amplitude: 1.5\nA4/q", "amplitude must be in"),
        ("amplitude: loud\nA4/q", "amplitude must be a number"),
        ("sample_rate: 44100.5\nA4/q", "sample_rate must be an integer"),
        ("sample_rate: fast\nA4/q", "sample_rate must be an integer"),
        ("meter: tomato\nA4/q", "meter must look"),
        ("dynamic: loud\nA4/q", "unknown directive"),
        ("title:\nA4/q", "directive 'title' is empty"),
        ("A4", "pitch/duration"),
        ("A4/", "pitch/duration"),
        ("H4/q", "Invalid note"),
        ("A4/z", "unknown duration"),
        ("A4/q\ntempo: 100", "appears after music"),
    ],
)
def test_parse_score_rejects_invalid_scores(score_text: str, message: str) -> None:
    with pytest.raises(ValueError, match=message):
        parse_score(score_text)


def test_parse_score_rejects_empty_score() -> None:
    with pytest.raises(ValueError, match="at least one event"):
        parse_score("title: Empty")


def test_parse_score_requires_string_input() -> None:
    with pytest.raises(ValueError, match="score text must be a string"):
        parse_score(123)  # type: ignore[arg-type]


def test_parse_score_enforces_resource_limits() -> None:
    with pytest.raises(ValueError, match="max_score_length=4"):
        parse_score("A4/q R/q", max_score_length=4)

    with pytest.raises(ValueError, match="max_line_length=3"):
        parse_score("A4/q", max_line_length=3)

    with pytest.raises(ValueError, match="max_event_count=1"):
        parse_score("A4/q R/q", max_event_count=1)


def test_parse_score_validates_resource_limit_values() -> None:
    with pytest.raises(ValueError, match="max_score_length must be an integer"):
        parse_score("A4/q", max_score_length=True)  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="max_line_length must be >= 0"):
        parse_score("A4/q", max_line_length=-1)

    with pytest.raises(ValueError, match="max_event_count must be an integer"):
        parse_score("A4/q", max_event_count=1.5)  # type: ignore[arg-type]


def test_score_event_validates_shape() -> None:
    with pytest.raises(ValueError, match="rest events cannot include"):
        ScoreEvent(
            kind="rest",
            note="A4",
            duration_symbol="q",
            beat_count=1.0,
            duration_seconds=0.5,
            source_token="R/q",
        )

    with pytest.raises(ValueError, match="note events must include"):
        ScoreEvent(
            kind="note",
            note=None,
            duration_symbol="q",
            beat_count=1.0,
            duration_seconds=0.5,
            source_token="/q",
        )

    with pytest.raises(ValueError, match="kind must be"):
        ScoreEvent(
            kind="chord",  # type: ignore[arg-type]
            note="A4",
            duration_symbol="q",
            beat_count=1.0,
            duration_seconds=0.5,
            source_token="A4/q",
        )


def test_text_score_validates_settings_and_events() -> None:
    event = ScoreEvent(
        kind="note",
        note="A4",
        duration_symbol="q",
        beat_count=1.0,
        duration_seconds=0.5,
        source_token="A4/q",
    )

    with pytest.raises(ValueError, match="score must contain"):
        TextScore(
            title="No Events",
            tempo_bpm=120,
            meter="4/4",
            amplitude=0.18,
            sample_rate_hz=8000,
            events=(),
        )

    with pytest.raises(ValueError, match="sample_rate_hz must be > 0"):
        TextScore(
            title="Bad Rate",
            tempo_bpm=120,
            meter="4/4",
            amplitude=0.18,
            sample_rate_hz=0,
            events=(event,),
        )

    with pytest.raises(ValueError, match="tempo_bpm must be a finite real"):
        TextScore(
            title="Bad Tempo",
            tempo_bpm=True,  # type: ignore[arg-type]
            meter="4/4",
            amplitude=0.18,
            sample_rate_hz=8000,
            events=(event,),
        )

    with pytest.raises(ValueError, match="amplitude must be finite"):
        TextScore(
            title="Bad Amplitude",
            tempo_bpm=120,
            meter="4/4",
            amplitude=float("nan"),
            sample_rate_hz=8000,
            events=(event,),
        )

    with pytest.raises(ValueError, match="sample_rate_hz must be an integer"):
        TextScore(
            title="Bad Rate Type",
            tempo_bpm=120,
            meter="4/4",
            amplitude=0.18,
            sample_rate_hz=True,  # type: ignore[arg-type]
            events=(event,),
        )


def test_rendered_score_validates_shape() -> None:
    score = parse_score("A0/q")
    pcm_buffer = PCMBuffer(samples=(), pcm_format=PCMFormat(sample_rate_hz=100))

    with pytest.raises(ValueError, match="score must be a TextScore"):
        RenderedScore(
            score="not a score",  # type: ignore[arg-type]
            pcm_buffer=pcm_buffer,
            rendered_notes=(),
        )

    with pytest.raises(ValueError, match="pcm_buffer must be"):
        RenderedScore(
            score=score,
            pcm_buffer="not pcm",  # type: ignore[arg-type]
            rendered_notes=(),
        )

    with pytest.raises(ValueError, match="rendered_notes\\[0\\]"):
        RenderedScore(
            score=score,
            pcm_buffer=pcm_buffer,
            rendered_notes=("not a note",),  # type: ignore[arg-type]
        )


def test_render_score_to_pcm_joins_notes_and_zero_rest_samples() -> None:
    score = parse_score(
        """
        tempo: 600
        amplitude: 0.25
        sample_rate: 100

        A0/q R/q
        """
    )

    rendered = render_score_to_pcm(score)

    assert rendered.pcm_buffer.sample_count() == 20
    assert len(rendered.rendered_notes) == 1
    assert rendered.rendered_notes[0].frequency_hz == pytest.approx(27.5)
    assert rendered.pcm_buffer.samples[10:] == (0,) * 10


def test_render_score_to_pcm_rejects_sample_budget_overflow() -> None:
    score = parse_score(
        """
        tempo: 600
        sample_rate: 100

        A0/q R/q
        """
    )

    with pytest.raises(ValueError, match="above max_sample_count=19"):
        render_score_to_pcm(score, max_sample_count=19)


def test_render_score_rejects_wrong_input_type() -> None:
    with pytest.raises(ValueError, match="score must be a TextScore"):
        render_score_to_pcm("A4/q")  # type: ignore[arg-type]


def test_happy_birthday_fixture_parses_and_renders() -> None:
    score = parse_score(HAPPY_BIRTHDAY_TEXT)

    assert score.title == "Happy Birthday"
    assert score.meter == "3/4"
    assert len(score.events) == 28
    assert score.events[0].note == "G4"

    rendered = render_score_to_pcm(score)

    assert rendered.pcm_buffer.sample_count() == 595_350
    assert len(rendered.rendered_notes) == 25
    assert rendered.pcm_buffer.duration_seconds() == pytest.approx(13.5)


def test_play_score_delegates_to_injected_sink() -> None:
    score = parse_score(
        """
        tempo: 600
        sample_rate: 100

        A0/q
        """
    )
    seen: dict[str, float] = {}

    def fake_sink(buffer: PCMBuffer) -> str:
        seen["sample_count"] = buffer.sample_count()
        seen["sample_rate"] = buffer.pcm_format.sample_rate_hz
        return "played"

    assert play_score(score, play_pcm_buffer=fake_sink) == "played"
    assert seen == {"sample_count": 10, "sample_rate": 100.0}


def test_play_score_text_delegates_to_injected_sink() -> None:
    seen: dict[str, int] = {}

    def fake_sink(buffer: PCMBuffer) -> str:
        seen["sample_count"] = buffer.sample_count()
        return "played text"

    result = play_score_text(
        """
        tempo: 600
        sample_rate: 100

        A0/q R/q
        """,
        play_pcm_buffer=fake_sink,
    )

    assert result == "played text"
    assert seen == {"sample_count": 20}


def test_play_score_text_imports_default_sink_lazily(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    seen: dict[str, int] = {}

    def fake_sink(buffer: PCMBuffer) -> str:
        seen["sample_count"] = buffer.sample_count()
        return "lazy played"

    monkeypatch.setitem(
        sys.modules,
        "audio_device_sink",
        SimpleNamespace(play_pcm_buffer=fake_sink),
    )

    result = play_score_text(
        """
        tempo: 600
        sample_rate: 100

        A0/q
        """,
    )

    assert result == "lazy played"
    assert seen == {"sample_count": 10}


def test_play_score_text_rejects_non_callable_default_sink(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setitem(
        sys.modules,
        "audio_device_sink",
        SimpleNamespace(play_pcm_buffer="not callable"),
    )

    with pytest.raises(ValueError, match="at least one event"):
        play_score_text("title: empty")

    with pytest.raises(TypeError, match="must be callable"):
        play_score_text(
            """
            tempo: 600
            sample_rate: 100

            A0/q
            """,
        )
