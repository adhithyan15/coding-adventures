"""Validate the language-neutral barcode-layout-1d v1 corpus."""

from __future__ import annotations

import hashlib
import json
import math
import os
import runpy
import subprocess
import tempfile
import unittest
from itertools import pairwise
from pathlib import Path, PurePosixPath
from typing import Any, ClassVar, Self

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/barcode-layout-1d-v1"
MAX_FIXTURE_BYTES = 131_072
MAX_FIXTURE_DEPTH = 8
MAX_SCHEMA_DEPTH = 24


class FixtureLoadError(ValueError):
    """Stable failure from the bounded test-only loader."""


def _read_bounded(path: Path) -> bytes:
    with path.open("rb") as source:
        if os.fstat(source.fileno()).st_size > MAX_FIXTURE_BYTES:
            raise FixtureLoadError("fixture-size-limit")
        encoded = source.read(MAX_FIXTURE_BYTES + 1)
    if len(encoded) > MAX_FIXTURE_BYTES:
        raise FixtureLoadError("fixture-size-limit")
    return encoded


def _depth_preflight(encoded: bytes, limit: int) -> None:
    try:
        text = encoded.decode("utf-8")
    except UnicodeDecodeError as error:
        raise FixtureLoadError("fixture-invalid-json") from error
    depth = 0
    in_string = False
    escaped = False
    for character in text:
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
        elif character == '"':
            in_string = True
        elif character in "[{":
            depth += 1
            if depth > limit:
                raise FixtureLoadError("fixture-depth-limit")
        elif character in "]}":
            depth -= 1


def _strict_json(encoded: bytes) -> object:
    def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate object key")
            result[key] = value
        return result

    try:
        return json.loads(
            encoded,
            object_pairs_hook=unique_object,
            parse_constant=lambda token: (_ for _ in ()).throw(ValueError(token)),
        )
    except (
        UnicodeDecodeError,
        json.JSONDecodeError,
        RecursionError,
        ValueError,
    ) as error:
        raise FixtureLoadError("fixture-invalid-json") from error


def _validate_tree(value: object, depth_limit: int) -> None:
    stack: list[tuple[object, int]] = [(value, 0)]
    while stack:
        current, depth = stack.pop()
        if depth > depth_limit:
            raise FixtureLoadError("fixture-depth-limit")
        if isinstance(current, str):
            if any(0xD800 <= ord(character) <= 0xDFFF for character in current):
                raise FixtureLoadError("fixture-invalid-scalar")
        elif isinstance(current, float) and not math.isfinite(current):
            raise FixtureLoadError("fixture-invalid-json")
        elif isinstance(current, list):
            stack.extend((item, depth + 1) for item in current)
        elif isinstance(current, dict):
            stack.extend((key, depth + 1) for key in current)
            stack.extend((item, depth + 1) for item in current.values())


def _validate_local_refs(schema: object) -> None:
    stack = [schema]
    while stack:
        current = stack.pop()
        if isinstance(current, list):
            stack.extend(current)
        elif isinstance(current, dict):
            for key, value in current.items():
                if key in {"$ref", "$dynamicRef"} and (
                    not isinstance(value, str) or not value.startswith("#/")
                ):
                    raise FixtureLoadError("fixture-schema-invalid")
                stack.append(value)


def _load(schema_encoded: bytes, document_encoded: bytes) -> dict[str, Any]:
    if max(len(schema_encoded), len(document_encoded)) > MAX_FIXTURE_BYTES:
        raise FixtureLoadError("fixture-size-limit")
    _depth_preflight(schema_encoded, MAX_SCHEMA_DEPTH)
    _depth_preflight(document_encoded, MAX_FIXTURE_DEPTH)
    schema = _strict_json(schema_encoded)
    document = _strict_json(document_encoded)
    if not isinstance(schema, dict) or not isinstance(document, dict):
        raise FixtureLoadError("fixture-schema-invalid")
    _validate_tree(schema, MAX_SCHEMA_DEPTH)
    _validate_tree(document, MAX_FIXTURE_DEPTH)
    _validate_local_refs(schema)
    try:
        Draft202012Validator.check_schema(schema)
        Draft202012Validator(schema).validate(document)
    except Exception as error:
        raise FixtureLoadError("fixture-schema-invalid") from error
    return document


