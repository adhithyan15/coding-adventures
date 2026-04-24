"""Loader helpers for parsed Prolog sources and explicit initialization runs."""

from prolog_loader.loader import (
    GoalAdapter,
    LoadedPrologSource,
    PrologInitializationError,
    __version__,
    load_iso_prolog_source,
    load_parsed_prolog_source,
    load_swi_prolog_source,
    run_initialization_goals,
)

__all__ = [
    "__version__",
    "GoalAdapter",
    "LoadedPrologSource",
    "PrologInitializationError",
    "load_iso_prolog_source",
    "load_parsed_prolog_source",
    "load_swi_prolog_source",
    "run_initialization_goals",
]
