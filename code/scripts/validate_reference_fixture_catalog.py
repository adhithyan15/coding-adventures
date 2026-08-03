#!/usr/bin/env python3
"""Validate the NN03-NN32 catalog and execute every reference validator."""

from __future__ import annotations

import argparse
import json
import math
import re
import subprocess
import sys
import threading
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "reference-validation-v1"
)
EXPECTED_FILES = {"catalog.json", "schema.json", "README.md", "CHANGELOG.md"}
EXPECTED_ORDERS = tuple(range(3, 33))
EXPECTED_FAMILY_COUNT = 30
EXPECTED_LAB_COUNT = 33
MAX_FILE_BYTES = 1_000_000
MAX_TEXT_LENGTH = 512
MAX_VALIDATOR_OUTPUT_BYTES = 16_384
VALIDATOR_TIMEOUT_SECONDS = 60
TRACK_BY_ORDER = {
    **{order: "foundation" for order in range(3, 5)},
    **{order: "spatial" for order in range(5, 9)},
    **{order: "sequence" for order in range(9, 12)},
    **{order: "attention" for order in range(12, 16)},
    **{order: "representation" for order in range(16, 20)},
    **{order: "structured" for order in range(20, 23)},
    **{order: "deep-training" for order in range(23, 26)},
    **{order: "autograd" for order in range(26, 29)},
    **{order: "compilation" for order in range(29, 33)},
}
FAMILY_ID_PATTERN = re.compile(r"[a-z][a-z0-9-]{0,79}")


class ReferenceCatalogValidationError(ValueError):
    """Raised when the catalog or a registered reference validator is invalid."""


@dataclass(frozen=True)
class ReferenceRun:
    """One successful reference-validator execution."""

    order: int
    family_id: str
    lab_count: int
    output: str


Runner = Callable[..., subprocess.CompletedProcess[str]]


class _ValidatorOutputLimitExceeded(RuntimeError):
    """Raised when a running validator exceeds its bounded evidence channel."""


