"""
application — Conduit Flask-like application DSL.

``Conduit`` is the main entry point for building a Conduit web application.
It provides a decorator-based DSL modelled after Flask:

    app = Conduit()

    @app.before_request
    def auth(ctx):
        if not ctx.header("authorization"):
            ctx.halt(401, "Unauthorized")

    @app.get("/")
    def index(ctx):
        ctx.html("<h1>Hello!</h1>")

    @app.get("/hello/<name>")
    def greet(ctx):
        ctx.json({"message": f"Hello {ctx.params['name']}"})

    @app.post("/echo")
    def echo(ctx):
        ctx.json(ctx.request.json())

    @app.not_found
    def missing(ctx):
        ctx.html(f"<h1>Not Found: {ctx.path}</h1>", 404)

    @app.error_handler
    def on_error(ctx, err):
        ctx.json({"error": "Internal Server Error"}, 500)

    app.settings["app_name"] = "My App"

    if __name__ == "__main__":
        server = app.serve(host="127.0.0.1", port=3000)

## Route patterns

Routes use ``<param>`` syntax (Flask-style) which Conduit converts to the
``:param`` syntax expected by the Rust ``web-core`` router:

    ``/hello/<name>``  →  ``/hello/:name``
    ``/users/<id>``    →  ``/users/:id``

The converted pattern is what gets stored in ``Route.pattern`` and sent to
the Rust router at server-init time.

## Filter and handler registration

All decorator methods return the original function unchanged, so decorated
functions can still be called normally from tests.

This class is analogous to Ruby's ``CodingAdventures::Conduit::Application``.
"""

from __future__ import annotations

import re
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

# ── Route ───────────────────────────────────────────────────────────────────


@dataclass
class Route:
    """A registered HTTP route.

    ``method``  — upper-case HTTP verb: ``"GET"``, ``"POST"``, …
    ``pattern`` — Rust-style route pattern: ``"/hello/:name"``
    ``handler`` — Python callable that receives a ``HandlerContext``
    """

    method: str
    pattern: str
    handler: Callable


# ── Conduit application ──────────────────────────────────────────────────────


