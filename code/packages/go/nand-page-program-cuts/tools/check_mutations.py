"""Prove three disposable page-cut mutations fail their intended Go tests."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

TARGETS = (
    "TestPrefixCutDoesNotProgramSuffix|TestPrefixCutConsumesProgramOpportunity|"
    "TestUnacknowledgedCutsRemainUnacknowledged"
)


def run(go: str, cwd: Path, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [go, "test", "-count=1", "-run", TARGETS],
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=60,
        check=False,
    )


def main() -> int:
    package = Path(__file__).resolve().parents[1]
    workspace = package.parent
    go = os.environ.get("GO", "go")
    source = (package / "program.go").read_text(encoding="utf-8")
    mutants = {
        "extra-prefix-suffix-byte": (
            "copy(bytes[cut.PrefixBytes:], before.Bytes[cut.PrefixBytes:])",
            "copy(bytes[cut.PrefixBytes:], payload[cut.PrefixBytes:])",
            "TestPrefixCutDoesNotProgramSuffix",
        ),
        "clear-marker-after-prefix": (
            "case AfterPrefix:\n\t\tcopy(bytes[:cut.PrefixBytes], payload[:cut.PrefixBytes])\n\t\tcopy(bytes[cut.PrefixBytes:], before.Bytes[cut.PrefixBytes:])\n\t\tprogrammed = true",
            "case AfterPrefix:\n\t\tcopy(bytes[:cut.PrefixBytes], payload[:cut.PrefixBytes])\n\t\tcopy(bytes[cut.PrefixBytes:], before.Bytes[cut.PrefixBytes:])\n\t\tprogrammed = false",
            "TestPrefixCutConsumesProgramOpportunity",
        ),
        "acknowledge-before-completion": (
            "case AfterMedia:\n\t\tcopy(bytes, payload)\n\t\tprogrammed = true",
            "case AfterMedia:\n\t\tcopy(bytes, payload)\n\t\tprogrammed = true\n\t\tacknowledged = true",
            "TestUnacknowledgedCutsRemainUnacknowledged",
        ),
    }
    env = {key: value for key, value in os.environ.items() if key not in {"GOFLAGS", "GOWORK"}}
    env["GOTOOLCHAIN"] = "local"
    env["GOWORK"] = "off"
    clean = run(go, package, env)
    if clean.returncode != 0:
        print("clean control failed:\n" + clean.stdout)
        return 1

    for name, (anchor, replacement, witness) in mutants.items():
        if source.count(anchor) != 1:
            print(f"{name}: expected one source anchor, found {source.count(anchor)}")
            return 1
        mutant = source.replace(anchor, replacement, 1)
        with tempfile.TemporaryDirectory(prefix=f"nand-page-cut-{name}-", dir=workspace) as temp:
            copy = Path(temp)
            for item in package.iterdir():
                if item.is_file() and item.suffix in {".go", ".mod"}:
                    shutil.copy2(item, copy / item.name)
            (copy / "program.go").write_text(mutant, encoding="utf-8")
            result = run(go, copy, env)
            if result.returncode != 1 or f"--- FAIL: {witness} (" not in result.stdout:
                print(f"{name}: inconclusive exit {result.returncode}:\n{result.stdout}")
                return 1
            print(f"{name}: killed by {witness}")
    print("clean control passed; all three mutations killed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
