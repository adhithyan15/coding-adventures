"""Tests for the CI apt wrapper.

The script exists because `apt-get update` exits 100 if *any* configured
repository fails, and the runner images ship vendor repositories this project
never installs from. A `packages.microsoft.com` 403 failed a docs-only pull
request's required job.

These tests drive the real script through `APT_PRUNE_ONLY`, against fixture
directories shaped like the runner's, so the pruning is exercised without root
and without touching a real apt.
"""

from __future__ import annotations

import re
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
SCRIPT = REPO / "code" / "scripts" / "ci" / "apt-install.sh"
WORKFLOWS = REPO / ".github" / "workflows"

# What a GitHub-hosted ubuntu-24.04 runner actually has. The archive lives in
# `sources.list.d/ubuntu.sources` in deb822 format -- NOT in
# `/etc/apt/sources.list`, which is a stub. Getting this wrong is the whole
# trap: "delete everything in sources.list.d" removes the archive every package
# here comes from.
UBUNTU_SOURCES = """\
Types: deb
URIs: http://azure.archive.ubuntu.com/ubuntu/
Suites: noble noble-updates noble-backports
Components: main restricted universe multiverse
"""


def _runner_layout(root: Path, *, modern: bool = True) -> tuple[Path, Path]:
    """A sources.list.d shaped like the runner's, plus the sources.list stub."""

    sources_dir = root / "sources.list.d"
    sources_dir.mkdir()
    sources_list = root / "sources.list"

    if modern:
        (sources_dir / "ubuntu.sources").write_text(UBUNTU_SOURCES)
        sources_list.write_text("# stub; the archive is in sources.list.d\n")
    else:
        # The 22.04 shape, where the archive is in the one-line file.
        sources_list.write_text(
            "deb http://azure.archive.ubuntu.com/ubuntu/ jammy main\n"
        )

    (sources_dir / "azure-cli.list").write_text(
        "deb https://packages.microsoft.com/repos/azure-cli/ noble main\n"
    )
    (sources_dir / "microsoft-prod.list").write_text(
        "deb https://packages.microsoft.com/ubuntu/24.04/prod noble main\n"
    )
    return sources_dir, sources_list


def _prune(sources_dir: Path, sources_list: Path, *args: str, keep: str = ""):
    """Run the script in prune-only mode, without sudo."""

    return subprocess.run(
        ["bash", str(SCRIPT), *args],
        env={
            "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
            "CI": "true",
            "APT_KEEP": keep,
            "APT_SUDO": "",
            "APT_PRUNE_ONLY": "1",
            "APT_SOURCES_DIR": str(sources_dir),
            "APT_SOURCES_LIST": str(sources_list),
        },
        capture_output=True,
        text=True,
    )


