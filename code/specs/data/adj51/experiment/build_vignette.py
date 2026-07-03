#!/usr/bin/env python3
"""Generate an adj-lang vignette (observe lines + queries) from the
ingestion JSON. Combined with the derived rulebook, this gives the
engine a complete adj-lang program."""

import json
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: build_vignette.py <ingestion.json> <out.adj>")
        return 2
    ingestion_path, out_path = sys.argv[1], sys.argv[2]
    with open(ingestion_path) as f:
        d = json.load(f)

    lines = [
        "% Vignette generated from 02-ingestion.json by build_vignette.py",
        "% Each observe line corresponds to one entry in observations[].",
        "% raw_value metadata is held by the framework but not surfaced here.",
        "",
    ]
    for o in d["observations"]:
        rv = o.get("raw_value")
        if rv:
            lines.append(f"% raw_value({o['id']}) = {rv!r}")
        lines.append(f"observe {o['term']}")
    lines.append("")
    lines.append("% Queries extracted by ingester:")
    for q in d["queries"]:
        lines.append(f"? {q['term']}")
    lines.append("")
    with open(out_path, "w") as f:
        f.write("\n".join(lines))
    print(f"wrote {out_path} — {len(d['observations'])} observations, {len(d['queries'])} queries")
    return 0


if __name__ == "__main__":
    sys.exit(main())