def _bounded_subprocess_run(
    args: list[str],
    *,
    cwd: Path,
    capture_output: bool,
    text: bool,
    encoding: str,
    errors: str,
    timeout: int,
    check: bool,
) -> subprocess.CompletedProcess[str]:
    """Run one validator while bounding stdout and stderr during execution."""
    if not capture_output or not text:
        raise ValueError("bounded validator runner requires captured text output")

    process = subprocess.Popen(
        args,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if process.stdout is None or process.stderr is None:
        process.kill()
        process.wait()
        raise OSError("validator output pipes were not created")

    overflow = threading.Event()
    read_errors: list[BaseException] = []
    stdout_chunks: list[bytes] = []
    stderr_chunks: list[bytes] = []

    def read_bounded(stream: Any, chunks: list[bytes]) -> None:
        captured = 0
        try:
            while chunk := stream.read(4096):
                remaining = MAX_VALIDATOR_OUTPUT_BYTES + 1 - captured
                if remaining > 0:
                    chunks.append(chunk[:remaining])
                    captured += len(chunk[:remaining])
                if captured > MAX_VALIDATOR_OUTPUT_BYTES or len(chunk) > remaining:
                    overflow.set()
                    try:
                        process.kill()
                    except OSError:
                        pass
                    return
        except (OSError, ValueError) as error:  # pragma: no cover - OS boundary
            read_errors.append(error)
            try:
                process.kill()
            except OSError:
                pass

    readers = [
        threading.Thread(
            target=read_bounded, args=(process.stdout, stdout_chunks), daemon=True
        ),
        threading.Thread(
            target=read_bounded, args=(process.stderr, stderr_chunks), daemon=True
        ),
    ]
    for reader in readers:
        reader.start()
    try:
        returncode = process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()
        raise
    finally:
        for reader in readers:
            reader.join()
        process.stdout.close()
        process.stderr.close()

    if read_errors:
        raise OSError(f"could not read validator output: {read_errors[0]}")
    if overflow.is_set():
        raise _ValidatorOutputLimitExceeded(
            f"validator output exceeded {MAX_VALIDATOR_OUTPUT_BYTES} bytes"
        )

    stdout = b"".join(stdout_chunks).decode(encoding, errors)
    stderr = b"".join(stderr_chunks).decode(encoding, errors)
    completed = subprocess.CompletedProcess(args, returncode, stdout, stderr)
    if check:
        completed.check_returncode()
    return completed


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ReferenceCatalogValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        if path.stat().st_size > MAX_FILE_BYTES:
            raise ReferenceCatalogValidationError(f"{path}: file is too large")
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ReferenceCatalogValidationError(f"non-finite JSON number: {token}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise ReferenceCatalogValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise ReferenceCatalogValidationError(
            f"{path}: top-level value must be an object"
        )
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ReferenceCatalogValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise ReferenceCatalogValidationError(
            f"{context}: keys differ; missing={sorted(missing)}, extra={sorted(extra)}"
        )
    return value


def _text(value: Any, context: str, maximum: int = MAX_TEXT_LENGTH) -> str:
    if not isinstance(value, str) or not value.strip() or len(value) > maximum:
        raise ReferenceCatalogValidationError(
            f"{context}: expected non-empty text of at most {maximum} characters"
        )
    if any(ord(character) < 32 and character not in "\n\r\t" for character in value):
        raise ReferenceCatalogValidationError(
            f"{context}: contains a control character"
        )
    return value


def _integer(value: Any, minimum: int, maximum: int, context: str) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or value < minimum
        or value > maximum
    ):
        raise ReferenceCatalogValidationError(
            f"{context}: expected an integer in [{minimum}, {maximum}]"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ReferenceCatalogValidationError(f"{context}: expected a number")
    number = float(value)
    if not math.isfinite(number):
        raise ReferenceCatalogValidationError(f"{context}: expected a finite number")
    return number


def _repo_path(value: Any, allowed_root: str, context: str) -> Path:
    text = _text(value, context, 240)
    if "\\" in text:
        raise ReferenceCatalogValidationError(
            f"{context}: paths must use forward slashes"
        )
    relative = PurePosixPath(text)
    if (
        relative.is_absolute()
        or relative.as_posix() != text
        or "." in relative.parts
        or ".." in relative.parts
    ):
        raise ReferenceCatalogValidationError(
            f"{context}: path is not normalized and relative"
        )
    resolved = (REPO_ROOT / Path(*relative.parts)).resolve()
    allowed = (REPO_ROOT / allowed_root).resolve()
    try:
        resolved.relative_to(allowed)
    except ValueError as error:
        raise ReferenceCatalogValidationError(
            f"{context}: path escapes {allowed_root}"
        ) from error
    if not resolved.exists():
        raise ReferenceCatalogValidationError(f"{context}: path does not exist")
    return resolved


def _validate_hand_check(value: Any) -> dict[str, Any]:
    check = _object(
        value,
        {
            "equation",
            "stored",
            "recomputed",
            "absolute_tolerance",
            "absolute_error",
            "passes",
        },
        "protocol.hand_check",
    )
    if check["equation"] != "absolute_error = |recomputed - stored|":
        raise ReferenceCatalogValidationError("hand-check equation is not canonical")
    stored = _number(check["stored"], "protocol.hand_check.stored")
    recomputed = _number(check["recomputed"], "protocol.hand_check.recomputed")
    tolerance = _number(
        check["absolute_tolerance"], "protocol.hand_check.absolute_tolerance"
    )
    expected_recomputed = 0.1 + 0.05
    error = abs(recomputed - stored)
    if tolerance <= 0 or recomputed != expected_recomputed:
        raise ReferenceCatalogValidationError("hand-check operands are not canonical")
    if _number(check["absolute_error"], "protocol.hand_check.absolute_error") != error:
        raise ReferenceCatalogValidationError(
            "hand-check error was not recomputed honestly"
        )
    if not isinstance(check["passes"], bool) or check["passes"] != (error <= tolerance):
        raise ReferenceCatalogValidationError("hand-check pass result is dishonest")
    return json.loads(json.dumps(check))


def _discover_specs() -> set[str]:
    result: set[str] = set()
    for path in (REPO_ROOT / "code" / "specs").glob("NN[0-9][0-9]-*labs.md"):
        try:
            order = int(path.name[2:4])
        except ValueError:
            continue
        if order in EXPECTED_ORDERS:
            result.add(path.relative_to(REPO_ROOT).as_posix())
    return result


def _discover_validators() -> set[str]:
    return {
        path.relative_to(REPO_ROOT).as_posix()
        for path in (REPO_ROOT / "code" / "scripts").glob("validate_*_labs.py")
    }


def validate_catalog_document(document: Any) -> dict[str, Any]:
    catalog = _object(
        document,
        {"schema_version", "id", "title", "question", "protocol", "families"},
        "catalog",
    )
    if catalog["schema_version"] != 1 or catalog["id"] != "neural-reference-catalog":
        raise ReferenceCatalogValidationError("catalog identity is not canonical")
    _text(catalog["title"], "catalog.title", 160)
    _text(catalog["question"], "catalog.question", 240)
    protocol = _object(
        catalog["protocol"],
        {"command", "success_exit_code", "steps", "hand_check"},
        "protocol",
    )
    if (
        protocol["command"]
        != "python code/scripts/validate_reference_fixture_catalog.py"
        or protocol["success_exit_code"] != 0
    ):
        raise ReferenceCatalogValidationError(
            "reference command contract is not canonical"
        )
    if not isinstance(protocol["steps"], list) or len(protocol["steps"]) != 4:
        raise ReferenceCatalogValidationError(
            "protocol must contain exactly four steps"
        )
    for index, step in enumerate(protocol["steps"]):
        _text(step, f"protocol.steps[{index}]", 200)
    _validate_hand_check(protocol["hand_check"])

    families = catalog["families"]
    if not isinstance(families, list) or len(families) != EXPECTED_FAMILY_COUNT:
        raise ReferenceCatalogValidationError(
            f"catalog must contain exactly {EXPECTED_FAMILY_COUNT} families"
        )
    orders: list[int] = []
    ids: set[str] = set()
    specs: set[str] = set()
    validators: set[str] = set()
    fixture_roots: set[str] = set()
    total_labs = 0
    normalized_families: list[dict[str, Any]] = []
    family_keys = {
        "order",
        "id",
        "title",
        "track",
        "spec",
        "fixture_root",
        "validator",
        "lab_count",
        "oracle",
    }
    for index, value in enumerate(families):
        context = f"families[{index}]"
        family = _object(value, family_keys, context)
        order = _integer(family["order"], 3, 32, f"{context}.order")
        identifier = _text(family["id"], f"{context}.id", 80)
        if FAMILY_ID_PATTERN.fullmatch(identifier) is None:
            raise ReferenceCatalogValidationError(f"{context}.id: invalid identifier")
        _text(family["title"], f"{context}.title", 120)
        _text(family["oracle"], f"{context}.oracle", 80)
        if family["track"] != TRACK_BY_ORDER[order]:
            raise ReferenceCatalogValidationError(
                f"{context}.track: wrong curriculum track"
            )
        spec = _text(family["spec"], f"{context}.spec", 240)
        fixture_root = _text(family["fixture_root"], f"{context}.fixture_root", 240)
        validator = _text(family["validator"], f"{context}.validator", 240)
        if (
            identifier in ids
            or spec in specs
            or validator in validators
            or fixture_root in fixture_roots
        ):
            raise ReferenceCatalogValidationError(
                f"{context}: duplicate family mapping"
            )
        spec_path = _repo_path(spec, "code/specs", f"{context}.spec")
        fixture_path = _repo_path(
            fixture_root, "code/specs/fixtures", f"{context}.fixture_root"
        )
        validator_path = _repo_path(validator, "code/scripts", f"{context}.validator")
        if not spec_path.name.startswith(f"NN{order:02d}-"):
            raise ReferenceCatalogValidationError(f"{context}.spec: NN order mismatch")
        if fixture_path.name != f"{identifier}-v1":
            raise ReferenceCatalogValidationError(
                f"{context}: family id and fixture-root name disagree"
            )
        try:
            if validator_path.stat().st_size > MAX_FILE_BYTES:
                raise ReferenceCatalogValidationError(
                    f"{context}.validator: validator file is too large"
                )
            validator_source = validator_path.read_text(encoding="utf-8")
        except OSError as error:
            raise ReferenceCatalogValidationError(
                f"{context}.validator: cannot read validator"
            ) from error
        if fixture_path.name not in validator_source:
            raise ReferenceCatalogValidationError(
                f"{context}.validator: does not name its registered fixture root"
            )
        lab_count = _integer(family["lab_count"], 1, 100, f"{context}.lab_count")
        actual_lab_count = len(list((fixture_path / "labs").glob("*.json")))
        if (
            actual_lab_count != lab_count
            or not (fixture_path / "schema.json").is_file()
        ):
            raise ReferenceCatalogValidationError(
                f"{context}: fixture lab count or schema does not match the catalog"
            )
        orders.append(order)
        ids.add(identifier)
        specs.add(spec)
        validators.add(validator)
        fixture_roots.add(fixture_root)
        total_labs += lab_count
        normalized_families.append(json.loads(json.dumps(family)))

    if tuple(orders) != EXPECTED_ORDERS:
        raise ReferenceCatalogValidationError(
            "family orders must be the sorted NN03-NN32 range"
        )
    if specs != _discover_specs():
        raise ReferenceCatalogValidationError(
            "catalog does not cover every NN03-NN32 spec"
        )
    if validators != _discover_validators():
        raise ReferenceCatalogValidationError(
            "catalog does not cover every neural-learning reference validator"
        )
    if total_labs != EXPECTED_LAB_COUNT:
        raise ReferenceCatalogValidationError(
            f"catalog must cover exactly {EXPECTED_LAB_COUNT} lab documents"
        )
    return {
        "schema_version": 1,
        "id": catalog["id"],
        "title": catalog["title"],
        "question": catalog["question"],
        "protocol": json.loads(json.dumps(protocol)),
        "families": normalized_families,
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> dict[str, Any]:
    try:
        actual_files = {
            path.relative_to(root).as_posix()
            for path in root.rglob("*")
            if path.is_file()
        }
    except OSError as error:
        raise ReferenceCatalogValidationError(
            f"cannot enumerate reference catalog fixture: {error}"
        ) from error
    if actual_files != EXPECTED_FILES:
        raise ReferenceCatalogValidationError(
            "reference catalog file roster is not canonical"
        )
    schema = load_json(root / "schema.json")
    if (
        schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema"
        or schema.get("$id")
        != "https://coding-adventures.dev/schemas/reference-validation-v1.json"
    ):
        raise ReferenceCatalogValidationError("schema identity is not canonical")
    return validate_catalog_document(load_json(root / "catalog.json"))


def _clean_output(value: str, context: str) -> str:
    encoded = value.encode("utf-8")
    if len(encoded) > MAX_VALIDATOR_OUTPUT_BYTES:
        raise ReferenceCatalogValidationError(
            f"{context}: validator output is too large"
        )
    if not value:
        return ""
    _text(value, context, MAX_VALIDATOR_OUTPUT_BYTES)
    cleaned = " | ".join(line.strip() for line in value.splitlines() if line.strip())
    return cleaned.replace(str(REPO_ROOT), ".")


def execute_catalog(
    catalog: dict[str, Any],
    *,
    family_id: str | None = None,
    runner: Runner = _bounded_subprocess_run,
) -> list[ReferenceRun]:
    selected = [
        family
        for family in catalog["families"]
        if family_id is None or family["id"] == family_id
    ]
    if not selected:
        raise ReferenceCatalogValidationError(f"unknown family id: {family_id}")
    results: list[ReferenceRun] = []
    for family in selected:
        validator_path = _repo_path(
            family["validator"], "code/scripts", f"{family['id']}.validator"
        )
        try:
            completed = runner(
                [sys.executable, str(validator_path)],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="strict",
                timeout=VALIDATOR_TIMEOUT_SECONDS,
                check=False,
            )
        except (
            OSError,
            UnicodeError,
            subprocess.TimeoutExpired,
            _ValidatorOutputLimitExceeded,
        ) as error:
            raise ReferenceCatalogValidationError(
                f"NN{family['order']:02d} {family['id']}: validator could not complete: {error}"
            ) from error
        stdout = _clean_output(completed.stdout, f"{family['id']}.stdout")
        stderr = (
            _clean_output(completed.stderr, f"{family['id']}.stderr")
            if completed.stderr.strip()
            else ""
        )
        if completed.returncode != 0:
            detail = stderr or stdout or "no validator output"
            raise ReferenceCatalogValidationError(
                f"NN{family['order']:02d} {family['id']}: reference validator failed "
                f"with exit {completed.returncode}: {detail}"
            )
        if not stdout:
            raise ReferenceCatalogValidationError(
                f"NN{family['order']:02d} {family['id']}: validator emitted no evidence"
            )
        results.append(
            ReferenceRun(
                order=family["order"],
                family_id=family["id"],
                lab_count=family["lab_count"],
                output=stdout,
            )
        )
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    parser.add_argument(
        "--family", help="run one registered family after full catalog validation"
    )
    args = parser.parse_args()
    try:
        catalog = validate_fixture_root(args.fixture_root)
        results = execute_catalog(catalog, family_id=args.family)
    except ReferenceCatalogValidationError as error:
        parser.exit(1, f"reference fixture catalog invalid: {error}\n")
    for result in results:
        print(f"NN{result.order:02d} {result.family_id}: {result.output}")
    print(
        f"validated {len(results)} reference fixture families "
        f"({sum(result.lab_count for result in results)} lab documents)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
