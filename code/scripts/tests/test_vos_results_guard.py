"""Tests for code/scripts/vos_results_guard.py (VOS-D126).

Each case builds a throwaway git repository and runs the guard on a range.
"""

from __future__ import annotations

import json
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


def results_json(trees):
    doc = {"schema": 2, "tool": {"name": "vos-capacity-probe", "version": "0.2.0"}}
    if trees is not None:
        doc["tool"]["trees"] = trees
    return json.dumps(doc)


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
        self.r.commit({"unrelated/file.rs": "main work"}, "main moves")
        self.r.git("checkout", "-q", "pr")
        self.r.git("merge", "-q", "--no-ff", "-m", "merge main", "main")
        self.assertEqual(self.run_guard(), 0)

    def test_merge_bringing_results_from_main_is_not_a_results_commit(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.r.commit({PROBE: "1"}, "A")
        self.r.git("checkout", "-q", "main")
        self.r.commit({RES: "{}"}, "main records")
        self.r.git("checkout", "-q", "pr")
        self.r.git("merge", "-q", "--no-ff", "-m", "merge main", "main")
        self.r.commit({PROBE: "2"}, "more probe work")
        self.assertEqual(self.run_guard(), 0)

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
        self.r.commit({PROBE: "main"}, "main edits probe")
        self.r.git("checkout", "-q", "pr")
        merge = self.r.run("merge", "main", "-m", "m", check=False)
        self.assertNotEqual(merge.returncode, 0)  # conflict on PROBE
        self.r.write({PROBE: "resolved"})
        self.r.git("add", PROBE)
        self.r.git("commit", "-q", "-m", "resolve")
        self.assertEqual(self.run_guard(), 1)

    def record(self):
        """Commit the five tree dirs, then a results file recording their ids."""
        files = {f"{p}/f.txt": "a" for p in guard.TREE_PATHS.values()}
        self.r.commit(files, "trees")
        trees = {n: self.r.rev(f"HEAD:{p}") for n, p in guard.TREE_PATHS.items()}
        self.r.commit({RES: results_json(trees)}, "record")

    def test_rule2_matching_trees_pass(self):
        self.record()
        self.assertEqual(self.run_guard(), 0)

    def test_rule2_changed_tree_fails_and_names_path(self):
        self.record()
        self.r.commit({"code/packages/rust/cdcl-sat/f.txt": "changed"}, "drift")
        problems, _ = guard.check_trees(self.r.root, "HEAD")
        self.assertEqual(len(problems), 1)
        self.assertIn("code/packages/rust/cdcl-sat", problems[0])
        self.assertIn("revert the change", problems[0])
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_drift_from_main_via_merge_fails(self):
        self.r.git("checkout", "-q", "-b", "pr")
        self.record()
        self.r.git("checkout", "-q", "main")
        self.r.commit({"code/packages/rust/bit-blaster/g.txt": "main"}, "main drift")
        self.r.git("checkout", "-q", "pr")
        self.r.git("merge", "-q", "--no-ff", "-m", "merge main", "main")
        # Rule 1 alone passes (the merge takes main's path unchanged).
        self.assertEqual(guard.check(self.r.root, self.base, self.r.rev("HEAD")), [])
        self.assertEqual(self.run_guard(), 1)

    def test_rule2_no_trees_passes(self):
        self.r.commit({RES: results_json(None)})
        self.assertEqual(self.run_guard(), 0)

    def test_rule2_malformed_json_exit_2(self):
        self.r.commit({RES: "{not json"})
        self.assertEqual(self.run_guard(), 2)

    def test_rule2_non_hex_id_exit_2(self):
        self.r.commit({RES: results_json({"cdcl-sat": "xyz"})})
        self.assertEqual(self.run_guard(), 2)

    def test_rule2_unknown_name_exit_2(self):
        self.r.commit({RES: results_json({"nope": "0" * 40})})
        self.assertEqual(self.run_guard(), 2)

    def test_bad_sha_exit_2(self):
        self.assertEqual(self.run_guard(base="HEAD"), 2)
        self.assertEqual(self.run_guard(base="0" * 40), 2)

    def test_missing_args_exit_2(self):
        self.assertEqual(guard.main(["--repo", self.r.root]), 2)


if __name__ == "__main__":
    unittest.main()
