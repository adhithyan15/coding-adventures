from __future__ import annotations

import copy
import hashlib
import io
import json
import os
import struct
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_conformance as bootstrap
import build_tool_conformance_authority as authority

FIXTURE_ROOT = bootstrap.DEFAULT_FIXTURE_ROOT
COMMIT_OID = "a" * 40
TREE_OID = "b" * 40
EMPTY_DIGEST = hashlib.sha256(b"").hexdigest()

REPOSITORY_COMPONENTS = {
    "authority_bundle_schema": (
        "code/specs/fixtures/build-tool-v1/execution-authority.schema.json"
    ),
    "execution_policy_schema": (
        "code/specs/fixtures/build-tool-v1/execution-policy.schema.json"
    ),
    "execution_policy": "code/specs/fixtures/build-tool-v1/execution-policy.json",
    "linux_backend_identity_schema": (
        "code/specs/fixtures/build-tool-v1/linux-oci-backend.schema.json"
    ),
    "bootstrap_runner": "code/scripts/build_tool_conformance.py",
    "authority_verifier": "code/scripts/build_tool_conformance_authority.py",
    "linux_preflight_backend": "code/scripts/build_tool_conformance_linux_oci.py",
}


def backend_identity() -> dict[str, object]:
    manifest = "4" * 64
    return {
        "schema_version": 1,
        "backend_kind": "linux_oci",
        "platform": "linux",
        "architecture": "amd64",
        "runtime": {
            "implementation": "podman",
            "path": "/usr/bin/podman",
            "version": "5.8.3",
            "sha256": "1" * 64,
        },
        "oci_runtime": {
            "implementation": "crun",
            "path": "/usr/bin/crun",
            "sha256": "2" * 64,
        },
        "conmon": {
            "implementation": "conmon",
            "path": "/usr/bin/conmon",
            "sha256": "8" * 64,
        },
        "image": {
            "reference": f"localhost/build-tool@sha256:{manifest}",
            "manifest_sha256": manifest,
            "config_sha256": "5" * 64,
            "os": "linux",
            "architecture": "amd64",
        },
        "seccomp_profile_sha256": "6" * 64,
        "shim": {
            "path": "/opt/conformance/shim",
            "sha256": "7" * 64,
        },
        "probe": {
            "path": "/opt/conformance/probe",
            "sha256": "8" * 64,
        },
    }


def component_path(
    repository_root: Path,
    bundle_path: Path,
    role: str,
    record: dict[str, object],
) -> Path:
    if record["provenance"] == "repository":
        return repository_root / str(record["path"])
    return bundle_path.parent / str(record["path"])


