#!/usr/bin/env python3
"""transcribe.py - voice -> transcript, on-device. The front of the ER spine.

MYCIN-2026 C3. Turns spoken ER input into text using `mlx-whisper` (local, on the
doctor's Apple-Silicon machine - the audio never leaves it). The rest of the spine
(decompose -> diagnose -> triage) is the same warm path. To stay runnable without
audio hardware / the optional dependency, `transcribe()` is a graceful
abstraction: an audio file is transcribed if mlx-whisper is available; a string
that is already text is passed straight through (the typed-transcript path the
tests and CI use).

Config: MYCIN_WHISPER_MODEL (default: mlx-community/whisper-small-mlx).
"""

from __future__ import annotations

import os
from pathlib import Path

AUDIO_SUFFIXES = {".wav", ".mp3", ".m4a", ".flac", ".ogg", ".aac"}
DEFAULT_WHISPER = "mlx-community/whisper-small-mlx"


def is_audio_path(source: str) -> bool:
    """Does `source` name an audio file that exists on disk?"""
    try:
        p = Path(source)
    except (TypeError, ValueError):
        return False
    return p.suffix.lower() in AUDIO_SUFFIXES and p.is_file()


def whisper_available() -> bool:
    import importlib.util
    return importlib.util.find_spec("mlx_whisper") is not None


def transcribe(source: str) -> str:
    """Return text for `source`: transcribe it if it is an audio file (requires
    mlx-whisper), else treat it as an already-typed transcript and return it
    verbatim. Raises only when an audio file is given but mlx-whisper is missing -
    never silently drops the audio."""
    if is_audio_path(source):
        if not whisper_available():
            raise RuntimeError(
                f"{source} is audio but mlx-whisper is not installed. "
                "`pip install mlx-whisper`, or pass an already-typed transcript.")
        import mlx_whisper
        model = os.environ.get("MYCIN_WHISPER_MODEL", DEFAULT_WHISPER)
        return mlx_whisper.transcribe(source, path_or_hf_repo=model)["text"].strip()
    # Already text (the typed-transcript path).
    return source.strip()
