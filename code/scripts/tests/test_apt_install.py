"""Tests for the CI apt wrapper.

The script exists because `apt-get update` exits 100 if *any* configured
repository fails, and the runner images ship vendor repositories this project
never installs from. A `packages.microsoft.com` 403 failed a docs-only pull
request's required job.

These tests drive the real script through `APT_PRUNE_ONLY`, against fixture
directories shaped like the runner's, so the pruning is exercised without root
and without touching a real apt. The retry and timeout layers (#12163, #12181)
are driven the same way, with `APT_GET` pointed at a fake apt-get.
"""

from __future__ import annotations

import hashlib
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


# A stand-in for apt-get, so the retry and timeout layers can be driven without
# root, without a network, and without a real apt. It appends one line per
# invocation to $FAKE_LOG -- the DEBIAN_FRONTEND it saw, then its argv -- and
# behaves according to $FAKE_MODE:
#
#   ok              every call succeeds
#   flaky-update    `update` fails until it has been called $FAKE_FAILS times
#   stall-update    `update` hangs (exec'd sleep, so timeout's SIGTERM ends it
#                   outright rather than orphaning a child that holds the pipe)
#   missing-package `install` always exits 100, apt's "unable to locate"
#
# Whatever the mode, `--print-uris` prints $FAKE_URIS (the lines a real apt
# would print for the archives it still has to fetch) and does nothing else.
FAKE_APT_GET = """\
#!/usr/bin/env bash
echo "DEBIAN_FRONTEND=${DEBIAN_FRONTEND:-} $*" >> "$FAKE_LOG"
for arg in "$@"; do
  if [ "$arg" = "--print-uris" ]; then
    [ -n "${FAKE_URIS:-}" ] && cat "$FAKE_URIS"
    exit 0
  fi
done
verb=""
for arg in "$@"; do
  case "$arg" in update|install) verb="$arg"; break ;; esac
done
case "$FAKE_MODE:$verb" in
  flaky-update:update)
    count=$(grep -c ' update$' "$FAKE_LOG")
    [ "$count" -ge "$FAKE_FAILS" ] && exit 0
    exit 100 ;;
  stall-update:update) exec sleep 30 ;;
  missing-package:install) exit 100 ;;
esac
exit 0
"""


def _install(root: Path, mode: str, *packages: str, **extra: str):
    """Run the whole script, update and install, against the fake apt-get."""

    sources_dir, sources_list = _runner_layout(root)
    fake = root / "apt-get"
    fake.write_text(FAKE_APT_GET)
    fake.chmod(0o755)
    log = root / "apt.log"
    log.touch()
    env = {
        "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
        "CI": "true",
        "APT_SUDO": "",
        "APT_GET": str(fake),
        "APT_RETRY_DELAY": "0",
        "APT_SOURCES_DIR": str(sources_dir),
        "APT_SOURCES_LIST": str(sources_list),
        "FAKE_LOG": str(log),
        "FAKE_MODE": mode,
    }
    env.update(extra)
    result = subprocess.run(
        ["bash", str(SCRIPT), *packages],
        env=env,
        capture_output=True,
        text=True,
        timeout=60,
    )
    return result, log.read_text().splitlines()


