"""Tests for code/scripts/logic_gates_basic_drift.py (LG02 section 4, LGE-S1b).

The script takes no path arguments, so every negative test builds a temporary
repository tree (the same six repo-relative paths the script reads), edits one
file, and calls ``drift.run(tmp_root)``, the function the command line also
uses with its realpath-derived root.  The positive test runs the real tree
through the real command line.
"""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
import unittest.mock

SCRIPTS = pathlib.Path(__file__).resolve().parents[1]
REPO = SCRIPTS.parents[1]
sys.path.insert(0, str(SCRIPTS))

import logic_gates_basic_drift as drift

COPIED = [drift.FULL_SPEC, drift.BASIC_SPEC, *drift.BUILD_FILES, drift.BINDINGS]


def _write(root: str, rel: str, data: bytes) -> None:
    path = os.path.join(root, *rel.split("/"))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as handle:
        handle.write(data)


def _read(root: str, rel: str) -> bytes:
    with open(os.path.join(root, *rel.split("/")), "rb") as handle:
        return handle.read()


def _fixture(root: str) -> None:
    """Synthesise a consistent fixture directory from the copied BUILD and bindings."""
    bindings = drift._parse_bindings(_read(root, drift.BINDINGS).decode())
    pins = drift._parse_pins(_read(root, drift.BUILD_FILE).decode())
    profile = "".join(f"fault {op} {fault}\n" for op, fault in sorted(bindings))
    controls = [
        {
            "op": op,
            "fault": fault,
            "required_claims": [f"warrant/logic_gates_basic/exhaustive/{claim}"],
        }
        for op, fault, claim in pins
    ]
    digest = hashlib.sha256(_read(root, drift.BASIC_SPEC)).hexdigest()
    _write(root, drift.PROFILE, profile.encode())
    _write(
        root,
        drift.CORPUS,
        json.dumps({"controls": controls, "spec_sha256": digest}).encode(),
    )


