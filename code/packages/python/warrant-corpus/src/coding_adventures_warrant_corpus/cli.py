"""The ``warrant-corpus`` command line (VER04i section 6.2).

::

    python -m coding_adventures_warrant_corpus <command> --repo-root R
        --corpus DIR --binding FILE --package-dir DIR [command options]

====================  =======================================================
command               what it does
====================  =======================================================
``run [--out F]``     full run; writes the manifest on a clean pass
``check F``           full run; fails if the regenerated manifest differs
``sabotage``          every control, in a child process, must print ``not ok``
``list-faults``       ``<fault>:<op>`` per control
``source-digest D``   ``warrant-src-v1`` digest of a source tree
``validate-manifest`` structural validation of a manifest against the corpus
====================  =======================================================

Exit status: ``0`` everything passed, ``1`` a verdict failed, ``2`` the run was
refused (bad corpus, bad binding, bad usage).  Output is TAP-style:
``ok <id>`` / ``not ok <id> <failure-id>``.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

from .corpus import list_control_rows, load_corpus
from .errors import LaneError, UsageError
from .jsonio import MAX_TEXT_BYTES, read_bytes, render_json, sha256_hex
from .labels import derive_labels
from .manifest import source_digest, validate_manifest
from .session import (
    ENV_FAULT,
    ENV_FILTER,
    ENV_OUT,
    RunConfig,
    check_filter_rules,
    environment,
    execute,
    parse_fault_spec,
    prepare,
)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="warrant-corpus", description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    def common(sub: argparse.ArgumentParser) -> None:
        sub.add_argument("--repo-root", type=Path, required=True)
        sub.add_argument("--corpus", type=Path, required=True)
        sub.add_argument("--binding", type=Path)
        sub.add_argument("--package-dir", type=Path)

    run = commands.add_parser("run")
    common(run)
    run.add_argument("--out", type=Path)
    check = commands.add_parser("check")
    common(check)
    check.add_argument("manifest", type=Path)
    common(commands.add_parser("sabotage"))
    common(commands.add_parser("list-faults"))
    digest = commands.add_parser("source-digest")
    digest.add_argument("root", type=Path)
    validate = commands.add_parser("validate-manifest")
    common(validate)
    validate.add_argument("manifest", type=Path)
    validate.add_argument("--registry", type=Path)
    validate.add_argument("--require-active", action="store_true")
    return parser


def _config(
    args: argparse.Namespace, fault: str | None, op: str | None, needle: str | None
) -> RunConfig:
    if args.binding is None or args.package_dir is None:
        raise UsageError("usage", "--binding and --package-dir are required")
    return RunConfig(
        args.repo_root, args.corpus, args.binding, args.package_dir, fault, op, needle
    )


def _run(args: argparse.Namespace) -> int:
    fault_text, needle, env_out = environment()
    check_filter_rules(fault_text, needle)
    fault, op = parse_fault_spec(fault_text)
    prepared = prepare(_config(args, fault, op, needle))
    outcome = execute(prepared, print)
    if not outcome.passed:
        return 1
    if args.command == "check":
        return _compare(args.manifest, outcome.manifest, prepared.corpus)
    out = getattr(args, "out", None) or (Path(env_out) if env_out else None)
    if outcome.manifest is None:
        print("# faulted or filtered run: no manifest written")
    elif out is not None:
        out.parent.mkdir(parents=True, exist_ok=True)
        text = render_json(outcome.manifest)
        out.write_text(text, encoding="utf-8", newline="\n")
        print(f"# wrote {out}")
        labels = _labels_for(prepared.corpus, outcome.manifest)
        sidecar = out.parent / LABELS_FILE
        sidecar.write_text(render_json(labels), encoding="utf-8", newline="\n")
        print(f"# wrote {sidecar}")
    return 0


LABELS_FILE = "domain-labels.json"


def _labels_for(corpus, manifest: dict[str, object]) -> dict:  # noqa: ANN001
    """The coverage-label sidecar, bound to the manifest by digest.

    The digest ignores ``toolchain.version`` (diagnostic, VER04i 8), exactly as
    ``check`` does, so a different CPython patch release is not label drift.
    """
    stable = dict(manifest)
    # The toolchain *name* stays in the digest (a lane that swapped runtimes is
    # drift); only its version is diagnostic.  Reading the name from the
    # manifest, not hard-coding "python", lets this validator accept the labels
    # of another lane's manifest (the TypeScript lane's toolchain is "node").
    stable["toolchain"] = {"name": manifest["toolchain"]["name"], "version": ""}  # type: ignore[index]
    return derive_labels(
        corpus,
        str(manifest["package"]),
        str(manifest["module"]),
        sha256_hex(render_json(stable).encode("utf-8")),
    )


def _compare(
    committed: Path,
    fresh: dict[str, object] | None,
    corpus=None,  # noqa: ANN001
) -> int:
    """``check``: the regenerated manifest must equal the committed one.

    ``toolchain.version`` is diagnostic (VER04i 8), so a different CPython patch
    release is not drift; every other byte is.
    """
    if fresh is None:
        print(
            "not ok manifest: this run cannot produce a manifest (faulted or filtered)"
        )
        return 1
    try:
        text = read_bytes(committed, MAX_TEXT_BYTES).decode("utf-8")
    except (LaneError, UnicodeDecodeError):
        print(f"not ok manifest: cannot read the committed manifest {committed.name}")
        return 1
    import json

    try:
        old = json.loads(text)
        old_version = old["toolchain"]["version"]
        new = dict(fresh)
        new["toolchain"] = {"name": fresh["toolchain"]["name"], "version": old_version}  # type: ignore[index]
    except (ValueError, KeyError, TypeError):
        print("not ok manifest: the committed manifest is not a lane manifest")
        return 1
    if render_json(new) != text:
        changed = sorted(
            key
            for key in set(new) | set(old)
            if new.get(key) != old.get(key) and key != "toolchain"
        )
        what = ", ".join(changed) or "formatting"
        print(f"not ok manifest: the committed manifest drifted ({what})")
        return 1
    print(f"ok manifest {committed.name} matches the regenerated manifest")
    if corpus is None:
        return 0
    expected = render_json(_labels_for(corpus, new))
    try:
        actual = read_bytes(committed.parent / LABELS_FILE, MAX_TEXT_BYTES).decode()
    except (LaneError, UnicodeDecodeError):
        print(f"not ok {LABELS_FILE}: missing or unreadable next to the manifest")
        return 1
    if actual != expected:
        print(f"not ok {LABELS_FILE}: differs from the labels derived from the corpus")
        return 1
    print(f"ok {LABELS_FILE} matches the labels derived from the corpus")
    return 0


def _sabotage(args: argparse.Namespace) -> int:
    """Re-run ``run`` once per control with a fault set; each must fail loudly."""
    rows = list_control_rows(args.corpus)
    failures = 0
    for row in rows:
        env = _child_env(dict(os.environ), row)
        command = [sys.executable, *_module_args(), "run", *_common_flags(args)]
        done = subprocess.run(  # noqa: S603 - fixed argv, same interpreter
            command, env=env, capture_output=True, text=True, check=False
        )
        caught = done.returncode == 1 and any(
            line.startswith("not ok ") for line in done.stdout.splitlines()
        )
        print(f"{'ok' if caught else 'not ok'} sabotage {row} (exit {done.returncode})")
        failures += 0 if caught else 1
    return 1 if failures else 0


def _child_env(base: dict[str, str], fault: str) -> dict[str, str]:
    """The environment of one sabotage child.

    The child runs ``python -m coding_adventures_warrant_corpus``, so it must be
    able to import this very package.  Relying on whatever ``PYTHONPATH`` the
    parent happened to have (or on an editable install) made the command
    environment-dependent: a caller that *replaced* ``PYTHONPATH`` to expose
    the library under test hid the runner.  So the runner's own directory is
    always first, and the caller's entries follow.
    """
    own = str(Path(__file__).resolve().parent.parent)
    existing = [
        entry for entry in base.get("PYTHONPATH", "").split(os.pathsep) if entry
    ]
    env = dict(base)
    env["PYTHONPATH"] = os.pathsep.join([own, *existing])
    env[ENV_FAULT] = fault
    env[ENV_FILTER] = "/exhaustive/"
    env.pop(ENV_OUT, None)
    return env


def _module_args() -> list[str]:
    return ["-m", "coding_adventures_warrant_corpus"]


def _common_flags(args: argparse.Namespace) -> list[str]:
    return [
        "--repo-root", str(args.repo_root),
        "--corpus", str(args.corpus),
        "--binding", str(args.binding),
        "--package-dir", str(args.package_dir),
    ]  # fmt: skip


def _validate(args: argparse.Namespace) -> int:
    corpus = load_corpus(args.repo_root, args.corpus)
    relative = args.corpus.resolve().relative_to(args.repo_root.resolve()).as_posix()
    registry = read_bytes(args.registry, MAX_TEXT_BYTES) if args.registry else None
    validate_manifest(
        read_bytes(args.manifest, MAX_TEXT_BYTES),
        corpus,
        relative,
        registry,
        require_active=args.require_active,
    )
    sidecar = args.manifest.parent / LABELS_FILE
    if sidecar.exists():
        manifest_bytes = read_bytes(args.manifest, MAX_TEXT_BYTES)
        document = validate_manifest(
            manifest_bytes,
            corpus,
            relative,
            registry,
            require_active=args.require_active,
        )
        expected = render_json(_labels_for(corpus, document))
        if read_bytes(sidecar, MAX_TEXT_BYTES).decode("utf-8", "replace") != expected:
            raise UsageError(
                "labels",
                f"{LABELS_FILE} differs from the labels derived from the corpus",
            )
    print(f"ok {args.manifest.name} is a structurally valid lane manifest")
    return 0


def main(argv: list[str] | None = None) -> int:
    """Entry point; returns the process exit status."""
    args = _parser().parse_args(argv)
    try:
        if args.command in ("run", "check"):
            return _run(args)
        if args.command == "sabotage":
            _config(args, None, None, None)
            return _sabotage(args)
        if args.command == "list-faults":
            print("\n".join(list_control_rows(args.corpus)))
            return 0
        if args.command == "source-digest":
            print(source_digest(args.root))
            return 0
        return _validate(args)
    except LaneError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
