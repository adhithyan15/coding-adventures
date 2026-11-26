"""Tests for `release_gate.py`, the VER31 release gate.

The gate decides what a package may be published as, so a rule that is only
tested positively could be deleted without a test noticing.  Every rule below
therefore has a *falsification* test: a deliberately broken input that must be
refused, or a hand-inflated label that must be computed back down.  The
positive tests run the gate over the real `rust/ct-compare` report and expect
exactly the numbers VER31 section 10 publishes.

Signature tests use real `ssh-keygen`.  A FIDO2 key cannot be created in CI,
so the tests use a software ed25519 key and check that the gate *refuses* it
(only `sk-*` keys may sign releases); the security-key flag parser is tested on
hand-built SSHSIG blobs.
"""

from __future__ import annotations

import base64
import copy
import dataclasses
import io
import json
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
import unittest.mock
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import release_gate as rg

CT = REPO / "code" / "packages" / "rust" / "ct-compare"
REPORT = json.loads((CT / "assurance_report.json").read_text("utf-8"))
BUCKETS = rg.load_buckets(REPO / rg.BUCKETS_REL, REPO / rg.CATALOGUE_REL)
RAW_BUCKETS = json.loads((REPO / rg.BUCKETS_REL).read_text("utf-8"))
HAVE_SSH = shutil.which("ssh-keygen") is not None


def claim(**over) -> dict:
    """A synthetic passing claim with a proved label and one id per bucket."""
    base = {
        "id": "c1",
        "symbol": "f",
        "example": "pass",
        "property": {"status": "pass"},
        "fuzz": {"status": "pass"},
        "proof": {
            "label": "proved",
            "certificate": "drat:sha256:" + "0" * 64,
            "assumptions": ["cc.version-pinned", "spec.secret-labels.complete", "wir.inline.sound"],
            "discharged": [],
        },
    }
    for key, value in over.items():
        if key.startswith("proof_"):
            base["proof"][key[6:]] = value
        else:
            base[key] = value
    return base


def proved_claim(assumptions: list[str], discharged: list[str] = ()) -> dict:
    """A proved-label claim whose tool-trust ids are discharged (as they must be)."""
    tools = list(rg.ALWAYS_ASSUMED)
    return claim(proof_assumptions=sorted(set(assumptions) | set(tools)),
                 proof_discharged=sorted(set(discharged) | set(tools)))


def with_catalogue_discharged(*ids: str) -> rg.Buckets:
    """The real buckets, as if the catalogue recorded ``ids`` (and the tools) discharged."""
    return dataclasses.replace(BUCKETS, catalogue_discharged=frozenset(ids) | frozenset(rg.ALWAYS_ASSUMED))


def write_buckets(tmp: Path, doc: dict) -> Path:
    path = tmp / "buckets.json"
    path.write_text(json.dumps(doc), encoding="utf-8")
    return path


class BucketTests(unittest.TestCase):
    def test_real_file_covers_catalogue(self) -> None:
        self.assertEqual(len(BUCKETS.bucket), 84)
        counts = {k: sum(1 for v in BUCKETS.bucket.values() if v == k) for k in "abc"}
        self.assertEqual(counts, {"a": 35, "b": 5, "c": 44})
        self.assertEqual(BUCKETS.bucket["tcb.rust-toolchain.trusted"], "a")
        for kept_c in ("tcb.drat.checker-correct", "tcb.report.checker-correct", "wir.lowering.faithful"):
            self.assertEqual(BUCKETS.bucket[kept_c], "c", kept_c)

    def _refused(self, mutate, code: str = "BUCKETS") -> None:
        doc = copy.deepcopy(RAW_BUCKETS)
        mutate(doc)
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(rg.GateError) as ctx:
            rg.load_buckets(write_buckets(Path(tmp), doc), REPO / rg.CATALOGUE_REL)
        self.assertEqual(ctx.exception.code, code)

    def test_missing_id_refused(self) -> None:
        self._refused(lambda d: d["assumptions"].pop())

    def test_extra_id_refused(self) -> None:
        self._refused(lambda d: d["assumptions"].append(
            {"id": "wir.invented", "bucket": "c", "floor": False, "rationale": "x"}))

    def test_duplicate_refused(self) -> None:
        self._refused(lambda d: d["assumptions"].append(dict(d["assumptions"][0])))

    def test_bucket_meaning_cannot_be_redefined(self) -> None:
        def flip(d):
            d["buckets"]["c"]["blocks_proved"] = False
        self._refused(flip)

    def test_floor_flag_must_match_catalogue(self) -> None:
        def lie(d):
            for e in d["assumptions"]:
                if e["id"] == "uarch.speculation.none":
                    e["floor"] = True
        self._refused(lie)

    def test_bad_schema_and_revision_refused(self) -> None:
        self._refused(lambda d: d.update(schema="other"))
        self._refused(lambda d: d.update(revision=0))
        self._refused(lambda d: d.update(revision=True))

    def test_unknown_bucket_refused(self) -> None:
        self._refused(lambda d: d["assumptions"][0].update(bucket="k"))


