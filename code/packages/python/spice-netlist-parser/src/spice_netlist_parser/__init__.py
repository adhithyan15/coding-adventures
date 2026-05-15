"""SPICE3 netlist parser for the coding-adventures SPICE engine."""

from spice_netlist_parser.parser import (
    AcAnalysis,
    DcAnalysis,
    ModelCard,
    NetlistParseError,
    OpAnalysis,
    ParsedNetlist,
    TranAnalysis,
    parse_netlist,
)

parse = parse_netlist
__version__ = "0.1.6"

__all__ = [
    "AcAnalysis",
    "DcAnalysis",
    "ModelCard",
    "NetlistParseError",
    "OpAnalysis",
    "ParsedNetlist",
    "TranAnalysis",
    "__version__",
    "parse",
    "parse_netlist",
]
