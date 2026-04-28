"""synthesis: HIR -> HNL.

RTL inference and operator lowering. Combinational synthesis for v0.1.0;
sequential (FF inference from clocked processes) and FSM extraction land in v0.2.0.
"""

from synthesis.synth import SynthCtx, synthesize

__version__ = "0.1.0"

__all__ = ["SynthCtx", "__version__", "synthesize"]