def _target_package_path(entry: dict[str, Any], field: str) -> Path:
    """Resolve one registry path only when it stays inside its package root."""

    encoded_path = entry[field]
    if "\\" in encoded_path or ":" in encoded_path:
        raise FixtureLoadError("target-path-outside-package")
    package_root = PurePosixPath(entry["package_root"])
    relative_path = PurePosixPath(encoded_path)
    if relative_path.is_absolute() or ".." in relative_path.parts:
        raise FixtureLoadError("target-path-outside-package")
    try:
        relative_path.relative_to(package_root)
    except ValueError as error:
        raise FixtureLoadError("target-path-outside-package") from error
    native_package_root = (REPO_ROOT / entry["package_root"]).resolve()
    native_path = REPO_ROOT.joinpath(*relative_path.parts).resolve()
    if not native_path.is_relative_to(native_package_root):
        raise FixtureLoadError("target-path-outside-package")
    return native_path


# Dependency lockfiles a package may refresh after its lane was verified.
#
# `package_tree` binds a lane's evidence to the exact git tree that was tested,
# so the implementation, tests, manifests, and capability files cannot drift
# without re-verification. A lockfile is different: security tooling
# (Dependabot, or a repo-wide advisory sweep) rewrites it across hundreds of
# packages at once, and the lane's own CI still re-runs the conformance suite
# against the new lock on that same PR. Pinning the lock too would make every
# such sweep re-verify fifteen lanes by hand for a change nobody reviews as a
# design change.
#
#   HEAD's package tree vs package_tree    verdict
#   ------------------------------------   -----------------------------------
#   identical                              accepted
#   only files named below differ          accepted: dependency refresh
#   anything else (src, tests, manifests,  rejected: re-run the lane, then set
#   BUILD, CHANGELOG)                      package_tree to the new tree
#
# The manifests that *declare* dependency ranges (package.json, Cargo.toml,
# ...) are deliberately absent: a range change is a reviewed design change.
DEPENDENCY_LOCK_FILES = frozenset(
    {
        "package-lock.json",
        "npm-shrinkwrap.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "Cargo.lock",
        "Gemfile.lock",
        "go.sum",
        "mix.lock",
        "uv.lock",
        "poetry.lock",
        "Package.resolved",
        "pubspec.lock",
        "packages.lock.json",
        "gradle.lockfile",
    }
)


class LaneEvidenceError(AssertionError):
    """A conformant lane's package-tree evidence does not hold."""


# Why the evidence is a *tree*, not a commit
# ------------------------------------------
#
# Every PR in this repository is squash-merged. A squash builds one brand-new
# commit on main and throws the PR's own commits away, so any commit id a PR
# writes into the registry stops existing on main the moment it merges:
#
#   PR branch   A --- B (lane changed, tests pass) --- C (registry pins B)
#                                 |
#                                 |  squash merge
#                                 v
#   main        M --------------- S          B and C are not reachable from S
#
# A tree id is different. Git names a directory by the hash of its contents,
# so `B:<package_root>` and `S:<package_root>` are the *same object* whenever
# the squash carried the same bytes, which it always does. Pinning the tree
# therefore survives the merge with no follow-up repoint. The commit stays in
# the registry as provenance and is cross-checked whenever git can still see
# it; when the squash has discarded it, the tree alone carries the proof.
#
#   package_tree       verified_revision       verdict
#   -----------------  ----------------------  -----------------------------
#   not a tree object  (any)                   rejected
#   tree               resolvable commit,      rejected: the pin contradicts
#                      different package tree  the tree it claims
#   tree               resolvable commit,      checked against HEAD below
#                      same package tree
#   tree               unresolvable or not a   checked against HEAD below
#                      HEAD ancestor (squash)
#   tree               resolves, not a commit  rejected


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    """Run one git command in ``repo`` without raising; callers read the status."""

    return subprocess.run(
        ["git", *args],
        cwd=repo,
        check=False,
        capture_output=True,
        text=True,
    )


def _object_type(repo: Path, object_id: str) -> str | None:
    """The git object type of ``object_id`` (``tree``, ``commit``...), or None."""

    result = _git(repo, "cat-file", "-t", object_id)
    return result.stdout.strip() if result.returncode == 0 else None


def _tree_at(repo: Path, revision: str, package_root: str) -> str | None:
    """The tree id of ``package_root`` at ``revision``, or None if it is absent."""

    result = _git(
        repo, "rev-parse", "--verify", "--quiet", f"{revision}:{package_root}"
    )
    if result.returncode != 0:
        return None
    tree = result.stdout.strip()
    return tree if _object_type(repo, tree) == "tree" else None


