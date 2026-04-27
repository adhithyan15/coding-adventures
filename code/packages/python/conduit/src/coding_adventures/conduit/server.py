"""
server — NativeServer: capsule wrapper and Python-side dispatch hub.

``NativeServer`` is the Python bridge between Rust's ``web-core`` and the
Conduit DSL layer. It:

1. Calls ``conduit_native.server_new(self, app, host, port, max_connections)``
   to create a Rust-side ``PyConduitServer`` wrapped in a ``PyCapsule``.
2. Stores the capsule as ``self._capsule`` for all subsequent Rust calls.
3. Implements the Python-side dispatch methods that Rust calls back into
   for every HTTP request:

       native_dispatch_route(index, env) → None | [s,h,b]
       native_run_before_filters(env)    → None | [s,h,b]
       native_run_after_filters(env, response) → [s,h,b]
       native_run_not_found(env)         → None | [s,h,b]
       native_run_error_handler(env, msg) → None | [s,h,b]

The return protocol is: ``None`` means "no override" (let Rust handle it);
a three-element list ``[status, [[name,val],...], body]`` means "use this
response."

``HaltException`` never crosses the Rust boundary. The dispatch methods catch
it and convert it to the ``[s,h,b]`` list format before returning to Rust.

This class mirrors Ruby's ``CodingAdventures::Conduit::Server`` (server.rb).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from .halt_exception import HaltException
from .handler_context import HandlerContext
from .request import Request

if TYPE_CHECKING:
    from .application import Conduit


class NativeServer:
    """Wraps the Rust ``PyConduitServer`` capsule and dispatches Python callbacks.

    Typical usage: create via ``Conduit.serve(host, port)`` or directly:

        server = NativeServer(app, host="127.0.0.1", port=3000)
        server.serve()   # blocks until stopped

    For testing, use ``serve()`` in a background thread and ``stop()`` to
    shut down after assertions.
    """

    def __init__(
        self,
        app: Conduit,
        host: str = "127.0.0.1",
        port: int = 3000,
        max_connections: int = 1024,
    ) -> None:
        # Import here (not at module top level) so that importing
        # coding_adventures.conduit does not fail on machines without the
        # compiled extension.  The extension must exist by the time NativeServer
        # is instantiated (i.e. after the BUILD script ran).
        try:
            from . import conduit_native  # type: ignore[import]
        except ImportError:
            import importlib
            import os

            _pkg_dir = os.path.dirname(__file__)
            _spec = importlib.util.spec_from_file_location(
                "conduit_native",
                next(
                    (
                        os.path.join(_pkg_dir, f)
                        for f in os.listdir(_pkg_dir)
                        if f.startswith("conduit_native") and f.endswith(".so")
                    ),
                    None,
                )
                or "",
            )
            if _spec is None or _spec.origin is None:
                raise ImportError(  # noqa: B904
                    "conduit_native extension not found. "
                    "Run the BUILD script first to compile the Rust extension."
                )
            conduit_native = importlib.util.module_from_spec(_spec)
            _spec.loader.exec_module(conduit_native)  # type: ignore[union-attr]

        self._app = app
        # server_new(owner, app, host, port, max_connections) → PyCapsule
        # owner=self so Rust can call self.native_dispatch_route(…) etc.
        self._capsule = conduit_native.server_new(
            self, app, host, int(port), int(max_connections)
        )
        self._conduit_native = conduit_native

    # ── Lifecycle ───────────────────────────────────────────────────────────

    def serve(self) -> None:
        """Start the server and block until ``stop()`` is called.

        Releases the Python GIL inside the Rust extension so other Python
        threads (including signal handlers for Ctrl-C) can run.
        """
        self._conduit_native.server_serve(self._capsule)

    def stop(self) -> None:
        """Signal the server to stop accepting new connections."""
        self._conduit_native.server_stop(self._capsule)

    def running(self) -> bool:
        """Return ``True`` if the server is currently serving requests."""
        return bool(self._conduit_native.server_running(self._capsule))

    def local_host(self) -> str:
        """Return the IP address the server is listening on."""
        return self._conduit_native.server_local_host(self._capsule)

    def local_port(self) -> int:
        """Return the port number the server is listening on."""
        return self._conduit_native.server_local_port(self._capsule)

    def dispose(self) -> None:
        """Release all server resources. Must be stopped first."""
        self._conduit_native.server_dispose(self._capsule)

    # ── Python dispatch methods (called by Rust) ─────────────────────────────
    #
    # These methods are the callback targets for the Rust extension.
    # Rust holds a reference to ``self`` (the ``owner`` PyObjectPtr) and calls
    # these methods via PyObject_CallMethodObjArgs.
    #
    # All methods follow the same contract:
    #   - Return None       → no short-circuit; Rust decides the response.
    #   - Return [s,h,b]   → use this response.
    #   - HaltException caught and converted to [s,h,b] before returning.
    #   - Other exceptions: let propagate (Rust's extract_exception_message
    #     will catch them as the PyObject_CallMethodObjArgs return NULL path).
    #
    # The env dict has the same keys as the Ruby env hash (see spec WEB03).

    def native_dispatch_route(self, index: int, env: dict) -> list | None:
        """Dispatch the route at ``index`` with the given env dict.

        Called by Rust when a route matches the incoming request. ``index``
        is the zero-based position in ``app.routes`` (the same order in which
        routes were registered).

        Returns ``None`` (fall-through, should not happen for a matched route)
        or ``[status, [[name,val],...], body]``.
        """
        route = self._app.routes[index]
        request = Request(env)
        ctx = HandlerContext(request)
        try:
            route.handler(ctx)
            # Handler returned without raising — means it didn't call any
            # response helper. This is unusual (routes should always respond).
            # Fall through to Rust's default (which will be a 200 empty body).
            return None
        except HaltException as e:
            return e.to_response()

    def native_run_before_filters(self, env: dict) -> list | None:
        """Run all before-request filters in registration order.

        Called by Rust before route lookup for every request. If any filter
        raises ``HaltException``, the response is returned immediately and no
        further filters or route handlers run.

        Returns ``None`` (no short-circuit) or ``[s,h,b]``.
        """
        request = Request(env)
        ctx = HandlerContext(request)
        try:
            for fn in self._app.before_filters:
                fn(ctx)
            return None
        except HaltException as e:
            return e.to_response()

    def native_run_after_filters(self, env: dict, response: list) -> list:
        """Run all after-request filters in registration order.

        Called by Rust after the route handler for matched routes. After
        filters are for side effects (logging, metrics) — any ``HaltException``
        they raise is swallowed and the original ``response`` is returned
        unchanged.

        Always returns a ``[s,h,b]`` list.
        """
        request = Request(env)
        ctx = HandlerContext(request)
        try:
            for fn in self._app.after_filters:
                fn(ctx)
        except HaltException:
            pass  # after-filter halts are intentionally discarded
        return response

    def native_run_not_found(self, env: dict) -> list | None:
        """Call the custom not-found handler, if one is registered.

        Called by Rust when no route matched the request. If no handler is
        registered (``app.not_found_handler is None``), returns ``None`` and
        Rust serves its built-in 404 response.

        Returns ``None`` or ``[s,h,b]``.
        """
        fn = self._app.not_found_handler
        if fn is None:
            return None
        request = Request(env)
        ctx = HandlerContext(request)
        try:
            fn(ctx)
            return None
        except HaltException as e:
            return e.to_response()

    def native_run_error_handler(self, env: dict, error: str) -> list | None:
        """Call the custom error handler with the exception message.

        Called by Rust (via ``call_error_handler_in_gil``) when a route handler
        raises a Python exception. ``error`` is the ``str()`` of the exception.

        If no error handler is registered, returns ``None`` and Rust generates
        a 500 Internal Server Error response.

        Returns ``None`` or ``[s,h,b]``.
        """
        fn = self._app.error_handler_fn
        if fn is None:
            return None
        request = Request(env)
        ctx = HandlerContext(request)
        try:
            fn(ctx, error)
            return None
        except HaltException as e:
            return e.to_response()
