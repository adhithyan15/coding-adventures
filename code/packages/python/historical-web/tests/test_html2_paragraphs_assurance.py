"""Bounded verification of the 1995 P/H1 paragraph profile.

The transition model in this file is intentionally smaller than the parser: it
only tracks one open paragraph or heading and does not reuse scanner helpers.
"""

from __future__ import annotations

import itertools
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
from coding_adventures_test_bootstrap import (
    FuzzOptions,
    PbtOptions,
    check_property,
    fuzz_function,
    gen_choice,
    gen_list,
)

from historical_web import (
    Element,
    parse_html2_paragraphs_1995,
    parse_html_tags_1992,
)

FIXTURE = json.loads(
    (
        Path(__file__).resolve().parents[4]
        / "specs/fixtures/historical-web-html2-paragraphs-v1/vectors.json"
    ).read_text(encoding="utf-8")
)

TOKENS = ("<P>", "</P>", "<H1>", "</H1>", "x")
TOKEN_KIND = {
    "<P>": "start-p",
    "</P>": "end-p",
    "<H1>": "start-h1",
    "</H1>": "end-h1",
    "x": "text",
}


def _tree(nodes: list[object]) -> list[object]:
    """Canonical fixture shape, after verifying HWEB06 has text-only children."""
    result: list[object] = []
    for node in nodes:
        assert isinstance(node, Element)
        assert node.tag in {"p", "h1"}
        assert node.attributes == {}
        assert all(isinstance(child, str) for child in node.children)
        result.append(
            {"tag": node.tag, "attributes": {}, "children": list(node.children)}
        )
    return result


def _historical_tree(nodes: list[object]) -> list[object]:
    """Canonical mixed Element/text projection for the original 1992 profile."""
    result: list[object] = []
    for node in nodes:
        if isinstance(node, str):
            result.append(node)
            continue
        assert isinstance(node, Element)
        result.append(
            {
                "tag": node.tag,
                "attributes": node.attributes,
                "children": _historical_tree(node.children),
            }
        )
    return result


def _append_text(children: list[object], text: str) -> None:
    if children and isinstance(children[-1], str):
        children[-1] += text
    else:
        children.append(text)


def _model(tokens: list[str]) -> list[object] | None:
    """Independent state model; None means the token word must be rejected."""
    roots: list[dict[str, object]] = []
    paragraph: dict[str, object] | None = None
    heading: dict[str, object] | None = None

    for token in tokens:
        kind = TOKEN_KIND[token]
        if kind == "start-p":
            if heading is not None:
                return None
            paragraph = None  # A new P implies the previous P's end.
            paragraph = {"tag": "p", "attributes": {}, "children": []}
            roots.append(paragraph)
        elif kind == "end-p":
            if paragraph is None or heading is not None:
                return None
            paragraph = None
        elif kind == "start-h1":
            if heading is not None:
                return None
            paragraph = None  # H1 start closes an open paragraph.
            heading = {"tag": "h1", "attributes": {}, "children": []}
            roots.append(heading)
        elif kind == "end-h1":
            if heading is None:
                return None
            heading = None
        elif kind == "text":
            target = heading if heading is not None else paragraph
            if target is None:
                return None
            _append_text(target["children"], token)

    if heading is not None:
        return None  # Unlike P, H1 has no omitted end at EOF.
    return roots


def _check_against_model(tokens: list[str]) -> bool:
    source = "".join(tokens)
    expected = _model(tokens)
    try:
        actual = parse_html2_paragraphs_1995(source)
    except ValueError:
        return expected is None
    return expected is not None and _tree(actual) == expected


def _assert_document_invariants(nodes: list[object]) -> None:
    seen: set[int] = set()
    for node in nodes:
        assert isinstance(node, Element)
        assert id(node) not in seen, "each top-level block must have one owner"
        seen.add(id(node))
        assert node.tag in {"p", "h1"}
        assert node.attributes == {}
        assert isinstance(node.children, list)
        assert all(isinstance(child, str) for child in node.children)


def test_new_fixture_replays_every_positive_and_negative_vector() -> None:
    assert FIXTURE["format"] == "historical-web-html2-paragraphs-v1"
    assert FIXTURE["profile"] == "html2-paragraphs-1995"
    assert FIXTURE["positive"]
    assert FIXTURE["negative"]
    assert FIXTURE["xml_comparison"]
    assert FIXTURE["historical_comparison"]

    for vector in FIXTURE["positive"]:
        parsed = parse_html2_paragraphs_1995(vector["source"])
        assert _tree(parsed) == vector["tree"], vector["id"]

    for vector in FIXTURE["negative"]:
        error_name = vector["error"]
        error_type = {"ValueError": ValueError}[error_name]
        with pytest.raises(error_type):
            parse_html2_paragraphs_1995(vector["source"])

    with pytest.raises(TypeError):
        parse_html2_paragraphs_1995(7)  # type: ignore[arg-type]