def _tree_drift(
    repo: Path, from_tree: str, to_tree: str, package_root: str
) -> list[str]:
    """Repository paths under package_root whose content differs between two trees.

    ``git diff-tree`` on two bare trees reports paths relative to those trees,
    i.e. relative to the package root. They are re-anchored at package_root so
    messages name real repository paths and `_non_lockfile_changes` sees the
    same shape it always has.
    """

    result = _git(
        repo, "diff-tree", "-r", "--name-only", "--no-renames", from_tree, to_tree
    )
    if result.returncode != 0:
        raise LaneEvidenceError(
            f"cannot diff package_tree {from_tree} against {to_tree}: "
            f"{result.stderr.strip()}"
        )
    root = PurePosixPath(package_root)
    return [str(root / path) for path in result.stdout.splitlines() if path]


def _check_lane_tree_evidence(
    repo: Path,
    *,
    package_root: str,
    package_tree: str,
    verified_revision: str,
    head: str = "HEAD",
) -> None:
    """Fail closed unless ``package_tree`` is a tree ``head`` still carries.

    Raises LaneEvidenceError with a message that says what to change. See the
    table above for the rule; lockfile-only drift is admitted, everything else
    must be re-verified and repinned in the PR that changes the lane.
    """

    if _object_type(repo, package_tree) != "tree":
        raise LaneEvidenceError(
            f"package_tree {package_tree} is not a git tree object in this "
            f"repository; set it to `git rev-parse <verified commit>:{package_root}`"
        )

    revision_type = _object_type(repo, verified_revision)
    if revision_type is not None:
        if revision_type != "commit":
            raise LaneEvidenceError(
                f"verified_revision {verified_revision} is a {revision_type}, "
                "not a commit"
            )
        revision_tree = _tree_at(repo, verified_revision, package_root)
        if revision_tree != package_tree:
            raise LaneEvidenceError(
                f"verified_revision {verified_revision} has package tree "
                f"{revision_tree} at {package_root}, but package_tree is "
                f"{package_tree}; pin the commit whose tree was verified"
            )
    # An unresolvable or non-ancestor verified_revision is the normal state
    # after a squash merge; the content-addressed tree carries the evidence.

    head_tree = _tree_at(repo, head, package_root)
    if head_tree is None:
        raise LaneEvidenceError(f"{package_root} is not a directory at {head}")
    if head_tree == package_tree:
        return
    drift = _tree_drift(repo, package_tree, head_tree, package_root)
    if not drift:
        raise LaneEvidenceError(
            f"{package_root} tree {head_tree} differs from package_tree "
            f"{package_tree} but git reports no changed paths"
        )
    unverified = _non_lockfile_changes(drift)
    if unverified:
        raise LaneEvidenceError(
            f"{package_root} changed outside dependency lockfiles since its "
            f"verified package_tree {package_tree}: {', '.join(unverified)}. "
            "Re-run this lane's tests; once they pass, in the same PR that "
            f"changes the lane set package_tree to {head_tree} "
            f"(`git rev-parse HEAD:{package_root}`) and set verified_revision, "
            "together with the owner item's implementation_revision in "
            ".claude/package-parity-loop-state.json, to the commit you verified"
        )


def _non_lockfile_changes(paths: list[str]) -> list[str]:
    """The changed paths that are not dependency lockfiles (empty = lock-only)."""

    return [
        path for path in paths if PurePosixPath(path).name not in DEPENDENCY_LOCK_FILES
    ]


class _ScratchRepo:
    """A throwaway git repository that replays a squash-merged lane change.

    It runs with global and system git config disabled so a developer's
    signing, hooks, or default-branch settings cannot change the outcome.
    """

    def __enter__(self) -> Self:
        self._directory = tempfile.TemporaryDirectory()
        self.root = Path(self._directory.name)
        self._env = {
            **os.environ,
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_SYSTEM": os.devnull,
            "GIT_AUTHOR_NAME": "fixture",
            "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
            "GIT_COMMITTER_NAME": "fixture",
            "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
        }
        self._run("init", "-q", "-b", "main")
        self.base = self.commit(
            {
                "pkg/BUILD": "build\n",
                "pkg/Cargo.lock": "lock\n",
                "pkg/deep/go.sum": "sum\n",
                "outside.txt": "not in the lane\n",
            }
        )
        return self

    def __exit__(self, *_exc: object) -> None:
        self._directory.cleanup()

    def _run(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args],
            cwd=self.root,
            env=self._env,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()

    def commit(self, files: dict[str, str]) -> str:
        for relative_path, content in files.items():
            path = self.root / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        self._run("add", "-A")
        self._run("commit", "-q", "--no-verify", "-m", "change")
        return self._run("rev-parse", "HEAD")

    def tree(self, revision: str) -> str:
        return self._run("rev-parse", f"{revision}:pkg")

    def lane_change_then_squash(self) -> tuple[str, str]:
        """Change the lane on a branch, then squash it onto main.

        Returns the branch commit that was verified (absent from main's
        history) and its package tree (identical on main after the squash).
        """

        self._run("checkout", "-q", "-b", "feature")
        verified = self.commit({"pkg/BUILD": "build with a new linker flag\n"})
        self.commit({"registry.json": "pins the commit above\n"})
        self._run("checkout", "-q", "main")
        self._run("merge", "-q", "--squash", "feature")
        self._run("commit", "-q", "--no-verify", "-m", "squash")
        return verified, self.tree(verified)