def write_authority_fixture(root: Path) -> dict[str, object]:
    repository_root = root / "repository"
    bundle_root = root / "authority"
    bundle_root.mkdir(parents=True)

    fixture_sources = {
        "authority_bundle_schema": FIXTURE_ROOT / "execution-authority.schema.json",
        "execution_policy_schema": FIXTURE_ROOT / "execution-policy.schema.json",
        "execution_policy": FIXTURE_ROOT / "execution-policy.json",
        "linux_backend_identity_schema": FIXTURE_ROOT / "linux-oci-backend.schema.json",
    }
    raw_components: dict[str, bytes] = {
        role: source.read_bytes() for role, source in fixture_sources.items()
    }
    raw_components.update(
        {
            "bootstrap_runner": b"# process-free bootstrap\n",
            "authority_verifier": b"# process-free authority verifier\n",
            "linux_preflight_backend": b"# process-owning Linux preflight\n",
        }
    )
    for role, relative_path in REPOSITORY_COMPONENTS.items():
        target = repository_root / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw_components[role])

    identity_path = bundle_root / "linux-oci-backend.json"
    identity_path.write_bytes(
        json.dumps(
            backend_identity(),
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        + b"\n"
    )
    bundle_path = bundle_root / "authority.json"
    bundle: dict[str, object] = {
        "schema_version": 1,
        "purpose": "build-tool-trusted-authority",
        "authorization_scope": "linux_capability_preflight_v1",
        "repository": "github.com/adhithyan15/coding-adventures",
        "conformance_revision": "v1",
        "platform": "linux",
        "architecture": "amd64",
        "source": {
            "git_object_format": "sha1",
            "commit_oid": COMMIT_OID,
            "tree_oid": TREE_OID,
        },
        "components": {
            **{
                role: {
                    "provenance": "repository",
                    "path": relative_path,
                    "byte_length": len(raw_components[role]),
                    "sha256": hashlib.sha256(raw_components[role]).hexdigest(),
                }
                for role, relative_path in REPOSITORY_COMPONENTS.items()
            },
            "linux_backend_identity": {
                "provenance": "bundle",
                "path": "linux-oci-backend.json",
                "byte_length": len(identity_path.read_bytes()),
                "sha256": hashlib.sha256(identity_path.read_bytes()).hexdigest(),
            },
        },
    }
    environment: dict[str, object] = {
        "repository_root": repository_root,
        "bundle_path": bundle_path,
        "bundle": bundle,
    }
    rewrite_bundle(environment)
    return environment


def rewrite_bundle(environment: dict[str, object]) -> str:
    bundle_path = environment["bundle_path"]
    bundle = environment["bundle"]
    assert isinstance(bundle_path, Path)
    assert isinstance(bundle, dict)
    raw = (
        json.dumps(bundle, sort_keys=True, separators=(",", ":")).encode("utf-8")
        + b"\n"
    )
    bundle_path.write_bytes(raw)
    digest = authority.authority_bundle_sha256(raw)
    environment["approved_digest"] = digest
    return digest


def refresh_component(
    environment: dict[str, object],
    role: str,
    raw: bytes,
) -> str:
    repository_root = environment["repository_root"]
    bundle_path = environment["bundle_path"]
    bundle = environment["bundle"]
    assert isinstance(repository_root, Path)
    assert isinstance(bundle_path, Path)
    assert isinstance(bundle, dict)
    components = bundle["components"]
    assert isinstance(components, dict)
    record = components[role]
    assert isinstance(record, dict)
    component_path(repository_root, bundle_path, role, record).write_bytes(raw)
    record["byte_length"] = len(raw)
    record["sha256"] = hashlib.sha256(raw).hexdigest()
    return rewrite_bundle(environment)


def authorize(environment: dict[str, object]) -> authority.PreflightAuthority:
    return authority.authorize_preflight(
        environment["bundle_path"],  # type: ignore[arg-type]
        approved_digest=environment["approved_digest"],  # type: ignore[arg-type]
        expected_commit_oid=COMMIT_OID,
        expected_tree_oid=TREE_OID,
        repository_root=environment["repository_root"],  # type: ignore[arg-type]
    )


class AuthoritySchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = bootstrap.load_document(
            FIXTURE_ROOT / "execution-authority.schema.json"
        )

    def test_closed_preflight_bundle_validates(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            self.assertEqual(
                bootstrap._schema_errors(environment["bundle"], self.schema),
                [],
            )

    def test_self_digest_future_scope_revision_and_role_swap_are_rejected(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            bundle = environment["bundle"]
            assert isinstance(bundle, dict)

            self_digest = copy.deepcopy(bundle)
            self_digest["authority_sha256"] = "0" * 64
            self.assertTrue(bootstrap._schema_errors(self_digest, self.schema))

            future_scope = copy.deepcopy(bundle)
            future_scope["authorization_scope"] = "trusted_execution_v1"
            self.assertTrue(bootstrap._schema_errors(future_scope, self.schema))

            future_revision = copy.deepcopy(bundle)
            future_revision["conformance_revision"] = "v2"
            self.assertTrue(bootstrap._schema_errors(future_revision, self.schema))

            swapped = copy.deepcopy(bundle)
            components = swapped["components"]
            assert isinstance(components, dict)
            first = components["bootstrap_runner"]
            second = components["authority_verifier"]
            assert isinstance(first, dict)
            assert isinstance(second, dict)
            first["path"], second["path"] = second["path"], first["path"]
            self.assertTrue(bootstrap._schema_errors(swapped, self.schema))


class AuthorityVerifierTests(unittest.TestCase):
    def test_digest_has_exact_domain_length_and_raw_byte_framing(self) -> None:
        raw = b'{"schema_version":1}\n'
        manual = hashlib.sha256(
            authority.AUTHORITY_DOMAIN + struct.pack(">Q", len(raw)) + raw
        ).hexdigest()
        self.assertEqual(authority.authority_bundle_sha256(raw), manual)
        for changed in (
            b'{ "schema_version": 1 }\n',
            b'{"schema_version":1}',
            b'{"schema_version":1}\r\n',
        ):
            self.assertNotEqual(authority.authority_bundle_sha256(changed), manual)

    def test_invalid_approval_fails_before_any_read(self) -> None:
        with (
            mock.patch.object(authority, "_read_bound_regular") as reader,
            self.assertRaises(bootstrap.ConformanceError) as raised,
        ):
            authority.authorize_preflight(
                Path("missing.json"),
                approved_digest="not-a-digest",
                expected_commit_oid=COMMIT_OID,
                expected_tree_oid=TREE_OID,
                repository_root=Path("missing"),
            )
        self.assertEqual(raised.exception.code, "AUTHORITY_DIGEST_INVALID")
        reader.assert_not_called()

    def test_invalid_source_identities_fail_before_any_read(self) -> None:
        for commit, tree in (
            ("A" * 40, TREE_OID),
            ("a" * 39, TREE_OID),
            (COMMIT_OID, "not-a-tree"),
        ):
            with (
                self.subTest(commit=commit, tree=tree),
                mock.patch.object(authority, "_read_bound_regular") as reader,
                self.assertRaises(bootstrap.ConformanceError) as raised,
            ):
                authority.authorize_preflight(
                    Path("missing.json"),
                    approved_digest="0" * 64,
                    expected_commit_oid=commit,
                    expected_tree_oid=tree,
                    repository_root=Path("missing"),
                )
            self.assertEqual(raised.exception.code, "AUTHORITY_SOURCE_ID_INVALID")
            reader.assert_not_called()

    def test_digest_mismatch_fails_before_parse_or_component_read(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            with (
                mock.patch.object(bootstrap, "strict_load_bytes") as parser,
                mock.patch.object(authority, "_read_component_bytes") as components,
                self.assertRaises(bootstrap.ConformanceError) as raised,
            ):
                authority.authorize_preflight(
                    environment["bundle_path"],  # type: ignore[arg-type]
                    approved_digest="0" * 64,
                    expected_commit_oid=COMMIT_OID,
                    expected_tree_oid=TREE_OID,
                    repository_root=environment["repository_root"],  # type: ignore[arg-type]
                )
            self.assertEqual(raised.exception.code, "AUTHORITY_DIGEST_MISMATCH")
            parser.assert_not_called()
            components.assert_not_called()

    def test_source_mismatch_fails_before_component_read(self) -> None:
        for commit, tree in (("c" * 40, TREE_OID), (COMMIT_OID, "d" * 40)):
            with self.subTest(commit=commit, tree=tree), tempfile.TemporaryDirectory() as directory:
                environment = write_authority_fixture(Path(directory))
                with (
                    mock.patch.object(
                        authority,
                        "_read_component_bytes",
                    ) as components,
                    self.assertRaises(bootstrap.ConformanceError) as raised,
                ):
                    authority.authorize_preflight(
                        environment["bundle_path"],  # type: ignore[arg-type]
                        approved_digest=environment["approved_digest"],  # type: ignore[arg-type]
                        expected_commit_oid=commit,
                        expected_tree_oid=tree,
                        repository_root=environment["repository_root"],  # type: ignore[arg-type]
                    )
                self.assertEqual(
                    raised.exception.code,
                    "AUTHORITY_SOURCE_MISMATCH",
                )
                components.assert_not_called()

    def test_valid_bundle_authorizes_only_disabled_preflight_profile(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            approved = authorize(environment)
        self.assertEqual(approved.bundle_digest, environment["approved_digest"])
        self.assertEqual(approved.bundle["authorization_scope"], "linux_capability_preflight_v1")
        self.assertFalse(approved.policy["enabled"])
        self.assertEqual(approved.policy["execution_corpus_sha256"], EMPTY_DIGEST)
        self.assertEqual(approved.policy["adapters"], [])
        linux = next(
            item for item in approved.policy["backends"] if item["platform"] == "linux"
        )
        self.assertEqual(linux["status"], "unavailable")
        self.assertIsNone(linux["identity_sha256"])
        self.assertEqual(approved.identity["backend_kind"], "linux_oci")
        self.assertFalse(hasattr(authority, "subprocess"))

    def test_every_component_is_bound_by_exact_length_and_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            bundle = environment["bundle"]
            repository_root = environment["repository_root"]
            bundle_path = environment["bundle_path"]
            assert isinstance(bundle, dict)
            assert isinstance(repository_root, Path)
            assert isinstance(bundle_path, Path)
            components = bundle["components"]
            assert isinstance(components, dict)
            for role, record in components.items():
                assert isinstance(record, dict)
                path = component_path(repository_root, bundle_path, role, record)
                original = path.read_bytes()
                with self.subTest(role=role, mutation="digest"):
                    path.write_bytes(original + b"\n")
                    with self.assertRaises(bootstrap.ConformanceError) as raised:
                        authorize(environment)
                    self.assertEqual(
                        raised.exception.code,
                        "AUTHORITY_COMPONENT_LENGTH_MISMATCH",
                    )
                    path.write_bytes(original)
                with self.subTest(role=role, mutation="declared-length"):
                    record["byte_length"] = len(original) + 1
                    rewrite_bundle(environment)
                    with self.assertRaises(bootstrap.ConformanceError) as raised:
                        authorize(environment)
                    self.assertEqual(
                        raised.exception.code,
                        "AUTHORITY_COMPONENT_LENGTH_MISMATCH",
                    )
                    record["byte_length"] = len(original)
                    rewrite_bundle(environment)
                with self.subTest(role=role, mutation="declared-digest"):
                    record["sha256"] = "0" * 64
                    rewrite_bundle(environment)
                    with self.assertRaises(bootstrap.ConformanceError) as raised:
                        authorize(environment)
                    self.assertEqual(
                        raised.exception.code,
                        "AUTHORITY_COMPONENT_DIGEST_MISMATCH",
                    )
                    record["sha256"] = hashlib.sha256(original).hexdigest()
                    rewrite_bundle(environment)

    def test_policy_must_remain_disabled_empty_and_unavailable(self) -> None:
        mutations = [
            ("enabled", lambda policy: policy.__setitem__("enabled", True)),
            (
                "corpus",
                lambda policy: policy.__setitem__("execution_corpus_sha256", "9" * 64),
            ),
            (
                "adapter",
                lambda policy: policy["adapters"].append(
                    {
                        "language": "go",
                        "platform": "linux",
                        "executable": "code/programs/go/build-tool",
                        "executable_sha256": "1" * 64,
                    }
                ),
            ),
            (
                "backend",
                lambda policy: policy["backends"][1].update(
                    {"status": "ready", "identity_sha256": "2" * 64}
                ),
            ),
        ]
        for name, mutate in mutations:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                environment = write_authority_fixture(Path(directory))
                policy = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.json")
                mutate(policy)
                digest = refresh_component(
                    environment,
                    "execution_policy",
                    json.dumps(policy, sort_keys=True).encode("utf-8"),
                )
                environment["approved_digest"] = digest
                with self.assertRaises(bootstrap.ConformanceError) as raised:
                    authorize(environment)
                self.assertEqual(
                    raised.exception.code,
                    "AUTHORITY_POLICY_PROFILE_INVALID",
                )

    def test_identity_cross_field_mismatches_fail_closed(self) -> None:
        mutations = [
            (
                "reference",
                lambda identity: identity["image"].__setitem__(  # type: ignore[union-attr]
                    "reference",
                    f"localhost/build-tool@sha256:{'9' * 64}",
                ),
                "AUTHORITY_IMAGE_IDENTITY_MISMATCH",
            ),
            (
                "artifact-collision",
                lambda identity: identity["probe"].__setitem__(  # type: ignore[union-attr]
                    "path",
                    identity["shim"]["path"],  # type: ignore[index]
                ),
                "AUTHORITY_IMAGE_ARTIFACT_COLLISION",
            ),
        ]
        for name, mutate, code in mutations:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                environment = write_authority_fixture(Path(directory))
                identity = backend_identity()
                mutate(identity)
                digest = refresh_component(
                    environment,
                    "linux_backend_identity",
                    json.dumps(identity, sort_keys=True).encode("utf-8"),
                )
                environment["approved_digest"] = digest
                with self.assertRaises(bootstrap.ConformanceError) as raised:
                    authorize(environment)
                self.assertEqual(raised.exception.code, code)

    def test_hardlinked_component_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            repository_root = environment["repository_root"]
            assert isinstance(repository_root, Path)
            target = repository_root / REPOSITORY_COMPONENTS["bootstrap_runner"]
            sibling = target.with_name("bootstrap-hardlink.py")
            try:
                os.link(target, sibling)
            except (NotImplementedError, OSError):
                self.skipTest("hard links are unavailable")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                authorize(environment)
            self.assertEqual(
                raised.exception.code,
                "AUTHORITY_COMPONENT_FILE_INVALID",
            )

    def test_symlinked_component_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            bundle_path = environment["bundle_path"]
            assert isinstance(bundle_path, Path)
            identity_path = bundle_path.parent / "linux-oci-backend.json"
            target = bundle_path.parent / "identity-target.json"
            target.write_bytes(identity_path.read_bytes())
            identity_path.unlink()
            try:
                identity_path.symlink_to(target)
            except (NotImplementedError, OSError):
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                authorize(environment)
            self.assertEqual(
                raised.exception.code,
                "AUTHORITY_COMPONENT_FILE_INVALID",
            )

    def test_directory_component_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            repository_root = environment["repository_root"]
            assert isinstance(repository_root, Path)
            target = repository_root / REPOSITORY_COMPONENTS["bootstrap_runner"]
            target.unlink()
            target.mkdir()
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                authorize(environment)
            self.assertEqual(
                raised.exception.code,
                "AUTHORITY_COMPONENT_FILE_INVALID",
            )

    def test_validate_cli_reports_stable_process_free_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            environment = write_authority_fixture(Path(directory))
            stdout = io.StringIO()
            stderr = io.StringIO()
            with redirect_stdout(stdout), redirect_stderr(stderr):
                exit_code = authority.main(
                    [
                        "validate-authority",
                        "--authority-bundle",
                        str(environment["bundle_path"]),
                        "--approved-authority-sha256",
                        str(environment["approved_digest"]),
                        "--source-commit",
                        COMMIT_OID,
                        "--source-tree",
                        TREE_OID,
                        "--repository-root",
                        str(environment["repository_root"]),
                    ]
                )
        self.assertEqual(exit_code, 0)
        output = json.loads(stdout.getvalue())
        self.assertEqual(output["status"], "valid")
        self.assertEqual(output["authorization_scope"], "linux_capability_preflight_v1")
        self.assertNotIn(str(environment["repository_root"]), stdout.getvalue())
        self.assertEqual(stderr.getvalue(), "")

        with redirect_stderr(io.StringIO()):
            self.assertEqual(authority.main([]), 2)

    def test_process_backend_handoff_is_not_exposed_by_this_tranche(self) -> None:
        self.assertFalse(hasattr(authority, "run_authorized_preflight"))
        self.assertFalse(hasattr(authority, "subprocess"))
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            exit_code = authority.main(["preflight"])
        self.assertEqual(exit_code, 2)
        self.assertIn("invalid choice", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