class LadderTests(unittest.TestCase):
    def test_real_ct_compare_matches_ver31_section_10(self) -> None:
        rungs = {c["id"]: rg.claim_rung(c, BUCKETS, set()) for c in REPORT["claims"]}
        self.assertEqual(rungs["ct_eq_u64_soundness"]["generator_label"], "proved")
        self.assertEqual(rungs["ct_eq_u64_soundness"]["rung"], "checked")
        for ident in ("ct_eq_soundness", "ct_eq_fixed_soundness", "ct_select_bytes_soundness"):
            self.assertEqual(rungs[ident]["rung"], "bounded(128)")
        for rec in rungs.values():
            self.assertEqual(len(rec["open_c"]), 20)
            self.assertEqual(len(rec["open_b"]), 5)
            self.assertEqual({k: len(v) for k, v in rec["assumptions"].items()}, {"a": 28, "b": 5, "c": 20})
        self.assertEqual(rg.weakest(r["rung"] for r in rungs.values()), "bounded(128)")

    def test_hand_typed_proved_with_open_c_computes_checked(self) -> None:
        self.assertEqual(rg.claim_rung(claim(), BUCKETS, {"spec.secret-labels.complete"})["rung"], "checked")

    def test_unsigned_statement_blocks_proved(self) -> None:
        b = with_catalogue_discharged()
        c = proved_claim(["cc.version-pinned", "spec.secret-labels.complete"])
        self.assertEqual(rg.claim_rung(c, b, set())["rung"], "checked")
        self.assertEqual(rg.claim_rung(c, b, {"spec.secret-labels.complete"})["rung"], "proved")

    def test_baseline_discharged_needs_every_non_floor_a(self) -> None:
        b = with_catalogue_discharged("cc.version-pinned")
        c = proved_claim(["cc.version-pinned", "phys.silicon.realization"], ["cc.version-pinned"])
        self.assertEqual(rg.claim_rung(c, b, set())["rung"], "proved, baseline discharged")
        c = proved_claim(["cc.version-pinned", "phys.silicon.realization"])
        self.assertEqual(rg.claim_rung(c, b, set())["rung"], "proved")

    def test_discharged_c_no_longer_blocks(self) -> None:
        c = proved_claim(["wir.inline.sound"], ["wir.inline.sound"])
        b = with_catalogue_discharged("wir.inline.sound")
        self.assertEqual(rg.claim_rung(c, b, set())["rung"], "proved, baseline discharged")

    def test_report_discharge_without_catalogue_record_does_not_count(self) -> None:
        c = proved_claim(["wir.inline.sound"], ["wir.inline.sound"])
        self.assertEqual(rg.claim_rung(c, with_catalogue_discharged(), set())["rung"], "checked")
        # The real catalogue discharges nothing, so even the tool ids stay open.
        self.assertEqual(rg.claim_rung(c, BUCKETS, set())["rung"], "checked")

    def test_tool_trust_ids_are_added_even_when_the_report_omits_them(self) -> None:
        bare = claim(proof_assumptions=[], proof_discharged=[])
        rec = rg.claim_rung(bare, BUCKETS, set())
        self.assertEqual(rec["rung"], "checked")
        self.assertEqual(sorted(rec["open_c"]), sorted(rg.ALWAYS_ASSUMED))

    def test_spoofing_text_refused(self) -> None:
        for over in ({"id": "c1\n## proved"}, {"symbol": "f|x"}, {"proof_label": "proved`"}):
            with self.assertRaises(rg.GateError):
                rg.claim_rung(claim(**over), BUCKETS, set())

    def test_proved_per_run_is_at_most_checked(self) -> None:
        c = claim(proof_label="proved-per-run", proof_assumptions=["cc.version-pinned"])
        self.assertEqual(rg.claim_rung(c, BUCKETS, set())["rung"], "checked")

    def test_label_without_certificate_is_only_tested(self) -> None:
        self.assertEqual(rg.claim_rung(claim(proof_certificate=None), BUCKETS, set())["rung"], "tested")
        bounded = claim(proof_label="bounded(64)", proof_certificate="drat:sha256:short")
        self.assertEqual(rg.claim_rung(bounded, BUCKETS, set())["rung"], "tested")

    def test_exhaustive_needs_explicit_full_domain(self) -> None:
        no_proof = claim(proof=None, exhaustive={"status": "pass", "cases": 65536})
        self.assertEqual(rg.claim_rung(no_proof, BUCKETS, set())["rung"], "tested")
        full = claim(proof=None, exhaustive={"status": "pass", "cases": 65536, "domain": "full"})
        self.assertEqual(rg.claim_rung(full, BUCKETS, set())["rung"], "exhaustive")

    def test_refusals(self) -> None:
        cases = [
            claim(example="fail"),
            claim(property={"status": "fail"}),
            claim(fuzz={"status": "fail"}),
            claim(proof_label="proven"),
            claim(proof_assumptions=["no.such.id"]),
            claim(proof_discharged=["cc.caller-context"]),
        ]
        for c in cases:
            with self.assertRaises(rg.GateError, msg=json.dumps(c)):
                rg.claim_rung(c, BUCKETS, set())

    def test_ordering(self) -> None:
        order = ["tested", "bounded(8)", "bounded(128)", "exhaustive", "checked", "proved",
                 "proved, baseline discharged"]
        shuffled = [order[i] for i in (4, 0, 6, 2, 5, 1, 3)]
        self.assertEqual(sorted(shuffled, key=rg.rung_key), order)
        self.assertEqual(rg.weakest(["proved", "bounded(8)", "checked"]), "bounded(8)")
        with self.assertRaises(rg.GateError):
            rg.weakest([])
        with self.assertRaises(rg.GateError):
            rg.rung_key("mostly-proved")


class LaneAndConditionTests(unittest.TestCase):
    def test_live_lane_is_tested(self) -> None:
        claims = rg.lane_claims(REPORT, "typescript/ct-compare")
        self.assertEqual({c["rung"] for c in claims}, {"tested"})
        self.assertEqual(len(claims), len(REPORT["claims"]))

    def test_lane_refusals(self) -> None:
        with self.assertRaises(rg.GateError):
            rg.lane_claims(REPORT, "typescript/absent")
        doc = copy.deepcopy(REPORT)
        doc["lanes"][0]["rung"] = "planned"
        with self.assertRaises(rg.GateError):
            rg.lane_claims(doc, doc["lanes"][0]["package"])
        doc = copy.deepcopy(REPORT)
        doc["lanes"].append(copy.deepcopy(doc["lanes"][0]))
        with self.assertRaises(rg.GateError):
            rg.lane_claims(doc, doc["lanes"][0]["package"])

    def test_real_lane_report_found(self) -> None:
        self.assertEqual(rg.find_lane_report(REPO, "typescript/ct-compare"), CT / "assurance_report.json")

    def test_conditions_today(self) -> None:
        held = {h["id"]: h["holds"] for h in rg.higher_tier_conditions(REPORT, True, False)}
        self.assertEqual(held, {"H1": True, "H2": False, "H3": False, "H4": False})
        doc = copy.deepcopy(REPORT)
        doc["verdict"] = "pass"
        doc["rungs"] = {k: "pass" for k in doc["rungs"]}
        doc["exemptions"] = []
        self.assertTrue(all(h["holds"] for h in rg.higher_tier_conditions(doc, True, True)))


