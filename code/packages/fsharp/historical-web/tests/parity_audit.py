"""Optional deterministic Python/F# semantic audit; not part of portable BUILD."""

from __future__ import annotations

import json
import os
import random
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PACKAGE = HERE.parent
ROOT = PACKAGE.parents[3]
PYTHON_SRC = ROOT / "code/packages/python/historical-web/src"
sys.path.insert(0, str(PYTHON_SRC))


def python_tree(node: object) -> object:
    from historical_web.html_tags_1992 import Element

    if isinstance(node, str):
        return node
    assert isinstance(node, Element)
    return {
        "tag": node.tag,
        "attributes": node.attributes,
        "children": [python_tree(child) for child in node.children],
    }


def reference_mode() -> int:
    from historical_web.html_tags_1992 import parse_html_tags_1992

    for line in sys.stdin:
        source = json.loads(line)
        print(
            json.dumps(
                [python_tree(node) for node in parse_html_tags_1992(source)],
                ensure_ascii=False,
            )
        )
    return 0


def generated_sources() -> list[str]:
    fixture = json.loads(
        (ROOT / "code/specs/fixtures/historical-web-v1/vectors.json").read_text(
            encoding="utf-8"
        )
    )
    sources = [case["source"] for case in fixture["html-tags-1992"]]
    sources += [
        "<TITLE>Hi</TITLE><P>See <A HREF=/b>B</A>" for _ in fixture["exchanges"]
    ]
    rng = random.Random(0x48454211)
    pieces = [
        "text",
        "é水😀",
        "&lt; &amp;lt; &#65; &quot;",
        "<P>",
        "<LI>",
        "</LI>",
        "<UL>",
        "</UL>",
        "<MENU>",
        "</MENU>",
        "<DIR>",
        "</DIR>",
        "<DL>",
        "<DT>",
        "<DD>",
        "</DL>",
        "<H1>",
        "</H1>",
        "<A HREF=/x NAME='n' TYPE=t>",
        "</A>",
        "<A HREF=>",
        "<A HREF==x>",
        "<XMP>",
        "</XMP>",
        "<LISTING>",
        "</LISTING>",
        "<PLAINTEXT>",
        "<!-- <P> -->",
        "<future x='>'>",
        "</unknown>",
        "<",
        "<1",
        "<?x?>",
        "&lt;",
        "\r\n",
        "\x00",
    ]
    for _ in range(5000):
        count = rng.randint(1, 9)
        sources.append("".join(rng.choice(pieces) for _ in range(count)))
    return sources


def run(command: list[str], data: str, env: dict[str, str], label: str) -> list[str]:
    try:
        completed = subprocess.run(
            command,
            input=data,
            text=True,
            capture_output=True,
            timeout=180,
            env=env,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise SystemExit(
            f"{label} audit command failed or timed out: {error}"
        ) from error
    if completed.returncode != 0:
        raise SystemExit(
            f"{label} audit exited {completed.returncode}: {completed.stderr[-3000:]}"
        )
    return completed.stdout.splitlines()


def main() -> int:
    if len(sys.argv) == 2 and sys.argv[1] == "--reference":
        return reference_mode()
    sources = generated_sources()
    payload = "".join(
        json.dumps(source, ensure_ascii=False) + "\n" for source in sources
    )
    env = os.environ.copy()
    python = env.get("PYTHON", sys.executable)
    dotnet = env.get("DOTNET", "dotnet")
    adapter = PACKAGE / "tests/ParityAdapter/ParityAdapter.fsproj"
    actual_lines = run(
        [dotnet, "run", "--project", str(adapter), "--configuration", "Release"],
        payload,
        env,
        "F#",
    )
    expected_lines = run(
        [python, str(Path(__file__).resolve()), "--reference"], payload, env, "Python"
    )
    if len(actual_lines) != len(sources) or len(expected_lines) != len(sources):
        raise SystemExit(
            f"line-count mismatch: inputs={len(sources)} F#={len(actual_lines)} Python={len(expected_lines)}"
        )
    for index, (expected, actual) in enumerate(
        zip(expected_lines, actual_lines, strict=True)
    ):
        if json.loads(expected) != json.loads(actual):
            raise SystemExit(
                f"tree mismatch at case {index}: source={sources[index]!r}\nPython={expected}\nF#={actual}"
            )
    print(
        f"matched={len(sources)} seed=0x48454211 python={sys.version.split()[0]} dotnet=9 target=F# HWEB00"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
