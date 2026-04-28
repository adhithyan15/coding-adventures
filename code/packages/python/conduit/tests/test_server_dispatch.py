"""Tests for NativeServer Python-side dispatch methods.

These tests exercise the pure-Python dispatch logic without starting the
real Rust server. We instantiate NativeServer with a patched conduit_native
module that doesn't actually bind a TCP socket.

Strategy: monkey-patch ``coding_adventures.conduit.server`` to use a fake
``conduit_native`` stub that records calls and returns a dummy capsule.
"""

from __future__ import annotations

import types
from unittest.mock import MagicMock

from coding_adventures.conduit.application import Conduit
from coding_adventures.conduit.handler_context import HandlerContext
from coding_adventures.conduit.server import NativeServer

# ── Helpers ──────────────────────────────────────────────────────────────────


def _fake_native():
    """Return a fake conduit_native module that doesn't touch Rust."""
    m = types.ModuleType("conduit_native")
    m.server_new = MagicMock(return_value=object())
    m.server_serve = MagicMock(return_value=None)
    m.server_stop = MagicMock(return_value=None)
    m.server_running = MagicMock(return_value=False)
    m.server_local_host = MagicMock(return_value="127.0.0.1")
    m.server_local_port = MagicMock(return_value=3000)
    m.server_dispose = MagicMock(return_value=None)
    return m


def _make_server(app: Conduit) -> NativeServer:
    """Create a NativeServer with a fake conduit_native module."""
    fake = _fake_native()
    # Direct approach: construct NativeServer without triggering the real import.
    server = NativeServer.__new__(NativeServer)
    server._app = app
    server._capsule = object()
    server._conduit_native = fake
    return server


def _env(
    path: str = "/",
    method: str = "GET",
    route_params: dict | None = None,
    body: str = "",
) -> dict:
    return {
        "REQUEST_METHOD": method,
        "PATH_INFO": path,
        "QUERY_STRING": "",
        "conduit.route_params": route_params or {},
        "conduit.query_params": {},
        "conduit.headers": {},
        "conduit.body": body,
        "REMOTE_ADDR": "127.0.0.1",
    }


# ── native_dispatch_route ────────────────────────────────────────────────────


class TestNativeDispatchRoute:
    def test_calls_handler_and_gets_halt_response(self) -> None:
        app = Conduit()

        @app.get("/")
        def index(ctx: HandlerContext) -> None:
            ctx.html("<h1>Hello</h1>")

        server = _make_server(app)
        result = server.native_dispatch_route(0, _env())
        assert result is not None
        assert result[0] == 200
        assert result[2] == "<h1>Hello</h1>"

    def test_handler_receives_handler_context(self) -> None:
        app = Conduit()
        received = []

        @app.get("/")
        def index(ctx: HandlerContext) -> None:
            received.append(type(ctx).__name__)
            ctx.text("ok")

        server = _make_server(app)
        server.native_dispatch_route(0, _env())
        assert received == ["HandlerContext"]

    def test_route_params_in_context(self) -> None:
        app = Conduit()

        @app.get("/hello/<name>")
        def greet(ctx: HandlerContext) -> None:
            ctx.json({"name": ctx.params["name"]})

        server = _make_server(app)
        result = server.native_dispatch_route(0, _env(route_params={"name": "alice"}))
        assert result is not None
        import json

        assert json.loads(result[2])["name"] == "alice"

    def test_handler_returns_none_without_halt(self) -> None:
        app = Conduit()

        @app.get("/")
        def silent(ctx: HandlerContext) -> None:
            pass  # doesn't call any helper

        server = _make_server(app)
        result = server.native_dispatch_route(0, _env())
        assert result is None

    def test_second_route_index(self) -> None:
        app = Conduit()

        @app.get("/a")
        def route_a(ctx: HandlerContext) -> None:
            ctx.text("A")

        @app.get("/b")
        def route_b(ctx: HandlerContext) -> None:
            ctx.text("B")

        server = _make_server(app)
        result = server.native_dispatch_route(1, _env(path="/b"))
        assert result is not None
        assert result[2] == "B"


# ── native_run_before_filters ────────────────────────────────────────────────


