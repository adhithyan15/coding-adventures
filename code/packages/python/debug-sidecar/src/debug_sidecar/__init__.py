"""debug-sidecar — source-location companion for the IIR pipeline.

Public API::

    from debug_sidecar import DebugSidecarWriter, DebugSidecarReader
    from debug_sidecar import SourceLocation, Variable
"""

from debug_sidecar.reader import DebugSidecarReader
from debug_sidecar.types import SourceLocation, Variable
from debug_sidecar.writer import DebugSidecarWriter

__all__ = [
    "DebugSidecarWriter",
    "DebugSidecarReader",
    "SourceLocation",
    "Variable",
]
