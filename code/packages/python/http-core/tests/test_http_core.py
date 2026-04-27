from http_core import (
    BodyKind,
    Header,
    HttpVersion,
    RequestHead,
    ResponseHead,
    find_header,
    parse_content_length,
    parse_content_type,
)


def test_http_version_round_trip() -> None:
    version = HttpVersion.parse("HTTP/1.1")
    assert version.major == 1
    assert version.minor == 1
    assert str(version) == "HTTP/1.1"


def test_find_header_is_case_insensitive() -> None:
    headers = [Header("Content-Type", "text/plain"), Header("X-Test", "ok")]
    assert find_header(headers, "content-type") == "text/plain"


def test_content_length_and_content_type_helpers() -> None:
    headers = [
        Header("Content-Length", "42"),
        Header("Content-Type", "text/html; charset=utf-8"),
    ]
    assert parse_content_length(headers) == 42
    assert parse_content_type(headers) == ("text/html", "utf-8")


def test_invalid_content_length_returns_none() -> None:
    assert parse_content_length([Header("Content-Length", "forty-two")]) is None


def test_heads_delegate_to_helpers() -> None:
    request = RequestHead(
        method="POST",
        target="/submit",
        version=HttpVersion(1, 1),
        headers=[Header("Content-Length", "5")],
    )
    response = ResponseHead(
        version=HttpVersion(1, 0),
        status=200,
        reason="OK",
        headers=[Header("Content-Type", "application/json")],
    )

    assert request.content_length() == 5
    assert response.content_type() == ("application/json", None)


def test_body_kind_constructors() -> None:
    assert BodyKind.none() == BodyKind("none")
    assert BodyKind.content_length(7) == BodyKind("content-length", 7)
    assert BodyKind.until_eof() == BodyKind("until-eof")
    assert BodyKind.chunked() == BodyKind("chunked")
