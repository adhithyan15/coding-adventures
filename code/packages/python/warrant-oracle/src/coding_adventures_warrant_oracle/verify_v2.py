"""``verify-v2``: gate an on-disk ``warrant-corpus-v2`` fixture (VER04r section 10.3, slice S4 part 2).

The fixture is **data being checked, never truth**.  The verifier reads the ``.warrant``
source and ``corpus.profile``, lets :mod:`.corpus_v2` derive what the fixture must
contain, then compares it with the bytes on disk::

    .warrant + corpus.profile --derive--> expected corpus.json, chunks/*.txt, cases.tfv
                                                   |
                  on disk: corpus.json, chunks/, cases.tfv   <--compared--+

The three checks of section 10.3 map onto the groups below (a mismatch names its group):

``lowering``      check 1: ``types ops models model_order examples`` and, per suite, ``id kind
                  claim ops requires admit body oracle result_type`` and each binder's type;
``enumeration``   check 2: ``budget chunk_size max_ir_cost`` and per suite the domains,
                  counts, digests, ``slim_*`` and the chunk files themselves;
``faults`` and ``controls``  check 3: ``faults fault_models controls accepted_blind_faults``.

Further groups are *integrity* checks the three do not cover: ``header`` (identity and
digest binding: ``profile module document spec spec_sha256 profile_sha256 generator levels
assumes``), ``files`` (the fixture directory: exactly the derived chunk files, regular files
only, size caps, ``cases.tfv``), ``bytes`` (the canonical writer: key order, indent, LF) and
``drift`` (``corpus.profile`` against the bindings' ``faults=`` lists).

Byte for byte where the spec fixes bytes (chunk files, ``cases.tfv``, the canonical form of
``corpus.json``); structurally where it is provisional (the ``corpus.json`` key set, section
1.4): the comparison is type-strict (``true`` is not ``1``) and closed (an unknown or missing
key is a mismatch).  Failures print a group, a path and a message, never input bytes.
"""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import re
import stat

from .corpus_v1 import Report, load_json
from .corpus_v2 import Derived, canonical, derive
from .errors import OracleError
from .profile import parse_profile

MAX_CORPUS_BYTES = 16 << 20
MAX_CHUNK_BYTES = 2 << 20
MAX_TFV_BYTES = 4 << 20
MAX_SPEC_BYTES = 1 << 20  # the lexer's own 1 MiB cap is the effective limit (VER04a)
MAX_SMALL_BYTES = 64 << 10
MAX_DIFFS = 12  # per group: a corrupt corpus must not flood the report

_GROUP_OF_TOP = {
    "types": "lowering",
    "ops": "lowering",
    "models": "lowering",
    "model_order": "lowering",
    "examples": "lowering",
    "budget": "enumeration",
    "chunk_size": "enumeration",
    "max_ir_cost": "enumeration",
    "suites": "enumeration",
    "faults": "faults",
    "fault_models": "faults",
    "controls": "controls",
    "accepted_blind_faults": "controls",
}
_LOWERING_SUITE_KEYS = frozenset(
    ["id", "kind", "claim", "ops", "requires", "admit", "body", "oracle", "result_type"]
)


def read_regular(path: pathlib.Path, cap: int) -> bytes:
    """Read a regular, non-symbolic-link file of at most ``cap`` bytes (section 14, Paths)."""
    try:
        # lstat first: opening a FIFO for reading blocks until a writer appears, so a
        # non-regular file must be refused *before* any open (a hostile fixture could hang CI).
        mode = os.lstat(path).st_mode
        if stat.S_ISLNK(mode):
            raise OracleError("source", "an input must not be a symbolic link")
        if not stat.S_ISREG(mode):
            raise OracleError("source", "an input must be a regular file")
        flags = (
            os.O_RDONLY
            | getattr(os, "O_NOFOLLOW", 0)
            | getattr(os, "O_BINARY", 0)
            | getattr(os, "O_NONBLOCK", 0)
        )
        fd = os.open(path, flags)
    except OSError as exc:
        raise OracleError("source", "could not open an input file") from exc
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            raise OracleError("source", "an input must be a regular file")
        with os.fdopen(fd, "rb", closefd=False) as handle:
            data = handle.read(cap + 1)
    except OSError as exc:
        raise OracleError("source", "could not read an input file") from exc
    finally:
        os.close(fd)
    if len(data) > cap:
        raise OracleError("limit", "an input file exceeds its size cap")
    return data


