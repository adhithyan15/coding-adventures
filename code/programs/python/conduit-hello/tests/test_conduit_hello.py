"""
E2E tests for conduit-hello.

These tests start a real Conduit server on a random port in a background
thread, send real HTTP/1.1 requests via urllib, and assert on the responses.

The pattern is:
  1. Create a NativeServer on a free port (port=0 lets the OS pick).
  2. Start serve() in a daemon thread.
  3. Read back the actual port with server.local_port().
  4. Send requests with urllib.request.
  5. Stop the server in teardown.

We use port=0 everywhere so tests can run in parallel without port conflicts.
"""

from __future__ import annotations

import json
import socket
import threading
import time
import urllib.error
import urllib.request
from typing import Generator

import pytest

from coding_adventures.conduit import Conduit, NativeServer

# Import the application object from hello.py (lives in the parent directory).
# We add the programs directory to sys.path so pytest can find it.
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import hello  # noqa: E402  — must come after sys.path modification


# ── Test fixtures ─────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def server() -> Generator[NativeServer, None, None]:
    """Start the conduit-hello server on a random port for the test module."""
    srv = NativeServer(hello.app, host="127.0.0.1", port=0)
    thread = threading.Thread(target=srv.serve, daemon=True)
    thread.start()
    # Wait until the server is ready to accept connections.
    _wait_for_port("127.0.0.1", srv.local_port())
    yield srv
    srv.stop()
    thread.join(timeout=5)


