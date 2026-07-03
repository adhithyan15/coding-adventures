#!/usr/bin/env python3
"""ADJ71 CAS program-cache experiment.

The experiment tests whether shared legal reasoning can be paid for once,
stored as a content-addressed executable library, and then reused for a new
case where only the input is compiled.

Domain slice: ordinary five-year naturalization eligibility under
8 U.S.C. 1427(a). This is an experiment harness, not legal advice.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import pprint
import re
import shutil
import subprocess
import sys
import time
import urllib.request
from dataclasses import dataclass
from io import BytesIO
from pathlib import Path
from typing import Any
from zipfile import ZipFile


SOURCE_URL = (
    "https://uscode.house.gov/download/releasepoints/us/pl/119/95/"
    "xml_usc08@119-95.zip"
)

SOURCE_ID = "8usc1427_release_119_95_xml_section"
SOURCE_TITLE_XML_NAME = "usc08.xml"
SOURCE_SECTION_IDENTIFIER = b'identifier="/us/usc/t8/s1427"'

SOURCE_SPAN_SPECS = [
    {
        "id": "scope_ordinary_path",
        "quote": "No person, except as otherwise provided in this subchapter, shall be naturalized unless such applicant",
        "used_for": "ordinary naturalization rule applies only when no exception path is claimed",
    },
    {
        "id": "lpr_continuous_residence_5y",
        "quote": (
            "immediately preceding the date of filing his application for naturalization has resided "
            "continuously, after being lawfully admitted for permanent residence, within the United "
            "States for at least five years"
        ),
        "used_for": "requires LPR status and at least five years continuous residence before filing",
    },
    {
        "id": "physical_presence_half",
        "quote": (
            "during the five years immediately preceding the date of filing his application has been "
            "physically present therein for periods totaling at least half of that time"
        ),
        "used_for": "requires physical presence for at least half of the five-year period",
    },
    {
        "id": "state_or_district_3m",
        "quote": (
            "has resided within the State or within the district of the Service in the United States "
            "in which the applicant filed the application for at least three months"
        ),
        "used_for": "requires at least three months residence in filing state or USCIS district",
    },
    {
        "id": "continuous_until_admission",
        "quote": (
            "has resided continuously within the United States from the date of the application up "
            "to the time of admission to citizenship"
        ),
        "used_for": "requires continuous residence from filing through admission to citizenship",
    },
    {
        "id": "good_moral_character",
        "quote": "has been and still is a person of good moral character",
        "used_for": "requires good moral character during the relevant periods",
    },
    {
        "id": "constitutional_attachment",
        "quote": "attached to the principles of the Constitution of the United States",
        "used_for": "requires attachment to constitutional principles",
    },
    {
        "id": "well_disposed",
        "quote": "well disposed to the good order and happiness of the United States",
        "used_for": "requires disposition to the good order and happiness of the United States",
    },
]


@dataclass(frozen=True)
class CaseFixture:
    case_id: str
    applicant: str
    no_exception_claimed: bool
    lpr: bool
    continuous_residence_years: float
    physical_presence_months: float
    state_district_residence_months: float
    continuous_until_admission: bool
    good_moral_character: bool
    attached_constitution: bool
    well_disposed: bool
    expected_eligible: bool


TRAINING_CASES = [
    CaseFixture("case_001_clean_pass", "Asha", True, True, 5.4, 31.0, 4.0, True, True, True, True, True),
    CaseFixture("case_002_short_residence", "Bela", True, True, 4.9, 34.0, 8.0, True, True, True, True, False),
    CaseFixture("case_003_not_lpr", "Ciro", True, False, 6.2, 40.0, 9.0, True, True, True, True, False),
    CaseFixture("case_004_short_physical_presence", "Dina", True, True, 6.0, 29.0, 5.0, True, True, True, True, False),
    CaseFixture("case_005_short_state_residence", "Emil", True, True, 5.8, 35.0, 2.0, True, True, True, True, False),
    CaseFixture("case_006_not_continuous_after_filing", "Fara", True, True, 7.0, 42.0, 7.0, False, True, True, True, False),
    CaseFixture("case_007_no_good_moral_character", "Gita", True, True, 6.1, 33.0, 6.0, True, False, True, True, False),
    CaseFixture("case_008_not_attached", "Hugo", True, True, 5.5, 32.0, 4.0, True, True, False, True, False),
    CaseFixture("case_009_not_well_disposed", "Ilya", True, True, 5.5, 32.0, 4.0, True, True, True, False, False),
    CaseFixture("case_010_exception_claimed", "Jae", False, True, 3.0, 20.0, 1.0, True, True, True, True, False),
]

HELD_OUT_CASE = CaseFixture(
    "case_011_heldout_borderline_pass",
    "Kiran",
    True,
    True,
    5.0,
    30.0,
    3.0,
    True,
    True,
    True,
    True,
    True,
)


class JsonCas:
    """Tiny SHA-256 CAS that stores exact bytes plus small metadata JSON."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.objects = root / "objects"
        self.index_path = root / "index.json"
        self.objects.mkdir(parents=True, exist_ok=True)
        self.index: dict[str, dict[str, Any]] = {}

    def put(self, data: bytes, *, kind: str, label: str, metadata: dict[str, Any] | None = None) -> str:
        digest = hashlib.sha256(data).hexdigest()
        fanout = self.objects / digest[:2]
        fanout.mkdir(parents=True, exist_ok=True)
        blob_path = fanout / digest[2:]
        if not blob_path.exists():
            blob_path.write_bytes(data)
        self.index[digest] = {
            "kind": kind,
            "label": label,
            "size": len(data),
            "sha256": digest,
            "path": str(blob_path.relative_to(self.root)),
            "metadata": metadata or {},
        }
        return digest

    def write_index(self) -> None:
        self.index_path.write_text(json.dumps(self.index, indent=2, sort_keys=True) + "\n")


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def extract_xml_section(title_xml: bytes) -> tuple[bytes, int, int]:
    identifier_at = title_xml.find(SOURCE_SECTION_IDENTIFIER)
    if identifier_at < 0:
        raise RuntimeError("could not find 8 U.S.C. 1427 section identifier in Title 8 XML")
    start = title_xml.rfind(b"<section", 0, identifier_at)
    if start < 0:
        raise RuntimeError("could not find opening section tag for 8 U.S.C. 1427")
    section_tag_re = re.compile(br"</?section\b[^>]*>")
    depth = 0
    for match in section_tag_re.finditer(title_xml, start):
        token = match.group(0)
        if token.startswith(b"</"):
            depth -= 1
            if depth == 0:
                return title_xml[start:match.end()], start, match.end()
        else:
            depth += 1
    raise RuntimeError("could not find closing section tag for 8 U.S.C. 1427")