class Conduit:
    """Flask-like Conduit web application.

    Holds routes, filters, and handlers. Call ``app.serve(host, port)`` to
    start the server, or wrap in a ``NativeServer`` for lower-level control.

    Attributes:
        routes: List of ``Route`` objects in registration order.
        before_filters: List of callables run before every request.
        after_filters: List of callables run after every matched route.
        not_found_handler: Optional callable for unmatched paths.
        error_handler_fn: Optional callable for unhandled exceptions.
        settings: Free-form dict for application settings (e.g. app name).
    """

    def __init__(self) -> None:
        self.routes: list[Route] = []
        self.before_filters: list[Callable] = []
        self.after_filters: list[Callable] = []
        self.not_found_handler: Callable | None = None
        self.error_handler_fn: Callable | None = None
        self.settings: dict[str, Any] = {}

    # ── Route decorators ────────────────────────────────────────────────────

    def _add_route(self, method: str, pattern: str) -> Callable:
        """Return a decorator that registers ``fn`` as a route handler."""
        rust_pattern = _flask_to_rust_pattern(pattern)

        def decorator(fn: Callable) -> Callable:
            self.routes.append(
                Route(method=method.upper(), pattern=rust_pattern, handler=fn)
            )
            return fn

        return decorator

    def get(self, pattern: str) -> Callable:
        """Register a handler for ``GET pattern``."""
        return self._add_route("GET", pattern)

    def post(self, pattern: str) -> Callable:
        """Register a handler for ``POST pattern``."""
        return self._add_route("POST", pattern)

    def put(self, pattern: str) -> Callable:
        """Register a handler for ``PUT pattern``."""
        return self._add_route("PUT", pattern)

    def patch(self, pattern: str) -> Callable:
        """Register a handler for ``PATCH pattern``."""
        return self._add_route("PATCH", pattern)

    def delete(self, pattern: str) -> Callable:
        """Register a handler for ``DELETE pattern``."""
        return self._add_route("DELETE", pattern)

    def head(self, pattern: str) -> Callable:
        """Register a handler for ``HEAD pattern``."""
        return self._add_route("HEAD", pattern)

    def options(self, pattern: str) -> Callable:
        """Register a handler for ``OPTIONS pattern``."""
        return self._add_route("OPTIONS", pattern)

    # ── Filter decorators ───────────────────────────────────────────────────

    def before_request(self, fn: Callable) -> Callable:
        """Register a before-request filter.

        Runs before every request, *including* unmatched paths (before route
        lookup). This matches Sinatra semantics — useful for maintenance mode,
        authentication, or rate limiting where you want the filter to fire
        even when no route exists.

        Raising ``HaltException`` (or calling a response helper) in a before
        filter short-circuits the entire request pipeline. The after filter
        does NOT run if a before filter halts.

        Example::

            @app.before_request
            def maintenance(ctx):
                if ctx.path == "/down":
                    ctx.halt(503, "Under maintenance")
        """
        self.before_filters.append(fn)
        return fn

    def after_request(self, fn: Callable) -> Callable:
        """Register an after-request filter.

        Runs after every *matched* route handler. Used for side effects like
        logging. The filter receives the same ``HandlerContext``; any response
        helpers called here are swallowed (the original response is returned).

        Example::

            @app.after_request
            def logger(ctx):
                print(f"[after] {ctx.method} {ctx.path}")
        """
        self.after_filters.append(fn)
        return fn

    def not_found(self, fn: Callable) -> Callable:
        """Register a custom not-found handler.

        Called when no route matches the request. If omitted, Rust returns
        a plain-text 404 response.

        Example::

            @app.not_found
            def missing(ctx):
                ctx.html(f"<h1>Not Found: {ctx.path}</h1>", 404)
        """
        self.not_found_handler = fn
        return fn

    def error_handler(self, fn: Callable) -> Callable:
        """Register a custom error handler.

        Called when a route handler raises an unhandled exception. Receives
        ``(ctx, error_message: str)`` where ``error_message`` is the str()
        of the exception.

        Example::

            @app.error_handler
            def on_error(ctx, err):
                ctx.json({"error": "Internal Server Error"}, 500)
        """
        self.error_handler_fn = fn
        return fn

    # ── Server start ─────────────────────────────────────────────────────────

    def serve(
        self,
        host: str = "127.0.0.1",
        port: int = 3000,
        max_connections: int = 1024,
    ) -> NativeServer:  # noqa: F821
        """Start the server and block until stopped.

        Creates a ``NativeServer``, prints a startup banner, then calls
        ``server.serve()`` (blocking). Returns the server object if
        ``serve()`` returns (e.g. after ``server.stop()``).
        """
        from .server import NativeServer  # avoid circular import at module load

        server = NativeServer(
            self, host=host, port=port, max_connections=max_connections
        )
        print(
            f"Conduit listening on http://{server.local_host()}:{server.local_port()}"
        )
        server.serve()
        return server


# ── Pattern conversion ───────────────────────────────────────────────────────

# Flask uses <param> syntax; Rust web-core uses :param syntax.
# Example: "/hello/<name>" → "/hello/:name"
_FLASK_PARAM_RE = re.compile(r"<([^>]+)>")


def _flask_to_rust_pattern(pattern: str) -> str:
    """Convert Flask-style ``<param>`` placeholders to Rust ``:param`` syntax.

    Examples::

        "/hello/<name>"        → "/hello/:name"
        "/users/<id>/posts"    → "/users/:id/posts"
        "/static/file.html"    → "/static/file.html"  (unchanged)
    """
    return _FLASK_PARAM_RE.sub(r":\1", pattern)
