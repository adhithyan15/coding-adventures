"""Durable contracts for the numbered Forme specification map."""

from __future__ import annotations

import json
import os
import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
SPECS = REPO_ROOT / "code" / "specs"
TYPESCRIPT_PACKAGES = REPO_ROOT / "code" / "packages" / "typescript"
TYPESCRIPT_PROGRAMS = REPO_ROOT / "code" / "programs" / "typescript"
TYPESCRIPT_SITES = REPO_ROOT / "code" / "sites"
FORME_STABLE_PACKAGE_VERSION = "1.0.0"

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
    def test_live_site_local_stages_target_kernel_api_v2(self) -> None:
        stage_sources: list[tuple[Path, str]] = []
        for directory, child_dirs, filenames in os.walk(TYPESCRIPT_SITES):
            child_dirs[:] = sorted(
                name
                for name in child_dirs
                if name not in {".forme", "dist", "node_modules"}
            )
            for filename in sorted(filenames):
                if not filename.endswith((".ts", ".tsx")):
                    continue
                path = Path(directory) / filename
                source = path.read_text(encoding="utf-8")
                if "defineStage(" in source:
                    stage_sources.append((path, source))

        self.assertTrue(stage_sources, "live Forme sites must expose stage definitions")
        for path, source in stage_sources:
            with self.subTest(stage=path.relative_to(TYPESCRIPT_SITES)):
                self.assertIn("KERNEL_API_VERSION", source)
                self.assertRegex(source, r"apiVersion:\s*KERNEL_API_VERSION")

    def test_forme_products_report_the_stable_version(self) -> None:
        for program_name in (
            "forme-doc-demo",
            "forme-hello-world",
            "forme-shell-desktop",
        ):
            with self.subTest(program=program_name):
                program_dir = TYPESCRIPT_PROGRAMS / program_name
                package = json.loads(
                    (program_dir / "package.json").read_text(encoding="utf-8")
                )
                lock = json.loads(
                    (program_dir / "package-lock.json").read_text(encoding="utf-8")
                )
                self.assertEqual(package["version"], FORME_STABLE_PACKAGE_VERSION)
                self.assertEqual(lock["version"], FORME_STABLE_PACKAGE_VERSION)
                self.assertEqual(
                    lock["packages"][""]["version"], FORME_STABLE_PACKAGE_VERSION
                )

        native_dir = TYPESCRIPT_PROGRAMS / "forme-shell-desktop" / "src-tauri"
        cargo_toml = (native_dir / "Cargo.toml").read_text(encoding="utf-8")
        cargo_lock = (native_dir / "Cargo.lock").read_text(encoding="utf-8")
        tauri_config = json.loads(
            (native_dir / "tauri.conf.json").read_text(encoding="utf-8")
        )
        self.assertRegex(
            cargo_toml,
            rf'(?m)^version = "{re.escape(FORME_STABLE_PACKAGE_VERSION)}"$',
        )
        self.assertRegex(
            cargo_lock,
            rf'(?s)name = "forme-shell-desktop-native"\nversion = "{re.escape(FORME_STABLE_PACKAGE_VERSION)}"',
        )
        self.assertEqual(tauri_config["version"], FORME_STABLE_PACKAGE_VERSION)

    def test_forme_packages_and_local_locks_share_the_stable_version(self) -> None:
        package_dirs = sorted(TYPESCRIPT_PACKAGES.glob("forme-*"))
        self.assertTrue(package_dirs, "Forme package set is empty")

        for package_dir in package_dirs:
            with self.subTest(package=package_dir.name):
                package = json.loads(
                    (package_dir / "package.json").read_text(encoding="utf-8")
                )
                self.assertTrue(package["name"].startswith("@coding-adventures/forme-"))
                self.assertEqual(package["version"], FORME_STABLE_PACKAGE_VERSION)

                lock = json.loads(
                    (package_dir / "package-lock.json").read_text(encoding="utf-8")
                )
                self.assertEqual(lock["version"], FORME_STABLE_PACKAGE_VERSION)
                self.assertEqual(
                    lock["packages"][""]["version"], FORME_STABLE_PACKAGE_VERSION
                )
                for entry_path, entry in lock["packages"].items():
                    name = entry.get("name", "")
                    if name.startswith("@coding-adventures/forme-"):
                        self.assertEqual(
                            entry.get("version"),
                            FORME_STABLE_PACKAGE_VERSION,
                            f"{package_dir.name}:{entry_path} carries {name} at a mixed version",
                        )

                changelog = (package_dir / "CHANGELOG.md").read_text(
                    encoding="utf-8"
                )
                self.assertIn(
                    "## 1.0.0 — 2026-10-05\n",
                    changelog,
                    "stable package releases must be recorded in every changelog",
                )

    def test_live_site_locks_do_not_retain_pre_v1_forme_snapshots(self) -> None:
        for site_name in ("landing-page", "blog"):
            lock = json.loads(
                (TYPESCRIPT_SITES / site_name / "package-lock.json").read_text(
                    encoding="utf-8"
                )
            )
            for entry_path, entry in lock["packages"].items():
                name = entry.get("name", "")
                if name.startswith("@coding-adventures/forme-"):
                    with self.subTest(site=site_name, dependency=entry_path):
                        self.assertEqual(
                            entry.get("version"),
                            FORME_STABLE_PACKAGE_VERSION,
                            f"{site_name}:{entry_path} carries {name} at a mixed version",
                        )

    def test_kernel_api_v2_contract_and_migration_guide_are_pinned(self) -> None:
        kinds = (
            TYPESCRIPT_PACKAGES / "forme-types" / "src" / "kinds.ts"
        ).read_text(encoding="utf-8")
        self.assertRegex(kinds, r"KERNEL_API_VERSION = 2 as const")
        self.assertRegex(
            kinds,
            r'RenderedPage:\s+Object\.freeze\(\{ name: "RenderedPage",\s+version: "2\.0" \}\)',
        )

        migration = (
            TYPESCRIPT_PACKAGES / "forme-types" / "MIGRATION-v2.md"
        ).read_text(encoding="utf-8")
        self.assertIn("`RenderedPage.source` is removed", migration)
        self.assertIn("Hosts and runners reject v1", migration)

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

    def test_release_quality_milestone_is_closed(self) -> None:
        roadmap = (SPECS / "FM00-forme-completion-roadmap.md").read_text(
            encoding="utf-8"
        )
        self.assertIn(
            "| 69 | FM-B072 | done | Compose the supported-platform release gate |",
            roadmap,
        )
        self.assertIn(
            "| 70 | FM-B018 | done | Close release-quality gates |", roadmap
        )
        self.assertIn(
            "| 71 | FM-B029 | active | Make duplicate PR CI cancellation and merge state unambiguous |",
            roadmap,
        )
        self.assertIn("Authoring v1 is complete.", roadmap)

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
