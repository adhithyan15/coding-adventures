"""Tests for HaltException — short-circuit signal for response helpers."""

from coding_adventures.conduit.halt_exception import HaltException, _normalize_headers

# ── Construction ─────────────────────────────────────────────────────────────


class TestHaltExceptionConstruction:
    def test_default_body_and_headers(self) -> None:
        e = HaltException(200)
        assert e.status == 200
        assert e.body == ""
        assert e.halt_headers == []

    def test_status_coerced_to_int(self) -> None:
        e = HaltException("404")  # type: ignore[arg-type]
        assert e.status == 404
        assert isinstance(e.status, int)

    def test_body_coerced_to_str(self) -> None:
        e = HaltException(200, 42)  # type: ignore[arg-type]
        assert e.body == "42"

    def test_dict_headers_normalized(self) -> None:
        e = HaltException(200, "", {"content-type": "application/json"})
        assert e.halt_headers == [["content-type", "application/json"]]

    def test_list_headers_kept(self) -> None:
        e = HaltException(200, "", [["x-foo", "bar"]])
        assert e.halt_headers == [["x-foo", "bar"]]

    def test_none_headers_empty_list(self) -> None:
        e = HaltException(200, "", None)
        assert e.halt_headers == []

    def test_multiple_headers(self) -> None:
        headers = {"content-type": "text/html", "x-custom": "val"}
        e = HaltException(200, "", headers)
        assert len(e.halt_headers) == 2
        names = [pair[0] for pair in e.halt_headers]
        assert "content-type" in names
        assert "x-custom" in names

    def test_is_exception(self) -> None:
        e = HaltException(500, "oops")
        assert isinstance(e, Exception)

    def test_str_representation(self) -> None:
        e = HaltException(503)
        assert "503" in str(e)


# ── to_response ──────────────────────────────────────────────────────────────


class TestHaltExceptionToResponse:
    def test_returns_list(self) -> None:
        e = HaltException(200, "hello", {"content-type": "text/plain"})
        r = e.to_response()
        assert isinstance(r, list)
        assert len(r) == 3

    def test_status_first(self) -> None:
        e = HaltException(404, "not found")
        r = e.to_response()
        assert r[0] == 404

    def test_headers_second(self) -> None:
        e = HaltException(200, "", {"x-foo": "bar"})
        r = e.to_response()
        assert r[1] == [["x-foo", "bar"]]

    def test_body_third(self) -> None:
        e = HaltException(200, "the body", {})
        r = e.to_response()
        assert r[2] == "the body"

    def test_full_json_response(self) -> None:
        import json

        data = {"key": "value"}
        e = HaltException(200, json.dumps(data), {"content-type": "application/json"})
        r = e.to_response()
        assert r[0] == 200
        assert json.loads(r[2]) == data


# ── _normalize_headers ───────────────────────────────────────────────────────


class TestNormalizeHeaders:
    def test_none_returns_empty(self) -> None:
        assert _normalize_headers(None) == []

    def test_empty_dict_returns_empty(self) -> None:
        assert _normalize_headers({}) == []

    def test_empty_list_returns_empty(self) -> None:
        assert _normalize_headers([]) == []

    def test_dict_single(self) -> None:
        result = _normalize_headers({"location": "/home"})
        assert result == [["location", "/home"]]

    def test_list_passthrough(self) -> None:
        result = _normalize_headers([["x-a", "1"], ["x-b", "2"]])
        assert result == [["x-a", "1"], ["x-b", "2"]]

    def test_coerces_to_str(self) -> None:
        result = _normalize_headers({"x-count": 42})  # type: ignore[dict-item]
        assert result == [["x-count", "42"]]
