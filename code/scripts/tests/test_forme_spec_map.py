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
}

MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)]+\.md(?:#[^)]+)?)\)")


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
