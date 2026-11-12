"""Tests for code/scripts/cargo_lockfiles.py.

Every fixture is a tiny git repository of path-only Cargo projects, so the
tests need no network.  A "registry" dependency is declared (never resolved)
wherever a test only needs the static "this root needs a lock" answer.
"""

import shutil
import unittest.mock as mock
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import cargo_lockfiles as cl  # noqa: E402

HAVE_CARGO = shutil.which("cargo") is not None


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def package(name: str, deps: str = "") -> str:
    return (f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n'
            f"[dependencies]\n{deps}\n")


class Fixture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name).resolve()
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)

    def add(self, rel: str, text: str) -> None:
        write(self.repo / rel / "Cargo.toml", text)
        write(self.repo / rel / "src" / "lib.rs", "")

    def track(self) -> None:
        subprocess.run(["git", "add", "."], cwd=self.repo, check=True)

    def roots(self):
        self.track()
        return {r.path.relative_to(self.repo).as_posix(): r
                for r in cl.discover(self.repo)}


class DiscoveryTests(Fixture):
    def test_workspace_members_are_not_roots(self):
        self.add("ws", '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n')
        self.add("ws/crates/a", package("a"))
        self.add("ws/crates/b", package("b", 'a = { path = "../a" }'))
        self.assertEqual(set(self.roots()), {"ws"})

    def test_excluded_and_standalone_crates_are_roots(self):
        self.add("ws", '[workspace]\nmembers = ["crates/*"]\nexclude = ["crates/x"]\n')
        self.add("ws/crates/a", package("a"))
        self.add("ws/crates/x", package("x"))
        self.add("solo", package("solo"))
        self.assertEqual(set(self.roots()), {"ws", "ws/crates/x", "solo"})

    def test_detached_workspace_below_workspace_is_a_root(self):
        self.add("ws", '[workspace]\nmembers = ["a"]\n')
        self.add("ws/a", package("a"))
        self.add("ws/a/native", package("n") + "[workspace]\n")
        self.assertEqual(set(self.roots()), {"ws", "ws/a/native"})

    def test_needs_lock_only_with_external_dependency(self):
        self.add("pathonly", package("p", 'q = { path = "../q" }'))
        self.add("q", package("q"))
        self.add("ext", package("e", 'serde = "1"'))
        self.add("viapath", package("v", 'e = { path = "../ext" }'))
        roots = self.roots()
        self.assertFalse(roots["pathonly"].needs_lock)
        self.assertTrue(roots["ext"].needs_lock)
        # External-ness is transitive through path dependencies.
        self.assertTrue(roots["viapath"].needs_lock)

    def test_workspace_inherited_dependency(self):
        self.add("ws", '[workspace]\nmembers = ["a"]\n'
                       '[workspace.dependencies]\nserde = "1"\n')
        self.add("ws/a", package("a", "serde = { workspace = true }"))
        self.assertTrue(self.roots()["ws"].needs_lock)


class ChangedSinceTests(Fixture):
    def test_manifest_in_closure_marks_root_affected(self):
        self.add("lib", package("lib"))
        self.add("app", package("app", 'lib = { path = "../lib" }'))
        self.add("other", package("other"))
        roots = self.roots()
        changed = [self.repo / "lib" / "Cargo.toml"]
        self.assertTrue(cl.affected(roots["app"], changed))
        self.assertTrue(cl.affected(roots["lib"], changed))
        self.assertFalse(cl.affected(roots["other"], changed))

    def test_source_files_do_not_matter(self):
        self.add("lib", package("lib"))
        roots = self.roots()
        self.assertFalse(cl.affected(roots["lib"], [self.repo / "lib/src/lib.rs"]))


@unittest.skipUnless(HAVE_CARGO, "cargo not installed")
class CheckTests(Fixture):
    def test_missing_lock_is_an_error(self):
        self.add("ext", package("e", 'serde = "1"'))
        root = self.roots()["ext"]
        msg = cl.check_one(self.repo, root)
        self.assertIn("Cargo.lock is missing", msg)
        self.assertIn("cargo generate-lockfile", msg)

    def test_path_only_root_without_lock_is_fine(self):
        self.add("p", package("p"))
        self.assertIsNone(cl.check_one(self.repo, self.roots()["p"]))

    def test_clean_pass_then_stale_failure(self):
        self.add("p", package("p"))
        root = self.roots()["p"]
        code, out = cl.run_cargo(root, ["cargo", "generate-lockfile", "--offline"])
        self.assertEqual(code, 0, out)
        self.assertIsNone(cl.check_one(self.repo, root))

        # Add a path dependency; the committed lock no longer matches.
        self.add("q", package("q"))
        self.add("p", package("p", 'q = { path = "../q" }'))
        msg = cl.check_one(self.repo, root)
        self.assertIsNotNone(msg)
        self.assertIn("stale", msg)
        self.assertIn("cargo update --workspace", msg)


class ToolchainTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        self.addCleanup(setattr, cl, "TOOLCHAIN", cl.TOOLCHAIN)

    def mise(self, rust):
        (self.repo / "mise.toml").write_text(f'[tools]\nrust = "{rust}"\n')

    def test_exact_pin_is_used(self):
        self.mise("1.99.1")
        self.assertEqual(cl.pinned_rust_version(self.repo), "1.99.1")

    def test_non_exact_values_fall_back(self):
        for value in ("stable", "latest", "1.99", "nightly-2026-01-01"):
            self.mise(value)
            self.assertIsNone(cl.pinned_rust_version(self.repo), value)

    def test_missing_mise_falls_back(self):
        self.assertIsNone(cl.pinned_rust_version(self.repo))

    def test_cargo_argv(self):
        cl.TOOLCHAIN = "1.99.1"
        self.assertEqual(cl.cargo_argv(["cargo", "update"]),
                         ["cargo", "+1.99.1", "update"])
        cl.TOOLCHAIN = None
        self.assertEqual(cl.cargo_argv(["cargo", "update"]), ["cargo", "update"])

    def test_installs_only_when_missing(self):
        listed = mock.Mock(stdout="stable-x86_64-unknown-linux-gnu (default)\n")
        with mock.patch.object(cl.subprocess, "run", return_value=listed) as run:
            cl.ensure_toolchain("1.99.1")
        self.assertEqual(run.call_args_list[-1].args[0][:3],
                         ["rustup", "toolchain", "install"])
        listed = mock.Mock(stdout="1.99.1-x86_64-unknown-linux-gnu\n")
        with mock.patch.object(cl.subprocess, "run", return_value=listed) as run:
            cl.ensure_toolchain("1.99.1")
        self.assertEqual(run.call_count, 1)

    def test_no_roots_means_no_install(self):
        self.mise("1.99.1")
        with mock.patch.object(cl, "ensure_toolchain") as ens:
            cl.select_toolchain  # selection is only reached with roots
            args = mock.Mock(root=None, changed_since=None, jobs=1)
            with mock.patch.object(cl, "select", return_value=[]):
                cl.cmd_check(self.repo, [], args)
        ens.assert_not_called()


if __name__ == "__main__":
    unittest.main()