class BoundedAptTests(unittest.TestCase):
    """The stall layer (#12163, #12181).

    A mirror that accepted the connection and then went silent hung
    `build (ubuntu-latest)` for the full six-hour job limit, more than once.
    These pin the three layers the script now puts around every apt call:
    apt's own timeouts, a wall-clock cap per attempt, and a retry loop.
    """

    def test_passes_the_acquire_and_lock_timeouts_to_every_call(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result, calls = _install(Path(tmp), "ok", "libcairo2-dev")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(calls), 2, calls)
            for call in calls:
                for option in (
                    "-o Acquire::Retries=3",
                    "-o Acquire::http::Timeout=30",
                    "-o Acquire::https::Timeout=30",
                    "-o DPkg::Lock::Timeout=60",
                ):
                    self.assertIn(option, call)
            self.assertTrue(calls[0].endswith(" update"), calls[0])
            self.assertTrue(calls[1].endswith(" install -y libcairo2-dev"), calls[1])

    def test_apt_runs_noninteractive(self) -> None:
        # sudo resets the environment, so this has to be set on the far side
        # of it; a debconf prompt with no terminal is just another hang.
        with tempfile.TemporaryDirectory() as tmp:
            result, calls = _install(Path(tmp), "ok", "libcairo2-dev")

            self.assertEqual(result.returncode, 0, result.stderr)
            for call in calls:
                self.assertTrue(
                    call.startswith("DEBIAN_FRONTEND=noninteractive "), call
                )

    def test_retries_a_transient_update_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result, calls = _install(
                Path(tmp), "flaky-update", "libcairo2-dev", FAKE_FAILS="2"
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(sum(c.endswith(" update") for c in calls), 2, calls)
            self.assertIn("attempt 1/3 failed", result.stderr)
            self.assertTrue(calls[-1].endswith(" install -y libcairo2-dev"))

    def test_a_stalled_update_is_cut_off_and_retried_not_waited_on(self) -> None:
        # The bug itself, in miniature: `update` never returns. The per-attempt
        # cap has to turn that into a failed attempt, retry it, and then fail
        # the script -- in seconds here, minutes on a runner, never hours.
        with tempfile.TemporaryDirectory() as tmp:
            result, calls = _install(
                Path(tmp),
                "stall-update",
                "libcairo2-dev",
                APT_UPDATE_TIMEOUT="1",
                APT_ATTEMPTS="2",
            )

            self.assertEqual(result.returncode, 124, result.stderr)
            self.assertEqual(sum(c.endswith(" update") for c in calls), 2, calls)
            self.assertIn("stalled mirror", result.stderr)
            # And install never ran against an index that was never fetched.
            self.assertFalse(any(" install " in c for c in calls), calls)

    def test_a_non_numeric_knob_is_refused_before_apt_runs(self) -> None:
        # Bash evaluates `-ge` operands as arithmetic, so a knob like
        # `a[$(cmd)]` would run cmd. The script refuses anything but digits.
        with tempfile.TemporaryDirectory() as tmp:
            result, calls = _install(
                Path(tmp), "flaky-update", "libcairo2-dev", APT_ATTEMPTS="a[$(true)]"
            )

            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("APT_ATTEMPTS must be a non-negative integer", result.stderr)
            self.assertEqual(calls, [], calls)

    def test_a_missing_package_still_fails_after_its_retries(self) -> None:
        # Retrying must not become swallowing: the last attempt's status is
        # the script's status, exactly as it was before the retry loop.
        with tempfile.TemporaryDirectory() as tmp:
            result, calls = _install(Path(tmp), "missing-package", "no-such-package")

            self.assertEqual(result.returncode, 100, result.stderr)
            self.assertEqual(sum(" install " in c for c in calls), 3, calls)
            self.assertIn("failed on all 3 attempt(s)", result.stderr)


# A stand-in for apt-cache: `show` prints $FAKE_SHOW, the Packages stanzas a
# real index would hold, whatever package is asked for.
FAKE_APT_CACHE = """\
#!/usr/bin/env bash
[ -n "${FAKE_SHOW:-}" ] && cat "$FAKE_SHOW"
exit 0
"""


class _Archive:
    """One archive as apt describes it in two places, both written by a test.

    `--print-uris` names it by URI and local file name and, like the real
    apt on Ubuntu, quotes only its MD5 sum. The Packages index (read back with
    `apt-cache show`) names it by pool file and records SHA256 and SHA512. The
    two names differ for a version with an epoch, whose colon apt's local name
    spells %3a and the pool name leaves out.
    """

    def __init__(self, package: str, local: str, pool: str, data: bytes) -> None:
        self.package, self.local, self.pool, self.data = package, local, pool, data

    def uri_line(self, size: int | None = None) -> str:
        size = len(self.data) if size is None else size
        md5 = hashlib.md5(self.data).hexdigest()
        encoded = self.pool.replace("+", "%2b")
        return f"'http://mirror.invalid/pool/x/{encoded}' {self.local} {size} MD5Sum:{md5}\n"

    def stanza(self, data: bytes | None = None, *, sha512: bool = True) -> str:
        data = self.data if data is None else data
        lines = [
            f"Package: {self.package}",
            f"Filename: pool/x/{self.pool}",
            f"Size: {len(data)}",
            f"MD5sum: {hashlib.md5(data).hexdigest()}",
            f"SHA256: {hashlib.sha256(data).hexdigest()}",
        ]
        if sha512:
            lines.append(f"SHA512: {hashlib.sha512(data).hexdigest()}")
        return "\n".join(lines) + "\n\n"


class DebCacheTests(unittest.TestCase):
    """The verified .deb cache (#17191).

    A mirror serving the 180 MB TeX download at ~150 KB/s outlasted every
    per-attempt cap, because each attempt was making progress. The cache
    removes the download; these pin that it can only ever save time: a file
    reaches apt's archive directory only if its SHA512/SHA256 and size match
    the signed index, and nothing about the cache can fail an install.
    """

    def _layout(self, root: Path) -> tuple[Path, Path]:
        cache = root / "cache"
        archives = root / "archives"
        cache.mkdir()
        archives.mkdir()
        return cache, archives

    def _run(self, root: Path, uris: str, show: str, **extra: str):
        cache, archives = root / "cache", root / "archives"
        (root / "uris.txt").write_text(uris)
        (root / "show.txt").write_text(show)
        fake_cache = root / "apt-cache"
        fake_cache.write_text(FAKE_APT_CACHE)
        fake_cache.chmod(0o755)
        env = {
            "APT_DEB_CACHE": str(cache),
            "APT_ARCHIVES_DIR": str(archives),
            "APT_CACHE": str(fake_cache),
            "FAKE_URIS": str(root / "uris.txt"),
            "FAKE_SHOW": str(root / "show.txt"),
        }
        env.update(extra)
        return _install(root, "ok", "texlive-xetex", **env)

    def test_a_verified_archive_is_seeded_and_a_tampered_one_is_not(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            good = _Archive("good", "good_1.0_all.deb", "good_1.0_all.deb", b"good")
            bad = _Archive("bad", "bad_1.0_all.deb", "bad_1.0_all.deb", b"expected")
            absent = _Archive("gone", "gone_1.0_all.deb", "gone_1.0_all.deb", b"x")
            (cache / good.local).write_bytes(good.data)
            # Same size as what the index expects, different bytes.
            (cache / bad.local).write_bytes(b"EXPECTED")

            result, calls = self._run(
                root,
                good.uri_line() + bad.uri_line() + absent.uri_line(),
                good.stanza() + bad.stanza(sha512=False) + absent.stanza(),
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(
                "seeded 1 verified archive(s); 1 not cached; 1 rejected", result.stdout
            )
            self.assertEqual((archives / good.local).read_bytes(), good.data)
            self.assertFalse((archives / bad.local).exists())
            # The listing runs after update and before the real install.
            self.assertTrue(calls[0].endswith(" update"), calls)
            self.assertIn("--print-uris install -y texlive-xetex", calls[1])
            self.assertTrue(calls[2].endswith(" install -y texlive-xetex"), calls)

    def test_an_epoch_and_a_plus_still_find_their_index_entry(self) -> None:
        # The real shape: latexmk 1:4.83-1 is latexmk_1%3a4.83-1_all.deb on
        # disk, latexmk_4.83-1_all.deb in the pool; `+` arrives as %2b.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            latexmk = _Archive(
                "latexmk", "latexmk_1%3a4.83-1_all.deb", "latexmk_4.83-1_all.deb", b"mk"
            )
            cowsay = _Archive(
                "cowsay",
                "cowsay_3.03+dfsg2-8_all.deb",
                "cowsay_3.03+dfsg2-8_all.deb",
                b"moo",
            )
            for archive in (latexmk, cowsay):
                (cache / archive.local).write_bytes(archive.data)

            result, _ = self._run(
                root,
                latexmk.uri_line() + cowsay.uri_line(),
                latexmk.stanza() + cowsay.stanza(),
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("seeded 2 verified archive(s)", result.stdout)
            self.assertTrue((archives / latexmk.local).exists())

    def test_only_md5_in_the_index_or_a_size_mismatch_is_rejected(self) -> None:
        # MD5 alone is not a check worth trusting a file on, and the listing's
        # own MD5 field is never used as one.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            weak = _Archive("weak", "weak_1_all.deb", "weak_1_all.deb", b"bytes")
            sized = _Archive("sized", "sized_1_all.deb", "sized_1_all.deb", b"bytes")
            for archive in (weak, sized):
                (cache / archive.local).write_bytes(archive.data)
            md5_only = "Package: weak\nFilename: pool/x/weak_1_all.deb\nSize: 5\n"
            md5_only += f"MD5sum: {hashlib.md5(b'bytes').hexdigest()}\n\n"

            result, _ = self._run(
                root,
                weak.uri_line() + sized.uri_line(size=6),
                md5_only + sized.stanza(),
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(
                "seeded 0 verified archive(s); 0 not cached; 2 rejected", result.stdout
            )
            self.assertEqual([p.name for p in archives.iterdir() if p.name != "partial"], [])

    def test_the_whole_pool_path_must_match_not_just_the_file_name(self) -> None:
        # Two signed sources could publish the same file name and size; the
        # stanza used is the one whose pool path ends the URI apt will fetch.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            ours = _Archive("dup", "dup_1_all.deb", "dup_1_all.deb", b"ours")
            (cache / ours.local).write_bytes(ours.data)
            elsewhere = ours.stanza().replace("pool/x/", "pool/other/")

            result, _ = self._run(root, ours.uri_line(), elsewhere)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("seeded 0 verified archive(s); 0 not cached; 1 rejected", result.stdout)

    def test_a_rejected_copy_leaves_nothing_staged(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            bad = _Archive("bad", "bad_1_all.deb", "bad_1_all.deb", b"expected")
            (cache / bad.local).write_bytes(b"EXPECTED")

            result, _ = self._run(root, bad.uri_line(), bad.stanza())

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(list((archives / "partial").iterdir()), [])
            self.assertEqual([p.name for p in archives.iterdir()], ["partial"])

    def test_a_name_that_is_not_a_plain_archive_name_is_refused(self) -> None:
        # The name comes from apt's output, but it is joined onto two paths;
        # a slash in it must never be followed anywhere.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            escape = _Archive("escape", "../escape.deb", "escape.deb", b"payload")
            (root / "escape.deb").write_bytes(escape.data)

            result, _ = self._run(root, escape.uri_line(), escape.stanza())

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("1 rejected", result.stdout)
            self.assertEqual([p.name for p in archives.iterdir() if p.name != "partial"], [])

    def test_a_symlinked_cache_entry_is_not_followed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            link = _Archive("link", "link_1_all.deb", "link_1_all.deb", b"payload")
            target = root / "elsewhere.deb"
            target.write_bytes(link.data)
            (cache / link.local).symlink_to(target)

            result, _ = self._run(root, link.uri_line(), link.stanza())

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("seeded 0 verified archive(s); 1 not cached", result.stdout)

    def test_after_install_the_cache_holds_exactly_this_install_s_archives(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache, archives = self._layout(root)
            (cache / "old_0.9_all.deb").write_bytes(b"superseded")
            (archives / "new_1.0_all.deb").write_bytes(b"fresh download")

            result, _ = self._run(root, "", "")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(sorted(p.name for p in cache.iterdir()), ["new_1.0_all.deb"])
            self.assertIn("kept 1 archive(s)", result.stdout)

    def test_without_the_knob_nothing_is_listed_or_kept(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result, calls = _install(Path(tmp), "ok", "libcairo2-dev")

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(any("--print-uris" in c for c in calls), calls)
            self.assertNotIn("deb cache", result.stdout + result.stderr)

    def test_a_failed_listing_never_fails_the_install(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._layout(root)
            # The same fake, except that its `--print-uris` branch fails.
            listing_exit = "    exit 0\n  fi\ndone"
            self.assertIn(listing_exit, FAKE_APT_GET)
            failing = root / "apt-get-listing-fails"
            failing.write_text(
                FAKE_APT_GET.replace(listing_exit, "    exit 7\n  fi\ndone", 1)
            )
            failing.chmod(0o755)

            result, calls = self._run(root, "", "", APT_GET=str(failing))

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("could not list the archives", result.stderr)
            self.assertTrue(calls[-1].endswith(" install -y texlive-xetex"), calls)


# A step starts at a list item whose first key is a step key. Matching only
# step keys keeps a `- item` line inside a `run: |` script, or under `with:`,
# from being mistaken for one.
STEP_START = re.compile(
    r"^(\s*)- (name|run|uses|id|if|shell|env|with|working-directory"
    r"|timeout-minutes|continue-on-error):"
)


def _step_around(lines: list[str], index: int) -> tuple[int, list[str]]:
    """The line number of the step containing lines[index], and its lines."""

    indent = len(lines[index]) - len(lines[index].lstrip())
    start = index
    while start >= 0:
        match = STEP_START.match(lines[start])
        if match and (start == index or len(match.group(1)) < indent):
            break
        start -= 1
    if start < 0:
        raise AssertionError(f"no step found around line {index + 1}")
    dash = len(STEP_START.match(lines[start]).group(1))
    end = start + 1
    while end < len(lines):
        line = lines[end]
        if line.strip() and not line.lstrip().startswith("#"):
            if len(line) - len(line.lstrip()) <= dash:
                break
        end += 1
    return start, lines[start:end]


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

    def test_every_apt_get_install_assumes_yes(self) -> None:
        # `-q` is QUIET, not assume-yes. Two sites passed `-q` alone, and a
        # non-interactive `apt-get install` cannot complete an install that
        # requires changes without `-y` -- it prompts and aborts. Those jobs
        # were green, which means the step was not installing anything: it read
        # as a dependency step and behaved as a no-op, and would have started
        # failing the day the runner image stopped shipping those packages.
        #
        # Not routed through the wrapper: these sites run no `apt-get update`,
        # and adding one would change what they do. See #14211.
        offenders = []
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            for number, line in enumerate(workflow.read_text().splitlines(), start=1):
                if not re.search(r"apt-get\s+install", line):
                    continue
                flags = re.findall(r"-[A-Za-z]+", line.split("install", 1)[1])
                if not any("y" in flag for flag in flags):
                    offenders.append(f"{workflow.name}:{number}: {line.strip()}")
        self.assertEqual(
            offenders,
            [],
            "`apt-get install` needs `-y`; without it apt prompts and aborts "
            "in CI, and the step silently does nothing when the packages "
            "happen to be preinstalled:\n  " + "\n  ".join(offenders),
        )

    def test_every_apt_step_has_its_own_timeout(self) -> None:
        # The outermost layer of #12163 / #12181. The script bounds itself,
        # but a step with no `timeout-minutes` inherits the job's -- six hours
        # when the job sets none -- so a hang anywhere the script's caps do not
        # reach still burns a runner and reports `cancelled`. Every step that
        # installs from apt, through the wrapper or directly, carries its own.
        apt_call = re.compile(r"apt-install\.sh|apt-get\s+install")
        offenders = []
        checked = 0
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            lines = workflow.read_text().splitlines()
            seen = set()
            for number, line in enumerate(lines):
                if line.lstrip().startswith("#") or not apt_call.search(line):
                    continue
                start, body = _step_around(lines, number)
                if start in seen:
                    continue
                seen.add(start)
                checked += 1
                if not any(
                    re.match(r"^\s*(- )?timeout-minutes:\s*\d+", step_line)
                    for step_line in body
                ):
                    offenders.append(
                        f"{workflow.name}:{start + 1}: {lines[start].strip()}"
                    )
        # Driven by a regex over the workflows: if it ever stops matching,
        # this would pass by finding nothing. Fail loudly instead.
        self.assertGreater(checked, 10, "apt-step scan found almost nothing")
        self.assertEqual(
            offenders,
            [],
            "these steps install from apt with no `timeout-minutes`, so a "
            "stalled mirror runs them into the job limit:\n  "
            + "\n  ".join(offenders),
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
