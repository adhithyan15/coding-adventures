"""The ``warrant-lane-evidence-v1`` manifest: build, digest, validate.

A manifest is the lane's *receipt*: what ran, against which corpus, binding and
source, and what happened.  Every number in a report is meant to be read from
it, never typed by hand (VER04i section 0).

``warrant-src-v1`` -- digesting a source tree
---------------------------------------------
``source_sha256`` must change whenever code that can change the library's
behaviour changes, and never because of anything else.  The framing is the same
one Rust uses, so every lane's producers and the checker agree::

    "warrant-src-v1\\0"
    u64-LE file count
    for each file, sorted by path bytes:
        u64-LE path length,    path bytes   (relative to the source root, "/")
        u64-LE content length, content bytes

Every field is length-prefixed, so moving bytes between a path and its content,
or between two files, changes a length and therefore the digest.  For Python
the files are ``**/*.py``, ``**/*.pyi`` and ``py.typed`` (``__pycache__`` is
skipped, since it is derived output).  A symlink, a non-UTF-8 name, an
oversized file or an over-deep tree is an error: the digest must cover exactly
what the interpreter would read, so anything it cannot account for fails closed.
"""

from __future__ import annotations

import os
import platform
import re
from pathlib import Path

from .binding import Binding
from .corpus import Corpus
from .errors import CorpusError, need
from .jsonio import parse_json, sha256_hex

MAX_SOURCE_FILE_BYTES = 8 * 1024 * 1024
MAX_SOURCE_DEPTH = 32
SCHEMA = "warrant-lane-evidence-v1"
RUNNER_NAME = "warrant-corpus"
_HEX64 = re.compile(r"[0-9a-f]{64}")
_IDENT = re.compile(r"[a-z][a-z0-9_]*")
_PACKAGE_NAME = re.compile(r"[a-z0-9][a-z0-9_-]*")


def _is_source(name: str) -> bool:
    return name.endswith((".py", ".pyi")) or name == "py.typed"


def read_source_tree(root: Path) -> list[tuple[str, bytes]]:
    """Every production source file under ``root`` as ``(relative path, bytes)``."""
    files: list[tuple[str, bytes]] = []

    def walk(directory: Path, relative: str, depth: int) -> None:
        if depth > MAX_SOURCE_DEPTH:
            raise CorpusError("source-tree", "source tree is nested too deeply")
        try:
            entries = sorted(os.scandir(directory), key=lambda entry: entry.name)
        except OSError as error:
            raise CorpusError("source-tree", "cannot list the source tree") from error
        for entry in entries:
            name = entry.name
            try:
                name.encode("utf-8")
            except UnicodeEncodeError as error:
                raise CorpusError("source-tree", "non-UTF-8 file name") from error
            rel = f"{relative}/{name}" if relative else name
            if entry.is_symlink():
                raise CorpusError("source-tree", f"{rel} is a symlink")
            if entry.is_dir(follow_symlinks=False):
                if name != "__pycache__":
                    walk(Path(entry.path), rel, depth + 1)
            elif entry.is_file(follow_symlinks=False) and _is_source(name):
                with open(entry.path, "rb") as handle:
                    data = handle.read(MAX_SOURCE_FILE_BYTES + 1)
                if len(data) > MAX_SOURCE_FILE_BYTES:
                    raise CorpusError("source-tree", f"{rel} is larger than the limit")
                files.append((rel, data))

    walk(root, "", 0)
    files.sort(key=lambda item: item[0].encode("utf-8"))
    return files


def frame_sources(files: list[tuple[str, bytes]]) -> bytes:
    """The ``warrant-src-v1`` byte framing of ``files`` (see the module docstring)."""
    ordered = sorted(files, key=lambda item: item[0].encode("utf-8"))
    out = bytearray(b"warrant-src-v1\x00")
    out += len(ordered).to_bytes(8, "little")
    for path, data in ordered:
        encoded = path.encode("utf-8")
        out += len(encoded).to_bytes(8, "little") + encoded
        out += len(data).to_bytes(8, "little") + data
    return bytes(out)


def source_digest(root: Path) -> str:
    """``source_sha256`` of the source tree at ``root``."""
    need(root.is_dir(), "source-tree", "source root is not a directory")
    return sha256_hex(frame_sources(read_source_tree(root)))


def runner_digest() -> str:
    """``runner.src_sha256``: the digest of this runner package's own source."""
    return source_digest(Path(__file__).resolve().parent)


