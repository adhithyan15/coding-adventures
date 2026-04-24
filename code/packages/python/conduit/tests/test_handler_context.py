"""Tests for HandlerContext — execution context for Conduit handlers."""

import json

import pytest

from coding_adventures.conduit.halt_exception import HaltException
from coding_adventures.conduit.handler_context import HandlerContext
from coding_adventures.conduit.request import Request


def _ctx(
    path: str = "/",
    method: str = "GET",
    route_params: dict | None = None,
    body: str = "",
) -> HandlerContext:
    env = {
        "REQUEST_METHOD": method,
        "PATH_INFO": path,
        "QUERY_STRING": "",
        "conduit.route_params": route_params or {},
        "conduit.query_params": {},
        "conduit.headers": {},
        "conduit.body": body,
        "REMOTE_ADDR": "127.0.0.1",
    }
    return HandlerContext(Request(env))


# ── json helper ──────────────────────────────────────────────────────────────


class TestHandlerContextJson:
    def test_raises_halt_exception(self) -> None:
        with pytest.raises(HaltException):
            _ctx().json({"ok": True})

    def test_status_200_default(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().json({"ok": True})
        assert exc_info.value.status == 200

    def test_custom_status(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().json({"error": "not found"}, 404)
        assert exc_info.value.status == 404

    def test_body_is_json(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().json({"key": "value"})
        assert json.loads(exc_info.value.body) == {"key": "value"}

    def test_content_type_application_json(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().json({})
        headers = dict(exc_info.value.halt_headers)
        assert headers.get("content-type") == "application/json; charset=utf-8"


# ── html helper ──────────────────────────────────────────────────────────────


class TestHandlerContextHtml:
    def test_raises_halt_exception(self) -> None:
        with pytest.raises(HaltException):
            _ctx().html("<h1>Hi</h1>")

    def test_status_200_default(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().html("<h1>Hi</h1>")
        assert exc_info.value.status == 200

    def test_custom_status_404(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().html("<h1>Not Found</h1>", 404)
        assert exc_info.value.status == 404

    def test_body_content(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().html("<p>Hello</p>")
        assert exc_info.value.body == "<p>Hello</p>"

    def test_content_type_text_html(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().html("")
        headers = dict(exc_info.value.halt_headers)
        assert headers.get("content-type") == "text/html; charset=utf-8"


# ── text helper ──────────────────────────────────────────────────────────────


class TestHandlerContextText:
    def test_raises_halt_exception(self) -> None:
        with pytest.raises(HaltException):
            _ctx().text("hello")

    def test_content_type_text_plain(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().text("hi")
        headers = dict(exc_info.value.halt_headers)
        assert headers.get("content-type") == "text/plain; charset=utf-8"

    def test_body_coerced_to_str(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().text(42)  # type: ignore[arg-type]
        assert exc_info.value.body == "42"


# ── halt helper ──────────────────────────────────────────────────────────────


class TestHandlerContextHalt:
    def test_raises_halt_exception(self) -> None:
        with pytest.raises(HaltException):
            _ctx().halt(403, "Forbidden")

    def test_status_body_passed_through(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().halt(503, "Under maintenance")
        assert exc_info.value.status == 503
        assert exc_info.value.body == "Under maintenance"

    def test_custom_headers(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().halt(200, "", {"x-custom": "value"})
        headers = dict(exc_info.value.halt_headers)
        assert headers.get("x-custom") == "value"

    def test_empty_body_default(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().halt(204)
        assert exc_info.value.body == ""


# ── redirect helper ──────────────────────────────────────────────────────────


class TestHandlerContextRedirect:
    def test_raises_halt_exception(self) -> None:
        with pytest.raises(HaltException):
            _ctx().redirect("/home")

    def test_302_default(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().redirect("/home")
        assert exc_info.value.status == 302

    def test_301_permanent(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().redirect("/", 301)
        assert exc_info.value.status == 301

    def test_location_header(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().redirect("/new-path")
        headers = dict(exc_info.value.halt_headers)
        assert headers.get("location") == "/new-path"

    def test_empty_body(self) -> None:
        with pytest.raises(HaltException) as exc_info:
            _ctx().redirect("/")
        assert exc_info.value.body == ""


# ── Request delegation via __getattr__ ───────────────────────────────────────


class TestHandlerContextDelegation:
    def test_path_delegated(self) -> None:
        ctx = _ctx(path="/test")
        assert ctx.path == "/test"

    def test_method_delegated(self) -> None:
        ctx = _ctx(method="POST")
        assert ctx.method == "POST"

    def test_params_delegated(self) -> None:
        ctx = _ctx(route_params={"name": "alice"})
        assert ctx.params == {"name": "alice"}

    def test_request_accessible_directly(self) -> None:
        ctx = _ctx()
        assert isinstance(ctx.request, Request)

    def test_unknown_attr_raises_attribute_error(self) -> None:
        ctx = _ctx()
        with pytest.raises(AttributeError):
            _ = ctx.definitely_not_a_real_attribute  # type: ignore[attr-defined]
