"""Text score parser and music machine for simple note melodies."""

from .music_machine import (
    DEFAULT_AMPLITUDE,
    DEFAULT_MAX_EVENT_COUNT,
    DEFAULT_MAX_LINE_LENGTH,
    DEFAULT_MAX_SAMPLE_COUNT,
    DEFAULT_MAX_SCORE_LENGTH,
    DEFAULT_METER,
    DEFAULT_SAMPLE_RATE_HZ,
    DEFAULT_TEMPO_BPM,
    DURATION_BEATS,
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

__version__ = "0.1.0"

__all__ = [
    "DEFAULT_AMPLITUDE",
    "DEFAULT_MAX_EVENT_COUNT",
    "DEFAULT_MAX_LINE_LENGTH",
    "DEFAULT_MAX_SAMPLE_COUNT",
    "DEFAULT_MAX_SCORE_LENGTH",
    "DEFAULT_METER",
    "DEFAULT_SAMPLE_RATE_HZ",
    "DEFAULT_TEMPO_BPM",
    "DURATION_BEATS",
    "HAPPY_BIRTHDAY_TEXT",
    "RenderedScore",
    "ScoreEvent",
    "TextScore",
    "__version__",
    "beats_for_duration_symbol",
    "parse_score",
    "play_score",
    "play_score_text",
    "render_score_to_pcm",
]