def fetch_source() -> tuple[bytes, dict[str, Any]]:
    request = urllib.request.Request(
        SOURCE_URL,
        headers={"User-Agent": "coding-adventures-adj71-cas-program-cache-experiment/0.1"},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        zip_bytes = response.read()
    with ZipFile(BytesIO(zip_bytes)) as archive:
        title_xml = archive.read(SOURCE_TITLE_XML_NAME)
    section_bytes, section_start, section_end = extract_xml_section(title_xml)
    metadata = {
        "url": SOURCE_URL,
        "release_point": "Public Law 119-95 (05/29/2026)",
        "archive_sha256": hashlib.sha256(zip_bytes).hexdigest(),
        "archive_bytes": len(zip_bytes),
        "title_xml_name": SOURCE_TITLE_XML_NAME,
        "title_xml_sha256": hashlib.sha256(title_xml).hexdigest(),
        "title_xml_bytes": len(title_xml),
        "section_identifier": SOURCE_SECTION_IDENTIFIER.decode("ascii"),
        "section_start_in_title_xml": section_start,
        "section_end_in_title_xml": section_end,
    }
    return section_bytes, metadata


def locate_source_spans(raw: bytes, source_hash: str) -> list[dict[str, Any]]:
    spans = []
    for spec in SOURCE_SPAN_SPECS:
        needle = spec["quote"].encode("utf-8")
        start = raw.find(needle)
        if start < 0:
            raise RuntimeError(f"source quote not found: {spec['id']}")
        end = start + len(needle)
        spans.append(
            {
                "id": spec["id"],
                "source_hash": source_hash,
                "start": start,
                "end": end,
                "length": end - start,
                "quote": raw[start:end].decode("utf-8"),
                "quote_sha256": hashlib.sha256(raw[start:end]).hexdigest(),
                "used_for": spec["used_for"],
            }
        )
    spans.sort(key=lambda span: span["start"])
    return spans


def partition_source(raw: bytes, represented_spans: list[dict[str, Any]]) -> list[dict[str, Any]]:
    coverage: list[dict[str, Any]] = []
    cursor = 0
    for span in represented_spans:
        if cursor < span["start"]:
            coverage.append(
                {
                    "kind": "discarded",
                    "start": cursor,
                    "end": span["start"],
                    "length": span["start"] - cursor,
                    "reason": (
                        "source section bytes outside the ordinary 8 U.S.C. 1427(a) "
                        "five-year naturalization rule subset used in this experiment"
                    ),
                }
            )
        coverage.append(
            {
                "kind": "represented",
                "start": span["start"],
                "end": span["end"],
                "length": span["length"],
                "source_span_id": span["id"],
                "used_for": span["used_for"],
            }
        )
        cursor = span["end"]
    if cursor < len(raw):
        coverage.append(
            {
                "kind": "discarded",
                "start": cursor,
                "end": len(raw),
                "length": len(raw) - cursor,
                "reason": (
                    "source section bytes outside the ordinary 8 U.S.C. 1427(a) "
                    "five-year naturalization rule subset used in this experiment"
                ),
            }
        )
    if sum(part["length"] for part in coverage) != len(raw):
        raise AssertionError("source coverage does not account for every byte")
    return coverage


def bool_phrase(value: bool, positive: str, negative: str) -> str:
    return positive if value else negative


def render_case(fixture: CaseFixture) -> str:
    return "\n".join(
        [
            f"Applicant {fixture.applicant} claims no statutory exception to the ordinary five-year naturalization requirements."
            if fixture.no_exception_claimed
            else f"Applicant {fixture.applicant} claims a statutory exception to the ordinary five-year naturalization requirements.",
            f"Applicant {fixture.applicant} has been lawfully admitted for permanent residence."
            if fixture.lpr
            else f"Applicant {fixture.applicant} has not been lawfully admitted for permanent residence.",
            (
                f"Applicant {fixture.applicant} has resided continuously in the United States after LPR admission "
                f"for {fixture.continuous_residence_years:g} years immediately before filing."
            ),
            (
                f"Applicant {fixture.applicant} was physically present in the United States for "
                f"{fixture.physical_presence_months:g} months during the five years immediately before filing."
            ),
            (
                f"Applicant {fixture.applicant} resided in the filing state or USCIS district for "
                f"{fixture.state_district_residence_months:g} months."
            ),
            f"Applicant {fixture.applicant} has resided continuously in the United States from application to admission to citizenship."
            if fixture.continuous_until_admission
            else f"Applicant {fixture.applicant} has not resided continuously in the United States from application to admission to citizenship.",
            bool_phrase(
                fixture.good_moral_character,
                f"Applicant {fixture.applicant} has good moral character.",
                f"Applicant {fixture.applicant} does not have good moral character.",
            ),
            bool_phrase(
                fixture.attached_constitution,
                f"Applicant {fixture.applicant} is attached to the principles of the Constitution of the United States.",
                f"Applicant {fixture.applicant} is not attached to the principles of the Constitution of the United States.",
            ),
            bool_phrase(
                fixture.well_disposed,
                f"Applicant {fixture.applicant} is well disposed to the good order and happiness of the United States.",
                f"Applicant {fixture.applicant} is not well disposed to the good order and happiness of the United States.",
            ),
            (
                "Question: Is the applicant eligible for ordinary five-year naturalization "
                "under 8 U.S.C. 1427(a)?"
            ),
        ]
    ) + "\n"


FACT_LINE_PATTERNS = [
    (
        "no_exception_claimed",
        re.compile(r"^Applicant (?P<name>.+?) claims no statutory exception to the ordinary five-year naturalization requirements\.$"),
        lambda match: True,
    ),
    (
        "no_exception_claimed",
        re.compile(r"^Applicant (?P<name>.+?) claims a statutory exception to the ordinary five-year naturalization requirements\.$"),
        lambda match: False,
    ),
    (
        "lpr",
        re.compile(r"^Applicant (?P<name>.+?) has been lawfully admitted for permanent residence\.$"),
        lambda match: True,
    ),
    (
        "lpr",
        re.compile(r"^Applicant (?P<name>.+?) has not been lawfully admitted for permanent residence\.$"),
        lambda match: False,
    ),
    (
        "continuous_residence_years",
        re.compile(
            r"^Applicant (?P<name>.+?) has resided continuously in the United States after LPR admission "
            r"for (?P<value>\d+(?:\.\d+)?) years immediately before filing\.$"
        ),
        lambda match: float(match.group("value")),
    ),
    (
        "physical_presence_months",
        re.compile(
            r"^Applicant (?P<name>.+?) was physically present in the United States for "
            r"(?P<value>\d+(?:\.\d+)?) months during the five years immediately before filing\.$"
        ),
        lambda match: float(match.group("value")),
    ),
    (
        "state_district_residence_months",
        re.compile(
            r"^Applicant (?P<name>.+?) resided in the filing state or USCIS district for "
            r"(?P<value>\d+(?:\.\d+)?) months\.$"
        ),
        lambda match: float(match.group("value")),
    ),
    (
        "continuous_until_admission",
        re.compile(
            r"^Applicant (?P<name>.+?) has resided continuously in the United States from application "
            r"to admission to citizenship\.$"
        ),
        lambda match: True,
    ),
    (
        "continuous_until_admission",
        re.compile(
            r"^Applicant (?P<name>.+?) has not resided continuously in the United States from application "
            r"to admission to citizenship\.$"
        ),
        lambda match: False,
    ),
    (
        "good_moral_character",
        re.compile(r"^Applicant (?P<name>.+?) has good moral character\.$"),
        lambda match: True,
    ),
    (
        "good_moral_character",
        re.compile(r"^Applicant (?P<name>.+?) does not have good moral character\.$"),
        lambda match: False,
    ),
    (
        "attached_constitution",
        re.compile(r"^Applicant (?P<name>.+?) is attached to the principles of the Constitution of the United States\.$"),
        lambda match: True,
    ),
    (
        "attached_constitution",
        re.compile(
            r"^Applicant (?P<name>.+?) is not attached to the principles of the Constitution of the United States\.$"
        ),
        lambda match: False,
    ),
    (
        "well_disposed",
        re.compile(r"^Applicant (?P<name>.+?) is well disposed to the good order and happiness of the United States\.$"),
        lambda match: True,
    ),
    (
        "well_disposed",
        re.compile(
            r"^Applicant (?P<name>.+?) is not well disposed to the good order and happiness of the United States\.$"
        ),
        lambda match: False,
    ),
]

QUERY_RE = re.compile(r"^Question: Is the applicant eligible for ordinary five-year naturalization under 8 U\.S\.C\. 1427\(a\)\?$")


def parse_case(case_id: str, text: str, input_hash: str) -> dict[str, Any]:
    facts: dict[str, Any] = {}
    fact_refs: dict[str, Any] = {}
    coverage: list[dict[str, Any]] = []
    cursor = 0
    applicant_names: set[str] = set()
    query_seen = False

    for raw_line in text.splitlines(keepends=True):
        line_start = cursor
        line_end = cursor + len(raw_line.encode("utf-8"))
        cursor = line_end
        line = raw_line.rstrip("\n")
        if not line:
            coverage.append(
                {
                    "kind": "discarded",
                    "start": line_start,
                    "end": line_end,
                    "length": line_end - line_start,
                    "reason": "blank line separator",
                }
            )
            continue

        matched = False
        for field, pattern, extractor in FACT_LINE_PATTERNS:
            match = pattern.match(line)
            if not match:
                continue
            matched = True
            applicant_names.add(match.group("name"))
            facts[field] = extractor(match)
            fact_refs[field] = {
                "input_hash": input_hash,
                "start": line_start,
                "end": line_end,
                "text": raw_line,
            }
            coverage.append(
                {
                    "kind": "represented",
                    "role": "fact",
                    "field": field,
                    "start": line_start,
                    "end": line_end,
                    "length": line_end - line_start,
                }
            )
            break

        if matched:
            continue

        if QUERY_RE.match(line):
            query_seen = True
            coverage.append(
                {
                    "kind": "represented",
                    "role": "query",
                    "query": "eligible_under_8usc1427a_ordinary_five_year_path",
                    "start": line_start,
                    "end": line_end,
                    "length": line_end - line_start,
                }
            )
            continue

        coverage.append(
            {
                "kind": "discarded",
                "start": line_start,
                "end": line_end,
                "length": line_end - line_start,
                "reason": "unrecognized line outside experiment grammar",
            }
        )

    encoded_len = len(text.encode("utf-8"))
    if cursor != encoded_len:
        raise AssertionError("case parser cursor did not reach input end")
    if sum(part["length"] for part in coverage) != encoded_len:
        raise AssertionError("case coverage does not account for every byte")
    required_fields = {
        "no_exception_claimed",
        "lpr",
        "continuous_residence_years",
        "physical_presence_months",
        "state_district_residence_months",
        "continuous_until_admission",
        "good_moral_character",
        "attached_constitution",
        "well_disposed",
    }
    missing = sorted(required_fields - set(facts))
    if missing:
        raise RuntimeError(f"{case_id} missing facts: {', '.join(missing)}")
    if not query_seen:
        raise RuntimeError(f"{case_id} missing query")
    if len(applicant_names) != 1:
        raise RuntimeError(f"{case_id} has inconsistent applicant names: {sorted(applicant_names)}")

    return {
        "case_id": case_id,
        "applicant": next(iter(applicant_names)),
        "facts": facts,
        "fact_refs": fact_refs,
        "coverage": coverage,
        "coverage_summary": {
            "input_bytes": encoded_len,
            "represented_bytes": sum(part["length"] for part in coverage if part["kind"] == "represented"),
            "discarded_bytes": sum(part["length"] for part in coverage if part["kind"] == "discarded"),
        },
        "query": "eligible_under_8usc1427a_ordinary_five_year_path",
    }


def build_rule_library(source_spans: list[dict[str, Any]]) -> str:
    provenance = {span["id"]: span for span in source_spans}
    return strip_template_indent(
        f'''\
        """Generated ADJ71 rule library for ordinary 8 U.S.C. 1427(a) naturalization.

        This module is generated from byte-cited source spans. It is intentionally
        deterministic: no model calls, no network calls, and no wall-clock state.
        """

        RULE_PROVENANCE = {json.dumps(provenance, indent=8, sort_keys=True)}

        REQUIREMENTS = [
            {{
                "id": "ordinary_path_scope",
                "source_span_id": "scope_ordinary_path",
                "description": "no statutory exception path is claimed",
                "field": "no_exception_claimed",
                "expected": True,
            }},
            {{
                "id": "lpr_status",
                "source_span_id": "lpr_continuous_residence_5y",
                "description": "applicant has been lawfully admitted for permanent residence",
                "field": "lpr",
                "expected": True,
            }},
            {{
                "id": "continuous_residence_5y",
                "source_span_id": "lpr_continuous_residence_5y",
                "description": "continuous U.S. residence after LPR admission for at least five years before filing",
                "field": "continuous_residence_years",
                "minimum": 5.0,
            }},
            {{
                "id": "physical_presence_half_of_5y",
                "source_span_id": "physical_presence_half",
                "description": "physical presence for at least half of five years",
                "field": "physical_presence_months",
                "minimum": 30.0,
                "derived_arithmetic": "5 years * 12 months/year / 2 = 30 months",
            }},
            {{
                "id": "state_or_district_3m",
                "source_span_id": "state_or_district_3m",
                "description": "residence in filing state or USCIS district for at least three months",
                "field": "state_district_residence_months",
                "minimum": 3.0,
            }},
            {{
                "id": "continuous_until_admission",
                "source_span_id": "continuous_until_admission",
                "description": "continuous U.S. residence from application to admission to citizenship",
                "field": "continuous_until_admission",
                "expected": True,
            }},
            {{
                "id": "good_moral_character",
                "source_span_id": "good_moral_character",
                "description": "good moral character",
                "field": "good_moral_character",
                "expected": True,
            }},
            {{
                "id": "constitutional_attachment",
                "source_span_id": "constitutional_attachment",
                "description": "attachment to constitutional principles",
                "field": "attached_constitution",
                "expected": True,
            }},
            {{
                "id": "well_disposed",
                "source_span_id": "well_disposed",
                "description": "well disposed to the good order and happiness of the United States",
                "field": "well_disposed",
                "expected": True,
            }},
        ]


        def _evaluate_requirement(requirement, facts, fact_refs):
            field = requirement["field"]
            actual = facts[field]
            if "minimum" in requirement:
                passed = actual >= requirement["minimum"]
                expected = {{">=": requirement["minimum"]}}
            else:
                passed = actual == requirement["expected"]
                expected = requirement["expected"]
            return {{
                "requirement_id": requirement["id"],
                "passed": passed,
                "description": requirement["description"],
                "field": field,
                "actual": actual,
                "expected": expected,
                "source_span": RULE_PROVENANCE[requirement["source_span_id"]],
                "fact_span": fact_refs[field],
                "derived_arithmetic": requirement.get("derived_arithmetic"),
            }}


        def evaluate_case(case):
            facts = case["facts"]
            fact_refs = case["fact_refs"]
            requirements = [
                _evaluate_requirement(requirement, facts, fact_refs)
                for requirement in REQUIREMENTS
            ]
            eligible = all(requirement["passed"] for requirement in requirements)
            return {{
                "case_id": case["case_id"],
                "applicant": case["applicant"],
                "query": case["query"],
                "eligible": eligible,
                "failed_requirements": [
                    requirement["requirement_id"]
                    for requirement in requirements
                    if not requirement["passed"]
                ],
                "proof_trace": requirements,
                "engine": "generated_python_rule_library",
                "answer_time_model_calls": 0,
            }}
        '''
    )


def build_case_program(case_ir: dict[str, Any], library_hash: str) -> str:
    case_literal = pprint.pformat(case_ir, sort_dicts=True, width=100)
    return strip_template_indent(
        f'''\
        """Generated ADJ71 held-out case program.

        This program imports the cached rule library and evaluates only the fresh
        input IR. The library hash at generation time was {library_hash}.
        """

        import json

        from immigration_316a_rules import evaluate_case


        CASE_IR = {case_literal}


        if __name__ == "__main__":
            print(json.dumps(evaluate_case(CASE_IR), indent=2, sort_keys=True))
        '''
    )


def strip_template_indent(text: str) -> str:
    """Remove the eight spaces used to nest generated code in this file."""
    lines = text.lstrip("\n").splitlines()
    stripped = [line[8:] if line.startswith("        ") else line for line in lines]
    return "\n".join(stripped) + "\n"


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def load_generated_module(path: Path, module_name: str) -> Any:
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load generated module {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path(__file__).resolve().parent / "out",
        help="directory for generated CAS, library, programs, and manifests",
    )
    parser.add_argument(
        "--keep-existing",
        action="store_true",
        help="do not clear the output directory before running",
    )
    args = parser.parse_args(argv)

    started = time.perf_counter()
    out_dir = args.out_dir
    if out_dir.exists() and not args.keep_existing:
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    generated_dir = out_dir / "generated"
    generated_dir.mkdir(parents=True, exist_ok=True)
    cas = JsonCas(out_dir / "cas")

    fetched_at = dt.datetime.now(dt.timezone.utc).isoformat()
    source_raw, source_fetch_metadata = fetch_source()
    source_hash = cas.put(
        source_raw,
        kind="source_bytes",
        label=SOURCE_ID,
        metadata={**source_fetch_metadata, "fetched_at": fetched_at},
    )
    source_spans = locate_source_spans(source_raw, source_hash)
    source_coverage = partition_source(source_raw, source_spans)
    source_ir = {
        "source_id": SOURCE_ID,
        "source_hash": source_hash,
        "url": SOURCE_URL,
        "fetched_at": fetched_at,
        "fetch_metadata": source_fetch_metadata,
        "spans": source_spans,
        "coverage": source_coverage,
        "coverage_summary": {
            "source_bytes": len(source_raw),
            "represented_bytes": sum(part["length"] for part in source_coverage if part["kind"] == "represented"),
            "discarded_bytes": sum(part["length"] for part in source_coverage if part["kind"] == "discarded"),
        },
    }
    source_ir_bytes = json.dumps(source_ir, indent=2, sort_keys=True).encode("utf-8")
    source_ir_hash = cas.put(source_ir_bytes, kind="source_ir", label=f"{SOURCE_ID}_ir")

    library_source = build_rule_library(source_spans)
    library_path = generated_dir / "immigration_316a_rules.py"
    library_path.write_text(library_source)
    library_hash = cas.put(
        library_source.encode("utf-8"),
        kind="program_library",
        label="immigration_316a_rules.py",
        metadata={"derived_from_source_ir_hash": source_ir_hash},
    )

    library = load_generated_module(library_path, "immigration_316a_rules")

    case_results: list[dict[str, Any]] = []
    case_irs: list[dict[str, Any]] = []
    for fixture in TRAINING_CASES:
        case_text = render_case(fixture)
        case_hash = cas.put(
            case_text.encode("utf-8"),
            kind="case_input",
            label=f"{fixture.case_id}.txt",
            metadata={"role": "training"},
        )
        case_ir = parse_case(fixture.case_id, case_text, case_hash)
        case_ir_hash = cas.put(
            json.dumps(case_ir, indent=2, sort_keys=True).encode("utf-8"),
            kind="case_ir",
            label=f"{fixture.case_id}.json",
            metadata={"input_hash": case_hash, "role": "training"},
        )
        case_ir["case_ir_hash"] = case_ir_hash
        result = library.evaluate_case(case_ir)
        result["expected_eligible"] = fixture.expected_eligible
        result["matched_expected"] = result["eligible"] == fixture.expected_eligible
        case_results.append(result)
        case_irs.append(case_ir)

    heldout_text = render_case(HELD_OUT_CASE)
    heldout_input_hash = cas.put(
        heldout_text.encode("utf-8"),
        kind="case_input",
        label=f"{HELD_OUT_CASE.case_id}.txt",
        metadata={"role": "held_out"},
    )
    heldout_ir = parse_case(HELD_OUT_CASE.case_id, heldout_text, heldout_input_hash)
    heldout_ir_hash = cas.put(
        json.dumps(heldout_ir, indent=2, sort_keys=True).encode("utf-8"),
        kind="case_ir",
        label=f"{HELD_OUT_CASE.case_id}.json",
        metadata={"input_hash": heldout_input_hash, "role": "held_out"},
    )
    heldout_ir["case_ir_hash"] = heldout_ir_hash

    case_program_source = build_case_program(heldout_ir, library_hash)
    case_program_path = generated_dir / f"{HELD_OUT_CASE.case_id}_program.py"
    case_program_path.write_text(case_program_source)
    case_program_hash = cas.put(
        case_program_source.encode("utf-8"),
        kind="case_program",
        label=case_program_path.name,
        metadata={"input_ir_hash": heldout_ir_hash, "imports_library_hash": library_hash},
    )

    executed = subprocess.run(
        [sys.executable, str(case_program_path.resolve())],
        cwd=generated_dir,
        text=True,
        capture_output=True,
        check=True,
    )
    heldout_result = json.loads(executed.stdout)
    heldout_result["expected_eligible"] = HELD_OUT_CASE.expected_eligible
    heldout_result["matched_expected"] = heldout_result["eligible"] == HELD_OUT_CASE.expected_eligible
    heldout_result_hash = cas.put(
        json.dumps(heldout_result, indent=2, sort_keys=True).encode("utf-8"),
        kind="execution_result",
        label=f"{HELD_OUT_CASE.case_id}_result.json",
        metadata={"case_program_hash": case_program_hash},
    )

    cas.write_index()
    write_json(generated_dir / "source_ir.json", source_ir)
    write_json(generated_dir / "training_results.json", case_results)
    write_json(generated_dir / f"{HELD_OUT_CASE.case_id}_ir.json", heldout_ir)
    write_json(generated_dir / f"{HELD_OUT_CASE.case_id}_result.json", heldout_result)

    manifest = {
        "experiment": "ADJ71 CAS program-cache experiment",
        "started_at": fetched_at,
        "wallclock_seconds": round(time.perf_counter() - started, 6),
        "source": {
            "url": SOURCE_URL,
            "hash": source_hash,
            "source_ir_hash": source_ir_hash,
            "source_bytes": len(source_raw),
            "release_point": source_fetch_metadata["release_point"],
            "archive_sha256": source_fetch_metadata["archive_sha256"],
            "title_xml_sha256": source_fetch_metadata["title_xml_sha256"],
            "section_start_in_title_xml": source_fetch_metadata["section_start_in_title_xml"],
            "section_end_in_title_xml": source_fetch_metadata["section_end_in_title_xml"],
            "represented_rule_spans": len(source_spans),
            "source_coverage_summary": source_ir["coverage_summary"],
        },
        "cas": {
            "object_count": len(cas.index),
            "index": str(cas.index_path.relative_to(out_dir)),
        },
        "library": {
            "path": str(library_path.relative_to(out_dir)),
            "hash": library_hash,
            "source_span_count": len(source_spans),
            "answer_time_model_calls": 0,
        },
        "training_corpus": {
            "case_count": len(TRAINING_CASES),
            "passed_expected": sum(1 for result in case_results if result["matched_expected"]),
            "case_ids": [fixture.case_id for fixture in TRAINING_CASES],
        },
        "held_out": {
            "case_id": HELD_OUT_CASE.case_id,
            "input_hash": heldout_input_hash,
            "input_ir_hash": heldout_ir_hash,
            "case_program_hash": case_program_hash,
            "result_hash": heldout_result_hash,
            "eligible": heldout_result["eligible"],
            "expected_eligible": HELD_OUT_CASE.expected_eligible,
            "matched_expected": heldout_result["matched_expected"],
            "failed_requirements": heldout_result["failed_requirements"],
            "case_coverage_summary": heldout_ir["coverage_summary"],
        },
        "claim": (
            "The reusable rule corpus was derived once from source bytes into a CAS-backed "
            "program library. The held-out case was compiled into a case program that imported "
            "the library and executed without answer-time model calls."
        ),
    }
    write_json(out_dir / "manifest.json", manifest)

    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(run(sys.argv[1:]))
