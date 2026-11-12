from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import ruby_pins_check as pins  # noqa: E402

# (locked version, requirement, expected).  The expectations were produced by
# Ruby's own Gem::Requirement#satisfied_by?, so the Python matcher is held to
# RubyGems' semantics rather than to our reading of them.
REQUIREMENT_CASES = [
    ("5.27.0", "~> 5.0", True),
    ("6.0.0", "~> 5.0", False),
    ("5.27.0", "~> 5.27", True),
    ("5.27.0", "~> 5.28", False),
    ("0.22.0", "~> 0.22.0", True),
    ("0.23.0", "~> 0.22.0", False),
    ("0.22.0", "~> 0.22", True),
    ("0.23.0", "~> 0.22", True),
    ("1.56.0", "~> 1.56.0", True),
    ("1.57.0", "~> 1.56.0", False),
    ("1.56.0", "~> 1", True),
    ("2.0", "~> 1", False),
    ("13.4.2", "~> 13.0", True),
    ("2.21.2", ">= 2.21.0", True),
    ("2.9.1", ">= 2.21.0", False),
    ("0.3.0", ">= 0.2.0", True),
    ("1.0", "= 1", True),
    ("1.0.1", "= 1", False),
    ("1.0", "!= 1", False),
    ("1.0", "< 1.0.pre", False),
    ("1.0.pre", "< 1.0", True),
    ("1.0", "<= 1.0", True),
    ("1.0", "> 1.0", False),
    ("10.0", "> 9.9", True),
]


class VersionRequirementTests(unittest.TestCase):
    def test_requirements_match_rubygems(self) -> None:
        for version, requirement, expected in REQUIREMENT_CASES:
            with self.subTest(version=version, requirement=requirement):
                self.assertIs(pins.satisfies(version, requirement), expected)

    def test_rejects_unparseable_requirement(self) -> None:
        with self.assertRaises(pins.RubyPinsError):
            pins.satisfies("1.0", "about 1")

    def test_requirement_runs_stop_at_options(self) -> None:
        self.assertEqual(
            pins.requirements_from_tail(', "~> 5.0", ">= 5.1", require: false'),
            ["~> 5.0", ">= 5.1"],
        )
        self.assertEqual(pins.requirements_from_tail(", require: false"), [])


LOCK = """GEM
  remote: https://rubygems.org/
  specs:
    minitest (5.27.0)
    prism (2.1.0)
    prism (2.1.0-x86_64-linux-gnu)
    rake (13.4.2)

PLATFORMS
  arm64-darwin
  ruby
  x64-mingw-ucrt
  x86_64-darwin
  x86_64-linux

DEPENDENCIES
  minitest (~> 5.27, ~> 5.0)
  rake (~> 13.0)

CHECKSUMS
  minitest (5.27.0) sha256=aa
  prism (2.1.0) sha256=bb
  prism (2.1.0-x86_64-linux-gnu) sha256=cc
  rake (13.4.2) sha256=dd

BUNDLED WITH
  4.0.24
"""

GEMFILE = """source "https://rubygems.org"
gem "minitest", "~> 5.0", "~> 5.27"
gem "rake", "~> 13.0"
"""

GEMSPEC = """Gem::Specification.new do |spec|
  spec.name = "coding_adventures_demo"
  spec.add_dependency "coding_adventures_other", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
"""


class CheckTests(unittest.TestCase):
    def build_repo(self, *, gemfile=GEMFILE, lock=LOCK, gemspec=GEMSPEC, extra=None):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        (root / pins.THIRD_PARTY_DIR).mkdir(parents=True)
        (root / pins.THIRD_PARTY_DIR / "Gemfile").write_text(gemfile)
        (root / pins.THIRD_PARTY_DIR / "Gemfile.lock").write_text(lock)
        allow = root / pins.ALLOWLIST
        allow.parent.mkdir(parents=True)
        allow.write_text(
            json.dumps({"allowed_dependency_names": ["minitest", "rake", "rspec"]})
        )
        pkg = root / "pkg"
        pkg.mkdir()
        (pkg / "demo.gemspec").write_text(gemspec)
        other = root / "other"
        other.mkdir()
        (other / "other.gemspec").write_text(
            'Gem::Specification.new do |spec|\n  spec.name = "coding_adventures_other"\nend\n'
        )
        (pkg / "Gemfile").write_text(
            'source "https://rubygems.org"\ngemspec\n'
            'gem "coding_adventures_other", path: "../other"\n'
            + (extra or "")
        )
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        return root

    def test_in_sync(self) -> None:
        self.assertEqual(pins.check(self.build_repo()), [])

    def test_unlisted_gem_is_reported(self) -> None:
        root = self.build_repo(extra='gem "rack"\n')
        self.assertTrue(any("rack" in p and "missing" in p for p in pins.check(root)))

    def test_new_requirement_range_is_reported(self) -> None:
        root = self.build_repo(extra='gem "rake", "~> 12.0"\n')
        problems = pins.check(root)
        self.assertTrue(any('rake "~> 12.0"' in p for p in problems))
        self.assertTrue(any("violates" in p for p in problems))

    def test_lock_out_of_range_is_reported(self) -> None:
        lock = LOCK.replace("rake (13.4.2)", "rake (12.3.3)")
        problems = pins.check(self.build_repo(lock=lock))
        self.assertTrue(any("rake is locked at 12.3.3" in p for p in problems))

    def test_missing_checksum_and_platform(self) -> None:
        lock = LOCK.replace("  rake (13.4.2) sha256=dd\n", "").replace(
            "  x64-mingw-ucrt\n", ""
        )
        problems = pins.check(self.build_repo(lock=lock))
        self.assertIn("lock has no checksum for rake", problems)
        self.assertIn("lock is missing platform x64-mingw-ucrt", problems)

    def test_non_allowlisted_central_gem(self) -> None:
        gemfile = GEMFILE + 'gem "nokogiri"\n'
        problems = pins.check(self.build_repo(gemfile=gemfile))
        self.assertTrue(any("nokogiri" in p and "allowlisted" in p for p in problems))

    def test_trivial_any_version_requirement_is_ignored(self) -> None:
        root = self.build_repo(extra='gem "minitest", ">= 0"\n')
        self.assertEqual(pins.check(root), [])

    def test_missing_lock(self) -> None:
        root = self.build_repo()
        (root / pins.THIRD_PARTY_DIR / "Gemfile.lock").unlink()
        self.assertTrue(any("missing" in p for p in pins.check(root)))


class RepositoryTests(unittest.TestCase):
    def test_this_repository_is_in_sync(self) -> None:
        root = SCRIPTS_DIR.parents[1]
        self.assertEqual(pins.check(root), [])


if __name__ == "__main__":
    unittest.main()
