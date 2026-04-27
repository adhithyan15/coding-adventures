"""Entry point for ``python -m ircd``.

Importing ``ircd`` and calling ``main()`` is the canonical way to start the
server.  Placing the call here (rather than in ``__init__.py``) means the
server does *not* start automatically when another module imports ``ircd``.
"""

from ircd import main

main()
