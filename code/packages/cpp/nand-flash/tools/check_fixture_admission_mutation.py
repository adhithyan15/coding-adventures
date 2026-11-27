"""Prove that omitting whole-document model admission breaks its named test."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
BOUND_JSON = PACKAGE.parent / "bounded-json"
HARNESS = PACKAGE.parents[1] / "c/iso-harness"
FIXTURE = PACKAGE.parents[2] / "specs/fixtures/database-media-v1/nand-flash.json"
WITNESS = "ISO_CHECK(rejected(wrong_late_read))"


def copy_inputs(root: Path) -> Path:
    code = root / "code"
    package = code / "packages/cpp/nand-flash"
    package.parent.mkdir(parents=True)
    shutil.copytree(PACKAGE, package)
    shutil.copytree(BOUND_JSON, code / "packages/cpp/bounded-json")
    (code / "packages/c").mkdir(parents=True, exist_ok=True)
    shutil.copytree(HARNESS, code / "packages/c/iso-harness")
    fixture = code / "specs/fixtures/database-media-v1/nand-flash.json"
    fixture.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(FIXTURE, fixture)
    return package


def run(package: Path, build_dir: Path) -> subprocess.CompletedProcess[str]:
    env = dict(os.environ)
    for name in list(env):
        if name.startswith("ISO_"):
            env.pop(name)
    env["ISO_BUILD_DIR"] = str(build_dir)
    return subprocess.run(
        ["sh", "tools/run.sh"],
        cwd=package,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=180,
        check=False,
    )


def main() -> int:
    parent = Path(os.environ.get("TMPDIR", tempfile.gettempdir())).resolve()
    with tempfile.TemporaryDirectory(prefix="db-e133-model-admission-", dir=parent) as raw:
        root = Path(raw)
        package = copy_inputs(root / "clean")
        clean = run(package, root / "clean-build")
        summaries = re.findall(r"(\d+) checks, 0 failed", clean.stdout)
        if clean.returncode != 0 or not summaries or not any(int(item) > 0 for item in summaries):
            print(clean.stdout[-8_000:])
            raise SystemExit("clean control failed or reported no positive checks")
        print(f"clean control: {', '.join(summaries)} checks passed")

        source = package / "tests/nand_flash_test.cpp"
        text = source.read_text(encoding="utf-8")
        start = text.find("void admit_fixture_with_model(const Fixture &fixture) {")
        end = text.find("\nFixture parse_fixture(", start)
        if start < 0 or end < 0:
            raise SystemExit("whole-document model-admission source anchor changed")
        mutant = (
            text[:start]
            + "void admit_fixture_with_model(const Fixture &fixture) {\n"
            + "    (void)fixture; // mutation: whole-document model admission omitted\n"
            + "}\n"
            + text[end + 1 :]
        )
        source.write_text(mutant, encoding="utf-8")
        mutated = run(package, root / "mutant-build")
        failed = re.search(r"\d+ checks, [1-9]\d* failed", mutated.stdout)
        if (mutated.returncode == 0 or failed is None or WITNESS not in mutated.stdout):
            print(mutated.stdout[-8_000:])
            raise SystemExit("model-admission mutant survived or failed without its witness")
        print(f"omitted-model-admission mutant: killed by {WITNESS}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
