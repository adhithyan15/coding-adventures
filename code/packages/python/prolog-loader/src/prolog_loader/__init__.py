"""Loader helpers for parsed Prolog sources and explicit initialization runs."""

from prolog_loader.adapters import adapt_prolog_goal
from prolog_loader.loader import (
    GoalAdapter,
    LoadedPrologProject,
    LoadedPrologSource,
    PrologInitializationError,
    __version__,
    link_loaded_prolog_sources,
    load_iso_prolog_source,
    load_parsed_prolog_source,
    load_swi_prolog_project,
    load_swi_prolog_source,
    run_initialization_goals,
    run_prolog_initialization_goals,
)

__all__ = [
    "__version__",
    "adapt_prolog_goal",
    "GoalAdapter",
    "LoadedPrologProject",
    "LoadedPrologSource",
    "PrologInitializationError",
    "load_iso_prolog_source",
    "load_parsed_prolog_source",
    "load_swi_prolog_project",
    "load_swi_prolog_source",
    "link_loaded_prolog_sources",
    "run_initialization_goals",
    "run_prolog_initialization_goals",
]