class ManifestTests(unittest.TestCase):
    def test_cargo_runtime_deps(self) -> None:
        clean = '[package]\nname = "x"\n\n[dependencies]\n\n[dev-dependencies]\nfoo = "1"\n'
        self.assertEqual(rg.cargo_has_runtime_deps(clean), [])
        for text in (
            '[dependencies]\nserde = "1"\n',
            '[build-dependencies]\ncc = "1"\n',
            "[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n",
            '[dependencies.serde]\nversion = "1"\n',
            # The six forms the line scanner missed (security review, LV-007a).
            '[dependencies] # runtime\nserde = "1"\n',
            '["dependencies"]\nserde = "1"\n',
            'dependencies = { serde = "1" }\n',
            'target."cfg(unix)".dependencies.serde = "1"\n',
            '[target]\n"cfg(unix)".dependencies.serde = "1"\n',
            '[target."cfg(unix)"]\ndependencies = { serde = "1" }\n',
            '[build_dependencies]\ncc = "1"\n',
        ):
            self.assertTrue(rg.cargo_has_runtime_deps(text), text)
        self.assertEqual(rg.cargo_has_runtime_deps("[target.'cfg(unix)'.dev-dependencies]\nx = \"1\"\n"), [])
        # A dotted key under [package] is package.dependencies, not a dependency.
        self.assertEqual(rg.cargo_has_runtime_deps('[package]\nname = "x"\n'), [])
        with self.assertRaises(rg.GateError):
            rg.cargo_has_runtime_deps("[dependencies\n")

    def test_cargo_package_field(self) -> None:
        text = '[package]\nname = "a"\nversion = "0.1.0" # c\n[lib]\nname = "b"\n'
        self.assertEqual(rg.cargo_package_field(text, "name"), "a")
        self.assertEqual(rg.cargo_package_field(text, "version"), "0.1.0")
        self.assertIsNone(rg.cargo_package_field(text, "license"))

    def test_real_manifests(self) -> None:
        m = rg.read_manifest(CT, "rust")
        self.assertEqual((m.registry_name, m.runtime_deps), ("coding_adventures_ct_compare", []))
        ts = rg.read_manifest(REPO / "code/packages/typescript/ct-compare", "typescript")
        self.assertEqual((ts.registry_name, ts.runtime_deps), ("@coding-adventures/ct-compare", []))
        with self.assertRaises(rg.GateError):
            rg.read_manifest(CT, "cobol")

    def test_changelog_and_lcov(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            pkg = Path(tmp)
            (pkg / "CHANGELOG.md").write_text("## [0.1.0]\n- first\n", "utf-8")
            self.assertTrue(rg.changelog_mentions(pkg, "0.1.0"))
            self.assertFalse(rg.changelog_mentions(pkg, "0.1"))
            self.assertFalse(rg.changelog_mentions(pkg, "10.1.0"))
        lcov = "SF:/w/code/pkg/src/lib.rs\nLF:10\nLH:9\nend_of_record\nSF:/w/other/src/x.rs\nLF:100\nLH:0\n"
        self.assertEqual(rg.lcov_line_coverage(lcov, "code/pkg/src/"), (9, 10))

    def test_safe_rel(self) -> None:
        for good in ("a/b.rs", "code/packages/rust/x/src/lib.rs"):
            self.assertTrue(rg.safe_rel(good), good)
        for bad in ("", "/etc/passwd", "../x", "a/../b", ":(exclude)a", "a/*.rs", "a//b", "./a", "a\\b"):
            self.assertFalse(rg.safe_rel(bad), bad)


def _sshsig(key_type: str, namespace: str, flags: int | None) -> str:
    """Hand-build an armored SSHSIG blob (no valid cryptography)."""
    def s(b: bytes) -> bytes:
        return len(b).to_bytes(4, "big") + b
    pub = s(key_type.encode()) + s(b"\x00" * 32)
    sig = s(key_type.encode()) + s(b"\x01" * 64)
    if flags is not None:
        sig += bytes([flags]) + (7).to_bytes(4, "big")
    blob = b"SSHSIG" + (1).to_bytes(4, "big") + s(pub) + s(namespace.encode()) + s(b"") + s(b"sha512") + s(sig)
    body = base64.b64encode(blob).decode()
    return "-----BEGIN SSH SIGNATURE-----\n" + body + "\n-----END SSH SIGNATURE-----\n"


SK = "sk-ssh-ed25519@openssh.com"


class SignatureFormatTests(unittest.TestCase):
    def test_flags_parsed(self) -> None:
        sig = rg.parse_sshsig(_sshsig(SK, "git", 0x05))
        self.assertEqual((sig["key_type"], sig["namespace"], sig["flags"]), (SK, "git", 0x05))
        rg.require_hardware_flags(sig)

    def test_missing_user_verification_refused(self) -> None:
        with self.assertRaises(rg.GateError):
            rg.require_hardware_flags(rg.parse_sshsig(_sshsig(SK, "git", 0x01)))
        with self.assertRaises(rg.GateError):
            rg.require_hardware_flags(rg.parse_sshsig(_sshsig(SK, "git", 0x04)))

    def test_software_key_refused(self) -> None:
        with self.assertRaises(rg.GateError):
            rg.require_hardware_flags(rg.parse_sshsig(_sshsig("ssh-ed25519", "git", None)))

    def test_malformed_signatures_refused(self) -> None:
        for bad in ("", "hello", "-----BEGIN SSH SIGNATURE-----\n!!!\n-----END SSH SIGNATURE-----",
                    "-----BEGIN SSH SIGNATURE-----\nQUJD\n-----END SSH SIGNATURE-----"):
            with self.assertRaises(rg.GateError, msg=bad):
                rg.parse_sshsig(bad)

    def test_signers_parsing(self) -> None:
        good = f'# revision: 3\nowner@example namespaces="git" {SK} AAAAkey comment\n'
        rev, entries = rg.parse_signers(good)
        self.assertEqual((rev, entries[0]["principal"]), (3, "owner@example"))
        for bad in (
            f"owner {SK} AAAA\n",  # no revision
            "# revision: 1\n",  # no key
            "# revision: 1\nowner ssh-ed25519 AAAA\n",  # software key
            f"# revision: 1\nowner cert-authority {SK} AAAA\n",  # unsupported option
            "# revision: 1\nowner\n",
        ):
            with self.assertRaises(rg.GateError, msg=bad):
                rg.parse_signers(bad)


def _git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True, text=True).stdout


def _init_repo(tmp: Path) -> Path:
    repo = tmp / "r"
    repo.mkdir()
    _git(repo, "init", "-q", "-b", "main")
    _git(repo, "config", "user.email", "gate@test")
    _git(repo, "config", "user.name", "gate")
    _git(repo, "config", "commit.gpgsign", "false")
    _git(repo, "config", "tag.gpgsign", "false")
    return repo


def _commit(repo: Path, files: dict[str, str], message: str = "c") -> str:
    for rel, text in files.items():
        path = repo / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, "utf-8")
        _git(repo, "add", rel)
    _git(repo, "commit", "-q", "-m", message)
    return _git(repo, "rev-parse", "HEAD").strip()


