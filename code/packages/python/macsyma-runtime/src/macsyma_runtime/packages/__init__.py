"""Loadable MACSYMA runtime packages.

This sub-package holds the *optional* handler tables that a MACSYMA
session can install on demand via the ``load("name")`` runtime
directive (Track M1).

Why loadable?
=============

Most CAS substrates (factor, solve, integrate, …) are always available
because users expect them on day one.  Specialised tables — orthogonal
polynomial evaluators, package-specific number-theory bundles — are
heavier and rarely needed at startup.  Maxima ships them as separate
``load("foo")`` packages for the same reason: pay for what you use.

Today only ``orthopoly`` lives here.  Adding a new package is a four-step
recipe:

1. Drop a new module ``packages/<name>.py`` that exposes
   ``register_handlers(backend: MacsymaBackend) -> None``.
2. Add ``"<name>"`` to the allowlist in
   :func:`macsyma_runtime.handlers.make_load_handler`.
3. Wire the dispatch arm that imports ``register_handlers`` from the new
   module and calls it.
4. Add a regression test in ``tests/test_load_package.py``.

The allowlist matters.  The ``load`` handler refuses any name that isn't
in the hardcoded set, so user input never reaches ``importlib`` or the
filesystem.  See the security review in the M1 commit message.
"""

from __future__ import annotations
