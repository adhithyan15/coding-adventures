"""The private bytecode cache (VER04i 8.1).

Why this exists
---------------
CPython runs a cached ``__pycache__/*.pyc`` *instead of* its source.  A
hash-based pyc (PEP 552, flags nonzero) is never revalidated, and a timestamp
pyc can be forged to carry the source's mtime and size, so a hostile checkout
can make the lane run code the source tree does not contain.  No inspection of
the header is sound.  What is sound is never consulting the tree's caches at
all: with ``sys.pycache_prefix`` set, CPython reads and writes bytecode *only*
under that directory.

What this module checks
-----------------------
Before a run produces evidence, the interpreter must have been started with a
prefix that is a fresh directory this runner's entry script created
(``mkdtemp``: private, owned by the current user, under the temporary
directory) and named in ``WARRANT_PYCACHE_PRIVATE``.  The entry script
(``tests/warrant_evidence.py``) creates it and re-runs itself before importing
anything from the checkout; it is empty by construction because only that child
ever receives its name.  The manifest then records ``"pycache_isolated": true``,
and the Rust loader ignores ``__pycache__`` in the source root only for a
manifest that says so.
"""

from __future__ import annotations

import os
import stat
import sys
import tempfile

from .errors import CorpusError

ENV_PRIVATE = "WARRANT_PYCACHE_PRIVATE"
CODE = "pycache-isolation"


def _refuse(message: str) -> CorpusError:
    return CorpusError(CODE, f"the bytecode cache is not isolated: {message}")


def verify_isolated_cache() -> None:
    """Raise a named :class:`CorpusError` unless the run uses a private cache."""
    prefix = sys.pycache_prefix
    if prefix is None:
        raise _refuse("sys.pycache_prefix is not set (run through the entry script)")
    named = os.environ.get(ENV_PRIVATE)
    if named is None or os.path.realpath(named) != os.path.realpath(prefix):
        raise _refuse(
            f"it is not the directory the entry script created ({ENV_PRIVATE})"
        )
    try:
        info = os.lstat(prefix)
    except OSError:
        raise _refuse("the directory does not exist") from None
    if stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
        raise _refuse("it is not a plain directory")
    temp = os.path.realpath(tempfile.gettempdir())
    real = os.path.realpath(prefix)
    if os.path.commonpath([temp, real]) != temp or real == temp:
        raise _refuse("it is not inside the private temporary directory")
    if os.name != "nt":
        if info.st_uid != os.geteuid():
            raise _refuse("it is not owned by the current user")
        if stat.S_IMODE(info.st_mode) & 0o077:
            raise _refuse("it is readable or writable by others")