class SignersChainTests(unittest.TestCase):
    def test_anchor_only_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            _commit(repo, {rg.SIGNERS_REL: "# placeholder\n"})
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 1\nowner {SK} AAAA\n"})
            _text, entries = rg.verify_signers_chain(repo, "HEAD")
            self.assertEqual(entries[0]["principal"], "owner")

    def test_no_history_or_placeholder_only_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            _commit(repo, {"README": "x"})
            with self.assertRaises(rg.GateError):
                rg.verify_signers_chain(repo, "HEAD")
            _commit(repo, {rg.SIGNERS_REL: "# placeholder\n"})
            with self.assertRaises(rg.GateError):
                rg.verify_signers_chain(repo, "HEAD")

    def test_unsigned_or_skipped_revision_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 1\nowner {SK} AAAA\n"})
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 2\nowner {SK} AAAA\nnew {SK} BBBB\n"})
            with self.assertRaises(rg.GateError) as ctx:
                rg.verify_signers_chain(repo, "HEAD")
            self.assertIn("no .sig", ctx.exception.message)
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 1\nowner {SK} AAAA\n"})
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 3\nowner {SK} AAAA\n"})
            with self.assertRaises(rg.GateError):
                rg.verify_signers_chain(repo, "HEAD")

    @unittest.skipUnless(HAVE_SSH, "ssh-keygen not installed")
    def test_software_signed_transition_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            key = Path(tmp) / "k"
            subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)], check=True)
            new = f"# revision: 2\nowner {SK} AAAA\n"
            sig = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", str(key), "-n", rg.NS_SIGNERS],
                                 input=new.encode(), capture_output=True, check=True).stdout.decode()
            repo = _init_repo(Path(tmp))
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 1\nowner {SK} AAAA\n"})
            _commit(repo, {rg.SIGNERS_REL: new, rg.SIGNERS_REL + ".sig": sig})
            with self.assertRaises(rg.GateError) as ctx:
                rg.verify_signers_chain(repo, "HEAD")
            self.assertIn("security key", ctx.exception.message)

    @unittest.skipUnless(HAVE_SSH, "ssh-keygen not installed")
    def test_ssh_verify_checks_cryptography(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            key = Path(tmp) / "k"
            subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)], check=True)
            pub = (Path(tmp) / "k.pub").read_text().split()
            signers = f"me {pub[0]} {pub[1]}\n"
            sig = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", str(key), "-n", "ns"],
                                 input=b"data", capture_output=True, check=True).stdout.decode()
            self.assertTrue(rg.ssh_verify(signers, "me", "ns", b"data", sig))
            self.assertFalse(rg.ssh_verify(signers, "me", "ns", b"tampered", sig))
            self.assertFalse(rg.ssh_verify(signers, "me", "other-namespace", b"data", sig))


class TagTests(unittest.TestCase):
    def test_lightweight_and_misplaced_tags_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            first = _commit(repo, {"a": "1"})
            second = _commit(repo, {"a": "2"})
            _git(repo, "tag", "rust/x/v0.1.0", first)
            with self.assertRaises(rg.GateError) as ctx:
                rg.verify_tag(repo, "rust/x/v0.1.0", first, "")
            self.assertIn("lightweight", ctx.exception.message)
            _git(repo, "tag", "-a", "-m", "m", "rust/y/v0.1.0", first)
            with self.assertRaises(rg.GateError) as ctx:
                rg.verify_tag(repo, "rust/y/v0.1.0", second, "")
            self.assertIn("points at", ctx.exception.message)
            with self.assertRaises(rg.GateError):  # unsigned annotated tag
                rg.verify_tag(repo, "rust/y/v0.1.0", first, f"# revision: 1\nowner {SK} AAAA\n")

    def test_input_validation(self) -> None:
        rg.validate_inputs("rust/ct-compare/v0.1.0", "a" * 40, "refs/remotes/origin/main")
        for tag, commit, ref in (
            ("rust/ct-compare/v0.1.0;rm", None, "HEAD"),
            ("go/x/v1.0.0", None, "HEAD"),
            (None, "abc", "HEAD"),
            (None, None, "main"),
            (None, None, "refs/heads/$(x)"),
        ):
            with self.assertRaises(rg.GateError):
                rg.validate_inputs(tag, commit, ref)


def _tgz(members: dict[str, bytes], link: str | None = None) -> bytes:
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tar:
        for name, data in members.items():
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))
        if link:
            info = tarfile.TarInfo(link)
            info.type = tarfile.SYMTYPE
            info.linkname = "/etc/passwd"
            tar.addfile(info)
    return buf.getvalue()