# -- type-strict structural comparison ---------------------------------------------------------


def _diff(want: object, got: object, path: str, out: list) -> None:
    """Append the paths at which ``got`` differs from ``want`` (type-strict, closed)."""
    if len(out) >= MAX_DIFFS:
        return
    if isinstance(want, dict):
        if not isinstance(got, dict):
            out.append((path, "a different kind of value"))
            return
        for key in sorted(set(want) | set(got)):
            if key not in got:
                out.append((f"{path}.{key}", "missing"))
            elif key not in want:
                out.append((f"{path}", "an unexpected key"))
            else:
                _diff(want[key], got[key], f"{path}.{key}", out)
        return
    if isinstance(want, list):
        if not isinstance(got, list):
            out.append((path, "a different kind of value"))
        elif len(want) != len(got):
            out.append((path, f"length {len(got)}, expected {len(want)}"))
        else:
            for i, (w, g) in enumerate(zip(want, got)):
                _diff(w, g, f"{path}[{i}]", out)
        return
    if type(want) is not type(got) or want != got:
        out.append((path, "differs"))


def _compare_suite(i: int, want: dict, got: object, report: Report) -> None:
    base = f"suites[{i}:{want['claim']}]"
    if not isinstance(got, dict):
        report.miss("enumeration", base, "a suite row is not an object")
        return
    if set(got) != set(want):
        extra = len(set(got) - set(want))
        missing = sorted(set(want) - set(got))
        report.miss(
            "enumeration", base, f"suite keys differ ({extra} unexpected; missing {missing})"
        )
    for key in sorted(set(want) & set(got)):
        diffs: list = []
        if key == "binders":
            _compare_binders(want[key], got[key], f"{base}.binders", report)
            continue
        _diff(want[key], got[key], f"{base}.{key}", diffs)
        group = "lowering" if key in _LOWERING_SUITE_KEYS else "enumeration"
        for path, message in diffs:
            report.miss(group, path, message)


def _compare_binders(want: list, got: object, base: str, report: Report) -> None:
    if not isinstance(got, list) or len(got) != len(want):
        report.miss("enumeration", base, "the binder list differs")
        return
    for i, (w, g) in enumerate(zip(want, got)):
        if not (isinstance(g, list) and len(g) == 2 and isinstance(g[1], dict)):
            report.miss("lowering", f"{base}[{i}]", "a binder is [name, {type, domain}]")
            continue
        if g[0] != w[0] or g[1].get("type") != w[1]["type"] or set(g[1]) != set(w[1]):
            report.miss("lowering", f"{base}[{i}]", "a binder name or type differs")
        diffs: list = []
        _diff(w[1]["domain"], g[1].get("domain"), f"{base}[{i}].domain", diffs)
        for path, message in diffs:
            report.miss("enumeration", path, message)


def compare_corpus(want: dict, got: dict, report: Report) -> None:
    """Group every difference between the derived and the on-disk ``corpus.json``."""
    for key in sorted(set(want) | set(got)):
        if key not in got:
            report.miss(_GROUP_OF_TOP.get(key, "header"), key, "missing")
            continue
        if key not in want:
            report.miss("header", "corpus", "an unexpected top-level key")
            continue
        if key == "suites":
            _compare_suites(want["suites"], got["suites"], report)
            continue
        diffs: list = []
        _diff(want[key], got[key], key, diffs)
        for path, message in diffs:
            report.miss(_GROUP_OF_TOP.get(key, "header"), path, message)


def _compare_suites(want: list, got: object, report: Report) -> None:
    if not isinstance(got, list) or not all(isinstance(r, dict) for r in got):
        report.miss("enumeration", "suites", "suites must be a list of objects")
        return
    if [r.get("claim") for r in got] != [r["claim"] for r in want]:
        report.miss("enumeration", "suites", "the suite set or order differs from the spec")
        return
    for i, (w, g) in enumerate(zip(want, got)):
        _compare_suite(i, w, g, report)


