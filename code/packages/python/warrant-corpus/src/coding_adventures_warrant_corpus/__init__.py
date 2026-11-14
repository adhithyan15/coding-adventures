"""Python lane runner for the language-neutral Warrant evidence corpus.

The Rust compiler (``warrant-grammar``) turns a package's ``.warrant`` file into
a ``warrant-corpus-v1`` directory.  This package is the *Python lane* of that
design (VER04i): it loads the corpus, re-enumerates every exhaustive domain in
Python, verifies the input and expected-value digests against the corpus,
drives a Python library through its ``evidence.bindings`` table, runs the
negative controls that prove the runner can fail, and writes the lane's
``warrant-lane-evidence-v1`` manifest.

Reading order for newcomers::

    values -> tfv -> domains -> ir -> corpus -> faults -> binding
           -> runner -> manifest -> session -> cli

It is standard-library only and has no dependency on any library it tests.
"""

from __future__ import annotations

from .binding import Binding, Subject, parse_binding
from .cli import main
from .corpus import Corpus, load_corpus
from .errors import BindingError, CorpusError, LaneError, UsageError
from .manifest import (
    build_manifest,
    frame_sources,
    source_digest,
    validate_manifest,
)
from .session import RunConfig, execute, prepare

__version__ = "0.1.1"

__all__ = [
    "Binding",
    "BindingError",
    "Corpus",
    "CorpusError",
    "LaneError",
    "RunConfig",
    "Subject",
    "UsageError",
    "build_manifest",
    "execute",
    "frame_sources",
    "load_corpus",
    "main",
    "parse_binding",
    "prepare",
    "source_digest",
    "validate_manifest",
]
