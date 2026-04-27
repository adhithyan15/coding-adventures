"""Test configuration for local package imports."""

from __future__ import annotations

import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
PROTOCOL_SRC = ROOT.parent / "simulator-protocol" / "src"

if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))
if str(PROTOCOL_SRC) not in sys.path:
    sys.path.insert(0, str(PROTOCOL_SRC))