class ArtifactTests(unittest.TestCase):
    def test_members_and_byte_identity(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            art = Path(tmp) / "a.crate"
            art.write_bytes(_tgz({"x-0.1.0/assurance_report.json": b"R", "x-0.1.0/ASSURANCE.md": b"A",
                                  "x-0.1.0/LICENSE": b"MIT License", "x-0.1.0/src/lib.rs": b""}))
            members = rg.artifact_members(art)
            self.assertIn("src/lib.rs", members)
            self.assertEqual(rg.check_artifact(members, b"R", b"A")["status"], "pass")
            self.assertEqual(rg.check_artifact(members, b"R2", b"A")["status"], "fail")
            self.assertEqual(rg.check_artifact(members, b"R", b"stale")["status"], "fail")

    def test_unsafe_members_refused(self) -> None:
        for data in (_tgz({"package/../../evil": b"x"}), _tgz({"/abs": b"x"}),
                     _tgz({"package/a": b"x"}, link="package/link")):
            with tempfile.TemporaryDirectory() as tmp:
                art = Path(tmp) / "a.tgz"
                art.write_bytes(data)
                with self.assertRaises(rg.GateError):
                    rg.artifact_members(art)


class ReviewTests(unittest.TestCase):
    def _setup(self, tmp: Path) -> tuple[Path, str]:
        repo = _init_repo(tmp)
        pkg = "code/packages/rust/demo"
        _commit(repo, {f"{pkg}/src/lib.rs": "fn a() {}\n", f"{pkg}/Cargo.toml": "[package]\n",
                       f"{pkg}/README.md": "r\n"})
        return repo, pkg

    def _record(self, repo: Path, pkg: str, paths: list[str], **over) -> None:
        doc = {"schema": rg.REVIEW_SCHEMA, "outcome": "pass", "reviewer": "t", "date": "2026-11-23",
               "findings": [], "files": {f: rg.review_digest(repo, f) for f in paths}}
        doc.update(over)
        (repo / pkg / "security-review.json").write_text(json.dumps(doc), "utf-8")

    def test_scope_is_computed_and_must_be_fresh(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo, pkg = self._setup(Path(tmp))
            required = rg.required_review_scope(repo, pkg, "rust", ["src/lib.rs", "README.md",
                                                                   "Cargo.toml.orig", "security-review.json"])
            self.assertEqual(required, [f"{pkg}/Cargo.toml", f"{pkg}/README.md", f"{pkg}/src/lib.rs"])
            self._record(repo, pkg, required)
            self.assertEqual(rg.check_review(repo, pkg, required)["status"], "pass")
            (repo / pkg / "src/lib.rs").write_text("fn b() {}\n", "utf-8")
            result = rg.check_review(repo, pkg, required)
            self.assertEqual(result["status"], "fail")
            self.assertIn("changed since review", result["detail"])

    def test_narrow_scope_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo, pkg = self._setup(Path(tmp))
            required = rg.required_review_scope(repo, pkg, "rust", ["src/lib.rs", "README.md"])
            self._record(repo, pkg, [f"{pkg}/src/lib.rs"])
            self.assertIn("does not cover", rg.check_review(repo, pkg, required)["detail"])

    def test_bad_records_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo, pkg = self._setup(Path(tmp))
            required = [f"{pkg}/src/lib.rs"]
            self.assertEqual(rg.check_review(repo, pkg, required)["status"], "fail")  # missing
            for over in ({"outcome": "fail"}, {"schema": "x"}, {"files": []},
                         {"findings": [{"id": "S1", "severity": "HIGH", "status": "accepted"}]}):
                self._record(repo, pkg, required, **over)
                self.assertEqual(rg.check_review(repo, pkg, required)["status"], "fail", over)
            self._record(repo, pkg, required)
            doc = json.loads((repo / pkg / "security-review.json").read_text())
            doc["files"]["../escape"] = "0" * 64
            (repo / pkg / "security-review.json").write_text(json.dumps(doc))
            self.assertIn("unsafe scope path", rg.check_review(repo, pkg, required)["detail"])


class EndToEndTests(unittest.TestCase):
    def test_real_ct_compare_dry_run(self) -> None:
        ctx = rg.Context(repo=REPO, pkg_rel="code/packages/rust/ct-compare", lang="rust", version="0.1.0",
                         dry_run=True, signers_ref="HEAD")
        gate = rg.run_check(ctx)
        self.assertEqual(gate["claim_tier"], "bounded(128)")
        self.assertEqual(gate["release_tier"], "tested")
        self.assertIn(gate["decision"], ("refused", "incomplete"))
        notes = rg.render_notes(gate)
        self.assertIn("Why the release tier is Tested", notes)
        self.assertIn("`ct_eq_u64_soundness`", notes)

    def test_version_major_needs_proved(self) -> None:
        ctx = rg.Context(repo=REPO, pkg_rel="code/packages/rust/ct-compare", lang="rust", version="1.0.0",
                         dry_run=True, signers_ref="HEAD")
        floor = {f["id"]: f["status"] for f in rg.run_check(ctx)["floor"]}
        self.assertEqual(floor["F11b"], "fail")
        self.assertEqual(floor["F11a"], "fail")  # manifest says 0.1.0

    def test_assurance_rendering_is_deterministic(self) -> None:
        sha = rg.sha256_hex((CT / "assurance_report.json").read_bytes())
        claims = [rg.claim_rung(c, BUCKETS, set()) for c in REPORT["claims"]]
        first = rg.render_assurance("n", claims, REPORT, sha, BUCKETS, None)
        self.assertEqual(first, rg.render_assurance("n", claims, REPORT, sha, BUCKETS, None))
        self.assertIn("### (c) own gap: 20 ids", first)
        self.assertIn("`phys.silicon.realization` (floor", first)

    def test_cli_refuses_bad_package(self) -> None:
        self.assertEqual(rg.main(["check", "--package", "../etc", "--version", "0.1.0", "--out", "/dev/null"]), 2)
        self.assertEqual(rg.main(["check", "--package", "code/packages/rust/ct-compare", "--version", "x",
                                  "--out", "/dev/null"]), 2)


if __name__ == "__main__":
    unittest.main()


def _mini_repo(tmp: Path) -> tuple[Path, str]:
    """A git repo holding the real catalogue, buckets and ct-compare package files
    the gate reads, with the floor gaps closed: licence, ASSURANCE.md."""
    repo = _init_repo(tmp)
    pkg = "code/packages/rust/ct-compare"
    files = {}
    # Only files whose shape the gate reads are copied from the real package;
    # the changelog is written here, so converting the real one to CHANGELOG.d
    # (or any later edit to it) cannot break this test.
    for rel in (rg.BUCKETS_REL, rg.CATALOGUE_REL, f"{pkg}/assurance_report.json", f"{pkg}/README.md",
                f"{pkg}/evidence/bench-manifest.json", f"{pkg}/src/lib.rs",
                "code/specs/ct-compare.warrant"):
        files[rel] = (REPO / rel).read_text("utf-8")
    files[f"{pkg}/Cargo.toml"] = '[package]\nname = "coding_adventures_ct_compare"\nversion = "0.1.0"\nlicense = "MIT"\n'
    files[f"{pkg}/LICENSE"] = "MIT License\n"
    files[f"{pkg}/CHANGELOG.md"] = "# Changelog\n\n## 0.1.0\n\n- First release.\n"
    _commit(repo, files)
    return repo, pkg


class PipelineTests(unittest.TestCase):
    def test_full_dry_run_passes_only_with_every_floor_check(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo, pkg = _mini_repo(Path(tmp))
            self.assertEqual(rg.main(["--repo-root", str(repo), "assurance", "--package", pkg, "--check"]), 1)
            self.assertEqual(rg.main(["--repo-root", str(repo), "assurance", "--package", pkg]), 0)
            self.assertEqual(rg.main(["--repo-root", str(repo), "assurance", "--package", pkg, "--check"]), 0)
            _commit(repo, {f"{pkg}/ASSURANCE.md": (repo / pkg / "ASSURANCE.md").read_text("utf-8")})
            members = {f"x-0.1.0/{name}": (repo / pkg / name).read_bytes()
                       for name in ("assurance_report.json", "ASSURANCE.md", "LICENSE", "src/lib.rs",
                                    "README.md", "CHANGELOG.md", "Cargo.toml")}
            art = Path(tmp) / "x.crate"
            art.write_bytes(_tgz(members))
            required = rg.required_review_scope(repo, pkg, "rust", sorted(rg.artifact_members(art)))
            review = {"schema": rg.REVIEW_SCHEMA, "outcome": "pass", "reviewer": "t", "date": "2026-11-23",
                      "findings": [], "files": {f: rg.review_digest(repo, f) for f in required}}
            (repo / pkg / "security-review.json").write_text(json.dumps(review), "utf-8")
            lcov = Path(tmp) / "lcov.info"
            lcov.write_text(f"SF:{repo}/{pkg}/src/lib.rs\nLF:100\nLH:93\nend_of_record\n", "utf-8")
            out = Path(tmp) / "gate.json"
            argv = ["--repo-root", str(repo), "check", "--package", pkg, "--version", "0.1.0",
                    "--out", str(out), "--signers-ref", "HEAD", "--artifact", str(art), "--lcov", str(lcov),
                    "--dry-run", "--tests-ran", "--lint-ran", "--report-check-ran",
                    "--evidence-reproduced", "--registry-has-version", "no", "--trusted-publishing"]
            self.assertEqual(rg.main(argv), 0)
            gate = json.loads(out.read_text("utf-8"))
            self.assertEqual(gate["decision"], "dry-run-pass", [f for f in gate["floor"] if f["status"] != "pass"])
            self.assertEqual(gate["release_tier"], "tested")
            self.assertIn("93.0%", {f["id"]: f["detail"] for f in gate["floor"]}["F2"])
            # Without --dry-run the missing signers file refuses the release.
            self.assertEqual(rg.main([a for a in argv if a != "--dry-run"]), 1)
            notes = Path(tmp) / "notes.md"
            self.assertEqual(rg.main(["notes", "--gate", str(out), "--out", str(notes)]), 0)
            self.assertIn("release tier Tested", notes.read_text("utf-8"))
            # Low coverage fails F2; a registry that already has the version fails F11c.
            lcov.write_text(f"SF:{repo}/{pkg}/src/lib.rs\nLF:100\nLH:79\nend_of_record\n", "utf-8")
            argv2 = [*argv[:-3], "--registry-has-version", "yes", "--trusted-publishing", "--report-only"]
            self.assertEqual(rg.main(argv2), 0)
            failed = {f["id"] for f in json.loads(out.read_text())["floor"] if f["status"] == "fail"}
            self.assertEqual(failed, {"F2", "F11c"})

    def test_notes_refuses_other_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bad = Path(tmp) / "g.json"
            bad.write_text("{}", "utf-8")
            self.assertEqual(rg.main(["notes", "--gate", str(bad), "--out", str(Path(tmp) / "n")]), 2)

    def test_verify_tag_cli_refuses_without_signers(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            sha = _commit(repo, {"a": "1"})
            self.assertEqual(rg.main(["--repo-root", str(repo), "verify-tag", "--tag", "rust/x/v0.1.0",
                                      "--commit", sha, "--signers-ref", "HEAD"]), 2)


class BucketSignatureTests(unittest.TestCase):
    def test_rollback_refused_before_signature(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            doc = copy.deepcopy(RAW_BUCKETS)
            doc["revision"] = 2
            _commit(repo, {rg.BUCKETS_REL: json.dumps(doc), rg.CATALOGUE_REL: (REPO / rg.CATALOGUE_REL).read_text()})
            _commit(repo, {rg.BUCKETS_REL: json.dumps(RAW_BUCKETS)})
            current = rg.load_buckets(repo / rg.BUCKETS_REL, repo / rg.CATALOGUE_REL)
            with self.assertRaises(rg.GateError) as ctx:
                rg.verify_buckets_signature(repo, "HEAD", current, "")
            self.assertIn("lower than 2", ctx.exception.message)

    def test_unsigned_bucket_file_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            _commit(repo, {rg.BUCKETS_REL: json.dumps(RAW_BUCKETS),
                           rg.CATALOGUE_REL: (REPO / rg.CATALOGUE_REL).read_text()})
            current = rg.load_buckets(repo / rg.BUCKETS_REL, repo / rg.CATALOGUE_REL)
            with self.assertRaises(rg.GateError) as ctx:
                rg.verify_buckets_signature(repo, "HEAD", current, "")
            self.assertIn("not signed", ctx.exception.message)
            (repo / (rg.BUCKETS_REL + ".sig")).write_text(_sshsig(SK, rg.NS_SIGNERS, 0x05), "utf-8")
            with self.assertRaises(rg.GateError) as ctx:  # wrong namespace
                rg.verify_buckets_signature(repo, "HEAD", current, f"# revision: 1\nowner {SK} AAAA\n")
            self.assertIn("namespace", ctx.exception.message)


class SignoffTests(unittest.TestCase):
    def test_defective_signoffs_close_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            warrant = "code/specs/demo.warrant"
            _commit(repo, {warrant: "claim x\n"})
            signers = f"# revision: 1\nowner {SK} AAAA\n"
            self.assertEqual(rg.verify_signoff(repo, warrant, signers), set())  # no record
            rec = repo / "code/specs/signoffs/demo.json"
            rec.parent.mkdir(parents=True)
            good = {"schema": rg.SIGNOFF_SCHEMA, "warrant": warrant, "signer": "owner",
                    "warrant_sha256": rg.sha256_hex(b"claim x\n"), "ids": ["spec.secret-labels.complete"]}
            for doc, sig in (
                (good, _sshsig(SK, rg.NS_STATEMENT, 0x05)),  # signature does not verify
                (good, _sshsig(SK, rg.NS_TAG, 0x05)),  # wrong namespace
                (good, _sshsig(SK, rg.NS_STATEMENT, 0x01)),  # no user verification
                ({**good, "warrant": "code/specs/other.warrant"}, _sshsig(SK, rg.NS_STATEMENT, 0x05)),
                ({**good, "warrant_sha256": "0" * 64}, _sshsig(SK, rg.NS_STATEMENT, 0x05)),
                ({**good, "schema": "x"}, _sshsig(SK, rg.NS_STATEMENT, 0x05)),
                (good, "garbage"),
            ):
                rec.write_text(json.dumps(doc), "utf-8")
                rec.with_name("demo.json.sig").write_text(sig, "utf-8")
                self.assertEqual(rg.verify_signoff(repo, warrant, signers), set(), doc)


class StubbedCryptoTests(unittest.TestCase):
    """Success paths that need a FIDO2 signature CI cannot make.

    Only ``ssh_verify`` (the call into ``ssh-keygen``, whose cryptography is
    tested above with a software key) is stubbed; the namespace, key-type and
    flag checks around it still run on hand-built security-key signatures.
    """

    def setUp(self) -> None:
        patcher = unittest.mock.patch.object(rg, "ssh_verify", return_value=True)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_signed_chain_transition_and_bucket_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 1\nowner {SK} AAAA\n"})
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 2\nowner {SK} AAAA\nci {SK} BBBB\n",
                           rg.SIGNERS_REL + ".sig": _sshsig(SK, rg.NS_SIGNERS, 0x05)})
            text, entries = rg.verify_signers_chain(repo, "HEAD")
            self.assertEqual([e["principal"] for e in entries], ["owner", "ci"])
            _commit(repo, {rg.BUCKETS_REL: json.dumps(RAW_BUCKETS),
                           rg.CATALOGUE_REL: (REPO / rg.CATALOGUE_REL).read_text(),
                           rg.BUCKETS_REL + ".sig": _sshsig(SK, rg.NS_BUCKETS, 0x05)})
            current = rg.load_buckets(repo / rg.BUCKETS_REL, repo / rg.CATALOGUE_REL)
            self.assertEqual(rg.verify_buckets_signature(repo, "HEAD", current, text), "owner")

    def test_valid_signoff_closes_b_and_can_reach_proved(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            warrant = "code/specs/demo.warrant"
            doc = {"schema": rg.SIGNOFF_SCHEMA, "warrant": warrant, "signer": "owner",
                   "warrant_sha256": rg.sha256_hex(b"claim x\n"), "ids": ["spec.secret-labels.complete"]}
            _commit(repo, {warrant: "claim x\n", "code/specs/signoffs/demo.json": json.dumps(doc),
                           "code/specs/signoffs/demo.json.sig": _sshsig(SK, rg.NS_STATEMENT, 0x05)})
            signed = rg.verify_signoff(repo, warrant, f"# revision: 1\nowner {SK} AAAA\n")
            self.assertEqual(signed, {"spec.secret-labels.complete"})
            c = proved_claim(["cc.version-pinned", "spec.secret-labels.complete"])
            self.assertEqual(rg.claim_rung(c, with_catalogue_discharged(), signed)["rung"], "proved")

    def test_pipeline_reports_signers_anchor_and_lane(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo, _pkg = _mini_repo(Path(tmp))
            _commit(repo, {rg.SIGNERS_REL: f"# revision: 1\nowner {SK} AAAAkeymaterial0123456789\n"})
            gate = rg.run_check(rg.Context(repo=repo, pkg_rel="code/packages/rust/ct-compare", lang="rust",
                                           version="0.1.0", dry_run=True, signers_ref="HEAD"))
            auth = {a["id"]: a for a in gate["authorization"]}
            self.assertEqual(auth["A1"]["status"], "pass")
            self.assertIn("anchor", auth["A1"]["detail"])
            self.assertEqual(auth["A2"]["status"], "fail")  # bucket file not signed
            self.assertEqual(auth["A3"]["status"], "not-run")

    def test_lane_package_f5_needs_its_own_suites(self) -> None:
        gate = rg.run_check(rg.Context(repo=REPO, pkg_rel="code/packages/typescript/ct-compare",
                                       lang="typescript", version="0.2.0", dry_run=True, signers_ref="HEAD"))
        self.assertEqual(gate["lane"], "typescript/ct-compare")
        self.assertEqual(gate["claim_tier"], "tested")
        f5 = {f["id"]: f for f in gate["floor"]}["F5"]
        with tempfile.TemporaryDirectory() as tmp:
            pkg = Path(tmp)
            (pkg / "package.json").write_text(json.dumps(
                {"name": "x", "version": "0.2.0", "scripts": {"test:property": "a", "test:fuzz": "b"}}))
            checks = {c["id"]: c for c in rg.floor_static(pkg, "typescript", "0.2.0", REPORT, True)}
        self.assertEqual(checks["F5"]["status"], "pass")
        self.assertIn(f5["status"], ("pass", "fail"))


class WorkspaceSettingsTests(unittest.TestCase):
    def test_member_churn_does_not_stale_a_review_but_settings_do(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            ws = "code/packages/rust/Cargo.toml"
            pkg = "code/packages/rust/demo"
            _commit(repo, {ws: '[workspace]\nmembers = [\n  "demo",\n]\nresolver = "2"\n',
                           f"{pkg}/src/lib.rs": "fn a() {}\n", f"{pkg}/Cargo.toml": "[package]\n"})
            required = rg.required_review_scope(repo, pkg, "rust", ["src/lib.rs"])
            self.assertIn(ws + rg.SETTINGS_SUFFIX, required)
            self.assertNotIn("code/packages/rust/Cargo.lock", required)
            doc = {"schema": rg.REVIEW_SCHEMA, "outcome": "pass", "reviewer": "t", "date": "d", "findings": [],
                   "files": {f: rg.review_digest(repo, f) for f in required}}
            (repo / pkg / "security-review.json").write_text(json.dumps(doc), "utf-8")
            (repo / ws).write_text('[workspace]\nmembers = [\n  "demo",\n  "other",\n]\nresolver = "2"\n')
            self.assertEqual(rg.check_review(repo, pkg, required)["status"], "pass")
            (repo / ws).write_text('[workspace]\nmembers = ["demo"]\nresolver = "2"\n[patch.crates-io]\nx = { path = "x" }\n')
            self.assertEqual(rg.check_review(repo, pkg, required)["status"], "fail")


class ReviewRoundTwoTests(unittest.TestCase):
    """Rules added after the LV-007a security review."""

    def test_signed_tag_object_cannot_be_replayed_under_another_ref(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            sha = _commit(repo, {"a": "1"})
            _git(repo, "tag", "-a", "-m", "m", "rust/real/v0.1.0", sha)
            obj = _git(repo, "rev-parse", "refs/tags/rust/real/v0.1.0").strip()
            _git(repo, "update-ref", "refs/tags/rust/other/v9.9.9", obj)
            with self.assertRaises(rg.GateError) as ctx:
                rg.verify_tag(repo, "rust/other/v9.9.9", sha, "")
            self.assertIn("do not name", ctx.exception.message)

    def test_warrant_comes_only_from_the_spec_directive(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / "code/specs").mkdir(parents=True)
            (repo / "code/specs/a.warrant").write_text("x")
            pkg = repo / "code/packages/rust/a"
            pkg.mkdir(parents=True)
            report = pkg / "assurance_report.json"
            (pkg / "evidence.bindings").write_text("# see code/specs/a.warrant\n")
            self.assertIsNone(rg.warrant_path(repo, report))
            (pkg / "evidence.bindings").write_text("spec code/specs/a.warrant\n")
            self.assertEqual(rg.warrant_path(repo, report), "code/specs/a.warrant")
            (pkg / "evidence.bindings").write_text("spec code/specs/a.warrant\nspec code/specs/a.warrant\n")
            self.assertIsNone(rg.warrant_path(repo, report))
        self.assertEqual(rg.warrant_path(REPO, CT / "assurance_report.json"), "code/specs/ct-compare.warrant")

    def test_h1_needs_warrant_derived_evidence(self) -> None:
        self.assertTrue(rg.h1_holds(REPORT, "code/specs/ct-compare.warrant"))
        self.assertFalse(rg.h1_holds(REPORT, None))
        doc = copy.deepcopy(REPORT)
        doc["claims"][0]["evidence"].append("handwritten/test")
        self.assertFalse(rg.h1_holds(doc, "code/specs/ct-compare.warrant"))

    def test_shipped_manifest_must_match_tree_and_tag(self) -> None:
        tree = rg.read_manifest(CT, "rust")
        good = b'[package]\nname = "coding_adventures_ct_compare"\nversion = "0.1.0"\nlicense = "MIT"\n'
        self.assertEqual(rg.check_shipped_manifest({"Cargo.toml": good}, "rust", tree, "0.1.0")["status"], "pass")
        for bad in (good.replace(b"0.1.0", b"0.9.0"), good.replace(b"MIT", b"GPL-3.0"),
                    good.replace(b"coding_adventures_ct_compare", b"other"),
                    good + b'[dependencies]\nserde = "1"\n', b"not toml ["):
            self.assertEqual(rg.check_shipped_manifest({"Cargo.toml": bad}, "rust", tree, "0.1.0")["status"],
                             "fail", bad)
        self.assertEqual(rg.check_shipped_manifest({}, "rust", tree, "0.1.0")["status"], "fail")
        npm = rg.Manifest("typescript", "@x/y", "0.2.0", "MIT", [])
        pj = json.dumps({"name": "@x/y", "version": "0.2.0", "license": "MIT",
                         "dependencies": {"left-pad": "1"}}).encode()
        self.assertEqual(rg.check_shipped_manifest({"package.json": pj}, "typescript", npm, "0.2.0")["status"],
                         "fail")

    def test_review_scope_covers_package_tree_and_ancestor_build_config(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            pkg = "code/packages/rust/demo"
            _commit(repo, {f"{pkg}/src/lib.rs": "x", f"{pkg}/tests/t.rs": "x", f"{pkg}/BUILD": "x",
                           ".cargo/config.toml": "x", "code/packages/rust/.cargo/config.toml": "x",
                           "rust-toolchain.toml": "x", f"{pkg}/security-review.json": "{}",
                           "code/other/file": "x"})
            required = rg.required_review_scope(repo, pkg, "rust", [])
            for want in (f"{pkg}/src/lib.rs", f"{pkg}/tests/t.rs", f"{pkg}/BUILD", ".cargo/config.toml",
                         "code/packages/rust/.cargo/config.toml", "rust-toolchain.toml"):
                self.assertIn(want, required)
            self.assertNotIn(f"{pkg}/security-review.json", required)
            self.assertNotIn("code/other/file", required)

    def test_signoff_must_name_the_package(self) -> None:
        with unittest.mock.patch.object(rg, "ssh_verify", return_value=True), tempfile.TemporaryDirectory() as tmp:
            repo = _init_repo(Path(tmp))
            warrant = "code/specs/demo.warrant"
            doc = {"schema": rg.SIGNOFF_SCHEMA, "warrant": warrant, "signer": "owner", "package": "rust/demo",
                   "warrant_sha256": rg.sha256_hex(b"claim x\n"), "ids": ["spec.secret-labels.complete"]}
            _commit(repo, {warrant: "claim x\n", "code/specs/signoffs/demo.json": json.dumps(doc),
                           "code/specs/signoffs/demo.json.sig": _sshsig(SK, rg.NS_STATEMENT, 0x05)})
            signers = f"# revision: 1\nowner {SK} AAAA\n"
            self.assertEqual(rg.verify_signoff(repo, warrant, signers, "rust/demo"), {"spec.secret-labels.complete"})
            self.assertEqual(rg.verify_signoff(repo, warrant, signers, "rust/other"), set())

    def test_exemption_text_is_escaped_or_refused(self) -> None:
        doc = copy.deepcopy(REPORT)
        doc["exemptions"][0]["id"] = "X\n## H2 holds"
        with self.assertRaises(rg.GateError):
            rg.higher_tier_conditions(doc, True, False)

    def test_bucket_file_cannot_move_tool_trust_out_of_c(self) -> None:
        doc = copy.deepcopy(RAW_BUCKETS)
        for e in doc["assumptions"]:
            if e["id"] == "tcb.report.checker-correct":
                e["bucket"] = "a"
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(rg.GateError):
            rg.load_buckets(write_buckets(Path(tmp), doc), REPO / rg.CATALOGUE_REL)


class WorkflowHygieneTests(unittest.TestCase):
    """Rules about publish.yml that a reviewer could miss in a later edit."""

    WORKFLOW = (REPO / ".github" / "workflows" / "publish.yml").read_text("utf-8")

    def test_every_python_call_is_isolated(self) -> None:
        # Without -I, python3 puts the checkout first on sys.path, so a planted
        # json.py / tomllib.py runs inside the judge and the publish jobs.
        bad = [line.strip() for line in self.WORKFLOW.splitlines()
               if re.search(r"\bpython3?\s", line) and not re.search(r"\bpython3?\s+-I\b", line)
               and not line.strip().startswith("#")]
        self.assertEqual(bad, [])

    def test_no_expression_inside_run_scripts(self) -> None:
        in_run = False
        offenders = []
        for line in self.WORKFLOW.splitlines():
            stripped = line.strip()
            if stripped.startswith("run:"):
                in_run = True
                indent = len(line) - len(line.lstrip())
                if "${{" in stripped:
                    offenders.append(stripped)
                continue
            if in_run:
                if stripped and len(line) - len(line.lstrip()) <= indent:
                    in_run = False
                elif "${{" in line:
                    offenders.append(stripped)
        self.assertEqual(offenders, [])

    def test_workspace_settings_ignore_members_only(self) -> None:
        base = '[workspace]\nmembers = ["a"] # c\nresolver = "2"\n'
        moved = '[workspace]\nmembers = ["a", "b"]\nresolver = "2"\n'
        sneaky = '[workspace]\nmembers = ["a"] # c\n[profile.release] # x\nlto = true\n[x]\ny = [1]\n'
        self.assertEqual(rg.workspace_settings(base), rg.workspace_settings(moved))
        self.assertNotEqual(rg.workspace_settings(base), rg.workspace_settings(sneaky))
