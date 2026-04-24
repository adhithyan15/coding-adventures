"""
handler_context — execution context for Conduit route handlers.

Every route handler, before/after filter, not_found handler, and error handler
receives a ``HandlerContext`` as its first (and only) argument:

    @app.get("/hello/<name>")
    def hello(ctx):
        ctx.json({"message": f"Hello {ctx.params['name']}"})

``HandlerContext`` bundles two concerns:
1. **Response helpers** — ``json``, ``html``, ``text``, ``halt``, ``redirect``.
   Each raises ``HaltException`` to short-circuit the handler immediately.
   This mirrors Ruby's ``HandlerContext`` and Sinatra's DSL helpers.
2. **Request delegation** — ``__getattr__`` proxies unknown attributes to the
   wrapped ``Request`` object, so ``ctx.path`` is the same as
   ``ctx.request.path`` and ``ctx.params`` is the same as
   ``ctx.request.params``.

The delegation model is intentional: users rarely need to write
``ctx.request.params`` — just ``ctx.params`` is cleaner. But when the full
``Request`` object is needed (e.g. to parse the body or read all headers),
``ctx.request`` provides direct access.

This class is analogous to Ruby's ``CodingAdventures::Conduit::HandlerContext``.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

from .halt_exception import HaltException

if TYPE_CHECKING:
    from .request import Request


class HandlerContext:
    """Execution context passed to every Conduit handler function.

    Provides response helpers and delegates request attribute access to the
    wrapped ``Request``. Response helpers all raise ``HaltException`` to
    short-circuit the handler pipeline — they do not return a value.

    Attributes:
        request: The underlying ``Request`` object with full HTTP request data.
    """

    def __init__(self, request: Request) -> None:
        self.request = request

    # ── Response helpers ────────────────────────────────────────────────────

    def json(self, data: object, status: int = 200) -> None:
        """Respond with a JSON body.

        Serializes ``data`` with ``json.dumps``, sets
        ``Content-Type: application/json; charset=utf-8``, and raises
        ``HaltException`` to exit the handler immediately.

        Example::

            @app.get("/user/<id>")
            def get_user(ctx):
                ctx.json({"id": ctx.params["id"], "name": "Alice"})
        """
        raise HaltException(
            status,
            json.dumps(data),
            {"content-type": "application/json; charset=utf-8"},
        )

    def html(self, content: str, status: int = 200) -> None:
        """Respond with an HTML body.

        Sets ``Content-Type: text/html; charset=utf-8`` and raises
        ``HaltException``.

        Example::

            @app.get("/")
            def index(ctx):
                ctx.html("<h1>Welcome!</h1>")
        """
        raise HaltException(
            status,
            str(content),
            {"content-type": "text/html; charset=utf-8"},
        )

    def text(self, content: str, status: int = 200) -> None:
        """Respond with a plain-text body.

        Sets ``Content-Type: text/plain; charset=utf-8`` and raises
        ``HaltException``.
        """
        raise HaltException(
            status,
            str(content),
            {"content-type": "text/plain; charset=utf-8"},
        )

    def halt(
        self,
        status: int,
        body: str = "",
        headers: dict[str, str] | None = None,
    ) -> None:
        """Short-circuit immediately with an explicit status, body, and headers.

        Use this when you need full control over the response without the
        convenience wrappers of ``json`` or ``html``.

        Example::

            @app.before_request
            def auth(ctx):
                if not ctx.header("authorization"):
                    ctx.halt(401, "Unauthorized")
        """
        raise HaltException(status, body, headers or {})

    def redirect(self, url: str, status: int = 302) -> None:
        """Redirect the client to ``url``.

        Default status is 302 Found. Use 301 for a permanent redirect.

        SECURITY: This method does NOT validate ``url``. If ``url`` is derived
        from user-supplied input (e.g. a ``return_to`` query parameter),
        validate that it is a trusted relative path or known origin before
        calling this method — otherwise an attacker can craft a link that
        sends users to a phishing site (open redirect / CWE-601).

        Example::

            @app.get("/old-path")
            def old_path(ctx):
                ctx.redirect("/new-path", 301)
        """
        raise HaltException(status, "", {"location": str(url)})

    # ── Request delegation ──────────────────────────────────────────────────

    def __getattr__(self, name: str) -> object:
        """Delegate unknown attributes to ``self.request``.

        This is what makes ``ctx.path`` and ``ctx.params`` work without
        writing ``ctx.request.path`` every time. Python calls ``__getattr__``
        only when the normal attribute lookup fails, so ``ctx.request`` and
        the response helpers (which are real methods) take precedence.
        """
        return getattr(self.request, name)