# -- the fixture directory --------------------------------------------------------------------


def _compare_text(name: str, want: bytes, got: bytes, report: Report, group: str = "files") -> None:
    if want == got:
        return
    wl, gl = want.split(b"\n"), got.split(b"\n")
    first = next((i for i, (a, b) in enumerate(zip(wl, gl)) if a != b), min(len(wl), len(gl)))
    report.miss(group, name, f"differs from the derived bytes (first difference at line {first + 1})")


def _check_dir(root: pathlib.Path, derived: Derived, report: Report) -> None:
    chunks = root / "chunks"
    isjunction = getattr(os.path, "isjunction", None)
    if chunks.is_symlink() or (isjunction is not None and isjunction(chunks)):
        # Reported before anything is read through it.
        report.miss("files", "chunks", "the chunks directory is a symbolic link or junction")
    else:
        for name, want in sorted(derived.chunk_files.items()):
            try:
                got = read_regular(chunks / name, MAX_CHUNK_BYTES)
            except OracleError as exc:
                report.miss("files", f"chunks/{name}", exc.message)
                continue
            _compare_text(f"chunks/{name}", want, got, report)
        try:
            present = sorted(entry.name for entry in os.scandir(chunks))
        except OSError:
            report.miss("files", "chunks", "the chunks directory is missing")
            present = []
        for name in present:
            if name not in derived.chunk_files:
                report.miss(
                    "files", "chunks", f"an undeclared file in chunks/ ({len(name)} bytes of name)"
                )
    try:
        tfv_bytes = read_regular(root / "cases.tfv", MAX_TFV_BYTES)
    except OracleError as exc:
        report.miss("files", "cases.tfv", exc.message)
    else:
        _compare_text("cases.tfv", derived.tfv, tfv_bytes, report, group="lowering")


def parse_bindings_faults(raw: bytes) -> dict[str, set[str]]:
    """``{op: {fault, ..}}`` from the ``op ... faults=a,b`` lines of an ``evidence.bindings``."""
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise OracleError("corpus", "the bindings file is not UTF-8") from exc
    out: dict[str, set[str]] = {}
    for line in text.split("\n"):
        parts = line.split()
        if len(parts) < 2 or parts[0] != "op":
            continue
        if parts[1] in out:
            raise OracleError("corpus", "an op is bound twice in the bindings file")
        faults: set[str] = set()
        for token in parts[2:]:
            if token.startswith("faults="):
                faults = {f for f in token[7:].split(",") if f}
        out[parts[1]] = faults
    return out


def _check_bindings(raw: bytes, profile_raw: bytes, report: Report) -> None:
    bound = parse_bindings_faults(raw)
    profile: dict[str, set[str]] = {}
    for op, fault in parse_profile(profile_raw).faults:
        profile.setdefault(op, set()).add(fault)
    for op in sorted(set(bound) | set(profile)):
        if bound.get(op, set()) != profile.get(op, set()):
            report.miss("drift", f"faults.{op}", "corpus.profile and the bindings' faults= differ")


# -- the gate ---------------------------------------------------------------------------------


def find_repo_root(warrant: pathlib.Path) -> pathlib.Path | None:
    """The nearest ancestor of the (resolved) warrant file that contains a ``code`` directory."""
    for parent in warrant.resolve().parents:
        if (parent / "code").is_dir():
            return parent
    return None


def expected_spec_ref(warrant: pathlib.Path, repo_root: pathlib.Path | None) -> str:
    """The repo-relative POSIX path the corpus must record as ``spec`` (section 1.4)."""
    root = repo_root if repo_root is not None else find_repo_root(warrant)
    if root is None:
        raise OracleError("source", "no repository root found for --warrant; pass --repo-root")
    try:
        return warrant.resolve().relative_to(root.resolve()).as_posix()
    except ValueError as exc:
        raise OracleError("source", "--warrant is not inside the repository root") from exc


_IDENT = re.compile(r"[A-Za-z0-9._+-]{1,64}")