def build_manifest(
    corpus: Corpus,
    binding: Binding,
    corpus_relative: str,
    source_root: Path,
    rows: list[dict[str, object]],
) -> dict[str, object]:
    """Assemble the manifest for a clean, passing run."""
    return {
        "schema": SCHEMA,
        "lane": binding.lane,
        "package": binding.package,
        "module": binding.module,
        "spec": corpus.spec,
        "spec_sha256": corpus.spec_sha256,
        "corpus": corpus_relative,
        "corpus_sha256": corpus.corpus_sha256,
        "cases_sha256": corpus.cases_sha256,
        "runner": {
            "name": RUNNER_NAME,
            "lane": binding.lane,
            "src_sha256": runner_digest(),
        },
        "binding_sha256": sha256_hex(binding.raw),
        "source_sha256": source_digest(source_root),
        "toolchain": {"name": "python", "version": platform.python_version()},
        "evidence_seed": str(binding.seed),
        "faulted": False,
        "tests": sorted(rows, key=lambda row: str(row["id"])),
    }


# ---------------------------------------------------------------------------
# Structural validation (a Python port of WD-004a's closed ct-compare-v0 rules)
# ---------------------------------------------------------------------------
_MANIFEST_KEYS = {
    "schema", "lane", "package", "module", "spec", "spec_sha256", "corpus",
    "corpus_sha256", "cases_sha256", "runner", "binding_sha256", "source_sha256",
    "toolchain", "evidence_seed", "faulted", "tests",
}  # fmt: skip
_ROW_KEYS = {
    "example": {"id", "layer", "kind", "op", "cases", "verdict"},
    "exhaustive": {
        "id", "layer", "kind", "op", "claim", "cases", "discarded", "calls",
        "inputs_sha256", "expected_sha256", "domain", "verdict",
    },
    "negctl": {"id", "layer", "kind", "fault", "op", "target", "caught_by", "verdict"},
}  # fmt: skip


def _obj(value: object, keys: set[str], where: str) -> dict[str, object]:
    need(
        type(value) is dict and set(value) == keys,
        "manifest",
        f"{where} has unexpected fields",
    )
    assert isinstance(value, dict)
    return value


def _digest(value: object, where: str) -> str:
    need(
        type(value) is str and _HEX64.fullmatch(value) is not None,
        "manifest",
        f"{where} must be a lowercase SHA-256",
    )
    assert isinstance(value, str)
    return value


def validate_manifest(
    manifest_bytes: bytes,
    corpus: Corpus,
    corpus_relative: str,
    registry_bytes: bytes | None = None,
    *,
    require_active: bool = False,
) -> dict[str, object]:
    """Validate a lane manifest against the corpus it claims to have run.

    This mirrors ``ca-assurance-report``'s ``validate_lane_evidence`` (WD-004a):
    closed key sets, exact ids and counts, sorted rows, and joins on digests and
    controls.  It checks *structure and joins only*.  It does not run anything,
    and it cannot tell a real run from a hand-written file that happens to be
    consistent -- that is what regenerating (``check``) is for.
    """
    document = parse_json(manifest_bytes)
    root = _obj(document, _MANIFEST_KEYS, "manifest")
    need(root["schema"] == SCHEMA, "manifest", "unsupported schema")
    lane, package, module = root["lane"], root["package"], root["module"]
    need(
        type(lane) is str and _IDENT.fullmatch(lane) is not None, "manifest", "bad lane"
    )
    parts = str(package).split("/")
    need(
        len(parts) == 2
        and parts[0] == lane
        and _PACKAGE_NAME.fullmatch(parts[1]) is not None,
        "manifest",
        "package must be <lane>/<name> and match the lane",
    )
    need(module == corpus.module, "manifest", "module differs from the corpus")
    need(
        root["spec"] == corpus.spec and root["spec_sha256"] == corpus.spec_sha256,
        "manifest",
        "spec path or digest differs from the corpus",
    )
    need(root["corpus"] == corpus_relative, "manifest", "corpus path differs")
    need(
        root["corpus_sha256"] == corpus.corpus_sha256
        and root["cases_sha256"] == corpus.cases_sha256,
        "manifest",
        "corpus or cases digest differs from the files",
    )
    runner = _obj(root["runner"], {"name", "lane", "src_sha256"}, "runner")
    need(
        runner["name"] == RUNNER_NAME and runner["lane"] == lane,
        "manifest",
        "bad runner",
    )
    _digest(runner["src_sha256"], "runner.src_sha256")
    _digest(root["binding_sha256"], "binding_sha256")
    _digest(root["source_sha256"], "source_sha256")
    toolchain = _obj(root["toolchain"], {"name", "version"}, "toolchain")
    need(
        type(toolchain["name"]) is str and type(toolchain["version"]) is str,
        "manifest",
        "bad toolchain",
    )
    seed = root["evidence_seed"]
    need(
        type(seed) is str
        and re.fullmatch(r"0|[1-9][0-9]*", seed) is not None
        and int(seed) < 1 << 64,
        "manifest",
        "evidence_seed must be a canonical u64 decimal string",
    )
    need(root["faulted"] is False, "manifest", "faulted manifests are not accepted")

    tests = root["tests"]
    expected_rows = len(corpus.examples) + len(corpus.suites) + len(corpus.controls)
    need(
        type(tests) is list and len(tests) == expected_rows,
        "manifest",
        "wrong number of rows",
    )
    assert isinstance(tests, list)
    ids = [row.get("id") if type(row) is dict else None for row in tests]
    need(all(type(i) is str for i in ids), "manifest", "row without an id")
    need(len(set(ids)) == len(ids), "manifest", "duplicate test id")
    need(
        all(a < b for a, b in zip(ids, ids[1:], strict=False)),
        "manifest",
        "rows must be sorted by id",
    )  # type: ignore[operator]
    _join_rows({row["id"]: row for row in tests}, corpus)  # type: ignore[index]

    if registry_bytes is not None:
        _join_registry(
            registry_bytes, str(lane), str(package), str(module), require_active
        )
    return root


