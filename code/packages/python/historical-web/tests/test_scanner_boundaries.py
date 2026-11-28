"""Python-only boundary cases for the regex-free historical markup scanner."""

from __future__ import annotations

from dataclasses import asdict

from historical_web import parse_html_tags_1992


def _tree(source: str) -> list[object]:
    return [
        node if isinstance(node, str) else asdict(node)
        for node in parse_html_tags_1992(source)
    ]


def test_malformed_candidates_preserve_following_text_and_tags() -> None:
    assert _tree("bad <A 'unterminated <P>ok") == [
        "bad <A 'unterminated ",
        {"tag": "p", "attributes": {}, "children": []},
        "ok",
    ]
    assert _tree('<A x="bad<P>ok') == [
        '<A x="bad',
        {"tag": "p", "attributes": {}, "children": []},
        "ok",
    ]


def test_attribute_scanner_stops_at_first_malformed_value() -> None:
    assert _tree("<A HREF=/ok BROKEN= TYPE='up>down' NAME>text</A>") == [
        {
            "tag": "a",
            "attributes": {"href": "/ok"},
            "children": ["text"],
        }
    ]


def test_raw_end_scanner_requires_exact_ascii_tag_shape() -> None:
    assert _tree("<XMP>a</XMPx>b</XmP>tail") == [
        {"tag": "xmp", "attributes": {}, "children": ["a</XMPx>b"]},
        "tail",
    ]


def test_entity_scanner_is_single_pass_and_handles_adjacent_unknowns() -> None:
    assert _tree("&lt;&amp;lt;&gt;&unknown;") == ["<&lt;>&unknown;"]
