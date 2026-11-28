"""Compare C# adapter trees with the public Python 1992 reference parser."""

from __future__ import annotations

import json
import random
import subprocess
import sys
from pathlib import Path

repo = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(repo / "code/packages/python/historical-web/src"))
from historical_web.html_tags_1992 import Element, parse_html_tags_1992


def canonical(nodes: list[Element | str]) -> list[object]:
    def one(node: Element | str) -> object:
        if isinstance(node, str):
            return node
        return {
            "tag": node.tag,
            "attributes": node.attributes,
            "children": [one(child) for child in node.children],
        }

    return [one(node) for node in nodes]


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: python_parity.py DOTNET ADAPTER_DLL")
    fixture_path = repo / "code/specs/fixtures/historical-web-v1/vectors.json"
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    sources = [case["source"] for case in fixture["html-tags-1992"]]
    sources.extend(
        bytes.fromhex(exchange["response_hex"]).decode("utf-8")
        for exchange in fixture["exchanges"]
    )

    generated = 5000
    seed = 0x485745423130
    rng = random.Random(seed)
    atoms = [
        "one",
        "é",
        "水",
        "🛰",
        "&lt;",
        "&amp;lt;",
        "&quot;",
        "&#65;",
        "<1",
        "<",
        "<!-- comment <P> -->",
        "<!-- open",
        "<A HREF=/x>link</A>",
        "<A HREF HREF=x>v</A>",
        "<A HREF='x>y'>q</A>",
        '<A HREF="a&amp;b">v</A>',
        "<P>",
        "<LI>",
        "</A>",
        "<UL>",
        "</UL>",
        "<MENU>",
        "</MENU>",
        "<DIR>",
        "</DIR>",
        "<DL>",
        "</DL>",
        "<DT>",
        "</DT>",
        "<DD>",
        "</DD>",
        "<H1>",
        "</H1>",
        "<TITLE>",
        "</TITLE>",
        "<ADDRESS>",
        "</ADDRESS>",
        "<XMP>raw &amp; <A></XMP>",
        "<LISTING>a\r\nb</LISTING>",
        "<PLAINTEXT>tail <P>",
        "<unknown>skip</unknown>",
        "<A HREF==x>broken</A>",
    ]
    for _ in range(generated):
        sources.append("".join(rng.choice(atoms) for _ in range(rng.randrange(1, 9))))

    payload = "".join(source.encode("utf-8").hex() + "\n" for source in sources)
    try:
        result = subprocess.run(
            [sys.argv[1], sys.argv[2]],
            input=payload,
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=True,
            timeout=60,
        )
    except subprocess.TimeoutExpired as error:
        raise SystemExit("C# parity adapter timed out after 60 seconds") from error
    except subprocess.CalledProcessError as error:
        detail = error.stderr.strip() if error.stderr else "no stderr"
        raise SystemExit(
            f"C# parity adapter exited {error.returncode}: {detail}"
        ) from error
    except OSError as error:
        raise SystemExit(f"could not start C# parity adapter: {error}") from error

    actual = [json.loads(line) for line in result.stdout.splitlines()]
    if len(actual) != len(sources):
        raise SystemExit(
            f"C# adapter returned {len(actual)} rows for {len(sources)} documents"
        )
    for index, source in enumerate(sources):
        expected = canonical(parse_html_tags_1992(source))
        if actual[index] != expected:
            raise SystemExit(
                f"C# / Python mismatch at {index}: {source!r}\n"
                f"expected={expected!r}\nactual={actual[index]!r}"
            )

    print(f"C# / Python parity: {len(sources)} documents matched, seed=0x{seed:016X}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
