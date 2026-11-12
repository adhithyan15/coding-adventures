"""Guard: every reader of the human-languages corpus is covered by the gate.

CI checks the repository out WITHOUT ``code/learning/human-languages`` (about 740
MB in 313,000 files, 60% of the repository) except where a job needs it. The
build job restores it only when the ``human-language-corpus`` gate in
``code/specs/data/ci-gates.json`` fires, which happens when one of the gate's
``packages`` is part of the build. It deliberately has no path glob for the corpus.

That only works while the gate's package list is complete. If a new package or
program starts reading the corpus and is not listed, its tests pass locally
(full checkout) and then fail in CI with a missing-file error, on whichever PR
happens to touch it first. This test finds the readers from the source so the
list cannot silently go stale.

What counts as a reader: a source or fixture file under ``code/packages`` or
``code/programs`` whose text mentions ``human-languages``, other than the
``_fonts`` folder (always checked out, 2.4 MB, read by rust/font-subset,
glyph-parser and pdf) and changelogs. A mention is accepted when its package is
in the gate, or is one of the ``MENTION_ONLY`` build tools below.
"""

from __future__ import annotations

import json
import os
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
GATE_FILE = REPO / "code" / "specs" / "data" / "ci-gates.json"
GATE_ID = "human-language-corpus"

# Packages that name corpus paths without reading the corpus when they build or
# test. Each entry says why, because adding one removes the guard for it.
MENTION_ONLY = {
    # Generated registry of declared source inputs for the human-language-data
    # package. It lists paths as data; it reads them only when hashing that
    # package, which is in the gate.
    "dotnet/programs/build-tool-csharp": "source-input registry strings",
    "swift/programs/build-tool": "source-input registry strings",
    # Gate and planner tests build synthetic changed-file lists with corpus
    # paths; nothing opens a real corpus file (verified on a sparse tree).
    "go/programs/build-tool": "string literals in gate and planner tests",
}

SOURCE_SUFFIXES = (
    ".ts", ".tsx", ".mjs", ".js", ".cjs", ".py", ".rs", ".go", ".rb", ".sh",
    ".cs", ".fs", ".swift", ".kt", ".java", ".lua", ".pl", ".pm", ".ex", ".exs",
    ".hs", ".dart", ".json", ".txt", ".toml", ".ps1",
)
SKIP_DIRS = {
    "node_modules", "target", "dist", "build", ".venv", "__pycache__", ".git",
    ".dart_tool", ".build", "vendor", "_build", "deps",
}


def gate_packages() -> set[str]:
    registry = json.loads(GATE_FILE.read_text(encoding="utf-8"))
    return set(registry["gates"][GATE_ID]["packages"])


def package_of(relative: str) -> str:
    """Qualified package name the build tool uses for a repo-relative path."""
    parts = relative.split("/")
    if parts[1] == "packages":
        return f"{parts[2]}/{parts[3]}"
    return f"{parts[2]}/programs/{parts[3]}"


def corpus_mentions() -> dict[str, list[str]]:
    """Map package -> files that mention the corpus (outside ``_fonts``)."""
    found: dict[str, list[str]] = {}
    for top in ("code/packages", "code/programs"):
        for directory, subdirs, files in os.walk(REPO / top):
            subdirs[:] = [d for d in subdirs if d not in SKIP_DIRS]
            for name in files:
                if not name.endswith(SOURCE_SUFFIXES) or name.startswith("CHANGELOG"):
                    continue
                path = Path(directory) / name
                try:
                    text = path.read_text(encoding="utf-8", errors="ignore")
                except OSError:
                    continue
                if "human-languages" not in text:
                    continue
                if not any(
                    "human-languages" in line and "human-languages/_fonts" not in line
                    for line in text.splitlines()
                ):
                    continue
                relative = path.relative_to(REPO).as_posix()
                found.setdefault(package_of(relative), []).append(relative)
    return found


class HumanLanguageCorpusConsumers(unittest.TestCase):
    def test_every_reader_is_in_the_gate(self) -> None:
        covered = gate_packages() | set(MENTION_ONLY)
        missing = {
            package: files[0]
            for package, files in sorted(corpus_mentions().items())
            if package not in covered
        }
        self.assertEqual(
            {},
            missing,
            "These packages mention code/learning/human-languages but are not in "
            f"the '{GATE_ID}' gate in code/specs/data/ci-gates.json. CI checks out "
            "without the corpus, so their tests would fail there. Add them to the "
            "gate's packages, or to MENTION_ONLY here with a reason if they only "
            "name the path as data.",
        )

    def test_gate_packages_still_exist_and_still_read_the_corpus(self) -> None:
        # A stale gate entry costs a full corpus checkout for nothing.
        mentions = corpus_mentions()
        stale = sorted(gate_packages() - set(mentions))
        self.assertEqual(
            [],
            stale,
            f"'{GATE_ID}' lists packages that no longer mention the corpus; "
            "remove them so builds that touch them stay on the lean checkout.",
        )

    def test_mention_only_entries_are_real(self) -> None:
        mentions = corpus_mentions()
        stale = sorted(set(MENTION_ONLY) - set(mentions))
        self.assertEqual([], stale, "MENTION_ONLY lists a package that mentions nothing")

    def test_gate_has_no_corpus_path_glob(self) -> None:
        # Deliberate. A glob such as code/learning/human-languages/** makes the
        # gate fire for every corpus-only change, which the selectivity test in
        # cigates_test.go (TestPathShardedLanguageDiffsStaySelective) forbids, and
        # at ~10,000 changed corpus paths it exceeds the matcher's work limit
        # (CI_GATE_MATCH_LIMIT_EXCEEDED). The gate only has to answer "is a
        # package that reads the corpus being built", which `packages` does: a
        # corpus-only change affects no package, so its build needs no corpus.
        registry = json.loads(GATE_FILE.read_text(encoding="utf-8"))
        paths = registry["gates"][GATE_ID].get("paths", [])
        self.assertEqual(
            [],
            [p for p in paths if "human-languages" in p],
            "do not add a corpus path glob to this gate; see this test's comment",
        )


if __name__ == "__main__":
    unittest.main()
