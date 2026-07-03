#!/usr/bin/env python3
"""ADJ57 Layer 0 — the content-addressed source store (CAS).

Every source text the spider ever reads is interned here, keyed by the SHA-256 of
its content. The point is to pay decomposition cost ONCE per source, ever: the
first case that needs PIOPED II interns it with byte-provenanced citation spans;
every future case — any domain — that cites PIOPED II reuses the interned spans
instead of re-fetching and re-decomposing. This is the realization of ADJ51's
"indexed-source corpus."

An entry holds:
  - the raw `content` (the fetched text),
  - `citations`: the exact byte spans cited from it, each with what it was used for
    (this is "byte provenance ON a source" — a retrievable span, not a footnote),
  - `onward_citations`: the sources THIS source cites (so the spider can follow them
    toward a root source), and where each is interned once fetched.

Content-addressing gives free deduplication: interning identical content twice
yields the same hash and a single object.

CLI: python cas.py stats | get <hash> | ls
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

_HEX64 = re.compile(r"[0-9a-f]{64}")


def _require_hash(h: str) -> str:
    """A CAS key is always a sha256 hexdigest. Reject anything else so a hash can
    never become a path-traversal fragment, even if a future caller passes a raw
    JSON-supplied identifier straight into get()/cite()/add_onward()."""
    if not _HEX64.fullmatch(h):
        raise ValueError(f"not a valid CAS hash: {h!r}")
    return h

CAS_DIR = Path(__file__).resolve().parent.parent / "cas"
OBJECTS = CAS_DIR / "objects"
INDEX = CAS_DIR / "index.json"


def _load_index() -> dict:
    return json.loads(INDEX.read_text()) if INDEX.exists() else {}


def _save_index(idx: dict) -> None:
    CAS_DIR.mkdir(parents=True, exist_ok=True)
    INDEX.write_text(json.dumps(idx, indent=2, sort_keys=True))


def content_hash(content: str) -> str:
    return hashlib.sha256(content.encode("utf-8")).hexdigest()


def intern(content: str, url: str = "", title: str = "", interned_at: str = "") -> str:
    """Intern a source by content. Idempotent: identical content -> same hash, no
    duplicate object. Returns the hash."""
    h = content_hash(content)
    OBJECTS.mkdir(parents=True, exist_ok=True)
    obj_path = OBJECTS / f"{h}.json"
    if not obj_path.exists():
        obj_path.write_text(json.dumps({
            "hash": h, "url": url, "title": title, "interned_at": interned_at,
            "content": content, "citations": [], "onward_citations": [],
        }, indent=2))
        idx = _load_index()
        idx[h] = {"url": url, "title": title, "bytes": len(content), "citations": 0}
        _save_index(idx)
    return h


def get(h: str) -> dict:
    return json.loads((OBJECTS / f"{_require_hash(h)}.json").read_text())


def _put(obj: dict) -> None:
    (OBJECTS / f"{obj['hash']}.json").write_text(json.dumps(obj, indent=2))


def find_by_url(url: str) -> str | None:
    """Reuse hook: has this URL already been interned? (So a new derivation can skip
    the fetch + decomposition entirely.)"""
    for h, meta in _load_index().items():
        if meta.get("url") and meta["url"] == url:
            return h
    return None


def cite(h: str, quote: str, used_for: str) -> dict:
    """Byte-provenance a span IN an interned source: locate `quote` in the content
    and record the retrievable [start,end). Raises if the quote is not literally
    present — a citation must point at real bytes, not a paraphrase."""
    obj = get(h)
    start = obj["content"].find(quote)
    if start < 0:
        raise ValueError(f"quote not found verbatim in source {h[:12]}: {quote[:80]!r}")
    end = start + len(quote)
    rec = {"start": start, "end": end, "quote": quote, "used_for": used_for}
    obj["citations"].append(rec)
    _put(obj)
    idx = _load_index()
    idx[h]["citations"] = len(obj["citations"])
    _save_index(idx)
    return {"hash": h, **rec}


def add_onward(h: str, citation: str, interned_hash: str = "") -> None:
    """Record that source `h` cites `citation` (optionally already interned as
    `interned_hash`) — the edge the spider follows toward a root source."""
    obj = get(h)
    obj["onward_citations"].append({"citation": citation, "interned_hash": interned_hash})
    _put(obj)


def stats() -> dict:
    idx = _load_index()
    return {
        "sources": len(idx),
        "total_bytes": sum(m.get("bytes", 0) for m in idx.values()),
        "total_citations": sum(m.get("citations", 0) for m in idx.values()),
    }


def main() -> None:
    cmd = sys.argv[1] if len(sys.argv) > 1 else "stats"
    if cmd == "stats":
        print(json.dumps(stats(), indent=2))
    elif cmd == "ls":
        for h, m in _load_index().items():
            print(f"{h[:16]}  {m.get('bytes',0):>7}b  {m.get('citations',0)} cites  {m.get('title','')[:60]}")
    elif cmd == "get" and len(sys.argv) > 2:
        print(json.dumps(get(sys.argv[2]), indent=2))
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
