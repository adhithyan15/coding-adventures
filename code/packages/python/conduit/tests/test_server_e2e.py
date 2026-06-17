"""Real-socket end-to-end tests for the Conduit Python server.

Every other Conduit port tests its server at the *library* level over a real TCP
socket (Rust `TcpStream`, TypeScript `fetch`, Ruby threaded TCP, …). The Python
library previously only had ``test_server_dispatch.py``, which exercises the
Python dispatch logic against a *mocked* native module (no socket). That mock is
a useful unit test of the dispatch wiring, but it does not prove the real Rust
engine serves requests.

This module closes that gap: it builds a self-contained app, starts the **real**
`NativeServer` on an ephemeral port (``port=0``) in a daemon thread, and drives
every feature — routing, path params, JSON body, before-filter, ``halt``,
``redirect``, custom ``not_found`` and ``error_handler`` — over real HTTP/1.1 via
``urllib``. It mirrors the conduit-hello demo's E2E harness, but lives in the
library and constructs its own app so the library is self-tested.
"""

from __future__ import annotations

import socket
import threading
import time
import urllib.error
import urllib.request
from typing import Generator

import pytest

from coding_adventures.conduit import Conduit, NativeServer


# ── Test application (self-contained — no dependency on any demo program) ─────


def _build_app() -> Conduit:
    app = Conduit()
    app.settings["app_name"] = "Conduit E2E"

    @app.before_request
    def _maintenance(ctx) -> None:
        # Halts BEFORE route lookup, so /down returns 503 with no registered route.
        if ctx.path == "/down":
            ctx.halt(503, "Under maintenance")

    @app.get("/")
    def _index(ctx) -> None:
        ctx.html("<h1>Hello, Conduit!</h1>")

    @app.get("/hello/<name>")
    def _hello(ctx) -> None:
        ctx.json({"message": f"Hello {ctx.params['name']}"})

    @app.post("/echo")
    def _echo(ctx) -> None:
        ctx.json(ctx.request.json())

    @app.get("/redirect")
    def _redirect(ctx) -> None:
        ctx.redirect("/", 301)

    @app.get("/halt")
    def _halt(ctx) -> None:
        ctx.halt(403, "Forbidden")

    @app.get("/error")
    def _error(ctx) -> None:
        raise RuntimeError("intentional error for testing")

    @app.not_found
    def _not_found(ctx) -> None:
        ctx.html("<h1>Not Found</h1>", 404)

    @app.error_handler
    def _on_error(ctx, err) -> None:
        # Never leak internal detail to the client.
        ctx.json({"error": "Internal Server Error"}, 500)

    return app


# ── Fixtures / helpers ────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def server() -> Generator[NativeServer, None, None]:
    srv = NativeServer(_build_app(), host="127.0.0.1", port=0)
    thread = threading.Thread(target=srv.serve, daemon=True)
    thread.start()
    _wait_for_port("127.0.0.1", srv.local_port())
    yield srv
    srv.stop()
    thread.join(timeout=5)


def _wait_for_port(host: str, port: int, timeout: float = 5.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with socket.create_connection((host, port), timeout=0.1):
                return
        except OSError:
            time.sleep(0.05)
    raise RuntimeError(f"Server at {host}:{port} did not start in {timeout}s")


def _fetch(
    server: NativeServer,
    path: str,
    method: str = "GET",
    data: bytes | None = None,
    content_type: str | None = None,
) -> tuple[int, dict[str, str], bytes]:
    """Send a real HTTP request; return (status, headers, body). Does NOT follow
    redirects, so 3xx responses are asserted directly."""
    url = f"http://127.0.0.1:{server.local_port()}{path}"
    req = urllib.request.Request(url, method=method, data=data)
    if content_type is not None:
        req.add_header("Content-Type", content_type)

    class _NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *_args, **_kwargs):  # noqa: D401
            return None

    # HTTP header names are case-insensitive; the Rust engine emits them
    # lowercase. Normalise keys to lowercase so assertions don't depend on case.
    def _lower(headers) -> dict[str, str]:
        return {k.lower(): v for k, v in headers.items()}

    opener = urllib.request.build_opener(_NoRedirect)
    try:
        with opener.open(req, timeout=5) as resp:
            return resp.status, _lower(resp.headers), resp.read()
    except urllib.error.HTTPError as e:
        return e.code, _lower(e.headers), e.read()


# ── Tests ─────────────────────────────────────────────────────────────────────


def test_get_root_returns_html(server: NativeServer) -> None:
    status, headers, body = _fetch(server, "/")
    assert status == 200
    assert "text/html" in headers.get("content-type", "")
    assert b"Hello, Conduit!" in body


def test_path_param_in_json(server: NativeServer) -> None:
    status, headers, body = _fetch(server, "/hello/Adhithya")
    assert status == 200
    assert "application/json" in headers.get("content-type", "")
    assert b"Hello Adhithya" in body


def test_post_echo_round_trips_json_body(server: NativeServer) -> None:
    status, _headers, body = _fetch(
        server, "/echo", method="POST", data=b'{"ping":"pong"}',
        content_type="application/json",
    )
    assert status == 200
    assert b"pong" in body


def test_redirect_is_301_with_location(server: NativeServer) -> None:
    status, headers, _body = _fetch(server, "/redirect")
    assert status == 301
    assert headers.get("location") == "/"


def test_halt_short_circuits_with_403(server: NativeServer) -> None:
    status, _headers, _body = _fetch(server, "/halt")
    assert status == 403


def test_before_filter_blocks_down_with_503(server: NativeServer) -> None:
    status, _headers, _body = _fetch(server, "/down")
    assert status == 503


def test_error_handler_returns_500_without_leaking_detail(server: NativeServer) -> None:
    status, _headers, body = _fetch(server, "/error")
    assert status == 500
    assert b"Internal Server Error" in body
    assert b"intentional error" not in body  # internal detail must not leak


def test_unmatched_route_uses_custom_404(server: NativeServer) -> None:
    status, _headers, body = _fetch(server, "/nope")
    assert status == 404
    assert b"Not Found" in body