def _join_rows(rows: dict[object, dict[str, object]], corpus: Corpus) -> None:
    """Every corpus example, suite and control appears once, with matching data."""
    expected_ids = (
        {e.id for e in corpus.examples}
        | {s.id for s in corpus.suites}
        | {c.id for c in corpus.controls}
    )
    need(set(rows) == expected_ids, "manifest", "row ids differ from the corpus")
    for example in corpus.examples:
        row = _obj(rows[example.id], _ROW_KEYS["example"], "example row")
        _agrees(
            row,
            "example",
            {"op": example.op, "cases": 1},
        )
    for suite in corpus.suites:
        row = _obj(rows[suite.id], _ROW_KEYS["exhaustive"], "exhaustive row")
        need(len(suite.ops) == 1, "manifest", "profile allows one subject op per suite")
        _agrees(
            row,
            "exhaustive",
            {
                "op": suite.ops[0],
                "claim": suite.claim,
                "cases": suite.cases,
                "discarded": suite.discarded,
                "calls": suite.expected_calls,
                "domain": suite.domain_text,
                "inputs_sha256": suite.inputs_sha256,
                "expected_sha256": suite.expected_sha256,
            },
        )
    for control in corpus.controls:
        row = _obj(rows[control.id], _ROW_KEYS["negctl"], "negctl row")
        caught = row["caught_by"]
        need(
            type(caught) is list
            and bool(caught)
            and len(set(caught)) == len(caught)
            and set(caught) <= set(control.required_claims),
            "manifest",
            "caught_by must be a non-empty subset of the required claims",
        )
        _agrees(
            row,
            "negctl",
            {"fault": control.fault, "op": control.op, "target": control.target_kind},
        )


def _agrees(row: dict[str, object], kind: str, expected: dict[str, object]) -> None:
    """A row is a portable, passing ``kind`` row whose fields equal ``expected``."""
    common = {"kind": kind, "layer": "portable", "verdict": "pass"}
    need(
        all(row[key] == value for key, value in {**common, **expected}.items()),
        "manifest",
        f"{kind} row {row['id']} disagrees with the corpus",
    )


def _join_registry(
    registry_bytes: bytes, lane: str, package: str, module: str, require_active: bool
) -> None:
    registry = parse_json(registry_bytes)
    need(
        type(registry) is dict and type(registry.get("consumers")) is list,
        "registry",
        "bad registry",
    )
    assert isinstance(registry, dict)
    rows = [
        row
        for row in registry["consumers"]
        if type(row) is dict
        and row.get("lane") == lane
        and row.get("package") == package
    ]
    need(len(rows) == 1, "registry", "lane/package is not registered exactly once")
    need(
        rows[0].get("module") == module,
        "registry",
        "module differs from the registered consumer",
    )
    need(
        not require_active or rows[0].get("status") == "active",
        "registry",
        "the registered consumer is not active",
    )
