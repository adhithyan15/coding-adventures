"""Fail-closed tests for Spec 13 capability profile approval.

The gate never uses a test key as a trust anchor. Successful path tests replace
only the call that asks OpenSSH to verify a registered hardware signature;
release_gate's parser, namespace, signer-chain and user-presence checks remain
covered by its own tests and the integration tests below.
"""

from __future__ import annotations

import base64
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType
from typing import Any

REPO = Path(__file__).resolve().parents[3]
WORKFLOW = REPO / ".github" / "workflows" / "ci.yml"
sys.path.insert(0, str(REPO / "code" / "scripts"))

import capability_approval_gate as gate
import release_gate

MANIFEST = "code/packages/rust/demo/required_capabilities.json"
PACKAGE = "rust/demo"
APPROVAL = gate.approval_path_for_manifest(MANIFEST)
CAPABILITY = {
    "category": "time",
    "action": "read",
    "target": "*",
    "justification": "Reads a monotonic clock for measured benchmark batches.",
}


def manifest(capabilities: list[dict[str, Any]]) -> bytes:
    return (
        json.dumps(
            {
                "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
                "version": 1,
                "package": PACKAGE,
                "capabilities": capabilities,
                "justification": "Declares the measured package operating-system access.",
            },
            sort_keys=True,
            indent=2,
        )
        + "\n"
    ).encode()


class StubTrustedReleaseGate:
    """Trusted base helper with only the hardware verification call stubbed."""

    def __init__(self, principal: str = "owner@example.test") -> None:
        self.principal = principal
        self.verified: list[tuple[bytes, str, str]] = []

    def verify_signers_chain(self, repo: Path, ref: str) -> tuple[str, list[dict[str, str]]]:
        self.repo = repo
        self.ref = ref
        return (
            '# revision: 1\nowner@example.test namespaces="coding-adventures-capabilities" '
            "sk-ssh-ed25519@openssh.com AAAATEST owner\n",
            [
                {
                    "principal": self.principal,
                    "options": 'namespaces="coding-adventures-capabilities"',
                    "key_type": "sk-ssh-ed25519@openssh.com",
                    "key": "AAAATEST",
                }
            ],
        )

    def verify_signed_file(self, signers: str, data: bytes, signature: str, namespace: str) -> str:
        self.verified.append((data, signature, namespace))
        if signature != "test-signature\n" or namespace != gate.SIGNATURE_NAMESPACE:
            raise release_gate.GateError("SIG", "test signature rejected")
        return self.principal


class CapabilityApprovalGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="capability-approval-test-")
        self.root = Path(self.temp.name)
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Capability Gate Test")
        self.git("config", "user.email", "capability-gate@example.test")
        self.write(".github/release-signers", (REPO / ".github/release-signers").read_bytes())
        self.write("code/specs/schemas/required_capabilities.schema.json", (REPO / gate.REQUIRED_SCHEMA_PATH).read_bytes())
        self.write("code/specs/schemas/capability-approval.schema.json", (REPO / gate.APPROVAL_SCHEMA_PATH).read_bytes())
        self.write(gate.RELEASE_GATE_PATH, (REPO / gate.RELEASE_GATE_PATH).read_bytes())
        self.write("code/packages/rust/demo/README.md", b"demo\n")
        self.write(MANIFEST, manifest([]))
        self.base = self.commit("base")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str, check: bool = True) -> str:
        proc = subprocess.run(
            ["git", *args],
            cwd=self.root,
            text=True,
            capture_output=True,
            check=False,
        )
        if check and proc.returncode:
            self.fail(f"git {' '.join(args)} failed: {proc.stderr}")
        return proc.stdout.strip()

    def write(self, relative: str, raw: bytes) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)

    def remove(self, relative: str) -> None:
        path = self.root / relative
        if path.is_dir():
            shutil.rmtree(path)
        elif path.exists():
            path.unlink()

    def commit(self, message: str) -> str:
        self.git("add", "-A")
        self.git("commit", "-m", message)
        return self.git("rev-parse", "HEAD")

    def add_approval(self, reason: str = "Approve the monotonic clock requirement.") -> str:
        # Commit the package change first so the approval signs its stable tree.
        package_head = self.commit("change package profile")
        payload = gate.propose_approval(
            str(self.root),
            manifest_path=MANIFEST,
            base_ref="main",
            base_commit=self.base,
            head_commit=package_head,
            reason=reason,
            approved_by="owner@example.test",
            approved_at="2026-11-27T12:00:00Z",
        )
        self.write(APPROVAL, payload)
        self.write(APPROVAL + ".sig", b"test-signature\n")
        return self.commit("add signed approval fixture")

    def validators(self) -> tuple[gate.Draft202012Validator, gate.Draft202012Validator]:
        return gate._load_trusted_schemas(str(self.root), self.base)

    def test_empty_profile_addition_needs_no_approval(self) -> None:
        self.write("code/packages/rust/empty/required_capabilities.json", manifest([]).replace(b"rust/demo", b"rust/empty"))
        head = self.commit("add empty profile")
        self.assertEqual(
            gate.verify_changes(
                str(self.root),
                base_ref="main",
                base_commit=self.base,
                head_commit=head,
                trusted_schema_validators=self.validators(),
            ),
            [],
        )

    def test_nested_fixture_manifest_is_not_a_package_profile(self) -> None:
        fixture = "code/packages/rust/demo/tests/fixtures/required_capabilities.json"
        self.write(fixture, b"fixture data\n")
        head = self.commit("edit nested fixture")
        self.assertEqual(
            gate.verify_changes(
                str(self.root),
                base_ref="main",
                base_commit=self.base,
                head_commit=head,
                trusted_schema_validators=self.validators(),
            ),
            [],
        )

    def test_deep_json_becomes_a_controlled_invalid_json_error(self) -> None:
        with self.assertRaises(gate.GateError) as caught:
            gate.strict_json_loads(b"[" * 10000 + b"0" + b"]" * 10000, "deep input")
        self.assertEqual(caught.exception.code, "INVALID_JSON")
        self.assertIn("nesting depth", caught.exception.message)

    def test_git_blob_size_is_checked_before_blob_read(self) -> None:
        self.write("code/packages/rust/demo/large.bin", b"x" * 32)
        head = self.commit("add bounded blob")
        original_git = gate._git
        commands: list[tuple[str, ...]] = []

        def observing_git(root: str, *args: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
            commands.append(tuple(args))
            return original_git(root, *args, check=check)

        gate._git = observing_git
        try:
            with self.assertRaises(gate.GateError) as caught:
                gate.git_blob(str(self.root), head, "code/packages/rust/demo/large.bin", max_size=8)
        finally:
            gate._git = original_git
        self.assertEqual(caught.exception.code, "INPUT_TOO_LARGE")
        self.assertFalse(any(args[:2] == ("cat-file", "blob") for args in commands))

    def test_duplicate_principal_entries_choose_explicit_namespace(self) -> None:
        entries = [
            {"principal": "owner@example.test", "options": 'namespaces="release"'},
            {"principal": "owner@example.test", "options": f'namespaces="{gate.SIGNATURE_NAMESPACE}"'},
        ]
        self.assertIs(gate._authorized_signer(entries, "owner@example.test"), entries[1])
        self.assertIsNone(gate._authorized_signer(entries[:1], "owner@example.test"))

    def test_safe_proposal_output_rejects_symlinks_and_overwrite(self) -> None:
        outside = self.root.parent / f"{self.root.name}-outside"
        outside.mkdir()
        try:
            (self.root / "linked-dir").symlink_to(outside, target_is_directory=True)
            with self.assertRaises(gate.GateError):
                gate.safe_write_repo_file(str(self.root), "linked-dir/proposal.json", b"x")
            (self.root / "linked-file").symlink_to(outside / "target")
            with self.assertRaises(gate.GateError):
                gate.safe_write_repo_file(str(self.root), "linked-file", b"x")
            self.write("existing.json", b"keep\n")
            with self.assertRaises(gate.GateError):
                gate.safe_write_repo_file(str(self.root), "existing.json", b"replace\n")
            self.assertEqual((self.root / "existing.json").read_bytes(), b"keep\n")
            with self.assertRaises(gate.GateError):
                gate.safe_write_repo_file(str(self.root), "../escape.json", b"x")
        finally:
            shutil.rmtree(outside)

    def test_nonempty_profile_without_signature_is_held(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        head = self.commit("add clock profile")
        with self.assertRaises(gate.GateError) as caught:
            gate.verify_changes(
                str(self.root),
                base_ref="main",
                base_commit=self.base,
                head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(),
                trusted_schema_validators=self.validators(),
            )
        self.assertEqual(caught.exception.code, "MISSING_APPROVAL")
        self.assertIn("capability_approval_gate.py propose", caught.exception.message)

    def test_signed_profile_addition_checks_exact_payload(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        head = self.add_approval()
        trusted = StubTrustedReleaseGate()
        self.assertEqual(
            gate.verify_changes(
                str(self.root),
                base_ref="main",
                base_commit=self.base,
                head_commit=head,
                trusted_release_gate=trusted,
                trusted_schema_validators=self.validators(),
            ),
            [PACKAGE],
        )
        self.assertEqual(trusted.verified[0][2], gate.SIGNATURE_NAMESPACE)
        approval = gate.strict_json_loads(trusted.verified[0][0], "captured approval")
        self.assertEqual(approval["package_tree"], self.git("rev-parse", f"{head}:code/packages/rust/demo"))

    def test_profile_reductions_require_signature(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        old = self.commit("declare clock capability")
        self.base = old
        self.write(MANIFEST, manifest([]))
        head = self.add_approval("Approve removal of the clock capability.")
        self.assertEqual(
            gate.verify_changes(
                str(self.root),
                base_ref="main",
                base_commit=self.base,
                head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(),
                trusted_schema_validators=self.validators(),
            ),
            [PACKAGE],
        )

    def _invalid_base_repair(self, *, delete_manifest: bool = False, delete_package: bool = False) -> str:
        self.write(MANIFEST, b"[]\n")
        self.base = self.commit("historical invalid manifest")
        if delete_manifest:
            self.remove(MANIFEST)
            if delete_package:
                self.remove("code/packages/rust/demo")
        else:
            self.write(MANIFEST, manifest([]))
        package_head = self.commit("repair invalid manifest")
        payload = gate.propose_approval(
            str(self.root),
            manifest_path=MANIFEST,
            base_ref="main",
            base_commit=self.base,
            head_commit=package_head,
            reason="Approve repair of the invalid historical capability profile.",
            approved_by="owner@example.test",
            approved_at="2026-11-27T12:00:00Z",
        )
        doc = gate.strict_json_loads(payload, "repair proposal")
        self.assertEqual(doc["previous_profile_status"], "invalid")
        self.assertIsNone(doc["previous_capabilities"])
        self.assertIsNone(doc["previous_banned_construct_exceptions"])
        self.assertEqual(doc["previous_manifest_sha256"], hashlib.sha256(b"[]\n").hexdigest())
        self.write(APPROVAL, payload)
        self.write(APPROVAL + ".sig", b"test-signature\n")
        return self.commit("add invalid-profile repair approval")

    def test_invalid_base_to_valid_empty_profile_requires_signed_repair(self) -> None:
        head = self._invalid_base_repair()
        self.assertEqual(
            gate.verify_changes(
                str(self.root),
                base_ref="main",
                base_commit=self.base,
                head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(),
                trusted_schema_validators=self.validators(),
            ),
            [PACKAGE],
        )

    def test_manifest_only_deletion_has_absent_payload(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        self.base = self.commit("valid nonempty profile")
        self.remove(MANIFEST)
        package_head = self.commit("delete manifest")
        payload = gate.propose_approval(
            str(self.root), manifest_path=MANIFEST, base_ref="main", base_commit=self.base,
            head_commit=package_head, reason="Approve deletion of the capability manifest.",
            approved_by="owner@example.test", approved_at="2026-11-27T12:00:00Z",
        )
        doc = gate.strict_json_loads(payload, "deletion proposal")
        self.assertEqual(doc["new_profile_status"], "absent")
        self.assertIsNone(doc["new_manifest_sha256"])
        self.assertEqual(doc["new_capabilities"], [])
        self.assertIsNotNone(doc["package_tree"])
        self.write(APPROVAL, payload)
        self.write(APPROVAL + ".sig", b"test-signature\n")
        head = self.commit("approve manifest deletion")
        self.assertEqual(
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(), trusted_schema_validators=self.validators(),
            ),
            [PACKAGE],
        )

    def test_whole_package_deletion_has_null_tree(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        self.base = self.commit("valid nonempty profile")
        self.remove("code/packages/rust/demo")
        package_head = self.commit("delete whole package")
        payload = gate.propose_approval(
            str(self.root), manifest_path=MANIFEST, base_ref="main", base_commit=self.base,
            head_commit=package_head, reason="Approve deletion of the capability-bearing package.",
            approved_by="owner@example.test", approved_at="2026-11-27T12:00:00Z",
        )
        doc = gate.strict_json_loads(payload, "package deletion proposal")
        self.assertEqual(doc["new_profile_status"], "absent")
        self.assertIsNone(doc["package_tree"])
        self.write(APPROVAL, payload)
        self.write(APPROVAL + ".sig", b"test-signature\n")
        head = self.commit("approve package deletion")
        self.assertEqual(
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(), trusted_schema_validators=self.validators(),
            ),
            [PACKAGE],
        )

    def test_changed_package_tree_invalidates_signature(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        self.add_approval()
        self.write("code/packages/rust/demo/README.md", b"source changed after signature\n")
        head = self.commit("change package tree after approval")
        with self.assertRaises(gate.GateError) as caught:
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(), trusted_schema_validators=self.validators(),
            )
        self.assertEqual(caught.exception.code, "STALE_APPROVAL")

    def test_duplicate_and_noncanonical_approval_json_are_rejected(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        self.add_approval()
        raw = (self.root / APPROVAL).read_bytes()
        self.write(APPROVAL, raw.rstrip()[:-1] + b',"version":1}\n')
        head = self.commit("duplicate approval field")
        with self.assertRaises(gate.GateError) as caught:
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(), trusted_schema_validators=self.validators(),
            )
        self.assertEqual(caught.exception.code, "DUPLICATE_JSON_MEMBER")

    def test_noncanonical_approval_json_is_rejected(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        self.add_approval()
        approval = gate.strict_json_loads((self.root / APPROVAL).read_bytes(), "approval")
        self.write(APPROVAL, json.dumps(approval, indent=2).encode() + b"\n")
        head = self.commit("noncanonical approval")
        with self.assertRaises(gate.GateError) as caught:
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=head,
                trusted_release_gate=StubTrustedReleaseGate(), trusted_schema_validators=self.validators(),
            )
        self.assertEqual(caught.exception.code, "NONCANONICAL_APPROVAL")

    def test_invalid_prior_profile_cannot_claim_empty_previous_profile(self) -> None:
        head = self._invalid_base_repair()
        approval = gate.strict_json_loads((self.root / APPROVAL).read_bytes(), "approval")
        approval["previous_capabilities"] = []
        self.write(APPROVAL, gate.canonical_json_bytes(approval))
        final = self.commit("invent old profile")
        with self.assertRaises(gate.GateError) as caught:
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=final,
                trusted_release_gate=StubTrustedReleaseGate(), trusted_schema_validators=self.validators(),
            )
        self.assertEqual(caught.exception.code, "INVALID_APPROVAL")

    def test_candidate_schema_and_registry_do_not_supply_trust(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        self.write("code/specs/schemas/required_capabilities.schema.json", b"{}\n")
        self.write("code/specs/schemas/capability-approval.schema.json", b"{}\n")
        self.write("code/scripts/release_gate.py", b"raise RuntimeError('candidate verifier ran')\n")
        self.write(".github/release-signers", b"# revision: 1\nattacker namespaces=\"coding-adventures-capabilities\" sk-ssh-ed25519@openssh.com AAAA\n")
        candidate = self.commit("tamper with candidate trust files")
        proposal = gate.propose_approval(
            str(self.root), manifest_path=MANIFEST, base_ref="main", base_commit=self.base,
            head_commit=candidate, reason="Review the changed capability profile.",
        )
        self.write(APPROVAL, proposal)
        self.write(APPROVAL + ".sig", b"made-up-signature\n")
        head = self.commit("add unsigned approval-shaped input")
        with self.assertRaises(gate.GateError) as caught:
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=head,
            )
        self.assertEqual(caught.exception.code, "NO_TRUSTED_SIGNER")

    def test_missing_base_verifier_does_not_fall_back_to_candidate(self) -> None:
        self.remove(gate.RELEASE_GATE_PATH)
        self.base = self.commit("base without verifier")
        self.write(gate.RELEASE_GATE_PATH, b"raise RuntimeError('candidate verifier ran')\n")
        self.write(MANIFEST, manifest([CAPABILITY]))
        candidate = self.commit("candidate adds profile and its own helper")
        proposal = gate.propose_approval(
            str(self.root), manifest_path=MANIFEST, base_ref="main", base_commit=self.base,
            head_commit=candidate, reason="Review the changed capability profile.",
        )
        self.write(APPROVAL, proposal)
        self.write(APPROVAL + ".sig", b"made-up-signature\n")
        head = self.commit("candidate adds an unsigned approval")
        with self.assertRaises(gate.GateError) as caught:
            gate.verify_changes(
                str(self.root), base_ref="main", base_commit=self.base, head_commit=head,
            )
        self.assertEqual(caught.exception.code, "TRUSTED_VERIFIER")

    def test_unsigned_proposal_command_writes_no_signature(self) -> None:
        self.write(MANIFEST, manifest([CAPABILITY]))
        head = self.commit("profile for unsigned proposal")
        raw = gate.propose_approval(
            str(self.root), manifest_path=MANIFEST, base_ref="main", base_commit=self.base,
            head_commit=head, reason="Review the monotonic clock profile.",
        )
        proposal = gate.strict_json_loads(raw, "proposal")
        self.assertEqual(proposal["approved_by"], "UNSIGNED-PROPOSAL")
        self.assertTrue(raw.endswith(b"\n"))
        self.assertFalse((self.root / (APPROVAL + ".sig")).exists())


def _ssh_string(value: bytes) -> bytes:
    return len(value).to_bytes(4, "big") + value


def _security_key_sshsig(flags: int) -> str:
    key_type = b"sk-ssh-ed25519@openssh.com"
    public_key = _ssh_string(key_type) + _ssh_string(b"\0" * 32) + _ssh_string(b"application")
    signature = _ssh_string(key_type) + _ssh_string(b"\1" * 64) + bytes([flags]) + (7).to_bytes(4, "big")
    blob = (
        b"SSHSIG"
        + (1).to_bytes(4, "big")
        + _ssh_string(public_key)
        + _ssh_string(gate.SIGNATURE_NAMESPACE.encode())
        + _ssh_string(b"")
        + _ssh_string(b"sha512")
        + _ssh_string(signature)
    )
    return "-----BEGIN SSH SIGNATURE-----\n" + base64.b64encode(blob).decode() + "\n-----END SSH SIGNATURE-----\n"


@unittest.skipUnless(shutil.which("ssh-keygen"), "OpenSSH is required")
class SSHSignatureBoundaryTests(unittest.TestCase):
    def test_real_openssh_signature_with_wrong_namespace_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="capability-sshsig-test-") as tmp:
            root = Path(tmp)
            key = root / "test-key"
            message = root / "payload.json"
            message.write_bytes(b"test payload\n")
            subprocess.run(
                ["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)],
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ["ssh-keygen", "-Y", "sign", "-n", "wrong-namespace", "-f", str(key), str(message)],
                check=True,
                capture_output=True,
            )
            signature = message.with_name(message.name + ".sig").read_text("utf-8")
            with self.assertRaises(release_gate.GateError) as caught:
                release_gate.verify_signed_file(
                    "", message.read_bytes(), signature, gate.SIGNATURE_NAMESPACE
                )
            self.assertIn("namespace", caught.exception.message)

    def test_user_presence_and_verification_flags_are_required(self) -> None:
        for flags in (0x00, 0x01, 0x04):
            with self.subTest(flags=flags), self.assertRaises(release_gate.GateError):
                release_gate.require_hardware_flags(
                    release_gate.parse_sshsig(_security_key_sshsig(flags))
                )
        release_gate.require_hardware_flags(
            release_gate.parse_sshsig(_security_key_sshsig(0x05))
        )


class CapabilityBootstrapWorkflowTests(unittest.TestCase):
    def test_actual_first_rollout_block_never_executes_candidate_verifier(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        marker = "      - name: Verify capability profile approvals\n"
        start = workflow.index(marker)
        run_marker = "        run: |\n"
        run_start = workflow.index(run_marker, start) + len(run_marker)
        end = workflow.find("\n      - name:", run_start)
        self.assertNotEqual(end, -1, "workflow step must have a following step")
        block = workflow[run_start:end]
        script_lines = []
        for line in block.splitlines():
            if line.startswith("          "):
                script_lines.append(line[10:])
            elif line:
                break
        script = "\n".join(script_lines) + "\n"

        with tempfile.TemporaryDirectory(prefix="capability-bootstrap-workflow-") as temp:
            root = Path(temp)
            subprocess.run(["git", "init", "-b", "main"], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.name", "Bootstrap test"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "bootstrap@example.test"], cwd=root, check=True)

            def commit(message: str) -> str:
                subprocess.run(["git", "add", "-A"], cwd=root, check=True)
                subprocess.run(["git", "commit", "-m", message], cwd=root, check=True, capture_output=True)
                return subprocess.run(
                    ["git", "rev-parse", "HEAD"], cwd=root, check=True,
                    capture_output=True, text=True,
                ).stdout.strip()

            (root / "README.md").write_text("base\n", encoding="utf-8")
            base = commit("base without verifier")
            candidate_script = root / gate.RELEASE_GATE_PATH.replace("release_gate.py", "capability_approval_gate.py")
            candidate_script.parent.mkdir(parents=True, exist_ok=True)
            candidate_script.write_text(
                "from pathlib import Path\nPath('candidate-verifier-ran').write_text('bad')\n",
                encoding="utf-8",
            )
            head_without_profile = commit("candidate adds verifier")

            def run_block(head: str) -> subprocess.CompletedProcess[str]:
                env = dict(os.environ)
                env.update({"GITHUB_BASE_REF": "main", "PR_BASE_SHA": base, "PR_HEAD_SHA": head})
                return subprocess.run(
                    ["bash", "-e", "-u", "-o", "pipefail", "-c", script, "workflow-step", base, head],
                    cwd=root,
                    env=env,
                    check=False,
                    capture_output=True,
                    text=True,
                )

            first = run_block(head_without_profile)
            self.assertEqual(first.returncode, 0, first.stderr)
            self.assertIn("not an enforced approval gate", first.stdout)
            self.assertFalse((root / "candidate-verifier-ran").exists())

            profile = root / "code/packages/rust/demo/required_capabilities.json"
            profile.parent.mkdir(parents=True)
            profile.write_text("{}\n", encoding="utf-8")
            head_with_profile = commit("candidate changes profile")
            rejected = run_block(head_with_profile)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("cannot change in this rollout", rejected.stderr)
            self.assertFalse((root / "candidate-verifier-ran").exists())


if __name__ == "__main__":
    unittest.main()