def _generator_ok(value: object) -> bool:
    """``{name, version}``, each ``[A-Za-z0-9._+-]{1,64}`` (no escapes, no odd Unicode)."""
    return (
        isinstance(value, dict)
        and set(value) == {"name", "version"}
        and all(isinstance(v, str) and _IDENT.fullmatch(v) for v in value.values())
    )


def parse_expected_generator(text: str) -> tuple[str, str]:
    """``name@version`` (each ``[A-Za-z0-9._+-]{1,64}``) as ``(name, version)``, else ``source``."""
    name, at, version = text.partition("@")
    if not at or not _IDENT.fullmatch(name) or not _IDENT.fullmatch(version):
        raise OracleError("source", "--expect-generator is name@version (letters, digits, ._+-)")
    return name, version


def verify_v2(
    fixture: pathlib.Path,
    warrant: pathlib.Path,
    profile: pathlib.Path,
    bindings: pathlib.Path | None = None,
    cache: dict | None = None,
    repo_root: pathlib.Path | None = None,
    expect_generator: tuple[str, str] | None = None,
) -> tuple[Report, Derived | None]:
    """Run the three oracle checks (and the integrity checks) on a v2 fixture directory.

    ``cache`` (optional) memoises the derivation, which depends only on the spec, the
    profile, the recorded spec path and generator: a caller that verifies many corpora
    of one spec (the seeded-wrong tests) passes one dict and derives once.

    ``expect_generator`` (optional, ``(name, version)``) pins the recorded ``generator``: the
    oracle cannot derive the generator, so a build that wants a generator bump reviewed
    passes the pair it expects, and any other value is a ``header`` difference (owner
    decision [D13], section 16.8).
    """
    report = Report()
    spec_bytes = read_regular(warrant, MAX_SPEC_BYTES)
    profile_raw = read_regular(profile, MAX_SMALL_BYTES)
    if fixture.is_symlink() or not fixture.is_dir():
        raise OracleError("source", "the fixture must be a directory (not a symbolic link)")
    raw = read_regular(fixture / "corpus.json", MAX_CORPUS_BYTES)
    got = load_json(raw, MAX_CORPUS_BYTES)
    if not isinstance(got, dict):
        raise OracleError("corpus", "corpus root must be an object")
    ref = expected_spec_ref(warrant, repo_root)
    if got.get("spec") != ref:
        report.miss("header", "spec", "spec is not the repo-relative path of --warrant")
    generator = got.get("generator")
    if not _generator_ok(generator):
        report.miss("header", "generator", "generator is {name, version}, two short strings")
        generator = {"name": "?", "version": "?"}
    elif expect_generator is not None and (generator["name"], generator["version"]) != expect_generator:
        report.miss("header", "generator", "generator is not the expected name@version")
    key = (spec_bytes, profile_raw, ref, json.dumps(generator, sort_keys=True))
    if cache is not None and key in cache:
        derived = cache[key]
    else:
        derived = derive(spec_bytes, profile_raw, spec_ref=ref, generator=generator)
        if cache is not None:
            cache[key] = derived
    compare_corpus(derived.corpus, got, report)
    if report.ok and raw != canonical(derived.corpus):
        report.miss("bytes", "corpus.json", "not in the canonical writer's form (key order, indent)")
    _check_dir(fixture, derived, report)
    present = fixture / "corpus.profile"
    if present.exists() or present.is_symlink():
        try:
            if read_regular(present, MAX_SMALL_BYTES) != profile_raw:
                report.miss("header", "corpus.profile", "the fixture's corpus.profile is not --profile")
        except OracleError as exc:
            report.miss("files", "corpus.profile", exc.message)
    if bindings is not None:
        _check_bindings(read_regular(bindings, MAX_SMALL_BYTES), profile_raw, report)
    report.suites = {
        s["claim"]: {
            "cases": s["cases"],
            "discarded": s["discarded"],
            "chunks": s["chunks"],
            "slim_chunks": s["slim_chunks"],
        }
        for s in derived.corpus["suites"]
    }
    report.notes.append(f"profile_sha256 {hashlib.sha256(profile_raw).hexdigest()}")
    return report, derived
