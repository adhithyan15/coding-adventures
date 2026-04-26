"""coverage-hdl: code + functional coverage measurement for the silicon stack."""

from coverage_hdl.coverage import (
    Bin,
    CoverageRecorder,
    CoverageReport,
    Coverpoint,
    CrossPoint,
    ToggleStats,
    bin_default,
    bin_range,
    bin_value,
)

__version__ = "0.1.0"

__all__ = [
    "Bin",
    "CoverageRecorder",
    "CoverageReport",
    "Coverpoint",
    "CrossPoint",
    "ToggleStats",
    "__version__",
    "bin_default",
    "bin_range",
    "bin_value",
]
