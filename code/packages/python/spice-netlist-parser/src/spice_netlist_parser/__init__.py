"""SPICE3 netlist parser for the coding-adventures SPICE engine."""

from spice_netlist_parser.parser import (
    AcAnalysis,
    DcAnalysis,
    McAnalysis,
    ModelCard,
    NetlistParseError,
    OpAnalysis,
    ParsedNetlist,
    SensAnalysis,
    TfAnalysis,
    TranAnalysis,
    parse_netlist,
)

parse = parse_netlist
__version__ = "0.1.6"

__all__ = [
    "AcAnalysis",
    "DcAnalysis",
    "McAnalysis",
    "ModelCard",
    "NetlistParseError",
    "OpAnalysis",
    "ParsedNetlist",
    "SensAnalysis",
    "TfAnalysis",
    "TranAnalysis",
    "__version__",
    "parse",
    "parse_netlist",
]