class TestNativeRunBeforeFilters:
    def test_no_filters_returns_none(self) -> None:
        server = _make_server(Conduit())
        assert server.native_run_before_filters(_env()) is None

    def test_filter_runs(self) -> None:
        app = Conduit()
        ran = []

        @app.before_request
        def fn(ctx: HandlerContext) -> None:
            ran.append(True)

        server = _make_server(app)
        server.native_run_before_filters(_env())
        assert ran == [True]

    def test_filter_halt_returns_response(self) -> None:
        app = Conduit()

        @app.before_request
        def maintenance(ctx: HandlerContext) -> None:
            ctx.halt(503, "Under maintenance")

        server = _make_server(app)
        result = server.native_run_before_filters(_env())
        assert result is not None
        assert result[0] == 503
        assert result[2] == "Under maintenance"

    def test_filters_run_in_order(self) -> None:
        app = Conduit()
        order = []

        @app.before_request
        def first(ctx: HandlerContext) -> None:
            order.append("first")

        @app.before_request
        def second(ctx: HandlerContext) -> None:
            order.append("second")

        server = _make_server(app)
        server.native_run_before_filters(_env())
        assert order == ["first", "second"]

    def test_first_filter_halt_stops_chain(self) -> None:
        app = Conduit()
        ran = []

        @app.before_request
        def first(ctx: HandlerContext) -> None:
            ctx.halt(503, "stop")

        @app.before_request
        def second(ctx: HandlerContext) -> None:
            ran.append("second")

        server = _make_server(app)
        server.native_run_before_filters(_env())
        assert ran == []


# ── native_run_after_filters ─────────────────────────────────────────────────


class TestNativeRunAfterFilters:
    def test_returns_original_response(self) -> None:
        app = Conduit()
        response = [200, [], "body"]

        @app.after_request
        def log(ctx: HandlerContext) -> None:
            pass

        server = _make_server(app)
        result = server.native_run_after_filters(_env(), response)
        assert result == response

    def test_filter_runs_as_side_effect(self) -> None:
        app = Conduit()
        log = []

        @app.after_request
        def fn(ctx: HandlerContext) -> None:
            log.append(ctx.path)

        server = _make_server(app)
        server.native_run_after_filters(_env(path="/test"), [200, [], ""])
        assert log == ["/test"]

    def test_halt_in_after_filter_discarded(self) -> None:
        app = Conduit()
        response = [200, [], "original"]

        @app.after_request
        def fn(ctx: HandlerContext) -> None:
            ctx.halt(500, "won't matter")

        server = _make_server(app)
        result = server.native_run_after_filters(_env(), response)
        # After-filter halt is swallowed; original response preserved.
        assert result == response


# ── native_run_not_found ──────────────────────────────────────────────────────


class TestNativeRunNotFound:
    def test_no_handler_returns_none(self) -> None:
        server = _make_server(Conduit())
        assert server.native_run_not_found(_env()) is None

    def test_custom_handler_returns_response(self) -> None:
        app = Conduit()

        @app.not_found
        def missing(ctx: HandlerContext) -> None:
            ctx.html("<h1>Not Found</h1>", 404)

        server = _make_server(app)
        result = server.native_run_not_found(_env(path="/missing"))
        assert result is not None
        assert result[0] == 404

    def test_custom_handler_receives_path(self) -> None:
        app = Conduit()
        paths = []

        @app.not_found
        def missing(ctx: HandlerContext) -> None:
            paths.append(ctx.path)
            ctx.html("", 404)

        server = _make_server(app)
        server.native_run_not_found(_env(path="/gone"))
        assert paths == ["/gone"]


# ── native_run_error_handler ──────────────────────────────────────────────────


class TestNativeRunErrorHandler:
    def test_no_handler_returns_none(self) -> None:
        server = _make_server(Conduit())
        assert server.native_run_error_handler(_env(), "oops") is None

    def test_custom_handler_returns_response(self) -> None:
        app = Conduit()

        @app.error_handler
        def on_error(ctx: HandlerContext, err: str) -> None:
            ctx.json({"error": err}, 500)

        server = _make_server(app)
        result = server.native_run_error_handler(_env(), "something went wrong")
        assert result is not None
        assert result[0] == 500
        import json

        assert json.loads(result[2])["error"] == "something went wrong"

    def test_error_message_passed_through(self) -> None:
        app = Conduit()
        errors = []

        @app.error_handler
        def on_error(ctx: HandlerContext, err: str) -> None:
            errors.append(err)
            ctx.text("err", 500)

        server = _make_server(app)
        server.native_run_error_handler(_env(), "test error")
        assert errors == ["test error"]
