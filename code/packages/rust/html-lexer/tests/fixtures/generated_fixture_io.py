"""Shared JSON IO helpers for checked-in HTML lexer fixtures."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def write_fixture_json(output: Path, fixture: dict[str, Any], *, check: bool) -> int:
    text = json.dumps(fixture, indent=2, ensure_ascii=False, sort_keys=True) + "\n"

    if check:
        if output.read_text() != text:
            raise SystemExit(f"{output} is stale; regenerate it")
        return 0

    output.write_text(text)
    return 0
