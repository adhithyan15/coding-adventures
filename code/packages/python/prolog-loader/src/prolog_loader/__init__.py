"""Loader helpers for parsed Prolog sources and explicit initialization runs."""

from prolog_loader.adapters import adapt_prolog_goal
from prolog_loader.loader import (
    GoalAdapter,
    LoadedPrologSource,
    PrologInitializationError,
    __version__,
    load_iso_prolog_source,
    load_parsed_prolog_source,
    load_swi_prolog_source,
    run_initialization_goals,
    run_prolog_initialization_goals,
)

__all__ = [
    "__version__",
    "adapt_prolog_goal",
    "GoalAdapter",
    "LoadedPrologSource",
    "PrologInitializationError",
    "load_iso_prolog_source",
    "load_parsed_prolog_source",
    "load_swi_prolog_source",
    "run_initialization_goals",
    "run_prolog_initialization_goals",
]
