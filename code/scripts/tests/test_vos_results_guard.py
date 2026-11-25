"""Tests for code/scripts/vos_results_guard.py (VOS-D126).

Each case builds a throwaway git repository and runs the guard on a range.
"""

from __future__ import annotations

import json
import re
import unittest.mock as mock
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS))
import vos_results_guard as guard  # noqa: E402

RES = guard.RESULTS_FILE
PROBE = "code/programs/rust/vos-capacity-probe/src/lib.rs"
SPEC = "code/specs/VOS30.md"
IDENT = ["-c", "user.name=t", "-c", "user.email=t@example.invalid",
         "-c", "commit.gpgsign=false"]


def entry_files(content: str = "a") -> dict[str, str]:
    """One committed file per TREE_PATHS entry: a directory entry (no extension in its last
    component) gets `f.txt` inside it, a file entry is the file itself."""
    files = {}
    for path in guard.TREE_PATHS.values():
        leaf = path.rsplit("/", 1)[-1]
        files[path if "." in leaf else f"{path}/f.txt"] = content
    return files


def meta_rs_entries() -> dict[str, str]:
    """TREE_ENTRIES parsed from the probe's src/meta.rs (name -> path)."""
    src = (SCRIPTS.parent / "programs/rust/vos-capacity-probe/src/meta.rs").read_text(
        encoding="utf-8"
    )
    body = src.split("pub const TREE_ENTRIES", 1)[1].split("];", 1)[0]
    body = body.split("= [", 1)[1]
    pairs = re.findall(r'\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,?\s*\)', body)
    entries = dict(pairs)
    declared = int(re.search(r"\[\(&str, &str\); (\d+)\]", src).group(1))
    assert len(pairs) == len(entries), "meta.rs repeats an entry name"
    assert len(pairs) == declared, f"meta.rs declares {declared} entries but lists {len(pairs)}"
    return entries


def results_json(trees, recording=False, regeneration="absent"):
    doc = {"schema": 2, "tool": {"name": "vos-capacity-probe", "version": "0.2.0"}}
    if trees is not None:
        doc["tool"]["trees"] = trees
    if recording:
        doc["recording"] = {"mode": "record"}
    if regeneration != "absent":
        doc["regeneration"] = regeneration
    return json.dumps(doc)


def banner(verified_trees, **extra):
    """A signed file's `regeneration` banner (VOS-028c3c-3b-1b); only `verified.trees` is read."""
    verified = {"run_id": "2", "run_attempt": "1", "head_sha": "c" * 40,
                "verify_sha256": "d" * 64}
    if verified_trees is not None:
        verified["trees"] = verified_trees
    verified.update(extra)
    return {"reviewer": "R", "host": "H", "run_id": "1", "run_attempt": "1",
            "head_sha": "a" * 40, "unsigned_sha256": "b" * 64, "verified": verified}


class Repo:
    def __init__(self, root: str):
        self.root = root
        self.git("init", "-q", "-b", "main")

    def run(self, *args: str, check: bool = True):
        return subprocess.run(
            ["git", *IDENT, "-C", self.root, *args],
            capture_output=True, text=True, encoding="utf-8", check=check,
        )

    def git(self, *args: str) -> str:
        return self.run(*args).stdout.strip()

    def write(self, files: dict[str, str]) -> None:
        for name, text in files.items():
            path = Path(self.root, name)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")

    def commit(self, files: dict[str, str], msg: str = "c") -> str:
        self.write(files)
        self.git("add", "--", *files)
        self.git("commit", "-q", "-m", msg)
        return self.rev("HEAD")

    def rev(self, ref: str) -> str:
        return self.git("rev-parse", ref)


class GuardTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.r = Repo(self._tmp.name)
        self.base = self.r.commit({"README.md": "x", PROBE: "0", SPEC: "0"}, "base")

    def run_guard(self, base=None, head=None):
        return guard.main(["--repo", self.r.root, "--base", base or self.base,
                           "--head", head or self.r.rev("HEAD")])

    def test_no_results_change_passes(self):
        self.r.commit({PROBE: "1"})
        self.r.commit({"other.txt": "1"})
        self.assertEqual(self.run_guard(), 0)

    def test_a_then_b_passes(self):
        self.r.commit({PROBE: "1"}, "A")
        self.r.commit({RES: "{}", SPEC: "1"}, "B")
        self.assertEqual(self.run_guard(), 0)

    def test_results_then_specs_only_passes(self):
        self.r.commit({RES: "{}"})
        self.r.commit({SPEC: "2"})
        self.assertEqual(self.run_guard(), 0)

    def test_later_probe_change_fails(self):
        self.r.commit({RES: "{}"})
        self.r.commit({PROBE: "2"})
        self.assertEqual(self.run_guard(), 1)

    def test_results_with_probe_file_fails(self):
        self.r.commit({RES: "{}", PROBE: "2"})
        self.assertEqual(self.run_guard(), 1)

    def test_probe_change_before_last_results_commit_is_fine(self):
        self.r.commit({RES: "{}"})
        self.r.commit({PROBE: "2"})
        self.r.commit({RES: '{"v":2}'})
        # The middle commit is followed by a later results commit, but the
        # rule starts at the LAST results commit: only that one onward counts.
        self.assertEqual(self.run_guard(), 0)

    def test_merge_of_main_after_b_passes(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.r.commit({PROBE: "1"}, "A")
        self.r.commit({RES: "{}"}, "B")
        self.r.git("checkout", "-q", "main")
        main_tip = self.r.commit({"unrelated/file.rs": "main work"}, "main moves")
        self.r.git("checkout", "-q", "pr")
        self.r.git("merge", "-q", "--no-ff", "-m", "merge main", "main")
        # main's commit is part of the base the PR was opened against
        self.assertEqual(self.run_guard(base=main_tip), 0)

    def test_merge_of_side_branch_after_b_fails(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.r.commit({RES: "{}"}, "B")
        self.r.git("checkout", "-q", "-b", "side")
        self.r.commit({PROBE: "sneaky"}, "probe edit after the recording")
        self.r.git("checkout", "-q", "pr")
        self.r.commit({SPEC: "x"}, "specs")
        self.r.git("merge", "-q", "--no-ff", "-m", "merge side", "side")
        self.assertEqual(self.run_guard(), 1)

    def test_octopus_with_one_side_branch_fails(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.r.commit({RES: "{}"}, "B")
        self.r.git("checkout", "-q", "-b", "side")
        self.r.commit({PROBE: "sneaky"}, "probe edit")
        self.r.git("checkout", "-q", "main")
        main_tip = self.r.commit({"unrelated/file.rs": "m"}, "main moves")
        self.r.git("checkout", "-q", "pr")
        self.r.commit({SPEC: "x"}, "specs")
        self.r.git("merge", "-q", "-m", "octopus", "main", "side")
        self.assertEqual(self.run_guard(base=main_tip), 1)

    def test_merge_bringing_results_from_main_is_not_a_results_commit(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.r.commit({PROBE: "1"}, "A")
        self.r.git("checkout", "-q", "main")
        main_tip = self.r.commit({RES: "{}"}, "main records")
        self.r.git("checkout", "-q", "pr")
        self.r.git("merge", "-q", "--no-ff", "-m", "merge main", "main")
        self.r.commit({PROBE: "2"}, "more probe work")
        self.assertEqual(self.run_guard(base=main_tip), 0)

    def test_late_commit_outside_specs_after_results_fails(self):
        self.r.commit({RES: "{}"}, "B")
        self.r.commit({SPEC: "1"}, "specs")
        self.r.commit({PROBE: "late"}, "late edit")
        self.assertEqual(self.run_guard(), 1)

    def test_merge_conflict_resolution_outside_specs_after_results_fails(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.r.commit({PROBE: "pr"}, "A")
        self.r.commit({RES: "{}"}, "B")
        self.r.git("checkout", "-q", "main")
        main_tip = self.r.commit({PROBE: "main"}, "main edits probe")
        self.r.git("checkout", "-q", "pr")
        merge = self.r.run("merge", "main", "-m", "m", check=False)
        self.assertNotEqual(merge.returncode, 0)  # conflict on PROBE
        self.r.write({PROBE: "resolved"})
        self.r.git("add", PROBE)
        self.r.git("commit", "-q", "-m", "resolve")
        self.assertEqual(self.run_guard(base=main_tip), 1)

    def record(self):
        """Commit every tree entry, then a results file recording their ids."""
        self.r.commit(entry_files(), "trees")
        trees = {n: self.r.rev(f"HEAD:{p}") for n, p in guard.TREE_PATHS.items()}
        self.r.commit({RES: results_json(trees)}, "record")

    def test_rule2_matching_trees_pass(self):
        self.record()
        self.assertEqual(self.run_guard(), 0)

    def test_rule2_changed_tree_fails_and_names_path(self):
        self.record()
        self.r.commit({"code/packages/rust/cdcl-sat/src/f.txt": "changed"}, "drift")
        problems, _ = guard.check_trees(self.r.root, "HEAD")
        self.assertEqual(len(problems), 1)
        self.assertIn("code/packages/rust/cdcl-sat/src", problems[0])
        self.assertIn("revert the change", problems[0])
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_drift_from_main_via_merge_fails(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.record()
        self.r.git("checkout", "-q", "main")
        self.r.commit({"code/packages/rust/bit-blaster/src/g.txt": "main"}, "main drift")
        self.r.git("checkout", "-q", "pr")
        self.r.git("merge", "-q", "--no-ff", "-m", "merge main", "main")
        # Rule 1 alone passes (the merge takes main's path unchanged).
        main_tip = self.r.rev("main")
        self.assertEqual(guard.check(self.r.root, main_tip, self.r.rev("HEAD")), [])
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_no_trees_passes(self):
        self.r.commit({RES: results_json(None)})
        self.assertEqual(self.run_guard(), 0)

    def test_rule2_malformed_json_exit_2(self):
        self.r.commit({RES: "{not json"})
        self.assertEqual(self.run_guard(), 2)

    def test_rule2_non_hex_id_exit_2(self):
        self.r.commit({RES: results_json({"cdcl-sat/src": "xyz"})})
        self.assertEqual(self.run_guard(), 2)

    def test_rule2_unknown_name_exit_2(self):
        self.r.commit({RES: results_json({"nope": "0" * 40})})
        self.assertEqual(self.run_guard(), 2)

    def full_trees(self):
        self.r.commit(entry_files(), "trees")
        return {n: self.r.rev(f"HEAD:{p}") for n, p in guard.TREE_PATHS.items()}

    def stale_tool_trees(self):
        """(tool trees recorded earlier, trees now): a shared crate moved after the recording."""
        old = self.full_trees()
        self.r.commit({"code/packages/rust/cdcl-sat/src/f.txt": "moved"}, "shared crate moves")
        now = {n: self.r.rev(f"HEAD:{p}") for n, p in guard.TREE_PATHS.items()}
        self.assertNotEqual(old, now)
        return old, now

    def test_rule2_signed_file_passes_on_verified_trees_when_tool_trees_are_stale(self):
        """VOS-028c3c-3b-1b: the verify ran at the tree that merges, the recording did not."""
        old, now = self.stale_tool_trees()
        self.r.commit({RES: results_json(old, recording=True, regeneration=banner(now))})
        problems, _ = guard.check_trees(self.r.root, "HEAD")
        self.assertEqual(problems, [])
        self.assertEqual(self.run_guard(), 0)

    def test_rule2_signed_file_passes_when_tool_trees_match_whatever_verified_says(self):
        """Either anchor is enough; the verified ones must still be well formed."""
        old, now = self.stale_tool_trees()
        self.r.commit({RES: results_json(now, recording=True, regeneration=banner(old))})
        self.assertEqual(self.run_guard(), 0)

    def test_rule2_signed_file_fails_when_neither_matches_and_names_the_path(self):
        old, now = self.stale_tool_trees()
        other = dict(old)
        other["bit-blaster/src"] = "e" * 40
        self.r.commit({RES: results_json(old, recording=True, regeneration=banner(other))})
        problems, _ = guard.check_trees(self.r.root, "HEAD")
        text = " ".join(problems)
        self.assertIn("tool.trees[cdcl-sat/src]", text)
        self.assertIn("regeneration.verified.trees[bit-blaster/src]", text)
        self.assertIn("code/packages/rust/bit-blaster/src", text)
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_unsigned_file_behaves_as_before(self):
        old, _ = self.stale_tool_trees()
        for reg in ("absent", None):
            with self.subTest(regeneration=reg):
                self.r.commit({RES: results_json(old, recording=True, regeneration=reg)})
                problems, _ = guard.check_trees(self.r.root, "HEAD")
                self.assertEqual(len(problems), 1)
                self.assertIn("tool.trees[cdcl-sat/src]", problems[0])
                self.assertEqual(self.run_guard(), 1)

    def test_rule2_a_signed_banner_without_verified_exits_2(self):
        """Fail closed: the probe refuses such a file, so the guard does not guess either."""
        old, _ = self.stale_tool_trees()
        reg = banner({})
        del reg["verified"]
        self.r.commit({RES: results_json(old, recording=True, regeneration=reg)})
        self.assertEqual(self.run_guard(), 2)

    def test_rule2_malformed_banner_fields_exit_2(self):
        _, now = self.stale_tool_trees()
        good = banner(now)
        cases = {}
        for key in ("verify_sha256", "head_sha", "run_id", "run_attempt"):
            for label, bad in (("missing", None), ("not a string", 5), ("empty", ""),
                               ("trailing newline", good["verified"][key] + "\n"),
                               ("upper case", "A" * len(good["verified"][key]))
                               if key.endswith(("sha256", "head_sha")) else ("letters", "1a")):
                reg = banner(now)
                if bad is None:
                    del reg["verified"][key]
                else:
                    reg["verified"][key] = bad
                cases[f"{key} {label}"] = reg
        for label, bad in (("missing", None), ("short", "ab"), ("upper", "B" * 64)):
            reg = banner(now)
            if bad is None:
                del reg["unsigned_sha256"]
            else:
                reg["unsigned_sha256"] = bad
            cases[f"unsigned_sha256 {label}"] = reg
        for name, reg in cases.items():
            with self.subTest(case=name):
                self.r.commit({RES: results_json(now, recording=True, regeneration=reg)})
                self.assertEqual(self.run_guard(), 2)

    def test_patterns_are_applied_with_fullmatch_so_a_trailing_newline_cannot_pass(self):
        nl = "\n"
        for pattern in (guard.HEX40, guard.SHA_RE):
            self.assertIsNotNone(pattern.fullmatch("a" * 40))
            self.assertIsNone(pattern.fullmatch("a" * 40 + nl))
        self.assertIsNone(guard.HEX64.fullmatch("a" * 64 + nl))
        self.assertIsNone(guard.RUN_NUMBER.fullmatch("12" + nl))
        self.assertIsNone(guard.REV_RE.fullmatch("main" + nl))
        # And through the guard: a tool.trees id with a trailing newline exits 2.
        trees = self.full_trees()
        trees["cdcl-sat/src"] += nl
        self.r.commit({RES: results_json(trees, recording=True)})
        self.assertEqual(self.run_guard(), 2)
        with self.assertRaises(guard.GuardError):
            guard.analyse(self.r.root, "a" * 40 + nl, "b" * 40)

    def test_rule2_malformed_verified_exits_2(self):
        old, now = self.stale_tool_trees()
        short = dict(now)
        del short["cdcl-sat/src"]
        extra = dict(now)
        extra["extra"] = "0" * 40
        bad_id = dict(now)
        bad_id["cdcl-sat/src"] = "xyz"
        upper = dict(now)
        upper["cdcl-sat/src"] = now["cdcl-sat/src"].upper()
        cases = {
            "not an object": banner(["x"]),
            "no trees": banner(None),
            "missing entry": banner(short),
            "extra entry": banner(extra),
            "non-hex id": banner(bad_id),
            "upper-case id": banner(upper),
            "non-string id": banner({**now, "cdcl-sat/src": 5}),
            "verified not an object": {**banner(now), "verified": "x"},
            "regeneration not an object": "signed",
        }
        for name, reg in cases.items():
            with self.subTest(case=name):
                # Even when tool.trees match, a malformed banner is never waved through.
                self.r.commit({RES: results_json(now, recording=True, regeneration=reg)})
                self.assertEqual(self.run_guard(), 2)

    def test_rule2_verified_trees_must_be_the_trees_of_the_merge_ref(self):
        """A verified id that is a valid hex id but not the tree at the merge ref does not pass."""
        old, now = self.stale_tool_trees()
        wrong = dict(now)
        wrong["cdcl-sat/src"] = "f" * 40
        self.r.commit({RES: results_json(old, recording=True, regeneration=banner(wrong))})
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_non_input_edit_does_not_stale_a_recording(self):
        """VOS-D140: a README, CHANGELOG or test edit in a recorded crate is not an input."""
        trees = self.full_trees()
        self.r.commit({RES: results_json(trees, recording=True)})
        self.r.commit({
            "code/packages/rust/cdcl-sat/README.md": "docs",
            "code/packages/rust/cdcl-sat/CHANGELOG.md": "log",
            "code/programs/rust/vos-capacity-probe/tests/t.rs": "test",
        }, "docs and tests")
        problems, _ = guard.check_trees(self.r.root, "HEAD")
        self.assertEqual(problems, [])

    def test_rule2_manifest_or_lockfile_edit_stales_a_recording(self):
        trees = self.full_trees()
        self.r.commit({RES: results_json(trees, recording=True)})
        self.r.commit({"code/programs/rust/vos-capacity-probe/Cargo.lock": "b"}, "lock")
        problems, _ = guard.check_trees(self.r.root, "HEAD")
        self.assertEqual(len(problems), 1)
        self.assertIn("vos-capacity-probe/Cargo.lock", problems[0])

    def test_guard_map_equals_the_probes_tree_entries(self):
        """The guard's name -> path map cannot drift from `meta::TREE_ENTRIES` (VOS-D140)."""
        entries = meta_rs_entries()
        self.assertGreaterEqual(len(entries), 15)
        self.assertEqual(entries, guard.TREE_PATHS)
        self.assertEqual(list(entries), list(guard.TREE_PATHS))

    def test_rule2_recorded_file_with_all_trees_passes(self):
        self.r.commit({RES: results_json(self.full_trees(), recording=True)})
        self.assertEqual(self.run_guard(), 0)

    def test_rule2_recorded_file_missing_trees_fails(self):
        self.full_trees()
        self.r.commit({RES: results_json(None, recording=True)})
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_recorded_file_with_four_trees_fails(self):
        trees = self.full_trees()
        del trees["cdcl-sat/src"]
        self.r.commit({RES: results_json(trees, recording=True)})
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_recorded_file_with_extra_tree_fails(self):
        trees = self.full_trees()
        trees["extra"] = "0" * 40
        self.r.commit({RES: results_json(trees, recording=True)})
        self.assertEqual(self.run_guard(), 1)

    def test_ratchet_base_with_trees_pr_without_fails(self):
        self.r.commit({RES: results_json(self.full_trees())})
        base = self.r.rev("HEAD")
        self.r.commit({RES: results_json(None)})
        self.assertEqual(self.run_guard(base=base), 1)

    def test_ratchet_base_with_recording_pr_without_fails(self):
        self.full_trees()
        self.r.commit({RES: results_json(None, recording=True)})
        base = self.r.rev("HEAD")
        self.r.commit({RES: results_json(None)})
        self.assertEqual(self.run_guard(base=base), 1)

    def test_ratchet_base_without_trees_pr_without_trees_passes(self):
        self.r.commit({RES: results_json(None)})
        base = self.r.rev("HEAD")
        self.r.commit({RES: json.dumps({"schema": 2, "tool": {"version": "0.1.0"}})})
        self.assertEqual(self.run_guard(base=base), 0)

    def test_ratchet_base_with_trees_pr_with_matching_trees_passes(self):
        trees = self.full_trees()
        self.r.commit({RES: results_json(trees)})
        base = self.r.rev("HEAD")
        doc = json.loads(results_json(trees))
        doc["schema"] = 3
        self.r.commit({RES: json.dumps(doc)})
        self.assertEqual(self.run_guard(base=base), 0)

    def test_pr_not_touching_results_file_ignores_tree_drift(self):
        # The key scope regression: a recorded file at the base, a PR that
        # changes a shared solver crate but not the results file => passes.
        self.r.commit({RES: results_json(self.full_trees(), recording=True)})
        base = self.r.rev("HEAD")
        self.r.commit({"code/packages/rust/cdcl-sat/f.txt": "changed"})
        self.assertEqual(self.run_guard(base=base), 0)

    def test_pr_not_touching_results_file_ignores_malformed_file(self):
        self.r.commit({RES: "{not json"})
        base = self.r.rev("HEAD")
        self.r.commit({PROBE: "changed"})
        self.assertEqual(self.run_guard(base=base), 0)

    def test_ratchet_deleting_recorded_file_fails(self):
        self.r.commit({RES: results_json(self.full_trees())})
        base = self.r.rev("HEAD")
        self.r.git("rm", "-q", "--", RES)
        self.r.git("commit", "-q", "-m", "delete")
        self.assertEqual(self.run_guard(base=base), 1)

    def test_runs_from_a_subdirectory(self):
        self.r.commit({RES: results_json(self.full_trees(), recording=True)})
        self.r.commit({PROBE: "late"})
        sub = Path(self.r.root, "code", "specs")
        old = Path.cwd()
        try:
            import os
            os.chdir(sub)
            rc = guard.main(["--repo", ".", "--base", self.base,
                             "--head", self.r.rev("HEAD")])
        finally:
            os.chdir(old)
        self.assertEqual(rc, 1)  # the file was found: rule 2 sees the changed probe tree

    def test_duplicate_json_key_exit_2(self):
        self.r.commit({RES: '{"tool": {}, "tool": {}}'})
        self.assertEqual(self.run_guard(), 2)

    def test_deeply_nested_json_exit_2(self):
        self.r.commit({RES: "[" * 200000 + "]" * 200000})
        self.assertEqual(self.run_guard(), 2)

    def test_undecodable_bytes_do_not_crash(self):
        raw = guard.decode(b"code/specs/\xff.txt")
        self.assertTrue(raw.startswith("code/specs/"))
        self.assertNotIn("\udcff", guard.printable(raw))
        self.assertTrue(guard.printable(raw).endswith("\ufffd.txt"))

    def test_commit_cap_exit_2(self):
        self.r.commit({SPEC: "1"})
        self.r.commit({SPEC: "2"})
        self.r.commit({SPEC: "3"})
        with mock.patch.object(guard, "MAX_COMMITS", 2):
            self.assertEqual(self.run_guard(), 2)
        with mock.patch.object(guard, "MAX_COMMITS", 3):
            self.assertEqual(self.run_guard(), 0)

    def test_bad_sha_exit_2(self):
        self.assertEqual(self.run_guard(base="HEAD"), 2)
        self.assertEqual(self.run_guard(base="0" * 40), 2)

    def test_missing_args_exit_2(self):
        self.assertEqual(guard.main(["--repo", self.r.root]), 2)


if __name__ == "__main__":
    unittest.main()
