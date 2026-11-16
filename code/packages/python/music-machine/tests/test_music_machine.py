from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace

import pytest
from pcm_audio import PCMBuffer, PCMFormat

from music_machine import (
    DEFAULT_INSTRUMENT_ID,
    HAPPY_BIRTHDAY_TEXT,
    MAX_DURATION_SECONDS,
    MAX_EVENTS,
    MAX_INSTRUMENTS,
    MAX_SAMPLE_COUNT,
    MAX_SAMPLE_RATE_HZ,
    MAX_SCORE_BYTES,
    MAX_TRACKS,
    MINI_ORCHESTRA_TEXT,
    PITCHED_PERCUSSION_MIX_TEXT,
    ArrangementSection,
    ArrangementSectionEvent,
    InstrumentDeclaration,
    MeterEvent,
    PhraseBuilder,
    PhraseMotif,
    PhraseMotifEvent,
    PortableScore,
    PortableScoreBuilder,
    PortableScoreEvent,
    RenderedPortableScore,
    RenderedScore,
    ScoreEvent,
    TempoEvent,
    TextScore,
    TrackDeclaration,
    beats_for_duration_symbol,
    parse_portable_score,
    parse_portable_score_file,
    parse_score,
    parse_score_file,
    play_portable_score,
    play_portable_score_text,
    play_score,
    play_score_text,
    render_portable_score_to_pcm,
    render_score_to_pcm,
    resolve_instrument_id,
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
        instrument: flute_naive
        sample_rate: 8000

        C4/q D4/e | R/e rest/q
        """
    )

    assert score.title == "Tiny Song"
    assert score.tempo_bpm == 120.0
    assert score.meter == "3/4"
    assert score.amplitude == 0.25
    assert score.instrument_id == "flute_naive"
    assert score.gm_program is None
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
    assert score.instrument_id == DEFAULT_INSTRUMENT_ID
    assert score.gm_program is None
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
        ("instrument: laser_harp\nA4/q", "unknown instrument"),
        ("program: 0\nA4/q", "program must be > 0"),
        ("program: 129\nA4/q", "program must be in"),
        ("program: flute\nA4/q", "program must be an integer"),
        ("instrument: sine\nprogram: 74\nA4/q", "instrument or program"),
        ("program: 74\ninstrument: sine\nA4/q", "instrument or program"),
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
            instrument_id=DEFAULT_INSTRUMENT_ID,
            gm_program=None,
            events=(),
        )

    with pytest.raises(ValueError, match="sample_rate_hz must be > 0"):
        TextScore(
            title="Bad Rate",
            tempo_bpm=120,
            meter="4/4",
            amplitude=0.18,
            sample_rate_hz=0,
            instrument_id=DEFAULT_INSTRUMENT_ID,
            gm_program=None,
            events=(event,),
        )

    with pytest.raises(ValueError, match="tempo_bpm must be a finite real"):
        TextScore(
            title="Bad Tempo",
            tempo_bpm=True,  # type: ignore[arg-type]
            meter="4/4",
            amplitude=0.18,
            sample_rate_hz=8000,
            instrument_id=DEFAULT_INSTRUMENT_ID,
            gm_program=None,
            events=(event,),
        )

    with pytest.raises(ValueError, match="amplitude must be finite"):
        TextScore(
            title="Bad Amplitude",
            tempo_bpm=120,
            meter="4/4",
            amplitude=float("nan"),
            sample_rate_hz=8000,
            instrument_id=DEFAULT_INSTRUMENT_ID,
            gm_program=None,
            events=(event,),
        )

    with pytest.raises(ValueError, match="sample_rate_hz must be an integer"):
        TextScore(
            title="Bad Rate Type",
            tempo_bpm=120,
            meter="4/4",
            amplitude=0.18,
            sample_rate_hz=True,  # type: ignore[arg-type]
            instrument_id=DEFAULT_INSTRUMENT_ID,
            gm_program=None,
            events=(event,),
        )


def test_text_score_resolves_gm_program() -> None:
    event = ScoreEvent(
        kind="note",
        note="A4",
        duration_symbol="q",
        beat_count=1.0,
        duration_seconds=0.5,
        source_token="A4/q",
    )
    score = TextScore(
        title="Flute",
        tempo_bpm=120,
        meter="4/4",
        amplitude=0.18,
        sample_rate_hz=8000,
        instrument_id=DEFAULT_INSTRUMENT_ID,
        gm_program=74,
        events=(event,),
    )

    assert score.gm_program == 74
    assert score.instrument_id == "flute_naive"


def test_resolve_instrument_id_rejects_conflicting_settings() -> None:
    assert resolve_instrument_id(gm_program=74) == "flute_naive"

    with pytest.raises(ValueError, match="instrument or program"):
        resolve_instrument_id(instrument_id="sine", gm_program=74)


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
    assert rendered.rendered_notes[0].fundamental_hz == pytest.approx(27.5)
    assert any(rendered.pcm_buffer.samples[10:13])
    assert rendered.pcm_buffer.samples[13:] == (0,) * 7


def test_render_score_uses_selected_instrument() -> None:
    flute = render_score_to_pcm(
        parse_score(
            """
            tempo: 600
            amplitude: 0.25
            instrument: flute_naive
            sample_rate: 8000

            A4/q
            """
        )
    )
    violin = render_score_to_pcm(
        parse_score(
            """
            tempo: 600
            amplitude: 0.25
            program: 41
            sample_rate: 8000

            A4/q
            """
        )
    )

    assert flute.score.instrument_id == "flute_naive"
    assert violin.score.instrument_id == "violin_naive"
    assert flute.rendered_notes[0].instrument.id == "flute_naive"
    assert violin.rendered_notes[0].instrument.id == "violin_naive"
    assert flute.pcm_buffer.samples != violin.pcm_buffer.samples


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

    assert rendered.pcm_buffer.sample_count() == 596_673
    assert len(rendered.rendered_notes) == 25
    assert rendered.pcm_buffer.duration_seconds() == pytest.approx(13.53)


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
    assert seen == {"sample_count": 13, "sample_rate": 100.0}


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
    assert seen == {"sample_count": 13}


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


PORTABLE_DUET_TEXT = """
format: music-machine-score/v2
title: Tiny Duet
ppq: 100
sample_rate: 1000
tempo 0 600
meter 0 4/4

instrument lead profile=flute_naive gain=0.5
instrument bass program=33 gain=0.4

track melody instrument=lead
track bassline instrument=bass

event melody 0 100 note A4 velocity=0.8
event melody 100 100 note B4 velocity=0.8
event bassline 0 200 note A2,E3 velocity=0.7
"""


def test_parse_portable_score_reads_tracks_instruments_and_events() -> None:
    score = parse_portable_score(PORTABLE_DUET_TEXT)

    assert score.title == "Tiny Duet"
    assert score.ppq == 100
    assert score.sample_rate_hz == 1000
    assert score.tempo_events == (TempoEvent(start_tick=0, bpm=600.0),)
    assert score.meter_events == (MeterEvent(start_tick=0, meter="4/4"),)
    assert score.instruments == (
        InstrumentDeclaration("lead", "flute_naive", 0.5),
        InstrumentDeclaration("bass", "pluck_naive", 0.4),
    )
    assert score.tracks == (
        TrackDeclaration("melody", "lead"),
        TrackDeclaration("bassline", "bass"),
    )
    assert score.events[0] == PortableScoreEvent(
        track_id="melody",
        start_tick=0,
        duration_tick=100,
        kind="note",
        notes=("A4",),
        velocity=0.8,
        source_order=0,
    )
    assert score.events[1].notes == ("A2", "E3")


def test_render_portable_score_mixes_tracks_and_chords() -> None:
    rendered = render_portable_score_to_pcm(parse_portable_score(PORTABLE_DUET_TEXT))

    assert isinstance(rendered, RenderedPortableScore)
    assert rendered.pcm_buffer.sample_count() > 200
    assert len(rendered.rendered_notes) == 4
    assert {note.instrument.id for note in rendered.rendered_notes} == {
        "flute_naive",
        "pluck_naive",
    }
    assert any(rendered.pcm_buffer.samples[:100])
    assert any(rendered.pcm_buffer.samples[100:200])


def test_mini_orchestra_fixture_parses_and_renders() -> None:
    score = parse_portable_score(MINI_ORCHESTRA_TEXT)
    rendered = render_portable_score_to_pcm(score)

    assert score.title == "Mini Orchestra"
    assert len(score.instruments) == 4
    assert len(score.tracks) == 4
    assert len(score.events) == 36
    assert rendered.pcm_buffer.sample_count() > 0
    assert len(rendered.rendered_notes) == 52
    assert any(rendered.pcm_buffer.samples[:10_000])
    assert any(rendered.pcm_buffer.samples[-10_000:])


def test_pitched_percussion_mix_fixture_parses_and_renders() -> None:
    score = parse_portable_score(PITCHED_PERCUSSION_MIX_TEXT)
    rendered = render_portable_score_to_pcm(score)

    assert score.title == "Pitched Percussion Mix"
    assert len(score.instruments) == 5
    assert len(score.tracks) == 5
    assert len(score.events) == 40
    assert {instrument.profile_id for instrument in score.instruments} == {
        "flute_naive",
        "piano_naive",
        "glockenspiel_naive",
        "vibraphone_naive",
        "timpani_naive",
    }
    assert rendered.pcm_buffer.sample_count() > 400_000
    assert rendered.pcm_buffer.clipped_sample_count == 0
    assert len(rendered.rendered_notes) == 56
    assert any(rendered.pcm_buffer.samples[:20_000])
    assert any(rendered.pcm_buffer.samples[-20_000:])


def test_portable_score_builder_builds_round_trippable_score_text() -> None:
    builder = PortableScoreBuilder(title="Builder Song", ppq=100, sample_rate_hz=2000)
    builder.add_tempo(0, 600)
    builder.add_meter(0, "4/4")
    builder.add_instrument("lead", kind="sine", gain=0.5)
    builder.add_instrument("bass", program=33, gain=0.4)
    builder.add_track("melody", instrument_id="lead")
    builder.add_track("low", instrument_id="bass")
    builder.add_note("melody", 0, 100, "A4", velocity=0.8)
    builder.add_chord("melody", 100, 100, ("C5", "E5"), velocity=0.6)
    builder.add_rest("melody", 200, 50)
    builder.add_note("low", 0, 250, "A2", velocity=0.7)

    score = builder.build()
    rebuilt = parse_portable_score(builder.to_text())

    assert score.title == "Builder Song"
    assert score.instruments[1] == InstrumentDeclaration("bass", "pluck_naive", 0.4)
    assert any(event.notes == ("C5", "E5") for event in score.events)
    assert any(event.kind == "rest" for event in score.events)
    assert rebuilt.title == score.title
    assert rebuilt.ppq == score.ppq
    assert rebuilt.sample_rate_hz == score.sample_rate_hz
    assert rebuilt.instruments == score.instruments
    assert rebuilt.tracks == score.tracks
    assert [
        (
            event.track_id,
            event.start_tick,
            event.duration_tick,
            event.kind,
            event.notes,
            event.velocity,
        )
        for event in rebuilt.events
    ] == [
        (
            event.track_id,
            event.start_tick,
            event.duration_tick,
            event.kind,
            event.notes,
            event.velocity,
        )
        for event in score.events
    ]


def test_portable_score_builder_requires_one_instrument_selector() -> None:
    builder = PortableScoreBuilder()

    with pytest.raises(ValueError, match="exactly one"):
        builder.add_instrument("lead", profile="flute_naive", kind="sine")

    with pytest.raises(ValueError, match="exactly one"):
        builder.add_instrument("lead")


def test_portable_score_builder_rejects_string_chords() -> None:
    builder = PortableScoreBuilder()

    with pytest.raises(ValueError, match="iterable of note strings"):
        builder.add_chord("melody", 0, 100, "C5,E5")  # type: ignore[arg-type]


def test_portable_score_builder_measure_helpers() -> None:
    builder = PortableScoreBuilder(ppq=120)
    builder.add_meter(0, "3/4")

    assert builder.beats_to_ticks(1.5) == 180
    assert builder.measure_ticks() == 360
    assert builder.measure_start_tick(3) == 720
    assert builder.tick_in_measure(2, 1.5) == 540


def test_phrase_builder_sequences_musical_time() -> None:
    builder = PortableScoreBuilder(title="Phrase Song", ppq=120, sample_rate_hz=2000)
    builder.add_tempo(0, 600)
    builder.add_meter(0, "4/4")
    builder.add_instrument("lead", kind="sine", gain=0.5)
    builder.add_track("melody", instrument_id="lead")

    phrase = builder.phrase("melody", measure_number=2, beat_offset=1.0)
    assert isinstance(phrase, PhraseBuilder)

    phrase.note("A4", 1.0, velocity=0.8).rest(0.5).chord(
        ("C5", "E5"),
        0.5,
        velocity=0.6,
    )

    score = builder.build()
    events = score.events

    assert events[0] == PortableScoreEvent(
        track_id="melody",
        start_tick=600,
        duration_tick=120,
        kind="note",
        notes=("A4",),
        velocity=0.8,
        source_order=0,
    )
    assert events[1].kind == "rest"
    assert events[1].start_tick == 720
    assert events[1].duration_tick == 60
    assert events[2].notes == ("C5", "E5")
    assert events[2].start_tick == 780
    assert phrase.current_tick == 840


def test_phrase_builder_jump_and_advance() -> None:
    builder = PortableScoreBuilder(ppq=120)
    phrase = builder.phrase("melody")

    phrase.advance_beats(1.5)
    assert phrase.current_tick == 180

    phrase.jump_to_measure(3, 0.5, meter="3/4")
    assert phrase.current_tick == 780


def test_portable_score_builder_captures_and_reapplies_motif() -> None:
    builder = PortableScoreBuilder(title="Motif Song", ppq=120, sample_rate_hz=2000)
    builder.add_tempo(0, 600)
    builder.add_meter(0, "4/4")
    builder.add_instrument("lead", kind="sine", gain=0.5)
    builder.add_track("melody", instrument_id="lead")

    phrase = builder.phrase("melody")
    phrase.note("C4", 1.0, velocity=0.8).rest(0.5).chord(
        ("E4", "G4"),
        0.5,
        velocity=0.6,
    )
    motif = phrase.motif()

    assert motif == PhraseMotif(
        duration_tick=240,
        events=(
            PhraseMotifEvent(0, 120, "note", ("C4",), 0.8),
            PhraseMotifEvent(120, 60, "rest", (), 1.0),
            PhraseMotifEvent(180, 60, "note", ("E4", "G4"), 0.6),
        ),
    )

    builder.apply_motif(
        motif,
        "melody",
        480,
        transpose_semitones=12,
        velocity_scale=0.5,
        repeat_count=2,
    )

    score = builder.build()
    note_events = [event for event in score.events if event.kind == "note"]
    rest_events = [event for event in score.events if event.kind == "rest"]

    assert note_events[2].start_tick == 480
    assert note_events[2].notes == ("C5",)
    assert note_events[2].velocity == 0.4
    assert note_events[3].notes == ("E5", "G5")
    assert note_events[3].start_tick == 660
    assert note_events[4].start_tick == 720
    assert rest_events[1].start_tick == 600


def test_phrase_builder_applies_motif_and_advances_cursor() -> None:
    builder = PortableScoreBuilder(ppq=120)
    builder.add_instrument("lead", kind="sine", gain=0.5)
    builder.add_track("melody", instrument_id="lead")

    motif = PhraseMotif(
        duration_tick=180,
        events=(PhraseMotifEvent(0, 120, "note", ("A4",), 0.8),),
    )

    phrase = builder.phrase("melody", measure_number=2)
    phrase.apply_motif(motif, repeat_count=2)

    score = builder.build()
    assert [event.start_tick for event in score.events] == [480, 660]
    assert phrase.current_tick == 840


def test_portable_score_builder_captures_and_reapplies_section() -> None:
    builder = PortableScoreBuilder(title="Section Song", ppq=120, sample_rate_hz=2000)
    builder.add_tempo(0, 600)
    builder.add_meter(0, "4/4")
    builder.add_instrument("lead", kind="sine", gain=0.5)
    builder.add_instrument("bass", kind="sine", gain=0.4)
    builder.add_track("melody", instrument_id="lead")
    builder.add_track("bassline", instrument_id="bass")
    builder.add_track("answer", instrument_id="lead")
    builder.add_track("low_answer", instrument_id="bass")

    builder.phrase("melody").note("C4", 1.0, velocity=0.8).note(
        "D4",
        1.0,
        velocity=0.7,
    )
    builder.phrase("bassline").note("C2", 2.0, velocity=0.6)

    section = builder.capture_section(start_tick=0, end_tick=240)

    assert section == ArrangementSection(
        duration_tick=240,
        events=(
            ArrangementSectionEvent("bassline", 0, 240, "note", ("C2",), 0.6),
            ArrangementSectionEvent("melody", 0, 120, "note", ("C4",), 0.8),
            ArrangementSectionEvent("melody", 120, 120, "note", ("D4",), 0.7),
        ),
    )

    builder.apply_section(
        section,
        480,
        track_map={"melody": "answer", "bassline": "low_answer"},
        transpose_semitones={"melody": 12},
        velocity_scale={"melody": 0.5, "bassline": 0.75},
    )

    score = builder.build()
    answer_events = [event for event in score.events if event.track_id == "answer"]
    low_answer_events = [
        event for event in score.events if event.track_id == "low_answer"
    ]

    answer_summary = [
        (event.track_id, event.start_tick, event.notes, event.source_order)
        for event in answer_events
    ]
    assert answer_summary == [
        ("answer", 480, ("C5",), 4),
        ("answer", 600, ("D5",), 5),
    ]
    assert answer_events[0].velocity == pytest.approx(0.4)
    assert answer_events[1].velocity == pytest.approx(0.35)

    assert len(low_answer_events) == 1
    assert low_answer_events[0].track_id == "low_answer"
    assert low_answer_events[0].start_tick == 480
    assert low_answer_events[0].duration_tick == 240
    assert low_answer_events[0].notes == ("C2",)
    assert low_answer_events[0].velocity == pytest.approx(0.45)
    assert low_answer_events[0].source_order == 3


def test_portable_score_builder_repeats_section_with_scalar_transpose() -> None:
    builder = PortableScoreBuilder(ppq=120)
    builder.add_instrument("lead", kind="sine", gain=0.5)
    builder.add_track("melody", instrument_id="lead")

    section = ArrangementSection(
        duration_tick=120,
        events=(ArrangementSectionEvent("melody", 0, 120, "note", ("A4",), 0.8),),
    )

    builder.apply_section(
        section,
        240,
        transpose_semitones=12,
        velocity_scale=0.5,
        repeat_count=2,
    )

    score = builder.build()
    assert score.events == (
        PortableScoreEvent("melody", 240, 120, "note", ("A5",), 0.4, 0),
        PortableScoreEvent("melody", 360, 120, "note", ("A5",), 0.4, 1),
    )


def test_capture_section_rejects_events_that_extend_past_the_end() -> None:
    builder = PortableScoreBuilder(ppq=120)
    builder.add_instrument("lead", kind="sine", gain=0.5)
    builder.add_track("melody", instrument_id="lead")
    builder.add_note("melody", 0, 180, "C4")

    with pytest.raises(ValueError, match="extends beyond end_tick"):
        builder.capture_section(start_tick=0, end_tick=120)


def test_render_portable_score_keeps_later_scheduled_notes_audible() -> None:
    score = parse_portable_score(
        """
        format: music-machine-score/v2
        ppq: 100
        sample_rate: 2000
        tempo 0 600
        instrument lead kind=sine gain=0.5
        track melody instrument=lead
        event melody 0 100 note A4 velocity=0.8
        event melody 300 100 note C5 velocity=0.8
        """
    )

    rendered = render_portable_score_to_pcm(score)

    assert any(rendered.pcm_buffer.samples[:200])
    assert any(rendered.pcm_buffer.samples[600:800])


def test_portable_score_supports_tempo_changes() -> None:
    score = parse_portable_score(
        """
        format: music-machine-score/v2
        ppq: 100
        sample_rate: 1000
        tempo 0 600
        tempo 100 300
        instrument lead kind=sine gain=0.5
        track melody instrument=lead
        event melody 50 100 note A4 velocity=0.5
        """
    )

    rendered = render_portable_score_to_pcm(score)

    assert rendered.rendered_notes[0].floating_samples.start_time_seconds == 0.05
    duration = rendered.rendered_notes[0].floating_samples.duration_seconds()
    assert duration == pytest.approx(0.15 + 0.03)


@pytest.mark.parametrize(
    ("text", "message"),
    [
        ("title: no format", "first directive"),
        ("format: music-machine-score/v2\nunknown: yep", "unknown directive"),
        ("format: music-machine-score/v2\ntempo 0", "tempo lines"),
        ("format: music-machine-score/v2\nmeter 0 tomato", "meter must look"),
        (
            "format: music-machine-score/v2\ninstrument lead profile=laser",
            "unknown instrument",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead kind=sample",
            "kind must be",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead kind=sine gain=loud",
            "gain must be a number",
        ),
        (
            "format: music-machine-score/v2\ntrack melody",
            "track lines",
        ),
        (
            "format: music-machine-score/v2\ntrack melody instrument=missing",
            "unknown instrument",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody instrument=lead\n"
            "event missing 0 100 note A4",
            "unknown track",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody instrument=lead\n"
            "event melody 0 0 note A4",
            "duration_tick must be > 0",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody instrument=lead\n"
            "event melody 0 100 note",
            "pitch list",
        ),
    ],
)
def test_parse_portable_score_rejects_invalid_input(
    text: str,
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        parse_portable_score(text)


@pytest.mark.parametrize(
    ("text", "message"),
    [
        (
            "format: music-machine-score/v2\ninstrument lead badtoken",
            "key=value",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead kind=",
            "key=value",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead kind=sine kind=sine",
            "duplicate property",
        ),
        (
            "format: music-machine-score/v2\ninstrument 1lead kind=sine",
            "identifier",
        ),
        (
            "format: music-machine-score/v2\ntempo start 120",
            "tempo start_tick must be an integer",
        ),
        (
            "format: music-machine-score/v2\ntempo -1 120",
            "tempo start_tick must be >= 0",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead program=1 profile=sine",
            "only one",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead profile=sine kind=sine",
            "only one",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead gain=0.5",
            "must include program, profile, or kind",
        ),
        (
            "format: music-machine-score/v2\ninstrument lead kind=sine color=blue",
            "unknown instrument properties",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody instrument=lead gain=1.0",
            "unknown track properties",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody color=blue",
            "unknown track properties",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody instrument=lead\n"
            "event melody 0 100 rest velocity=1.0",
            "rest events must not include",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody instrument=lead\n"
            "event melody 0 100 chord A4",
            "event kind must be",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead kind=sine\n"
            "track melody instrument=lead\n"
            "event melody 0 100 note A4 color=blue",
            "unknown event properties",
        ),
    ],
)
def test_parse_portable_score_rejects_more_invalid_input(
    text: str,
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        parse_portable_score(text)


def test_parse_portable_score_validates_resource_limits() -> None:
    with pytest.raises(ValueError, match="score text must be a string"):
        parse_portable_score(123)  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="max_score_length=4"):
        parse_portable_score("format: music-machine-score/v2", max_score_length=4)

    with pytest.raises(ValueError, match="max_line_length=3"):
        parse_portable_score("format: music-machine-score/v2", max_line_length=3)

    score_text = """
    format: music-machine-score/v2
    instrument lead kind=sine
    track melody instrument=lead
    event melody 0 100 note A4
    """
    with pytest.raises(ValueError, match="max_event_count=0"):
        parse_portable_score(score_text, max_event_count=0)

    with pytest.raises(ValueError, match="first directive"):
        parse_portable_score("")


def test_parse_portable_score_defaults_tempo_and_meter() -> None:
    score = parse_portable_score(
        """
        format: music-machine-score/v2
        instrument lead kind=sine
        track melody instrument=lead
        event melody 0 100 note A4
        """
    )

    assert score.tempo_events == (TempoEvent(0, 120.0),)
    assert score.meter_events == (MeterEvent(0, "4/4"),)


def test_portable_score_validates_duplicate_ids() -> None:
    with pytest.raises(ValueError, match="instrument ids"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="bad",
            ppq=100,
            sample_rate_hz=1000,
            tempo_events=(),
            meter_events=(),
            instruments=(
                InstrumentDeclaration("lead", "sine"),
                InstrumentDeclaration("lead", "sine"),
            ),
            tracks=(),
            events=(),
        )

    with pytest.raises(ValueError, match="format_version"):
        PortableScore(
            format_version="music-machine-score/v3",
            title="bad",
            ppq=100,
            sample_rate_hz=1000,
            tempo_events=(),
            meter_events=(),
            instruments=(),
            tracks=(),
            events=(),
        )

    with pytest.raises(ValueError, match="track ids"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="bad",
            ppq=100,
            sample_rate_hz=1000,
            tempo_events=(),
            meter_events=(),
            instruments=(InstrumentDeclaration("lead", "sine"),),
            tracks=(
                TrackDeclaration("melody", "lead"),
                TrackDeclaration("melody", "lead"),
            ),
            events=(),
        )


def test_portable_score_event_validates_shape() -> None:
    with pytest.raises(ValueError, match="kind must be"):
        PortableScoreEvent(
            track_id="melody",
            start_tick=0,
            duration_tick=100,
            kind="chord",  # type: ignore[arg-type]
            notes=("A4",),
        )

    with pytest.raises(ValueError, match="at least one note"):
        PortableScoreEvent(
            track_id="melody",
            start_tick=0,
            duration_tick=100,
            kind="note",
            notes=(),
        )

    with pytest.raises(ValueError, match="rest events cannot include"):
        PortableScoreEvent(
            track_id="melody",
            start_tick=0,
            duration_tick=100,
            kind="rest",
            notes=("A4",),
        )


def test_render_portable_score_rejects_wrong_type_and_sample_overflow() -> None:
    with pytest.raises(ValueError, match="PortableScore"):
        render_portable_score_to_pcm("not a score")  # type: ignore[arg-type]

    score = parse_portable_score(PORTABLE_DUET_TEXT)
    with pytest.raises(ValueError, match="max_sample_count"):
        render_portable_score_to_pcm(score, max_sample_count=10)


def test_render_portable_score_rejects_long_rest_before_allocation() -> None:
    score = parse_portable_score(
        """
        format: music-machine-score/v2
        ppq: 100
        sample_rate: 1000
        tempo 0 600
        instrument lead kind=sine
        track melody instrument=lead
        event melody 0 10000 rest
        """
    )

    with pytest.raises(ValueError, match="max_sample_count=100"):
        render_portable_score_to_pcm(score, max_sample_count=100)


def test_render_portable_score_counts_clipped_samples() -> None:
    score = parse_portable_score(
        """
        format: music-machine-score/v2
        ppq: 100
        sample_rate: 1000
        tempo 0 600
        instrument one kind=sine gain=1.0
        instrument two kind=sine gain=1.0
        track melody instrument=one
        track harmony instrument=two
        event melody 0 100 note A4 velocity=1.0
        event harmony 0 100 note A4 velocity=1.0
        """
    )

    rendered = render_portable_score_to_pcm(score)

    assert rendered.pcm_buffer.clipped_sample_count > 0


def test_rendered_portable_score_validates_shape() -> None:
    score = parse_portable_score(PORTABLE_DUET_TEXT)
    pcm_buffer = PCMBuffer(samples=(), pcm_format=PCMFormat(sample_rate_hz=1000))

    with pytest.raises(ValueError, match="score must be a PortableScore"):
        RenderedPortableScore(
            score="not a score",  # type: ignore[arg-type]
            pcm_buffer=pcm_buffer,
            rendered_notes=(),
        )

    with pytest.raises(ValueError, match="pcm_buffer must be"):
        RenderedPortableScore(
            score=score,
            pcm_buffer="not pcm",  # type: ignore[arg-type]
            rendered_notes=(),
        )

    with pytest.raises(ValueError, match="rendered_notes\\[0\\]"):
        RenderedPortableScore(
            score=score,
            pcm_buffer=pcm_buffer,
            rendered_notes=("not a note",),  # type: ignore[arg-type]
        )


def test_play_portable_score_delegates_to_injected_sink() -> None:
    score = parse_portable_score(PORTABLE_DUET_TEXT)
    seen: dict[str, int] = {}

    def fake_sink(buffer: PCMBuffer) -> str:
        seen["sample_count"] = buffer.sample_count()
        return "portable played"

    assert play_portable_score(score, play_pcm_buffer=fake_sink) == "portable played"
    assert seen["sample_count"] > 200


def test_play_portable_score_text_delegates_to_injected_sink() -> None:
    seen: dict[str, int] = {}

    def fake_sink(buffer: PCMBuffer) -> str:
        seen["sample_count"] = buffer.sample_count()
        return "portable text played"

    result = play_portable_score_text(
        PORTABLE_DUET_TEXT,
        play_pcm_buffer=fake_sink,
    )

    assert result == "portable text played"


# =============================================================================
# Hardened Security & Determinism Audits (CWE-1333 / CWE-400 / CWE-20 / CWE-22)
# =============================================================================


def test_ast_audit_zero_regex_and_eval() -> None:
    import ast
    from pathlib import Path

    src_root = Path(__file__).resolve().parent.parent / "src" / "music_machine"
    assert src_root.is_dir()
    py_files = list(src_root.glob("*.py"))
    assert len(py_files) >= 3

    for py_file in py_files:
        content = py_file.read_text(encoding="utf-8")
        tree = ast.parse(content, filename=str(py_file))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"Found 'import re' in {py_file.name}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", (
                    f"Found 'from re import ...' in {py_file.name}"
                )
            elif isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                assert node.func.id not in {"eval", "exec"}, (
                    f"Found forbidden call {node.func.id}() in {py_file.name}"
                )


def test_additive_instrument_parsing_and_rendering() -> None:
    score_text = """format: music-machine-score/v2
title: Additive Test
ppq: 120
sample_rate: 8000
tempo 0 120
meter 0 4/4

instrument synth kind=additive harmonics=1:1.0,2:0.5 envelope=0.01,0.05,0.7,0.1 gain=0.6
track lead instrument=synth
event lead 0 120 note C4 velocity=0.8
"""
    score = parse_portable_score(score_text)
    assert len(score.instruments) == 1
    inst = score.instruments[0]
    assert inst.kind == "additive"
    assert inst.custom_profile is not None
    assert inst.harmonics is not None
    assert inst.harmonics[0].frequency_multiplier == 1.0
    assert inst.harmonics[0].amplitude == 1.0
    assert inst.harmonics[1].frequency_multiplier == 2.0
    assert inst.harmonics[1].amplitude == 0.5
    assert inst.envelope is not None
    assert inst.envelope.attack_seconds == 0.01

    rendered = render_portable_score_to_pcm(score)
    assert rendered.pcm_buffer.sample_count() > 0
    assert any(s != 0 for s in rendered.pcm_buffer.samples)


def test_drumkit_parsing_and_hihat_choke() -> None:
    score_text = """format: music-machine-score/v2
title: Drum Test
ppq: 120
sample_rate: 8000
tempo 0 120
meter 0 4/4

instrument drums kind=drumkit kit=standard gain=0.8
track rhythm instrument=drums
event rhythm 0 120 hit open_hihat velocity=0.8
event rhythm 60 60 hit closed_hihat velocity=0.8
"""
    score = parse_portable_score(score_text)
    assert len(score.instruments) == 1
    inst = score.instruments[0]
    assert inst.kind == "drumkit"
    assert inst.kit == "standard_kit"
    assert len(score.events) == 2
    assert score.events[0].kind == "hit"
    assert score.events[0].hit_id == "open_hihat"
    assert score.events[1].hit_id == "closed_hihat"

    rendered = render_portable_score_to_pcm(score)
    assert rendered.pcm_buffer.sample_count() > 0
    assert len(rendered.rendered_hits) == 2


def test_drumkit_and_pitched_track_mismatch_fails() -> None:
    bad_note_on_drum = """format: music-machine-score/v2
ppq: 120
sample_rate: 8000
instrument drums kind=drumkit
track rhythm instrument=drums
event rhythm 0 120 note C4 velocity=0.8
"""
    with pytest.raises(ValueError, match="cannot play note events"):
        render_portable_score_to_pcm(parse_portable_score(bad_note_on_drum))

    bad_hit_on_pitched = """format: music-machine-score/v2
ppq: 120
sample_rate: 8000
instrument lead kind=sine
track melody instrument=lead
event melody 0 120 hit snare velocity=0.8
"""
    with pytest.raises(ValueError, match="cannot play hit events"):
        render_portable_score_to_pcm(parse_portable_score(bad_hit_on_pitched))


def test_portable_score_json_roundtrip_and_rejections() -> None:
    import json

    score = parse_portable_score(PORTABLE_DUET_TEXT)
    as_dict = score.to_dict()
    assert as_dict["format_version"] == "music-machine-score/v2"
    assert as_dict["title"] == "Tiny Duet"

    rebuilt = PortableScore.from_dict(as_dict)
    assert rebuilt == score

    json_str = score.to_json()
    assert isinstance(json_str, str)
    from_json_score = PortableScore.from_json(json_str)
    assert from_json_score == score

    # Reject non-string
    with pytest.raises(ValueError, match="json_text must be a string"):
        PortableScore.from_json(123)  # type: ignore[arg-type]

    # Reject oversized JSON
    with pytest.raises(ValueError, match="exceeds max_bytes"):
        score.from_json(json_str, max_bytes=10)

    # Reject non-finite constants
    nan_json = json.dumps({"title": "NaN", "tempo": float("nan")})
    with pytest.raises(ValueError, match="non-finite"):
        PortableScore.from_json(nan_json)

    inf_json = '{"title": "Inf", "tempo": Infinity}'
    with pytest.raises(ValueError, match="non-finite|Expecting value"):
        PortableScore.from_json(inf_json)

    # Reject invalid dict schema
    with pytest.raises(ValueError, match="dictionary|mapping"):
        PortableScore.from_dict([])  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="format_version"):
        PortableScore.from_dict({"format_version": "unsupported/v99"})


def test_file_path_validation_and_cwe22(tmp_path: pytest.TempPathFactory) -> None:
    from pathlib import Path

    # Null bytes rejection
    with pytest.raises(ValueError, match="null byte"):
        parse_score_file("test\0bad.txt")
    with pytest.raises(ValueError, match="null byte"):
        parse_portable_score_file("test\0bad.txt")

    # Non-existent file rejection
    with pytest.raises(FileNotFoundError):
        parse_score_file("nonexistent_score_file_12345.txt")
    with pytest.raises(FileNotFoundError):
        parse_portable_score_file("nonexistent_score_file_12345.txt")

    # Valid file reading
    temp_dir = Path(str(tmp_path))
    score_file = temp_dir / "valid_score.txt"
    score_file.write_text("A4/q B4/q", encoding="utf-8")
    loaded = parse_score_file(score_file)
    assert len(loaded.events) == 2

    port_file = temp_dir / "valid_port.txt"
    port_file.write_text(PORTABLE_DUET_TEXT, encoding="utf-8")
    loaded_port = parse_portable_score_file(port_file)
    assert loaded_port.title == "Tiny Duet"


class SplitMix64:
    """Minimal SplitMix64 PRNG for deterministic property-based testing."""

    def __init__(self, seed: int) -> None:
        self.state = seed & 0xFFFFFFFFFFFFFFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = self.state
        z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9 & 0xFFFFFFFFFFFFFFFF
        z = (z ^ (z >> 27)) * 0x94D049BB133111EB & 0xFFFFFFFFFFFFFFFF
        return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF

    def next_int(self, low: int, high: int) -> int:
        return low + (self.next_u64() % (high - low + 1))


def test_splitmix64_property_based_rendering() -> None:
    notes = ["C4", "D4", "E4", "F4", "G4", "A4", "B4"]
    for seed in [1337, 42, 9999]:
        rng = SplitMix64(seed)
        builder = PortableScoreBuilder(
            title=f"PBT_{seed}", ppq=120, sample_rate_hz=4000
        )
        builder.add_tempo(0, rng.next_int(80, 160))
        builder.add_meter(0, "4/4")
        builder.add_instrument("lead", kind="sine", gain=0.4)
        builder.add_track("melody", instrument_id="lead")

        current_tick = 0
        for _ in range(5):
            note = notes[rng.next_int(0, len(notes) - 1)]
            dur = rng.next_int(60, 120)
            vel = 0.5 + (rng.next_int(0, 50) / 100.0)
            builder.add_note("melody", current_tick, dur, note, velocity=vel)
            current_tick += dur

        score1 = builder.build()
        render1 = render_portable_score_to_pcm(score1)
        render2 = render_portable_score_to_pcm(score1)

        # Exact bit-for-bit determinism
        assert render1.pcm_buffer.samples == render2.pcm_buffer.samples
        assert all(-32768 <= s <= 32767 for s in render1.pcm_buffer.samples)


def test_hostile_inputs_and_bounds() -> None:
    # 0. Assert resource bounds constants
    assert MAX_DURATION_SECONDS == 600.0
    assert MAX_EVENTS == 10_000
    assert MAX_INSTRUMENTS == 128
    assert MAX_TRACKS == 128
    assert MAX_SCORE_BYTES == 1_000_000

    # 1. Oversized score text
    oversized = "A4/q\n" * 300_000
    with pytest.raises(ValueError, match="exceeds"):
        parse_score(oversized, max_score_bytes=100)

    # 2. Exceeding MAX_SAMPLE_RATE_HZ
    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        parse_score(f"sample_rate: {int(MAX_SAMPLE_RATE_HZ + 1)}\nA4/q")

    # 3. Exceeding MAX_DURATION_SECONDS
    score_long = parse_score(
        "tempo: 0.1\nsample_rate: 100\nA4/w\n",
        max_event_count=100,
    )
    with pytest.raises(ValueError, match="MAX_DURATION_SECONDS"):
        render_score_to_pcm(score_long, max_sample_count=MAX_SAMPLE_COUNT)

    # 4. Hostile ReDoS-style line with repeated spaces
    adversarial = " " * 8_000 + "A4/q"
    parsed = parse_score(adversarial)
    assert len(parsed.events) == 1

    oversized_line = " " * 15_000 + "A4/q"
    with pytest.raises(ValueError, match="exceeds max_line_length"):
        parse_score(oversized_line)

    # 5. Invalid identifier characters
    with pytest.raises(ValueError, match="identifier"):
        InstrumentDeclaration("bad id with spaces", "sine")


def test_comprehensive_defensive_validations_and_scanners() -> None:
    from music_machine.music_machine import (
        MAX_NOTE_DURATION_SECONDS,
        MAX_PARTIALS,
        _finite_float,
        _integer_directive,
        _integer_valued_sample_rate,
        _is_identifier,
        _is_valid_meter,
        _non_negative_float,
        _non_negative_integer,
        _parse_directive_line,
        _parse_envelope,
        _parse_float_directive,
        _parse_harmonics,
        _parse_properties,
        _parse_tick,
        _positive_float,
        _positive_integer,
        _safe_round_sample,
        _signed_integer,
        _unit_float,
        _validate_identifier,
        _validate_meter,
    )

    # _finite_float
    assert _finite_float("x", 42) == 42.0
    with pytest.raises(ValueError, match="finite real"):
        _finite_float("x", True)
    with pytest.raises(ValueError, match="finite real"):
        _finite_float("x", "42")
    with pytest.raises(ValueError, match="finite"):
        _finite_float("x", float("nan"))
    with pytest.raises(ValueError, match="finite"):
        _finite_float("x", float("inf"))

    # _positive_float
    assert _positive_float("x", 0.5) == 0.5
    with pytest.raises(ValueError, match="> 0.0"):
        _positive_float("x", 0.0)
    with pytest.raises(ValueError, match="> 0.0"):
        _positive_float("x", -1.0)

    # _non_negative_float
    assert _non_negative_float("x", 0.0) == 0.0
    with pytest.raises(ValueError, match=">= 0.0"):
        _non_negative_float("x", -0.1)

    # _unit_float
    assert _unit_float("x", 0.0) == 0.0
    assert _unit_float("x", 1.0) == 1.0
    with pytest.raises(ValueError, match=r"\[0.0, 1.0\]"):
        _unit_float("x", -0.1)
    with pytest.raises(ValueError, match=r"\[0.0, 1.0\]"):
        _unit_float("x", 1.1)

    # _positive_integer
    assert _positive_integer("x", 10) == 10
    with pytest.raises(ValueError, match="integer > 0"):
        _positive_integer("x", True)
    with pytest.raises(ValueError, match="integer > 0"):
        _positive_integer("x", 1.5)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="> 0"):
        _positive_integer("x", 0)

    # _non_negative_integer
    assert _non_negative_integer("x", 0) == 0
    with pytest.raises(ValueError, match="integer >= 0"):
        _non_negative_integer("x", False)
    with pytest.raises(ValueError, match=">= 0"):
        _non_negative_integer("x", -1)

    # _signed_integer
    assert _signed_integer("x", -5) == -5
    with pytest.raises(ValueError, match="integer"):
        _signed_integer("x", True)

    # _integer_valued_sample_rate
    assert _integer_valued_sample_rate("44100") == 44100
    with pytest.raises(ValueError, match="sample_rate must be an integer"):
        _integer_valued_sample_rate("fast")
    with pytest.raises(ValueError, match="sample_rate must be an integer"):
        _integer_valued_sample_rate("44100.5")
    with pytest.raises(ValueError, match="sample_rate must be > 0"):
        _integer_valued_sample_rate("0")
    with pytest.raises(ValueError, match="exceeds MAX_SAMPLE_RATE_HZ"):
        _integer_valued_sample_rate("200000")

    # _integer_directive
    assert _integer_directive("ppq", "120") == 120
    with pytest.raises(ValueError, match="ppq must be an integer"):
        _integer_directive("ppq", "abc")
    with pytest.raises(ValueError, match="ppq must be an integer"):
        _integer_directive("ppq", "120.4")

    # _parse_float_directive
    assert _parse_float_directive("gain", "0.5") == 0.5
    with pytest.raises(ValueError, match="gain must be a number"):
        _parse_float_directive("gain", "loud")
    with pytest.raises(ValueError, match="gain must be a number"):
        _parse_float_directive("gain", "nan")

    # _parse_directive_line
    assert _parse_directive_line("title: Foo") == ("title", "Foo")
    assert _parse_directive_line("no_colon") is None
    assert _parse_directive_line(": empty_key") is None
    assert _parse_directive_line("123bad: val") is None
    assert _parse_directive_line("bad key: val") is None

    # _is_identifier & _validate_identifier
    assert _is_identifier("valid_id-1")
    assert not _is_identifier("")
    assert not _is_identifier("123bad")
    assert not _is_identifier("has space")
    assert _validate_identifier("track", "lead_1") == "lead_1"
    with pytest.raises(ValueError, match="must be an identifier"):
        _validate_identifier("track", "bad id")

    # _is_valid_meter & _validate_meter
    assert _is_valid_meter("4/4")
    assert _is_valid_meter("6/8")
    assert not _is_valid_meter("4")
    assert not _is_valid_meter("4/4/4")
    assert not _is_valid_meter("0/4")
    assert not _is_valid_meter("4/0")
    assert not _is_valid_meter("a/b")
    assert _validate_meter("3/4") == "3/4"
    with pytest.raises(ValueError, match="meter must look like"):
        _validate_meter("tomato")

    # _parse_properties
    assert _parse_properties(["gain=0.5", "profile=flute_naive"]) == {
        "gain": "0.5",
        "profile": "flute_naive",
    }
    with pytest.raises(ValueError, match="must be key=value"):
        _parse_properties(["no_equals"])
    with pytest.raises(ValueError, match="must be key=value"):
        _parse_properties(["=val"])
    with pytest.raises(ValueError, match="must be key=value"):
        _parse_properties(["key="])
    with pytest.raises(ValueError, match="duplicate property"):
        _parse_properties(["key=1", "key=2"])

    # _parse_tick
    assert _parse_tick("120", "tick") == 120
    with pytest.raises(ValueError, match="tick must be an integer"):
        _parse_tick("bad", "tick")

    # _safe_round_sample
    assert _safe_round_sample(-1.0, 44100) == 0
    assert _safe_round_sample(float("nan"), 44100) == 0
    assert _safe_round_sample(1000000.0, 44100) == 16_777_216
    assert _safe_round_sample(1.0, 100) == 100

    # _parse_harmonics
    h3 = _parse_harmonics("1:1.0:0.1, 2:0.5:0.2")
    assert len(h3) == 2
    assert h3[0].phase_cycles == 0.1
    with pytest.raises(ValueError, match="at least one partial"):
        _parse_harmonics("")
    with pytest.raises(ValueError, match="exceeds MAX_PARTIALS"):
        _parse_harmonics(",".join(f"{i}:1.0" for i in range(1, MAX_PARTIALS + 2)))
    with pytest.raises(ValueError, match="must be multiplier:amplitude"):
        _parse_harmonics("1:2:3:4")

    # _parse_envelope
    env = _parse_envelope("0.01, 0.02, 0.8, 0.05")
    assert env.attack_seconds == 0.01
    assert env.sustain_level == 0.8
    with pytest.raises(ValueError, match="must be attack,decay,sustain,release"):
        _parse_envelope("0.01, 0.02, 0.8")
    with pytest.raises(
        ValueError, match=f"times must be <= {MAX_NOTE_DURATION_SECONDS}"
    ):
        _parse_envelope(f"{MAX_NOTE_DURATION_SECONDS + 1}, 0.01, 0.5, 0.01")


def test_portable_score_structural_validations() -> None:
    inst1 = InstrumentDeclaration("lead", "sine", gain=0.5)
    trk1 = TrackDeclaration("melody", "lead")
    ev1 = PortableScoreEvent("melody", 0, 120, "note", ("C4",))

    # Duplicate instrument
    with pytest.raises(ValueError, match="unique"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="Dup Inst",
            ppq=120,
            sample_rate_hz=8000,
            tempo_events=(TempoEvent(0, 120),),
            meter_events=(MeterEvent(0, "4/4"),),
            instruments=(inst1, inst1),
            tracks=(trk1,),
            events=(ev1,),
        )

    # Duplicate track
    with pytest.raises(ValueError, match="unique"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="Dup Trk",
            ppq=120,
            sample_rate_hz=8000,
            tempo_events=(TempoEvent(0, 120),),
            meter_events=(MeterEvent(0, "4/4"),),
            instruments=(inst1,),
            tracks=(trk1, trk1),
            events=(ev1,),
        )

    # Unknown instrument on track
    with pytest.raises(ValueError, match="unknown instrument"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="Unk Inst",
            ppq=120,
            sample_rate_hz=8000,
            tempo_events=(TempoEvent(0, 120),),
            meter_events=(MeterEvent(0, "4/4"),),
            instruments=(inst1,),
            tracks=(TrackDeclaration("melody", "unknown_inst"),),
            events=(ev1,),
        )

    # Unknown track on event
    with pytest.raises(ValueError, match="unknown track"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="Unk Trk",
            ppq=120,
            sample_rate_hz=8000,
            tempo_events=(TempoEvent(0, 120),),
            meter_events=(MeterEvent(0, "4/4"),),
            instruments=(inst1,),
            tracks=(trk1,),
            events=(PortableScoreEvent("unknown_track", 0, 120, "note", ("C4",)),),
        )

    # PortableScoreEvent validations
    with pytest.raises(ValueError, match="kind must be"):
        PortableScoreEvent("melody", 0, 120, "invalid_kind")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="at least one note"):
        PortableScoreEvent("melody", 0, 120, "note", notes=())
    with pytest.raises(ValueError, match="rest events cannot include notes"):
        PortableScoreEvent("melody", 0, 120, "rest", notes=("C4",))
    with pytest.raises(ValueError, match="hit events cannot include notes"):
        PortableScoreEvent("melody", 0, 120, "hit", notes=("C4",))
    with pytest.raises(ValueError, match="hit events must include a hit_id"):
        PortableScoreEvent("melody", 0, 120, "hit", hit_id=None)

    # to_text round-trip
    score = PortableScore(
        format_version="music-machine-score/v2",
        title="Roundtrip Song",
        ppq=120,
        sample_rate_hz=8000,
        tempo_events=(TempoEvent(0, 120),),
        meter_events=(MeterEvent(0, "4/4"),),
        instruments=(inst1,),
        tracks=(trk1,),
        events=(ev1,),
    )
    serialized = score.to_text()
    assert "format: music-machine-score/v2" in serialized
    reparsed = parse_portable_score(serialized)
    assert reparsed.title == score.title
    assert len(reparsed.events) == 1


def test_tier0_golden_fixtures() -> None:
    import hashlib
    import json
    import struct
    from pathlib import Path

    fix_dir = (
        Path(__file__).resolve().parents[4] / "specs" / "fixtures" / "music-machine"
    )
    score_txt = (fix_dir / "canonical_score.txt").read_text(encoding="utf-8")
    score_json = (fix_dir / "canonical_score.json").read_text(encoding="utf-8")
    expected_render = json.loads(
        (fix_dir / "expected_render.json").read_text(encoding="utf-8")
    )

    parsed = parse_portable_score(score_txt)
    reconstructed = PortableScore.from_json(score_json)
    assert parsed == reconstructed

    rendered = render_portable_score_to_pcm(parsed)
    samples = rendered.pcm_buffer.samples
    assert len(samples) == expected_render["total_samples"]
    assert (
        rendered.pcm_buffer.clipped_sample_count
        == expected_render["clipped_sample_count"]
    )

    raw_bytes = struct.pack(f"<{len(samples)}h", *samples)
    pcm_sha = hashlib.sha256(raw_bytes).hexdigest()
    assert pcm_sha == expected_render["pcm16_le_sha256"]
    assert list(samples[:32]) == expected_render["first_32_samples"]
    assert list(samples[-32:]) == expected_render["last_32_samples"]


def test_dict_serialization_edge_cases() -> None:
    # InstrumentDeclaration.from_dict error branches
    with pytest.raises(ValueError, match="mapping"):
        InstrumentDeclaration.from_dict("not a map")  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="list of partials"):
        InstrumentDeclaration.from_dict({"id": "inst", "harmonics": "not_list"})

    with pytest.raises(ValueError, match="malformed partial"):
        InstrumentDeclaration.from_dict({"id": "inst", "harmonics": [["bad partial"]]})

    with pytest.raises(ValueError, match=r"\[atk, dec, sus, rel\]"):
        InstrumentDeclaration.from_dict({"id": "inst", "envelope": [0.01, 0.02]})

    # PortableScore.from_dict error branches
    base_dict = parse_portable_score(PORTABLE_DUET_TEXT).to_dict()

    bad_keys = dict(base_dict)
    bad_keys["unexpected_key"] = 123
    with pytest.raises(ValueError, match="unknown score keys"):
        PortableScore.from_dict(bad_keys)

    bad_tempo = dict(base_dict)
    bad_tempo["tempo_events"] = "not_list"
    with pytest.raises(ValueError, match="tempo_events must be a list"):
        PortableScore.from_dict(bad_tempo)

    bad_meter = dict(base_dict)
    bad_meter["meter_events"] = "not_list"
    with pytest.raises(ValueError, match="meter_events must be a list"):
        PortableScore.from_dict(bad_meter)

    bad_inst = dict(base_dict)
    bad_inst["instruments"] = "not_list"
    with pytest.raises(ValueError, match="instruments must be a list"):
        PortableScore.from_dict(bad_inst)

    bad_tracks = dict(base_dict)
    bad_tracks["tracks"] = "not_list"
    with pytest.raises(ValueError, match="tracks must be a list"):
        PortableScore.from_dict(bad_tracks)

    bad_events = dict(base_dict)
    bad_events["events"] = "not_list"
    with pytest.raises(ValueError, match="events must be a list"):
        PortableScore.from_dict(bad_events)


def test_text_score_drumkit_and_validation() -> None:
    hit_ev1 = ScoreEvent(
        kind="hit",
        note=None,
        duration_symbol="q",
        beat_count=1.0,
        duration_seconds=0.5,
        source_token="kick/q",
        hit_id="kick",
    )
    hit_ev2 = ScoreEvent(
        kind="hit",
        note=None,
        duration_symbol="q",
        beat_count=1.0,
        duration_seconds=0.5,
        source_token="snare/q",
        hit_id="snare",
    )
    score = TextScore(
        title="Drums",
        tempo_bpm=120.0,
        meter="4/4",
        amplitude=0.8,
        sample_rate_hz=8000,
        instrument_id="drumkit",
        gm_program=None,
        events=(hit_ev1, hit_ev2),
    )
    rendered = render_score_to_pcm(score)
    assert rendered.pcm_buffer.sample_count() > 0

    # TextScore error branches
    with pytest.raises(ValueError, match=r"\[1, 128\]"):
        TextScore(
            title="Bad GM",
            tempo_bpm=120,
            meter="4/4",
            amplitude=0.5,
            sample_rate_hz=8000,
            instrument_id="piano_naive",
            gm_program=130,
            events=(hit_ev1,),
        )

    # RenderedScore error branch
    with pytest.raises(ValueError, match="rendered_hits\\[0\\]"):
        RenderedScore(
            score=score,
            pcm_buffer=rendered.pcm_buffer,
            rendered_notes=(),
            rendered_hits=("not_a_hit",),  # type: ignore[arg-type]
        )


def test_builder_beats_ticks_measure_helpers() -> None:
    builder = PortableScoreBuilder(ppq=120)
    builder.add_instrument("lead", kind="sine")
    builder.add_instrument("drums", kind="drumkit")
    builder.add_track("melody", instrument_id="lead")
    builder.add_track("rhythm", instrument_id="drums")

    # add_hit and add_rest
    builder.add_hit("rhythm", 0, 120, "kick", velocity=0.9)
    builder.add_rest("melody", 0, 120)

    # beats_to_ticks validations
    with pytest.raises(ValueError, match="beat_count must be >= 0.0"):
        builder.beats_to_ticks(-1.0)
    with pytest.raises(ValueError, match="not representable at ppq"):
        builder.beats_to_ticks(0.333333333)

    # measure_start_tick validations
    with pytest.raises(ValueError, match="integer >= 1"):
        builder.measure_start_tick("one")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="measure_number must be >= 1"):
        builder.measure_start_tick(0)
    assert builder.measure_start_tick(1) == 0
    assert builder.measure_start_tick(2) == 480


def test_motif_and_section_edge_cases() -> None:
    from music_machine.music_machine import _validated_track_map

    # PhraseMotifEvent with hit
    hit_motif_ev = PhraseMotifEvent(0, 120, "hit", notes=(), hit_id="kick")
    assert hit_motif_ev.hit_id == "kick"

    with pytest.raises(ValueError, match="unknown hit_id"):
        PhraseMotifEvent(0, 120, "hit", notes=(), hit_id="unknown_drum_hit")

    with pytest.raises(ValueError, match="motif event extends beyond"):
        PhraseMotif(
            duration_tick=100,
            events=(PhraseMotifEvent(0, 120, "note", ("C4",)),),
        )

    # _validated_track_map with non-mapping
    with pytest.raises(ValueError, match="track_map must be a mapping"):
        _validated_track_map("not a mapping")  # type: ignore[arg-type]


def test_meter_and_instrument_resolution_coverage() -> None:
    from music_machine.music_machine import (
        DEFAULT_INSTRUMENT_ID,
        _is_valid_meter,
        _validate_meter,
        resolve_instrument_id,
    )

    assert not _is_valid_meter("4")
    assert not _is_valid_meter("4/4/4")
    assert not _is_valid_meter("/4")
    assert not _is_valid_meter("4/")
    assert not _is_valid_meter("04/4")
    assert not _is_valid_meter("4/04")
    assert not _is_valid_meter("a/b")
    assert _is_valid_meter("4/4")

    with pytest.raises(ValueError, match="meter must look like"):
        _validate_meter("invalid_meter")

    assert resolve_instrument_id() == DEFAULT_INSTRUMENT_ID
    assert resolve_instrument_id(instrument_id="standard_kit") == "standard_kit"
    assert resolve_instrument_id(instrument_id="standard") == "standard"


def test_score_event_and_text_score_edge_cases() -> None:
    from music_machine.music_machine import (
        MAX_EVENTS,
        MeterEvent,
        TempoEvent,
        TrackDeclaration,
    )

    with pytest.raises(ValueError, match="hit events must include a hit_id"):
        ScoreEvent(
            kind="hit",
            note=None,
            hit_id=None,
            duration_symbol="q",
            beat_count=1.0,
            duration_seconds=0.5,
            source_token="x",
        )

    with pytest.raises(ValueError, match="unknown hit_id"):
        ScoreEvent(
            kind="hit",
            note=None,
            hit_id="unknown_drum_voice",
            duration_symbol="q",
            beat_count=1.0,
            duration_seconds=0.5,
            source_token="x",
        )

    ev = ScoreEvent("note", "C4", "q", 1.0, 0.5, "C4/q")
    with pytest.raises(ValueError, match="exceeds MAX_EVENTS"):
        TextScore(
            title="Too Many",
            tempo_bpm=120.0,
            meter="4/4",
            amplitude=0.5,
            sample_rate_hz=8000,
            instrument_id="piano_naive",
            gm_program=None,
            events=(ev,) * (MAX_EVENTS + 1),
        )

    with pytest.raises(ValueError, match="start_tick"):
        TempoEvent(start_tick=-1, bpm=120.0)

    with pytest.raises(ValueError, match="meter must look like"):
        MeterEvent(start_tick=0, meter="bad_meter")

    with pytest.raises(ValueError, match="track id"):
        TrackDeclaration(id="", instrument_id="inst")


def test_instrument_declaration_additional_edge_cases() -> None:
    from musical_instruments import ADSREnvelope, HarmonicPartial, InstrumentProfile

    with pytest.raises(ValueError, match="unknown drum kit"):
        InstrumentDeclaration(
            id="bad_kit",
            profile_id="drumkit",
            kind="drumkit",
            kit="unknown_kit",
        )

    with pytest.raises(ValueError, match="must provide harmonics"):
        InstrumentDeclaration(
            id="bad_add",
            profile_id="bad",
            kind="additive",
            harmonics=None,
        )

    custom_prof = InstrumentProfile(
        id="custom_prof",
        display_name="Custom Profile",
        synthesis_kind="additive",
        gain=1.0,
        harmonic_profile=(HarmonicPartial(1, 1.0, 0.0),),
        envelope_profile=ADSREnvelope(0.01, 0.02, 0.8, 0.05),
    )
    inst = InstrumentDeclaration(
        id="custom_prof",
        profile_id="custom_prof",
        kind="additive",
        custom_profile=custom_prof,
    )
    assert inst.harmonics == (HarmonicPartial(1, 1.0, 0.0),)
    assert inst.envelope == custom_prof.envelope_profile


def test_portable_score_event_and_builder_edge_cases() -> None:
    from musical_instruments import HarmonicPartial

    with pytest.raises(ValueError, match="hit events cannot include notes"):
        PortableScoreEvent("trk", 0, 120, "hit", notes=("C4",), hit_id="kick")

    with pytest.raises(ValueError, match="unknown hit_id"):
        PortableScoreEvent("trk", 0, 120, "hit", hit_id="unknown_drum")

    with pytest.raises(ValueError, match="start_tick"):
        PortableScoreEvent("trk", -1, 120, "note", notes=("C4",))

    with pytest.raises(ValueError, match="duration_tick"):
        PortableScoreEvent("trk", 0, 0, "note", notes=("C4",))

    with pytest.raises(ValueError, match="malformed JSON"):
        PortableScore.from_json("{broken json")

    with pytest.raises(ValueError, match="JSON score root must be an object"):
        PortableScore.from_json('["not_a_dict"]')

    inst1 = InstrumentDeclaration("lead", profile_id="sine", kind="sine")
    trk1 = TrackDeclaration("melody", "lead")
    ev1 = PortableScoreEvent("melody", 0, 120, "note", ("C4",))

    with pytest.raises(ValueError, match="exceeds MAX_PPQ"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="Empty",
            ppq=200000,
            sample_rate_hz=8000,
            tempo_events=(TempoEvent(0, 120.0),),
            meter_events=(MeterEvent(0, "4/4"),),
            instruments=(inst1,),
            tracks=(trk1,),
            events=(ev1,),
        )

    with pytest.raises(ValueError, match="exceeds MAX_SAMPLE_RATE_HZ"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="Empty",
            ppq=120,
            sample_rate_hz=200000,
            tempo_events=(TempoEvent(0, 120.0),),
            meter_events=(MeterEvent(0, "4/4"),),
            instruments=(inst1,),
            tracks=(trk1,),
            events=(ev1,),
        )

    with pytest.raises(ValueError, match="exceeds MAX_EVENTS"):
        PortableScore(
            format_version="music-machine-score/v2",
            title="Empty",
            ppq=120,
            sample_rate_hz=8000,
            tempo_events=(TempoEvent(0, 120.0),),
            meter_events=(MeterEvent(0, "4/4"),),
            instruments=(inst1,),
            tracks=(trk1,),
            events=(ev1,) * (MAX_EVENTS + 1),
        )

    with pytest.raises(ValueError, match="format_version must be"):
        PortableScoreBuilder(format_version="wrong_version")

    b = PortableScoreBuilder(ppq=120, sample_rate_hz=8000)
    b.add_instrument("lead_preset", profile="piano_naive")
    b.add_instrument(
        "add_inst", kind="additive", harmonics=(HarmonicPartial(1, 1.0, 0.0),)
    )
    with pytest.raises(
        ValueError, match="kind must be sine, silence, drumkit, or additive"
    ):
        b.add_instrument("bad_inst", kind="unknown_kind")

    b.add_track("melody", instrument_id="lead_preset")
    b.add_note("melody", 0, 120, "C4")
    b.add_rest("melody", 120, 120)

    with pytest.raises(ValueError, match="end_tick must be >= start_tick"):
        b.capture_motif("melody", start_tick=100, end_tick=50)

    with pytest.raises(ValueError, match="motif must be a PhraseMotif"):
        b.apply_motif("not_a_motif", "melody", 0)  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="end_tick must be >= start_tick"):
        b.capture_section(start_tick=100, end_tick=50)

    with pytest.raises(ValueError, match="track_ids must be an iterable"):
        b.capture_section(start_tick=0, end_tick=100, track_ids="string_id")  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="section must be an ArrangementSection"):
        b.apply_section("not_a_section", 0)  # type: ignore[arg-type]


def test_phrase_builder_and_section_application() -> None:
    from music_machine.music_machine import (
        ArrangementSection,
        ArrangementSectionEvent,
        PhraseBuilder,
        PhraseMotifEvent,
    )

    with pytest.raises(
        ValueError, match="score_builder must be a PortableScoreBuilder"
    ):
        PhraseBuilder("not_builder", "melody")  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="origin_tick"):
        PhraseBuilder(PortableScoreBuilder(), "melody", start_tick=0, origin_tick=-1)

    builder = PortableScoreBuilder(ppq=120, sample_rate_hz=8000)
    builder.add_instrument("lead", kind="sine")
    builder.add_instrument("drums", kind="drumkit")
    builder.add_track("melody", instrument_id="lead")
    builder.add_track("rhythm", instrument_id="drums")

    pb = PhraseBuilder(builder, "melody")
    pb.chord(["C4", "E4", "G4"], 1.0, velocity=0.8)
    pb.rest(1.0)
    pb.advance_beats(1.0)
    pb.jump_to_measure(2, beat_offset=0.5)

    pb_drum = PhraseBuilder(builder, "rhythm")
    pb_drum.hit("kick", 1.0, velocity=0.9)
    pb_drum.hit("snare", 1.0, velocity=0.8)

    motif = pb.motif(start_tick=0)
    pb.apply_motif(
        motif,
        transpose_semitones=2,
        velocity_scale=0.8,
        repeat_count=2,
        spacing_beats=2.0,
    )

    # Section with note, hit, and rest applied with track_map,
    # dictionary transpose, and dictionary velocity scale
    section_events = (
        ArrangementSectionEvent("melody", 0, 120, "note", ("C4",), velocity=0.9),
        ArrangementSectionEvent(
            "rhythm", 0, 120, "hit", notes=(), hit_id="kick", velocity=0.8
        ),
        ArrangementSectionEvent("melody", 120, 120, "rest", notes=()),
    )
    section = ArrangementSection(duration_tick=240, events=section_events)
    builder.apply_section(
        section,
        start_tick=480,
        track_map={"melody": "melody", "rhythm": "rhythm"},
        transpose_semitones={"melody": 4, "rhythm": 0},
        velocity_scale={"melody": 0.7, "rhythm": 0.9},
    )

    # ArrangementSectionEvent validations
    with pytest.raises(ValueError, match="kind must be"):
        ArrangementSectionEvent("melody", 0, 120, "invalid")  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="at least one note"):
        ArrangementSectionEvent("melody", 0, 120, "note", notes=())

    with pytest.raises(ValueError, match="rest section events cannot include notes"):
        ArrangementSectionEvent("melody", 0, 120, "rest", notes=("C4",))

    with pytest.raises(ValueError, match="hit section events cannot include notes"):
        ArrangementSectionEvent("melody", 0, 120, "hit", notes=("C4",))

    with pytest.raises(ValueError, match="hit section events must include a hit_id"):
        ArrangementSectionEvent("melody", 0, 120, "hit", hit_id=None)

    with pytest.raises(ValueError, match="unknown hit_id"):
        ArrangementSectionEvent("melody", 0, 120, "hit", hit_id="unknown_drum")

    # PhraseMotifEvent validations
    with pytest.raises(ValueError, match="kind must be"):
        PhraseMotifEvent(0, 120, "invalid")  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="at least one note"):
        PhraseMotifEvent(0, 120, "note", notes=())

    with pytest.raises(ValueError, match="rest motif events cannot include notes"):
        PhraseMotifEvent(0, 120, "rest", notes=("C4",))

    with pytest.raises(ValueError, match="hit motif events cannot include notes"):
        PhraseMotifEvent(0, 120, "hit", notes=("C4",))

    with pytest.raises(ValueError, match="hit motif events must include a hit_id"):
        PhraseMotifEvent(0, 120, "hit", hit_id=None)


def test_render_pcm_additional_edge_cases() -> None:
    from music_machine.music_machine import MAX_SAMPLE_COUNT

    # Mixed note and hit render in TextScore
    score = TextScore(
        title="Mixed",
        tempo_bpm=120.0,
        meter="4/4",
        amplitude=0.5,
        sample_rate_hz=8000,
        instrument_id="piano_naive",
        gm_program=None,
        events=(
            ScoreEvent("note", "C4", "q", 1.0, 0.1, "C4/q"),
            ScoreEvent("hit", None, "q", 1.0, 0.1, "kick/q", hit_id="kick"),
            ScoreEvent("rest", None, "q", 1.0, 0.1, "r/q"),
        ),
    )
    rendered = render_score_to_pcm(score)
    assert len(rendered.pcm_buffer.samples) > 0

    # render_portable_score_to_pcm validations
    with pytest.raises(ValueError, match="score must be a PortableScore"):
        render_portable_score_to_pcm("not_a_portable_score")  # type: ignore[arg-type]

    b = PortableScoreBuilder(ppq=120, sample_rate_hz=8000)
    b.add_instrument("lead", kind="sine")
    b.add_track("melody", instrument_id="lead")
    b.add_note("melody", 0, 120, "C4")
    pscore = b.build()

    with pytest.raises(ValueError, match="exceeds MAX_SAMPLE_COUNT"):
        render_portable_score_to_pcm(pscore, max_sample_count=MAX_SAMPLE_COUNT + 1)

    # Note on drumkit track
    b_drum = PortableScoreBuilder(ppq=120, sample_rate_hz=8000)
    b_drum.add_instrument("drums", kind="drumkit")
    b_drum.add_track("rhythm", instrument_id="drums")
    b_drum.add_note("rhythm", 0, 120, "C4")
    score_bad_drum = b_drum.build()
    with pytest.raises(ValueError, match="cannot play note events"):
        render_portable_score_to_pcm(score_bad_drum)

    # Hit on pitched track
    b_pitched = PortableScoreBuilder(ppq=120, sample_rate_hz=8000)
    b_pitched.add_instrument("lead", kind="sine")
    b_pitched.add_track("melody", instrument_id="lead")
    b_pitched.add_hit("melody", 0, 120, "kick")
    score_bad_pitched = b_pitched.build()
    with pytest.raises(ValueError, match="cannot play hit events"):
        render_portable_score_to_pcm(score_bad_pitched)

    # Score exceeding MAX_DURATION_SECONDS
    with pytest.raises(ValueError, match="exceeds MAX_DURATION_SECONDS"):
        render_portable_score_to_pcm(
            PortableScore(
                format_version="music-machine-score/v2",
                title="Long",
                ppq=120,
                sample_rate_hz=8000,
                tempo_events=(TempoEvent(0, 120.0),),
                meter_events=(MeterEvent(0, "4/4"),),
                instruments=(
                    InstrumentDeclaration("lead", profile_id="sine", kind="sine"),
                ),
                tracks=(TrackDeclaration("melody", "lead"),),
                events=(PortableScoreEvent("melody", 0, 100_000_000, "rest"),),
            )
        )


def test_security_review_fixes(tmp_path: Path) -> None:
    from unittest.mock import patch

    from musical_instruments import MAX_NOTE_DURATION_SECONDS, MAX_PARTIALS

    # 1. CWE-22: Symlink rejection
    target_file = tmp_path / "valid.txt"
    target_file.write_text("C4 1/4\n", encoding="utf-8")
    symlink_file = tmp_path / "symlink.txt"
    try:
        symlink_file.symlink_to(target_file)
        has_symlink = True
    except (OSError, NotImplementedError):
        has_symlink = False

    if has_symlink:
        with pytest.raises(ValueError, match="symlinks are not permitted"):
            parse_score_file(symlink_file)
        with pytest.raises(ValueError, match="symlinks are not permitted"):
            parse_portable_score_file(symlink_file)

    # 2. CWE-400: Bounded file read before read_text
    oversized_file = tmp_path / "large.txt"
    oversized_file.write_text("x" * 200, encoding="utf-8")
    with pytest.raises(ValueError, match="exceeds limit"):
        parse_score_file(oversized_file, max_score_length=50)
    with pytest.raises(ValueError, match="exceeds limit"):
        parse_portable_score_file(oversized_file, max_score_length=50)

    # 3. CWE-400 / CWE-20: Missing keys & non-string types in submodel from_dict
    with pytest.raises(ValueError, match="missing required keys"):
        TempoEvent.from_dict({"start_tick": 0})
    with pytest.raises(ValueError, match="missing required keys"):
        MeterEvent.from_dict({"start_tick": 0})
    with pytest.raises(ValueError, match="must be a string"):
        MeterEvent.from_dict({"start_tick": 0, "meter": 123})
    with pytest.raises(ValueError, match="missing 'id'"):
        InstrumentDeclaration.from_dict({})
    with pytest.raises(ValueError, match="must be a string"):
        InstrumentDeclaration.from_dict({"id": 123})
    with pytest.raises(ValueError, match="must be a string"):
        InstrumentDeclaration.from_dict({"id": "inst", "kind": 123})
    with pytest.raises(ValueError, match="must be a string"):
        InstrumentDeclaration.from_dict({"id": "inst", "kit": 123})
    with pytest.raises(ValueError, match="must be a string"):
        InstrumentDeclaration.from_dict({"id": "inst", "profile_id": 123})
    with pytest.raises(ValueError, match="exceeds MAX_PARTIALS"):
        InstrumentDeclaration.from_dict(
            {"id": "inst", "harmonics": [[1, 1.0]] * (MAX_PARTIALS + 1)}
        )
    with pytest.raises(ValueError, match="envelope times must be"):
        InstrumentDeclaration.from_dict(
            {
                "id": "inst",
                "envelope": [
                    MAX_NOTE_DURATION_SECONDS + 1.0,
                    0.01,
                    0.8,
                    0.01,
                ],
            }
        )
    with pytest.raises(ValueError, match="missing required keys"):
        TrackDeclaration.from_dict({"id": "trk"})
    with pytest.raises(ValueError, match="must be a string"):
        TrackDeclaration.from_dict({"id": 123, "instrument_id": "inst"})
    with pytest.raises(ValueError, match="must be a string"):
        TrackDeclaration.from_dict({"id": "trk", "instrument_id": 123})
    with pytest.raises(ValueError, match="missing required key"):
        PortableScoreEvent.from_dict({"track_id": "trk"})
    with pytest.raises(ValueError, match="must be a string"):
        PortableScoreEvent.from_dict(
            {"track_id": 123, "start_tick": 0, "duration_tick": 120, "kind": "note"}
        )
    with pytest.raises(ValueError, match="must be a string"):
        PortableScoreEvent.from_dict(
            {"track_id": "trk", "start_tick": 0, "duration_tick": 120, "kind": 123}
        )
    with pytest.raises(ValueError, match="must be a list or tuple"):
        PortableScoreEvent.from_dict(
            {
                "track_id": "trk",
                "start_tick": 0,
                "duration_tick": 120,
                "kind": "note",
                "notes": "not_a_list",
            }
        )
    with pytest.raises(ValueError, match="must be a string"):
        PortableScoreEvent.from_dict(
            {
                "track_id": "trk",
                "start_tick": 0,
                "duration_tick": 120,
                "kind": "hit",
                "hit_id": 123,
            }
        )

    # PortableScore.from_dict allocation pre-checks
    valid_dict = parse_portable_score(PORTABLE_DUET_TEXT).to_dict()
    bad_events_dict = dict(valid_dict)
    bad_events_dict["events"] = [valid_dict["events"][0]] * (MAX_EVENTS + 1)
    with pytest.raises(ValueError, match="events count .* exceeds MAX_EVENTS"):
        PortableScore.from_dict(bad_events_dict)

    bad_tempos_dict = dict(valid_dict)
    bad_tempos_dict["tempo_events"] = [
        {"start_tick": i, "bpm": 120.0} for i in range(MAX_EVENTS + 1)
    ]
    with pytest.raises(ValueError, match="tempo_events count .* exceeds MAX_EVENTS"):
        PortableScore.from_dict(bad_tempos_dict)

    # RecursionError catch in from_json
    with (
        patch("json.loads", side_effect=RecursionError("deep")),
        pytest.raises(ValueError, match="malformed JSON"),
    ):
        PortableScore.from_json('{"title": "x"}')

    # 4. CWE-407 / CWE-400: Sample rate bounds and drumkit budget validation
    with pytest.raises(ValueError, match="exceeds MAX_SAMPLE_RATE_HZ"):
        TextScore(
            title="Fast",
            tempo_bpm=120.0,
            meter="4/4",
            amplitude=0.5,
            sample_rate_hz=200_000,
            instrument_id="piano_naive",
            gm_program=None,
            events=(ScoreEvent("note", "C4", "q", 1.0, 0.5, "C4/q"),),
        )

    with pytest.raises(ValueError, match="exceeds MAX_SAMPLE_RATE_HZ"):
        PortableScoreBuilder(sample_rate_hz=200_000)

    # Drumkit budget pre-flight check in render_score_to_pcm
    drum_score = TextScore(
        title="Drums",
        tempo_bpm=120.0,
        meter="4/4",
        amplitude=0.8,
        sample_rate_hz=8000,
        instrument_id="drumkit",
        gm_program=None,
        events=(ScoreEvent("hit", None, "q", 1.0, 1.0, "kick/q", hit_id="kick"),),
    )
    with pytest.raises(ValueError, match="above max_sample_count"):
        render_score_to_pcm(drum_score, max_sample_count=10)