class Tree(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = os.path.realpath(self._tmp.name)
        for rel in COPIED:
            _write(self.root, rel, (REPO / rel).read_bytes())
        # The shipped flag is True (the fixture exists).  The synthetic trees below exercise the
        # pre-fixture skip logic and the fail-closed paths, so they start in the False mode; the
        # tests that need "required" patch it back, and Fixture trees add a fixture.
        patcher = unittest.mock.patch.object(drift, "FIXTURE_REQUIRED", False)
        patcher.start()
        self.addCleanup(patcher.stop)

    # -- helpers ----------------------------------------------------------
    def run_drift(self, git=None):
        # The temporary tree is not a git repository: stub the tracked-files query.
        return drift.run(self.root, git_ls_files=git or (lambda root, rel: []))

    def symlink(self, target: str, link: str) -> None:
        """Create a symlink; skip (loudly) only where the OS lacks the privilege."""
        try:
            os.symlink(target, link)
        except NotImplementedError as exc:
            print(f"SKIPPED: os.symlink not implemented here ({exc})", file=sys.stderr)
            self.skipTest("os.symlink not implemented")
        except OSError as exc:
            if os.name == "nt":
                print(
                    f"SKIPPED: no symlink privilege on Windows ({exc})", file=sys.stderr
                )
                self.skipTest("no symlink privilege")
            self.fail(f"os.symlink failed on a platform where symlinks work: {exc}")

    def basic(self) -> str:
        return _read(self.root, drift.BASIC_SPEC).decode()

    def set_basic(self, text) -> None:
        _write(
            self.root,
            drift.BASIC_SPEC,
            text if isinstance(text, bytes) else text.encode(),
        )

    def replace_basic(self, old: str, new: str, count: int = 1) -> None:
        text = self.basic()
        self.assertIn(old, text)
        self.set_basic(text.replace(old, new, count))

    def fails(self, *needles: str) -> str:
        with self.assertRaises(drift.DriftError) as ctx:
            self.run_drift()
        for needle in needles:
            self.assertIn(needle, str(ctx.exception))
        return str(ctx.exception)


class RealTree(unittest.TestCase):
    def test_real_repository_passes_through_the_command_line(self) -> None:
        proc = subprocess.run(
            [sys.executable, str(SCRIPTS / "logic_gates_basic_drift.py")],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("documents match", proc.stdout)

    def test_the_flag_is_true_and_the_fixture_exists(self) -> None:
        self.assertTrue(drift.FIXTURE_REQUIRED)
        self.assertTrue(os.path.isdir(os.path.join(REPO, *drift.FIXTURE_DIR.split("/"))))

    def test_command_line_refuses_arguments(self) -> None:
        proc = subprocess.run(
            [
                sys.executable,
                str(SCRIPTS / "logic_gates_basic_drift.py"),
                "/etc/passwd",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(proc.returncode, 2)

    def test_expected_counts(self) -> None:
        full = drift.parse_document(
            drift.FULL_SPEC, (REPO / drift.FULL_SPEC).read_bytes()
        )
        keys = drift.expected_keys(full)
        counts = {k: len(v) for k, v in keys.items()}
        self.assertEqual(
            counts,
            {
                "error": 1,
                "assume": 2,
                "model": 1,
                "op": 11,
                "labels": 11,
                "input": 20,
                "bound": 11,
                "example": 39,
                "claim": 26,
            },
        )


class Documents(Tree):
    def test_unmodified_copy_passes(self) -> None:
        notes = self.run_drift()
        self.assertTrue(
            any("skipped" in n for n in notes),
            "fixture absent -> checks 10-12 say skipped",
        )

    def test_one_byte_drift_in_every_block_kind_fails(self) -> None:
        # One edit per block kind, each a single byte inside the copied block.
        edits = {
            "error": ("INVALID_INPUT\n", "INVALID_INPUS\n"),
            "assume": ("assume platform:\n    none", "assume platform:\n    nonf"),
            "model": ("then 1u8 else", "then 1u8 elsf"),
            "op": ("op not_gate(a: U8)", "op not_gate(a: U9)"),
            "labels": (
                "labels and_gate:\n    a public",
                "labels and_gate:\n    a publid",
            ),
            "input": (
                "input and_gate.b:\n    hostile",
                "input and_gate.b:\n    hostilf",
            ),
            "bound": (
                "bound or_gate:\n    steps <= 1024",
                "bound or_gate:\n    steps <= 1025",
            ),
            "example": (
                "and_gate(1u8, 1u8) == Ok(1u8)",
                "and_gate(1u8, 1u8) == Ok(0u8)",
            ),
            "claim": ("Ok(b2u(a != b))", "Ok(b2u(a == b))"),
        }
        original = self.basic()
        for kind, (old, new) in edits.items():
            with self.subTest(kind=kind):
                self.assertIn(old, original)
                self.set_basic(original.replace(old, new, 1))
                self.fails(kind + " block")
        self.set_basic(original)
        self.run_drift()

    def test_one_byte_change_inside_a_copied_claim(self) -> None:
        self.replace_basic(
            "a > 1u8 or b > 1u8. and_gate", "a > 1u8 or b > 2u8. and_gate"
        )
        self.fails("claim block and_rejects_non_bit")

    def test_message_names_one_block_not_the_files(self) -> None:
        self.replace_basic("Ok(b2u(a != b))", "Ok(b2u(a == b))")
        self.assertLess(len(self.fails("claim block xor_truth_table")), 600)

    def test_indented_comment_line_changes_nothing(self) -> None:
        self.replace_basic(
            "  op not_gate", "      -- an indented comment\n  op not_gate"
        )
        self.run_drift()
        self.replace_basic(
            "a public\n  labels and_gate",
            "a public -- trailing comment\n  labels and_gate",
        )
        self.run_drift()

    def test_stray_column_zero_line_fails(self) -> None:
        self.set_basic(self.basic() + "stray\n")
        self.fails("column-0")

    def test_two_space_line_with_unknown_keyword_fails(self) -> None:
        self.set_basic(self.basic() + "  frobnicate x\n")
        self.fails("starts no block")

    def test_deep_line_with_no_open_block_fails(self) -> None:
        text = self.basic().replace(
            "warrant logic_gates_basic v1\n",
            "warrant logic_gates_basic v1\n    orphan\n",
            1,
        )
        self.set_basic(text)
        self.fails("no open block")

    def test_odd_indentation_fails(self) -> None:
        self.replace_basic(
            "    none\n  assume environment", "     none\n  assume environment"
        )
        self.fails("multiple of two")

    def test_tab_cr_bom_and_missing_final_lf_fail(self) -> None:
        text = self.basic()
        cases = {
            "tab": text.replace("a public", "a\tpublic", 1),
            "cr": text.replace("\n", "\r\n", 1),
            "bom": "\ufeff" + text,
            "final lf": text[:-1],
        }
        for name, bad in cases.items():
            with self.subTest(name):
                self.set_basic(bad)
                self.fails()
        self.set_basic(b"\xff\xfe" + text.encode())
        self.fails()
        self.set_basic(text.encode() + b"\xc3\x28\n")
        self.fails()

    def test_plain_quoted_string_with_dash_dash_is_refused(self) -> None:
        self.replace_basic("  op not_gate(a: U8)", '  op not_gate(a: U8) "x -- y"')
        self.fails("string literal")

    def test_any_non_bytes_quote_is_refused(self) -> None:
        self.replace_basic(
            "not_gate(0u8) == Ok(1u8)", 'not_gate(0u8) == Ok(1u8) and "x"'
        )
        self.fails("string literal")

    def test_quote_inside_a_comment_is_ignored(self) -> None:
        self.replace_basic("  op not_gate", '  -- a "quoted" comment\n  op not_gate')
        self.run_drift()

    def test_one_byte_difference_inside_a_bytes_literal_fails(self) -> None:
        # The basic eleven have no BYTES_LITERAL, so plant the same literal in both
        # documents and then change one hex digit in the copy only.
        full = (REPO / drift.FULL_SPEC).read_text()
        anchor = "not_gate(1u8) == Ok(0u8)"
        planted = anchor + ' and b"0a0b" == b"0a0b"'
        _write(self.root, drift.FULL_SPEC, full.replace(anchor, planted, 1).encode())
        self.set_basic(self.basic().replace(anchor, planted, 1))
        self.run_drift()
        self.set_basic(
            self.basic().replace('b"0a0b" == b"0a0b"', 'b"0a0b" == b"0a0c"', 1)
        )
        self.fails("example block not_1")

    def test_ab_before_a_literal_is_one_name_not_a_literal(self) -> None:
        self.replace_basic(
            "not_gate(0u8) == Ok(1u8)", 'not_gate(0u8) == Ok(1u8) ab"00"'
        )
        self.fails("string literal")

    def test_double_dash_after_a_non_blank_is_refused(self) -> None:
        self.replace_basic(
            "a public\n  labels and_gate", "a public\n  labels and_gate--x"
        )
        self.fails("'--' directly after")

    def test_missing_extra_and_duplicate_blocks_fail(self) -> None:
        text = self.basic()
        self.set_basic(
            text.replace("  bound nand_xor:\n    steps <= 1024\n    depth <= 64\n", "")
        )
        self.fails("missing bound block nand_xor")
        self.set_basic(text + "  model junk(x: U8) -> U8 = x\n")
        self.fails("extra model block junk")
        self.set_basic(text + "  error GateError = INVALID_INPUT\n")
        self.fails("duplicate error block")

    def test_blocks_out_of_order_fail(self) -> None:
        text = self.basic()
        a = "  labels and_gate:\n    a public\n    b public\n"
        b = "  labels or_gate:\n    a public\n    b public\n"
        self.assertIn(a + b, text)
        self.set_basic(text.replace(a + b, b + a, 1))
        self.fails("out of order")

    def test_non_basic_example_is_refused_even_if_byte_identical(self) -> None:
        full = (REPO / drift.FULL_SPEC).read_text()
        block = full[full.index("  example and_n_four_ones:") :]
        block = block[: block.index("  example and_n_one_zero:")]
        self.set_basic(self.basic() + block)
        self.fails("extra example block and_n_four_ones")

    def test_closure_names_an_outside_operation(self) -> None:
        # An allowed block that mentions a non-basic op is refused by the closure rule
        # when the full document carries the same text (so the copy is byte-identical).
        full = (REPO / drift.FULL_SPEC).read_text()
        anchor = "not_gate(1u8) == Ok(0u8)"
        planted = anchor + ' and and_n(b"01") == Ok(1u8)'
        _write(self.root, drift.FULL_SPEC, full.replace(anchor, planted, 1).encode())
        self.set_basic(self.basic().replace(anchor, planted, 1))
        self.fails("mixes basic and non-basic")

    def test_headers_checked(self) -> None:
        self.set_basic(
            self.basic().replace(
                "warrant logic_gates_basic v1", "warrant logic_gates v1", 1
            )
        )
        self.fails("header")


class Safety(Tree):
    def test_symlinked_file_refused(self) -> None:
        path = os.path.join(self.root, *drift.BASIC_SPEC.split("/"))
        real = path + ".real"
        os.rename(path, real)
        self.symlink(real, path)
        self.fails("real path differs")

    def test_symlinked_parent_directory_refused(self) -> None:
        specs = os.path.join(self.root, "code", "specs")
        other = os.path.join(self.root, "code", "specs_real")
        os.rename(specs, other)
        self.symlink(other, specs)
        self.fails("real path differs")

    def test_oversize_refused(self) -> None:
        self.set_basic(self.basic().encode() + b"\n" * (drift.MAX_BYTES + 1))
        self.fails("larger than")

    def test_exactly_the_cap_is_not_refused_for_size(self) -> None:
        pad = drift.MAX_BYTES - len(self.basic().encode())
        # Trailing blank lines are harmless content; the size rule alone must not fire.
        self.set_basic(self.basic().encode() + b"\n" * pad)
        self.run_drift()

    def test_missing_input_refused(self) -> None:
        os.remove(os.path.join(self.root, *drift.BASIC_SPEC.split("/")))
        self.fails("missing")

    def test_directory_in_place_of_a_file_refused(self) -> None:
        path = os.path.join(self.root, *drift.BASIC_SPEC.split("/"))
        os.remove(path)
        os.mkdir(path)
        self.fails("not a regular file")


class Fixture(Tree):
    def setUp(self) -> None:
        super().setUp()
        _fixture(self.root)

    def test_consistent_fixture_passes(self) -> None:
        notes = self.run_drift()
        self.assertTrue(any("passed" in n for n in notes))

    def test_pin_count_is_31(self) -> None:
        pins = drift._parse_pins(_read(self.root, drift.BUILD_FILE).decode())
        basic = [p for p in pins if p[0] in drift.OPS]
        self.assertEqual(len(basic), 31)

    def test_missing_corpus_once_fixture_exists_fails(self) -> None:
        os.remove(os.path.join(self.root, *drift.CORPUS.split("/")))
        self.fails("missing")

    def test_triple_mismatch_fails(self) -> None:
        # The BUILD pins and_gate/saturate-add to and_truth_table; make the corpus
        # call a different claim required, so the pinned claim is "blind".
        corpus = json.loads(_read(self.root, drift.CORPUS))
        for control in corpus["controls"]:
            if (control["op"], control["fault"]) == ("and_gate", "saturate-add"):
                control["required_claims"] = [
                    "warrant/logic_gates_basic/exhaustive/and_rejects_non_bit"
                ]
        _write(self.root, drift.CORPUS, json.dumps(corpus).encode())
        self.fails("and_truth_table", "and_gate/saturate-add")

    def _blind_corpus(self, op: str, fault: str, claim: str) -> None:
        """Make the corpus call ``claim`` blind (and not required) for ``op``/``fault``."""
        corpus = json.loads(_read(self.root, drift.CORPUS))
        for control in corpus["controls"]:
            if (control["op"], control["fault"]) == (op, fault):
                control["required_claims"] = [
                    "warrant/logic_gates_basic/exhaustive/" + claim + "_other"
                ]
                control["blind_claims"] = ["warrant/logic_gates_basic/exhaustive/" + claim]
        _write(self.root, drift.CORPUS, json.dumps(corpus).encode())

    def test_known_stand_in_blind_pins_are_accepted(self) -> None:
        for op, fault, claim in sorted(drift.KNOWN_STAND_IN_BLIND):
            self._blind_corpus(op, fault, claim)
        notes = self.run_drift()
        self.assertTrue(any("passed" in n for n in notes))

    def test_known_exception_needs_the_corpus_to_list_it_as_blind(self) -> None:
        corpus = json.loads(_read(self.root, drift.CORPUS))
        for control in corpus["controls"]:
            if (control["op"], control["fault"]) == ("not_gate", "shift-mask"):
                control["required_claims"] = ["warrant/logic_gates_basic/exhaustive/x"]
        _write(self.root, drift.CORPUS, json.dumps(corpus).encode())
        self.fails("not_rejects_non_bit", "not_gate/shift-mask")

    def test_any_other_blind_pin_still_fails(self) -> None:
        self._blind_corpus("and_gate", "saturate-add", "and_truth_table")
        self.fails("and_truth_table", "and_gate/saturate-add")

    def test_pin_without_a_control_and_control_without_a_pin_fail(self) -> None:
        corpus = json.loads(_read(self.root, drift.CORPUS))
        dropped = corpus["controls"].pop()
        _write(self.root, drift.CORPUS, json.dumps(corpus).encode())
        msg = self.fails("controls: 30 (op, fault) pairs")
        self.assertIn(dropped["op"], msg)
        corpus["controls"].append(dropped)
        corpus["controls"].append(
            {"op": "and_gate", "fault": "bogus", "required_claims": ["x"]}
        )
        _write(self.root, drift.CORPUS, json.dumps(corpus).encode())
        self.fails("controls: 32 (op, fault) pairs", "and_gate/bogus")

    def test_profile_versus_bindings_fails(self) -> None:
        path = drift.PROFILE
        _write(self.root, path, _read(self.root, path) + b"fault and_gate bogus\n")
        self.fails("corpus.profile: 32 (op, fault) pairs", "and_gate/bogus")

    def test_stale_digest_fails(self) -> None:
        self.replace_basic(
            "  -- ---- claims ----", "  -- ---- the claims ----"
        )  # comment-only edit
        self.fails("spec_sha256")

    def test_fixture_directory_symlink_refused(self) -> None:
        fixture = os.path.join(self.root, *drift.FIXTURE_DIR.split("/"))
        other = fixture + "_real"
        os.rename(fixture, other)
        self.symlink(other, fixture)
        self.fails("real path differs")


class FailClosed(Tree):
    """Review fixes M1-M4, L5-L7: no parser may match nothing and pass."""

    def edit(self, rel: str, fn) -> None:
        _write(self.root, rel, fn(_read(self.root, rel).decode()).encode())

    def pin_lines(self, text: str) -> list:
        return [ln for ln in text.split("\n") if "WARRANT_EVIDENCE_FAULT=" in ln]

    # M1: empty or reformatted parses must fail inside the script
    def test_build_without_pins_fails(self) -> None:
        self.edit(
            drift.BUILD_FILE,
            lambda t: "\n".join(
                l for l in t.split("\n") if "WARRANT_EVIDENCE_FAULT=" not in l
            ),
        )
        self.fails("0 basic-op fault pins")

    def test_empty_build_file_fails(self) -> None:
        _write(self.root, drift.BUILD_FILE, b"")
        self.fails("0 basic-op fault pins")

    def test_reformatted_pin_line_fails(self) -> None:
        self.edit(
            drift.BUILD_FILE,
            lambda t: t.replace(
                "WARRANT_EVIDENCE_FAULT=", "env WARRANT_EVIDENCE_FAULT=", 1
            ),
        )
        self.fails("unrecognised fault pin")

    def test_commented_out_pins_do_not_count(self) -> None:
        self.edit(
            drift.BUILD_FILE,
            lambda t: t.replace(
                "\nWARRANT_EVIDENCE_FAULT=", "\n# WARRANT_EVIDENCE_FAULT=", 1
            ),
        )
        self.fails("30 basic-op fault pins")

    def test_trailing_comment_or_noop_pin_is_rejected(self) -> None:
        lines = _read(self.root, drift.BUILD_FILE).decode().split("\n")
        i = next(n for n, ln in enumerate(lines) if "WARRANT_EVIDENCE_FAULT=" in ln)
        lines[i] += " # not really"
        _write(self.root, drift.BUILD_FILE, "\n".join(lines).encode())
        self.fails("contains '#'")

    def test_empty_bindings_fails(self) -> None:
        _write(self.root, drift.BINDINGS, b"")
        self.fails("0 (op, fault) pairs")

    def test_bindings_missing_one_fault_fails(self) -> None:
        self.edit(
            drift.BINDINGS,
            lambda t: t.replace(
                "faults=missing-error,saturate-add,saturate-sub",
                "faults=missing-error,saturate-add",
                1,
            ),
        )
        self.fails("30 (op, fault) pairs")

    # M2: every BUILD variant is parsed
    def test_divergent_pin_in_build_linux_fails(self) -> None:
        self.edit(
            drift.BUILD_FILES[1],
            lambda t: t.replace(
                "/exhaustive/and_truth_table", "/exhaustive/and_rejects_non_bit", 1
            ),
        )
        self.fails("BUILD_linux pins differ")

    def test_divergent_pin_in_build_windows_fails(self) -> None:
        self.edit(
            drift.BUILD_FILES[2],
            lambda t: t.replace(
                "&& set WARRANT_EVIDENCE_FILTER=/exhaustive/or_truth_table",
                "&& set WARRANT_EVIDENCE_FILTER=/exhaustive/or_rejects_non_bit",
                1,
            ),
        )
        self.fails("BUILD_windows pins differ")

    def test_missing_build_windows_fails(self) -> None:
        os.remove(os.path.join(self.root, *drift.BUILD_FILES[2].split("/")))
        self.fails("missing")

    def test_windows_form_is_parsed(self) -> None:
        pins = drift._parse_pins(_read(self.root, drift.BUILD_FILES[2]).decode())
        self.assertEqual(len(pins), 31)

    # M3: the fixture skip is explicit and fails closed
    def test_absent_fixture_skips_only_when_git_tracks_nothing(self) -> None:
        self.assertTrue(any("skipped" in n for n in self.run_drift()))
        self.fails_with_git(
            lambda root, rel: [rel + "/corpus.json"], "git tracks 1 file"
        )

    def fails_with_git(self, git, needle: str) -> None:
        with self.assertRaises(drift.DriftError) as ctx:
            self.run_drift(git)
        self.assertIn(needle, str(ctx.exception))

    def test_git_unavailable_fails_closed(self) -> None:
        with (
            unittest.mock.patch.object(drift.shutil, "which", return_value=None),
            self.assertRaises(drift.DriftError) as ctx,
        ):
            drift.run(self.root)
        self.assertIn("failing closed", str(ctx.exception))
        with (
            unittest.mock.patch.object(
                drift.subprocess, "run", side_effect=FileNotFoundError("git")
            ),
            self.assertRaises(drift.DriftError) as ctx,
        ):
            drift.run(self.root)
        self.assertIn("failing closed", str(ctx.exception))

    def test_git_query_ignores_git_environment(self) -> None:
        # A tracked-looking directory in a real repository, queried while GIT_DIR points
        # at an empty repository: the answer must not change.
        if shutil.which("git") is None:
            self.skipTest("git not installed")
        repo = os.path.join(self.root, "repo")
        decoy = os.path.join(self.root, "decoy")
        for path in (repo, decoy):
            os.makedirs(path)
            subprocess.run(["git", "init", "-q", path], check=True)
        _write(
            repo, "code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.json", b"{}"
        )
        subprocess.run(["git", "-C", repo, "add", "-A"], check=True)
        plain = drift._git_tracked_under(repo, drift.FIXTURE_DIR)
        self.assertEqual(len(plain), 1)
        env = {
            "GIT_DIR": os.path.join(decoy, ".git"),
            "GIT_WORK_TREE": decoy,
            "GIT_INDEX_FILE": os.path.join(decoy, ".git", "index"),
        }
        with unittest.mock.patch.dict(os.environ, env):
            self.assertEqual(drift._git_tracked_under(repo, drift.FIXTURE_DIR), plain)

    def test_pin_command_must_be_the_exact_template(self) -> None:
        for suffix in (" || true", " ; true", " > /dev/null", " & true"):
            with self.subTest(suffix):
                self.edit(
                    drift.BUILD_FILE,
                    lambda t, suffix=suffix: t.replace(
                        "--test warrant_evidence\nWARRANT_EVIDENCE_FAULT=saturate-add",
                        "--test warrant_evidence"
                        + suffix
                        + "\nWARRANT_EVIDENCE_FAULT=saturate-add",
                        1,
                    ),
                )
                self.fails("shell operator")
                _write(
                    self.root, drift.BUILD_FILE, (REPO / drift.BUILD_FILE).read_bytes()
                )

    def test_pin_re_override_or_other_command_fails(self) -> None:
        self.edit(
            drift.BUILD_FILE,
            lambda t: t.replace(
                "python3 ../../../scripts/warrant_negctl.py",
                "WARRANT_EVIDENCE_FAULT=none python3 ../../../scripts/warrant_negctl.py",
                1,
            ),
        )
        self.fails("exact template")
        _write(self.root, drift.BUILD_FILE, (REPO / drift.BUILD_FILE).read_bytes())
        self.edit(
            drift.BUILD_FILES[2],
            lambda t: t.replace(
                "-- cargo test -q -p logic-gates", "-- cargo test -p logic-gates", 1
            ),
        )
        self.fails("exact template")

    def test_required_fixture_absent_fails(self) -> None:
        with unittest.mock.patch.object(drift, "FIXTURE_REQUIRED", True):
            self.fails("FIXTURE_REQUIRED")

    def test_non_enoent_error_on_the_fixture_path_fails_closed(self) -> None:
        fixtures = os.path.join(self.root, "code", "specs", "fixtures")
        _write(self.root, "code/specs/fixtures", b"a file where a directory should be")
        self.assertTrue(os.path.isfile(fixtures))
        self.fails("failing closed")

    def test_windows_style_enoent_under_a_regular_file_fails_closed(self) -> None:
        # Simulate Windows, which reports "path not found" (FileNotFoundError) for a child
        # of a regular file; the guard must still notice that the parent is not a directory.
        _write(self.root, "code/specs/fixtures", b"a file where a directory should be")
        real = os.lstat

        def windows_lstat(path, *args, **kwargs):
            parent = os.path.dirname(path)
            if os.path.isfile(parent):
                raise FileNotFoundError(2, "path not found", path)
            return real(path, *args, **kwargs)

        with unittest.mock.patch.object(drift.os, "lstat", windows_lstat):
            self.fails("not a real directory")

    def test_symlinked_ancestor_of_the_fixture_fails_closed(self) -> None:
        fixtures = os.path.join(self.root, "code", "specs", "fixtures")
        os.makedirs(os.path.join(self.root, "elsewhere"))
        self.symlink(os.path.join(self.root, "elsewhere"), fixtures)
        self.fails("not a real directory")

    # L5: duplicates
    def test_duplicate_block_key_in_either_document_fails(self) -> None:
        dup = "  error GateError = INVALID_INPUT\n"
        self.set_basic(self.basic() + dup)
        self.fails("duplicate error block")
        self.set_basic((REPO / drift.BASIC_SPEC).read_bytes())
        full = (REPO / drift.FULL_SPEC).read_text()
        _write(self.root, drift.FULL_SPEC, (full + dup).encode())
        self.fails("duplicate error block", drift.FULL_SPEC)

    # L7: the file must be the one lstat saw
    def test_inode_change_between_lstat_and_open_fails(self) -> None:
        real = os.fstat

        def swapped(fd):
            st = real(fd)
            return os.stat_result(
                (
                    st.st_mode,
                    st.st_ino + 1,
                    st.st_dev,
                    st.st_nlink,
                    st.st_uid,
                    st.st_gid,
                    st.st_size,
                    0,
                    0,
                    0,
                )
            )

        with unittest.mock.patch.object(drift.os, "fstat", swapped):
            self.fails("changed between lstat and open")


class FixtureExact(Tree):
    def setUp(self) -> None:
        super().setUp()
        _fixture(self.root)

    def test_required_claim_must_be_the_exact_form(self) -> None:
        for bad in (
            "and_truth_table",
            "warrant/other_module/exhaustive/and_truth_table",
            "x/warrant/logic_gates_basic/exhaustive/and_truth_table",
        ):
            with self.subTest(bad):
                _fixture(self.root)
                corpus = json.loads(_read(self.root, drift.CORPUS))
                for control in corpus["controls"]:
                    if (control["op"], control["fault"]) == (
                        "and_gate",
                        "saturate-add",
                    ):
                        control["required_claims"] = [bad]
                _write(self.root, drift.CORPUS, json.dumps(corpus).encode())
                self.fails("expected " + drift.CLAIM_PREFIX + "and_truth_table")

    def test_duplicate_control_fails(self) -> None:
        corpus = json.loads(_read(self.root, drift.CORPUS))
        corpus["controls"].append(dict(corpus["controls"][0]))
        _write(self.root, drift.CORPUS, json.dumps(corpus).encode())
        self.fails("duplicate control")

    def test_empty_controls_fail(self) -> None:
        corpus = json.loads(_read(self.root, drift.CORPUS))
        corpus["controls"] = []
        _write(self.root, drift.CORPUS, json.dumps(corpus).encode())
        self.fails("0 (op, fault) pairs")

    def test_empty_profile_fails(self) -> None:
        _write(self.root, drift.PROFILE, b"")
        self.fails("0 (op, fault) pairs")


class ClaimTable(unittest.TestCase):
    def test_equals_claims_map_to_the_nand_built_op(self) -> None:
        self.assertEqual(drift.CLAIM_TO_OP["nand_xor_equals_xor"], "nand_xor")
        self.assertEqual(drift.CLAIM_TO_OP["xor_truth_table"], "xor_gate")
        self.assertEqual(drift.CLAIM_TO_OP["nand_not_rejects_non_bit"], "nand_not")
        self.assertEqual(len(drift.CLAIM_TO_OP), 26)


if __name__ == "__main__":
    unittest.main()
