"""Naive musical instrument profiles and additive note rendering."""

from .musical_instruments import (
    DEFAULT_MAX_PARTIAL_COUNT,
    DEFAULT_MAX_SAMPLE_COUNT,
    DEFAULT_SAMPLE_RATE_HZ,
    SYNTHESIS_KINDS,
    ADSREnvelope,
    GMProgram,
    HarmonicPartial,
    InstrumentNoteRender,
    InstrumentProfile,
    InstrumentSignal,
    VariationProfile,
    all_gm_programs,
    all_instruments,
    get_gm_program,
    get_instrument,
    instrument_for_gm_program,
    render_instrument_note,
)

__version__ = "0.1.0"

__all__ = [
    "ADSREnvelope",
    "DEFAULT_MAX_PARTIAL_COUNT",
    "DEFAULT_MAX_SAMPLE_COUNT",
    "DEFAULT_SAMPLE_RATE_HZ",
    "GMProgram",
    "HarmonicPartial",
    "InstrumentNoteRender",
    "InstrumentProfile",
    "InstrumentSignal",
    "SYNTHESIS_KINDS",
    "VariationProfile",
    "__version__",
    "all_gm_programs",
    "all_instruments",
    "get_gm_program",
    "get_instrument",
    "instrument_for_gm_program",
    "render_instrument_note",
]