def _wait_for_port(host: str, port: int, timeout: float = 5.0) -> None:
    """Block until a TCP connection to host:port succeeds or timeout expires."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with socket.create_connection((host, port), timeout=0.1):
                return
        except OSError:
            time.sleep(0.05)
    raise RuntimeError(f"Server at {host}:{port} did not start in {timeout}s")


def _get(server: NativeServer, path: str, **kwargs) -> urllib.request.Request:
    url = f"http://127.0.0.1:{server.local_port()}{path}"
    return urllib.request.Request(url, method="GET", **kwargs)


def _post(server: NativeServer, path: str, data: bytes, content_type: str) -> urllib.request.Request:
    url = f"http://127.0.0.1:{server.local_port()}{path}"
    req = urllib.request.Request(url, method="POST", data=data)
    req.add_header("Content-Type", content_type)
    return req


def _fetch(req) -> tuple[int, dict[str, str], bytes]:
    """Execute a request and return (status, headers, body)."""
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, dict(resp.headers), resp.read()
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read()


# ── GET / — html helper ───────────────────────────────────────────────────────


class TestGetRoot:
    def test_status_200(self, server: NativeServer) -> None:
        status, _, _ = _fetch(_get(server, "/"))
        assert status == 200

    def test_content_type_html(self, server: NativeServer) -> None:
        _, headers, _ = _fetch(_get(server, "/"))
        assert "text/html" in headers.get("content-type", "")

    def test_body_contains_hello(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/"))
        assert b"Hello" in body


# ── GET /hello/<name> — json helper ──────────────────────────────────────────


class TestGetHello:
    def test_status_200(self, server: NativeServer) -> None:
        status, _, _ = _fetch(_get(server, "/hello/Alice"))
        assert status == 200

    def test_content_type_json(self, server: NativeServer) -> None:
        _, headers, _ = _fetch(_get(server, "/hello/Alice"))
        assert "application/json" in headers.get("content-type", "")

    def test_message_field(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/hello/Alice"))
        data = json.loads(body)
        assert data["message"] == "Hello Alice"

    def test_app_field(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/hello/World"))
        data = json.loads(body)
        assert "app" in data

    def test_different_names(self, server: NativeServer) -> None:
        for name in ["Bob", "Charlie", "Adhithya"]:
            _, _, body = _fetch(_get(server, f"/hello/{name}"))
            assert json.loads(body)["message"] == f"Hello {name}"


# ── POST /echo — request.json() ───────────────────────────────────────────────


class TestPostEcho:
    def test_status_200(self, server: NativeServer) -> None:
        req = _post(server, "/echo", b'{"ping":"pong"}', "application/json")
        status, _, _ = _fetch(req)
        assert status == 200

    def test_echoes_body(self, server: NativeServer) -> None:
        payload = {"key": "value", "num": 42}
        req = _post(server, "/echo", json.dumps(payload).encode(), "application/json")
        _, _, body = _fetch(req)
        assert json.loads(body) == payload

    def test_invalid_json_returns_400(self, server: NativeServer) -> None:
        req = _post(server, "/echo", b"{bad json", "application/json")
        status, _, _ = _fetch(req)
        assert status == 400


# ── GET /redirect ─────────────────────────────────────────────────────────────


class TestGetRedirect:
    def test_status_301(self, server: NativeServer) -> None:
        url = f"http://127.0.0.1:{server.local_port()}/redirect"
        req = urllib.request.Request(url, method="GET")
        # Do NOT follow redirects — we want to see the 301 itself.
        opener = urllib.request.build_opener(NoRedirectHandler())
        try:
            with opener.open(req) as resp:
                status = resp.status
        except urllib.error.HTTPError as e:
            status = e.code
        assert status == 301

    def test_location_header(self, server: NativeServer) -> None:
        url = f"http://127.0.0.1:{server.local_port()}/redirect"
        req = urllib.request.Request(url, method="GET")
        opener = urllib.request.build_opener(NoRedirectHandler())
        try:
            with opener.open(req) as resp:
                location = resp.headers.get("location", "")
        except urllib.error.HTTPError as e:
            location = e.headers.get("location", "")
        assert location == "/"


class NoRedirectHandler(urllib.request.HTTPRedirectHandler):
    def http_error_301(self, req, fp, code, msg, headers):
        raise urllib.error.HTTPError(req.full_url, code, msg, headers, fp)


# ── GET /halt ─────────────────────────────────────────────────────────────────


class TestGetHalt:
    def test_status_403(self, server: NativeServer) -> None:
        status, _, _ = _fetch(_get(server, "/halt"))
        assert status == 403

    def test_body_is_forbidden(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/halt"))
        assert b"Forbidden" in body


# ── GET /down — before filter ─────────────────────────────────────────────────


class TestGetDown:
    def test_status_503(self, server: NativeServer) -> None:
        status, _, _ = _fetch(_get(server, "/down"))
        assert status == 503

    def test_body_under_maintenance(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/down"))
        assert b"maintenance" in body.lower()


# ── GET /error — error handler ────────────────────────────────────────────────


class TestGetError:
    def test_status_500(self, server: NativeServer) -> None:
        status, _, _ = _fetch(_get(server, "/error"))
        assert status == 500

    def test_response_is_json(self, server: NativeServer) -> None:
        _, headers, _ = _fetch(_get(server, "/error"))
        assert "application/json" in headers.get("content-type", "")

    def test_error_field_present(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/error"))
        data = json.loads(body)
        assert "error" in data

    def test_no_detail_field_in_response(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/error"))
        data = json.loads(body)
        assert "detail" not in data, "error detail must not be exposed to clients"


# ── GET /missing — not_found handler ──────────────────────────────────────────


class TestGetMissing:
    def test_status_404(self, server: NativeServer) -> None:
        status, _, _ = _fetch(_get(server, "/definitely/not/here"))
        assert status == 404

    def test_content_type_html(self, server: NativeServer) -> None:
        _, headers, _ = _fetch(_get(server, "/missing"))
        assert "text/html" in headers.get("content-type", "")

    def test_path_in_body(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/unique-missing-path"))
        assert b"/unique-missing-path" in body

    def test_before_filter_fires_for_unmatched_routes(self, server: NativeServer) -> None:
        # The before filter for /down fires even when the path has no route.
        # /down has no route but the before filter returns 503.
        status, _, _ = _fetch(_get(server, "/down"))
        assert status == 503


# ── Settings ──────────────────────────────────────────────────────────────────


class TestSettings:
    def test_app_name_in_hello_app(self) -> None:
        assert hello.app.settings["app_name"] == "Conduit Hello"

    def test_app_name_in_response(self, server: NativeServer) -> None:
        _, _, body = _fetch(_get(server, "/"))
        assert b"Conduit Hello" in body
