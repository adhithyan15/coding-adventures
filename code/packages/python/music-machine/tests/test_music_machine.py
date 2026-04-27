from __future__ import annotations

import sys
from types import SimpleNamespace

import pytest
from pcm_audio import PCMBuffer, PCMFormat

from music_machine import (
    DEFAULT_INSTRUMENT_ID,
    HAPPY_BIRTHDAY_TEXT,
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
    parse_score,
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
            "format: music-machine-score/v2\n"
            "instrument lead program=1 profile=sine",
            "only one",
        ),
        (
            "format: music-machine-score/v2\n"
            "instrument lead profile=sine kind=sine",
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