class BarcodeLayoutFixtureTests(unittest.TestCase):
    document: ClassVar[dict[str, Any]]
    by_id: ClassVar[dict[str, dict[str, Any]]]

    @classmethod
    def setUpClass(cls) -> None:
        cls.schema_encoded = _read_bounded(FIXTURE_ROOT / "schema.json")
        cls.document_encoded = _read_bounded(FIXTURE_ROOT / "cases.json")
        cls.document = _load(cls.schema_encoded, cls.document_encoded)
        cls.by_id = {case["id"]: case for case in cls.document["cases"]}

    def test_envelope_limits_errors_and_operations_are_closed(self) -> None:
        self.assertEqual(
            {path.name for path in FIXTURE_ROOT.iterdir() if path.is_file()},
            {
                "CHANGELOG.md",
                "README.md",
                "cases.json",
                "generate_cases.py",
                "schema.json",
                "targets.json",
                "targets.schema.json",
            },
        )
        self.assertEqual(len(self.by_id), len(self.document["cases"]))
        self.assertEqual(
            self.document["limits"],
            {
                "max_pattern_scalars": 65_567,
                "max_runs": 40_979,
                "max_content_modules": 65_567,
                "max_quiet_zone_modules": 4_096,
                "max_symbols": 40_979,
                "max_label_scalars": 4_096,
                "max_metadata_entries": 64,
                "max_metadata_key_scalars": 128,
                "max_metadata_value_scalars": 4_096,
                "max_metadata_utf8_bytes": 65_536,
                "max_module_width": 8_192,
                "max_bar_height": 8_192,
                "max_color_scalars": 128,
                "max_cases": 64,
                "max_fixture_bytes": 131_072,
                "max_fixture_depth": 8,
            },
        )
        self.assertEqual(
            {case["operation"] for case in self.document["cases"]},
            {"expand-binary", "expand-width", "compute-layout", "project-scene"},
        )
        exercised = {
            case["expected"]["error"]
            for case in self.document["cases"]
            if "error" in case["expected"]
        }
        self.assertEqual(exercised, set(self.document["error_ids"]))

    def test_run_layout_and_scene_algebra_is_self_consistent(self) -> None:
        for case in self.document["cases"]:
            expected = case["expected"]
            if "runs" in expected:
                runs = expected["runs"]
                self.assertTrue(all(run["modules"] > 0 for run in runs), case["id"])
                for left, right in pairwise(runs):
                    self.assertNotEqual(left["color"], right["color"], case["id"])
                self.assertLessEqual(
                    sum(run["modules"] for run in runs),
                    self.document["limits"]["max_content_modules"],
                    case["id"],
                )
            if "layout" in expected:
                layout = expected["layout"]
                self.assertEqual(
                    layout["totalModules"],
                    layout["leftQuietZoneModules"]
                    + layout["contentModules"]
                    + layout["rightQuietZoneModules"],
                    case["id"],
                )
                for symbol in layout["symbolLayouts"]:
                    self.assertLess(
                        symbol["startModule"], symbol["endModule"], case["id"]
                    )
                    self.assertLessEqual(
                        symbol["endModule"], layout["contentModules"], case["id"]
                    )
            if "scene" in expected:
                scene = expected["scene"]
                metadata = scene["metadata"]
                self.assertEqual(
                    scene["width"], int(metadata["sceneWidthPx"]), case["id"]
                )
                self.assertEqual(
                    scene["height"], int(metadata["sceneHeightPx"]), case["id"]
                )
                for rectangle in scene["rectangles"]:
                    rect_metadata = rectangle["metadata"]
                    self.assertEqual(
                        rectangle["width"],
                        (
                            int(rect_metadata["moduleEnd"])
                            - int(rect_metadata["moduleStart"])
                        )
                        * int(metadata["moduleWidthPx"]),
                        case["id"],
                    )

    def test_large_run_digest_uses_the_normative_canonical_encoding(self) -> None:
        case = self.by_id["layout-v1-width-run-limit"]
        value = case["input"]
        count = value["repeat"]["count"]
        runs = [
            {
                "color": "bar" if index % 2 == 0 else "space",
                "modules": 1,
                "sourceLabel": value["sourceLabel"],
                "sourceIndex": value["sourceIndex"],
                "role": value["role"],
            }
            for index in range(count)
        ]
        encoded = json.dumps(
            runs, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode()
        digest = case["expected"]["runDigest"]
        self.assertEqual(digest["runCount"], count)
        self.assertEqual(digest["contentModules"], count)
        self.assertEqual(digest["firstRun"], runs[0])
        self.assertEqual(digest["lastRun"], runs[-1])
        self.assertEqual(digest["runsSha256"], hashlib.sha256(encoded).hexdigest())
        self.assertEqual(
            digest["runsSha256"],
            "6b5abd84e83b8cca52df0750f6a8ad78535e0a3056e553cea8faf00f217f4383",
        )

    def test_representative_policy_vectors_are_pinned(self) -> None:
        binary = self.by_id["layout-v1-binary-basic"]["expected"]["runs"]
        self.assertEqual(
            [(run["color"], run["modules"]) for run in binary],
            [("bar", 2), ("space", 1), ("bar", 1), ("space", 2)],
        )
        gap = self.by_id["layout-v1-layout-gap-span"]["expected"]["layout"]
        self.assertEqual(gap["symbolLayouts"][0]["endModule"], 4)
        scene = self.by_id["layout-v1-scene-default"]["expected"]["scene"]
        self.assertEqual((scene["width"], scene["height"]), (92, 120))
        self.assertEqual(scene["metadata"]["totalModules"], "23")
        metadata_limit = self.by_id["layout-v1-scene-metadata-entry-limit"]["expected"][
            "scene"
        ]["metadata"]
        self.assertEqual(len(metadata_limit), 74)

    def test_target_registry_lifecycle_and_repository_evidence(self) -> None:
        target_schema = _read_bounded(FIXTURE_ROOT / "targets.schema.json")
        targets = _load(target_schema, _read_bounded(FIXTURE_ROOT / "targets.json"))
        entries = targets["targets"]
        self.assertEqual(
            {entry["language"] for entry in entries},
            {
                "csharp",
                "dart",
                "elixir",
                "fsharp",
                "go",
                "haskell",
                "java",
                "kotlin",
                "lua",
                "perl",
                "python",
                "ruby",
                "rust",
                "swift",
                "typescript",
            },
        )
        self.assertEqual(len(entries), 15)
        corpus_digest = hashlib.sha256(self.document_encoded).hexdigest()
        self.assertEqual(
            corpus_digest,
            "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388",
        )
        capability_schema = json.loads(
            (
                REPO_ROOT / "code/specs/schemas/required_capabilities.schema.json"
            ).read_text(encoding="utf-8")
        )
        state = json.loads(
            (REPO_ROOT / ".claude/package-parity-loop-state.json").read_text(
                encoding="utf-8"
            )
        )
        package_roots = {
            "csharp": "code/packages/csharp/barcode-layout-1d",
            "dart": "code/packages/dart/barcode-layout-1d",
            "elixir": "code/packages/elixir/barcode_layout_1d",
            "fsharp": "code/packages/fsharp/barcode-layout-1d",
            "go": "code/packages/go/barcode-layout-1d",
            "haskell": "code/packages/haskell/barcode-layout-1d",
            "java": "code/packages/java/barcode-layout-1d",
            "kotlin": "code/packages/kotlin/barcode-layout-1d",
            "lua": "code/packages/lua/barcode_layout_1d",
            "perl": "code/packages/perl/barcode-layout-1d",
            "python": "code/packages/python/barcode-layout-1d",
            "ruby": "code/packages/ruby/barcode_layout_1d",
            "rust": "code/packages/rust/barcode-layout-1d",
            "swift": "code/packages/swift/BarcodeLayout1D",
            "typescript": "code/packages/typescript/barcode-layout-1d",
        }
        owner_ids = {
            "elixir": "barcode-layout-1d-elixir-lua-v1-conformance",
            "dart": "barcode-layout-1d-java-kotlin-dart-lane-parity",
            "java": "barcode-layout-1d-java-kotlin-dart-lane-parity",
            "kotlin": "barcode-layout-1d-java-kotlin-dart-lane-parity",
            "haskell": "haskell-itf-stack-windows-build-front-coverage",
            "lua": "barcode-layout-1d-elixir-lua-v1-conformance",
            "ruby": "barcode-layout-1d-ruby-swift-v1-conformance",
            "swift": "barcode-layout-1d-ruby-swift-v1-conformance",
        }
        for entry in entries:
            package_root = REPO_ROOT / entry["package_root"]
            self.assertEqual(
                entry["package_root"],
                package_roots[entry["language"]],
            )
            native_test_path = _target_package_path(entry, "native_test_path")
            if entry["status"] == "pending-adoption":
                self.assertTrue(entry["known_divergences"])
                self.assertIn("planned_conformance_test_path", entry)
                _target_package_path(entry, "planned_conformance_test_path")
                if entry["capability_manifest"] is not None:
                    _target_package_path(entry, "capability_manifest")
                continue

            self.assertEqual(entry["status"], "conformant")
            self.assertTrue(package_root.is_dir())
            self.assertTrue(native_test_path.is_file())
            self.assertEqual(entry["known_divergences"], [])
            self.assertEqual(entry["corpus_sha256"], corpus_digest)
            conformance_path = _target_package_path(entry, "conformance_test_path")
            self.assertTrue(conformance_path.is_file())
            capability_path = _target_package_path(entry, "capability_manifest")
            self.assertTrue(capability_path.is_file())
            capability = json.loads(capability_path.read_text(encoding="utf-8"))
            Draft202012Validator(capability_schema).validate(capability)
            self.assertEqual(
                capability["package"], f"{entry['language']}/barcode-layout-1d"
            )
            self.assertEqual(capability["capabilities"], [])
            evidence = entry["zero_authority_evidence"]
            self.assertEqual(evidence["test_path"], entry["conformance_test_path"])
            self.assertEqual(
                set(evidence["assertions"]),
                {
                    "text-value-fails-before-native-resolution",
                    "text-enabled-fails-before-native-resolution",
                },
            )
            owner_id = owner_ids.get(
                entry["language"], "barcode-layout-existing-lanes-v1-conformance"
            )
            adoption_item = next(
                item for item in state["items"] if item["id"] == owner_id
            )
            self.assertIn(adoption_item["status"], {"pr-open", "merged"})
            self.assertEqual(entry["adoption_pr"], adoption_item["pr_number"])
            self.assertEqual(
                entry["verified_revision"], adoption_item["implementation_revision"]
            )
            # Content-addressed evidence: see _check_lane_tree_evidence. No
            # ancestry requirement, because a squash merge discards every
            # commit the lane-changing PR could have pinned.
            try:
                _check_lane_tree_evidence(
                    REPO_ROOT,
                    package_root=entry["package_root"],
                    package_tree=entry["package_tree"],
                    verified_revision=entry["verified_revision"],
                )
            except LaneEvidenceError as error:
                self.fail(f"{entry['language']} lane: {error}")
            for relative_path in (
                entry["conformance_test_path"],
                entry["capability_manifest"],
            ):
                subprocess.run(
                    ["git", "cat-file", "-e", f"HEAD:{relative_path}"],
                    cwd=REPO_ROOT,
                    check=True,
                    capture_output=True,
                )

    def test_target_registry_rejects_untruthful_lifecycle_evidence(self) -> None:
        target_schema = _read_bounded(FIXTURE_ROOT / "targets.schema.json")
        targets = _load(target_schema, _read_bounded(FIXTURE_ROOT / "targets.json"))

        pending = json.loads(json.dumps(targets))
        pending_target = pending["targets"][0]
        pending_target["status"] = "pending-adoption"
        pending_target["planned_conformance_test_path"] = pending_target.pop(
            "conformance_test_path"
        )
        pending_target["known_divergences"] = ["not adopted"]
        for field in (
            "verified_revision",
            "package_tree",
            "corpus_sha256",
            "adoption_pr",
            "zero_authority_evidence",
        ):
            pending_target.pop(field)
        _load(target_schema, json.dumps(pending).encode())

        pending_with_evidence = json.loads(json.dumps(pending))
        pending_with_evidence["targets"][0]["corpus_sha256"] = (
            "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388"
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(target_schema, json.dumps(pending_with_evidence).encode())

        promoted = json.loads(json.dumps(targets))
        _load(target_schema, json.dumps(promoted).encode())

        mutations = []
        wrong_digest = json.loads(json.dumps(promoted))
        wrong_digest["targets"][0]["corpus_sha256"] = "0" * 64
        mutations.append(wrong_digest)
        divergence = json.loads(json.dumps(promoted))
        divergence["targets"][0]["known_divergences"] = ["still divergent"]
        mutations.append(divergence)
        wrong_root = json.loads(json.dumps(promoted))
        wrong_root["targets"][0]["package_root"] = (
            "code/packages/fsharp/barcode-layout-1d"
        )
        mutations.append(wrong_root)
        free_text = json.loads(json.dumps(promoted))
        free_text["targets"][0]["zero_authority_evidence"] = "trust me"
        mutations.append(free_text)
        planned = json.loads(json.dumps(promoted))
        planned["targets"][0]["planned_conformance_test_path"] = planned["targets"][0][
            "conformance_test_path"
        ]
        mutations.append(planned)
        for mutation in mutations:
            with (
                self.subTest(mutation=mutation["targets"][0]),
                self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"),
            ):
                _load(target_schema, json.dumps(mutation).encode())

        cross_package_path = json.loads(json.dumps(promoted["targets"][0]))
        cross_package_path["conformance_test_path"] = (
            "code/packages/fsharp/barcode-layout-1d/test/Spec.fs"
        )
        with self.assertRaisesRegex(FixtureLoadError, "target-path-outside-package"):
            _target_package_path(cross_package_path, "conformance_test_path")

        windows_traversal = json.loads(json.dumps(promoted["targets"][0]))
        windows_traversal["conformance_test_path"] = (
            f"{windows_traversal['package_root']}/test\\..\\..\\..\\..\\CHANGELOG.md"
        )
        with self.assertRaisesRegex(FixtureLoadError, "target-path-outside-package"):
            _target_package_path(windows_traversal, "conformance_test_path")

        pending_traversal = json.loads(json.dumps(pending_target))
        pending_traversal["native_test_path"] = (
            f"{pending_traversal['package_root']}/../../outside_test.py"
        )
        with self.assertRaisesRegex(FixtureLoadError, "target-path-outside-package"):
            _target_package_path(pending_traversal, "native_test_path")

    def test_post_verification_drift_admits_only_dependency_lockfiles(self) -> None:
        root = "code/packages/typescript/barcode-layout-1d"
        self.assertEqual(_non_lockfile_changes([]), [])
        self.assertEqual(
            _non_lockfile_changes(
                [f"{root}/package-lock.json", "code/packages/rust/x/Cargo.lock"]
            ),
            [],
        )
        for path in (
            f"{root}/package.json",
            f"{root}/src/index.ts",
            f"{root}/tests/conformance.test.ts",
            f"{root}/required_capabilities.json",
            f"{root}/package-lock.json.orig",
            f"{root}/not-a-package-lock.json",
        ):
            with self.subTest(path=path):
                self.assertEqual(
                    _non_lockfile_changes([f"{root}/package-lock.json", path]),
                    [path],
                )

    def test_tree_evidence_survives_a_squash_merge(self) -> None:
        with _ScratchRepo() as repo:
            verified, tree = repo.lane_change_then_squash()
            # The scenario is real: the verified commit is not on main.
            self.assertNotEqual(
                _git(
                    repo.root, "merge-base", "--is-ancestor", verified, "HEAD"
                ).returncode,
                0,
            )
            self.assertEqual(repo.tree("HEAD"), tree)
            # Discarded by the squash but still in the object store.
            _check_lane_tree_evidence(
                repo.root,
                package_root="pkg",
                package_tree=tree,
                verified_revision=verified,
            )
            # Absent entirely, as in a fresh clone of main.
            _check_lane_tree_evidence(
                repo.root,
                package_root="pkg",
                package_tree=tree,
                verified_revision="1" * 40,
            )

    def test_tree_evidence_rejects_drift_outside_lockfiles(self) -> None:
        with _ScratchRepo() as repo:
            verified, tree = repo.lane_change_then_squash()
            repo.commit({"pkg/BUILD": "changed again\n"})
            head_tree = repo.tree("HEAD")
            with self.assertRaisesRegex(
                LaneEvidenceError,
                "changed outside dependency lockfiles.*pkg/BUILD.*"
                "Re-run this lane's tests.*in the same PR that changes the lane "
                f"set package_tree to {head_tree}",
            ):
                _check_lane_tree_evidence(
                    repo.root,
                    package_root="pkg",
                    package_tree=tree,
                    verified_revision=verified,
                )
            # A lockfile change alongside does not dilute the verdict.
            repo.commit({"pkg/Cargo.lock": "bumped\n"})
            with self.assertRaisesRegex(LaneEvidenceError, "pkg/BUILD"):
                _check_lane_tree_evidence(
                    repo.root,
                    package_root="pkg",
                    package_tree=tree,
                    verified_revision="1" * 40,
                )

    def test_tree_evidence_admits_lockfile_only_drift(self) -> None:
        with _ScratchRepo() as repo:
            verified, tree = repo.lane_change_then_squash()
            repo.commit({"pkg/Cargo.lock": "bumped\n", "pkg/deep/go.sum": "x\n"})
            self.assertEqual(
                _tree_drift(repo.root, tree, repo.tree("HEAD"), "pkg"),
                ["pkg/Cargo.lock", "pkg/deep/go.sum"],
            )
            _check_lane_tree_evidence(
                repo.root,
                package_root="pkg",
                package_tree=tree,
                verified_revision=verified,
            )

    def test_tree_evidence_requires_a_tree_object(self) -> None:
        with _ScratchRepo() as repo:
            verified, _tree = repo.lane_change_then_squash()
            blob = _git(repo.root, "rev-parse", "HEAD:pkg/BUILD").stdout.strip()
            for not_a_tree in (verified, blob, "2" * 40):
                with (
                    self.subTest(package_tree=not_a_tree),
                    self.assertRaisesRegex(
                        LaneEvidenceError, "is not a git tree object"
                    ),
                ):
                    _check_lane_tree_evidence(
                        repo.root,
                        package_root="pkg",
                        package_tree=not_a_tree,
                        verified_revision="1" * 40,
                    )

    def test_resolvable_verified_revision_must_match_package_tree(self) -> None:
        with _ScratchRepo() as repo:
            _verified, tree = repo.lane_change_then_squash()
            with self.assertRaisesRegex(
                LaneEvidenceError, "pin the commit whose tree was verified"
            ):
                _check_lane_tree_evidence(
                    repo.root,
                    package_root="pkg",
                    package_tree=tree,
                    verified_revision=repo.base,
                )
            with self.assertRaisesRegex(LaneEvidenceError, "is a tree, not a commit"):
                _check_lane_tree_evidence(
                    repo.root,
                    package_root="pkg",
                    package_tree=tree,
                    verified_revision=tree,
                )

    def test_generator_is_byte_for_byte_clean(self) -> None:
        namespace = runpy.run_path(str(FIXTURE_ROOT / "generate_cases.py"))
        self.assertEqual(namespace["render_document"](), self.document_encoded)

    def test_bounded_loader_rejects_hostile_json_and_schema(self) -> None:
        with self.assertRaisesRegex(FixtureLoadError, "fixture-size-limit"):
            _load(self.schema_encoded, b" " * (MAX_FIXTURE_BYTES + 1))
        with self.assertRaisesRegex(FixtureLoadError, "fixture-depth-limit"):
            _load(self.schema_encoded, (b'{"x":' * 10) + b"0" + (b"}" * 10))
        duplicate = self.document_encoded.replace(
            b'"schema_version": 1,', b'"schema_version": 1, "schema_version": 1,', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-json"):
            _load(self.schema_encoded, duplicate)
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-json"):
            _load(self.schema_encoded, b'{"value": NaN}')
        surrogate = self.document_encoded.replace(
            b'"barcode-layout-1d-v1"', b'"\\ud800"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-scalar"):
            _load(self.schema_encoded, surrogate)
        external = self.schema_encoded.replace(
            b'"#/$defs/case"', b'"https://example.invalid/case.json"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(external, self.document_encoded)

    def test_schema_couples_operations_inputs_and_outputs(self) -> None:
        missing_input = json.loads(self.document_encoded)
        missing_input["cases"][0]["input"] = {}
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(self.schema_encoded, json.dumps(missing_input).encode())

        wrong_output = json.loads(self.document_encoded)
        wrong_output["cases"][0]["expected"] = self.by_id["layout-v1-scene-default"][
            "expected"
        ]
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(self.schema_encoded, json.dumps(wrong_output).encode())

        invalid_success_run = json.loads(self.document_encoded)
        invalid_success_run["cases"][0]["expected"]["runs"][0]["modules"] = 0
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(self.schema_encoded, json.dumps(invalid_success_run).encode())


if __name__ == "__main__":
    unittest.main()