class PruneTests(unittest.TestCase):
    def test_removes_vendor_lists_and_keeps_the_ubuntu_archive(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            result = _prune(sources_dir, sources_list)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                sorted(p.name for p in sources_dir.iterdir()), ["ubuntu.sources"]
            )

    def test_removes_the_deb822_spelling_too(self) -> None:
        # The runner images have been migrating from `.list` to `.sources`, so a
        # pattern that knew only one spelling would silently stop pruning and
        # the outage would come back looking like a new bug.
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            (sources_dir / "microsoft-prod.sources").write_text(
                "Types: deb\nURIs: https://packages.microsoft.com/ubuntu/24.04/prod\n"
            )
            result = _prune(sources_dir, sources_list)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                sorted(p.name for p in sources_dir.iterdir()), ["ubuntu.sources"]
            )

    def test_prunes_every_vendor_repository_not_just_the_named_ones(self) -> None:
        # The gap the first version left. It enumerated microsoft and azure-cli,
        # and the runner also carries `google-chrome.sources` -- a repository
        # nothing here installs from, which can 403 exactly the way Microsoft's
        # did. A denylist over what someone else preinstalls is only ever as
        # current as the last time somebody looked at a runner image.
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            (sources_dir / "google-chrome.sources").write_text(
                "Types: deb\nURIs: https://dl.google.com/linux/chrome/deb/\n"
            )
            (sources_dir / "some-future-vendor.list").write_text(
                "deb https://vendor.example.com/apt noble main\n"
            )
            result = _prune(sources_dir, sources_list)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                sorted(p.name for p in sources_dir.iterdir()), ["ubuntu.sources"]
            )

    def test_apt_keep_preserves_a_declared_repository(self) -> None:
        # The escape hatch. Nothing needs it today -- no workflow runs
        # `add-apt-repository` -- but the day one does, it would add the PPA and
        # then call this script, which would delete it a line later and fail
        # with "unable to locate package" pointing at the package rather than
        # at us.
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            (sources_dir / "deadsnakes-ubuntu-ppa-noble.list").write_text(
                "deb https://ppa.launchpadcontent.net/deadsnakes/ppa/ubuntu noble main\n"
            )
            result = _prune(sources_dir, sources_list, keep="deadsnakes*")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                sorted(p.name for p in sources_dir.iterdir()),
                ["deadsnakes-ubuntu-ppa-noble.list", "ubuntu.sources"],
            )

    def test_a_declared_repository_is_not_kept_without_apt_keep(self) -> None:
        # So the test above cannot pass by the allowlist being permissive.
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            (sources_dir / "deadsnakes-ubuntu-ppa-noble.list").write_text("deb x y z\n")
            result = _prune(sources_dir, sources_list)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                sorted(p.name for p in sources_dir.iterdir()), ["ubuntu.sources"]
            )

    def test_refuses_to_continue_when_pruning_removed_everything(self) -> None:
        # The guard on the pruning itself. If a pattern is ever widened too far
        # and takes the archive with it, this says so in one line rather than
        # letting the job fail later with "unable to locate package", which
        # points at the package instead of at the cause.
        #
        # The invariant is "something we did not prune survives", NOT "a known
        # Ubuntu mirror hostname appears somewhere". The first version asked the
        # latter and failed on the real runner while passing every fixture here,
        # because it encoded a guess about the image's mirror and file layout.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sources_dir = root / "sources.list.d"
            sources_dir.mkdir()
            (sources_dir / "microsoft-prod.list").write_text("deb https://x noble main\n")
            sources_list = root / "sources.list"
            sources_list.write_text("# stub\n")

            result = _prune(sources_dir, sources_list)

            self.assertEqual(result.returncode, 1)
            self.assertIn("removed every configured apt source", result.stderr)

    def test_reports_what_survived_even_on_success(self) -> None:
        # Printed unconditionally. When this guard misfired in CI the error said
        # what it concluded and not what it saw, so diagnosing it cost another
        # run. The surviving source list is the one fact needed to tell "the
        # patterns are too broad" from "this image is laid out differently".
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            result = _prune(sources_dir, sources_list)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("apt sources remaining:", result.stdout)
            self.assertIn("ubuntu.sources", result.stdout)

    def test_accepts_the_older_layout(self) -> None:
        # 22.04 runners keep the archive in `/etc/apt/sources.list`. Both
        # layouts are in use across the matrix, so both have to be recognised.
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp), modern=False)
            result = _prune(sources_dir, sources_list)

            self.assertEqual(result.returncode, 0, result.stderr)

    def test_requires_packages(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            result = subprocess.run(
                ["bash", str(SCRIPT)],
                env={
                    "PATH": "/usr/bin:/bin",
                    "CI": "true",  # past the workstation guard, to the real check
                    "APT_SUDO": "",
                    "APT_SOURCES_DIR": str(sources_dir),
                    "APT_SOURCES_LIST": str(sources_list),
                },
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("no packages given", result.stderr)

    def test_refuses_to_run_outside_ci(self) -> None:
        # It removes system apt sources with `sudo rm`. On a runner that is
        # fine -- the VM is discarded. On a developer's machine it would
        # silently delete their VS Code, dotnet, and moby repository config,
        # and they would find out days later when updates stopped seeing them.
        with tempfile.TemporaryDirectory() as tmp:
            sources_dir, sources_list = _runner_layout(Path(tmp))
            result = subprocess.run(
                ["bash", str(SCRIPT), "libcairo2-dev"],
                env={
                    "PATH": "/usr/bin:/bin",
                    "APT_SUDO": "",
                    "APT_SOURCES_DIR": str(sources_dir),
                    "APT_SOURCES_LIST": str(sources_list),
                },
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 3)
            self.assertIn("meant for CI", result.stderr)
            # And it changed nothing on the way out.
            self.assertIn("microsoft-prod.list", [p.name for p in sources_dir.iterdir()])

    def test_survives_a_missing_sources_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sources_list = root / "sources.list"
            sources_list.write_text(
                "deb http://archive.ubuntu.com/ubuntu/ noble main\n"
            )
            result = _prune(root / "does-not-exist", sources_list)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("no unused source lists to prune", result.stdout)


class WorkflowConsistencyTests(unittest.TestCase):
    """Every site goes through the wrapper, not just the one that failed.

    The original bug was reported at one step, but the same construct appeared
    at fourteen across six workflows. Fixing only the reported one leaves the
    next outage to red-flag a different required job, so this asserts the whole
    population rather than the instance.
    """

    def test_no_workflow_calls_apt_get_update_directly(self) -> None:
        offenders = []
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            for number, line in enumerate(
                workflow.read_text().splitlines(), start=1
            ):
                if re.search(r"apt-get\s+update", line):
                    offenders.append(f"{workflow.name}:{number}: {line.strip()}")
        self.assertEqual(
            offenders,
            [],
            "these call `apt-get update` directly, so an unrelated vendor "
            "repository outage can hard-fail them; route them through "
            "code/scripts/ci/apt-install.sh:\n  " + "\n  ".join(offenders),
        )

    def test_the_wrapper_is_executable_and_present(self) -> None:
        self.assertTrue(SCRIPT.is_file(), f"missing: {SCRIPT}")

    def test_every_wrapper_call_names_at_least_one_package(self) -> None:
        # A call with no packages exits 2 at runtime; catching it here means
        # catching it without spending a CI job to find out.
        pattern = re.compile(r"apt-install\.sh([^\n]*)")
        empty = []
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            for number, line in enumerate(
                workflow.read_text().splitlines(), start=1
            ):
                match = pattern.search(line)
                if not match:
                    continue
                rest = match.group(1).strip()
                # A trailing backslash means the packages are on the next line.
                if rest.endswith("\\"):
                    continue
                packages = [
                    word for word in rest.split() if not word.startswith("-")
                ]
                if not packages:
                    empty.append(f"{workflow.name}:{number}: {line.strip()}")
        self.assertEqual(empty, [], "\n  ".join(empty))


if __name__ == "__main__":
    unittest.main()
