"""fpga-place-route-bridge: HNL -> existing fpga package's JSON config."""

from fpga_place_route_bridge.bridge import (
    TRUTH_TABLES,
    FpgaBridgeOptions,
    FpgaBridgeReport,
    hnl_to_fpga_json,
)

__version__ = "0.1.0"

__all__ = [
    "FpgaBridgeOptions",
    "FpgaBridgeReport",
    "TRUTH_TABLES",
    "__version__",
    "hnl_to_fpga_json",
]