def test_xml_1_0_comparison_uses_the_same_omitted_end_source() -> None:
    by_id = {v["id"]: v for v in FIXTURE["positive"]}
    positive_id = FIXTURE["xml_comparison"][0]["html2_vector"]
    html2_case = by_id[positive_id]
    assert (
        _tree(parse_html2_paragraphs_1995(html2_case["source"])) == html2_case["tree"]
    )

    for example in FIXTURE["xml_comparison"]:
        if example["xml_well_formed"]:
            ET.fromstring(example["source"])
        else:
            with pytest.raises(ET.ParseError):
                ET.fromstring(example["source"])


def test_historical_comparisons_replay_both_public_profiles() -> None:
    for example in FIXTURE["historical_comparison"]:
        source = example["source"]
        html2 = parse_html2_paragraphs_1995(source)
        html1992 = parse_html_tags_1992(source)
        assert _tree(html2) == example["html2_tree"], example["id"]
        assert _historical_tree(html1992) == example["html_tags_1992_tree"], example[
            "id"
        ]


def test_exhaustive_short_marker_words_match_independent_transition_model() -> None:
    cases = 0
    for length in range(7):
        for word in itertools.product(TOKENS, repeat=length):
            assert _check_against_model(list(word)), (
                f"word={word!r} source={''.join(word)!r}"
            )
            cases += 1
    assert cases == sum(len(TOKENS) ** length for length in range(7)) == 19_531


def test_ver05_property_engine_checks_generated_marker_histories() -> None:
    token_words = gen_list(gen_choice(TOKENS), 0, 12)
    verdict = check_property(
        0x1995_0601,
        token_words,
        _check_against_model,
        PbtOptions(num_tests=512, ci=False),
    )
    assert verdict.passed, verdict.message
    assert verdict.cases_run == 512


def _fuzz_marker_history(data: bytes) -> None:
    tokens = [TOKENS[byte % len(TOKENS)] for byte in data]
    expected = _model(tokens)
    source = "".join(tokens)
    try:
        actual = parse_html2_paragraphs_1995(source)
    except ValueError:
        assert expected is None
        return
    assert expected is not None
    assert _tree(actual) == expected


def test_ver06_fuzz_engine_checks_real_parser_and_independent_model() -> None:
    report = fuzz_function(
        _fuzz_marker_history,
        [b"\x00", b"\x00\x04\x04", b"\x02\x04\x01", b"\x02\x03"],
        FuzzOptions(max_iterations=2_048, seed=0x1995_0602, max_input_len=64),
    )
    assert report.iterations == 2_048
    assert report.is_clean
    assert not report.crashes and not report.timeouts and not report.ooms


def _fuzz_raw_source(data: bytes) -> None:
    """Treat arbitrary bytes as a Python string while preserving undecodable bytes."""
    source = data.decode("utf-8", errors="surrogateescape")
    try:
        nodes = parse_html2_paragraphs_1995(source)
    except ValueError:
        return  # Grammar rejection is an expected outcome for the strict profile.
    _assert_document_invariants(nodes)


def test_ver06_raw_source_fuzz_covers_malformed_scanner_candidates() -> None:
    report = fuzz_function(
        _fuzz_raw_source,
        [b"<P>x</P>", b"<P class=x>", b"<!--", b"<H1>open", b"\xff\x80<P>x</P>"],
        FuzzOptions(max_iterations=2_048, seed=0x1995_0603, max_input_len=128),
    )
    assert report.iterations == 2_048
    assert report.is_clean
    assert not report.crashes and not report.timeouts and not report.ooms


def test_str_subclass_cannot_override_crlf_normalization() -> None:
    class HostileString(str):
        def replace(self, *_args: object, **_kwargs: object) -> str:
            raise AssertionError("overridden replace must not run")

    parsed = parse_html2_paragraphs_1995(HostileString("<P>line1\r\nline2</P>"))
    assert _tree(parsed) == [
        {"tag": "p", "attributes": {}, "children": ["line1\nline2"]}
    ]


def test_long_sequential_paragraphs_and_malformed_heading_boundary() -> None:
    count = 20_000
    source = "<P>x" * count
    nodes = parse_html2_paragraphs_1995(source)
    assert len(nodes) == count
    _assert_document_invariants(nodes)
    assert nodes[0].children == ["x"]
    assert nodes[-1].children == ["x"]

    headings = "<H1>x</H1>" * count
    nodes = parse_html2_paragraphs_1995(headings)
    assert len(nodes) == count
    _assert_document_invariants(nodes)

    with pytest.raises(ValueError):
        parse_html2_paragraphs_1995("<H1>" + ("x" * 100_000))
