"""Tests for npm_lockfiles.py, the static `npm ci` readiness checker.

Each rule is pinned with one failing and one passing example so the checker
cannot silently turn into a no-op.  The last test runs the real repository.
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import npm_lockfiles as nl  # noqa: E402

REG = "https://registry.npmjs.org/x/-/x-1.0.0.tgz"


class BuildScanTests(unittest.TestCase):
    def test_plain_npm_install_is_rejected(self):
        bad, _ = nl.scan_build_line("code/p", "npm install --silent")
        self.assertEqual(len(bad), 1)

    def test_npm_i_is_rejected(self):
        bad, _ = nl.scan_build_line("code/p", "cd ../x && npm i")
        self.assertEqual(len(bad), 1)

    def test_global_npm_self_pin_allowed(self):
        bad, _ = nl.scan_build_line("p", "npm install --global npm@11.13.0")
        self.assertEqual(bad, [])

    def test_global_unpinned_rejected(self):
        bad, _ = nl.scan_build_line("p", "npm install -g npm")
        self.assertEqual(len(bad), 1)

    def test_no_save_exact_allowed_and_range_rejected(self):
        self.assertEqual(nl.scan_build_line("p", "npm install --no-save foo@1.2.3")[0], [])
        self.assertEqual(len(nl.scan_build_line("p", "npm install --no-save foo@^1.2.3")[0]), 1)

    def test_ci_directories_follow_cd_chain(self):
        _, dirs = nl.scan_build_line("code/a/b", "cd ../dep && npm ci --quiet && cd - && npm ci")
        self.assertEqual(dirs, ["code/a/dep", "code/a/b"])

    def test_subshell_cd_does_not_leak(self):
        _, dirs = nl.scan_build_line("code/a/b", "(cd ../dep && npm ci) && npm ci")
        self.assertEqual(dirs, ["code/a/dep", "code/a/b"])

    def test_prefix_and_windows_paths(self):
        _, dirs = nl.scan_build_line("code/a/b", "npm ci --prefix ..\\sib --silent")
        self.assertEqual(dirs, ["code/a/sib"])
        _, dirs = nl.scan_build_line("code/a/b", "cd /d ..\\..\\c && npm ci")
        self.assertEqual(dirs, ["code/c"])

    def test_quoted_parens_ignored(self):
        _, dirs = nl.scan_build_line("d", 'cd e && npm ci && test -n "$(find .)"')
        self.assertEqual(dirs, ["d/e"])

    def test_comments_skipped(self):
        bad, dirs = nl.scan_build_file("d/BUILD", "# npm install here\nnpm ci\n")
        self.assertEqual((bad, dirs), ([], {"d"}))


class LockTests(unittest.TestCase):
    def lock(self, **root):
        return {"packages": {"": root}}

    def test_in_sync(self):
        pkg = {"dependencies": {"a": "^1"}, "devDependencies": {"b": "2"}}
        lock = self.lock(**pkg)
        self.assertEqual(nl.check_lock_matches_package(lock, pkg, "L"), [])

    def test_missing_dev_dependency(self):
        pkg = {"devDependencies": {"b": "2"}}
        problems = nl.check_lock_matches_package(self.lock(), pkg, "L")
        self.assertEqual(len(problems), 1)
        self.assertIn("devDependencies[b]", problems[0])

    def test_changed_range(self):
        pkg = {"dependencies": {"a": "^2"}}
        lock = self.lock(dependencies={"a": "^1"})
        self.assertEqual(len(nl.check_lock_matches_package(lock, pkg, "L")), 1)

    def test_no_root_entry(self):
        self.assertEqual(len(nl.check_lock_matches_package({"packages": {}}, {}, "L")), 1)

    def test_sources(self):
        good = {"packages": {"node_modules/x": {"resolved": REG, "integrity": "sha512-a"},
                             "node_modules/s": {"resolved": "../s", "link": True}}}
        self.assertEqual(nl.check_lock_sources(good, "L"), [])
        no_int = {"packages": {"node_modules/x": {"resolved": REG}}}
        self.assertEqual(len(nl.check_lock_sources(no_int, "L")), 1)
        foreign = {"packages": {"node_modules/x": {"resolved": "https://evil.example/x.tgz", "integrity": "s"}}}
        self.assertEqual(len(nl.check_lock_sources(foreign, "L")), 1)
        git = {"packages": {"node_modules/x": {"resolved": "git+ssh://git@github.com/a/b.git#c"}}}
        self.assertEqual(len(nl.check_lock_sources(git, "L")), 1)


class RepoTests(unittest.TestCase):
    def make(self, files):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for name, text in files.items():
            p = root / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text, encoding="utf-8")
        return root, list(files)

    def test_missing_lock_flagged_and_sibling_chain_followed(self):
        root, files = self.make({
            "p/BUILD": "cd ../dep && npm ci\nnpm ci\n",
            "p/package.json": "{}",
            "p/package-lock.json": '{"packages": {"": {}}}',
            "dep/package.json": "{}",
        })
        problems = nl.check_repo(root, files)
        self.assertEqual(len(problems), 1)
        self.assertIn("dep/package-lock.json", problems[0])

    def test_dir_without_package_json_is_skipped(self):
        root, files = self.make({"py/BUILD": "npm ci\n"})
        self.assertEqual(nl.check_repo(root, files), [])

    def test_clean_repo(self):
        root, files = self.make({
            "p/BUILD": "npm ci --silent\n",
            "p/package.json": '{"dependencies": {"a": "1"}}',
            "p/package-lock.json": '{"packages": {"": {"dependencies": {"a": "1"}}}}',
        })
        self.assertEqual(nl.check_repo(root, files), [])

    def test_real_repository_is_clean(self):
        root = Path(__file__).resolve().parents[3]
        self.assertEqual(nl.check_repo(root, nl.tracked_files(root)), [])


if __name__ == "__main__":
    unittest.main()
