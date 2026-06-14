"""Shared JSON IO helpers for checked-in HTML lexer fixtures."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def write_fixture_json(
    output: Path,
    fixture: dict[str, Any],
    *,
    check: bool,
    ensure_ascii: bool = False,
    sort_keys: bool = True,
    stale_hint: str | None = None,
) -> int:
    text = json.dumps(
        fixture,
        indent=2,
        ensure_ascii=ensure_ascii,
        sort_keys=sort_keys,
    ) + "\n"

    if check:
        if output.read_text() != text:
            hint = f" {stale_hint}" if stale_hint is not None else ""
            raise SystemExit(f"{output} is stale; regenerate it{hint}")
        return 0

    output.write_text(text)
    return 0
