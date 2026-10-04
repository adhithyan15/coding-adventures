"""Durable contracts for the numbered Forme specification map."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
SPECS = REPO_ROOT / "code" / "specs"

NUMBERED_SPECS = {
    "FM01": "FM01-forme-kernel.md",
    "FM02": "FM02-forme-plugin-host.md",
    "FM03": "FM03-forme-orchestrator.md",
    "FM04": "FM04-forme-style-ir.md",
    "FM05": "FM05-forme-interactivity-ir.md",
    "FM06": "FM06-forme-aot-compiler.md",
    "FM07": "FM07-forme-cli-dev-server.md",
    "FM08": "FM08-forme-deploy-runner.md",
    "FM09": "FM09-forme-authoring-shell.md",
}

MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)]+\.md(?:#[^)]+)?)\)")
ROADMAP_ROW = re.compile(
    r"^\| (?P<priority>\d+) \| (?P<id>FM-B\d{3}) \| "
    r"(?P<status>done|active|ready|blocked|later) \| "
    r"(?P<work>[^|]+) \| (?P<gate>[^|]+) \|$"
)

ROADMAP_HEADER = "| Priority | ID | Status | Work item | Acceptance gate |"
ROADMAP_SEPARATOR = "|---:|---|---|---|---|"


class FormeSpecMapTests(unittest.TestCase):
    def test_numbered_spec_map_is_complete_and_collision_free(self) -> None:
        actual = sorted(path.name for path in SPECS.glob("FM[0-9][0-9]-*.md"))
        expected = sorted(
            ["FM00-forme-completion-roadmap.md", "FM00-forme-vision.md"]
            + list(NUMBERED_SPECS.values())
        )
        self.assertEqual(actual, expected)

        for number, filename in NUMBERED_SPECS.items():
            with self.subTest(spec=number):
                first_line = (SPECS / filename).read_text(encoding="utf-8").splitlines()[0]
                self.assertTrue(
                    first_line.startswith(f"# {number} — "),
                    f"{filename} must own the {number} heading",
                )

    def test_every_forme_spec_has_an_implementation_status_ledger(self) -> None:
        for path in sorted(SPECS.glob("FM[0-9][0-9]-*.md")):
            with self.subTest(spec=path.name):
                text = path.read_text(encoding="utf-8")
                self.assertIn("## Implementation status\n", text)
                self.assertRegex(text, r"(?m)^\| (?:Surface|Area|Contract) \| Status \|")

    def test_roadmap_links_every_canonical_numbered_spec(self) -> None:
        roadmap = (SPECS / "FM00-forme-completion-roadmap.md").read_text(
            encoding="utf-8"
        )
        for number, filename in NUMBERED_SPECS.items():
            with self.subTest(spec=number):
                self.assertIn(f"[{number}]({filename})", roadmap)

    def test_roadmap_has_one_unique_active_item(self) -> None:
        roadmap = (SPECS / "FM00-forme-completion-roadmap.md").read_text(
            encoding="utf-8"
        )
        backlog = roadmap.split("## Prioritized backlog\n", 1)[1].split(
            "## Dependency path\n", 1
        )[0]
        table_lines = [line for line in backlog.splitlines() if line.strip()]
        self.assertGreaterEqual(len(table_lines), 3, "roadmap backlog table is empty")
        self.assertEqual(table_lines[0], ROADMAP_HEADER)
        self.assertEqual(table_lines[1], ROADMAP_SEPARATOR)

        rows = []
        for line in table_lines[2:]:
            match = ROADMAP_ROW.fullmatch(line)
            self.assertIsNotNone(match, f"malformed roadmap backlog row: {line}")
            rows.append(match)

        self.assertTrue(rows, "roadmap must contain backlog rows")
        priorities = [int(row.group("priority")) for row in rows]
        identifiers = [row.group("id") for row in rows]
        active = [row.group("id") for row in rows if row.group("status") == "active"]
        self.assertEqual(
            priorities,
            list(range(len(rows))),
            "priorities must be ordered, unique, and contiguous from zero",
        )
        self.assertEqual(
            len(identifiers), len(set(identifiers)), "backlog IDs must be unique"
        )
        self.assertEqual(len(active), 1, "exactly one backlog item must be active")

    def test_forme_spec_markdown_links_resolve(self) -> None:
        for path in sorted(SPECS.glob("FM[0-9][0-9]-*.md")):
            text = path.read_text(encoding="utf-8")
            for target in MARKDOWN_LINK.findall(text):
                file_target = target.split("#", 1)[0]
                if file_target.startswith(("http://", "https://")):
                    continue
                with self.subTest(spec=path.name, target=file_target):
                    self.assertTrue(
                        (path.parent / file_target).resolve().is_file(),
                        f"{path.name} links to missing {file_target}",
                    )


if __name__ == "__main__":
    unittest.main()
