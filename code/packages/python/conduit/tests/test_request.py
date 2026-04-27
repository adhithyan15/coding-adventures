"""Tests for Request — HTTP request wrapper."""

import pytest

from coding_adventures.conduit.halt_exception import HaltException
from coding_adventures.conduit.request import Request


def _env(
    method: str = "GET",
    path: str = "/",
    query_string: str = "",
    route_params: dict | None = None,
    query_params: dict | None = None,
    headers: dict | None = None,
    body: str = "",
    content_type: str | None = None,
    content_length: int | None = None,
) -> dict:
    """Build a minimal env dict for tests."""
    env: dict = {
        "REQUEST_METHOD": method,
        "PATH_INFO": path,
        "QUERY_STRING": query_string,
        "conduit.route_params": route_params or {},
        "conduit.query_params": query_params or {},
        "conduit.headers": headers or {},
        "conduit.body": body,
        "REMOTE_ADDR": "127.0.0.1",
    }
    if content_type is not None:
        env["conduit.content_type"] = content_type
    if content_length is not None:
        env["conduit.content_length"] = content_length
    return env


# ── Core HTTP attributes ─────────────────────────────────────────────────────


class TestRequestCoreAttributes:
    def test_method(self) -> None:
        assert Request(_env("POST")).method == "POST"

    def test_path(self) -> None:
        assert Request(_env(path="/hello/world")).path == "/hello/world"

    def test_query_string_present(self) -> None:
        assert Request(_env(query_string="page=1")).query_string == "page=1"

    def test_query_string_absent_returns_empty(self) -> None:
        assert Request(_env()).query_string == ""

    def test_remote_addr(self) -> None:
        assert Request(_env()).remote_addr == "127.0.0.1"


# ── Parameters ───────────────────────────────────────────────────────────────


class TestRequestParams:
    def test_route_params(self) -> None:
        r = Request(_env(route_params={"name": "alice"}))
        assert r.params == {"name": "alice"}

    def test_route_params_empty_when_absent(self) -> None:
        r = Request(_env())
        assert r.params == {}

    def test_query_params(self) -> None:
        r = Request(_env(query_params={"page": "2", "sort": "desc"}))
        assert r.query_params["page"] == "2"
        assert r.query_params["sort"] == "desc"

    def test_query_params_empty_when_absent(self) -> None:
        r = Request(_env())
        assert r.query_params == {}


# ── Headers ──────────────────────────────────────────────────────────────────


class TestRequestHeaders:
    def test_headers_dict(self) -> None:
        r = Request(_env(headers={"content-type": "application/json"}))
        assert r.headers["content-type"] == "application/json"

    def test_header_case_insensitive(self) -> None:
        r = Request(_env(headers={"x-custom": "foo"}))
        assert r.header("X-Custom") == "foo"
        assert r.header("x-custom") == "foo"

    def test_header_missing_returns_none(self) -> None:
        assert Request(_env()).header("x-missing") is None


# ── Body ─────────────────────────────────────────────────────────────────────


class TestRequestBody:
    def test_body_string(self) -> None:
        r = Request(_env(body='{"ping":"pong"}'))
        assert r.body == '{"ping":"pong"}'

    def test_body_empty_by_default(self) -> None:
        assert Request(_env()).body == ""


# ── JSON parsing ─────────────────────────────────────────────────────────────


class TestRequestJson:
    def test_parses_object(self) -> None:
        r = Request(_env(body='{"key": "value"}'))
        assert r.json() == {"key": "value"}

    def test_parses_array(self) -> None:
        r = Request(_env(body="[1, 2, 3]"))
        assert r.json() == [1, 2, 3]

    def test_parses_number(self) -> None:
        r = Request(_env(body="42"))
        assert r.json() == 42

    def test_memoized(self) -> None:
        r = Request(_env(body='{"a": 1}'))
        first = r.json()
        second = r.json()
        assert first is second  # same object = memoized

    def test_invalid_json_raises_halt_400(self) -> None:
        r = Request(_env(body="{bad json"))
        with pytest.raises(HaltException) as exc_info:
            r.json()
        assert exc_info.value.status == 400
        assert "invalid JSON" in exc_info.value.body

    def test_invalid_json_content_type_set(self) -> None:
        r = Request(_env(body="not-json"))
        with pytest.raises(HaltException) as exc_info:
            r.json()
        headers = dict(exc_info.value.halt_headers)
        assert "text/plain" in headers.get("content-type", "")

    def test_empty_body_invalid_json(self) -> None:
        r = Request(_env(body=""))
        with pytest.raises(HaltException) as exc_info:
            r.json()
        assert exc_info.value.status == 400


# ── Form parsing ─────────────────────────────────────────────────────────────


class TestRequestForm:
    def test_parses_key_value(self) -> None:
        r = Request(_env(body="name=alice&age=30"))
        assert r.form() == {"name": "alice", "age": "30"}

    def test_empty_body_empty_dict(self) -> None:
        assert Request(_env()).form() == {}

    def test_memoized(self) -> None:
        r = Request(_env(body="k=v"))
        assert r.form() is r.form()

    def test_url_encoded_values(self) -> None:
        r = Request(_env(body="msg=hello+world"))
        assert r.form()["msg"] == "hello world"

    def test_single_key_no_value(self) -> None:
        r = Request(_env(body="key="))
        assert r.form() == {"key": ""}


# ── Metadata ─────────────────────────────────────────────────────────────────


class TestRequestMetadata:
    def test_content_type_present(self) -> None:
        r = Request(_env(content_type="application/json"))
        assert r.content_type == "application/json"

    def test_content_type_absent_none(self) -> None:
        assert Request(_env()).content_type is None

    def test_content_length_present(self) -> None:
        r = Request(_env(content_length=42))
        assert r.content_length == 42

    def test_content_length_absent_none(self) -> None:
        assert Request(_env()).content_length is None


# ── Raw env access ────────────────────────────────────────────────────────────


class TestRequestRawAccess:
    def test_getitem(self) -> None:
        env = _env(method="DELETE")
        r = Request(env)
        assert r["REQUEST_METHOD"] == "DELETE"
